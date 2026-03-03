mod commands;
pub mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(|app| {
            // Gracefully fall back to temp dir if app data dir is unavailable
            // (sandboxed environments, permission issues) rather than panicking.
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|e| {
                    log::warn!("Could not resolve app data dir ({e}); falling back to temp dir");
                    std::env::temp_dir().join("justice-ai")
                });
            std::fs::create_dir_all(&data_dir).ok();

            // Load persisted data synchronously before the window opens so
            // the first IPC calls from the renderer always see a fully loaded state.
            let mut rag = state::RagState::new(data_dir.clone());
            tauri::async_runtime::block_on(async {
                rag.load_from_disk().await;
            });

            app.manage(Arc::new(Mutex::new(rag)));
            // Register the close-permission flag used by on_window_event.
            app.manage(state::CloseAllowed(AtomicBool::new(false)));
            Ok(())
        })
        .on_window_event(|window, event| {
            // Two-phase close: the Rust handler always intercepts the first attempt
            // and emits 'app-close-requested' to JS. JS shows a confirmation dialog
            // if busy, then calls the `set_can_close` command to flip the flag, and
            // finally calls appWindow.close() — which re-fires this handler. On the
            // second pass the flag is true so we let it through.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let allowed = window.state::<state::CloseAllowed>();
                if allowed.0.load(Ordering::SeqCst) {
                    // JS confirmed the close — reset flag and let window close.
                    allowed.0.store(false, Ordering::SeqCst);
                } else {
                    api.prevent_close();
                    window.emit("app-close-requested", ()).ok();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::ollama::check_ollama,
            commands::rag::check_models,
            commands::rag::download_models,
            commands::rag::set_can_close,
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
