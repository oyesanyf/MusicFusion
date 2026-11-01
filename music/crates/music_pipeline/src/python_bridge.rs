//! Python and uv virtual environment bridge for neural music generation models.
//!
//! Provides `UvBridge` for bootstrapping `d:\harfile\music\.venv` via `uv venv` and `uv pip install`,
//! and invoking `workers/generate_song.py` with Windows `CREATE_NO_WINDOW` and clean exit code handling:
//! - 0: Success (output stems generated)
//! - 2: Missing / offline weights or dependencies
//! - 3: CUDA Out of Memory (OOM)

use std::path::{Path, PathBuf};
use crate::error::{PipelineError, Result};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Arguments passed to `workers/generate_song.py`.
#[derive(Debug, Clone)]
pub struct GenerateSongArgs {
    pub model: String,
    pub prompt: String,
    pub lyrics_file: PathBuf,
    pub bpm: u32,
    pub key: String,
    pub duration_secs: f32,
    pub output_dir: PathBuf,
    pub device: String,
}

/// Paths to dual stems produced by neural generation.
#[derive(Debug, Clone)]
pub struct GenerateSongOutput {
    pub vocals_path: PathBuf,
    pub instrumentals_path: PathBuf,
}

/// uv environment bridge and runner.
#[derive(Debug, Clone)]
pub struct UvBridge {
    uv_bin_path: Option<PathBuf>,
}

impl Default for UvBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl UvBridge {
    /// Detect or instantiate a new `UvBridge`.
    pub fn new() -> Self {
        Self {
            uv_bin_path: Self::detect_uv_binary(),
        }
    }

    /// Construct a `UvBridge` with an explicit path to `uv`.
    pub fn with_uv_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.uv_bin_path = Some(path.into());
        self
    }

    /// Returns the resolved path to `uv` if found.
    pub fn uv_bin(&self) -> Option<&Path> {
        self.uv_bin_path.as_deref()
    }

    /// Returns true if `uv` is available on the host system.
    pub fn is_available(&self) -> bool {
        self.uv_bin_path.is_some()
    }

    /// Detects `uv` executable across standard environment variables, cargo paths, and PATH.
    pub fn detect_uv_binary() -> Option<PathBuf> {
        // 1. Environment variable UV_BIN
        if let Ok(uv_bin) = std::env::var("UV_BIN") {
            let p = PathBuf::from(uv_bin);
            if p.is_file() {
                return Some(p);
            }
        }

        // 2. User profile .cargo/bin/uv.exe on Windows
        #[cfg(windows)]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let p = PathBuf::from(userprofile).join(".cargo").join("bin").join("uv.exe");
                if p.is_file() {
                    return Some(p);
                }
            }
        }

        // 3. User home .cargo/bin/uv on Unix
        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                let p = PathBuf::from(home).join(".cargo").join("bin").join("uv");
                if p.is_file() {
                    return Some(p);
                }
            }
        }

        // 4. Test "uv" in PATH via --version
        let mut cmd = std::process::Command::new("uv");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.arg("--version");
        if let Ok(status) = cmd.status() {
            if status.success() {
                return Some(PathBuf::from("uv"));
            }
        }

        None
    }

    /// Bootstraps Python virtual environment at `venv_dir` via `uv venv` and installs
    /// torch/transformers dependencies via `uv pip install`.
    pub fn bootstrap_environment(&self, venv_dir: &Path) -> Result<PathBuf> {
        let uv_bin = self.uv_bin_path.as_deref().ok_or_else(|| {
            PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: "uv executable not found in PATH or standard cargo locations".to_string(),
            }
        })?;

        let python_path = if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        // 1. Create venv if python executable does not exist
        if !python_path.exists() {
            if let Some(parent) = venv_dir.parent() {
                std::fs::create_dir_all(parent).map_err(PipelineError::IoError)?;
            }

            let mut cmd = std::process::Command::new(uv_bin);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            cmd.arg("venv").arg(venv_dir);

            let output = cmd.output().map_err(|e| PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Failed to spawn 'uv venv': {}", e),
            })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(PipelineError::SubprocessError {
                    stage: "neural_synthesis".to_string(),
                    message: format!("'uv venv' failed: {}", stderr),
                });
            }
        }

        // 2. Install dependencies if marker file not present
        let marker_path = venv_dir.join(".deps_installed");
        if !marker_path.exists() {
            let mut cmd = std::process::Command::new(uv_bin);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            cmd.arg("pip")
                .arg("install")
                .arg("--python")
                .arg(&python_path)
                .args(["torch", "torchaudio", "transformers", "diffusers", "accelerate", "soundfile"]);

            let output = cmd.output().map_err(|e| PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Failed to spawn 'uv pip install': {}", e),
            })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(PipelineError::SubprocessError {
                    stage: "neural_synthesis".to_string(),
                    message: format!("'uv pip install' failed: {}", stderr),
                });
            }

            let _ = std::fs::write(&marker_path, "torch torchaudio transformers diffusers accelerate soundfile\n");
        }

        Ok(python_path)
    }

    /// Runs `workers/generate_song.py` with `CREATE_NO_WINDOW` and clean exit code handling.
    pub async fn run_generate_song(
        &self,
        python_bin: &Path,
        script_path: &Path,
        args: &GenerateSongArgs,
        timeout_secs: u64,
    ) -> Result<GenerateSongOutput> {
        if !script_path.exists() {
            return Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Generator script not found: {:?}", script_path),
            });
        }

        let mut cmd = tokio::process::Command::new(python_bin);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.arg(script_path)
            .arg("--model").arg(&args.model)
            .arg("--prompt").arg(&args.prompt)
            .arg("--lyrics_file").arg(&args.lyrics_file)
            .arg("--bpm").arg(args.bpm.to_string())
            .arg("--key").arg(&args.key)
            .arg("--duration").arg(args.duration_secs.to_string())
            .arg("--output_dir").arg(&args.output_dir)
            .arg("--device").arg(&args.device);

        let child = cmd.spawn().map_err(|e| PipelineError::SubprocessError {
            stage: "neural_synthesis".to_string(),
            message: format!("Failed to spawn Python generator: {}", e),
        })?;

        let timeout_duration = std::time::Duration::from_secs(timeout_secs);
        let output = match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
            Ok(res) => res.map_err(|e| PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Subprocess I/O error: {}", e),
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
                if !vocals_path.exists() || !instrumentals_path.exists() {
                    return Err(PipelineError::MissingStem(
                        "Worker finished with code 0 but stems were not found on disk".to_string(),
                    ));
                }
                Ok(GenerateSongOutput {
                    vocals_path,
                    instrumentals_path,
                })
            }
            2 => Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Neural model weights missing or offline (exit code 2): {}", stderr.trim()),
            }),
            3 => Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("CUDA Out of Memory (exit code 3): {}", stderr.trim()),
            }),
            _ => Err(PipelineError::SubprocessError {
                stage: "neural_synthesis".to_string(),
                message: format!("Neural generation failed with exit code {}: {}", code, stderr.trim()),
            }),
        }
    }
}
