pub mod acl;
pub mod agentic;
pub mod config;
pub mod coordinator;
pub mod crypto;
pub mod error;
pub mod fusion;
pub mod hardware;
pub mod langextract;
pub mod mesh;
pub mod model_router;
pub mod offline_engine;
pub mod scatter_gather;
pub mod telemetry;
pub mod types;
pub mod volatile;

pub use acl::{AccessTierPolicy, PeerAclManager};
pub use agentic::AgenticStateGraphRunner;
pub use config::CoordinatorConfig;
pub use coordinator::QueryCoordinator;
pub use crypto::{
    canonical_query_bytes, oracle_sign_query, oracle_verify_query, sign_query, sign_request,
    verify_query, verify_request,
};
pub use error::CoordinatorError;
pub use fusion::RrfCombiner;
pub use hardware::HardwareTelemetry;
pub use mesh::{
    LocalOnlyDispatcher, MeshQueryDispatcher, MockMeshDispatcher, MockRemotePeer,
    SwarmMeshDispatcher,
};
pub use model_router::AutoFusionRouter;
pub use offline_engine::DeterministicOfflineSynthesisEngine;
pub use scatter_gather::{FederatedQueryCoordinator, ScatterGatherEngine, StorageSearcher};
pub use telemetry::AgenticTelemetryEvent;
pub use types::{
    AgenticGraphState, AgenticStateName, QueryIntentType, RelevanceGrade, SynthesisModelFamily,
};
pub use volatile::{scrub_volatile, wrap_candidates, VolatileRemoteChunk};
pub use langextract::{
    extract_query_metadata_filters, filter_grounded, ground_extraction, ConsensusExtractionResult,
    ExampleData, Extraction, LlmFusionArmReport, LlmFusionConsensus,
};
