use crate::iso::{self, IsoInfo};
use crate::usb::{self, UsbDevice};
use crate::writer::{self, FlashOptions, FlashResult};
use tauri::AppHandle;

/// List all connected USB devices
#[tauri::command]
pub fn list_usb_devices() -> Result<Vec<UsbDevice>, String> {
    usb::list_usb_devices()
}

/// Validate and get info about an ISO file
#[tauri::command]
pub fn validate_iso(path: String) -> Result<IsoInfo, String> {
    iso::validate_iso(&path)
}

/// Compute SHA-256 hash of an ISO file (async-friendly)
#[tauri::command]
pub async fn compute_iso_hash(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || iso::compute_sha256(&path))
        .await
        .map_err(|e| format!("Hash computation failed: {}", e))?
}

/// Start the flash process
#[tauri::command]
pub async fn start_flash(app: AppHandle, options: FlashOptions) -> Result<FlashResult, String> {
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || writer::flash_iso(&app_clone, options))
        .await
        .map_err(|e| format!("Flash failed: {}", e))?
}

/// Cancel the ongoing flash process
#[tauri::command]
pub fn cancel_flash() -> Result<(), String> {
    writer::cancel_flash();
    Ok(())
}

/// Get platform information
#[tauri::command]
pub fn get_platform_info() -> Result<PlatformInfo, String> {
    Ok(PlatformInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        is_admin: check_admin_status(),
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub is_admin: bool,
}

#[cfg(target_os = "windows")]
fn check_admin_status() -> bool {
    use std::process::Command;
    Command::new("net")
        .args(["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn check_admin_status() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(target_os = "macos")]
fn check_admin_status() -> bool {
    unsafe { libc::geteuid() == 0 }
}
