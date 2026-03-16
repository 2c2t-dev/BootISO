use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub size_formatted: String,
    pub vendor: String,
    pub model: String,
    pub removable: bool,
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(target_os = "windows")]
pub fn list_usb_devices() -> Result<Vec<UsbDevice>, String> {
    // Use PowerShell to get removable drives via WMI
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"
            Get-WmiObject Win32_DiskDrive | Where-Object { $_.MediaType -like '*removable*' -or $_.MediaType -like '*external*' -or $_.InterfaceType -eq 'USB' } | ForEach-Object {
                $disk = $_
                $partitions = $disk.GetRelated('Win32_DiskPartition')
                $letters = @()
                foreach ($part in $partitions) {
                    $logicals = $part.GetRelated('Win32_LogicalDisk')
                    foreach ($l in $logicals) { $letters += $l.DeviceID }
                }
                [PSCustomObject]@{
                    DeviceID = $disk.DeviceID
                    Model = $disk.Model
                    Size = $disk.Size
                    Letters = ($letters -join ',')
                } | ConvertTo-Json -Compress
            }
            "#,
        ])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            let device_id = val["DeviceID"].as_str().unwrap_or("").to_string();
            let model = val["Model"].as_str().unwrap_or("Unknown").to_string();
            let size = val["Size"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| val["Size"].as_u64())
                .unwrap_or(0);
            let letters = val["Letters"].as_str().unwrap_or("").to_string();

            let display_name = if !letters.is_empty() {
                format!("{} [{}]", model, letters)
            } else {
                model.clone()
            };

            devices.push(UsbDevice {
                name: display_name,
                path: device_id,
                size,
                size_formatted: format_size(size),
                vendor: String::new(),
                model,
                removable: true,
            });
        }
    }

    Ok(devices)
}

#[cfg(target_os = "linux")]
pub fn list_usb_devices() -> Result<Vec<UsbDevice>, String> {
    let output = Command::new("lsblk")
        .args(["-Jdno", "NAME,SIZE,VENDOR,MODEL,RM,TYPE,TRAN"])
        .output()
        .map_err(|e| format!("Failed to run lsblk: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse lsblk: {}", e))?;

    let mut devices = Vec::new();

    if let Some(blockdevices) = json["blockdevices"].as_array() {
        for dev in blockdevices {
            let rm = dev["rm"].as_bool().unwrap_or(false)
                || dev["rm"].as_str() == Some("1");
            let tran = dev["tran"].as_str().unwrap_or("");

            if !rm && tran != "usb" {
                continue;
            }

            let name_str = dev["name"].as_str().unwrap_or("").to_string();
            let model = dev["model"]
                .as_str()
                .unwrap_or("Unknown")
                .trim()
                .to_string();
            let vendor = dev["vendor"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            let size_str = dev["size"].as_str().unwrap_or("0");

            // Parse size from lsblk (returns in human-readable format)
            let size = parse_human_size(size_str);

            devices.push(UsbDevice {
                name: format!("{} {} ({})", vendor, model, format_size(size)),
                path: format!("/dev/{}", name_str),
                size,
                size_formatted: format_size(size),
                vendor,
                model,
                removable: true,
            });
        }
    }

    Ok(devices)
}

#[cfg(target_os = "linux")]
fn parse_human_size(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    let (num_part, suffix) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(s.len()),
    );
    let num: f64 = num_part.parse().unwrap_or(0.0);
    let multiplier = match suffix.trim().to_uppercase().as_str() {
        "B" => 1u64,
        "K" => 1024,
        "M" => 1024 * 1024,
        "G" => 1024 * 1024 * 1024,
        "T" => 1024 * 1024 * 1024 * 1024,
        _ => 1,
    };
    (num * multiplier as f64) as u64
}

#[cfg(target_os = "macos")]
pub fn list_usb_devices() -> Result<Vec<UsbDevice>, String> {
    let output = Command::new("diskutil")
        .args(["list", "-plist", "external", "physical"])
        .output()
        .map_err(|e| format!("Failed to run diskutil: {}", e))?;

    let _stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    // Parse plist output - look for disk identifiers
    // Simplified: use diskutil info on each found disk
    let disk_output = Command::new("diskutil")
        .args(["list", "external"])
        .output()
        .map_err(|e| format!("Failed to run diskutil list: {}", e))?;

    let disk_stdout = String::from_utf8_lossy(&disk_output.stdout);

    for line in disk_stdout.lines() {
        if line.starts_with("/dev/disk") {
            let disk_path = line.split_whitespace().next().unwrap_or("").to_string();
            if disk_path.is_empty() {
                continue;
            }

            // Get info for this disk
            if let Ok(info_out) = Command::new("diskutil")
                .args(["info", &disk_path])
                .output()
            {
                let info = String::from_utf8_lossy(&info_out.stdout);
                let mut model = String::from("USB Drive");
                let mut size: u64 = 0;

                for info_line in info.lines() {
                    let info_line = info_line.trim();
                    if info_line.starts_with("Device / Media Name:") {
                        model = info_line
                            .split(':')
                            .nth(1)
                            .unwrap_or("USB Drive")
                            .trim()
                            .to_string();
                    }
                    if info_line.starts_with("Disk Size:") {
                        // Extract byte count from parentheses
                        if let Some(start) = info_line.find('(') {
                            if let Some(end) = info_line.find(" Bytes") {
                                let byte_str = &info_line[start + 1..end];
                                let byte_str = byte_str.replace(',', "").replace(' ', "");
                                size = byte_str.parse().unwrap_or(0);
                            }
                        }
                    }
                }

                devices.push(UsbDevice {
                    name: format!("{} ({})", model, format_size(size)),
                    path: disk_path,
                    size,
                    size_formatted: format_size(size),
                    vendor: String::new(),
                    model,
                    removable: true,
                });
            }
        }
    }

    Ok(devices)
}
