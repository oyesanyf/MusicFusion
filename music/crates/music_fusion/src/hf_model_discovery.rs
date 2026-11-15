//! Dynamic Hugging Face Open-Weight Music, Vocal & Nimble Decision Model Discovery,
//! Commercial License Filtering, Cache Verification, and Live Hardware Routing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::memory::SystemMemory;

/// Pipeline stages for music generation, decision adjudication, and processing.
pub const STAGE_DECISION: &str = "Stage0Decision";
pub const STAGE_GROOVE: &str = "Stage1Groove";
pub const STAGE_YUE: &str = "Stage2YuE";
pub const STAGE_VOICE_CONVERSION: &str = "Stage3VoiceConversion";
pub const STAGE_INPAINTING: &str = "Stage4Inpainting";
pub const STAGE_MASTERING: &str = "Stage5Mastering";
pub const STAGE_FUSION_TTS: &str = "FusionTts";

/// License tiers explicitly governing open-weight synthesis and decision models.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelLicenseTier {
    #[default]
    Mit,
    Apache2,
    CcBy4,
    CommunityCommercial,
    CcByNc4,
    NonCommercial,
}

impl ModelLicenseTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelLicenseTier::Mit => "MIT",
            ModelLicenseTier::Apache2 => "Apache-2.0",
            ModelLicenseTier::CcBy4 => "CC-BY-4.0",
            ModelLicenseTier::CommunityCommercial => "CommunityCommercial",
            ModelLicenseTier::CcByNc4 => "CC-BY-NC-4.0",
            ModelLicenseTier::NonCommercial => "NonCommercial",
        }
    }

    pub fn is_commercial_allowed(&self) -> bool {
        !matches!(
            self,
            ModelLicenseTier::CcByNc4 | ModelLicenseTier::NonCommercial
        )
    }
}

/// Represents an open-weight local or discoverable Hugging Face model entry.
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
    #[serde(default)]
    pub license_tier: ModelLicenseTier,
    #[serde(default = "default_true")]
    pub commercial_allowed: bool,
    #[serde(default)]
    pub min_vram_with_offload_gb: f64,
    #[serde(default)]
    pub prefer_cpu_ram_offload: bool,
    #[serde(default)]
    pub family: String,
}

fn default_true() -> bool {
    true
}

impl HfModelEntry {
    /// Returns true if the checkpoint fits in live VRAM, offloaded VRAM+RAM, or CPU RAM.
    pub fn fits_hardware(&self, mem: &SystemMemory) -> bool {
        if self.prefer_cpu_ram_offload && mem.ram_budget_gb() >= self.required_ram_gb {
            return true;
        }
        if mem.has_usable_gpu() {
            let eff_vram = if self.min_vram_with_offload_gb > 0.0 && mem.ram_budget_gb() >= self.required_ram_gb {
                self.min_vram_with_offload_gb
            } else {
                self.required_vram_gb
            };
            if mem.gpu_budget_gb() >= eff_vram || mem.gpu_vram_total_gb >= eff_vram {
                return true;
            }
        }
        mem.ram_budget_gb() >= self.required_ram_gb
            || (self.required_vram_gb == 0.0 && self.required_ram_gb <= 1.0)
    }

    /// Checks whether this Hugging Face model folder or GGUF weight exists in `models_dir`.
    pub fn is_cached_in(&self, models_dir: &Path) -> bool {
        if self.id.starts_with("InternalPureRust/") {
            return true;
        }
        is_hf_repo_cached(&self.id, models_dir)
    }
}

/// Persistent registry of active and candidate open-weight Hugging Face models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HfModelRegistry {
    pub last_updated: String,
    pub discovered_count: usize,
    pub active_models: HashMap<String, String>,
    pub stage_selections: HashMap<String, HfModelEntry>,
    pub candidates_by_stage: HashMap<String, Vec<HfModelEntry>>,
    pub system_memory_snapshot: Option<SystemMemory>,
}

/// UI/REST catalog projection including local cache status and hardware fit telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenWeightCatalogItem {
    pub entry: HfModelEntry,
    pub cached_on_disk: bool,
    pub fits_hardware: bool,
    pub blocked_by_commercial_filter: bool,
}

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

/// Checks standard Hugging Face cache directory conventions inside `models_dir`.
pub fn is_hf_repo_cached(repo_id: &str, models_dir: &Path) -> bool {
    let sanitized = repo_id.replace('/', "--");
    let direct_hub = models_dir.join("hub").join(format!("models--{}", sanitized));
    let flat_hub = models_dir.join(format!("models--{}", sanitized));
    let named = models_dir.join(repo_id.replace('/', "_"));
    let short_dir = models_dir.join(repo_id.split('/').next_back().unwrap_or(repo_id));
    [direct_hub, flat_hub, named, short_dir]
        .iter()
        .any(|p| p.exists())
}

/// Resolves the open-weight checkpoint root directory (`MUSIC_MODELS_DIR` -> `HF_HOME` -> `D:\music\models` on Windows).
pub fn default_models_cache_dir() -> PathBuf {
    for key in ["MUSIC_MODELS_DIR", "HF_HOME"] {
        if let Ok(val) = std::env::var(key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
    }
    if cfg!(windows) {
        PathBuf::from(r"D:\music\models")
    } else {
        PathBuf::from("models")
    }
}

/// Helper builder to construct catalog entries without field repetition.
#[allow(clippy::too_many_arguments)]
fn builtin(
    id: &str,
    name: &str,
    family: &str,
    pipeline_tag: &str,
    stage: &str,
    required_vram_gb: f64,
    min_vram_with_offload_gb: f64,
    required_ram_gb: f64,
    prefer_cpu_ram_offload: bool,
    license_tier: ModelLicenseTier,
    trending_score: f64,
    downloads: u64,
    likes: u64,
    description: &str,
) -> HfModelEntry {
    HfModelEntry {
        id: id.to_string(),
        name: name.to_string(),
        pipeline_tag: pipeline_tag.to_string(),
        stage: stage.to_string(),
        required_vram_gb,
        required_ram_gb,
        trending_score,
        downloads,
        likes,
        description: description.to_string(),
        license_tier,
        commercial_allowed: license_tier.is_commercial_allowed(),
        min_vram_with_offload_gb,
        prefer_cpu_ram_offload,
        family: family.to_string(),
    }
}

/// Hugging Face Open-Weight Model Discovery, Selection & Upgrade Engine.
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

    pub fn load_registry(&self, mem: &SystemMemory) -> HfModelRegistry {
        if self.storage_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.storage_path) {
                if let Ok(reg) = serde_json::from_str::<HfModelRegistry>(&content) {
                    // Reject stale cached registries containing invalid or proprietary endpoints
                    let has_invalid = reg
                        .candidates_by_stage
                        .values()
                        .flatten()
                        .any(|m| !Self::is_valid_open_weight_model_id(&m.id));
                    if !has_invalid {
                        return reg;
                    }
                }
            }
        }
        self.default_registry(mem)
    }

    pub fn save_registry(&self, registry: &HfModelRegistry) -> Result<()> {
        if let Some(parent) = self.storage_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(registry)?;
        std::fs::write(&self.storage_path, json)?;
        Ok(())
    }

    /// Queries Hugging Face Hub API for trending open-weight models, merged with curated built-ins.
    pub async fn discover_models_online(&self) -> Vec<HfModelEntry> {
        let mut discovered = Vec::new();

        if let Ok(models) = self.query_hf_category("text-to-audio", "music", STAGE_GROOVE).await {
            discovered.extend(models);
        }
        if let Ok(models) = self.query_hf_category("text-to-speech", "singing", STAGE_FUSION_TTS).await {
            discovered.extend(models);
        }
        if let Ok(models) = self.query_hf_search("demucs", STAGE_INPAINTING).await {
            discovered.extend(models);
        }

        for default_cand in Self::curated_builtins() {
            if !discovered.iter().any(|d| d.id == default_cand.id) {
                discovered.push(default_cand);
            }
        }

        discovered
    }

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
        Ok(items
            .into_iter()
            .filter(|i| Self::is_valid_open_weight_model_id(&i.id))
            .map(|i| Self::api_item_to_entry(&i, stage, tag))
            .collect())
    }

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
        Ok(items
            .into_iter()
            .filter(|i| Self::is_valid_open_weight_model_id(&i.id))
            .map(|i| Self::api_item_to_entry(&i, stage, "audio-to-audio"))
            .collect())
    }

    fn api_item_to_entry(item: &HfApiItem, stage: &str, default_tag: &str) -> HfModelEntry {
        let id_lower = item.id.to_lowercase();
        let (vram, ram) = if id_lower.contains("minimax-music3") || id_lower.contains("minimax_music3") {
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
        let name = item.id.split('/').next_back().unwrap_or(&item.id).to_string();

        builtin(
            &item.id,
            &name,
            "Community",
            item.pipeline_tag.as_deref().unwrap_or(default_tag),
            stage,
            vram,
            (vram * 0.75).max(2.0),
            ram,
            false,
            ModelLicenseTier::Apache2,
            trending_score,
            downloads,
            likes,
            &format!("Discovered open-weight HF model for {}", stage),
        )
    }

    pub async fn evaluate_and_upgrade(&self, mem: &SystemMemory) -> Result<HfModelRegistry> {
        let discovered = self.discover_models_online().await;
        let registry = self.build_registry_from_entries(discovered, mem);
        let _ = self.save_registry(&registry);
        Ok(registry)
    }

    /// Lists curated open-weight music + Nimble decision models with cache & license flags.
    pub fn list_open_weight_catalog(
        mem: &SystemMemory,
        commercial_only: bool,
        models_dir: &Path,
    ) -> Vec<OpenWeightCatalogItem> {
        Self::curated_builtins()
            .into_iter()
            .filter(|e| !e.id.starts_with("InternalPureRust/"))
            .map(|entry| {
                let cached = entry.is_cached_in(models_dir);
                let fits = entry.fits_hardware(mem);
                let blocked = commercial_only && !entry.commercial_allowed;
                OpenWeightCatalogItem {
                    entry,
                    cached_on_disk: cached,
                    fits_hardware: fits,
                    blocked_by_commercial_filter: blocked,
                }
            })
            .collect()
    }

    /// Resolves any canonical HF repo ID or short alias (e.g. `ace-step`, `levo2`, `nimble`).
    fn resolve_alias(lower: &str) -> Option<&'static str> {
        match lower {
            "ace-step" | "acestep" | "ace-step-1.5" | "acestep-v15-turbo" => Some("ACE-Step/acestep-v15-turbo"),
            "ace-step-base" | "acestep-v15-base" => Some("ACE-Step/acestep-v15-base"),
            "ace-step-xl" | "acestep-v15-xl-turbo" | "4b-xl" => Some("ACE-Step/acestep-v15-xl-turbo"),
            "levo2" | "levo-2" | "songgeneration" => Some("tencent/SongGeneration"),
            "levo2-large" | "songgeneration-v2-large" => Some("lglg666/SongGeneration-v2-large"),
            "yue2" | "yue2-3b" => Some("m-a-p/YuE2-3B"),
            "yue" | "yue-7b" => Some("m-a-p/YuE-s1-7B-anneal-en-cot"),
            "minimax" | "minimax-music3" | "music3" => Some("MiniMaxAI/MiniMax-Music3"),
            "diffrhythm" | "diffrhythm2" | "diffrhythm-2" => Some("ASLP-lab/DiffRhythm2"),
            "muse" | "openmusic-muse" => Some("openmusic/Muse"),
            "stable-audio" | "stable-audio-small" => Some("stabilityai/stable-audio-open-small"),
            "stable-audio-1.0" | "stable-audio-open" => Some("stabilityai/stable-audio-open-1.0"),
            "stable-audio-3" | "stable-audio-3.0" => Some("stabilityai/stable-audio-3-medium"),
            "musicgen" | "musicgen-large" => Some("facebook/musicgen-large"),
            "magenta" | "magenta-realtime-2" => Some("google/magenta-realtime-2"),
            "nimble" | "nimble-9b" | "bespoke-nimble-9b" => Some("bespokelabs/Bespoke-Nimble-9B"),
            "nimble-v3" | "nimble-9b-v3" => Some("bespokelabs/Bespoke-Nimble-9B-v3"),
            "nimble-v3-gguf" | "nimble-gguf" => Some("ggml-org/Bespoke-Nimble-9B-v3-GGUF"),
            "nimble-27b-v3" | "nimble-27b" => Some("bespokelabs/Bespoke-Nimble-27B-v3"),
            "nimble-v4-max" | "nimble-v4" => Some("bespokelabs/Nimble-v4-max"),
            _ => None,
        }
    }

    /// Fail-closed check ensuring identifiers are valid open-weight HF `owner/repo`
    /// slugs or recognized curated open-weight aliases (never `models/...` cloud endpoints).
    pub fn is_valid_open_weight_model_id(id: &str) -> bool {
        let trimmed = id.trim();
        if trimmed.is_empty()
            || trimmed.contains("..")
            || trimmed.contains('\\')
            || trimmed.contains(':')
            || trimmed.starts_with('/')
            || trimmed.ends_with('/')
            || trimmed.chars().any(char::is_whitespace)
        {
            return false;
        }
        let lower = trimmed.to_ascii_lowercase();
        const FORBIDDEN_PREFIXES: &[&str] = &[
            "models/",
            "publishers/",
            "projects/",
            "http://",
            "https://",
            "gs://",
            "s3://",
        ];
        if FORBIDDEN_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            return false;
        }
        if let Some((owner, repo)) = trimmed.split_once('/') {
            if repo.contains('/') || owner.is_empty() || repo.is_empty() {
                return false;
            }
            let valid_seg = |seg: &str| {
                !matches!(seg, "." | "..")
                    && seg
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            };
            return valid_seg(owner) && valid_seg(repo);
        }
        Self::resolve_alias(&lower).is_some()
            || Self::curated_builtins()
                .iter()
                .any(|e| e.id.eq_ignore_ascii_case(trimmed))
    }

    pub fn resolve_model_entry(query: &str) -> Option<HfModelEntry> {
        let q = query.trim().to_ascii_lowercase();
        let all = Self::curated_builtins();
        if let Some(exact) = all.iter().find(|e| e.id.to_ascii_lowercase() == q) {
            return Some(exact.clone());
        }
        if let Some(alias) = Self::resolve_alias(&q) {
            if let Some(m) = all.iter().find(|e| e.id == alias) {
                return Some(m.clone());
            }
        }
        all.into_iter().find(|e| e.id.to_ascii_lowercase().contains(&q) || e.name.to_ascii_lowercase().contains(&q))
    }

    /// Curated 100% open-weight synthesis, vocal, separation, and Nimble decision checkpoints.
    pub fn curated_builtins() -> Vec<HfModelEntry> {
        vec![
            // === Decision Engine: Local Bespoke Nimble Tiers (Zero Cloud APIs) ===
            builtin(
                "bespokelabs/Bespoke-Nimble-9B",
                "Bespoke Nimble 9B (Commercial Apache-2.0)",
                "Nimble Decision",
                "text-classification",
                STAGE_DECISION,
                5.5,
                0.0,
                6.5,
                true,
                ModelLicenseTier::Apache2,
                100.0,
                180_000,
                2_400,
                "Single-forward-pass calibrated typed decision model (Apache-2.0, CPU/RAM GGUF safe alongside 6-8 GB GPU audio models)",
            ),
            builtin(
                "bespokelabs/Bespoke-Nimble-9B-v3",
                "Bespoke Nimble 9B v3 (High-Accuracy Research)",
                "Nimble Decision",
                "text-classification",
                STAGE_DECISION,
                5.5,
                0.0,
                6.5,
                true,
                ModelLicenseTier::CcByNc4,
                98.0,
                120_000,
                1_900,
                "Calibrated single-pass typed adjudicator v3 (CC-BY-NC-4.0, blocked when commercial_only=true)",
            ),
            builtin(
                "ggml-org/Bespoke-Nimble-9B-v3-GGUF",
                "Bespoke Nimble 9B v3 GGUF (CPU/RAM Quantized)",
                "Nimble Decision",
                "text-classification",
                STAGE_DECISION,
                0.0,
                0.0,
                6.0,
                true,
                ModelLicenseTier::CcByNc4,
                97.0,
                95_000,
                1_400,
                "Q4_K_M CPU-offloaded single-pass GGUF decision checkpoint (CC-BY-NC-4.0)",
            ),
            builtin(
                "bespokelabs/Bespoke-Nimble-27B-v3",
                "Bespoke Nimble 27B v3 (Workstation High-RAM)",
                "Nimble Decision",
                "text-classification",
                STAGE_DECISION,
                16.0,
                0.0,
                18.5,
                true,
                ModelLicenseTier::CcByNc4,
                95.0,
                60_000,
                1_100,
                "Large 27B single-pass typed rubric adjudicator for high-RAM workstations (CC-BY-NC-4.0)",
            ),
            builtin(
                "bespokelabs/Nimble-v4-max",
                "Bespoke Nimble v4 Max (27B Qwen3.8 Base)",
                "Nimble Decision",
                "text-classification",
                STAGE_DECISION,
                18.0,
                0.0,
                24.0,
                true,
                ModelLicenseTier::CommunityCommercial,
                94.5,
                40_000,
                950,
                "High-capacity Nimble v4 checkpoint running via CPU/RAM offload on workstation hosts",
            ),
            Self::fallback_dsp_entry(STAGE_DECISION),

            // === Family 1: ACE-Step 1.5 (Default MIT Anchor) ===
            builtin(
                "ACE-Step/acestep-v15-turbo",
                "ACE-Step 1.5 Turbo (2B MIT Default)",
                "ACE-Step 1.5",
                "text-to-audio",
                STAGE_YUE,
                3.8,
                3.2,
                8.0,
                false,
                ModelLicenseTier::Mit,
                100.0,
                2_100_000,
                14_500,
                "Fast all-round open-weight song & vocal generator fitting under 4 GB VRAM (6 GB GTX 1060 & 8 GB RTX 4000)",
            ),
            builtin(
                "ACE-Step/acestep-v15-base",
                "ACE-Step 1.5 Base (2B MIT)",
                "ACE-Step 1.5",
                "text-to-audio",
                STAGE_YUE,
                4.0,
                3.5,
                8.0,
                false,
                ModelLicenseTier::Mit,
                97.5,
                920_000,
                6_200,
                "Full controllability 2B base checkpoint for vocals, lyrics & stems under MIT license",
            ),
            builtin(
                "ACE-Step/acestep-v15-xl-turbo",
                "ACE-Step 1.5 XL Turbo (4B MIT)",
                "ACE-Step 1.5",
                "text-to-audio",
                STAGE_YUE,
                8.5,
                5.5,
                16.0,
                false,
                ModelLicenseTier::Mit,
                98.5,
                640_000,
                5_400,
                "High-detail 4B XL song generator using sequential CPU offload on 6-8 GB GPUs",
            ),

            // === Family 2: Tencent LeVo 2 (SongGeneration v2) ===
            builtin(
                "tencent/SongGeneration",
                "Tencent LeVo 2 / SongGeneration",
                "LeVo 2",
                "text-to-audio",
                STAGE_YUE,
                10.0,
                6.0,
                20.0,
                false,
                ModelLicenseTier::NonCommercial,
                99.0,
                780_000,
                7_900,
                "Highest blind acoustic & vocal realism across rock, jazz, and pop (non-commercial study license)",
            ),
            builtin(
                "lglg666/SongGeneration-v2-large",
                "LeVo 2 SongGeneration v2 Large",
                "LeVo 2",
                "text-to-audio",
                STAGE_YUE,
                12.0,
                6.5,
                24.0,
                false,
                ModelLicenseTier::NonCommercial,
                98.0,
                420_000,
                4_300,
                "Flagship large LeVo 2 vocal phrasing checkpoint with sequential offload (non-commercial)",
            ),

            // === Family 3: YuE2 & YuE ===
            builtin(
                "m-a-p/YuE2-3B",
                "YuE2 3B Full-Song Vocal Generator",
                "YuE2",
                "text-to-audio",
                STAGE_YUE,
                8.0,
                5.5,
                18.0,
                false,
                ModelLicenseTier::CcByNc4,
                98.8,
                690_000,
                5_800,
                "Improved multi-minute vocal song coherence and structure (CC-BY-NC-4.0)",
            ),
            builtin(
                "m-a-p/YuE-s1-7B-anneal-en-cot",
                "YuE s1 7B English CoT",
                "YuE2",
                "text-to-audio",
                STAGE_YUE,
                14.0,
                6.5,
                28.0,
                false,
                ModelLicenseTier::CcByNc4,
                95.0,
                450_000,
                4_500,
                "Original dual-track YuE 7B semantic stage (CC-BY-NC-4.0)",
            ),
            builtin(
                "m-a-p/YuE-s2-1B-cot",
                "YuE s2 1B Acoustic Decoder",
                "YuE2",
                "text-to-audio",
                STAGE_YUE,
                6.0,
                4.5,
                12.0,
                false,
                ModelLicenseTier::CcByNc4,
                92.0,
                380_000,
                2_100,
                "YuE stage-2 acoustic reconstruction codec",
            ),

            // === Family 4: MiniMax-Music3 ===
            builtin(
                "MiniMaxAI/MiniMax-Music3",
                "MiniMax Music 3 (Expressive 5-Min Vocal Songs)",
                "MiniMax-Music3",
                "text-to-audio",
                STAGE_YUE,
                8.0,
                6.0,
                20.0,
                false,
                ModelLicenseTier::CommunityCommercial,
                99.2,
                597_600,
                3_540,
                "5-minute vocal song generation with leaf-level streaming group offload (commercial under $20M revenue)",
            ),

            // === Family 5: DiffRhythm 2 ===
            builtin(
                "ASLP-lab/DiffRhythm2",
                "DiffRhythm 2 (Fast Apache-2.0 Full Track)",
                "DiffRhythm 2",
                "text-to-audio",
                STAGE_YUE,
                6.0,
                4.5,
                12.0,
                false,
                ModelLicenseTier::Apache2,
                96.5,
                340_000,
                3_100,
                "Rapid non-autoregressive block flow-matching + 5 Hz VAE song diffusion (Apache-2.0)",
            ),

            // === Family 6: Muse (LoRA-Ready MIT/Apache-2.0) ===
            builtin(
                "openmusic/Muse",
                "Muse Open Songwriting & LoRA Baseline",
                "Muse",
                "text-to-audio",
                STAGE_YUE,
                6.5,
                4.8,
                14.0,
                false,
                ModelLicenseTier::Apache2,
                95.5,
                230_000,
                2_600,
                "Open lyric-to-audio alignment model supporting custom LoRA adapters (MIT code / Apache-2.0 weights)",
            ),

            // === Family 7: Stable Audio Open / 3.0 ===
            builtin(
                "stabilityai/stable-audio-open-small",
                "Stable Audio Open Small (Fast Percussion & Loops)",
                "Stable Audio",
                "text-to-audio",
                STAGE_GROOVE,
                3.2,
                2.5,
                6.0,
                false,
                ModelLicenseTier::CommunityCommercial,
                97.0,
                520_000,
                4_100,
                "Fast Creative Commons trained instrumental beds and percussion (commercial under $1M revenue)",
            ),
            builtin(
                "stabilityai/stable-audio-open-1.0",
                "Stable Audio Open 1.0 (Stereo Instrumentals)",
                "Stable Audio",
                "text-to-audio",
                STAGE_GROOVE,
                6.0,
                4.2,
                12.0,
                false,
                ModelLicenseTier::CommunityCommercial,
                95.0,
                610_000,
                3_900,
                "44.1/48 kHz stereo instrumental beds & solos (commercial under $1M revenue)",
            ),
            builtin(
                "stabilityai/stable-audio-3-medium",
                "Stable Audio 3.0 Medium",
                "Stable Audio",
                "text-to-audio",
                STAGE_GROOVE,
                7.5,
                5.0,
                14.0,
                false,
                ModelLicenseTier::CommunityCommercial,
                96.0,
                280_000,
                2_400,
                "Next-gen clean instrumental composition and acoustic arrangement beds",
            ),

            // === Family 8: Meta MusicGen Large (3.3B) ===
            builtin(
                "facebook/musicgen-large",
                "Meta MusicGen Large (3.3B Text-to-Instrumental)",
                "MusicGen Large",
                "text-to-audio",
                STAGE_GROOVE,
                12.0,
                6.0,
                20.0,
                false,
                ModelLicenseTier::CcByNc4,
                94.0,
                1_150_000,
                6_900,
                "High-detail 3.3B text-conditioned instrumental composition in pure text mode (CC-BY-NC-4.0)",
            ),

            // === Family 9: Google Magenta RealTime 2 ===
            builtin(
                "google/magenta-realtime-2",
                "Google Magenta RealTime 2 (Low-Latency Jamming)",
                "Magenta RealTime 2",
                "text-to-audio",
                STAGE_GROOVE,
                4.5,
                3.5,
                8.0,
                false,
                ModelLicenseTier::CcBy4,
                96.8,
                310_000,
                3_400,
                "Chunked low-latency interactive instrumental streaming (Apache-2.0 code / CC-BY-4.0 weights)",
            ),
            Self::fallback_dsp_entry(STAGE_GROOVE),
            Self::fallback_dsp_entry(STAGE_YUE),

            // === Stage 3 & TTS Singing Support (100% Open-Weight Local) ===
            builtin(
                "hexgrad/Kokoro-82M",
                "Kokoro-82M Lead Vocal",
                "Kokoro",
                "text-to-speech",
                STAGE_FUSION_TTS,
                1.5,
                0.0,
                2.5,
                true,
                ModelLicenseTier::Apache2,
                99.0,
                950_000,
                7_100,
                "Crisp 48 kHz articulated lead singing vocal synthesis (Apache-2.0)",
            ),
            builtin(
                "SWivid/F5-TTS",
                "F5-TTS Expressive",
                "F5-TTS",
                "text-to-speech",
                STAGE_FUSION_TTS,
                4.0,
                2.8,
                8.0,
                false,
                ModelLicenseTier::Mit,
                96.0,
                420_000,
                3_800,
                "Non-autoregressive lead vocal synthesis with breath dynamics (MIT)",
            ),
            Self::fallback_dsp_entry(STAGE_FUSION_TTS),
            Self::fallback_dsp_entry(STAGE_VOICE_CONVERSION),

            // === Stage 4: Clean 4-Stem Separation ===
            builtin(
                "facebook/demucs_htdemucs",
                "Demucs Hybrid Transformer (htdemucs)",
                "Demucs",
                "audio-to-audio",
                STAGE_INPAINTING,
                4.0,
                2.0,
                8.0,
                false,
                ModelLicenseTier::Mit,
                99.0,
                2_100_000,
                6_800,
                "Clean 48 kHz 4-stem separation into vocals, drums, bass, and guitar/keys (MIT)",
            ),
            Self::fallback_dsp_entry(STAGE_INPAINTING),
            Self::fallback_dsp_entry(STAGE_MASTERING),
        ]
    }

    /// Internal Pure-Rust fallback entry for a stage (zero downloads, zero VRAM).
    pub fn fallback_dsp_entry(stage: &str) -> HfModelEntry {
        match stage {
            STAGE_DECISION => builtin(
                "InternalPureRust/NimbleSystem1Router",
                "Nimble Pure-Rust Calibrated System-1 Router (<2ms)",
                "Nimble Decision",
                "text-classification",
                STAGE_DECISION,
                0.0,
                0.0,
                0.2,
                true,
                ModelLicenseTier::Mit,
                90.0,
                0,
                0,
                "Zero-VRAM pure-Rust calibrated softmax decision router (<2 ms fallback)",
            ),
            STAGE_GROOVE => builtin(
                "InternalPureRust/GrooveResonator",
                "DSP Groove Resonator (Zero Latency)",
                "Pure-Rust DSP",
                "audio-dsp",
                STAGE_GROOVE,
                0.0,
                0.0,
                0.5,
                true,
                ModelLicenseTier::Mit,
                85.0,
                0,
                0,
                "Pure-Rust 48 kHz multi-genre drum & bass groove synthesizer",
            ),
            STAGE_YUE => builtin(
                "InternalPureRust/MelodicVocoder",
                "DSP Multi-Genre Studio Synthesizer",
                "Pure-Rust DSP",
                "audio-dsp",
                STAGE_YUE,
                0.0,
                0.0,
                0.5,
                true,
                ModelLicenseTier::Mit,
                85.0,
                0,
                0,
                "Pure-Rust 48 kHz multi-genre studio synthesizer & formant singing engine",
            ),
            STAGE_VOICE_CONVERSION => builtin(
                "InternalPureRust/FormantMorpher",
                "DSP Formant Morpher",
                "Pure-Rust DSP",
                "audio-dsp",
                STAGE_VOICE_CONVERSION,
                0.0,
                0.0,
                0.5,
                true,
                ModelLicenseTier::Mit,
                85.0,
                0,
                0,
                "Pure-Rust state-variable formant sculpting",
            ),
            STAGE_INPAINTING => builtin(
                "InternalPureRust/KarplusStrongWah",
                "DSP Karplus-Strong Soloist",
                "Pure-Rust DSP",
                "audio-dsp",
                STAGE_INPAINTING,
                0.0,
                0.0,
                0.5,
                true,
                ModelLicenseTier::Mit,
                88.0,
                0,
                0,
                "Phase-continuous physical string synthesis & stem isolator",
            ),
            STAGE_MASTERING => builtin(
                "InternalPureRust/StuderA800Limiter",
                "DSP 48 kHz Studio Console & Limiter",
                "Pure-Rust DSP",
                "audio-dsp",
                STAGE_MASTERING,
                0.0,
                0.0,
                0.5,
                true,
                ModelLicenseTier::Mit,
                92.0,
                0,
                0,
                "Full 20 Hz-20 kHz studio console summing, Studer tape warmth & -0.3 dBFS brickwall ceiling",
            ),
            _ => builtin(
                "InternalPureRust/MelodicSingingChoir",
                "DSP Melodic Singing Choir (48 kHz)",
                "Pure-Rust DSP",
                "audio-dsp",
                STAGE_FUSION_TTS,
                0.0,
                0.0,
                0.5,
                true,
                ModelLicenseTier::Mit,
                95.0,
                0,
                0,
                "Pure-Rust 48 kHz scale-locked melodic singing synthesizer with 3-part harmony choir",
            ),
        }
    }

    fn build_registry_from_entries(&self, entries: Vec<HfModelEntry>, mem: &SystemMemory) -> HfModelRegistry {
        let mut candidates_by_stage: HashMap<String, Vec<HfModelEntry>> = HashMap::new();
        for model in entries {
            candidates_by_stage.entry(model.stage.clone()).or_default().push(model);
        }

        for stage in [
            STAGE_DECISION,
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
            let mut fitting: Vec<&HfModelEntry> = candidates
                .iter()
                .filter(|c| c.fits_hardware(mem))
                .collect();

            fitting.sort_by(|a, b| {
                b.trending_score
                    .partial_cmp(&a.trending_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let chosen = fitting
                .first()
                .map(|c| (*c).clone())
                .unwrap_or_else(|| Self::fallback_dsp_entry(stage));

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

    fn default_registry(&self, mem: &SystemMemory) -> HfModelRegistry {
        self.build_registry_from_entries(Self::curated_builtins(), mem)
    }
}

/// Resolves a workspace worker script path reliably regardless of process working directory
/// (repo root, parent workspace directory, or crate test directory under `cargo test`).
pub fn resolve_worker_script(relative_worker: &str) -> std::path::PathBuf {
    let normalized = relative_worker.trim_start_matches(['/', '\\']);
    let direct = std::path::PathBuf::from(normalized);
    if direct.is_file() {
        return direct;
    }
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.extend(cwd.ancestors().take(5).map(std::path::PathBuf::from));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.extend(parent.ancestors().take(6).map(std::path::PathBuf::from));
        }
    }
    for root in roots {
        let candidate = root.join(normalized);
        if candidate.is_file() {
            return candidate;
        }
        let sub_candidate = root.join("music").join(normalized);
        if sub_candidate.is_file() {
            return sub_candidate;
        }
    }
    direct
}
