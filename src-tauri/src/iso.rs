use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IsoType {
    Windows,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsoInfo {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub size_formatted: String,
    pub iso_type: IsoType,
    pub label: String,
    pub valid: bool,
    pub sha256: Option<String>,
    pub boot_type: String, // "UEFI", "BIOS/Legacy", "Hybrid", "Unknown"
}

const ISO_MAGIC: &[u8; 5] = b"CD001";
const ISO_MAGIC_OFFSET: u64 = 0x8001; // Primary Volume Descriptor at sector 16 + 1

/// Validate an ISO file and extract metadata
pub fn validate_iso(path: &str) -> Result<IsoInfo, String> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }

    if !file_path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("iso"))
        .unwrap_or(false)
    {
        return Err("File is not an ISO image".to_string());
    }

    let file = File::open(file_path).map_err(|e| format!("Cannot open file: {}", e))?;
    let metadata = file
        .metadata()
        .map_err(|e| format!("Cannot read metadata: {}", e))?;
    let size = metadata.len();

    let filename = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown.iso")
        .to_string();

    // Check ISO 9660 magic number
    let mut reader = BufReader::new(&file);
    let valid = check_iso_magic(&mut reader);

    // Try to read the volume label from primary volume descriptor
    let label = if valid {
        read_volume_label(&mut reader).unwrap_or_else(|| filename.clone())
    } else {
        filename.clone()
    };

    // Detect ISO type (Windows vs Linux)
    let iso_type = detect_iso_type(&mut reader, &label, &filename);

    // Detect boot type
    let boot_type = detect_boot_type(&mut reader);

    Ok(IsoInfo {
        path: path.to_string(),
        filename,
        size,
        size_formatted: format_size(size),
        iso_type,
        label,
        valid,
        sha256: None, // Computed async on demand
        boot_type,
    })
}

/// Compute SHA-256 hash of an ISO file (can be slow for large files)
pub fn compute_sha256(path: &str) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
    let mut reader = BufReader::with_capacity(4 * 1024 * 1024, file); // 4MB buffer
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 4 * 1024 * 1024];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn check_iso_magic(reader: &mut BufReader<&File>) -> bool {
    if reader.seek(SeekFrom::Start(ISO_MAGIC_OFFSET)).is_err() {
        return false;
    }
    let mut magic = [0u8; 5];
    if reader.read_exact(&mut magic).is_err() {
        return false;
    }
    &magic == ISO_MAGIC
}

fn read_volume_label(reader: &mut BufReader<&File>) -> Option<String> {
    // Volume label is at offset 0x8028 in the primary volume descriptor (32 bytes)
    reader.seek(SeekFrom::Start(0x8028)).ok()?;
    let mut label = [0u8; 32];
    reader.read_exact(&mut label).ok()?;
    let label_str = String::from_utf8_lossy(&label).trim().to_string();
    if label_str.is_empty() {
        None
    } else {
        Some(label_str)
    }
}

fn detect_iso_type(reader: &mut BufReader<&File>, label: &str, filename: &str) -> IsoType {
    let label_upper = label.to_uppercase();
    let filename_upper = filename.to_uppercase();

    // Check for Windows indicators
    let windows_indicators = [
        "WINDOWS",
        "WIN10",
        "WIN11",
        "WINPE",
        "MICROSOFT",
        "CCCOMA_X64",
        "CCCOMA_X86",
        "J_CCSA_X64",
        "J_CCSA_X86",
    ];

    for indicator in &windows_indicators {
        if label_upper.contains(indicator) || filename_upper.contains(indicator) {
            return IsoType::Windows;
        }
    }

    // Try to detect by looking for Windows-specific files in the ISO
    // Check for bootmgr signature near known offsets
    if let Ok(_) = reader.seek(SeekFrom::Start(0)) {
        let mut buf = vec![0u8; 65536];
        if let Ok(n) = reader.read(&mut buf) {
            let content = String::from_utf8_lossy(&buf[..n]).to_uppercase();
            if content.contains("BOOTMGR") || content.contains("INSTALL.WIM") {
                return IsoType::Windows;
            }
        }
    }

    // Check for common Linux indicators
    let linux_indicators = [
        "UBUNTU",
        "DEBIAN",
        "FEDORA",
        "ARCH",
        "LINUX",
        "MINT",
        "CENTOS",
        "MANJARO",
        "OPENSUSE",
        "KALI",
        "TAILS",
        "POPOS",
    ];

    for indicator in &linux_indicators {
        if label_upper.contains(indicator) || filename_upper.contains(indicator) {
            return IsoType::Linux;
        }
    }

    IsoType::Unknown
}

fn detect_boot_type(reader: &mut BufReader<&File>) -> String {
    // Check for El Torito boot record at sector 17
    let el_torito_offset = 0x8801;
    let mut has_bios = false;
    let mut has_uefi = false;

    if let Ok(_) = reader.seek(SeekFrom::Start(el_torito_offset)) {
        let mut magic = [0u8; 5];
        if reader.read_exact(&mut magic).is_ok() && &magic == ISO_MAGIC {
            has_bios = true;
        }
    }

    // Check for EFI boot files by searching ISO content
    if let Ok(_) = reader.seek(SeekFrom::Start(0)) {
        let mut buf = vec![0u8; 1024 * 256]; // Read first 256KB
        if let Ok(n) = reader.read(&mut buf) {
            let content = String::from_utf8_lossy(&buf[..n]).to_uppercase();
            if content.contains("EFI") || content.contains("UEFI") {
                has_uefi = true;
            }
        }
    }

    match (has_bios, has_uefi) {
        (true, true) => "Hybrid (BIOS + UEFI)".to_string(),
        (true, false) => "BIOS/Legacy".to_string(),
        (false, true) => "UEFI".to_string(),
        (false, false) => "Unknown".to_string(),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
