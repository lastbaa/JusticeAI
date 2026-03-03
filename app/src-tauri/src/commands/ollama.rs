use crate::state::{OllamaStatus, RagState};
use std::sync::Arc;
use tokio::sync::Mutex;

const GGUF_MIN_SIZE: u64 = 4_000_000_000;

/// Check whether the local Saul GGUF model is present and complete.
/// Reuses the OllamaStatus type for compatibility with existing IPC plumbing.
#[tauri::command]
pub async fn check_ollama(
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<OllamaStatus, String> {
    let gguf_path = {
        let s = state.lock().await;
        s.model_dir.join("saul.gguf")
    };

    let ready = gguf_path
        .metadata()
        .map(|m| m.len() > GGUF_MIN_SIZE)
        .unwrap_or(false);

    Ok(OllamaStatus {
        running: ready,
        models: vec![],
        has_llm_model: ready,
        has_embed_model: ready,
        llm_model_name: "Saul-7B-Instruct-v1 (local)".to_string(),
        embed_model_name: "all-MiniLM-L6-v2 (local)".to_string(),
    })
}
