//! Proposed test additions and updates for crates/music_pipeline/tests/graph_tests.rs
//! Milestone 3 Remediation Explorer 2: Graph Fail-Closed Error Handling & Topology Validation

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use music_core::state::MusicProductionState;
use music_pipeline::checkpointer::CheckpointManager;
use music_pipeline::error::PipelineError;
use music_pipeline::graph::{StateGraph, END, START};
use music_pipeline::node::{GraphNode, GraphObserver};

// -----------------------------------------------------------------------------
// Test 1: Checkpoint security errors fail closed and return Err
// -----------------------------------------------------------------------------

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

    // Malicious song_id designed to trigger SecurityError in CheckpointManager::sanitize_path_component
    let bad_state = MusicProductionState::new(
        "../../etc/passwd",
        "Roots Reggae",
        76,
        "A Major",
    );

    let res = compiled.execute_with_checkpointer(bad_state, &checkpointer).await;

    // Must fail closed with SecurityError immediately
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

    // Fail-Closed Invariant Verification:
    // Node 1 executed and triggered save_checkpoint.
    // CheckpointManager rejected the malicious song_id with SecurityError.
    // Node 2 must NEVER execute!
    assert_eq!(counter_1.load(Ordering::SeqCst), 1, "Node 1 should run before checkpoint");
    assert_eq!(counter_2.load(Ordering::SeqCst), 0, "Node 2 must NEVER run after security violation");
}

// -----------------------------------------------------------------------------
// Test 2: Checkpoint security error notifies observer on_node_error
// -----------------------------------------------------------------------------

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

// -----------------------------------------------------------------------------
// Test 3: Duplicate node registration returns Err(PipelineError::DuplicateNode)
// -----------------------------------------------------------------------------

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
        other => panic!("Expected Err(PipelineError::DuplicateNode), got: {:?}", other),
    }
}

// -----------------------------------------------------------------------------
// Test 4: Reserved pseudo-node name (__start__, __end__) rejection
// -----------------------------------------------------------------------------

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

// -----------------------------------------------------------------------------
// Test 5: Dangling entry node caught at compile time
// -----------------------------------------------------------------------------

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
    // Notice: NO add_edge("dangling_entry", ...) was called!

    let res = graph.compile();
    match res {
        Err(PipelineError::DeadEndNode(name)) => {
            assert_eq!(name, "dangling_entry", "Should identify dangling entry node");
        }
        other => panic!("Expected Err(PipelineError::DeadEndNode), got: {:?}", other),
    }
}

// -----------------------------------------------------------------------------
// Test 6: Downstream non-terminal node without outgoing edge caught at compile time
// -----------------------------------------------------------------------------

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
    // "dead_end_stage" has no outgoing edge to END or another node!

    let res = graph.compile();
    match res {
        Err(PipelineError::DeadEndNode(name)) => {
            assert_eq!(name, "dead_end_stage", "Should catch dead_end_stage at compile time");
        }
        other => panic!("Expected Err(PipelineError::DeadEndNode), got: {:?}", other),
    }
}

// -----------------------------------------------------------------------------
// Test 7: Conditional edge with empty branches caught at compile time
// -----------------------------------------------------------------------------

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
        other => panic!("Expected Err(PipelineError::DeadEndNode), got: {:?}", other),
    }
}
