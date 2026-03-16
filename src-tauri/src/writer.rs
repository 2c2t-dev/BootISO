use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Write, BufWriter};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

const DEFAULT_BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashOptions {
    pub iso_path: String,
    pub device_path: String,
    pub buffer_size: Option<usize>,
    pub verify_after_write: Option<bool>,
    pub filesystem: Option<String>,      // "auto", "fat32", "ntfs", "ext4"
    pub partition_scheme: Option<String>, // "mbr", "gpt"
    pub volume_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashProgress {
    pub bytes_written: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub speed_mbps: f64,
    pub eta_seconds: u64,
    pub stage: String, // "writing", "verifying", "done", "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashResult {
    pub success: bool,
    pub message: String,
    pub duration_seconds: f64,
    pub bytes_written: u64,
    pub verified: bool,
}

static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

pub fn cancel_flash() {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
}

pub fn is_cancelled() -> bool {
    CANCEL_FLAG.load(Ordering::SeqCst)
}

/// Main flash function - writes ISO to USB device using DD-like raw writing
pub fn flash_iso(app: &AppHandle, options: FlashOptions) -> Result<FlashResult, String> {
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let buffer_size = options.buffer_size.unwrap_or(DEFAULT_BUFFER_SIZE);

    // Validate ISO to check its type
    let iso_info = crate::iso::validate_iso(&options.iso_path)
        .map_err(|e| format!("ISO validation failed: {}", e))?;

    // If it's a Windows ISO, use the specialized Windows flashing routine
    if iso_info.iso_type == crate::iso::IsoType::Windows {
        #[cfg(target_os = "windows")]
        {
            return crate::windows_iso::flash_windows_iso(app, options, iso_info.size);
        }
        #[cfg(not(target_os = "windows"))]
        {
            return Err("Flashing Windows ISOs is currently only supported on Windows hosts.".to_string());
        }
    }

    // Open ISO file for raw dd-like writing (Linux ISOs)
    let iso_file =
        File::open(&options.iso_path).map_err(|e| format!("Cannot open ISO: {}", e))?;
    let iso_metadata = iso_file
        .metadata()
        .map_err(|e| format!("Cannot read ISO metadata: {}", e))?;
    let total_bytes = iso_metadata.len();

    // Platform-specific device opening
    let device = open_device_for_writing(&options.device_path)?;

    let mut reader = BufReader::with_capacity(buffer_size, iso_file);
    let mut writer = BufWriter::with_capacity(buffer_size, device);
    let mut buffer = vec![0u8; buffer_size];
    let mut bytes_written: u64 = 0;
    let start_time = std::time::Instant::now();
    let mut last_emit = std::time::Instant::now();

    // SHA256 hasher for the ISO data as we write it
    use sha2::{Sha256, Digest};
    let mut iso_hasher = Sha256::new();

    // Emit initial progress
    let _ = app.emit(
        "flash-progress",
        FlashProgress {
            bytes_written: 0,
            total_bytes,
            percent: 0.0,
            speed_mbps: 0.0,
            eta_seconds: 0,
            stage: "writing".to_string(),
        },
    );

    loop {
        if is_cancelled() {
            return Ok(FlashResult {
                success: false,
                message: "Flash cancelled by user".to_string(),
                duration_seconds: start_time.elapsed().as_secs_f64(),
                bytes_written,
                verified: false,
            });
        }

        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("Read error: {}", e))?;

        if bytes_read == 0 {
            break;
        }

        // Hash the data as we write it
        iso_hasher.update(&buffer[..bytes_read]);

        writer
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Write error: {}", e))?;

        bytes_written += bytes_read as u64;

        // Emit progress every 100ms to avoid flooding the frontend
        if last_emit.elapsed().as_millis() >= 100 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed_mbps = if elapsed > 0.0 {
                (bytes_written as f64 / (1024.0 * 1024.0)) / elapsed
            } else {
                0.0
            };
            let remaining_bytes = total_bytes.saturating_sub(bytes_written);
            let eta = if speed_mbps > 0.0 {
                (remaining_bytes as f64 / (speed_mbps * 1024.0 * 1024.0)) as u64
            } else {
                0
            };

            let _ = app.emit(
                "flash-progress",
                FlashProgress {
                    bytes_written,
                    total_bytes,
                    percent: (bytes_written as f64 / total_bytes as f64) * 100.0,
                    speed_mbps,
                    eta_seconds: eta,
                    stage: "writing".to_string(),
                },
            );
            last_emit = std::time::Instant::now();
        }
    }

    // Flush the writer to ensure all data is written
    writer
        .flush()
        .map_err(|e| format!("Flush error: {}", e))?;

    // Drop the writer to release the device handle
    drop(writer);

    let iso_hash = format!("{:x}", iso_hasher.finalize());

    // --- Verification phase: re-read the USB and compute SHA256 ---
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "flash-progress",
        FlashProgress {
            bytes_written,
            total_bytes,
            percent: 0.0,
            speed_mbps: 0.0,
            eta_seconds: 0,
            stage: "verifying".to_string(),
        },
    );

    let verified = match verify_device(&options.device_path, total_bytes, buffer_size, &iso_hash, app) {
        Ok(v) => v,
        Err(e) => {
            // Verification failed to run, but writing succeeded
            let _ = app.emit(
                "flash-progress",
                FlashProgress {
                    bytes_written,
                    total_bytes,
                    percent: 100.0,
                    speed_mbps: 0.0,
                    eta_seconds: 0,
                    stage: "done".to_string(),
                },
            );
            let duration = start_time.elapsed().as_secs_f64();
            return Ok(FlashResult {
                success: true,
                message: format!(
                    "Flash completed in {:.1}s but verification failed: {}",
                    duration, e
                ),
                duration_seconds: duration,
                bytes_written,
                verified: false,
            });
        }
    };

    let duration = start_time.elapsed().as_secs_f64();

    // Emit completion
    let _ = app.emit(
        "flash-progress",
        FlashProgress {
            bytes_written,
            total_bytes,
            percent: 100.0,
            speed_mbps: 0.0,
            eta_seconds: 0,
            stage: "done".to_string(),
        },
    );

    Ok(FlashResult {
        success: true,
        message: format!(
            "Flash completed in {:.1}s ({})",
            duration,
            format_iso_size(bytes_written)
        ),
        duration_seconds: duration,
        bytes_written,
        verified,
    })
}

/// Re-read the device and compare SHA256 hash with the ISO hash.
/// Returns Ok(true) if hashes match, Ok(false) if user skipped (cancelled), Err on I/O error.
fn verify_device(
    device_path: &str,
    total_bytes: u64,
    buffer_size: usize,
    iso_hash: &str,
    app: &AppHandle,
) -> Result<bool, String> {
    use sha2::{Sha256, Digest};

    // Small delay to let OS release device handles
    std::thread::sleep(std::time::Duration::from_millis(500));

    let device = open_device_for_reading(device_path)?;
    let mut reader = BufReader::with_capacity(buffer_size, device);
    let mut buffer = vec![0u8; buffer_size];
    let mut bytes_read_total: u64 = 0;
    let mut device_hasher = Sha256::new();
    let verify_start = std::time::Instant::now();
    let mut last_emit = std::time::Instant::now();

    loop {
        // Check if user wants to skip verification
        if is_cancelled() {
            return Ok(false);
        }

        let remaining = total_bytes - bytes_read_total;
        if remaining == 0 {
            break;
        }

        let to_read = std::cmp::min(remaining as usize, buffer_size);
        let bytes_read = reader
            .read(&mut buffer[..to_read])
            .map_err(|e| format!("Verify read error: {}", e))?;

        if bytes_read == 0 {
            break;
        }

        device_hasher.update(&buffer[..bytes_read]);
        bytes_read_total += bytes_read as u64;

        // Emit verification progress every 100ms
        if last_emit.elapsed().as_millis() >= 100 {
            let elapsed = verify_start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (bytes_read_total as f64 / (1024.0 * 1024.0)) / elapsed
            } else {
                0.0
            };
            let remaining_bytes = total_bytes.saturating_sub(bytes_read_total);
            let eta = if speed > 0.0 {
                (remaining_bytes as f64 / (speed * 1024.0 * 1024.0)) as u64
            } else {
                0
            };

            let _ = app.emit(
                "flash-progress",
                FlashProgress {
                    bytes_written: bytes_read_total,
                    total_bytes,
                    percent: (bytes_read_total as f64 / total_bytes as f64) * 100.0,
                    speed_mbps: speed,
                    eta_seconds: eta,
                    stage: "verifying".to_string(),
                },
            );
            last_emit = std::time::Instant::now();
        }
    }

    let device_hash = format!("{:x}", device_hasher.finalize());

    if device_hash == iso_hash {
        Ok(true)
    } else {
        Err(format!(
            "Hash mismatch! ISO: {}... Device: {}...",
            &iso_hash[..16],
            &device_hash[..16]
        ))
    }
}

#[cfg(target_os = "windows")]
fn open_device_for_reading(device_path: &str) -> Result<File, String> {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x80000000) // FILE_FLAG_NO_BUFFERING
        .open(device_path)
        .map_err(|e| format!("Cannot open device {} for reading: {}", device_path, e))
}

#[cfg(target_os = "linux")]
fn open_device_for_reading(device_path: &str) -> Result<File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECT)
        .open(device_path)
        .map_err(|e| format!("Cannot open device {} for reading: {}", device_path, e))
}

#[cfg(target_os = "macos")]
fn open_device_for_reading(device_path: &str) -> Result<File, String> {
    let raw_path = device_path.replace("/dev/disk", "/dev/rdisk");
    std::fs::OpenOptions::new()
        .read(true)
        .open(&raw_path)
        .map_err(|e| format!("Cannot open device {} for reading: {}", raw_path, e))
}

fn format_iso_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

#[cfg(target_os = "windows")]
fn open_device_for_writing(device_path: &str) -> Result<File, String> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::process::CommandExt;

    // Use PowerShell to clear the disk first to prevent Windows "Access Denied" (os error 5)
    // when writing raw bytes to a disk that has mounted volumes.
    let disk_number = if device_path.starts_with(r"\\.\PHYSICALDRIVE") {
        device_path[17..].parse::<u32>().ok()
    } else {
        None
    };

    if let Some(disk) = disk_number {
        let script = format!(
            "Clear-Disk -Number {} -RemoveData -RemoveOEM -Confirm:$false -ErrorAction SilentlyContinue",
            disk
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        // Give Windows a moment to unmount everything
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }

    // On Windows, open the physical drive with GENERIC_WRITE
    std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(0x80000000) // FILE_FLAG_NO_BUFFERING
        .open(device_path)
        .map_err(|e| format!("Cannot open device {} for writing (run as admin): {}", device_path, e))
}

#[cfg(target_os = "linux")]
fn open_device_for_writing(device_path: &str) -> Result<File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_DIRECT | libc::O_SYNC)
        .open(device_path)
        .map_err(|e| format!("Cannot open device {} (run as root/sudo): {}", device_path, e))
}

#[cfg(target_os = "macos")]
fn open_device_for_writing(device_path: &str) -> Result<File, String> {
    // On macOS, we need to unmount first, then open the raw device
    let raw_path = device_path.replace("/dev/disk", "/dev/rdisk");

    // Unmount the disk first
    let _ = std::process::Command::new("diskutil")
        .args(["unmountDisk", device_path])
        .output();

    std::fs::OpenOptions::new()
        .write(true)
        .open(&raw_path)
        .map_err(|e| format!("Cannot open device {} (run with sudo): {}", raw_path, e))
}
