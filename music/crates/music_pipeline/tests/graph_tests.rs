//! Unit and integration tests for StateGraph, CheckpointManager, and path traversal defense.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use music_core::state::MusicProductionState;
use music_dsp::wav::AudioBuffer;
use music_pipeline::checkpointer::CheckpointManager;
use music_pipeline::error::PipelineError;
use music_pipeline::graph::{StateGraph, END, START};
use music_pipeline::node::{FnNode, GraphNode, GraphObserver};

struct IncrementNode {
    name: String,
    counter: Arc<AtomicUsize>,
}

#[async_trait]
impl GraphNode for IncrementNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        state.history.push(format!("Node {} executed", self.name));
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[tokio::test]
async fn test_stategraph_registration_and_compilation() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let n1 = IncrementNode {
        name: "node1".to_string(),
        counter: counter.clone(),
    };
    let n2 = IncrementNode {
        name: "node2".to_string(),
        counter: counter.clone(),
    };

    graph.add_node("node1", n1).expect("add node1");
    graph.add_node("node2", n2).expect("add node2");
    graph.add_edge(START, "node1");
    graph.add_edge("node1", "node2");
    graph.add_edge("node2", END);

    let compiled = graph.compile().expect("Graph should compile cleanly");
    assert!(compiled.has_node("node1"));
    assert!(compiled.has_node("node2"));
    assert_eq!(compiled.entry_point(), "node1");

    let initial = MusicProductionState::new("test-compile-1", "Roots Reggae", 76, "A Major");
    let final_state = compiled.execute(initial).await.expect("Execution should succeed");

    assert_eq!(counter.load(Ordering::SeqCst), 2);
    assert_eq!(final_state.history.len(), 5); // Created + entered node1 + node1 exec + entered node2 + node2 exec
}

#[tokio::test]
async fn test_stategraph_missing_entry_point() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("n1", IncrementNode { name: "n1".to_string(), counter }).expect("add n1");

    let res = graph.compile();
    assert!(matches!(res, Err(PipelineError::MissingEntryPoint)));
}

#[tokio::test]
async fn test_stategraph_invalid_node_reference() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("n1", IncrementNode { name: "n1".to_string(), counter }).expect("add n1");
    graph.set_entry_point("n1");
    graph.add_edge("n1", "non_existent_node");

    let res = graph.compile();
    assert!(matches!(res, Err(PipelineError::NodeNotFound(_))));
}

#[tokio::test]
async fn test_stategraph_conditional_edge_routing() {
    let mut graph = StateGraph::new();
    let counter_branch_a = Arc::new(AtomicUsize::new(0));
    let counter_branch_b = Arc::new(AtomicUsize::new(0));

    let evaluator = FnNode::new("evaluator", |state: &mut MusicProductionState| {
        // Sets quality pass flag based on bpm
        state.quality_passed = state.bpm >= 100;
        Ok(())
    });

    let branch_a = IncrementNode {
        name: "branch_fast".to_string(),
        counter: counter_branch_a.clone(),
    };
    let branch_b = IncrementNode {
        name: "branch_slow".to_string(),
        counter: counter_branch_b.clone(),
    };

    graph.add_node("evaluator", evaluator).expect("add evaluator");
    graph.add_node("branch_fast", branch_a).expect("add branch_fast");
    graph.add_node("branch_slow", branch_b).expect("add branch_slow");

    graph.set_entry_point("evaluator");

    let mut branches = HashMap::new();
    branches.insert("fast", "branch_fast");
    branches.insert("slow", "branch_slow");

    graph.add_conditional_edges(
        "evaluator",
        |state: &MusicProductionState| {
            if state.quality_passed {
                "fast".to_string()
            } else {
                "slow".to_string()
            }
        },
        branches,
    );

    graph.add_edge("branch_fast", END);
    graph.add_edge("branch_slow", END);

    let compiled = graph.compile().expect("Compile must succeed");

    // Case 1: BPM 120 -> quality_passed == true -> branch_fast
    let state_fast = MusicProductionState::new("s_fast", "Pop", 120, "C Major");
    let res1 = compiled.execute(state_fast).await.expect("Execution 1 should succeed");
    assert!(res1.quality_passed);
    assert_eq!(counter_branch_a.load(Ordering::SeqCst), 1);
    assert_eq!(counter_branch_b.load(Ordering::SeqCst), 0);

    // Case 2: BPM 76 -> quality_passed == false -> branch_slow
    let state_slow = MusicProductionState::new("s_slow", "Reggae", 76, "A Major");
    let res2 = compiled.execute(state_slow).await.expect("Execution 2 should succeed");
    assert!(!res2.quality_passed);
    assert_eq!(counter_branch_a.load(Ordering::SeqCst), 1);
    assert_eq!(counter_branch_b.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_stategraph_max_iterations_loop_guard() {
    let mut graph = StateGraph::new();
    let loop_counter = Arc::new(AtomicUsize::new(0));

    let looper = IncrementNode {
        name: "looper".to_string(),
        counter: loop_counter.clone(),
    };

    graph.add_node("looper", looper).expect("add looper");
    graph.set_entry_point("looper");
    // Infinite cycle back to looper
    graph.add_edge("looper", "looper");

    let mut compiled = graph.compile().expect("Compile must succeed");
    compiled.set_max_iterations(10);

    let initial = MusicProductionState::new("s_loop", "Rock", 120, "E Minor");
    let res = compiled.execute(initial).await;

    match res {
        Err(PipelineError::MaxIterationsExceeded { iterations, node }) => {
            assert_eq!(node, "looper");
            assert!(iterations > 10);
        }
        other => panic!("Expected MaxIterationsExceeded, got: {:?}", other),
    }

    assert_eq!(loop_counter.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn test_checkpoint_manager_file_persistence() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let mut state = MusicProductionState::new("song-cp-1", "Roots Reggae", 76, "A Major");
    state.style_prompt = "One love compassion anthem".to_string();

    let path = checkpointer.save_checkpoint(&state, "test_node").await.expect("save_checkpoint");
    assert!(path.exists());

    let loaded = checkpointer.load_checkpoint("song-cp-1", 1).await.expect("load_checkpoint");
    assert_eq!(loaded.song_id, state.song_id);
    assert_eq!(loaded.version, state.version);
    assert_eq!(loaded.bpm, state.bpm);
    assert_eq!(loaded.style_prompt, state.style_prompt);
}

#[tokio::test]
async fn test_checkpoint_manager_sqlite_version_logging() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::with_sqlite(temp_dir.path(), "test_storage.db")
        .expect("with_sqlite");

    let mut state = MusicProductionState::new("song-sql-1", "Afrobeats", 108, "F# Minor");
    checkpointer.save_checkpoint(&state, "node1").await.expect("save v1");

    state.advance_version();
    checkpointer.save_checkpoint(&state, "node2").await.expect("save v2");

    let versions = checkpointer.list_versions("song-sql-1").expect("list_versions");
    assert_eq!(versions, vec![1, 2]);

    let loaded_v2 = checkpointer.load_checkpoint("song-sql-1", 2).await.expect("load v2");
    assert_eq!(loaded_v2.version, 2);
}

#[tokio::test]
async fn test_checkpoint_manager_path_traversal_defense() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let bad_ids = [
        // Standard path traversal
        "../escape",
        "..\\escape",
        "../../etc/passwd",
        "c:/windows/system32",
        "C:\\Windows\\System32",
        "foo/bar",
        "foo\\bar",
        "foo:bar",
        "foo\0bar",
        "foo%2ebar",
        "foo%2fbar",
        "foo%5cbar",
        "foo%00bar",
        "",
        // Windows filesystem quirks (trailing dot / space)
        "foo.",
        "foo ",
        "song.",
        "song ",
        // Windows reserved device names (uppercase)
        "CON",
        "PRN",
        "AUX",
        "NUL",
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
        // Windows reserved device names (lowercase)
        "con",
        "prn",
        "aux",
        "nul",
        "com1",
        "com2",
        "com3",
        "com4",
        "com5",
        "com6",
        "com7",
        "com8",
        "com9",
        "lpt1",
        "lpt2",
        "lpt3",
        "lpt4",
        "lpt5",
        "lpt6",
        "lpt7",
        "lpt8",
        "lpt9",
        // Windows reserved device names with extensions
        "CON.txt",
        "prn.wav",
        "aux.json",
        "nul.dat",
        "com1.wav",
        "COM2.state",
        "LPT1.log",
    ];

    for bad in &bad_ids {
        assert!(
            CheckpointManager::sanitize_path_component(bad).is_err(),
            "Component '{}' should be rejected by path traversal defense",
            bad
        );

        let res = checkpointer.version_dir(bad, 1);
        assert!(res.is_err(), "version_dir for '{}' should be rejected", bad);
    }

    let good_ids = ["valid-song-id", "song_12345", "test-track-v1", "uuid-a1b2-c3d4"];
    for good in &good_ids {
        assert!(
            CheckpointManager::sanitize_path_component(good).is_ok(),
            "Component '{}' should be accepted",
            good
        );
    }
}

#[tokio::test]
async fn test_checkpoint_manager_stem_persistence() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let buffer = AudioBuffer::new_stereo(0.5);
    let stem_path = checkpointer
        .save_stem("song-stem-1", 1, "test_stem", &buffer)
        .await
        .expect("save_stem");
    assert!(stem_path.exists());

    let loaded_buffer = checkpointer
        .load_stem("song-stem-1", 1, "test_stem")
        .await
        .expect("load_stem");
    assert_eq!(loaded_buffer.sample_rate, 48_000);
    assert_eq!(loaded_buffer.channels, 2);
    assert_eq!(loaded_buffer.frame_count(), buffer.frame_count());
}

struct TrackingObserver {
    events: Arc<Mutex<Vec<String>>>,
}

use std::sync::Mutex;

#[async_trait]
impl GraphObserver for TrackingObserver {
    async fn on_node_start(&self, node: &str, _state: &MusicProductionState) {
        self.events.lock().unwrap().push(format!("start:{}", node));
    }

    async fn on_node_complete(&self, node: &str, _state: &MusicProductionState, _duration_ms: u64) {
        self.events.lock().unwrap().push(format!("complete:{}", node));
    }

    async fn on_node_error(&self, node: &str, _error: &PipelineError) {
        self.events.lock().unwrap().push(format!("error:{}", node));
    }
}

#[tokio::test]
async fn test_observer_lifecycle_events() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    graph.add_node("n1", IncrementNode { name: "n1".to_string(), counter }).expect("add n1");
    graph.set_entry_point("n1");
    graph.add_edge("n1", END);

    let compiled = graph.compile().expect("Compile must succeed");
    let events = Arc::new(Mutex::new(Vec::new()));
    let observer: Arc<dyn GraphObserver> = Arc::new(TrackingObserver {
        events: events.clone(),
    });

    let state = MusicProductionState::new("obs-test", "Jazz", 110, "Bb Major");
    let _ = compiled.execute_with_observer(state, None, Some(&observer)).await.expect("execute");

    let logged = events.lock().unwrap().clone();
    assert_eq!(logged, vec!["start:n1", "complete:n1"]);
}

#[tokio::test]
async fn test_checkpoint_security_error_fails_closed() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let mut graph = StateGraph::new();
    let counter_1 = Arc::new(AtomicUsize::new(0));
    let counter_2 = Arc::new(AtomicUsize::new(0));

    let n1 = IncrementNode {
        name: "node1".to_string(),
        counter: counter_1.clone(),
    };
    let n2 = IncrementNode {
        name: "node2".to_string(),
        counter: counter_2.clone(),
    };

    graph.add_node("node1", n1).expect("add node1");
    graph.add_node("node2", n2).expect("add node2");
    graph.set_entry_point("node1");
    graph.add_edge("node1", "node2");
    graph.add_edge("node2", END);

    let compiled = graph.compile().expect("Graph should compile cleanly");

    let bad_state = MusicProductionState::new(
        "../../etc/passwd",
        "Roots Reggae",
        76,
        "A Major",
    );

    let res = compiled.execute_with_checkpointer(bad_state, &checkpointer).await;

    match res {
        Err(PipelineError::SecurityError(msg)) => {
            assert!(
                msg.contains("Path traversal") || msg.contains("Invalid path component"),
                "Security error message should identify path traversal rejection: {}",
                msg
            );
        }
        other => panic!("Expected Err(PipelineError::SecurityError), got: {:?}", other),
    }

    assert_eq!(counter_1.load(Ordering::SeqCst), 1, "Node 1 should run before checkpoint");
    assert_eq!(counter_2.load(Ordering::SeqCst), 0, "Node 2 must NEVER run after security violation");
}

#[tokio::test]
async fn test_checkpoint_security_error_notifies_observer() {
    let temp_dir = tempfile::tempdir().expect("tempdir creation");
    let checkpointer = CheckpointManager::new(temp_dir.path());

    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let n1 = IncrementNode {
        name: "node1".to_string(),
        counter,
    };

    graph.add_node("node1", n1).expect("add node1");
    graph.set_entry_point("node1");
    graph.add_edge("node1", END);

    let compiled = graph.compile().expect("compile");

    let events = Arc::new(Mutex::new(Vec::new()));
    let observer: Arc<dyn GraphObserver> = Arc::new(TrackingObserver {
        events: events.clone(),
    });

    let bad_state = MusicProductionState::new("COM1", "Roots Reggae", 76, "A Major");
    let res = compiled
        .execute_with_observer(bad_state, Some(&checkpointer), Some(&observer))
        .await;

    assert!(matches!(res, Err(PipelineError::SecurityError(_))));
    let logged = events.lock().unwrap().clone();
    assert!(logged.contains(&"start:node1".to_string()));
    assert!(logged.contains(&"complete:node1".to_string()));
    assert!(logged.contains(&"error:node1".to_string()), "Observer must receive error:node1 event on security error");
}

#[tokio::test]
async fn test_stategraph_duplicate_node_rejection() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let n1 = IncrementNode {
        name: "duplicate_stage".to_string(),
        counter: counter.clone(),
    };
    let n2 = IncrementNode {
        name: "duplicate_stage".to_string(),
        counter: counter.clone(),
    };

    let res1 = graph.add_node("duplicate_stage", n1);
    assert!(res1.is_ok(), "First add_node call should succeed");

    let res2 = graph.add_node("duplicate_stage", n2);
    match res2 {
        Err(PipelineError::DuplicateNode(name)) => {
            assert_eq!(name, "duplicate_stage", "Error should report duplicate node name");
        }
        Err(e) => panic!("Expected Err(PipelineError::DuplicateNode), got: {:?}", e),
        Ok(_) => panic!("Expected Err(PipelineError::DuplicateNode), got Ok"),
    }
}

#[tokio::test]
async fn test_stategraph_reserved_pseudo_node_rejection() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let n_start = IncrementNode {
        name: START.to_string(),
        counter: counter.clone(),
    };
    let n_end = IncrementNode {
        name: END.to_string(),
        counter: counter.clone(),
    };

    let res_start = graph.add_node(START, n_start);
    assert!(res_start.is_err(), "Adding START node must be rejected");

    let res_end = graph.add_node(END, n_end);
    assert!(res_end.is_err(), "Adding END node must be rejected");
}

#[tokio::test]
async fn test_stategraph_dangling_entry_node_compile_time_rejection() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let n1 = IncrementNode {
        name: "dangling_entry".to_string(),
        counter: counter.clone(),
    };

    graph.add_node("dangling_entry", n1).expect("add node");
    graph.set_entry_point("dangling_entry");

    let res = graph.compile();
    match res {
        Err(PipelineError::DeadEndNode(name)) => {
            assert_eq!(name, "dangling_entry", "Should identify dangling entry node");
        }
        Err(e) => panic!("Expected Err(PipelineError::DeadEndNode), got: {:?}", e),
        Ok(_) => panic!("Expected Err(PipelineError::DeadEndNode), got Ok"),
    }
}

#[tokio::test]
async fn test_stategraph_downstream_dead_end_node_compile_time_rejection() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let n1 = IncrementNode {
        name: "node1".to_string(),
        counter: counter.clone(),
    };
    let n2 = IncrementNode {
        name: "dead_end_stage".to_string(),
        counter: counter.clone(),
    };

    graph.add_node("node1", n1).expect("add node1");
    graph.add_node("dead_end_stage", n2).expect("add node2");
    graph.set_entry_point("node1");
    graph.add_edge("node1", "dead_end_stage");

    let res = graph.compile();
    match res {
        Err(PipelineError::DeadEndNode(name)) => {
            assert_eq!(name, "dead_end_stage", "Should catch dead_end_stage at compile time");
        }
        Err(e) => panic!("Expected Err(PipelineError::DeadEndNode), got: {:?}", e),
        Ok(_) => panic!("Expected Err(PipelineError::DeadEndNode), got Ok"),
    }
}

#[tokio::test]
async fn test_stategraph_conditional_edge_empty_branches_rejection() {
    let mut graph = StateGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let n1 = IncrementNode {
        name: "cond_node".to_string(),
        counter,
    };

    graph.add_node("cond_node", n1).expect("add node");
    graph.set_entry_point("cond_node");
    let empty_branches: HashMap<String, String> = HashMap::new();
    graph.add_conditional_edges("cond_node", |_| "any".to_string(), empty_branches);

    let res = graph.compile();
    match res {
        Err(PipelineError::DeadEndNode(name)) => {
            assert_eq!(name, "cond_node", "Should catch node with empty conditional branches");
        }
        Err(e) => panic!("Expected Err(PipelineError::DeadEndNode), got: {:?}", e),
        Ok(_) => panic!("Expected Err(PipelineError::DeadEndNode), got Ok"),
    }
}
