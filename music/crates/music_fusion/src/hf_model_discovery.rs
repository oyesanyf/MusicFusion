//! Dynamic Hugging Face Music & Vocal Model Auto-Discovery, Resource Evaluation,
//! and Live Stage Upgrade Engine for MusicFusion.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::memory::SystemMemory;

/// Pipeline stages for music generation and processing.
pub const STAGE_GROOVE: &str = "Stage1Groove";
pub const STAGE_YUE: &str = "Stage2YuE";
pub const STAGE_VOICE_CONVERSION: &str = "Stage3VoiceConversion";
pub const STAGE_INPAINTING: &str = "Stage4Inpainting";
pub const STAGE_MASTERING: &str = "Stage5Mastering";
pub const STAGE_FUSION_TTS: &str = "FusionTts";

/// Represents a discovered Hugging Face model suitable for a pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HfModelEntry {
    pub id: String,
    pub name: String,
    pub pipeline_tag: String,
    pub stage: String,
    pub required_vram_gb: f64,
    pub required_ram_gb: f64,
    pub trending_score: f64,
    pub downloads: u64,
    pub likes: u64,
    pub description: String,
}

/// Persistent registry of active and candidate Hugging Face music models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HfModelRegistry {
    pub last_updated: String,
    pub discovered_count: usize,
    pub active_models: HashMap<String, String>,
    pub stage_selections: HashMap<String, HfModelEntry>,
    pub candidates_by_stage: HashMap<String, Vec<HfModelEntry>>,
    pub system_memory_snapshot: Option<SystemMemory>,
}

/// Raw model response from Hugging Face Hub API (`/api/models`).
#[derive(Debug, Clone, Deserialize)]
struct HfApiItem {
    id: String,
    #[serde(default)]
    likes: Option<u64>,
    #[serde(default)]
    downloads: Option<u64>,
    #[serde(default)]
    pipeline_tag: Option<String>,
}

/// Hugging Face Model Discovery & Upgrade Engine.
pub struct HfModelDiscoveryEngine {
    storage_path: PathBuf,
    client: Client,
}

impl Default for HfModelDiscoveryEngine {
    fn default() -> Self {
        Self::new("storage/hf_music_models_registry.json")
    }
}

impl HfModelDiscoveryEngine {
    /// Create a new discovery engine pointing to the given registry file path.
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        Self {
            storage_path: storage_path.into(),
            client,
        }
    }

    /// Load the cached model registry from disk, or initialize with defaults if missing.
    pub fn load_registry(&self, mem: &SystemMemory) -> HfModelRegistry {
        if self.storage_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.storage_path) {
                if let Ok(reg) = serde_json::from_str::<HfModelRegistry>(&content) {
                    return reg;
                }
            }
        }
        self.default_registry(mem)
    }

    /// Save the model registry to disk.
    pub fn save_registry(&self, registry: &HfModelRegistry) -> Result<()> {
        if let Some(parent) = self.storage_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(registry)?;
        std::fs::write(&self.storage_path, json)?;
        Ok(())
    }

    /// Query the Hugging Face Hub API for trending music models across categories.
    /// Falls back cleanly to curated built-in candidates if offline or timed out.
    pub async fn discover_models_online(&self) -> Vec<HfModelEntry> {
        let mut discovered = Vec::new();

        // 1. Music generation (text-to-audio)
        if let Ok(models) = self.query_hf_category("text-to-audio", "music", STAGE_GROOVE).await {
            discovered.extend(models);
        }

        // 2. Vocal / TTS synthesis (text-to-speech)
        if let Ok(models) = self.query_hf_category("text-to-speech", "singing", STAGE_FUSION_TTS).await {
            discovered.extend(models);
        }

        // 3. Voice conversion (audio-to-audio)
        if let Ok(models) = self.query_hf_category("audio-to-audio", "voice", STAGE_VOICE_CONVERSION).await {
            discovered.extend(models);
        }

        // 4. Stem separation / Demucs
        if let Ok(models) = self.query_hf_search("demucs", STAGE_INPAINTING).await {
            discovered.extend(models);
        }

        // Merge with built-in reference candidates so quality is never compromised
        for default_cand in Self::curated_builtins() {
            if !discovered.iter().any(|d| d.id == default_cand.id) {
                discovered.push(default_cand);
            }
        }

        discovered
    }

    /// Query HF API for a pipeline tag and search term.
    async fn query_hf_category(&self, tag: &str, search: &str, stage: &str) -> Result<Vec<HfModelEntry>> {
        let url = format!(
            "https://huggingface.co/api/models?pipeline_tag={}&search={}&sort=trending&limit=8",
            tag, search
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let items: Vec<HfApiItem> = resp.json().await.unwrap_or_default();
        let mut entries = Vec::new();
        for item in items {
            entries.push(Self::api_item_to_entry(&item, stage, tag));
        }
        Ok(entries)
    }

    /// Query HF API for search term.
    async fn query_hf_search(&self, search: &str, stage: &str) -> Result<Vec<HfModelEntry>> {
        let url = format!(
            "https://huggingface.co/api/models?search={}&sort=trending&limit=6",
            search
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let items: Vec<HfApiItem> = resp.json().await.unwrap_or_default();
        let mut entries = Vec::new();
        for item in items {
            entries.push(Self::api_item_to_entry(&item, stage, "audio-to-audio"));
        }
        Ok(entries)
    }

    /// Convert raw HF API item to structured HfModelEntry with heuristic resource requirements.
    fn api_item_to_entry(item: &HfApiItem, stage: &str, default_tag: &str) -> HfModelEntry {
        let id_lower = item.id.to_lowercase();
        let (vram, ram) = if id_lower.contains("minimax-music3") || id_lower.contains("minimax_music3") {
            // Supports 8GB VRAM video cards via diffusers ComponentsManager + leaf-level streaming group offloading
            (6.5, 16.0)
        } else if id_lower.contains("7b") || id_lower.contains("8b") {
            (14.0, 28.0)
        } else if id_lower.contains("1.5b") || id_lower.contains("2b") || id_lower.contains("medium") {
            (6.0, 12.0)
        } else if id_lower.contains("82m") || id_lower.contains("small") || id_lower.contains("mini") {
            (2.0, 4.0)
        } else {
            (4.0, 8.0)
        };

        let likes = item.likes.unwrap_or(0);
        let downloads = item.downloads.unwrap_or(0);
        let trending_score = (likes as f64 * 2.0 + (downloads as f64 / 1000.0)).clamp(10.0, 100.0);

        let name = item.id.split('/').last().unwrap_or(&item.id).to_string();

        HfModelEntry {
            id: item.id.clone(),
            name,
            pipeline_tag: item.pipeline_tag.clone().unwrap_or_else(|| default_tag.to_string()),
            stage: stage.to_string(),
            required_vram_gb: vram,
            required_ram_gb: ram,
            trending_score,
            downloads,
            likes,
            description: format!("Discovered trending HF model for {}", stage),
        }
    }

    /// Evaluate discovered models against live system memory and dynamically upgrade active models.
    pub async fn evaluate_and_upgrade(&self, mem: &SystemMemory) -> Result<HfModelRegistry> {
        let discovered = self.discover_models_online().await;
        let mut candidates_by_stage: HashMap<String, Vec<HfModelEntry>> = HashMap::new();

        for model in discovered {
            candidates_by_stage
                .entry(model.stage.clone())
                .or_default()
                .push(model);
        }

        // Ensure every known stage has candidates
        for stage in &[
            STAGE_GROOVE,
            STAGE_YUE,
            STAGE_VOICE_CONVERSION,
            STAGE_INPAINTING,
            STAGE_MASTERING,
            STAGE_FUSION_TTS,
        ] {
            candidates_by_stage.entry(stage.to_string()).or_default();
        }

        let mut active_models = HashMap::new();
        let mut stage_selections = HashMap::new();

        for (stage, candidates) in &candidates_by_stage {
            // Find highest trending model that safely fits in available GPU VRAM or RAM budget
            let mut fitting: Vec<&HfModelEntry> = candidates
                .iter()
                .filter(|c| {
                    if mem.has_usable_gpu() && c.required_vram_gb <= mem.gpu_budget_gb() {
                        true
                    } else if c.required_ram_gb <= mem.ram_budget_gb() {
                        true
                    } else {
                        // Internal DSP fallback models (0 req VRAM, <= 1 RAM) always fit
                        c.required_vram_gb == 0.0 && c.required_ram_gb <= 1.0
                    }
                })
                .collect();

            fitting.sort_by(|a, b| b.trending_score.partial_cmp(&a.trending_score).unwrap_or(std::cmp::Ordering::Equal));

            let chosen = if let Some(best) = fitting.first() {
                (*best).clone()
            } else {
                Self::fallback_dsp_entry(stage)
            };

            active_models.insert(stage.clone(), chosen.id.clone());
            stage_selections.insert(stage.clone(), chosen);
        }

        let registry = HfModelRegistry {
            last_updated: chrono::Utc::now().to_rfc3339(),
            discovered_count: candidates_by_stage.values().map(|v| v.len()).sum(),
            active_models,
            stage_selections,
            candidates_by_stage,
            system_memory_snapshot: Some(mem.clone()),
        };

        let _ = self.save_registry(&registry);
        Ok(registry)
    }

    /// Curated built-in candidates representing state-of-the-art production music models.
    pub fn curated_builtins() -> Vec<HfModelEntry> {
        vec![
            // Stage 1: Groove Foundation
            HfModelEntry {
                id: "facebook/musicgen-medium".to_string(),
                name: "MusicGen Medium".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_GROOVE.to_string(),
                required_vram_gb: 6.0,
                required_ram_gb: 12.0,
                trending_score: 96.0,
                downloads: 980_000,
                likes: 4_100,
                description: "Meta AudioCraft text-to-music & groove generator".to_string(),
            },
            HfModelEntry {
                id: "facebook/musicgen-small".to_string(),
                name: "MusicGen Small".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_GROOVE.to_string(),
                required_vram_gb: 4.0,
                required_ram_gb: 8.0,
                trending_score: 95.0,
                downloads: 850_000,
                likes: 3_200,
                description: "Meta AudioCraft fast text-to-groove generator".to_string(),
            },
            HfModelEntry {
                id: "ACE-Step/ACE-Step-v1-3.5B".to_string(),
                name: "ACE-Step v1 3.5B".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_GROOVE.to_string(),
                required_vram_gb: 7.0,
                required_ram_gb: 14.0,
                trending_score: 94.0,
                downloads: 160_000,
                likes: 1_250,
                description: "Dedicated foundation rhythm and percussion engine (Apache 2.0)".to_string(),
            },
            Self::fallback_dsp_entry(STAGE_GROOVE),

            // Stage 2: Full-Song & Dual-Stem Synthesis (Google Gemini Lyria 3.5 + MiniMax-Music3 + YuE)
            HfModelEntry {
                id: "models/lyria-3.5".to_string(),
                name: "Google Gemini Lyria 3.5 - Best-Sounding AI Music & Photo-to-Music (.env GEMINI_API_KEY)".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 100.0,
                downloads: 2_400_000,
                likes: 15_800,
                description: "Gemini's best-sounding AI music model: transforms text, poems, or photos into complete 3-minute studio tracks with rich expressive vocals and instrumentals".to_string(),
            },
            HfModelEntry {
                id: "models/lyria-3-pro-preview".to_string(),
                name: "Google Gemini Lyria 3 Pro Preview (.env GEMINI_API_KEY)".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 99.8,
                downloads: 1_450_000,
                likes: 11_200,
                description: "Gemini Lyria 3 Pro full-length vocal and instrumental studio song generator".to_string(),
            },
            HfModelEntry {
                id: "models/lyria-3-clip-preview".to_string(),
                name: "Google Gemini Lyria 3 Clip Preview (.env GEMINI_API_KEY)".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 99.6,
                downloads: 1_100_000,
                likes: 9_400,
                description: "Fast Gemini Lyria 3 clip generator for rapid musical hook and mood prototyping".to_string(),
            },
            HfModelEntry {
                id: "MiniMaxAI/MiniMax-Music3".to_string(),
                name: "MiniMax Music 3 (8B Global + 0.6B Local LLM + Flow-VAE)".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 6.5,
                required_ram_gb: 16.0,
                trending_score: 99.5,
                downloads: 597_600,
                likes: 1_540,
                description: "5-minute full-song generator with Structured Caption control, expressive vocals, and 8GB VRAM leaf-level streaming offload".to_string(),
            },
            HfModelEntry {
                id: "m-a-p/YuE-s1-7B-anneal-en-cot".to_string(),
                name: "YuE s1 7B English CoT".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 14.0,
                required_ram_gb: 28.0,
                trending_score: 98.0,
                downloads: 450_000,
                likes: 4_500,
                description: "Full-song dual-track vocal and accompaniment transformer (Apache 2.0)".to_string(),
            },
            HfModelEntry {
                id: "ASLP-lab/DiffRhythm".to_string(),
                name: "DiffRhythm 44.1kHz Full-Song".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 8.0,
                required_ram_gb: 16.0,
                trending_score: 95.0,
                downloads: 210_000,
                likes: 2_600,
                description: "Fast latent diffusion full-song music & vocal synthesizer (Apache 2.0)".to_string(),
            },
            HfModelEntry {
                id: "m-a-p/YuE-s2-1B-cot".to_string(),
                name: "YuE s2 1B Acoustic".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 6.0,
                required_ram_gb: 12.0,
                trending_score: 93.0,
                downloads: 380_000,
                likes: 2_100,
                description: "High-efficiency acoustic codebook synthesizer".to_string(),
            },
            Self::fallback_dsp_entry(STAGE_YUE),

            // Stage 3: Voice Conversion
            HfModelEntry {
                id: "Plachtaa/Seed-VC".to_string(),
                name: "Seed-VC Tenor (RVC)".to_string(),
                pipeline_tag: "audio-to-audio".to_string(),
                stage: STAGE_VOICE_CONVERSION.to_string(),
                required_vram_gb: 4.0,
                required_ram_gb: 8.0,
                trending_score: 96.0,
                downloads: 390_000,
                likes: 2_400,
                description: "Zero-shot singing voice conversion and vocal timbre fusion (MIT)".to_string(),
            },
            HfModelEntry {
                id: "coqui/XTTS-v2".to_string(),
                name: "XTTS-v2 Studio".to_string(),
                pipeline_tag: "text-to-speech".to_string(),
                stage: STAGE_VOICE_CONVERSION.to_string(),
                required_vram_gb: 5.0,
                required_ram_gb: 10.0,
                trending_score: 96.0,
                downloads: 1_200_000,
                likes: 5_200,
                description: "Voice cloning and expressive pitch-locked timbre delivery".to_string(),
            },
            Self::fallback_dsp_entry(STAGE_VOICE_CONVERSION),

            // Stage 4: Stem Separation & Inpainting
            HfModelEntry {
                id: "facebook/demucs_htdemucs".to_string(),
                name: "Demucs Hybrid Transformer".to_string(),
                pipeline_tag: "audio-to-audio".to_string(),
                stage: STAGE_INPAINTING.to_string(),
                required_vram_gb: 6.0,
                required_ram_gb: 12.0,
                trending_score: 97.0,
                downloads: 2_100_000,
                likes: 6_800,
                description: "Industry-standard drum, bass, and vocal 4-stem separator".to_string(),
            },
            HfModelEntry {
                id: "stabilityai/stable-audio-open-1.0".to_string(),
                name: "Stable Audio Open 1.0".to_string(),
                pipeline_tag: "text-to-audio".to_string(),
                stage: STAGE_INPAINTING.to_string(),
                required_vram_gb: 8.0,
                required_ram_gb: 16.0,
                trending_score: 92.0,
                downloads: 310_000,
                likes: 2_900,
                description: "Virtuoso instrumental solo and harmonic stem inpainting".to_string(),
            },
            Self::fallback_dsp_entry(STAGE_INPAINTING),

            // Stage 5: Analog Tape Mastering
            HfModelEntry {
                id: "haoheliu/AudioSR".to_string(),
                name: "AudioSR Super-Resolution".to_string(),
                pipeline_tag: "audio-to-audio".to_string(),
                stage: STAGE_MASTERING.to_string(),
                required_vram_gb: 4.0,
                required_ram_gb: 8.0,
                trending_score: 90.0,
                downloads: 180_000,
                likes: 1_200,
                description: "48 kHz neural audio bandwidth extension and mastering".to_string(),
            },
            Self::fallback_dsp_entry(STAGE_MASTERING),

            // Stage: Fusion TTS Singing Vocals
            HfModelEntry {
                id: "suno/bark".to_string(),
                name: "Suno Bark Singing".to_string(),
                pipeline_tag: "text-to-speech".to_string(),
                stage: STAGE_FUSION_TTS.to_string(),
                required_vram_gb: 6.0,
                required_ram_gb: 12.0,
                trending_score: 98.0,
                downloads: 1_500_000,
                likes: 8_200,
                description: "Melodic cadence, expressive vibrato, and backing choir (MIT)".to_string(),
            },
            HfModelEntry {
                id: "fishaudio/fish-speech-1.5".to_string(),
                name: "Fish Speech 1.5".to_string(),
                pipeline_tag: "text-to-speech".to_string(),
                stage: STAGE_FUSION_TTS.to_string(),
                required_vram_gb: 4.0,
                required_ram_gb: 8.0,
                trending_score: 97.0,
                downloads: 820_000,
                likes: 6_400,
                description: "SOTA expressive neural vocal and singing synthesis (Apache 2.0)".to_string(),
            },
            HfModelEntry {
                id: "hexgrad/Kokoro-82M".to_string(),
                name: "Kokoro-82M Lead Vocal".to_string(),
                pipeline_tag: "text-to-speech".to_string(),
                stage: STAGE_FUSION_TTS.to_string(),
                required_vram_gb: 2.0,
                required_ram_gb: 4.0,
                trending_score: 99.0,
                downloads: 950_000,
                likes: 7_100,
                description: "Ultra-crisp English lead vocal articulation at 48 kHz (Apache 2.0)".to_string(),
            },
            HfModelEntry {
                id: "SWivid/F5-TTS".to_string(),
                name: "F5-TTS Expressive".to_string(),
                pipeline_tag: "text-to-speech".to_string(),
                stage: STAGE_FUSION_TTS.to_string(),
                required_vram_gb: 4.0,
                required_ram_gb: 8.0,
                trending_score: 96.0,
                downloads: 420_000,
                likes: 3_800,
                description: "Non-autoregressive lead singer with breath dynamics".to_string(),
            },
            Self::fallback_dsp_entry(STAGE_FUSION_TTS),
        ]
    }

    /// Internal Pure-Rust DSP fallback entry for a stage.
    pub fn fallback_dsp_entry(stage: &str) -> HfModelEntry {
        match stage {
            STAGE_GROOVE => HfModelEntry {
                id: "InternalPureRust/GrooveResonator".to_string(),
                name: "DSP Groove Resonator (Zero Latency)".to_string(),
                pipeline_tag: "audio-dsp".to_string(),
                stage: STAGE_GROOVE.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 85.0,
                downloads: 0,
                likes: 0,
                description: "Pure-Rust 48 kHz band-pass shaped acoustic percussion".to_string(),
            },
            STAGE_YUE => HfModelEntry {
                id: "InternalPureRust/MelodicVocoder".to_string(),
                name: "DSP Melodic Vocoder".to_string(),
                pipeline_tag: "audio-dsp".to_string(),
                stage: STAGE_YUE.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 85.0,
                downloads: 0,
                likes: 0,
                description: "Pure-Rust resonant formants with harmonic vocal carrier".to_string(),
            },
            STAGE_VOICE_CONVERSION => HfModelEntry {
                id: "InternalPureRust/FormantMorpher".to_string(),
                name: "DSP Formant Morpher".to_string(),
                pipeline_tag: "audio-dsp".to_string(),
                stage: STAGE_VOICE_CONVERSION.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 85.0,
                downloads: 0,
                likes: 0,
                description: "Pure-Rust state-variable formant transformation".to_string(),
            },
            STAGE_INPAINTING => HfModelEntry {
                id: "InternalPureRust/KarplusStrongWah".to_string(),
                name: "DSP Karplus-Strong Wah Soloist".to_string(),
                pipeline_tag: "audio-dsp".to_string(),
                stage: STAGE_INPAINTING.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 88.0,
                downloads: 0,
                likes: 0,
                description: "Phase-continuous physical string synthesis with wah-wah filter".to_string(),
            },
            STAGE_MASTERING => HfModelEntry {
                id: "InternalPureRust/StuderA800Limiter".to_string(),
                name: "DSP Studer A800 Mastering".to_string(),
                pipeline_tag: "audio-dsp".to_string(),
                stage: STAGE_MASTERING.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 90.0,
                downloads: 0,
                likes: 0,
                description: "Analog tape saturation curve with -0.3 dBFS brickwall peak ceiling".to_string(),
            },
            _ => HfModelEntry {
                id: "InternalPureRust/MelodicSingingChoir".to_string(),
                name: "DSP Melodic Singing Choir (48 kHz)".to_string(),
                pipeline_tag: "audio-dsp".to_string(),
                stage: STAGE_FUSION_TTS.to_string(),
                required_vram_gb: 0.0,
                required_ram_gb: 0.5,
                trending_score: 95.0,
                downloads: 0,
                likes: 0,
                description: "Pure-Rust 48 kHz scale-locked melodic singing synthesizer with 3-part harmony choir and studio reverb".to_string(),
            },
        }
    }

    /// Construct a default registry based on live hardware sizing.
    fn default_registry(&self, mem: &SystemMemory) -> HfModelRegistry {
        let mut candidates_by_stage: HashMap<String, Vec<HfModelEntry>> = HashMap::new();
        for cand in Self::curated_builtins() {
            candidates_by_stage
                .entry(cand.stage.clone())
                .or_default()
                .push(cand);
        }

        let mut active_models = HashMap::new();
        let mut stage_selections = HashMap::new();

        for (stage, candidates) in &candidates_by_stage {
            let mut fitting: Vec<&HfModelEntry> = candidates
                .iter()
                .filter(|c| {
                    if mem.has_usable_gpu() && c.required_vram_gb <= mem.gpu_budget_gb() {
                        true
                    } else if c.required_ram_gb <= mem.ram_budget_gb() {
                        true
                    } else {
                        c.required_vram_gb == 0.0 && c.required_ram_gb <= 1.0
                    }
                })
                .collect();

            fitting.sort_by(|a, b| b.trending_score.partial_cmp(&a.trending_score).unwrap_or(std::cmp::Ordering::Equal));

            let chosen = if let Some(best) = fitting.first() {
                (*best).clone()
            } else {
                Self::fallback_dsp_entry(stage)
            };

            active_models.insert(stage.clone(), chosen.id.clone());
            stage_selections.insert(stage.clone(), chosen);
        }

        HfModelRegistry {
            last_updated: chrono::Utc::now().to_rfc3339(),
            discovered_count: candidates_by_stage.values().map(|v| v.len()).sum(),
            active_models,
            stage_selections,
            candidates_by_stage,
            system_memory_snapshot: Some(mem.clone()),
        }
    }
}
