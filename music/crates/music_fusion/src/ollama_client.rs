//! Resilient HTTP client for Ollama at 127.0.0.1:11434 with fast timeout probes,
//! dynamic model tag discovery, interactive missing model guidance, and zero-crash fallbacks.

use crate::error::{FusionError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(0.9),
            num_predict: Some(4096),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTag {
    pub name: String,
    #[serde(default)]
    pub modified_at: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<ModelTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ChatMessage,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
}

/// Interactive guidance card when a requested Ollama model is missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingModelGuidance {
    pub missing_model: String,
    pub pull_command: String,
    pub installed_models: Vec<String>,
    pub recommended_fallback: Option<String>,
}

/// Resilient HTTP client for local Ollama daemon.
#[derive(Clone)]
pub struct OllamaClient {
    pub endpoint: String,
    pub client: Client,
    pub probe_client: Client,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new("http://127.0.0.1:11434")
    }
}

impl OllamaClient {
    /// Construct a new client for the given endpoint with connection timeouts and proxy bypass.
    pub fn new(endpoint: impl Into<String>) -> Self {
        let ep = endpoint.into().trim_end_matches('/').to_string();

        let client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(180))
            .build()
            .unwrap_or_default();

        // 400ms fast probe client for health checks per the resilience protocol
        let probe_client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_millis(400))
            .timeout(Duration::from_millis(400))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: ep,
            client,
            probe_client,
        }
    }

    /// From environment or default.
    pub fn from_env() -> Self {
        let ep = std::env::var("LOCAL_OLLAMA_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        Self::new(ep)
    }

    /// Fast health probe with a strict 400ms timeout to prevent UI/pipeline freezes.
    pub async fn probe_health(&self) -> bool {
        let url = format!("{}/api/tags", self.endpoint);
        let fut = self.probe_client.get(&url).send();
        match tokio::time::timeout(Duration::from_millis(400), fut).await {
            Ok(Ok(resp)) => resp.status().is_success(),
            _ => false,
        }
    }

    /// Fetch list of installed models from `/api/tags`.
    pub async fn list_models(&self) -> Result<Vec<ModelTag>> {
        let url = format!("{}/api/tags", self.endpoint);
        let res = self.client.get(&url).send().await
            .map_err(|e| FusionError::OllamaUnreachable {
                endpoint: self.endpoint.clone(),
                source: e,
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let err_txt = res.text().await.unwrap_or_default();
            return Err(FusionError::OllamaApi { status, message: err_txt });
        }

        let parsed: TagsResponse = res.json().await?;
        Ok(parsed.models)
    }

    /// Fetch installed tags as lowercase string slice.
    pub async fn list_installed_tags(&self) -> Result<Vec<String>> {
        let models = self.list_models().await?;
        Ok(models.into_iter().map(|m| m.name.to_lowercase()).collect())
    }

    /// Fetch installed models as tag string list (alias for list_installed_tags).
    pub async fn list_installed_models(&self) -> Result<Vec<String>> {
        self.list_installed_tags().await
    }

    /// Execute a chat completion on `/api/chat`.
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: Option<ChatOptions>,
    ) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.endpoint);
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": options.unwrap_or_default(),
        });

        let res = self.client.post(&url).json(&body).send().await
            .map_err(|e| FusionError::OllamaUnreachable {
                endpoint: self.endpoint.clone(),
                source: e,
            })?;

        if res.status() == reqwest::StatusCode::NOT_FOUND {
            let tags = self.list_installed_tags().await.unwrap_or_default();
            return Err(FusionError::ModelNotFound {
                model: model.to_string(),
                available: tags,
            });
        }

        if !res.status().is_success() {
            let status = res.status();
            let err_txt = res.text().await.unwrap_or_default();
            return Err(FusionError::OllamaApi { status, message: err_txt });
        }

        let resp: ChatResponse = res.json().await?;
        Ok(resp)
    }

    /// Simple helper to execute a system + user prompt chat request.
    pub async fn simple_chat(
        &self,
        model: &str,
        system: &str,
        user: &str,
        temperature: f32,
    ) -> Result<String> {
        let options = ChatOptions {
            temperature: Some(temperature),
            ..Default::default()
        };
        let messages = vec![
            ChatMessage::system(system),
            ChatMessage::user(user),
        ];
        let resp = self.chat(model, messages, Some(options)).await?;
        Ok(resp.message.content)
    }

    /// Generate interactive missing model guidance card.
    pub fn get_missing_model_guidance(&self, missing: &str, installed: &[String]) -> MissingModelGuidance {
        let clean = missing.split(':').next().unwrap_or(missing);
        let fallback = installed.iter().find(|t| t.contains(clean)).cloned();
        MissingModelGuidance {
            missing_model: missing.to_string(),
            pull_command: format!("ollama pull {}", missing),
            installed_models: installed.to_vec(),
            recommended_fallback: fallback,
        }
    }

    /// Trigger model pull via `/api/pull`.
    pub async fn pull_model(&self, model: &str) -> Result<()> {
        let url = format!("{}/api/pull", self.endpoint);
        let body = serde_json::json!({
            "name": model,
            "stream": false,
        });

        let res = self.client.post(&url).json(&body).send().await
            .map_err(|e| FusionError::OllamaUnreachable {
                endpoint: self.endpoint.clone(),
                source: e,
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FusionError::OllamaApi { status, message: text });
        }

        Ok(())
    }
}
