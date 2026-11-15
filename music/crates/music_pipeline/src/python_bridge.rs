//! Secure Python / `uv` Subprocess Bridge for Open-Weight Hugging Face Music Models,
//! CUDA `cu124` Venv Bootstrapping, Process-Scoped `HF_HOME`, Stem Separation, and Local Nimble Inference.
//!
//! 100% Open-Weight & Local: Zero paid Gemini/Lyria endpoints, zero audio reference WAV args,
//! and strict zero-touch system/user PATH invariants (`HF_HOME` is injected ONLY into child processes).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::error::{PipelineError, Result};

/// Resolve creation flags for silent background child execution on Windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Resolves default open-weight Hugging Face cache directory (`D:\music\models` on Windows).
pub fn default_hf_home_dir() -> PathBuf {
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

/// Arguments forwarded to `workers/generate_song.py` for open-weight music rendering.
#[derive(Debug, Clone)]
pub struct GenerateSongArgs {
    pub model_id: String,
    pub genre_txt: PathBuf,
    pub lyrics_txt: PathBuf,
    pub output_dir: PathBuf,
    pub max_duration_secs: Option<f32>,
    pub bpm: u32,
    pub key_signature: String,
    pub lora_adapter_path: Option<PathBuf>,
    pub commercial_only: bool,
    pub device: String,
}

impl Default for GenerateSongArgs {
    fn default() -> Self {
        Self {
            model_id: "ACE-Step/acestep-v15-turbo".to_string(),
            genre_txt: PathBuf::from("genre.txt"),
            lyrics_txt: PathBuf::from("lyrics.txt"),
            output_dir: PathBuf::from("stems"),
            max_duration_secs: None,
            bpm: 100,
            key_signature: "A Major".to_string(),
            lora_adapter_path: None,
            commercial_only: false,
            device: "auto".to_string(),
        }
    }
}

/// Output files materialized by `workers/generate_song.py`.
#[derive(Debug, Clone)]
pub struct GenerateSongOutput {
    pub vocals_path: PathBuf,
    pub instrumentals_path: PathBuf,
    pub master_path: Option<PathBuf>,
    pub extra_stems: Vec<(String, PathBuf)>,
}

/// Isolated subprocess bridge managing Python 3.12 / `uv` environments and HF workers.
#[derive(Debug, Clone)]
pub struct UvBridge {
    pub uv_bin: String,
    pub hf_home: PathBuf,
}

impl Default for UvBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl UvBridge {
    pub fn new() -> Self {
        let uv_bin = Self::detect_uv_binary().unwrap_or_else(|| "uv".to_string());
        Self {
            uv_bin,
            hf_home: default_hf_home_dir(),
        }
    }

    pub fn with_uv_bin(mut self, uv_bin: impl Into<String>) -> Self {
        self.uv_bin = uv_bin.into();
        self
    }

    pub fn with_hf_home(mut self, hf_home: impl Into<PathBuf>) -> Self {
        self.hf_home = hf_home.into();
        self
    }

    /// Detects `uv` executable if already present on the machine.
    pub fn detect_uv_binary() -> Option<String> {
        let candidates = if cfg!(windows) {
            vec![
                "uv.exe".to_string(),
                "uv".to_string(),
                format!(
                    "{}\\.local\\bin\\uv.exe",
                    std::env::var("USERPROFILE").unwrap_or_default()
                ),
                format!(
                    "{}\\.cargo\\bin\\uv.exe",
                    std::env::var("USERPROFILE").unwrap_or_default()
                ),
            ]
        } else {
            vec![
                "uv".to_string(),
                format!("{}/.local/bin/uv", std::env::var("HOME").unwrap_or_default()),
                format!("{}/.cargo/bin/uv", std::env::var("HOME").unwrap_or_default()),
            ]
        };

        for cand in candidates {
            if cand.is_empty() {
                continue;
            }
            let mut cmd = Command::new(&cand);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            if cmd.arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
                return Some(cand);
            }
        }
        None
    }

    /// Locates host Python 3.12+ binary without touching system or user PATH.
    pub fn detect_host_python() -> Option<PathBuf> {
        if let Ok(explicit) = std::env::var("MUSIC_PYTHON_BIN") {
            let p = PathBuf::from(explicit.trim());
            if p.is_file() {
                return Some(p);
            }
        }
        #[cfg(windows)]
        {
            if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
                for py_rel in [
                    r"Programs\Python\Python312\python.exe",
                    r"Programs\Python\Python311\python.exe",
                    r"Programs\Python\Python313\python.exe",
                ] {
                    let cand = PathBuf::from(&local_app).join(py_rel);
                    if cand.is_file() {
                        return Some(cand);
                    }
                }
            }
        }
        for name in ["python3", "python"] {
            let mut cmd = Command::new(name);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            if cmd.arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
                return Some(PathBuf::from(name));
            }
        }
        None
    }

    fn apply_process_scoped_env(&self, cmd: &mut tokio::process::Command) {
        let _ = std::fs::create_dir_all(&self.hf_home);
        cmd.env("HF_HOME", &self.hf_home);
        cmd.env("HUGGINGFACE_HUB_CACHE", self.hf_home.join("hub"));
        cmd.env("PYTHONIOENCODING", "utf-8");
        cmd.env("TOKENIZERS_PARALLELISM", "false");
    }

    /// Creates or reuses the isolated `.venv` using `uv` (or Python 3.12 `-m venv` fallback).
    pub fn bootstrap_environment(&self, venv_dir: &Path) -> Result<PathBuf> {
        let py_bin = if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        if py_bin.is_file() {
            return Ok(py_bin);
        }

        if let Some(parent) = venv_dir.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        // Try `uv venv` first when installed, otherwise host Python `-m venv`
        let uv_ok = if Self::detect_uv_binary().is_some() {
            let mut cmd = Command::new(&self.uv_bin);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            cmd.arg("venv").arg(venv_dir);
            cmd.status().map(|s| s.success()).unwrap_or(false)
        } else {
            false
        };

        if !uv_ok {
            let host_py = Self::detect_host_python().ok_or_else(|| PipelineError::SubprocessError {
                stage: "uv_bootstrap".to_string(),
                message: "Neither `uv` nor Python 3.11+ executable was found on host".to_string(),
            })?;
            let mut cmd = Command::new(&host_py);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            let status = cmd.arg("-m").arg("venv").arg(venv_dir).status().map_err(|e| {
                PipelineError::SubprocessError {
                    stage: "uv_bootstrap".to_string(),
                    message: format!("Failed to create venv via {:?}: {}", host_py, e),
                }
            })?;
            if !status.success() {
                return Err(PipelineError::SubprocessError {
                    stage: "uv_bootstrap".to_string(),
                    message: format!("Python venv creation failed with status {:?}", status.code()),
                });
            }
        }

        if !py_bin.is_file() {
            return Err(PipelineError::SubprocessError {
                stage: "uv_bootstrap".to_string(),
                message: format!("Expected Python binary not found at {:?}", py_bin),
            });
        }

        Ok(py_bin)
    }

    /// Installs CUDA `cu124` PyTorch + open-weight synthesis stack (`diffusers`, `transformers`,
    /// `peft`, `demucs`, `huggingface_hub`, `soundfile`, `scipy`) into `venv_dir`.
    pub async fn install_cuda_open_weight_stack(&self, venv_dir: &Path) -> Result<()> {
        let py_bin = self.bootstrap_environment(venv_dir)?;
        let use_uv = Self::detect_uv_binary().is_some();

        let run_pkg_install = |args: &[&str]| -> std::result::Result<(), String> {
            let mut cmd = if use_uv {
                let mut c = Command::new(&self.uv_bin);
                c.arg("pip").arg("install").arg("--python").arg(&py_bin);
                c
            } else {
                let mut c = Command::new(&py_bin);
                c.arg("-m").arg("pip").arg("install");
                c
            };
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            cmd.args(args);
            let out = cmd.output().map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        };

        // 1. Install CUDA 12.4 PyTorch + torchaudio
        run_pkg_install(&[
            "torch",
            "torchaudio",
            "--index-url",
            "https://download.pytorch.org/whl/cu124",
        ])
        .map_err(|e| PipelineError::SubprocessError {
            stage: "cuda_torch_install".to_string(),
            message: e,
        })?;

        // 2. Install open-weight generation, LoRA, Demucs separation, and HF Hub dependencies
        run_pkg_install(&[
            "huggingface_hub",
            "transformers",
            "diffusers",
            "accelerate",
            "peft",
            "demucs",
            "soundfile",
            "scipy",
            "numpy",
            "edge-tts",
        ])
        .map_err(|e| PipelineError::SubprocessError {
            stage: "open_weight_deps_install".to_string(),
            message: e,
        })?;

        let _ = std::fs::write(venv_dir.join(".open_weight_cu124_ready"), "cu124\n");
        Ok(())
    }

    /// Downloads an open-weight Hugging Face repository snapshot directly into process-scoped `HF_HOME`.
    pub async fn pull_hf_checkpoint(
        &self,
        venv_dir: &Path,
        repo_id: &str,
        timeout_secs: u64,
    ) -> Result<PathBuf> {
        let trimmed = repo_id.trim();
        if trimmed.is_empty()
            || trimmed.contains("..")
            || trimmed.to_ascii_lowercase().contains("lyria")
        {
            return Err(PipelineError::CompilationError(format!(
                "Invalid or forbidden checkpoint repository '{}'",
                repo_id
            )));
        }

        let py_bin = self
            .bootstrap_environment(venv_dir)
            .ok()
            .or_else(Self::detect_host_python)
            .ok_or_else(|| PipelineError::SubprocessError {
                stage: "hf_pull".to_string(),
                message: "No Python runtime found for huggingface_hub snapshot download".to_string(),
            })?;

        let script = r#"
import os, sys
from huggingface_hub import snapshot_download
repo_id = sys.argv[1]
cache_dir = os.environ.get("HF_HOME")
path = snapshot_download(repo_id=repo_id, cache_dir=cache_dir)
print(path)
"#;

        let mut cmd = tokio::process::Command::new(&py_bin);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        self.apply_process_scoped_env(&mut cmd);
        cmd.arg("-c").arg(script).arg(trimmed);

        let fut = cmd.output();
        let output = tokio::time::timeout(Duration::from_secs(timeout_secs.max(30)), fut)
            .await
            .map_err(|_| PipelineError::SubprocessTimeout {
                stage: "hf_pull".to_string(),
                timeout_secs,
            })?
            .map_err(|e| PipelineError::SubprocessError {
                stage: "hf_pull".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(PipelineError::SubprocessError {
                stage: "hf_pull".to_string(),
                message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let out_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if out_path.is_empty() {
            self.hf_home.clone()
        } else {
            PathBuf::from(out_path)
        })
    }

    /// Executes `workers/generate_song.py` cleanly in isolated child process with VRAM cleanup.
    pub async fn run_generate_song(
        &self,
        python_bin: &Path,
        script_path: &Path,
        args: &GenerateSongArgs,
        timeout_secs: u64,
    ) -> Result<GenerateSongOutput> {
        let mut cmd = tokio::process::Command::new(python_bin);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        self.apply_process_scoped_env(&mut cmd);

        cmd.arg(script_path)
            .arg("--model")
            .arg(&args.model_id)
            .arg("--genre-txt")
            .arg(&args.genre_txt)
            .arg("--lyrics-txt")
            .arg(&args.lyrics_txt)
            .arg("--output-dir")
            .arg(&args.output_dir)
            .arg("--bpm")
            .arg(args.bpm.to_string())
            .arg("--key")
            .arg(&args.key_signature)
            .arg("--device")
            .arg(&args.device);

        if let Some(dur) = args.max_duration_secs {
            cmd.arg("--max-duration").arg(format!("{:.1}", dur));
        }
        if let Some(ref lora) = args.lora_adapter_path {
            cmd.arg("--lora").arg(lora);
        }
        if args.commercial_only {
            cmd.arg("--commercial-only");
        }

        let child = cmd.spawn().map_err(|e| PipelineError::SubprocessError {
            stage: "neural_synthesis".to_string(),
            message: format!("Failed to spawn {:?}: {}", python_bin, e),
        })?;

        let output = match tokio::time::timeout(
            Duration::from_secs(timeout_secs.max(30)),
            child.wait_with_output(),
        )
        .await
        {
            Ok(res) => res.map_err(|e| PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Worker I/O failure: {}", e),
            })?,
            Err(_) => {
                return Err(PipelineError::SubprocessTimeout {
                    stage: "neural_synthesis".to_string(),
                    timeout_secs,
                });
            }
        };

        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);

        match code {
            0 => {
                let vocals_path = args.output_dir.join("yue_vocals.wav");
                let instrumentals_path = args.output_dir.join("yue_instrumentals.wav");
                let master_cand = args.output_dir.join("mix_master.wav");
                if !vocals_path.exists() || !instrumentals_path.exists() {
                    return Err(PipelineError::MissingStem(
                        "Worker finished with code 0 but vocal/instrumental stems were not materialized on disk".to_string(),
                    ));
                }
                let mut extra_stems = Vec::new();
                for stem_key in ["drums", "bass", "guitar", "other"] {
                    let p = args.output_dir.join(format!("{}.wav", stem_key));
                    if p.is_file() {
                        extra_stems.push((stem_key.to_string(), p));
                    }
                }
                Ok(GenerateSongOutput {
                    vocals_path,
                    instrumentals_path,
                    master_path: master_cand.is_file().then_some(master_cand),
                    extra_stems,
                })
            }
            2 => Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Open-weight checkpoint not cached or offline (exit 2): {}", stderr.trim()),
            }),
            3 => Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("CUDA Out of Memory (exit 3): {}", stderr.trim()),
            }),
            _ => Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Neural synthesis exited with code {}: {}", code, stderr.trim()),
            }),
        }
    }

    /// Executes `workers/separate_and_inpaint.py` to split mixed tracks into `drums`, `bass`, `guitar`, `other`.
    pub async fn run_separate_and_inpaint(
        &self,
        python_bin: &Path,
        script_path: &Path,
        input_wav: &Path,
        output_dir: &Path,
        timeout_secs: u64,
    ) -> Result<Vec<(String, PathBuf)>> {
        let mut cmd = tokio::process::Command::new(python_bin);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        self.apply_process_scoped_env(&mut cmd);

        cmd.arg(script_path)
            .arg("--input")
            .arg(input_wav)
            .arg("--output-dir")
            .arg(output_dir);

        let output = tokio::time::timeout(Duration::from_secs(timeout_secs.max(30)), cmd.output())
            .await
            .map_err(|_| PipelineError::SubprocessTimeout {
                stage: "stem_separation".to_string(),
                timeout_secs,
            })?
            .map_err(|e| PipelineError::SubprocessError {
                stage: "stem_separation".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(PipelineError::SubprocessError {
                stage: "stem_separation".to_string(),
                message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let mut stems = Vec::new();
        for key in ["drums", "bass", "guitar", "other"] {
            let p = output_dir.join(format!("{}.wav", key));
            if p.is_file() {
                stems.push((key.to_string(), p));
            }
        }
        Ok(stems)
    }
}
