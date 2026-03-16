use crate::writer::{FlashOptions, FlashProgress, FlashResult};
#[cfg(target_os = "windows")]
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::AppHandle;
use tauri::Emitter;

// CREATE_NO_WINDOW: prevents PowerShell/diskpart windows from popping up
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

pub fn cancel_flash() {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
}

pub fn is_cancelled() -> bool {
    CANCEL_FLAG.load(Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
pub fn flash_windows_iso(app: &AppHandle, options: FlashOptions, total_bytes: u64) -> Result<FlashResult, String> {
    CANCEL_FLAG.store(false, Ordering::SeqCst);
    let start_time = Instant::now();

    // The options.device_path comes in as e.g., "\\.\PHYSICALDRIVE1"
    // We need just the disk number for PowerShell "Clear-Disk"
    let disk_number = extract_disk_number(&options.device_path)
        .ok_or_else(|| format!("Invalid Windows device path: {}", options.device_path))?;

    // 1. Mount ISO to get drive letter
    emit_stage(app, "Mounting ISO...");
    let iso_mount_letter = mount_iso(&options.iso_path)?;

    // Make sure we dismount the ISO even if something fails later
    let result = (|| -> Result<FlashResult, String> {
        // 2. Clean and Format USB Drive (exFAT to support >4GB install.wim)
        emit_stage(app, "Formatting USB drive...");
        let (usb_letter, uefi_ntfs_letter) = format_usb_drive(disk_number, &options)?;

        if is_cancelled() {
            return Ok(cancelled_result(start_time));
        }

        // Write UEFI:NTFS if GPT was chosen (uefi_ntfs_letter will be Some)
        if let Some(fat_letter) = uefi_ntfs_letter {
            emit_stage(app, "Writing UEFI:NTFS bootloader...");
            write_uefi_ntfs(&fat_letter)?;
        }

        if is_cancelled() {
            return Ok(cancelled_result(start_time));
        }

        // 3. Copy files using robocopy
        emit_stage(app, "Copying Windows files...");
        copy_files_robocopy(app, &iso_mount_letter, &usb_letter, total_bytes)?;

        if is_cancelled() {
            return Ok(cancelled_result(start_time));
        }

        // 4. Make drive bootable using bootsect (only for MBR)
        let scheme_cmd = if options.partition_scheme.as_deref().unwrap_or("mbr").eq_ignore_ascii_case("gpt") {
            "GPT"
        } else {
            "MBR"
        };
        
        if scheme_cmd == "MBR" {
            emit_stage(app, "Installing Bootloader...");
            install_bootloader(&iso_mount_letter, &usb_letter)?;
        }

        if is_cancelled() {
            return Ok(cancelled_result(start_time));
        }

        emit_stage(app, "done");
        
        let duration = start_time.elapsed().as_secs_f64();
        Ok(FlashResult {
            success: true,
            message: format!("Windows ISO flashed successfully in {:.1}s", duration),
            duration_seconds: duration,
            bytes_written: 0, // Not easily trackable with robocopy overall count
            verified: false,
        })
    })();

    // 5. Unmount ISO
    let _ = unmount_iso(&options.iso_path);

    result
}

#[cfg(not(target_os = "windows"))]
pub fn flash_windows_iso(_app: &AppHandle, _options: FlashOptions) -> Result<FlashResult, String> {
    Err("Windows ISO flashing is strictly supported on Windows hosts in Phase 2. Please use a Windows computer to flash this ISO.".to_string())
}

#[cfg(target_os = "windows")]
fn extract_disk_number(device_path: &str) -> Option<u32> {
    // Expected format: "\\.\PHYSICALDRIVE1"
    if device_path.starts_with(r"\\.\PHYSICALDRIVE") {
        device_path[17..].parse::<u32>().ok()
    } else {
        None
    }
}

fn emit_stage(app: &AppHandle, stage: &str) {
    let _ = app.emit(
        "flash-progress",
        FlashProgress {
            bytes_written: 0,
            total_bytes: 100,
            percent: 0.0,
            speed_mbps: 0.0,
            eta_seconds: 0,
            stage: stage.to_string(),
        },
    );
}

fn cancelled_result(start_time: Instant) -> FlashResult {
    FlashResult {
        success: false,
        message: "Flash cancelled by user".to_string(),
        duration_seconds: start_time.elapsed().as_secs_f64(),
        bytes_written: 0,
        verified: false,
    }
}

#[cfg(target_os = "windows")]
fn mount_iso(iso_path: &str) -> Result<String, String> {
    let script = format!(
        r#"
        $isoPath = "{}"
        Mount-DiskImage -ImagePath $isoPath | Out-Null
        
        $driveLetter = $null
        for ($i = 0; $i -lt 10; $i++) {{
            $vol = Get-DiskImage -ImagePath $isoPath | Get-Volume -ErrorAction SilentlyContinue
            if ($vol -and $vol.DriveLetter) {{
                if ($vol.Count -gt 1) {{
                    $driveLetter = $vol[0].DriveLetter
                }} else {{
                    $driveLetter = $vol.DriveLetter
                }}
                if ($driveLetter) {{ break }}
            }}
            Start-Sleep -Milliseconds 500
        }}
        
        if ($driveLetter) {{
            Write-Output $driveLetter
        }} else {{
            Write-Error "No drive letter assigned to mounted ISO"
        }}
        "#,
        iso_path
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to execute Mount PowerShell: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to mount ISO: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let letter = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if letter.is_empty() {
        return Err("ISO mounted but no drive letter found".to_string());
    }

    Ok(format!("{}:\\", letter))
}

#[cfg(target_os = "windows")]
fn unmount_iso(iso_path: &str) -> Result<(), String> {
    let script = format!(r#"Dismount-DiskImage -ImagePath "{}""#, iso_path);
    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to dismount ISO: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn format_usb_drive(disk_number: u32, options: &FlashOptions) -> Result<(String, Option<String>), String> {
    let fs = options.filesystem.as_deref().unwrap_or("auto");
    let fs_cmd = if fs.eq_ignore_ascii_case("ntfs") {
        "NTFS"
    } else if fs.eq_ignore_ascii_case("fat32") {
        "FAT32"
    } else if fs.eq_ignore_ascii_case("exfat") {
        "exFAT"
    } else {
        "NTFS"
    };

    let is_gpt = options.partition_scheme.as_deref().unwrap_or("mbr").eq_ignore_ascii_case("gpt");
    let label = options.volume_label.as_deref().unwrap_or("BOOTISO");

    // Create a temporary file for the diskpart script
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join(format!("bootiso_diskpart_{}.txt", disk_number));

    if is_gpt {
        let diskpart_script = format!(
            "select disk {}\nclean\nconvert gpt\ncreate partition primary\nshrink desired=2 minimum=2\nformat fs={} label=\"{}\" quick\nassign\ncreate partition primary\nformat fs=fat label=\"UEFI_NTFS\" quick\nassign\nexit\n",
            disk_number, fs_cmd, label
        );

        std::fs::write(&script_path, diskpart_script)
            .map_err(|e| format!("Failed to write diskpart script: {}", e))?;

        let output = Command::new("diskpart")
            .args(["/s", script_path.to_str().unwrap()])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to execute diskpart (GPT): {}", e))?;

        let _ = std::fs::remove_file(&script_path);

        if !output.status.success() {
            return Err(format!(
                "Diskpart GPT format failed: {}",
                String::from_utf8_lossy(&output.stdout) // diskpart usually outputs errors to stdout
            ));
        }

        // Retrieve the two assigned drive letters using PowerShell since diskpart output parsing is frail
        // We look for volumes on this specific disk
        let get_letters_script = format!(
            r#"
            $letters = Get-Partition -DiskNumber {} | Where-Object DriveLetter | Select-Object -ExpandProperty DriveLetter
            $letters -join "`n"
            "#,
            disk_number
        );
        
        let letters_output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &get_letters_script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to get drive letters: {}", e))?;
            
        let letters_str = String::from_utf8_lossy(&letters_output.stdout);
        let mut letters: Vec<&str> = letters_str.trim().lines().collect();
        letters.sort(); // Main partition is usually larger/first, but let's assume alphabetical if we must, or we can check labels.
        
        // Better: get letters by Label to be 100% sure
        let get_letters_by_label = format!(
            r#"
            $main = Get-Partition -DiskNumber {0} | Get-Volume | Where-Object FileSystemLabel -eq '{1}' | Select-Object -ExpandProperty DriveLetter
            $fat = Get-Partition -DiskNumber {0} | Get-Volume | Where-Object FileSystemLabel -eq 'UEFI_NTFS' | Select-Object -ExpandProperty DriveLetter
            Write-Output $main
            Write-Output $fat
            "#,
            disk_number, label
        );
        
        let precise_output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &get_letters_by_label])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to get precise drive letters: {}", e))?;
            
        let precise_str = String::from_utf8_lossy(&precise_output.stdout);
        let precise_lines: Vec<&str> = precise_str.trim().lines().collect();
        
        if precise_lines.len() >= 2 {
            let main_letter = precise_lines[0].trim().to_string();
            let fat_letter = precise_lines[1].trim().to_string();
            Ok((format!("{}:\\", main_letter), Some(format!("{}:\\", fat_letter))))
        } else {
            Err(format!(
                "GPT format succeeded but could not verify both partition letters via WMI. Output: {}",
                precise_str
            ))
        }

    } else {
        // ===== MBR: Use diskpart =====
        let diskpart_script = format!(
            "select disk {}\nclean\nconvert mbr\ncreate partition primary\nactive\nformat fs={} label=\"{}\" quick\nassign\nexit\n",
            disk_number, fs_cmd, label
        );

        std::fs::write(&script_path, diskpart_script)
            .map_err(|e| format!("Failed to write diskpart script: {}", e))?;

        let output = Command::new("diskpart")
            .args(["/s", script_path.to_str().unwrap()])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to execute diskpart (MBR): {}", e))?;

        let _ = std::fs::remove_file(&script_path);

        if !output.status.success() {
            return Err(format!(
                "Diskpart MBR format failed: {}",
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        let get_letter_script = format!(
            "(Get-Partition -DiskNumber {} | Where-Object DriveLetter | Select-Object -First 1).DriveLetter",
            disk_number
        );
        
        let letter_output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &get_letter_script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to get MBR drive letter: {}", e))?;
            
        let letter = String::from_utf8_lossy(&letter_output.stdout).trim().to_string();
        
        if letter.is_empty() {
            return Err("MBR format succeeded but could not find drive letter.".to_string());
        }

        Ok((format!("{}:\\", letter), None))
    }
}

#[cfg(target_os = "windows")]
fn write_uefi_ntfs(fat_letter: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::windows::io::FromRawHandle;
    use windows::Win32::System::Ioctl::{FSCTL_LOCK_VOLUME, FSCTL_DISMOUNT_VOLUME, FSCTL_UNLOCK_VOLUME};
    use windows::Win32::System::IO::DeviceIoControl;
    use windows::Win32::Foundation::HANDLE;

    // The UEFI:NTFS image
    let uefi_ntfs_img = include_bytes!("uefi-ntfs.img");
    
    // fat_letter looks like "E:\"
    let volume_path = format!("\\\\.\\{}:", &fat_letter[..1]);
    
    // Use winapi::CreateFileW for reliable raw volume access
    let wide_path: Vec<u16> = volume_path.encode_utf16().chain(std::iter::once(0)).collect();
    
    let raw_handle = unsafe {
        winapi::um::fileapi::CreateFileW(
            wide_path.as_ptr(),
            winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE,
            winapi::um::winnt::FILE_SHARE_READ | winapi::um::winnt::FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            winapi::um::fileapi::OPEN_EXISTING,
            winapi::um::winnt::FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };

    if raw_handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return Err(format!("Failed to open UEFI_NTFS volume {} for writing (GetLastError={})", 
            volume_path, unsafe { winapi::um::errhandlingapi::GetLastError() }));
    }

    // Wrap in HANDLE for DeviceIoControl calls
    let handle = HANDLE(raw_handle as _);
    // Also wrap in a std::fs::File for write_all (takes ownership for drop/close)
    let mut file = unsafe { std::fs::File::from_raw_handle(raw_handle as _) };

    let mut bytes_returned = 0u32;

    // 1. Lock the volume with retries (Windows may still hold handles from formatting/Explorer)
    let mut locked = false;
    for attempt in 0..10 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        let result = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_LOCK_VOLUME,
                None,
                0,
                None,
                0,
                Some(&mut bytes_returned),
                None,
            )
        };
        if result.is_ok() {
            locked = true;
            break;
        }
    }

    if !locked {
        return Err("Failed to lock UEFI_NTFS volume after 10 attempts. Another process may be using the drive.".to_string());
    }

    // 2. Dismount the volume (invalidates cached filesystem data)
    let _ = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_DISMOUNT_VOLUME,
            None,
            0,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    };

    // 3. Write the image
    let write_result = file.write_all(uefi_ntfs_img)
        .map_err(|e| format!("Failed to write UEFI_NTFS image: {}", e));

    let _ = file.sync_all();

    // 4. Unlock the volume
    let _ = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_UNLOCK_VOLUME,
            None,
            0,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    };
    
    // File is dropped here, closing the handle
    write_result
}

#[cfg(target_os = "windows")]
fn copy_files_robocopy(app: &AppHandle, source: &str, dest: &str, total_bytes: u64) -> Result<(), String> {
    use std::io::Read;
    use std::time::Instant;

    // We use /E instead of /MIR. We removed /NP to allow percentage tracking.
    let mut child = Command::new("robocopy")
        .args([source, dest, "/E", "/MT:16", "/J", "/R:0", "/W:0", "/BYTES", "/NDL", "/NJH", "/NJS", "/FP"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start robocopy: {}", e))?;

    let mut stdout = child.stdout.take().unwrap();
    
    let mut bytes_written_finished_files: u64 = 0;
    let mut current_file_size: u64 = 0;
    let mut current_filename = String::new();
    
    let start_time = Instant::now();
    let mut last_emit = Instant::now();

    let mut buffer = [0u8; 1024];
    let mut line_buf = Vec::new();

    loop {
        if is_cancelled() {
            let _ = child.kill();
            return Ok(());
        }

        match stdout.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                for &byte in &buffer[..n] {
                    // Robocopy uses \r to overwrite percentages inline, and \n for new lines
                    if byte == b'\r' || byte == b'\n' {
                        let line = String::from_utf8_lossy(&line_buf);
                        let trimmed = line.trim();

                        if !trimmed.is_empty() && !trimmed.starts_with("----------------") && !trimmed.starts_with("Total") {
                            // Check if it's a percentage update (e.g., "10.5%" or "100%")
                            if trimmed.ends_with('%') {
                                if let Ok(mut percent_val) = trimmed.trim_end_matches('%').parse::<f64>() {
                                    // Sometimes robocopy prints weird values, clamp it
                                    if percent_val > 100.0 { percent_val = 100.0; }
                                    if percent_val < 0.0 { percent_val = 0.0; }
                                    
                                    let current_file_written = (current_file_size as f64 * (percent_val / 100.0)) as u64;
                                    let total_written_now = bytes_written_finished_files + current_file_written;
                                    
                                    // If we hit 100%, consider this file fully written and add its size to the finished total
                                    if percent_val >= 100.0 {
                                        bytes_written_finished_files += current_file_size;
                                        current_file_size = 0; // Reset for next file
                                    }

                                    if last_emit.elapsed().as_millis() >= 100 {
                                        emit_progress(app, total_written_now, total_bytes, start_time, &current_filename);
                                        last_emit = Instant::now();
                                    }
                                }
                            } else {
                                // It might be a new file line. Format with /BYTES: Status | Size | Path
                                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                                for (_i, part) in parts.iter().enumerate() {
                                    if let Ok(size) = part.parse::<u64>() {
                                        // A new file has started. If we missed the 100% of the previous one,
                                        // add it to finished files now just in case.
                                        if current_file_size > 0 {
                                            bytes_written_finished_files += current_file_size;
                                        }

                                        current_file_size = size;
                                        current_filename = parts.last().unwrap_or(&"").to_string();
                                        
                                        // Just emit the new file name immediately
                                        if last_emit.elapsed().as_millis() >= 100 {
                                            emit_progress(app, bytes_written_finished_files, total_bytes, start_time, &current_filename);
                                            last_emit = Instant::now();
                                        }
                                        break; // Processed the size for this line
                                    }
                                }
                            }
                        }
                        line_buf.clear();
                    } else {
                        line_buf.push(byte);
                    }
                }
            }
            Err(_) => break, // Handle read error gracefully by just exiting loop
        }
    }

    let status = child.wait().map_err(|e| format!("Robocopy wait failed: {}", e))?;
    
    if status.code().unwrap_or(8) >= 8 {
        return Err(format!("Robocopy failed with exit code: {:?}", status.code()));
    }

    Ok(())
}

fn emit_progress(app: &AppHandle, written: u64, total: u64, start_time: Instant, filename: &str) {
    let elapsed = start_time.elapsed().as_secs_f64();
    let speed_mbps = if elapsed > 0.0 {
        (written as f64 / (1024.0 * 1024.0)) / elapsed
    } else {
        0.0
    };
    
    let remaining_bytes = total.saturating_sub(written);
    let eta = if speed_mbps > 0.0 {
        (remaining_bytes as f64 / (speed_mbps * 1024.0 * 1024.0)) as u64
    } else {
        0
    };
    
    let percent = ((written as f64 / total as f64) * 100.0).min(99.9);
    
    let _ = app.emit(
        "flash-progress",
        FlashProgress {
            bytes_written: written,
            total_bytes: total,
            percent,
            speed_mbps,
            eta_seconds: eta,
            stage: format!("Copying {}...", filename),
        },
    );
}

#[cfg(target_os = "windows")]
fn install_bootloader(iso_letter: &str, usb_letter: &str) -> Result<(), String> {
    // Use bootsect.exe from the ISO's boot folder to install the BOOTMGR boot code to the USB
    let bootsect_path = format!("{}boot\\bootsect.exe", iso_letter);
    let target = format!("{}", &usb_letter[..2]); // e.g. "E:"

    let output = Command::new(&bootsect_path)
        .args(["/nt60", &target, "/force", "/mbr"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                // Not a fatal error if bootsect fails (e.g. UEFI only ISO), but we log it
                println!("Bootsect warning: {}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => {
            println!("Could not run bootsect (might be a UEFI-only ISO): {}", e);
        }
    }

    Ok(())
}
