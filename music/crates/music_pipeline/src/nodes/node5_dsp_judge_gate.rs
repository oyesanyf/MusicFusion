//! Node 5: DSP & Judge Gate Node (`dsp_and_judge_gate_node`).
//!
//! Evaluates audio telemetry metrics across stems, queries the Ollama Judge model
//! (or deterministic heuristic fallback), and decides conditional retry routing.

use async_trait::async_trait;
use log::{info, warn};

use music_core::state::{
    MusicProductionState, STEM_BASS, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR,
    STEM_OTHER, STEM_RASPY_TENOR_VOCALS, STEM_YUE_VOCALS,
};
use music_dsp::telemetry_calc::calculate_telemetry;
use music_dsp::wav::AudioBuffer;
use music_fusion::ollama_client::{ChatMessage, OllamaClient};

use crate::error::PipelineError;
use crate::graph::GraphNode;

pub const ROUTE_MASTERING: &str = "mastering";
pub const ROUTE_RETRY_STEM: &str = "retry_stem";
pub const ROUTE_RETRY_GROOVE: &str = "retry_groove";
pub const ROUTE_SUCCESS: &str = "success";
pub const ROUTE_EXHAUSTED: &str = "exhausted";

/// Utility to mix one AudioBuffer into another with scaling.
pub fn mix_buffers(target: &mut AudioBuffer, source: &AudioBuffer, gain: f32) {
    if target.samples.is_empty() {
        *target = source.clone();
        for s in &mut target.samples {
            *s *= gain;
        }
        return;
    }

    let src_frames = source.frame_count();
    let tgt_frames = target.frame_count();
    if src_frames > tgt_frames {
        let diff_frames = src_frames - tgt_frames;
        target.samples.extend(vec![0.0; diff_frames * target.channels as usize]);
    }

    let frames_to_mix = source.frame_count().min(target.frame_count());
    let tgt_ch = target.channels as usize;
    let src_ch = source.channels as usize;

    for f in 0..frames_to_mix {
        for c in 0..tgt_ch {
            let src_sample = if src_ch == 1 {
                source.samples[f]
            } else {
                source.samples[f * src_ch + (c % src_ch)]
            };
            target.samples[f * tgt_ch + c] += src_sample * gain;
        }
    }
}

pub struct DspAndJudgeGateNode {
    pub ollama_host: String,
    pub judge_model: String,
}

impl Default for DspAndJudgeGateNode {
    fn default() -> Self {
        Self::new("http://127.0.0.1:11434", "qwen2.5:7b")
    }
}

impl DspAndJudgeGateNode {
    pub fn new(ollama_host: impl Into<String>, judge_model: impl Into<String>) -> Self {
        Self {
            ollama_host: ollama_host.into(),
            judge_model: judge_model.into(),
        }
    }

    /// Assembles composite mix buffer from active stems for evaluation.
    pub fn assemble_composite_mix(&self, state: &MusicProductionState) -> AudioBuffer {
        let mut mix = AudioBuffer::new_stereo(0.1);

        // Vocal stem
        let vocal_path = state.get_stem(STEM_RASPY_TENOR_VOCALS)
            .or_else(|| state.get_stem(STEM_YUE_VOCALS));
        if let Some(p) = vocal_path {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut mix, &buf, 1.0); // 0 dB baseline
            }
        }

        // Inpainted guitar
        if let Some(p) = state.get_stem(STEM_INPAINTED_GUITAR) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut mix, &buf, 0.85); // -1.5 dB
            }
        }

        // Inpainted drums
        if let Some(p) = state.get_stem(STEM_INPAINTED_DRUMS) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut mix, &buf, 0.90); // -1.0 dB
            }
        }

        // Bass
        if let Some(p) = state.get_stem(STEM_BASS) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut mix, &buf, 0.80); // -2.0 dB
            }
        }

        // Other
        if let Some(p) = state.get_stem(STEM_OTHER) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut mix, &buf, 0.70); // -3.0 dB
            }
        }

        mix
    }

    /// Primary execution method for Node 5.
    pub async fn execute_node(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        info!("Executing DspAndJudgeGateNode for song '{}'", state.song_id);

        // 1. Calculate genuine DSP telemetry
        let mix_buf = self.assemble_composite_mix(state);
        let telemetry = calculate_telemetry(&mix_buf, state.bpm);
        info!("Calculated Telemetry: {}", telemetry.summary());

        // 2. Ollama Judge or deterministic rule evaluation
        let mut score: f32 = 8.8; // Baseline broadcast pass score

        if !state.is_pure_rust() {
            let client = OllamaClient::new(&self.ollama_host);
            // Check live Ollama connection with fast timeout
            if let Ok(tags) = client.list_installed_tags().await {
                let model = if tags.contains(&self.judge_model) {
                    self.judge_model.clone()
                } else {
                    tags.first().cloned().unwrap_or_else(|| self.judge_model.clone())
                };

                let eval_prompt = format!(
                    "Evaluate audio quality for '{}' ({} BPM, Key of {}):\n- Peak: {:.2} dBFS\n- RMS: {:.2} dBFS\n- Dynamic Range: {:.2} dB\n- Tempo Drift: {:.4}\n- Clipping: {}\nEmit JSON with quality_score (0.0 to 10.0) and critique.",
                    state.style_prompt, state.bpm, state.key_signature,
                    telemetry.peak_dbfs, telemetry.rms_dbfs, telemetry.dynamic_range_db,
                    telemetry.tempo_drift_score, telemetry.clipping
                );

                let res = tokio::time::timeout(
                    std::time::Duration::from_millis(500),
                    client.chat(
                        &model,
                        vec![
                            ChatMessage::system("You are an expert audio mastering judge. Respond only with JSON."),
                            ChatMessage::user(eval_prompt),
                        ],
                        None,
                    ),
                ).await;

                if let Ok(Ok(chat_resp)) = res {
                    let content = chat_resp.message.content;
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(s) = val.get("quality_score").and_then(|v| v.as_f64()) {
                            score = s as f32;
                        }
                    }
                }
            }
        }

        // Apply telemetry penalties to score
        if telemetry.clipping {
            score -= 4.0;
        }
        if telemetry.peak_dbfs > 0.0 {
            score -= 2.5;
        }
        if telemetry.tempo_drift_score > 0.30 {
            score -= 2.0;
        }
        if telemetry.dynamic_range_db < 6.0 {
            score -= 1.5;
        }
        score = score.clamp(0.0, 10.0);

        // Quality gate pass threshold: score >= 7.5 and zero clipping and safe headroom
        let quality_passed = !telemetry.clipping
            && telemetry.peak_dbfs <= 0.0
            && telemetry.tempo_drift_score <= 0.30
            && score >= 7.5;

        state.set_telemetry(telemetry);
        state.set_judge_result(score, quality_passed);

        // If quality failed and retries not exhausted, increment retry count
        if !quality_passed && state.retry_count < 2 {
            state.increment_retry();
        }

        info!("Judge evaluation: score={:.2}, passed={}, retry_count={}", score, quality_passed, state.retry_count);
        state.record_node_transition("dsp_and_judge_gate_node");
        Ok(())
    }
}

#[async_trait]
impl GraphNode for DspAndJudgeGateNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.execute_node(state).await
    }

    fn name(&self) -> &str {
        "dsp_and_judge_gate_node"
    }

    fn description(&self) -> &str {
        "DSP & Judge Gate: Waveform telemetry, AI evaluation, and conditional retry routing"
    }
}

/// Conditional edge function routing after Node 5 inspection.
pub fn judge_gate_condition(state: &MusicProductionState) -> String {
    if state.quality_passed {
        info!("Quality gate PASSED. Routing forward to analog_mastering_node.");
        ROUTE_MASTERING.to_string()
    } else if state.retry_count < 2 {
        if state.telemetry.tempo_drift_score > 0.50 {
            warn!(
                "Severe vocal/tempo drift detected ({:.4} > 0.50). Routing back to groove_and_yue_synthesis_node (retry {}/2).",
                state.telemetry.tempo_drift_score, state.retry_count
            );
            ROUTE_RETRY_GROOVE.to_string()
        } else {
            warn!(
                "Quality critique failure (score {:.2} < 7.5 or telemetry alert). Routing back to virtuoso_stem_inpainting_node (retry {}/2).",
                state.judge_score, state.retry_count
            );
            ROUTE_RETRY_STEM.to_string()
        }
    } else {
        warn!(
            "Maximum retry count reached ({}). Advancing to analog_mastering_node to ensure completion.",
            state.retry_count
        );
        ROUTE_MASTERING.to_string()
    }
}
