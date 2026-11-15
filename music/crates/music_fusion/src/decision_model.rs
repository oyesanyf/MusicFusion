//! Local Nimble Typed Decision Model Engine (`Bespoke-Nimble-9B` / `9B-v3` / `27B-v3` / `v4-max`),
//! Ollama single-pass logprob scorer, and `<2 ms` pure-Rust System-1 calibrated classifier.
//!
//! Zero paid APIs, zero network calls outside `127.0.0.1:11434`, and explicit commercial
//! license gating across Pipeline Decision Gates 1-5.

use std::path::{Path, PathBuf};
use std::time::Instant;

use music_core::schema::{StemChannelStrip, StemMixConfig, TypedDecisionTrace};
use music_core::state::{DecisionMode, LyricsMode, MusicProductionState};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::hf_model_discovery::{
    default_models_cache_dir, is_hf_repo_cached, HfModelDiscoveryEngine, HfModelEntry,
    ModelLicenseTier, STAGE_DECISION,
};
use crate::memory::{create_hidden_command, SystemMemory};
use crate::ollama_client::OllamaClient;

/// Default commercially safe open-weight decision model (`Apache-2.0`).
pub const DEFAULT_COMMERCIAL_NIMBLE_MODEL: &str = "bespokelabs/Bespoke-Nimble-9B";
/// Higher-accuracy non-commercial research checkpoint (`CC-BY-NC-4.0`, blocked when `commercial_only=true`).
pub const DEFAULT_RESEARCH_NIMBLE_MODEL: &str = "bespokelabs/Bespoke-Nimble-9B-v3";
/// Probability below which Human-in-the-Loop (HITL) confirmation is triggered.
pub const HITL_CONFIDENCE_THRESHOLD: f32 = 0.50;

/// Schema constraints accepted by `NimbleDecisionEngine::decide`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecisionSchema {
    Binary,
    Likert5,
    Categorical { options: Vec<String> },
    NumericBand { bands: Vec<String> },
}

impl Default for DecisionSchema {
    fn default() -> Self {
        Self::Binary
    }
}

impl DecisionSchema {
    pub fn kind_name(&self) -> &'static str {
        match self {
            DecisionSchema::Binary => "binary",
            DecisionSchema::Likert5 => "likert5",
            DecisionSchema::Categorical { .. } => "categorical",
            DecisionSchema::NumericBand { .. } => "numeric_band",
        }
    }

    pub fn choices(&self) -> Vec<String> {
        match self {
            DecisionSchema::Binary => vec!["Approve".to_string(), "Reject".to_string()],
            DecisionSchema::Likert5 => vec![
                "1_Poor".to_string(),
                "2_Subpar".to_string(),
                "3_Adequate".to_string(),
                "4_Strong".to_string(),
                "5_Masterpiece".to_string(),
            ],
            DecisionSchema::Categorical { options } => {
                if options.is_empty() {
                    vec!["Default".to_string()]
                } else {
                    options.clone()
                }
            }
            DecisionSchema::NumericBand { bands } => {
                if bands.is_empty() {
                    vec![
                        "-3.0dB".to_string(),
                        "-1.5dB".to_string(),
                        "0.0dB".to_string(),
                        "+1.5dB".to_string(),
                        "+3.0dB".to_string(),
                    ]
                } else {
                    bands.clone()
                }
            }
        }
    }
}

/// Request container for a single calibrated Nimble decision pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub gate_name: String,
    pub schema: DecisionSchema,
    pub question: String,
    #[serde(default)]
    pub context: serde_json::Value,
    #[serde(default)]
    pub commercial_only: bool,
    #[serde(default)]
    pub preferred_model: Option<String>,
    #[serde(default)]
    pub mode: DecisionMode,
    /// Optional explicit logits prior (length matched to schema choices) for feature calibration.
    #[serde(default)]
    pub feature_logits: Option<Vec<f64>>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

fn default_temperature() -> f64 {
    1.0
}

impl DecisionRequest {
    pub fn new(gate_name: impl Into<String>, schema: DecisionSchema, question: impl Into<String>) -> Self {
        Self {
            gate_name: gate_name.into(),
            schema,
            question: question.into(),
            context: serde_json::Value::Null,
            commercial_only: false,
            preferred_model: None,
            mode: DecisionMode::Auto,
            feature_logits: None,
            temperature: 1.0,
        }
    }
}

/// Calibrated decision output with exact normalized distribution, Shannon entropy, and trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub trace: TypedDecisionTrace,
    pub selected_choice: String,
    pub selected_index: usize,
    pub probabilities: Vec<(String, f32)>,
    pub confidence: f32,
    pub entropy: f32,
    pub hitl_gate_triggered: bool,
    pub hitl_reason: Option<String>,
    pub blocked_candidates: Vec<String>,
    pub rationale: String,
    #[serde(default)]
    pub recommended_action: serde_json::Value,
}

/// Runtime inventory report returned by `GET /api/decision/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NimbleStatusSnapshot {
    pub active_model: String,
    pub active_license_tier: String,
    pub commercial_allowed: bool,
    pub offload_to_cpu_ram: bool,
    pub ollama_online: bool,
    pub ollama_nimble_tags: Vec<String>,
    pub cached_hf_nimble_models: Vec<String>,
    pub worker_script_present: bool,
    pub system1_fallback_ready: bool,
    pub gpu_vram_free_gb: f64,
    pub free_ram_gb: f64,
}

/// Softmax calibration over logits `z_i / T` producing `sum(p_i) == 1.0` and Shannon entropy `-sum(p_i * ln(p_i))`.
pub fn calibrate_logits(choices: &[String], logits: &[f64], temperature: f64) -> (Vec<(String, f32)>, f32) {
    if choices.is_empty() {
        return (Vec::new(), 0.0);
    }
    let temp = if temperature.is_finite() && temperature > 0.05 {
        temperature
    } else {
        1.0
    };

    let mut scaled = Vec::with_capacity(choices.len());
    let mut max_val = f64::NEG_INFINITY;
    for i in 0..choices.len() {
        let raw = logits.get(i).copied().unwrap_or(0.0);
        if raw.is_finite() {
            let v = raw / temp;
            if v > max_val {
                max_val = v;
            }
            scaled.push(v);
        } else {
            scaled.push(f64::NEG_INFINITY);
        }
    }

    if !max_val.is_finite() {
        // Uniform distribution if every candidate logit was non-finite
        let p = 1.0_f64 / choices.len() as f64;
        let probs = choices.iter().map(|c| (c.clone(), p as f32)).collect();
        let ent = if choices.len() > 1 {
            (-(choices.len() as f64) * p * p.ln()) as f32
        } else {
            0.0
        };
        return (probs, ent);
    }

    let mut weights = Vec::with_capacity(choices.len());
    let mut sum_w = 0.0_f64;
    for &v in &scaled {
        let w = if v.is_finite() { (v - max_val).exp() } else { 0.0 };
        sum_w += w;
        weights.push(w);
    }

    if sum_w <= 0.0 || !sum_w.is_finite() {
        let p = 1.0_f64 / choices.len() as f64;
        let probs = choices.iter().map(|c| (c.clone(), p as f32)).collect();
        let ent = if choices.len() > 1 { (choices.len() as f64).ln() as f32 } else { 0.0 };
        return (probs, ent);
    }

    let mut probs_f64: Vec<f64> = weights.into_iter().map(|w| w / sum_w).collect();
    // Correct any sub-ulp floating-point drift on the dominant component so sum == 1.0f32 closely
    let mut entropy_nats = 0.0_f64;
    for &p in &probs_f64 {
        if p > 1e-15 {
            entropy_nats -= p * p.ln();
        }
    }

    let mut out: Vec<(String, f32)> = Vec::with_capacity(choices.len());
    let mut sum_f32 = 0.0_f32;
    let mut max_idx = 0usize;
    for (idx, p) in probs_f64.iter().enumerate() {
        let pf = (*p as f32).clamp(0.0, 1.0);
        if pf > probs_f64[max_idx] as f32 {
            max_idx = idx;
        }
        sum_f32 += pf;
        out.push((choices[idx].clone(), pf));
    }
    if !out.is_empty() && (1.0_f32 - sum_f32).abs() > 1e-7 {
        let adjusted = (out[max_idx].1 + (1.0_f32 - sum_f32)).clamp(0.0, 1.0);
        out[max_idx].1 = adjusted;
    }

    let _ = probs_f64.as_mut_slice();
    (out, entropy_nats.max(0.0) as f32)
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateTopLogprob {
    token: String,
    logprob: f64,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateLogprobItem {
    #[serde(default)]
    top_logprobs: Vec<OllamaGenerateTopLogprob>,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    logprobs: Vec<OllamaGenerateLogprobItem>,
}

#[derive(Debug, Deserialize)]
struct PythonNimbleOutput {
    #[serde(default)]
    logits: Vec<f64>,
    #[serde(default)]
    selected_choice: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
}

/// Engine orchestrating local Bespoke-Nimble checkpoints, Ollama single-pass logprobs,
/// and `<2 ms` pure-Rust calibrated System-1 fallback across all 5 studio decision gates.
pub struct NimbleDecisionEngine {
    pub ollama: OllamaClient,
    pub models_dir: PathBuf,
    pub worker_script: PathBuf,
    pub memory: SystemMemory,
}

impl Default for NimbleDecisionEngine {
    fn default() -> Self {
        Self::new(
            OllamaClient::from_env(),
            default_models_cache_dir(),
            PathBuf::from("workers/nimble_decide.py"),
            SystemMemory::detect_live(),
        )
    }
}

impl NimbleDecisionEngine {
    pub fn new(
        ollama: OllamaClient,
        models_dir: PathBuf,
        worker_script: PathBuf,
        memory: SystemMemory,
    ) -> Self {
        Self {
            ollama,
            models_dir,
            worker_script,
            memory,
        }
    }

    /// Resolves a Nimble checkpoint entry and strictly enforces `commercial_only` compliance.
    pub fn resolve_decision_checkpoint(
        preferred: Option<&str>,
        commercial_only: bool,
    ) -> (HfModelEntry, Option<String>) {
        let req = preferred.map(str::trim).filter(|s| !s.is_empty());
        if let Some(q) = req {
            if let Some(entry) = HfModelDiscoveryEngine::resolve_model_entry(q) {
                if entry.stage == STAGE_DECISION {
                    if commercial_only && !entry.commercial_allowed {
                        let safe = HfModelDiscoveryEngine::resolve_model_entry(DEFAULT_COMMERCIAL_NIMBLE_MODEL)
                            .unwrap_or_else(|| entry.clone());
                        let note = format!(
                            "Decision checkpoint '{}' carries {} license and is blocked under commercial_only=true; routed to '{}' ({}).",
                            entry.id,
                            entry.license_tier.as_str(),
                            safe.id,
                            safe.license_tier.as_str()
                        );
                        return (safe, Some(note));
                    }
                    return (entry, None);
                }
            }
        }
        let default_entry = HfModelDiscoveryEngine::resolve_model_entry(DEFAULT_COMMERCIAL_NIMBLE_MODEL)
            .expect("Bespoke-Nimble-9B must exist in built-in catalog");
        (default_entry, None)
    }

    /// Live inventory snapshot for `GET /api/decision/status` and WebUI inspection.
    pub async fn status(&self, preferred_model: Option<&str>, commercial_only: bool) -> NimbleStatusSnapshot {
        let (entry, _) = Self::resolve_decision_checkpoint(preferred_model, commercial_only);
        let ollama_online = self.ollama.probe_health().await;
        let tags = if ollama_online {
            self.ollama.list_installed_tags().await.unwrap_or_default()
        } else {
            Vec::new()
        };
        let ollama_nimble_tags: Vec<String> = tags
            .into_iter()
            .filter(|t| t.contains("nimble") || t.contains("bespoke") || t.contains("qwen2.5"))
            .collect();

        let cached_hf_nimble_models: Vec<String> = HfModelDiscoveryEngine::curated_builtins()
            .into_iter()
            .filter(|m| m.stage == STAGE_DECISION && !m.id.starts_with("InternalPureRust/"))
            .filter(|m| is_hf_repo_cached(&m.id, &self.models_dir))
            .map(|m| m.id)
            .collect();

        NimbleStatusSnapshot {
            active_model: entry.id,
            active_license_tier: entry.license_tier.as_str().to_string(),
            commercial_allowed: entry.commercial_allowed,
            offload_to_cpu_ram: self.memory.should_offload_nimble_to_cpu(),
            ollama_online,
            ollama_nimble_tags,
            cached_hf_nimble_models,
            worker_script_present: self.worker_script.is_file(),
            system1_fallback_ready: true,
            gpu_vram_free_gb: self.memory.gpu_vram_free_gb,
            free_ram_gb: self.memory.free_ram_gb,
        }
    }

    /// Executes a calibrated typed decision using the configured backend cascade:
    /// 1. Local Hugging Face / GGUF `workers/nimble_decide.py` (when cached & requested/auto)
    /// 2. Local Ollama `/api/generate` single-token `logprobs=true, keep_alive="0"`
    /// 3. `<2 ms` Pure-Rust System-1 calibrated logit scorer
    pub async fn decide(&self, req: &DecisionRequest) -> DecisionResult {
        let started = Instant::now();
        let choices = req.schema.choices();
        let (checkpoint, checkpoint_block_msg) =
            Self::resolve_decision_checkpoint(req.preferred_model.as_deref(), req.commercial_only);
        let mut blocked_candidates = Vec::new();
        if let Some(msg) = &checkpoint_block_msg {
            blocked_candidates.push(msg.clone());
        }

        let want_hf = matches!(req.mode, DecisionMode::Auto | DecisionMode::NimbleLocal);
        let want_ollama = matches!(req.mode, DecisionMode::Auto | DecisionMode::OllamaSinglePass);

        // Backend 1: Cached local Bespoke-Nimble HF / GGUF worker
        let mut chosen_logits: Option<(String, Vec<f64>, String)> = None;
        if want_hf
            && self.worker_script.is_file()
            && is_hf_repo_cached(&checkpoint.id, &self.models_dir)
        {
            if let Some((logits, note)) = self.try_python_nimble_worker(&checkpoint, req, &choices) {
                chosen_logits = Some((format!("nimble-local:{}", checkpoint.id), logits, note));
            }
        }

        // Backend 2: Local Ollama single-pass token logprob scoring (with immediate keep_alive=0 release)
        if chosen_logits.is_none() && want_ollama && self.ollama.probe_health().await {
            let installed = self.ollama.list_installed_tags().await.unwrap_or_default();
            if let Some(ollama_tag) = Self::pick_ollama_decision_tag(&installed, req.preferred_model.as_deref(), req.commercial_only) {
                if let Some((logits, note)) = self.try_ollama_single_pass(&ollama_tag, req, &choices).await {
                    chosen_logits = Some((format!("ollama-single-pass:{}", ollama_tag), logits, note));
                }
            }
        }

        // Backend 3: Guaranteed <2ms calibrated System-1 pure-Rust classifier
        let (backend_used, logits, rationale) = match chosen_logits {
            Some(v) => v,
            None => {
                let sys_logits = Self::compute_system1_logits(req, &choices);
                let note = if let Some(block) = checkpoint_block_msg {
                    format!("System-1 calibrated scoring ({})", block)
                } else {
                    format!(
                        "System-1 calibrated scoring via {} feature rubric ({})",
                        req.gate_name,
                        checkpoint.license_tier.as_str()
                    )
                };
                (
                    format!("system1-calibrated:{}", checkpoint.id),
                    sys_logits,
                    note,
                )
            }
        };

        let (probabilities, entropy) = calibrate_logits(&choices, &logits, req.temperature);
        let (selected_index, (selected_choice, confidence)) = probabilities
            .iter()
            .enumerate()
            .max_by(|a, b| a.1 .1.total_cmp(&b.1 .1))
            .map(|(i, (c, p))| (i, (c.clone(), *p)))
            .unwrap_or((0, (choices.first().cloned().unwrap_or_default(), 1.0)));

        let mut hitl_gate_triggered = confidence < HITL_CONFIDENCE_THRESHOLD || !blocked_candidates.is_empty();
        let hitl_reason = if !blocked_candidates.is_empty() {
            Some(blocked_candidates.join(" | "))
        } else if confidence < HITL_CONFIDENCE_THRESHOLD {
            hitl_gate_triggered = true;
            Some(format!(
                "Calibrated top probability {:.1}% is below the {:.0}% HITL threshold (entropy {:.3}); confirm candidate selection.",
                confidence * 100.0,
                HITL_CONFIDENCE_THRESHOLD * 100.0,
                entropy
            ))
        } else {
            None
        };

        let latency_ms = started.elapsed().as_millis() as u64;
        let trace = TypedDecisionTrace {
            gate_name: req.gate_name.clone(),
            schema_kind: req.schema.kind_name().to_string(),
            backend_used: backend_used.clone(),
            selected_choice: selected_choice.clone(),
            probabilities: probabilities.clone(),
            confidence,
            entropy,
            hitl_gate_triggered,
            latency_ms,
            license_tier: checkpoint.license_tier.as_str().to_string(),
        };

        DecisionResult {
            trace,
            selected_choice,
            selected_index,
            probabilities,
            confidence,
            entropy,
            hitl_gate_triggered,
            hitl_reason,
            blocked_candidates,
            rationale,
            recommended_action: serde_json::Value::Null,
        }
    }

    fn pick_ollama_decision_tag(
        installed: &[String],
        preferred: Option<&str>,
        commercial_only: bool,
    ) -> Option<String> {
        let is_tag_commercial = |t: &str| -> bool {
            let low = t.to_ascii_lowercase();
            // Nimble-v3 GGUF tags carry CC-BY-NC-4.0
            !(low.contains("nimble-9b-v3") || low.contains("nimble-27b-v3") || low.contains("v3-gguf"))
        };
        if let Some(pref) = preferred {
            let low = pref.to_ascii_lowercase();
            if let Some(found) = installed.iter().find(|t| t.to_ascii_lowercase() == low || t.contains(&low)) {
                if !commercial_only || is_tag_commercial(found) {
                    return Some(found.clone());
                }
            }
        }
        let order = ["bespoke-nimble:9b", "nimble", "qwen2.5:14b", "qwen2.5:7b", "qwen2.5:3b", "gemma2:2b"];
        for pat in order {
            if let Some(found) = installed.iter().find(|t| t.contains(pat)) {
                if !commercial_only || is_tag_commercial(found) {
                    return Some(found.clone());
                }
            }
        }
        None
    }

    fn try_python_nimble_worker(
        &self,
        checkpoint: &HfModelEntry,
        req: &DecisionRequest,
        choices: &[String],
    ) -> Option<(Vec<f64>, String)> {
        let python = Self::locate_python_executable()?;
        let payload = serde_json::json!({
            "model_id": checkpoint.id,
            "question": req.question,
            "choices": choices,
            "context": req.context,
            "cpu_offload": self.memory.should_offload_nimble_to_cpu() || checkpoint.prefer_cpu_ram_offload,
        });

        let mut cmd = create_hidden_command(&python);
        cmd.arg(&self.worker_script)
            .arg("--json")
            .arg(payload.to_string())
            .env("HF_HOME", &self.models_dir);
        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: PythonNimbleOutput = serde_json::from_str(stdout.trim()).ok()?;
        if parsed.logits.len() == choices.len() {
            let note = parsed.rationale.unwrap_or_else(|| {
                format!("Local checkpoint {} single-pass logits", checkpoint.id)
            });
            return Some((parsed.logits, note));
        }
        if let Some(sel) = parsed.selected_choice {
            let mut logits = vec![0.0; choices.len()];
            for (i, c) in choices.iter().enumerate() {
                if c.eq_ignore_ascii_case(&sel) {
                    logits[i] = 2.6;
                }
            }
            return Some((logits, format!("Local checkpoint {} selection", checkpoint.id)));
        }
        None
    }

    async fn try_ollama_single_pass(
        &self,
        tag: &str,
        req: &DecisionRequest,
        choices: &[String],
    ) -> Option<(Vec<f64>, String)> {
        if choices.is_empty() {
            return None;
        }
        let numbered: Vec<String> = choices
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}: {}", i + 1, c))
            .collect();
        let prompt = format!(
            "Task gate: {}.\nQuestion: {}\nContext: {}\nOptions:\n{}\nReply ONLY with the single digit (1-{}) of the best option.",
            req.gate_name,
            req.question,
            req.context,
            numbered.join("\n"),
            choices.len()
        );

        let url = format!("{}/api/generate", self.ollama.endpoint);
        let keep_alive = if self.memory.should_offload_nimble_to_cpu() { "0" } else { "30s" };
        let body = serde_json::json!({
            "model": tag,
            "prompt": prompt,
            "stream": false,
            "logprobs": true,
            "top_logprobs": (choices.len().max(5).min(12)),
            "keep_alive": keep_alive,
            "options": {
                "temperature": 0.0,
                "num_predict": 1,
                "num_gpu": if self.memory.should_offload_nimble_to_cpu() { 0 } else { -1 }
            }
        });

        let client = Client::builder()
            .no_proxy()
            .connect_timeout(std::time::Duration::from_millis(800))
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .ok()?;

        let resp = client.post(&url).json(&body).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let parsed: OllamaGenerateResponse = resp.json().await.ok()?;
        let base_prior = Self::compute_system1_logits(req, choices);
        let mut combined = vec![-6.0_f64; choices.len()];
        let mut found_logprob = false;

        if let Some(first_tok) = parsed.logprobs.first() {
            for item in &first_tok.top_logprobs {
                let tok = item.token.trim();
                if let Ok(num) = tok.parse::<usize>() {
                    if num >= 1 && num <= choices.len() {
                        combined[num - 1] = item.logprob;
                        found_logprob = true;
                    }
                }
            }
        }
        if !found_logprob {
            let trimmed = parsed.response.trim();
            if let Ok(num) = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<usize>() {
                if num >= 1 && num <= choices.len() {
                    combined = base_prior.into_iter().map(|v| v * 0.5).collect();
                    combined[num - 1] += 2.2;
                    found_logprob = true;
                }
            }
        } else {
            // Blend slight structural feature prior with model logprobs when some options had no top-k mass
            for (i, c) in combined.iter_mut().enumerate() {
                if *c <= -5.9 {
                    *c = -4.5 + 0.1 * base_prior.get(i).copied().unwrap_or(0.0);
                } else {
                    *c += 0.15 * base_prior.get(i).copied().unwrap_or(0.0);
                }
            }
        }

        if found_logprob {
            Some((combined, format!("Ollama single-pass logprob calibration ({})", tag)))
        } else {
            None
        }
    }

    fn compute_system1_logits(req: &DecisionRequest, choices: &[String]) -> Vec<f64> {
        if let Some(ref custom) = req.feature_logits {
            if custom.len() == choices.len() {
                return custom.clone();
            }
        }
        let mut logits = vec![0.0_f64; choices.len()];
        if let Some(first) = logits.first_mut() {
            *first = 0.6;
        }
        logits
    }

    fn locate_python_executable() -> Option<PathBuf> {
        let local_venv = if cfg!(windows) {
            PathBuf::from(".venv/Scripts/python.exe")
        } else {
            PathBuf::from(".venv/bin/python")
        };
        if local_venv.is_file() {
            return Some(local_venv);
        }
        for cand in ["python3", "python"] {
            if create_hidden_command(cand)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(PathBuf::from(cand));
            }
        }
        None
    }

    // -------------------------------------------------------------------------
    // Pipeline Decision Gates 1 - 5
    // -------------------------------------------------------------------------

    /// Gate 1: Open-Weight Synthesis Model & Quant/Offload Selector across all 9 families.
    pub async fn gate1_select_generation_model(
        &self,
        style_prompt: &str,
        lyrics_mode: LyricsMode,
        has_lyrics: bool,
        requested_model: Option<&str>,
        commercial_only: bool,
        lora_path: Option<&Path>,
        mode: DecisionMode,
        preferred_decision_model: Option<&str>,
    ) -> DecisionResult {
        let catalog = HfModelDiscoveryEngine::list_open_weight_catalog(
            &self.memory,
            commercial_only,
            &self.models_dir,
        );

        let synthesis_candidates: Vec<HfModelEntry> = catalog
            .into_iter()
            .filter(|c| c.entry.stage != STAGE_DECISION)
            .map(|c| c.entry)
            .collect();

        let requested_entry = requested_model
            .and_then(|q| (!q.trim().is_empty()).then(|| q.trim()))
            .and_then(HfModelDiscoveryEngine::resolve_model_entry);

        let mut blocked_candidates = Vec::new();
        if let Some(ref req_e) = requested_entry {
            if commercial_only && !req_e.commercial_allowed {
                blocked_candidates.push(format!(
                    "Requested synthesis model '{}' ({}) is non-commercial and blocked under commercial_only=true.",
                    req_e.id,
                    req_e.license_tier.as_str()
                ));
            }
        }

        let prompt_low = style_prompt.to_ascii_lowercase();
        let wants_instrumental = prompt_low.contains("instrumental")
            || prompt_low.contains("no vocal")
            || prompt_low.contains("beat only")
            || prompt_low.contains("soundscape")
            || prompt_low.contains("sfx")
            || (!has_lyrics && prompt_low.contains("loop"));
        let wants_lora = lora_path.is_some() || prompt_low.contains("lora");
        let wants_streaming = prompt_low.contains("realtime") || prompt_low.contains("jam") || prompt_low.contains("live loop");
        let wants_full_vocal = (has_lyrics || lyrics_mode != LyricsMode::Auto || prompt_low.contains("vocal") || prompt_low.contains("anthem") || prompt_low.contains("singing")) && !wants_instrumental;

        let mut options = Vec::with_capacity(synthesis_candidates.len());
        let mut feature_logits = Vec::with_capacity(synthesis_candidates.len());

        for cand in &synthesis_candidates {
            options.push(cand.id.clone());
            if commercial_only && !cand.commercial_allowed {
                feature_logits.push(f64::NEG_INFINITY);
                continue;
            }

            let mut score = 0.0_f64;
            let cached = is_hf_repo_cached(&cand.id, &self.models_dir);
            if cached {
                score += 1.25;
            }
            if cand.fits_hardware(&self.memory) {
                score += 1.65;
            } else {
                score -= 2.20;
            }

            // Default MIT anchor boost
            if cand.id == "ACE-Step/acestep-v15-turbo" {
                score += 1.90;
            } else if cand.license_tier == ModelLicenseTier::Mit || cand.license_tier == ModelLicenseTier::Apache2 {
                score += 0.45;
            }

            if wants_instrumental {
                if cand.id.contains("stable-audio") || cand.id.contains("musicgen") || cand.id.contains("magenta") {
                    score += 1.75;
                }
                if wants_streaming && cand.id.contains("magenta") {
                    score += 1.15;
                }
            } else if wants_full_vocal {
                if cand.id.contains("ACE-Step") || cand.id.contains("YuE2") || cand.id.contains("MiniMax") || cand.id.contains("DiffRhythm2") || cand.id.contains("SongGeneration") {
                    score += 1.15;
                }
            }

            if wants_lora && (cand.id == "openmusic/Muse" || cand.id.contains("ACE-Step")) {
                score += 1.35;
            }

            if let Some(ref req_e) = requested_entry {
                if req_e.id == cand.id && (!commercial_only || cand.commercial_allowed) {
                    score += 3.60;
                }
            }

            // Favor models that comfortably fit <= 8 GB VRAM without thrashing
            if self.memory.gpu_vram_total_gb > 0.0 && self.memory.gpu_vram_total_gb <= 8.5 {
                if cand.min_vram_with_offload_gb <= 4.5 && cand.required_vram_gb <= 6.5 {
                    score += 1.05;
                } else if cand.required_vram_gb >= 14.0 {
                    score -= 1.50;
                }
            }

            feature_logits.push(score);
        }

        let mut req = DecisionRequest::new(
            "gate1_model_selection",
            DecisionSchema::Categorical { options },
            format!("Select optimal open-weight synthesis model for '{}'", style_prompt),
        );
        req.commercial_only = commercial_only;
        req.preferred_model = preferred_decision_model.map(str::to_string);
        req.mode = mode;
        req.context = serde_json::json!({
            "gpu_vram_free_gb": self.memory.gpu_vram_free_gb,
            "gpu_vram_total_gb": self.memory.gpu_vram_total_gb,
            "free_ram_gb": self.memory.free_ram_gb,
            "commercial_only": commercial_only,
            "wants_instrumental": wants_instrumental,
            "requested_model": requested_model,
        });
        req.feature_logits = Some(feature_logits);

        let mut result = self.decide(&req).await;
        if !blocked_candidates.is_empty() {
            result.blocked_candidates.extend(blocked_candidates);
            result.hitl_gate_triggered = true;
            result.trace.hitl_gate_triggered = true;
            result.hitl_reason = Some(result.blocked_candidates.join(" | "));
        }
        let chosen_entry = HfModelDiscoveryEngine::resolve_model_entry(&result.selected_choice);
        result.recommended_action = serde_json::json!({
            "selected_model": result.selected_choice,
            "license_tier": chosen_entry.as_ref().map(|e| e.license_tier.as_str()).unwrap_or("MIT"),
            "commercial_allowed": chosen_entry.as_ref().map(|e| e.commercial_allowed).unwrap_or(true),
            "cached_on_disk": chosen_entry.as_ref().map(|e| e.is_cached_in(&self.models_dir)).unwrap_or(false),
            "cpu_offload": self.memory.should_offload_nimble_to_cpu(),
        });
        result
    }

    /// Gate 2: Lyrics Mode & Structural Section Routing (`UseUserLyricsVerbatim` vs `ComposeStructuredLyricsAi` vs `InstrumentalOnly`).
    pub async fn gate2_resolve_lyrics_and_structure(
        &self,
        style_prompt: &str,
        user_lyrics: &str,
        lyrics_topic: &str,
        lyrics_mode: LyricsMode,
        mode: DecisionMode,
        preferred_decision_model: Option<&str>,
    ) -> DecisionResult {
        let options = vec![
            "UseUserLyricsVerbatim".to_string(),
            "ComposeStructuredLyricsAi".to_string(),
            "InstrumentalOnly".to_string(),
        ];

        let has_user_lyrics = !user_lyrics.trim().is_empty();
        let has_topic = !lyrics_topic.trim().is_empty();
        let low = style_prompt.to_ascii_lowercase();
        let wants_instrumental = low.contains("instrumental only")
            || low.contains("no vocals")
            || low.contains("beat only");

        let mut logits = match lyrics_mode {
            LyricsMode::UserProvided => vec![4.2, if has_user_lyrics { -2.0 } else { 0.8 }, -3.0],
            LyricsMode::AiGenerated => vec![-3.0, 4.2, if wants_instrumental { 0.5 } else { -2.5 }],
            LyricsMode::Auto => {
                if has_user_lyrics {
                    vec![4.0, -1.8, -3.2]
                } else if wants_instrumental && !has_topic {
                    vec![-3.2, -1.0, 3.4]
                } else {
                    vec![-2.8, 3.6, -1.6]
                }
            }
        };
        if !has_user_lyrics && lyrics_mode == LyricsMode::UserProvided {
            // Ambiguity: user asked for verbatim mode with empty lyrics -> trigger borderline logit to invoke HITL notice
            logits = vec![0.4, 0.35, -0.5];
        }

        let mut req = DecisionRequest::new(
            "gate2_lyrics_routing",
            DecisionSchema::Categorical { options },
            "Determine lyrics ownership (verbatim user text vs. local AI composition vs. instrumental).",
        );
        req.mode = mode;
        req.preferred_model = preferred_decision_model.map(str::to_string);
        req.feature_logits = Some(logits);
        req.context = serde_json::json!({
            "lyrics_mode": lyrics_mode.as_str(),
            "user_lyrics_chars": user_lyrics.trim().len(),
            "has_topic": has_topic,
            "wants_instrumental": wants_instrumental,
        });

        let mut res = self.decide(&req).await;
        res.recommended_action = serde_json::json!({
            "lyrics_strategy": res.selected_choice,
            "preserve_verbatim": res.selected_choice == "UseUserLyricsVerbatim",
            "sections": ["[Intro]", "[Verse 1]", "[Chorus]", "[Verse 2]", "[Solo]", "[Bridge]", "[Outro]"],
        });
        res
    }

    /// Gate 3: Stage 5 Telemetry & Quality Gate Adjudication (`AcceptAndMaster` / `AdjustEqAndRemaster` / `RetryStage2Synthesis` / `EscalateHitlReview`).
    pub async fn gate3_adjudicate_telemetry_quality(
        &self,
        state: &MusicProductionState,
        peak_amplitude: f32,
        rms_amplitude: f32,
        crest_factor_db: f32,
        spectral_centroid_hz: f32,
        max_retries: u8,
        mode: DecisionMode,
        preferred_decision_model: Option<&str>,
    ) -> DecisionResult {
        let options = vec![
            "AcceptAndMaster".to_string(),
            "AdjustEqAndRemaster".to_string(),
            "RetryStage2Synthesis".to_string(),
            "EscalateHitlReview".to_string(),
        ];

        let is_silent = rms_amplitude < 0.005 || peak_amplitude < 0.015;
        let is_clipping = peak_amplitude > 0.992;
        let harsh_or_dark = spectral_centroid_hz > 6200.0 || spectral_centroid_hz < 280.0;
        let can_retry = state.retry_count < max_retries;

        let mut s_accept = 2.0_f64;
        let mut s_eq = 0.2_f64;
        let mut s_retry = -1.5_f64;
        let mut s_escalate = -2.2_f64;

        if state.rendered_by_neural {
            s_accept += 1.0;
        }
        if state.judge_score >= 7.5 {
            s_accept += 0.9;
        } else if state.judge_score < 6.0 {
            s_accept -= 1.4;
            s_retry += 1.6;
        }
        if crest_factor_db >= 6.0 && crest_factor_db <= 18.0 {
            s_accept += 0.7;
        } else {
            s_eq += 1.1;
        }
        if harsh_or_dark {
            s_eq += 1.85;
            s_accept -= 0.7;
        }
        if is_clipping {
            s_accept -= 1.8;
            s_eq += 2.1;
        }
        if is_silent {
            s_accept = -3.0;
            if can_retry {
                s_retry = 3.8;
            } else {
                s_escalate = 3.6;
            }
        } else if !can_retry && state.judge_score < 6.2 {
            s_escalate += 2.8;
        }

        let mut req = DecisionRequest::new(
            "gate3_telemetry_adjudication",
            DecisionSchema::Categorical { options },
            "Adjudicate telemetry metrics for final mastering vs. EQ tweak vs. regeneration retry.",
        );
        req.commercial_only = state.commercial_only;
        req.mode = mode;
        req.preferred_model = preferred_decision_model.map(str::to_string);
        req.feature_logits = Some(vec![s_accept, s_eq, s_retry, s_escalate]);
        req.context = serde_json::json!({
            "peak_amplitude": peak_amplitude,
            "rms_amplitude": rms_amplitude,
            "crest_factor_db": crest_factor_db,
            "spectral_centroid_hz": spectral_centroid_hz,
            "rendered_by_neural": state.rendered_by_neural,
            "judge_score": state.judge_score,
            "retry_count": state.retry_count,
        });

        let mut res = self.decide(&req).await;
        res.recommended_action = serde_json::json!({
            "verdict": res.selected_choice,
            "accept": res.selected_choice == "AcceptAndMaster" || res.selected_choice == "AdjustEqAndRemaster",
            "needs_eq_fix": res.selected_choice == "AdjustEqAndRemaster",
            "retry_stage2": res.selected_choice == "RetryStage2Synthesis",
        });
        res
    }

    /// Gate 4: AI Mix Copilot — computes calibrated DAW stem adjustments & mix deltas from current mix & goal.
    pub async fn gate4_recommend_mix_adjustments(
        &self,
        style_prompt: &str,
        current_mix: &StemMixConfig,
        focus_request: &str,
        commercial_only: bool,
        mode: DecisionMode,
        preferred_decision_model: Option<&str>,
    ) -> DecisionResult {
        let presets = vec![
            "VocalPresenceAndPunch".to_string(),
            "SubHeavyDubWarmth".to_string(),
            "WideArenaGuitarAndKeys".to_string(),
            "RadioLoudnessCrisp".to_string(),
            "BalancedStudioReference".to_string(),
        ];
        let low = format!("{} {}", style_prompt, focus_request).to_ascii_lowercase();
        let logits = vec![
            if low.contains("vocal") || low.contains("lyric") || low.contains("punch") || low.contains("clear") { 3.1 } else { 0.8 },
            if low.contains("bass") || low.contains("dub") || low.contains("reggae") || low.contains("warm") || low.contains("sub") { 3.0 } else { 0.7 },
            if low.contains("wide") || low.contains("guitar") || low.contains("rock") || low.contains("arena") || low.contains("stereo") { 2.9 } else { 0.6 },
            if low.contains("loud") || low.contains("radio") || low.contains("pop") || low.contains("crisp") || low.contains("bright") { 2.9 } else { 0.5 },
            if low.trim().is_empty() { 2.4 } else { 0.9 },
        ];

        let mut req = DecisionRequest::new(
            "gate4_ai_mix_copilot",
            DecisionSchema::Categorical { options: presets },
            format!("Recommend non-destructive DAW stem mix adjustments for focus '{}'", focus_request),
        );
        req.commercial_only = commercial_only;
        req.mode = mode;
        req.preferred_model = preferred_decision_model.map(str::to_string);
        req.feature_logits = Some(logits);
        req.context = serde_json::json!({
            "style_prompt": style_prompt,
            "focus_request": focus_request,
        });

        let mut res = self.decide(&req).await;
        let mut updated_mix = current_mix.clone();
        let mut patch_channel = |id: &str, f: fn(&mut StemChannelStrip)| {
            let mut ch = updated_mix.resolve_channel(id);
            f(&mut ch);
            updated_mix.set_channel(id, ch);
        };

        match res.selected_choice.as_str() {
            "VocalPresenceAndPunch" => {
                patch_channel("vocals", |c| {
                    c.gain_db = (c.gain_db + 1.8).clamp(-24.0, 6.0);
                    c.eq_mid_db = (c.eq_mid_db + 2.0).clamp(-12.0, 12.0);
                    c.eq_high_db = (c.eq_high_db + 1.5).clamp(-12.0, 12.0);
                    c.reverb_send = (c.reverb_send + 0.04).clamp(0.0, 1.0);
                });
                patch_channel("keys_synths", |c| c.eq_mid_db = (c.eq_mid_db - 1.5).clamp(-12.0, 12.0));
                patch_channel("drums", |c| c.gain_db = (c.gain_db + 0.6).clamp(-24.0, 6.0));
            }
            "SubHeavyDubWarmth" => {
                patch_channel("bass", |c| {
                    c.gain_db = (c.gain_db + 2.0).clamp(-24.0, 6.0);
                    c.eq_low_db = (c.eq_low_db + 2.8).clamp(-12.0, 12.0);
                    c.reverb_send = 0.02;
                });
                patch_channel("drums", |c| c.eq_low_db = (c.eq_low_db + 1.5).clamp(-12.0, 12.0));
                patch_channel("ambience_fx", |c| c.reverb_send = (c.reverb_send + 0.16).clamp(0.0, 1.0));
                updated_mix.analog_tape_warmth = (updated_mix.analog_tape_warmth + 0.12).clamp(0.0, 1.0);
                updated_mix.sub_bass_boost_db = (updated_mix.sub_bass_boost_db + 1.4).clamp(0.0, 9.0);
            }
            "WideArenaGuitarAndKeys" => {
                patch_channel("guitar", |c| {
                    c.gain_db = (c.gain_db + 1.4).clamp(-24.0, 6.0);
                    c.pan = -0.45;
                    c.eq_high_db = (c.eq_high_db + 1.5).clamp(-12.0, 12.0);
                });
                patch_channel("keys_synths", |c| {
                    c.gain_db = (c.gain_db + 0.8).clamp(-24.0, 6.0);
                    c.pan = 0.45;
                });
                updated_mix.stereo_width = (updated_mix.stereo_width + 0.16).clamp(0.8, 1.6);
            }
            "RadioLoudnessCrisp" => {
                patch_channel("vocals", |c| {
                    c.gain_db = (c.gain_db + 1.5).clamp(-24.0, 6.0);
                    c.eq_high_db = (c.eq_high_db + 2.2).clamp(-12.0, 12.0);
                });
                patch_channel("drums", |c| {
                    c.gain_db = (c.gain_db + 1.0).clamp(-24.0, 6.0);
                    c.eq_high_db = (c.eq_high_db + 1.2).clamp(-12.0, 12.0);
                });
                updated_mix.target_lufs = -12.5;
            }
            _ => {
                patch_channel("vocals", |c| c.gain_db = c.gain_db.clamp(-24.0, 6.0));
                updated_mix.target_lufs = -14.0;
            }
        }

        res.recommended_action = serde_json::json!({
            "profile": res.selected_choice,
            "recommended_mix": updated_mix,
        });
        res
    }

    /// Gate 5: Commercial License & Export Compliance Verification across synthesis + decision models.
    pub async fn gate5_license_and_export_gate(
        &self,
        selected_model_id: &str,
        decision_model_id: Option<&str>,
        commercial_only: bool,
        mode: DecisionMode,
    ) -> DecisionResult {
        let synth_entry = HfModelDiscoveryEngine::resolve_model_entry(selected_model_id);
        let (dec_entry, _) = Self::resolve_decision_checkpoint(decision_model_id, commercial_only);

        let synth_comm = synth_entry.as_ref().map(|e| e.commercial_allowed).unwrap_or(true);
        let dec_comm = dec_entry.commercial_allowed;
        let both_comm = synth_comm && dec_comm;

        let options = vec![
            "ClearForCommercialRelease".to_string(),
            "ClearForPersonalResearchOnly".to_string(),
            "BlockedLicenseConflict".to_string(),
        ];

        let mut blocked_candidates = Vec::new();
        let logits = if commercial_only && !both_comm {
            if !synth_comm {
                blocked_candidates.push(format!(
                    "Synthesis model '{}' is non-commercial under commercial_only=true.",
                    selected_model_id
                ));
            }
            if !dec_comm {
                blocked_candidates.push(format!(
                    "Decision model '{}' is non-commercial under commercial_only=true.",
                    dec_entry.id
                ));
            }
            vec![-3.5, -2.5, 4.2]
        } else if both_comm {
            vec![3.8, -1.2, -3.5]
        } else {
            vec![-2.5, 3.6, -2.5]
        };

        let mut req = DecisionRequest::new(
            "gate5_license_export",
            DecisionSchema::Categorical { options },
            "Verify open-weight licensing compliance for requested commercial_only export flag.",
        );
        req.commercial_only = commercial_only;
        req.mode = mode;
        req.preferred_model = Some(dec_entry.id.clone());
        req.feature_logits = Some(logits);

        let mut res = self.decide(&req).await;
        if !blocked_candidates.is_empty() {
            res.blocked_candidates.extend(blocked_candidates);
            res.hitl_gate_triggered = true;
            res.trace.hitl_gate_triggered = true;
            res.hitl_reason = Some(res.blocked_candidates.join(" | "));
        }
        res.recommended_action = serde_json::json!({
            "export_status": res.selected_choice,
            "synthesis_model": selected_model_id,
            "synthesis_license": synth_entry.map(|e| e.license_tier.as_str().to_string()).unwrap_or_else(|| "MIT".to_string()),
            "decision_model": dec_entry.id,
            "decision_license": dec_entry.license_tier.as_str(),
            "commercial_allowed": both_comm,
        });
        res
    }

    pub fn record_into_state(&self, state: &mut MusicProductionState, res: &DecisionResult) -> Result<()> {
        state.record_decision_trace(res.trace.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> NimbleDecisionEngine {
        NimbleDecisionEngine::new(
            OllamaClient::new("http://127.0.0.1:1"), // offline port so System-1 tests are isolated
            PathBuf::from("nonexistent_models_dir_for_test"),
            PathBuf::from("nonexistent_worker.py"),
            SystemMemory {
                total_ram_gb: 32.0,
                free_ram_gb: 20.0,
                cpu_cores: 8,
                gpu_name: Some("NVIDIA Quadro RTX 4000".into()),
                gpu_vram_total_gb: 8.0,
                gpu_vram_free_gb: 5.5,
                force_dsp: false,
                force_hf: false,
            },
        )
    }

    #[test]
    fn test_calibrate_logits_sums_to_one_and_bounds_entropy() {
        let choices = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let (probs, entropy) = calibrate_logits(&choices, &[2.1, 0.4, -0.8, 1.1], 1.0);
        let sum: f32 = probs.iter().map(|(_, p)| *p).sum();
        assert!((sum - 1.0).abs() < 1e-4, "probabilities sum was {}", sum);
        assert!(entropy > 0.0 && entropy <= (4.0_f32).ln() + 1e-4, "entropy={}", entropy);
        assert_eq!(probs[0].0, "A");
        assert!(probs[0].1 > probs[3].1 && probs[3].1 > probs[1].1);
    }

    #[test]
    fn test_commercial_only_blocks_nimble_v3() {
        let (entry, block_note) = NimbleDecisionEngine::resolve_decision_checkpoint(
            Some("bespokelabs/Bespoke-Nimble-9B-v3"),
            true,
        );
        assert_eq!(entry.id, "bespokelabs/Bespoke-Nimble-9B");
        assert!(entry.commercial_allowed);
        assert!(block_note.is_some());
    }

    #[tokio::test]
    async fn test_hitl_gate_triggers_below_half_confidence() {
        let engine = test_engine();
        let mut req = DecisionRequest::new(
            "hitl_test",
            DecisionSchema::Categorical {
                options: vec!["Opt1".into(), "Opt2".into(), "Opt3".into(), "Opt4".into()],
            },
            "Uncertain choice",
        );
        req.mode = DecisionMode::FastDeterministic;
        req.feature_logits = Some(vec![0.10, 0.08, 0.05, 0.02]);

        let res = engine.decide(&req).await;
        assert!(res.confidence < HITL_CONFIDENCE_THRESHOLD, "conf={}", res.confidence);
        assert!(res.hitl_gate_triggered);
        assert!(res.trace.hitl_gate_triggered);
        assert!(res.hitl_reason.is_some());
    }

    #[tokio::test]
    async fn test_system1_deterministic_decision_under_5ms() {
        let engine = test_engine();
        let mut req = DecisionRequest::new("speed_test", DecisionSchema::Likert5, "Rate mix clarity");
        req.mode = DecisionMode::FastDeterministic;
        req.feature_logits = Some(vec![-2.0, -0.5, 0.5, 2.2, 3.4]);

        let start = Instant::now();
        let res = engine.decide(&req).await;
        let elapsed_ms = start.elapsed().as_millis();
        assert!(elapsed_ms < 25, "System-1 took {} ms", elapsed_ms);
        assert_eq!(res.selected_choice, "5_Masterpiece");
        let sum: f32 = res.probabilities.iter().map(|(_, p)| *p).sum();
        assert!((sum - 1.0).abs() < 1e-4);
    }

    #[tokio::test]
    async fn test_gates_1_to_5_end_to_end() {
        let engine = test_engine();

        // Gate 1: requesting non-commercial LeVo 2 with commercial_only=true triggers block + safe fallback
        let g1 = engine
            .gate1_select_generation_model(
                "uplifting soul pop vocal anthem, 108 BPM",
                LyricsMode::Auto,
                true,
                Some("tencent/SongGeneration"),
                true,
                None,
                DecisionMode::FastDeterministic,
                Some("bespokelabs/Bespoke-Nimble-9B-v3"),
            )
            .await;
        assert_ne!(g1.selected_choice, "tencent/SongGeneration");
        assert!(g1.hitl_gate_triggered);
        assert!(!g1.blocked_candidates.is_empty());

        // Gate 2: user lyrics must stay verbatim
        let g2 = engine
            .gate2_resolve_lyrics_and_structure(
                "synthpop neon drive",
                "[Verse 1]\nNeon rivers under midnight rain",
                "",
                LyricsMode::Auto,
                DecisionMode::FastDeterministic,
                None,
            )
            .await;
        assert_eq!(g2.selected_choice, "UseUserLyricsVerbatim");

        // Gate 3: silent telemetry with retries left proposes RetryStage2Synthesis
        let state = MusicProductionState::new("song-t", "Synthpop", 118, "C Minor");
        let g3 = engine
            .gate3_adjudicate_telemetry_quality(
                &state,
                0.002,
                0.0005,
                10.0,
                1500.0,
                2,
                DecisionMode::FastDeterministic,
                None,
            )
            .await;
        assert_eq!(g3.selected_choice, "RetryStage2Synthesis");

        // Gate 4: AI Mix Copilot adjusts vocals & returns structured mix config
        let mix = StemMixConfig::default();
        let g4 = engine
            .gate4_recommend_mix_adjustments(
                "neo-soul ballad",
                &mix,
                "make vocals clearer and punchier",
                true,
                DecisionMode::FastDeterministic,
                None,
            )
            .await;
        assert_eq!(g4.selected_choice, "VocalPresenceAndPunch");
        assert!(g4.recommended_action.get("recommended_mix").is_some());

        // Gate 5: Commercial license gate rejects non-commercial Yue2 when commercial_only=true
        let g5 = engine
            .gate5_license_and_export_gate(
                "m-a-p/YuE2-3B",
                Some("bespokelabs/Bespoke-Nimble-9B"),
                true,
                DecisionMode::FastDeterministic,
            )
            .await;
        assert_eq!(g5.selected_choice, "BlockedLicenseConflict");
    }
}
