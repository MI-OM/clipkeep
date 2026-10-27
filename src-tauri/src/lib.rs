mod commands;
mod download;
mod transcription;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::{Manager, image::Image};

            let window_icon = Image::new(include_bytes!("../icons/window-icon.rgba"), 256, 256);
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(window_icon)?;
            }

            let dependency_root = app.path().app_local_data_dir()?.join("media-tools");
            let transcription_root = app.path().app_local_data_dir()?.join("transcription");
            app.manage(download::AppState::new(dependency_root, transcription_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_media_metadata,
            commands::start_download,
            commands::cancel_download,
            commands::get_dependency_status,
            commands::install_media_tools,
            transcription::get_transcription_status,
            transcription::install_transcription_model,
            transcription::transcribe_file,
            transcription::save_transcript
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
