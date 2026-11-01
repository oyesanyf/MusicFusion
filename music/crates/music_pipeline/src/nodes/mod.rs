//! Submodules for the 6 sequential fusion nodes of the StateGraph pipeline.

pub mod node1_producer_panel;
pub mod node2_groove_yue;
pub mod node3_vocal_timbre;
pub mod node4_stem_inpainting;
pub mod node5_dsp_judge_gate;
pub mod node6_analog_mastering;

// Re-exports with canonical node names
pub use node1_producer_panel::OllamaProducerPanelNode;
pub use node2_groove_yue::GrooveAndYueSynthesisNode;
pub use node3_vocal_timbre::VocalTimbreFusionNode;
pub use node4_stem_inpainting::VirtuosoStemInpaintingNode;
pub use node5_dsp_judge_gate::{
    DspAndJudgeGateNode, judge_gate_condition, mix_buffers,
    ROUTE_EXHAUSTED, ROUTE_MASTERING, ROUTE_RETRY_GROOVE, ROUTE_RETRY_STEM, ROUTE_SUCCESS,
};
pub use node6_analog_mastering::AnalogMasteringNode;

// Ergonomic / requirement aliases
pub type ProducerPanelNode = OllamaProducerPanelNode;
pub type GrooveAndYueNode = GrooveAndYueSynthesisNode;
pub type JamaicanTenorTimbreNode = VocalTimbreFusionNode;
pub type StemInpaintingNode = VirtuosoStemInpaintingNode;
pub type JudgeGateNode = DspAndJudgeGateNode;
pub type MasteringNode = AnalogMasteringNode;
