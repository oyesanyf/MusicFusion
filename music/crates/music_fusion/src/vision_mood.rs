//! Local Open-Weight Vision Mood & Cover-Art Prompt Adapter via Ollama (`llava`, `gemma4`, `qwen2.5vl`).
//! Strictly 100% local (`127.0.0.1:11434`) — zero external cloud vision API calls.

use crate::ollama_client::OllamaClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionMoodSuggestion {
    pub model_used: String,
    pub style_prompt_tags: String,
    pub detected_mood: String,
}

/// Inspects an optional base64-encoded image using an installed local multimodal Ollama model.
/// Falls back cleanly to prompt keywords when offline or when no vision tag is installed.
pub async fn analyze_visual_mood_local(
    client: &OllamaClient,
    image_base64: Option<&str>,
    fallback_hint: &str,
) -> VisionMoodSuggestion {
    if let Some(b64) = image_base64.filter(|s| !s.trim().is_empty()) {
        if client.probe_health().await {
            let installed = client.list_installed_tags().await.unwrap_or_default();
            let vision_tag = ["gemma4", "qwen2.5vl", "llava", "moondream"]
                .iter()
                .find_map(|pat| installed.iter().find(|t| t.contains(pat)).cloned());

            if let Some(model) = vision_tag {
                let url = format!("{}/api/chat", client.endpoint);
                let body = serde_json::json!({
                    "model": model,
                    "stream": false,
                    "keep_alive": "0",
                    "messages": [{
                        "role": "user",
                        "content": "Describe musical mood, instrumentation tags, and tempo impression inspired by this artwork in 12 concise comma-separated tags.",
                        "images": [b64],
                    }]
                });
                if let Ok(resp) = client.client.post(&url).json(&body).send().await {
                    if let Ok(val) = resp.json::<serde_json::Value>().await {
                        if let Some(txt) = val.pointer("/message/content").and_then(|v| v.as_str()) {
                            let clean = txt.trim().to_string();
                            if !clean.is_empty() {
                                return VisionMoodSuggestion {
                                    model_used: model,
                                    style_prompt_tags: clean.clone(),
                                    detected_mood: clean,
                                };
                            }
                        }
                    }
                }
            }
        }
    }

    let hint = if fallback_hint.trim().is_empty() {
        "warm atmospheric studio texture, expressive dynamic phrasing".to_string()
    } else {
        fallback_hint.trim().to_string()
    };
    VisionMoodSuggestion {
        model_used: "LocalHeuristicMoodAdapter".to_string(),
        style_prompt_tags: hint.clone(),
        detected_mood: hint,
    }
}
