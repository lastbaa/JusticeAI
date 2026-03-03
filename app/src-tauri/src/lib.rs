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

            // Load persisted data synchronously before the window opens so
            // the first IPC calls from the renderer always see a fully loaded state.
            let mut rag = state::RagState::new(data_dir.clone());
            tauri::async_runtime::block_on(async {
                rag.load_from_disk().await;
            });

            app.manage(Arc::new(Mutex::new(rag)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ollama::check_ollama,
            commands::rag::check_models,
            commands::rag::download_models,
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
