pub mod windows_iso;
mod commands;
mod iso;
mod usb;
mod writer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_usb_devices,
            commands::validate_iso,
            commands::compute_iso_hash,
            commands::start_flash,
            commands::cancel_flash,
            commands::get_platform_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
