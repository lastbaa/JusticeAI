pub mod assertions;
pub mod commands;
pub mod pipeline;
pub mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

// Port of the local HTTP file server. Set once at startup, read by the command.
static FILE_SERVER_PORT: OnceLock<u16> = OnceLock::new();

/// Percent-decode a URL path (`/Users/foo/my%20file.pdf` → `/Users/foo/my file.pdf`).
fn percent_decode_path(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Start a minimal Tokio HTTP server on a random loopback port.
/// The server serves files from the filesystem by path — used by the document
/// viewer iframe. WKWebView reliably renders PDFs from http://127.0.0.1 URLs.
async fn start_file_server() -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("PDF file server: failed to bind");
    let port = listener.local_addr().unwrap().port();
    log::info!("PDF file server listening on 127.0.0.1:{port}");

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let Ok(n) = stream.read(&mut buf).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buf[..n]);

                // "GET /path/to/file HTTP/1.1" — extract the path segment.
                let raw_path = request
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/");

                // Strip the leading '/' then re-add it so we get an absolute path.
                // encodeURI on the frontend leaves slashes intact, so the path is
                // already properly structured — just decode percent-encoded chars.
                let file_path = percent_decode_path(raw_path);

                match tokio::fs::read(&file_path).await {
                    Ok(bytes) => {
                        let mime = if file_path.to_lowercase().ends_with(".pdf") {
                            "application/pdf"
                        } else {
                            "application/octet-stream"
                        };
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nContent-Disposition: inline\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                            bytes.len()
                        );
                        let _ = stream.write_all(header.as_bytes()).await;
                        let _ = stream.write_all(&bytes).await;
                    }
                    Err(e) => {
                        let body = format!("File not found: {e}");
                        let header = format!(
                            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes()).await;
                        let _ = stream.write_all(body.as_bytes()).await;
                    }
                }
            });
        }
    });

    port
}

/// Return the port the local PDF file server is listening on.
#[tauri::command]
fn get_file_server_port() -> u16 {
    *FILE_SERVER_PORT.get().unwrap_or(&0)
}

/// Return build fingerprint (git hash + timestamp) so the UI can show which build is running.
#[tauri::command]
fn get_build_info() -> String {
    format!("{} ({})", env!("BUILD_GIT_HASH"), env!("BUILD_TIMESTAMP"),)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("justice-ai".into()),
                    },
                ))
                .level(log::LevelFilter::Debug)
                .build(),
        )
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

            log::info!("Justice AI build: {} ({})", env!("BUILD_GIT_HASH"), env!("BUILD_TIMESTAMP"));

            // Load persisted data synchronously before the window opens so
            // the first IPC calls from the renderer always see a fully loaded state.
            // Start the local PDF file server before the window opens.
            let port = tauri::async_runtime::block_on(start_file_server());
            FILE_SERVER_PORT.set(port).ok();

            let mut rag = state::RagState::new(data_dir.clone());
            tauri::async_runtime::block_on(async {
                rag.load_from_disk().await;
                // Migration: if stored chunks were embedded with the old AllMiniL model,
                // re-embed them now using BGE-small-en-v1.5 before the window opens.
                // Text is stored in chunk metadata — no file re-parsing needed.
                if rag.embed_model != "bge-small-en-v1.5" {
                    if !rag.embedded_chunks.is_empty() {
                        log::info!("Stale embeddings detected — migrating to BGE-small-en-v1.5 (this runs once)…");
                        commands::rag::migrate_embeddings(&mut rag).await;
                    } else {
                        // Fresh install: stamp the model name so future loads skip migration.
                        rag.embed_model = "bge-small-en-v1.5".to_string();
                        rag.save_embed_model().await;
                    }
                }
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
            get_file_server_port,
            get_build_info,
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
            commands::rag::save_file,
            commands::rag::get_cases,
            commands::rag::save_case,
            commands::rag::delete_case,
            commands::rag::assign_file_to_case,
            commands::rag::assign_session_to_case,
            commands::rag::set_case_jurisdiction,
            commands::rag::get_case_summaries,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
