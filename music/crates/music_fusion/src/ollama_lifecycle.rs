//! Automatic Ollama Detection, Silent Installation, Daemon Startup,
//! and Dynamic Available Memory Model Sizing Engine.
//!
//! Strictly complies with:
//! 1. The Dynamic Available Memory Law (Anti-OOM): sizing based strictly on runtime
//!    free RAM and free GPU VRAM, never total physical RAM, with zero `-coder` suffixes.
//! 2. The Universal Zero-Touch PATH & Environment Protection Protocol: process-scoped
//!    `PATH` updates only (`std::env::set_var("PATH", ...)`), never mutating system/user registry.
//! 3. Fast 400ms timeout health probes to prevent UI and server freezes.

use std::path::{Path, PathBuf};
use std::time::Duration;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::memory::{create_hidden_command, SystemMemory};

/// Live operational status of the Ollama runtime and active models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OllamaStatus {
    pub running: bool,
    pub installed: bool,
    pub endpoint: String,
    pub recommended_model: String,
    pub installed_models: Vec<String>,
    pub free_ram_gb: f64,
    pub free_vram_mb: f64,
    pub install_path: Option<String>,
    pub message: String,
}

/// Tags response contract from Ollama `/api/tags`.
#[derive(Debug, Clone, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaTagItem {
    name: String,
}

/// Ollama Lifecycle and Memory Sizing Manager.
pub struct OllamaLifecycleManager {
    endpoint: String,
    probe_client: Client,
    http_client: Client,
}

impl Default for OllamaLifecycleManager {
    fn default() -> Self {
        Self::new("http://127.0.0.1:11434")
    }
}

impl OllamaLifecycleManager {
    /// Construct a manager for the given Ollama HTTP endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        let ep = endpoint.into().trim_end_matches('/').to_string();

        let probe_client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_millis(400))
            .timeout(Duration::from_millis(400))
            .build()
            .unwrap_or_default();

        let http_client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: ep,
            probe_client,
            http_client,
        }
    }

    /// Fast health probe with a strict 400ms timeout.
    pub async fn probe(&self) -> bool {
        let url = format!("{}/api/tags", self.endpoint);
        let fut = self.probe_client.get(&url).send();
        match tokio::time::timeout(Duration::from_millis(400), fut).await {
            Ok(Ok(resp)) => resp.status().is_success(),
            _ => false,
        }
    }

    /// Sizing models strictly evaluating runtime free/available memory, NEVER total physical RAM.
    /// - Free RAM >= 48 GB OR Free VRAM >= 24 GB -> `qwen2.5:32b`
    /// - Free RAM >= 24 GB OR Free VRAM >= 14 GB -> `qwen2.5:14b`
    /// - Free RAM >= 12 GB OR Free VRAM >= 4.5 GB -> `qwen2.5:7b`
    /// - Free RAM >= 6 GB OR Free VRAM >= 2 GB -> `qwen2.5:3b`
    /// - Free RAM < 6 GB -> `qwen2.5:1.5b`
    ///
    /// Never appends `-coder` suffixes that trigger silent 404 aborts.
    pub fn size_model_for_memory(free_ram_gb: f64, free_vram_mb: f64) -> String {
        let free_vram_gb = free_vram_mb / 1024.0;
        if free_ram_gb >= 48.0 || free_vram_gb >= 24.0 {
            "qwen2.5:32b".to_string()
        } else if free_ram_gb >= 24.0 || free_vram_gb >= 14.0 {
            "qwen2.5:14b".to_string()
        } else if free_ram_gb >= 12.0 || free_vram_gb >= 4.5 {
            "qwen2.5:7b".to_string()
        } else if free_ram_gb >= 6.0 || free_vram_gb >= 2.0 {
            "qwen2.5:3b".to_string()
        } else {
            "qwen2.5:1.5b".to_string()
        }
    }

    /// Locate `ollama.exe` / `ollama` executable on the host system.
    pub fn find_ollama_binary() -> Option<PathBuf> {
        // 1. Process PATH check
        if let Ok(path_var) = std::env::var("PATH") {
            for entry in std::env::split_paths(&path_var) {
                let candidate = entry.join("ollama.exe");
                if candidate.is_file() {
                    return Some(candidate);
                }
                let candidate_unix = entry.join("ollama");
                if candidate_unix.is_file() {
                    return Some(candidate_unix);
                }
            }
        }

        // 2. Standard Windows installation paths
        #[cfg(windows)]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let candidate = PathBuf::from(local_app_data).join("Programs").join("Ollama").join("ollama.exe");
                if candidate.is_file() {
                    Self::prepend_to_process_path(&candidate);
                    return Some(candidate);
                }
            }

            if let Ok(prog_files) = std::env::var("ProgramFiles") {
                let candidate = PathBuf::from(prog_files).join("Ollama").join("ollama.exe");
                if candidate.is_file() {
                    Self::prepend_to_process_path(&candidate);
                    return Some(candidate);
                }
            }

            let fallback_candidates = [
                r"C:\Users\oyesanyf\AppData\Local\Programs\Ollama\ollama.exe",
                r"C:\Program Files\Ollama\ollama.exe",
            ];
            for cand in &fallback_candidates {
                let p = PathBuf::from(cand);
                if p.is_file() {
                    Self::prepend_to_process_path(&p);
                    return Some(p);
                }
            }
        }

        None
    }

    /// Add directory to process-scoped `PATH` only (strictly obeying Zero-Touch PATH Law).
    fn prepend_to_process_path(bin_path: &Path) {
        if let Some(parent) = bin_path.parent() {
            if let Ok(curr_path) = std::env::var("PATH") {
                let parent_str = parent.to_string_lossy();
                if !curr_path.contains(&*parent_str) {
                    #[cfg(windows)]
                    let new_path = format!("{};{}", parent_str, curr_path);
                    #[cfg(not(windows))]
                    let new_path = format!("{}:{}", parent_str, curr_path);

                    std::env::set_var("PATH", new_path);
                    log::info!("Prepended {} to process-scoped PATH", parent.display());
                }
            }
        }
    }

    /// Ensure Ollama is installed on the system; installs silently via winget or setup binary if missing.
    pub async fn ensure_installed() -> Result<Option<PathBuf>> {
        if let Some(bin) = Self::find_ollama_binary() {
            return Ok(Some(bin));
        }

        log::warn!("Ollama binary not found on system. Initiating silent installation...");

        #[cfg(windows)]
        {
            // 1. Try silent winget install with CREATE_NO_WINDOW
            log::info!("Attempting winget install Ollama.Ollama --silent...");
            let winget_res = create_hidden_command("winget")
                .args([
                    "install",
                    "--id",
                    "Ollama.Ollama",
                    "-e",
                    "--silent",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .status();

            if let Ok(status) = winget_res {
                if status.success() {
                    log::info!("Winget silent install completed successfully.");
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    if let Some(bin) = Self::find_ollama_binary() {
                        return Ok(Some(bin));
                    }
                }
            }

            // 2. Fallback: Download OllamaSetup.exe to %TEMP% and run /SILENT /NORESTART
            log::info!("Winget unavailable or failed; downloading official OllamaSetup.exe...");
            let temp_dir = std::env::temp_dir();
            let installer_path = temp_dir.join("OllamaSetup.exe");

            let dl_client = Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_default();

            let resp = dl_client.get("https://ollama.com/download/OllamaSetup.exe").send().await;
            if let Ok(r) = resp {
                if r.status().is_success() {
                    if let Ok(bytes) = r.bytes().await {
                        let _ = std::fs::write(&installer_path, bytes);
                        if installer_path.exists() {
                            let _ = create_hidden_command(&installer_path)
                                .args(["/SILENT", "/NORESTART"])
                                .status();
                            let _ = std::fs::remove_file(&installer_path);
                            tokio::time::sleep(Duration::from_millis(2000)).await;
                            if let Some(bin) = Self::find_ollama_binary() {
                                return Ok(Some(bin));
                            }
                        }
                    }
                }
            }
        }

        Ok(Self::find_ollama_binary())
    }

    /// Spawns `ollama serve` background daemon with CREATE_NO_WINDOW if not already running.
    pub async fn ensure_daemon_running(&self) -> Result<bool> {
        if self.probe().await {
            return Ok(true);
        }

        let bin = match Self::find_ollama_binary() {
            Some(b) => b,
            None => match Self::ensure_installed().await? {
                Some(b) => b,
                None => {
                    log::warn!("Could not start Ollama: executable not found and auto-install was unavailable.");
                    return Ok(false);
                }
            },
        };

        log::info!("Spawning background Ollama daemon via {}...", bin.display());
        let spawn_res = create_hidden_command(&bin)
            .arg("serve")
            .spawn();

        if spawn_res.is_err() {
            log::error!("Failed to spawn Ollama daemon process");
            return Ok(false);
        }

        // Wait up to 3.5 seconds with 250ms fast polling for probe to become active
        for _ in 0..14 {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if self.probe().await {
                log::info!("Ollama daemon successfully booted and responding at {}", self.endpoint);
                return Ok(true);
            }
        }

        Ok(self.probe().await)
    }

    /// List installed models from `/api/tags`.
    pub async fn list_installed_tags(&self) -> Vec<String> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self.probe_client.get(&url).send().await;
        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(tags) = r.json::<OllamaTagsResponse>().await {
                    return tags.models.into_iter().map(|m| m.name.to_lowercase()).collect();
                }
            }
        }
        Vec::new()
    }

    /// Trigger background pull of missing model.
    pub async fn trigger_model_pull_background(&self, model: &str) {
        log::info!("Triggering background pull for Ollama model: {}", model);
        let url = format!("{}/api/pull", self.endpoint);
        let body = serde_json::json!({
            "name": model,
            "stream": false,
        });

        let client = self.http_client.clone();
        tokio::spawn(async move {
            let _ = client.post(&url).json(&body).send().await;
        });
    }

    /// Comprehensive bootstrap: probes daemon, installs/starts if needed, calculates memory sizing,
    /// checks installed models, and triggers pull if recommended model is missing.
    pub async fn bootstrap(&self, mem: &SystemMemory) -> Result<OllamaStatus> {
        let binary_path = Self::find_ollama_binary();
        let installed = binary_path.is_some();

        // Calculate model sizing per the Dynamic Available Memory Law
        let free_vram_mb = mem.free_vram_mb();
        let recommended_model = Self::size_model_for_memory(mem.free_ram_gb, free_vram_mb);

        let mut running = self.probe().await;
        if !running && installed {
            let _ = self.ensure_daemon_running().await;
            running = self.probe().await;
        }

        let mut installed_models = Vec::new();
        let message: String;

        if running {
            installed_models = self.list_installed_tags().await;
            let clean_rec = recommended_model.split(':').next().unwrap_or(&recommended_model);
            let has_rec = installed_models.iter().any(|t| t == &recommended_model || t.contains(clean_rec));

            if !has_rec && !installed_models.is_empty() {
                // Trigger background pull for recommended model
                self.trigger_model_pull_background(&recommended_model).await;
                message = format!(
                    "Ollama active with {} models. Triggered background pull for recommended {}",
                    installed_models.len(),
                    recommended_model
                );
            } else if has_rec {
                message = format!("Ollama active. Recommended model {} is ready.", recommended_model);
            } else {
                self.trigger_model_pull_background(&recommended_model).await;
                message = format!("Ollama active. Auto-pulling {} in background.", recommended_model);
            }
        } else if installed {
            message = "Ollama installed on disk but daemon is offline.".to_string();
        } else {
            message = "Ollama not installed. Click 'Bootstrap Ollama' to install silently.".to_string();
        }

        Ok(OllamaStatus {
            running,
            installed,
            endpoint: self.endpoint.clone(),
            recommended_model,
            installed_models,
            free_ram_gb: mem.free_ram_gb,
            free_vram_mb,
            install_path: binary_path.map(|p| p.to_string_lossy().to_string()),
            message,
        })
    }
}
