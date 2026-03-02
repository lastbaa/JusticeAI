mod commands;
pub mod state;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let rag_state = Arc::new(Mutex::new(state::RagState::new(data_dir.clone())));

            // Load persisted data on startup (non-blocking)
            let rag_clone = rag_state.clone();
            tauri::async_runtime::spawn(async move {
                let mut s = rag_clone.lock().await;
                s.load_from_disk().await;
            });

            app.manage(rag_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ollama::check_ollama,
            commands::rag::open_file_dialog,
            commands::rag::open_folder_dialog,
            commands::rag::load_files,
            commands::rag::get_files,
            commands::rag::remove_file,
            commands::rag::query,
            commands::rag::get_settings,
            commands::rag::save_settings,
            commands::rag::save_session,
            commands::rag::get_sessions,
            commands::rag::delete_session,
            commands::rag::get_file_data,
            commands::rag::get_page_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
