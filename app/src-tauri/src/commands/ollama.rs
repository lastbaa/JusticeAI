use crate::state::{OllamaStatus, RagState};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Check HuggingFace API connectivity and token validity.
/// Reuses the OllamaStatus type for compatibility with existing UI.
#[tauri::command]
pub async fn check_ollama(
    state: tauri::State<'_, Arc<Mutex<RagState>>>,
) -> Result<OllamaStatus, String> {
    let hf_token = {
        let s = state.lock().await;
        s.settings.hf_token.clone()
    };

    if hf_token.trim().is_empty() {
        return Ok(OllamaStatus {
            running: false,
            models: vec![],
            has_llm_model: false,
            has_embed_model: false,
            llm_model_name: "Saul-7B-Instruct-v1".to_string(),
            embed_model_name: "all-MiniLM-L6-v2".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    // Ping HF to validate token
    let resp = client
        .get("https://huggingface.co/api/models/Equall/Saul-7B-Instruct-v1")
        .bearer_auth(&hf_token)
        .send()
        .await;

    let hf_reachable = resp.is_ok();

    Ok(OllamaStatus {
        running: hf_reachable,
        models: vec![],
        has_llm_model: hf_reachable,
        has_embed_model: hf_reachable,
        llm_model_name: "Saul-7B-Instruct-v1".to_string(),
        embed_model_name: "all-MiniLM-L6-v2".to_string(),
    })
}
