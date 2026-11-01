//! Live System Memory and GPU VRAM detection, Dynamic Available Memory Law enforcement,
//! and stage execution routing.

use std::process::Command;
use sysinfo::System;

/// Helper to construct a Command with CREATE_NO_WINDOW on Windows to prevent console flashing.
#[inline]
pub fn create_hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Detected system memory and GPU resources.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemMemory {
    pub total_ram_gb: f64,
    pub free_ram_gb: f64,
    pub cpu_cores: usize,
    pub gpu_name: Option<String>,
    pub gpu_vram_total_gb: f64,
    pub gpu_vram_free_gb: f64,
    #[serde(default)]
    pub force_dsp: bool,
    #[serde(default)]
    pub force_hf: bool,
}

/// Execution route for a music production pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecutionRoute {
    /// Execute heavy Hugging Face checkpoint in isolated subprocess using GPU VRAM
    HuggingFaceGpu,
    /// Execute Hugging Face checkpoint offloaded to system CPU RAM
    HuggingFaceCpu,
    /// Execute standalone internal 48 kHz pure-Rust DSP synthesis engine
    PureRustDsp,
}

/// Music production pipeline stage identifier for memory requirement checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PipelineStageKind {
    /// Stage 0: Ollama specialist panel
    OllamaPanel,
    /// Stage 1: Drum & groove foundation (ACE-Step-v1.5, requires ~6.0 GB VRAM / ~12.0 GB RAM)
    Stage1Groove,
    /// Stage 2: Dual-stem semantic + acoustic synthesis (YuE-s1-7B + YuE-s2-1B, requires ~8.0 GB VRAM / ~24.0 GB RAM)
    Stage2YuE,
    /// Stage 3: Timbre & voice conversion (Seed-VC, requires ~4.0 GB VRAM / ~8.0 GB RAM)
    Stage3VoiceConversion,
    /// Stage 4: Stem separation & virtuoso solo inpainting (htdemucs + stable-audio-open, requires ~8.0 GB VRAM / ~16.0 GB RAM)
    Stage4Inpainting,
    /// Stage 5: Super-resolution & reference analog mastering (AudioSR + Matchering 2.0, requires ~4.0 GB VRAM / ~8.0 GB RAM)
    Stage5Mastering,

    // Aliases
    GrooveFoundation,
    YuESynthesis,
    VoiceConversion,
    StemSeparationAndInpainting,
    Mastering,
}

pub type StageType = PipelineStageKind;

impl PipelineStageKind {
    /// Minimum required GPU VRAM in GB to safely run the Hugging Face stage without OOM.
    pub fn required_vram_gb(&self) -> f64 {
        match self {
            PipelineStageKind::OllamaPanel => 2.0,
            PipelineStageKind::Stage1Groove | PipelineStageKind::GrooveFoundation => 6.0,
            PipelineStageKind::Stage2YuE | PipelineStageKind::YuESynthesis => 8.0,
            PipelineStageKind::Stage3VoiceConversion | PipelineStageKind::VoiceConversion => 4.0,
            PipelineStageKind::Stage4Inpainting | PipelineStageKind::StemSeparationAndInpainting => 8.0,
            PipelineStageKind::Stage5Mastering | PipelineStageKind::Mastering => 4.0,
        }
    }

    /// Minimum required CPU RAM in GB to run the stage via CPU offload if GPU is absent.
    pub fn required_ram_gb(&self) -> f64 {
        match self {
            PipelineStageKind::OllamaPanel => 4.0,
            PipelineStageKind::Stage1Groove | PipelineStageKind::GrooveFoundation => 12.0,
            PipelineStageKind::Stage2YuE | PipelineStageKind::YuESynthesis => 24.0,
            PipelineStageKind::Stage3VoiceConversion | PipelineStageKind::VoiceConversion => 8.0,
            PipelineStageKind::Stage4Inpainting | PipelineStageKind::StemSeparationAndInpainting => 16.0,
            PipelineStageKind::Stage5Mastering | PipelineStageKind::Mastering => 8.0,
        }
    }
}

impl SystemMemory {
    /// Dynamically detect current runtime system resources (probed live without caching).
    pub fn detect_live() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let total_ram_gb = sys.total_memory() as f64 / 1_073_741_824.0;
        let free_ram_gb = sys.available_memory() as f64 / 1_073_741_824.0;
        let cpu_cores = sys.physical_core_count().unwrap_or_else(|| sys.cpus().len()).max(1);

        let (gpu_name, gpu_vram_total_gb, gpu_vram_free_gb) = detect_gpu();
        let force_dsp = std::env::var("MUSIC_FORCE_DSP").is_ok();
        let force_hf = std::env::var("MUSIC_FORCE_HF").is_ok();
        SystemMemory {
            total_ram_gb,
            free_ram_gb,
            cpu_cores,
            gpu_name,
            gpu_vram_total_gb,
            gpu_vram_free_gb,
            force_dsp,
            force_hf,
        }
    }

    /// Construct a simulated SystemMemory instance for deterministic unit testing.
    pub fn simulated(
        total_ram_gb: f64,
        free_ram_gb: f64,
        cpu_cores: usize,
        gpu_name: Option<String>,
        gpu_vram_total_gb: f64,
        gpu_vram_free_gb: f64,
    ) -> Self {
        Self {
            total_ram_gb,
            free_ram_gb,
            cpu_cores,
            gpu_name,
            gpu_vram_total_gb,
            gpu_vram_free_gb,
            force_dsp: false,
            force_hf: false,
        }
    }

    /// Builder helper to simulate DSP override without modifying global process environment.
    pub fn with_force_dsp(mut self, force: bool) -> Self {
        self.force_dsp = force;
        self
    }

    /// Builder helper to simulate HF override without modifying global process environment.
    pub fn with_force_hf(mut self, force: bool) -> Self {
        self.force_hf = force;
        self
    }

    /// Whether a usable discrete GPU with free VRAM is present.
    pub fn has_usable_gpu(&self) -> bool {
        self.gpu_name.is_some() && self.gpu_vram_free_gb > 1.0
    }

    /// Free VRAM in megabytes.
    pub fn free_vram_mb(&self) -> f64 {
        self.gpu_vram_free_gb * 1024.0
    }

    /// Usable GPU VRAM budget with 15% safety headroom.
    pub fn gpu_budget_gb(&self) -> f64 {
        (self.gpu_vram_free_gb * 0.85).max(0.0)
    }

    /// Usable system RAM budget with 3.0 GB OS reserve and 70% safety headroom.
    pub fn ram_budget_gb(&self) -> f64 {
        ((self.free_ram_gb - 3.0) * 0.70).max(0.0)
    }

    /// Recommend the highest-quality installed Ollama model that fits in current runtime resources.
    /// Strictly adheres to the Dynamic Available Memory Law:
    /// - Free RAM >= 48 GB OR Free VRAM >= 24 GB -> `qwen2.5:32b`
    /// - Free RAM >= 24 GB OR Free VRAM >= 14 GB -> `qwen2.5:14b`
    /// - Free RAM >= 12 GB OR Free VRAM >= 4.5 GB -> `qwen2.5:7b`
    /// - Free RAM >= 6 GB OR Free VRAM >= 2 GB -> `qwen2.5:3b`
    /// - Free RAM < 6 GB -> `qwen2.5:1.5b`
    ///
    /// Never appends `-coder` suffixes that trigger silent 404 aborts.
    pub fn recommend_ollama_model(&self, installed_tags: &[String]) -> String {
        let preferred_tier = if self.free_ram_gb >= 48.0 || self.gpu_vram_free_gb >= 24.0 {
            "qwen2.5:32b"
        } else if self.free_ram_gb >= 24.0 || self.gpu_vram_free_gb >= 14.0 {
            "qwen2.5:14b"
        } else if self.free_ram_gb >= 12.0 || self.gpu_vram_free_gb >= 4.5 {
            "qwen2.5:7b"
        } else if self.free_ram_gb >= 6.0 || self.gpu_vram_free_gb >= 2.0 {
            "qwen2.5:3b"
        } else {
            "qwen2.5:1.5b"
        };

        if installed_tags.is_empty() {
            return preferred_tier.to_string();
        }

        // 1. Direct match for preferred tier
        if installed_tags.iter().any(|t| t == preferred_tier || t.starts_with(preferred_tier)) {
            return preferred_tier.to_string();
        }

        // 2. Check for Recursive Flint (40B MoE with 2.5B active params) if memory allows (>= 8 GB RAM or >= 4 GB VRAM)
        if (self.free_ram_gb >= 8.0 || self.gpu_vram_free_gb >= 4.0)
            && installed_tags.iter().any(|t| t.contains("flint"))
        {
            if let Some(flint) = installed_tags.iter().find(|t| t.contains("flint")) {
                return flint.clone();
            }
        }

        // 3. Check for GLM-4 9B if memory allows (>= 8 GB RAM or >= 6 GB VRAM)
        if (self.free_ram_gb >= 8.0 || self.gpu_vram_free_gb >= 6.0)
            && installed_tags.iter().any(|t| t.contains("glm4"))
        {
            if let Some(glm) = installed_tags.iter().find(|t| t.contains("glm4")) {
                return glm.clone();
            }
        }

        // 4. Candidate list search in descending quality order
        let candidates = [
            "qwen2.5:32b", "qwen2.5:14b", "qwen2.5:7b", "mistral:latest",
            "llama3.1:8b", "qwen2.5:3b", "gemma2:2b", "qwen2.5:1.5b"
        ];
        for cand in &candidates {
            if let Some(found) = installed_tags.iter().find(|t| t.as_str() == *cand || t.starts_with(cand)) {
                return found.clone();
            }
        }

        // 5. Default to first installed tag or preferred tier
        installed_tags.first().cloned().unwrap_or_else(|| preferred_tier.to_string())
    }

    /// Alias for recommend_ollama_model.
    pub fn pick_ollama_model(&self, installed: &[String]) -> String {
        self.recommend_ollama_model(installed)
    }

    /// Decide execution routing for a music pipeline stage based on live hardware resources.
    /// Supports manual override via `MUSIC_FORCE_DSP=1` or `MUSIC_FORCE_HF=1` (read during `detect_live()` or via builder).
    pub fn route_stage(&self, stage: PipelineStageKind, engine_mode: &str) -> ExecutionRoute {
        if engine_mode == "pure_rust" || self.force_dsp {
            return ExecutionRoute::PureRustDsp;
        }

        let required_vram = stage.required_vram_gb();
        let required_ram = stage.required_ram_gb();

        // 1. Discrete GPU with sufficient VRAM budget
        if self.has_usable_gpu() && self.gpu_budget_gb() >= required_vram {
            return ExecutionRoute::HuggingFaceGpu;
        }

        // 2. Forced HF or hybrid engine with sufficient RAM budget
        if (engine_mode == "hf_hybrid" || self.force_hf)
            && self.ram_budget_gb() >= required_ram
        {
            return ExecutionRoute::HuggingFaceCpu;
        }

        // 3. Default safe fail-closed path: Route to internal pure-Rust 48 kHz DSP engine
        ExecutionRoute::PureRustDsp
    }

    /// Returns true if the stage should execute via the internal pure-Rust DSP engine for a given mode.
    pub fn should_use_dsp(&self, stage: PipelineStageKind, engine_mode: &str) -> bool {
        self.route_stage(stage, engine_mode) == ExecutionRoute::PureRustDsp
    }

    /// Returns true if the stage should execute via the internal pure-Rust DSP engine under "auto" mode.
    pub fn should_use_dsp_auto(&self, stage: PipelineStageKind) -> bool {
        self.should_use_dsp(stage, "auto")
    }

    /// Selects the optimal Hugging Face neural model or pure-Rust fallback based on live GPU VRAM.
    pub fn select_neural_model(&self, engine_mode: &str) -> NeuralModelChoice {
        if engine_mode == "pure_rust" || self.force_dsp || !self.has_usable_gpu() {
            return NeuralModelChoice::PureRustFallback;
        }

        let budget_vram = self.gpu_budget_gb();
        if budget_vram >= 10.0 || self.gpu_vram_free_gb >= 12.0 {
            NeuralModelChoice::YueS1_7B
        } else if budget_vram >= 5.0 || self.gpu_vram_free_gb >= 6.0 {
            NeuralModelChoice::AceStepV1_5
        } else if budget_vram >= 3.0 || self.gpu_vram_free_gb >= 3.5 {
            NeuralModelChoice::MusicGenMedium
        } else {
            NeuralModelChoice::PureRustFallback
        }
    }

    /// Print hardware telemetry to stderr.
    pub fn print_telemetry(&self) {
        eprintln!(
            "💾 [MEMORY] RAM: {:.1}/{:.1} GB Free | CPU: {} cores | GPU: {} ({:.1}/{:.1} GB VRAM Free)",
            self.free_ram_gb,
            self.total_ram_gb,
            self.cpu_cores,
            self.gpu_name.as_deref().unwrap_or("None (CPU Mode)"),
            self.gpu_vram_free_gb,
            self.gpu_vram_total_gb
        );
    }
}

/// Hugging Face neural song/vocal generation model choice based on live GPU VRAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NeuralModelChoice {
    /// Full-song dual-track vocal & accompaniment transformer (~12+ GB VRAM)
    YueS1_7B,
    /// Acoustic drum kit & rhythm foundation diffusion model (~6-8 GB VRAM)
    AceStepV1_5,
    /// Conditional multi-instrument music generator (~3.5-6 GB VRAM)
    MusicGenMedium,
    /// Standalone pure-Rust 48 kHz physical modeling synthesizer (0 GB VRAM)
    PureRustFallback,
}

impl NeuralModelChoice {
    pub fn model_id(&self) -> &'static str {
        match self {
            Self::YueS1_7B => "m-a-p/YuE-s1-7B-anneal-en-cot",
            Self::AceStepV1_5 => "ACE-Studio/ACE-Step-v1.5",
            Self::MusicGenMedium => "facebook/musicgen-medium",
            Self::PureRustFallback => "InternalPureRust/PhysicalModeler",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::YueS1_7B => "YuE-s1-7B",
            Self::AceStepV1_5 => "ACE-Step-v1.5",
            Self::MusicGenMedium => "musicgen-medium",
            Self::PureRustFallback => "Pure-Rust Physical Modeler",
        }
    }
}

/// Detect GPU name and free/total VRAM across Windows and Unix platforms.
fn detect_gpu() -> (Option<String>, f64, f64) {
    // 1. Try nvidia-smi with CREATE_NO_WINDOW
    let output = create_hidden_command("nvidia-smi")
        .args(["--query-gpu=name,memory.total,memory.free", "--format=csv,noheader,nounits"])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            let first_line = text.lines().next().unwrap_or("").trim();
            let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let total_mb: f64 = parts[1].parse().unwrap_or(0.0);
                let free_mb: f64 = parts[2].parse().unwrap_or(0.0);
                return (Some(name), total_mb / 1024.0, free_mb / 1024.0);
            }
        }
    }

    // 2. Candidate paths on Windows if nvidia-smi is not in PATH
    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Windows\System32\nvidia-smi.exe",
            r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
        ];
        for cand in &candidates {
            if std::path::Path::new(cand).is_file() {
                if let Ok(o) = create_hidden_command(cand)
                    .args(["--query-gpu=name,memory.total,memory.free", "--format=csv,noheader,nounits"])
                    .output()
                {
                    if o.status.success() {
                        let text = String::from_utf8_lossy(&o.stdout);
                        let first_line = text.lines().next().unwrap_or("").trim();
                        let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 3 {
                            let name = parts[0].to_string();
                            let total_mb: f64 = parts[1].parse().unwrap_or(0.0);
                            let free_mb: f64 = parts[2].parse().unwrap_or(0.0);
                            return (Some(name), total_mb / 1024.0, free_mb / 1024.0);
                        }
                    }
                }
            }
        }
    }

    (None, 0.0, 0.0)
}
