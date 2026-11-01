use quorum_coordinator::{
    AgenticStateGraphRunner, AgenticTelemetryEvent, CoordinatorConfig, FederatedQueryCoordinator,
    MockMeshDispatcher, MockRemotePeer, PeerAclManager, ScatterGatherEngine, StorageSearcher,
    SynthesisModelFamily,
};
use quorum_types::{CandidateChunkDto, DocumentChunk, ScoredChunk, VolatileRemoteChunk};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use zeroize::Zeroize;

#[derive(Default, Clone)]
struct MockStorage {
    chunks: Vec<DocumentChunk>,
}

#[async_trait::async_trait]
impl StorageSearcher for MockStorage {
    async fn search_hybrid(
        &self,
        query_text: &str,
        _query_vec: &[f32],
        max_tier: u8,
        top_k: usize,
    ) -> Vec<ScoredChunk> {
        let mut matches: Vec<ScoredChunk> = self
            .chunks
            .iter()
            .filter(|c| c.access_tier <= max_tier && c.text_content.contains(query_text))
            .map(|c| ScoredChunk::new(c.clone(), 0.9, "local", false))
            .collect();
        matches.truncate(top_k);
        matches
    }
}

/// Adversarial Challenge 1: Slow/hanging peer added FIRST must NOT block fast peers added SECOND.
///
/// In a concurrent scatter-gather architecture, if peer 0 is hanging/delayed (>400ms)
/// and peer 1 finishes in 30ms, the coordinator MUST gather peer 1's chunks before the 400ms deadline.
#[tokio::test]
async fn challenge_slow_peer_first_must_not_drop_fast_peer() {
    let mut storage = MockStorage::default();
    storage.chunks.push(
        DocumentChunk::new(
            "c_local",
            "doc1",
            "loc.rs",
            1,
            5,
            0,
            vec![1.0; 768],
            "Fast query text",
            None,
        )
        .unwrap(),
    );

    let mut coord = FederatedQueryCoordinator::new(storage);

    // Slow peer (800ms) added FIRST
    coord.add_peer(MockRemotePeer {
        peer_id: "peer_slow".to_string(),
        latency: Duration::from_millis(800),
        max_access_tier: 0,
        candidate_chunks: vec![CandidateChunkDto {
            chunk_id: "c_slow".to_string(),
            file_path: "slow.rs".to_string(),
            line_start: 1,
            line_end: 5,
            content: "Slow hanging chunk".to_string(),
            score: 0.99,
            access_tier: 0,
        }],
    });

    // Fast peer (30ms) added SECOND
    coord.add_peer(MockRemotePeer {
        peer_id: "peer_fast".to_string(),
        latency: Duration::from_millis(30),
        max_access_tier: 0,
        candidate_chunks: vec![CandidateChunkDto {
            chunk_id: "c_fast".to_string(),
            file_path: "fast.rs".to_string(),
            line_start: 1,
            line_end: 5,
            content: "Fast query text remote".to_string(),
            score: 0.88,
            access_tier: 0,
        }],
    });

    let (results, _volatile, duration_ms) =
        coord.execute_query("Fast query text", &vec![1.0; 768], 0, 10).await;

    assert!(
        duration_ms <= 480,
        "Duration {}ms violated 400ms hard deadline",
        duration_ms
    );
    assert!(!results.iter().any(|r| r.chunk.chunk_id == "c_slow"), "Slow chunk should not be returned");
    assert!(
        results.iter().any(|r| r.chunk.chunk_id == "c_fast"),
        "CRITICAL BUG: Fast peer response was dropped because slow peer was earlier in the tasks list!"
    );
    assert!(results.iter().any(|r| r.chunk.chunk_id == "c_local"));
}

/// Adversarial Challenge 2: ScatterGatherEngine with MockMeshDispatcher where slow peer is first.
#[tokio::test]
async fn challenge_scatter_gather_engine_slow_peer_first() {
    let mut storage = MockStorage::default();
    storage.chunks.push(
        DocumentChunk::new(
            "c_local",
            "doc1",
            "loc.rs",
            1,
            5,
            0,
            vec![1.0; 768],
            "Fast query text",
            None,
        )
        .unwrap(),
    );

    let dispatcher = MockMeshDispatcher::new();
    // Add slow peer first
    dispatcher
        .add_peer(MockRemotePeer {
            peer_id: "peer_slow".to_string(),
            latency: Duration::from_millis(800),
            max_access_tier: 0,
            candidate_chunks: vec![CandidateChunkDto {
                chunk_id: "c_slow".to_string(),
                file_path: "slow.rs".to_string(),
                line_start: 1,
                line_end: 5,
                content: "Slow hanging chunk".to_string(),
                score: 0.99,
                access_tier: 0,
            }],
        })
        .await;

    // Add fast peer second
    dispatcher
        .add_peer(MockRemotePeer {
            peer_id: "peer_fast".to_string(),
            latency: Duration::from_millis(30),
            max_access_tier: 0,
            candidate_chunks: vec![CandidateChunkDto {
                chunk_id: "c_fast".to_string(),
                file_path: "fast.rs".to_string(),
                line_start: 1,
                line_end: 5,
                content: "Fast query text remote".to_string(),
                score: 0.88,
                access_tier: 0,
            }],
        })
        .await;

    let engine = ScatterGatherEngine::new(
        Arc::new(storage),
        Arc::new(dispatcher),
        Arc::new(RwLock::new(PeerAclManager::default())),
        CoordinatorConfig {
            timeout_ms: 400,
            ..Default::default()
        },
        [0x11u8; 32],
        "test_sender".to_string(),
    );

    let (results, _volatile, duration_ms) =
        engine.execute_query("Fast query text", &vec![1.0; 768], 0, 10).await;

    assert!(duration_ms <= 480, "Duration {}ms violated 400ms deadline", duration_ms);
    assert!(!results.iter().any(|r| r.chunk.chunk_id == "c_slow"));
    assert!(
        results.iter().any(|r| r.chunk.chunk_id == "c_fast"),
        "CRITICAL BUG: Fast peer response was dropped because slow peer was earlier in the tasks list!"
    );
}

/// Adversarial Challenge 3: Agentic state graph loop bounds when candidate score is 0.0
#[tokio::test]
async fn challenge_agentic_loop_terminates_strictly_at_max_hops() {
    #[derive(Default)]
    struct ZeroScoreStorage;

    #[async_trait::async_trait]
    impl StorageSearcher for ZeroScoreStorage {
        async fn search_hybrid(
            &self,
            _query_text: &str,
            _query_vec: &[f32],
            _max_tier: u8,
            _top_k: usize,
        ) -> Vec<ScoredChunk> {
            // Return candidate with score 0.0, far below threshold 0.60
            vec![ScoredChunk::new(
                DocumentChunk::new(
                    "zero_chunk",
                    "doc_zero",
                    "irrelevant.txt",
                    1,
                    1,
                    0,
                    vec![0.0; 768],
                    "Completely irrelevant text",
                    None,
                )
                .unwrap(),
                0.0,
                "local",
                false,
            )]
        }
    }

    let runner = AgenticStateGraphRunner::new(3, 0.60);
    let engine = ScatterGatherEngine::new(
        Arc::new(ZeroScoreStorage),
        Arc::new(quorum_coordinator::LocalOnlyDispatcher),
        Arc::new(RwLock::new(PeerAclManager::default())),
        CoordinatorConfig::default(),
        [0u8; 32],
        "node_zero".to_string(),
    );

    let (tx, mut rx) = mpsc::channel(64);
    let monitor = tokio::spawn(async move {
        let mut hops = Vec::new();
        while let Some(ev) = rx.recv().await {
            if let AgenticTelemetryEvent::StateTransition { to_state, hop, .. } = ev {
                hops.push((to_state, hop));
            }
        }
        hops
    });

    let state = runner
        .run(
            "Query that will never meet threshold",
            SynthesisModelFamily::AutoFusion,
            &engine,
            &vec![0.0; 768],
            0,
            5,
            Some(tx),
        )
        .await
        .expect("Must terminate cleanly without infinite loop");

    assert_eq!(state.hop_count, 3, "State graph must terminate at exactly max_hops (3)");
    assert!(state.synthesized_response.is_some());

    let transitions = monitor.await.unwrap();
    for (_st, hop) in &transitions {
        assert!(*hop <= 3, "Hop {} exceeded max_hops 3", hop);
    }
}

/// Adversarial Challenge 4: VolatileRemoteChunk zeroizes memory on drop.
#[test]
fn challenge_volatile_remote_chunk_zeroizes_buffer() {
    let secret = "VERY_CONFIDENTIAL_REMOTE_CONTENT_CANARY_123456789".to_string();
    let ptr = secret.as_ptr();
    let len = secret.len();

    let mut chunk = VolatileRemoteChunk {
        chunk_id: "c_canary".to_string(),
        peer_id: "peer_canary".to_string(),
        file_path: "classified.txt".to_string(),
        line_start: 1,
        line_end: 10,
        score: 0.95,
        access_tier: 3,
        content: secret,
    };

    assert_eq!(chunk.content(), "VERY_CONFIDENTIAL_REMOTE_CONTENT_CANARY_123456789");

    chunk.zeroize();
    assert!(chunk.content.is_empty());

    let slice_after = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(
        slice_after.iter().all(|&b| b == 0),
        "Memory at pointer was not zeroed!"
    );
}


/// Adversarial Challenge 5: VolatileRemoteChunk implicit drop zeroizes memory buffer.
#[test]
fn challenge_volatile_remote_chunk_implicit_drop_zeroizes() {
    let secret = "VERY_CONFIDENTIAL_IMPLICIT_DROP_CANARY_987654321".to_string();
    let ptr = secret.as_ptr();
    let len = secret.len();

    {
        let chunk = VolatileRemoteChunk {
            chunk_id: "c_drop".to_string(),
            peer_id: "peer_drop".to_string(),
            file_path: "classified.txt".to_string(),
            line_start: 1,
            line_end: 10,
            score: 0.95,
            access_tier: 3,
            content: secret,
        };
        assert_eq!(chunk.content(), "VERY_CONFIDENTIAL_IMPLICIT_DROP_CANARY_987654321");
    } // chunk dropped here; ZeroizeOnDrop must overwrite memory at ptr with zeroes

    let slice_after = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(
        slice_after.iter().all(|&b| b == 0),
        "Memory at pointer was not zeroed upon implicit drop!"
    );
}

/// Adversarial Challenge 6: Agentic state graph with empty retrieval terminates immediately.
#[tokio::test]
async fn challenge_agentic_loop_empty_retrieval_terminates() {
    #[derive(Default)]
    struct EmptyStorage;

    #[async_trait::async_trait]
    impl StorageSearcher for EmptyStorage {
        async fn search_hybrid(
            &self,
            _query_text: &str,
            _query_vec: &[f32],
            _max_tier: u8,
            _top_k: usize,
        ) -> Vec<ScoredChunk> {
            Vec::new()
        }
    }

    let runner = AgenticStateGraphRunner::new(3, 0.60);
    let engine = ScatterGatherEngine::new(
        Arc::new(EmptyStorage),
        Arc::new(quorum_coordinator::LocalOnlyDispatcher),
        Arc::new(RwLock::new(PeerAclManager::default())),
        CoordinatorConfig::default(),
        [0u8; 32],
        "node_empty".to_string(),
    );

    let state = runner
        .run(
            "Empty query",
            SynthesisModelFamily::AutoFusion,
            &engine,
            &vec![0.0; 768],
            0,
            5,
            None,
        )
        .await
        .expect("Must terminate cleanly on empty retrieval");

    // When retrieval is empty, GradeRelevance marks unsatisfactory, triggering rewrite hops
    // bounded strictly at max_hops (3), then synthesizes cleanly.
    assert_eq!(
        state.hop_count, 3,
        "Empty retrieval must trigger query rewrite hops up to max_hops (3)"
    );
    assert!(state.synthesized_response.is_some());
}
