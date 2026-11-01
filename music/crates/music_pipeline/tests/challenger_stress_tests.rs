//! Empirical Adversarial Stress Suite for Milestone 3 (Execution Engine & Checkpointing).
//!
//! Evaluates:
//! 1. Infinite graph cycle detection & `max_iterations = 50` threshold guard.
//! 2. Missing entry point, invalid edge, & dynamic branch key fail-closed behavior.
//! 3. CheckpointManager directory traversal, path injection, and Windows reserved device rejection.
//! 4. High-concurrency multi-song state and stem isolation across threads.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use music_core::state::MusicProductionState;
use music_dsp::wav::AudioBuffer;
use music_pipeline::checkpointer::CheckpointManager;
use music_pipeline::error::PipelineError;
use music_pipeline::graph::{StateGraph, END, START};
use music_pipeline::node::GraphNode;

struct CountingNode {
    name: String,
    counter: Arc<AtomicUsize>,
}

#[async_trait]
impl GraphNode for CountingNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        state.history.push(format!("Executed:{}", self.name));
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =========================================================================
// SECTION 1: Infinite Graph Cycles & Loop Threshold Stress Tests
// =========================================================================

#[tokio::test]
async fn stress_test_single_node_self_cycle_hits_default_50_limit() {
    let mut graph = StateGraph::new();
    let count = Arc::new(AtomicUsize::new(0));

    let node = CountingNode {
        name: "node_spin".to_string(),
        counter: count.clone(),
    };

    graph.add_node("node_spin", node).expect("add node_spin");
    graph.set_entry_point("node_spin");
    graph.add_edge("node_spin", "node_spin"); // Self-cycle

    let compiled = graph.compile().expect("Graph compilation must succeed");
    assert_eq!(compiled.max_iterations(), 50, "Default max iterations must be 50");

    let state = MusicProductionState::new("song-loop-1", "Reggae", 76, "A Major");
    let result = compiled.execute(state).await;

    match result {
        Err(PipelineError::MaxIterationsExceeded { node, iterations }) => {
            assert_eq!(node, "node_spin");
            assert_eq!(iterations, 51, "Must fail on iteration 51 after 50 executions");
        }
        other => panic!("Expected MaxIterationsExceeded, got: {:?}", other),
    }

    assert_eq!(count.load(Ordering::SeqCst), 50, "Node must have executed exactly 50 times before halt");
}

#[tokio::test]
async fn stress_test_multi_node_cycle_hits_default_50_limit() {
    // 3-node cycle: A -> B -> C -> A
    let mut graph = StateGraph::new();
    let count_a = Arc::new(AtomicUsize::new(0));
    let count_b = Arc::new(AtomicUsize::new(0));
    let count_c = Arc::new(AtomicUsize::new(0));

    graph.add_node("A", CountingNode { name: "A".to_string(), counter: count_a.clone() }).expect("add A");
    graph.add_node("B", CountingNode { name: "B".to_string(), counter: count_b.clone() }).expect("add B");
    graph.add_node("C", CountingNode { name: "C".to_string(), counter: count_c.clone() }).expect("add C");

    graph.set_entry_point("A");
    graph.add_edge("A", "B");
    graph.add_edge("B", "C");
    graph.add_edge("C", "A");

    let compiled = graph.compile().expect("Graph compilation must succeed");
    let state = MusicProductionState::new("song-cycle-3", "Afrobeats", 108, "F# Minor");
    let result = compiled.execute(state).await;

    match result {
        Err(PipelineError::MaxIterationsExceeded { iterations, .. }) => {
            assert_eq!(iterations, 51);
        }
        other => panic!("Expected MaxIterationsExceeded, got: {:?}", other),
    }

    let total = count_a.load(Ordering::SeqCst) + count_b.load(Ordering::SeqCst) + count_c.load(Ordering::SeqCst);
    assert_eq!(total, 50, "Total executions across cycle must equal exactly 50");
    // 50 executions across 3 nodes: A=17, B=17, C=16
    assert_eq!(count_a.load(Ordering::SeqCst), 17);
    assert_eq!(count_b.load(Ordering::SeqCst), 17);
    assert_eq!(count_c.load(Ordering::SeqCst), 16);
}

#[tokio::test]
async fn stress_test_conditional_ping_pong_loop_hits_limit() {
    let mut graph = StateGraph::new();
    let count_p = Arc::new(AtomicUsize::new(0));
    let count_q = Arc::new(AtomicUsize::new(0));

    graph.add_node("Ping", CountingNode { name: "Ping".to_string(), counter: count_p.clone() }).expect("add Ping");
    graph.add_node("Pong", CountingNode { name: "Pong".to_string(), counter: count_q.clone() }).expect("add Pong");

    let mut ping_branches = HashMap::new();
    ping_branches.insert("go_pong", "Pong");
    graph.add_conditional_edges("Ping", |_| "go_pong".to_string(), ping_branches);

    let mut pong_branches = HashMap::new();
    pong_branches.insert("go_ping", "Ping");
    graph.add_conditional_edges("Pong", |_| "go_ping".to_string(), pong_branches);

    graph.set_entry_point("Ping");

    let compiled = graph.compile().expect("Compile must succeed");
    let state = MusicProductionState::new("song-pingpong", "Jazz", 120, "C Major");
    let result = compiled.execute(state).await;

    match result {
        Err(PipelineError::MaxIterationsExceeded { iterations, .. }) => {
            assert_eq!(iterations, 51);
        }
        other => panic!("Expected MaxIterationsExceeded, got: {:?}", other),
    }

    assert_eq!(count_p.load(Ordering::SeqCst) + count_q.load(Ordering::SeqCst), 50);
}

#[tokio::test]
async fn stress_test_configurable_max_iterations_boundaries() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("loop", CountingNode { name: "loop".to_string(), counter: counter.clone() }).expect("add loop");
    graph.set_entry_point("loop");
    graph.add_edge("loop", "loop");

    let mut compiled = graph.compile().expect("compile");

    // Test bound 1: max_iterations = 1
    compiled.set_max_iterations(1);
    let state = MusicProductionState::new("bound-1", "Pop", 120, "G Major");
    let res = compiled.execute(state).await;
    match res {
        Err(PipelineError::MaxIterationsExceeded { iterations, .. }) => {
            assert_eq!(iterations, 2);
        }
        other => panic!("Expected limit at 2, got: {:?}", other),
    }
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Test bound 2: max_iterations = 5
    counter.store(0, Ordering::SeqCst);
    compiled.set_max_iterations(5);
    let state2 = MusicProductionState::new("bound-5", "Pop", 120, "G Major");
    let res2 = compiled.execute(state2).await;
    match res2 {
        Err(PipelineError::MaxIterationsExceeded { iterations, .. }) => {
            assert_eq!(iterations, 6);
        }
        other => panic!("Expected limit at 6, got: {:?}", other),
    }
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

// =========================================================================
// SECTION 2: Missing Entry Point & Routing Fail-Closed Verification
// =========================================================================

#[test]
fn stress_test_missing_entry_point_rejects_immediately() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("orphan", CountingNode { name: "orphan".to_string(), counter }).expect("add orphan");

    match graph.compile() {
        Err(PipelineError::MissingEntryPoint) => (),
        Err(e) => panic!("Expected MissingEntryPoint, got: {:?}", e),
        Ok(_) => panic!("Expected MissingEntryPoint, got Ok"),
    }
}

#[test]
fn stress_test_entry_point_nonexistent_node_rejects() {
    let mut graph = StateGraph::new();
    graph.set_entry_point("phantom_node");

    match graph.compile() {
        Err(PipelineError::NodeNotFound(name)) => assert_eq!(name, "phantom_node"),
        Err(e) => panic!("Expected NodeNotFound, got: {:?}", e),
        Ok(_) => panic!("Expected NodeNotFound, got Ok"),
    }
}

#[test]
fn stress_test_add_edge_start_to_nonexistent_node_rejects() {
    let mut graph = StateGraph::new();
    graph.add_edge(START, "ghost_start");

    match graph.compile() {
        Err(PipelineError::NodeNotFound(name)) => assert_eq!(name, "ghost_start"),
        Err(e) => panic!("Expected NodeNotFound, got: {:?}", e),
        Ok(_) => panic!("Expected NodeNotFound, got Ok"),
    }
}

#[test]
fn stress_test_edge_to_unregistered_destination_rejects() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("source", CountingNode { name: "source".to_string(), counter }).expect("add source");
    graph.set_entry_point("source");
    graph.add_edge("source", "unregistered_dest");

    match graph.compile() {
        Err(PipelineError::NodeNotFound(name)) => assert_eq!(name, "unregistered_dest"),
        Err(e) => panic!("Expected NodeNotFound, got: {:?}", e),
        Ok(_) => panic!("Expected NodeNotFound, got Ok"),
    }
}

#[test]
fn stress_test_conditional_edge_to_unregistered_branch_destination_rejects() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("eval", CountingNode { name: "eval".to_string(), counter }).expect("add eval");
    graph.set_entry_point("eval");

    let mut branches = HashMap::new();
    branches.insert("ok", "missing_node");
    graph.add_conditional_edges("eval", |_| "ok".to_string(), branches);

    match graph.compile() {
        Err(PipelineError::NodeNotFound(name)) => assert_eq!(name, "missing_node"),
        Err(e) => panic!("Expected NodeNotFound, got: {:?}", e),
        Ok(_) => panic!("Expected NodeNotFound, got Ok"),
    }
}

#[tokio::test]
async fn stress_test_runtime_unregistered_branch_key_fails_closed() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("start_node", CountingNode { name: "start_node".to_string(), counter: counter.clone() }).expect("add start_node");
    graph.add_node("valid_target", CountingNode { name: "valid_target".to_string(), counter }).expect("add valid_target");

    let mut branches = HashMap::new();
    branches.insert("branch_a", "valid_target");
    // Condition returns "branch_rogue", which is not in branches
    graph.add_conditional_edges("start_node", |_| "branch_rogue".to_string(), branches);
    graph.add_edge("valid_target", END);
    graph.set_entry_point("start_node");

    let compiled = graph.compile().expect("Compile should succeed as all target nodes exist");
    let state = MusicProductionState::new("rogue-branch-test", "Metal", 140, "D Minor");

    let res = compiled.execute(state).await;
    match res {
        Err(PipelineError::InvalidBranchKey { node, branch, available }) => {
            assert_eq!(node, "start_node");
            assert_eq!(branch, "branch_rogue");
            assert_eq!(available, vec!["branch_a"]);
        }
        other => panic!("Expected InvalidBranchKey, got: {:?}", other),
    }
}

// =========================================================================
// SECTION 3: CheckpointManager Path Traversal & Windows Injection Attacks
// =========================================================================

#[test]
fn stress_test_checkpoint_path_injection_and_traversal_attacks() {
    let malicious_components = [
        // Standard Unix traversal
        "../../etc/passwd",
        "../../../etc/shadow",
        "../../../../",
        "/etc/passwd",
        // Windows traversal & drive roots
        "..\\..\\windows\\system32",
        "..\\escape",
        "C:\\Windows\\System32",
        "c:/windows/system32",
        "D:\\music",
        "\\unc\\share",
        "\\\\?\\C:\\secret",
        // Alternate Data Streams & Colons
        "song_id:stream",
        "song:stem:$DATA",
        ":hidden",
        // Null bytes & encoded variants
        "song\0id",
        "stem\0.wav",
        "%00exploit",
        "foo%00bar",
        // Percent-encoded traversal
        "%2e%2e%2f",
        "%2e%2e%5c",
        "..%2f",
        "%2e%2e",
        "foo%2ebar",
        "foo%2fbar",
        "foo%5cbar",
        // Slashes & Backslashes inside names
        "foo/bar",
        "foo\\bar",
        "/",
        "\\",
        "",
        // Windows Reserved Device Names
        "COM1",
        "COM2",
        "COM3",
        "COM4",
        "COM5",
        "COM6",
        "COM7",
        "COM8",
        "COM9",
        "LPT1",
        "LPT2",
        "LPT3",
        "LPT4",
        "LPT5",
        "LPT6",
        "LPT7",
        "LPT8",
        "LPT9",
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "com1",
        "con",
        "prn",
        "aux",
        "nul",
    ];

    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let mut passed_sanitization = Vec::new();

    for &malicious in &malicious_components {
        let sanitize_res = CheckpointManager::sanitize_path_component(malicious);
        let version_dir_res = checkpointer.version_dir(malicious, 1);
        let stems_dir_res = checkpointer.stems_dir(malicious, 1);

        if sanitize_res.is_ok() || version_dir_res.is_ok() || stems_dir_res.is_ok() {
            passed_sanitization.push(malicious);
        }
    }

    assert!(
        passed_sanitization.is_empty(),
        "The following malicious components bypassed sanitize_path_component: {:?}",
        passed_sanitization
    );
}

// =========================================================================
// SECTION 4: High-Concurrency Multi-Song State & Stem Isolation
// =========================================================================

#[tokio::test]
async fn stress_test_concurrent_multi_song_checkpoint_isolation() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = Arc::new(
        CheckpointManager::with_sqlite(temp_dir.path(), "concurrent_test.db")
            .expect("with_sqlite"),
    );

    let num_songs = 12;
    let versions_per_song = 3;
    let mut handles = Vec::new();

    for song_idx in 0..num_songs {
        let cp = checkpointer.clone();
        let song_id = format!("song-concurrent-{:03}", song_idx);

        let handle = tokio::spawn(async move {
            let mut state = MusicProductionState::new(&song_id, "Reggae", 76 + (song_idx as u32), "C Major");

            for v in 1..=versions_per_song {
                state.version = v;
                state.style_prompt = format!("Prompt for {} version {}", song_id, v);
                state.history.push(format!("Stage-v{} finished", v));

                // Save checkpoint
                let cp_path = cp.save_checkpoint(&state, &format!("node_{}", v)).await
                    .expect("save_checkpoint should succeed concurrently");
                assert!(cp_path.exists());

                // Save audio stem buffer
                let buffer = AudioBuffer::new_stereo(0.1);
                let stem_name = format!("stem_v{}", v);
                let stem_path = cp.save_stem(&song_id, v, &stem_name, &buffer).await
                    .expect("save_stem should succeed concurrently");
                assert!(stem_path.exists());

                // Verify immediate reload of state
                let reloaded = cp.load_checkpoint(&song_id, v).await
                    .expect("load_checkpoint should succeed concurrently");
                assert_eq!(reloaded.song_id, song_id);
                assert_eq!(reloaded.version, v);
                assert_eq!(reloaded.style_prompt, state.style_prompt);

                // Verify immediate reload of stem
                let reloaded_stem = cp.load_stem(&song_id, v, &stem_name).await
                    .expect("load_stem should succeed concurrently");
                assert_eq!(reloaded_stem.sample_rate, 48_000);
                assert_eq!(reloaded_stem.frame_count(), buffer.frame_count());
            }

            // Verify version list for this song
            let versions = cp.list_versions(&song_id)
                .expect("list_versions should succeed concurrently");
            assert_eq!(versions, vec![1, 2, 3]);
        });

        handles.push(handle);
    }

    // Wait for all concurrent song tasks to complete
    for handle in handles {
        handle.await.expect("Task must not panic");
    }

    // Cross-verify all song versions remain isolated
    for song_idx in 0..num_songs {
        let song_id = format!("song-concurrent-{:03}", song_idx);
        let versions = checkpointer.list_versions(&song_id).expect("list_versions");
        assert_eq!(versions, vec![1, 2, 3], "Song {} must have exactly versions 1,2,3", song_id);

        for v in 1..=versions_per_song {
            let loaded = checkpointer.load_checkpoint(&song_id, v).await.expect("load");
            assert_eq!(loaded.song_id, song_id);
            assert_eq!(loaded.bpm, 76 + (song_idx as u32));
        }
    }
}
