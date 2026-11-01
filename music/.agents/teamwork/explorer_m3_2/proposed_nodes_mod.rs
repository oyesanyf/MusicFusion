//! crates/music_pipeline/src/nodes/mod.rs
//!
//! Submodules for the 6 sequential fusion nodes of the StateGraph pipeline.

pub mod node1_producer_panel;
pub mod node2_groove_yue;
pub mod node3_vocal_timbre;

// Re-exports with canonical node names and aliases
pub use node1_producer_panel::OllamaProducerPanelNode;
pub use node2_groove_yue::GrooveAndYueSynthesisNode;
pub use node3_vocal_timbre::VocalTimbreFusionNode;

// Backward-compatible / requirement aliases
pub type ProducerPanelNode = OllamaProducerPanelNode;
pub type GrooveAndYueNode = GrooveAndYueSynthesisNode;
pub type JamaicanTenorTimbreNode = VocalTimbreFusionNode;
