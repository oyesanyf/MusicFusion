# Handoff Report — Milestone 4 Explorer M4-3
# Agentic State Graph, Multi-Model Architecture & Hardware-Aware GLM Selection

- **Author**: `explorer_m4_3`
- **Milestone**: M4 (rust-libp2p Swarm, Coordinator, Fusion & GLM Selection)
- **Target Crate**: `crates/quorum-coordinator`
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m4_3\`
- **Date**: 2026-10-31

---

## 1. Observation

### 1.1 Model Routing & Hardware Telemetry in `d:\harfile\ModelFusion`
Inspection of `d:\harfile\ModelFusion\crates\model_selection` revealed production-tested implementations for hardware memory detection, model sizing, and dynamic routing:

1. **Memory Detection (`d:\harfile\ModelFusion\crates\model_selection\src\memory.rs`)**:
   - Lines 72–80: `SystemMemory` struct contains:
     ```rust
     pub struct SystemMemory {
         pub total_ram_gb: f64,
         pub free_ram_gb: f64,
         pub cpu_cores: usize,
         pub gpu_name: Option<String>,
         pub gpu_vram_total_gb: f64,
         pub gpu_vram_free_gb: f64,
     }
     ```
   - Lines 138–155: RAM is detected via `sysinfo::System::new_all()`:
     ```rust
     let total_ram_gb = sys.total_memory() as f64 / 1_073_741_824.0;
     let free_ram_gb = sys.available_memory() as f64 / 1_073_741_824.0;
     let cpu_cores = sys.physical_core_count().unwrap_or_else(|| sys.cpus().len()).max(1);
     ```
     On Windows, `sysinfo` queries `GlobalMemoryStatusEx` (`state.ullAvailPhys`).
   - Lines 662–684: GPU VRAM is detected using `nvidia-smi` executed with `CREATE_NO_WINDOW (0x08000000)`:
     ```rust
     let output = create_hidden_command("nvidia-smi")
         .args(["--query-gpu=name,memory.total,memory.free", "--format=csv,noheader,nounits"])
         .output();
     ```
     If `nvidia-smi` is not present, it gracefully returns `(None, 0.0, 0.0)`.

2. **Ollama Dynamic Memory Sizing Rule (`d:\harfile\ModelFusion\crates\model_selection\src\structured_router.rs`)**:
   - Lines 324–412: Available runtime memory sizing matrix strictly checks runtime **free RAM and free VRAM**, never total physical memory:
     - Tier 1: Free RAM >= 48 GB OR Free VRAM >= 24 GB -> `qwen2.5:32b`
     - Tier 2: Free RAM >= 24 GB OR Free VRAM >= 14 GB -> `qwen2.5:14b`
     - Tier 3: Free RAM >= 12 GB OR Free VRAM >= 4.5 GB -> `qwen2.5:7b`
     - Tier 4: Free RAM >= 6 GB OR Free VRAM >= 2 GB -> `qwen2.5:3b`
     - Tier 5: Free RAM < 6 GB -> `qwen2.5:1.5b`
   - Invariant: User rule `computer_use_and_ollama_resilience.md` strictly prohibits appending `-coder` suffixes that trigger silent 404 aborts on standard Ollama tags.

3. **GLM Model Calibration (`d:\harfile\ModelFusion\crates\model_selection\src\memory.rs`)**:
   - Lines 322–359: `assess_glm_for_mode` sets a strict memory floor for GLM: `free_ram_gb >= 8.0 || gpu_vram_free_gb >= 6.0`.
   - Lines 449–475: GLM-4 family mappings:
     - Mid-to-high tiers (free VRAM >= 6GB or free RAM >= 8GB) activate `glm4:9b` (Q4_0 / Q4_K_M ~5.5–8 GB) or `glm-4.7-flash` (MoE 30B/3.6B active).
     - Massive context length: `glm4:9b` supports 131,072 context tokens; `glm-4.7-flash` supports 202,752 context tokens.

4. **Live System Resources & Ollama Probe Observed on Current Host**:
   - Windows Host Memory: `TotalVisibleMemorySize = 234.5 GB`, `FreePhysicalMemory = 188.3 GB`.
   - GPU: `Quadro RTX 4000`, `8192 MiB total VRAM`, `6771 MiB free VRAM` (~6.77 GB).
   - Live Ollama daemon query (`http://127.0.0.1:11434/api/tags`) returned:
     - `glm-4.7-flash:latest` (29.9B, 202K context)
     - `glm4:9b` (9.4B, 131K context)
     - `qwen2.5:32b`, `qwen2.5:7b`, `qwen2.5:1.5b`
     - `gemma2:9b`, `gemma2:2b`
     - `embeddinggemma:latest` (307.58M, 768-dim)

### 1.2 Storage Abstraction & Arrow Encapsulation
Inspecting `crates/quorum-storage/src/lib.rs` (lines 1–27) and `crates/quorum-storage/src/engine.rs` (lines 19–77):
- Arrow (`arrow-array`, `arrow-schema`) and LanceDB types are strictly encapsulated within `pub(crate) mod arrow_interop` and `pub(crate) mod vector_store`.
- The storage public interface exports ONLY:
  - `StorageEngine`
  - `SearchMode::{Keyword, Vector, Hybrid}`
  - `StorageSearchQuery`
  - `CompactionStats`
  - `StorageError`
- All results are returned as pure domain types (`Vec<ScoredChunk>`, `DocumentChunk`) from `quorum-types`.
- **Finding**: `crates/quorum-coordinator` can interact with `StorageEngine` and `quorum-types` with ZERO Arrow/Lance type leakage.

---

## 2. Logic Chain

### 2.1 LangGraph-Style Agentic State Machine Architecture

LangGraph represents multi-hop reasoning as a directed state graph where nodes are functions and edges are conditional state transitions. In Rust, we achieve deterministic execution, memory safety, and zero overhead by modeling this as an explicit state machine:

```
                      [Start User Prompt]
                               │
                               ▼
                      [ClassifyIntent Node]
                               │
            ┌──────────────────┴──────────────────┐
            │ (General QA / No Retrieval)         │ (Retrieval Required)
            ▼                                     ▼
[Direct Synthesize]                     [RetrieveMesh Node] ◄──────────────┐
            │                                     │                        │
            │                                     ▼                        │
            │                           [GradeRelevance Node]              │
            │                                     │                        │
            │                ┌────────────────────┴────────────────────┐   │
            │                │ (Score < Threshold && hop < max_hops)   │   │
            │                ▼                                         │   │
            │        [RewriteQuery Node] (hop += 1) ───────────────────┘   │
            │                │                                             │
            │                │ (hop >= max_hops: Circuit Breaker)          │
            │                └────────────────────┬────────────────────────┘
            │                                     │ (Score >= Threshold OR Max Hops)
            │                                     ▼
            └───────────────────────► [SynthesizeWithCitations Node]
                                                  │
                                                  ▼
                                       [Execution Finished / SSE Complete]
```

#### State Definitions
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticStateName {
    ClassifyIntent,
    RetrieveMesh,
    GradeRelevance,
    RewriteQuery,
    SynthesizeWithCitations,
    Complete,
    Failed,
}
```

#### State Graph Execution Context
```rust
#[derive(Debug, Clone)]
pub struct AgenticGraphState {
    pub session_id: Uuid,
    pub original_prompt: String,
    pub current_query: String,
    pub query_history: Vec<String>,
    pub hop_count: usize,
    pub max_hops: usize, // default: 3
    pub intent: Option<QueryIntentType>,
    pub search_mode: SearchMode,
    pub model_family: SynthesisModelFamily,
    pub selected_model_tag: String,
    pub routing_rationale: String,
    pub retrieved_chunks: Vec<ScoredChunk>,
    pub volatile_chunks: Vec<VolatileRemoteChunk>,
    pub relevance_grade: Option<RelevanceGrade>,
    pub relevance_score: f32,
    pub start_time: Instant,
}
```

### 2.2 Bounded Loop Convergence Guard (Mathematical Proof of Termination)

**Problem**: Naive query rewriting in iterative RAG graphs can create infinite loops (e.g. oscillating between two query formulations).
**Solution**:
1. State variable `hop_count: usize` initialized to `0`.
2. Upper bound constant `max_hops: usize = 3` (configurable 1..=5).
3. In `GradeRelevanceNode`:
   - If `relevance_score >= threshold` (e.g. 0.65): transition to `SynthesizeWithCitations`.
   - If `relevance_score < threshold`:
     - If `hop_count < max_hops`: transition to `RewriteQuery`.
     - If `hop_count >= max_hops`: **Circuit Breaker trips**. Transition immediately to `SynthesizeWithCitations` with status `GradeStatus::MaxHopsExhausted`.
4. In `RewriteQueryNode`:
   - Increments `hop_count += 1`.
   - Checks cycle detection: if rewritten query is identical to any previous query in `query_history`, appends context disambiguation terms or trips the circuit breaker immediately.
   - Transitions back to `RetrieveMesh`.

**Termination Proof**:
At each rewrite iteration, `hop_count` strictly monotonically increases ($h_{n+1} = h_n + 1$). Since $h_0 = 0$ and $h_{max} = 3$, the loop condition $h < 3$ can evaluate to `true` at most 3 times. The state machine is guaranteed to reach the terminal node `SynthesizeWithCitations` within at most 3 retrieval cycles ($O(1)$ bounded steps).

### 2.3 Live SSE Telemetry Event Schema (`AgenticTelemetryEvent`)

To enable rich ChatGPT-like interactive visualization in the Web UI, every graph transition and state evaluation emits a structured SSE event:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", content = "data")]
pub enum AgenticTelemetryEvent {
    /// State machine transitioned to a new node
    #[serde(rename = "state_transition")]
    StateTransition {
        from_state: AgenticStateName,
        to_state: AgenticStateName,
        hop: usize,
        timestamp_ms: u64,
    },
    /// Query intent classified and routing rationale
    #[serde(rename = "intent_classified")]
    IntentClassified {
        intent: QueryIntentType,
        search_mode: SearchMode,
        model_family: SynthesisModelFamily,
        model_name: String,
        routing_rationale: String,
        is_retrieval_required: bool,
    },
    /// Host hardware resources probed
    #[serde(rename = "hardware_probed")]
    HardwareProbed {
        free_ram_gb: f64,
        free_vram_mb: u64,
        gpu_name: Option<String>,
        selected_tier: String,
    },
    /// Retrieval finished for a hop
    #[serde(rename = "retrieval_completed")]
    RetrievalCompleted {
        hop: usize,
        query: String,
        search_mode: SearchMode,
        retrieved_count: usize,
        local_count: usize,
        remote_count: usize,
        latency_ms: u64,
    },
    /// Relevance grading outcome
    #[serde(rename = "relevance_graded")]
    RelevanceGraded {
        hop: usize,
        grade: RelevanceGrade,
        score: f32,
        reasoning: String,
        will_rewrite: bool,
    },
    /// Query rewritten for the next retrieval hop
    #[serde(rename = "query_rewritten")]
    QueryRewritten {
        hop: usize,
        original_query: String,
        rewritten_query: String,
        rewrite_strategy: String,
    },
    /// LLM synthesis initiated
    #[serde(rename = "synthesis_started")]
    SynthesisStarted {
        model_name: String,
        model_family: SynthesisModelFamily,
        context_chunks_count: usize,
        estimated_prompt_tokens: usize,
    },
    /// Streaming token delta for live typing effect
    #[serde(rename = "token_delta")]
    TokenDelta {
        delta: String,
    },
    /// Grounded source citation emitted
    #[serde(rename = "citation")]
    CitationEmitted {
        citation: Citation,
    },
    /// Execution completed
    #[serde(rename = "execution_finished")]
    ExecutionFinished {
        total_hops: usize,
        total_duration_ms: u64,
        total_tokens: usize,
        final_state: AgenticStateName,
        model_name: String,
    },
    /// Fail-closed error reporting
    #[serde(rename = "execution_error")]
    ExecutionError {
        failed_state: AgenticStateName,
        error_message: String,
        fallback_action: String,
    },
}
```

### 2.4 Multi-Model Architecture & Hardware-Aware GLM Selection

The synthesis layer supports hot-swappable model families:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisModelFamily {
    #[default]
    Glm4Agentic,
    Qwen25Code,
    GemmaProse,
    AutoFusion,
}
```

#### Family Specialization & Tiering
1. **`Glm4Agentic` (Default)**:
   - Primary default reasoning engine.
   - Model tag: `glm-4-9b-chat` or `glm4:9b` (installed locally).
   - Supports 128K–1M context tokens (131,072 in `glm4:9b`), making it ideal for dense multi-document RAG and multi-hop synthesis without context truncation.
   - Resource floor: requires >= 8.0 GB free RAM OR >= 6.0 GB free VRAM (runs in 4-bit Q4_0 ~5.5 GB).

2. **`Qwen25Code`**:
   - Specialized for source code queries (Rust, Python, TypeScript, C++, etc.).
   - Dynamically sized strictly by runtime free RAM / free VRAM (Ollama Dynamic Memory Law):
     - Free RAM >= 48 GB OR Free VRAM >= 24 GB -> `qwen2.5:32b`
     - Free RAM >= 24 GB OR Free VRAM >= 14 GB -> `qwen2.5:14b`
     - Free RAM >= 12 GB OR Free VRAM >= 4.5 GB -> `qwen2.5:7b`
     - Free RAM >= 6 GB OR Free VRAM >= 2 GB -> `qwen2.5:3b`
     - Free RAM < 6 GB -> `qwen2.5:1.5b`
   - Invariant: Never appends `-coder` suffixes (uses official tags `qwen2.5:32b`, `qwen2.5:7b`, etc.).

3. **`GemmaProse`**:
   - Crisp, natural English prose and concise summarization.
   - Aligns cleanly with the underlying `embeddinggemma-300m` embedding space.
   - Models: `gemma3:27b` (high memory), `gemma3:9b` / `gemma2:9b` (mid memory), `gemma2:2b` (low memory).

4. **`AutoFusion` Adaptive Router**:
   - Dynamically evaluates both query/chunk characteristics and hardware resources:
     - **Code AST Detection**: Checks prompt for programming language keywords (`fn `, `def `, `impl `, `struct `, `.rs`, `.py`) or inspects retrieved chunks: if >50% of chunks have file paths ending in code extensions or Magika `FileCategory::Code` -> routes to `Qwen25Code`.
     - **Multi-Hop / Heavy Context**: If query is multi-hop or retrieved chunk content exceeds 16 KB -> routes to `Glm4Agentic` to leverage its 128K context window.
     - **General Prose / QA**: Routes to `GemmaProse` or `Glm4Agentic`.
     - **Hardware Telemetry Check**: Evaluates live free RAM and free VRAM to assign the exact model parameter size.
     - Emits selected model and reasoning rationale in `AgenticTelemetryEvent::IntentClassified`.

### 2.5 Deterministic Offline Synthesis Fallback

If Ollama daemon (`127.0.0.1:11434`) is offline or unreachable:
- Coordinator does NOT panic or fail with an empty response.
- Falls back to `DeterministicOfflineSynthesisEngine`:
  - Builds grounded synthesis from top scored chunks with explicit citations.
  - Generates bulleted synthesis with document references.
  - Streams token deltas and citations cleanly.
  - Emits telemetry indicating `offline_deterministic_fallback`.

---

## 3. Caveats

1. **Ollama Startup Latency & Probing Timeout**:
   - Probing `/api/tags` or `/api/generate` with long timeouts blocks async runtimes.
   - **Remediation**: Use a strict 400ms timeout for health checks, cache tag lists for 10 seconds (as observed in `ModelFusion`), and run blocking HTTP probes on dedicated `tokio::task::spawn_blocking` threads.
2. **Windows Non-NVIDIA Host Fallback**:
   - `nvidia-smi` is specific to NVIDIA GPUs.
   - **Remediation**: If `nvidia-smi` returns `None`, treat VRAM as `0.0 GB` and size models purely based on `free_ram_gb` via `sysinfo`.
3. **Loop Convergence on Paraphrase Oscillation**:
   - If the query rewriter produces a query that was already queried in a previous hop, retrieval will produce identical chunks.
   - **Remediation**: In `AgenticGraphState`, maintain `query_history: Vec<String>`. If `new_query == old_query`, force exit to `SynthesizeWithCitations`.
4. **Context Window Safety Margins**:
   - Although `glm4:9b` has a 131K context window, local Ollama default runner may configure `num_ctx = 4096` or `8192` unless explicitly requested in request options.
   - **Remediation**: When dispatching to Ollama, pass `"options": { "num_ctx": 32768 }` to unlock the expanded context headroom.

---

## 4. Conclusion & Actionable Recommendations

### 4.1 Recommended File Blueprint for `crates/quorum-coordinator`

To integrate this architecture cleanly into Milestone 4, the Worker should organize `crates/quorum-coordinator` as follows:

```
crates/quorum-coordinator/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Public exports: AgenticCoordinator, Telemetry, Models
    ├── error.rs                # CoordinatorError definitions
    ├── types.rs                # State graph state, intent, and grade types
    ├── telemetry.rs            # AgenticTelemetryEvent, SSE formatting, telemetry channel
    ├── hardware.rs             # HardwareTelemetry, Windows RAM/VRAM probe (sysinfo + nvidia-smi)
    ├── model_router.rs         # SynthesisModelFamily, AutoFusion adaptive router, Ollama client
    ├── offline_engine.rs       # Deterministic offline synthesis engine (fail-closed fallback)
    └── agentic/
        ├── mod.rs              # AgenticStateGraph runner and loop convergence guard
        ├── classify.rs         # ClassifyIntent node
        ├── retrieve.rs         # RetrieveMesh node (calls scatter-gather coordinator)
        ├── grade.rs            # GradeRelevance node (threshold check + max_hops circuit breaker)
        ├── rewrite.rs          # RewriteQuery node (query reformulation + history tracking)
        └── synthesize.rs       # SynthesizeWithCitations node (LLM streaming + citation cards)
```

### 4.2 Exact Implementation Code Blueprints

#### File 1: `crates/quorum-coordinator/src/types.rs`
```rust
use serde::{Deserialize, Serialize};
use quorum_types::{Citation, DocumentChunk, ScoredChunk, VolatileRemoteChunk};
use quorum_storage::SearchMode;
use uuid::Uuid;
use std::time::Instant;

/// Unique identifiers for the agentic state machine nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticStateName {
    ClassifyIntent,
    RetrieveMesh,
    GradeRelevance,
    RewriteQuery,
    SynthesizeWithCitations,
    Complete,
    Failed,
}

/// High-level classification of user intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntentType {
    /// General knowledge or conversational QA (retrieval optional/empty)
    GeneralConversation,
    /// Targeted local file or code query
    LocalDocumentSearch,
    /// Cross-node federated mesh query
    FederatedMeshSearch,
    /// Complex multi-hop comparative reasoning
    MultiHopReasoning,
}

/// Relevance assessment of retrieved chunks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelevanceGrade {
    pub is_relevant: bool,
    pub score: f32, // 0.0 .. 1.0
    pub reasoning: String,
    pub satisfactory: bool,
}

/// Model family options for synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisModelFamily {
    #[default]
    Glm4Agentic,
    Qwen25Code,
    GemmaProse,
    AutoFusion,
}
```

#### File 2: `crates/quorum-coordinator/src/hardware.rs`
```rust
use sysinfo::System;
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardwareTelemetry {
    pub total_ram_gb: f64,
    pub free_ram_gb: f64,
    pub cpu_cores: usize,
    pub gpu_name: Option<String>,
    pub free_vram_mb: u64,
}

impl HardwareTelemetry {
    pub fn probe() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let total_ram_gb = sys.total_memory() as f64 / 1_073_741_824.0;
        let free_ram_gb = sys.available_memory() as f64 / 1_073_741_824.0;
        let cpu_cores = sys.physical_core_count().unwrap_or_else(|| sys.cpus().len()).max(1);

        let (gpu_name, free_vram_mb) = Self::probe_gpu();

        Self {
            total_ram_gb,
            free_ram_gb,
            cpu_cores,
            gpu_name,
            free_vram_mb,
        }
    }

    fn probe_gpu() -> (Option<String>, u64) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut cmd = Command::new("nvidia-smi");
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            cmd.args(["--query-gpu=name,memory.free", "--format=csv,noheader,nounits"]);
            
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let line = text.trim();
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let free_mb: u64 = parts[1].parse().unwrap_or(0);
                        return (Some(name), free_mb);
                    }
                }
            }
        }
        (None, 0)
    }
}
```

#### File 3: `crates/quorum-coordinator/src/model_router.rs`
```rust
use crate::hardware::HardwareTelemetry;
use crate::types::SynthesisModelFamily;
use quorum_types::ScoredChunk;

pub struct AutoFusionRouter;

impl AutoFusionRouter {
    /// Selects model family, tag, and rationale based on intent, retrieved chunks, and hardware.
    pub fn select_model(
        requested_family: SynthesisModelFamily,
        prompt: &str,
        chunks: &[ScoredChunk],
        hw: &HardwareTelemetry,
    ) -> (SynthesisModelFamily, String, String) {
        match requested_family {
            SynthesisModelFamily::Glm4Agentic => {
                ("glm4:9b".to_string(), "GLM-4-9B explicitly selected (128K context window)".to_string())
            }
            SynthesisModelFamily::Qwen25Code => {
                let tag = Self::size_qwen_by_hardware(hw);
                (tag, "Qwen 2.5 Code selected with hardware-sized parameter tier".to_string())
            }
            SynthesisModelFamily::GemmaProse => {
                let tag = Self::size_gemma_by_hardware(hw);
                (tag, "Gemma Prose selected for natural language synthesis".to_string())
            }
            SynthesisModelFamily::AutoFusion => {
                // Adaptive routing
                let lower = prompt.to_lowercase();
                let has_code_keywords = lower.contains("fn ") || lower.contains("def ") || lower.contains("impl ")
                    || lower.contains("struct ") || lower.contains("rust") || lower.contains("python") || lower.contains("typescript");

                let code_chunks_count = chunks.iter().filter(|c| {
                    let p = c.chunk.file_path.to_lowercase();
                    p.ends_with(".rs") || p.ends_with(".py") || p.ends_with(".ts") || p.ends_with(".js") || p.ends_with(".go") || p.ends_with(".cpp")
                }).count();

                if has_code_keywords || (chunks.len() > 0 && code_chunks_count * 2 >= chunks.len()) {
                    let tag = Self::size_qwen_by_hardware(hw);
                    return (
                        SynthesisModelFamily::Qwen25Code,
                        tag,
                        format!("AutoFusion: query/chunks dominated by code ({}/{} chunks) -> Qwen 2.5 Code", code_chunks_count, chunks.len()),
                    );
                }

                // If multi-document context > 16KB or multi-hop -> GLM-4-9B (128K context)
                let total_chars: usize = chunks.iter().map(|c| c.chunk.text_content.len()).sum();
                if total_chars > 16_000 || chunks.len() > 6 {
                    return (
                        SynthesisModelFamily::Glm4Agentic,
                        "glm4:9b".to_string(),
                        format!("AutoFusion: heavy multi-document context ({} chars across {} chunks) -> GLM-4-9B (128K context)", total_chars, chunks.len()),
                    );
                }

                // Default general QA / prose -> GLM-4-9B or Gemma
                if hw.free_ram_gb >= 12.0 || hw.free_vram_mb >= 4500 {
                    (
                        SynthesisModelFamily::Glm4Agentic,
                        "glm4:9b".to_string(),
                        "AutoFusion: high hardware headroom -> GLM-4-9B default reasoning".to_string(),
                    )
                } else {
                    (
                        SynthesisModelFamily::GemmaProse,
                        "gemma2:2b".to_string(),
                        "AutoFusion: constrained hardware -> lightweight Gemma 2B".to_string(),
                    )
                }
            }
        }.into_tuple(requested_family)
    }

    /// Ollama Dynamic Memory Law: Size Qwen strictly by runtime free RAM/VRAM
    pub fn size_qwen_by_hardware(hw: &HardwareTelemetry) -> String {
        if hw.free_ram_gb >= 48.0 || hw.free_vram_mb >= 24_000 {
            "qwen2.5:32b".to_string()
        } else if hw.free_ram_gb >= 24.0 || hw.free_vram_mb >= 14_000 {
            "qwen2.5:14b".to_string()
        } else if hw.free_ram_gb >= 12.0 || hw.free_vram_mb >= 4_500 {
            "qwen2.5:7b".to_string()
        } else if hw.free_ram_gb >= 6.0 || hw.free_vram_mb >= 2_000 {
            "qwen2.5:3b".to_string()
        } else {
            "qwen2.5:1.5b".to_string()
        }
    }

    pub fn size_gemma_by_hardware(hw: &HardwareTelemetry) -> String {
        if hw.free_ram_gb >= 32.0 || hw.free_vram_mb >= 16_000 {
            "gemma3:27b".to_string()
        } else if hw.free_ram_gb >= 12.0 || hw.free_vram_mb >= 6_000 {
            "gemma2:9b".to_string()
        } else {
            "gemma2:2b".to_string()
        }
    }
}

trait TupleHelper {
    fn into_tuple(self, fam: SynthesisModelFamily) -> (SynthesisModelFamily, String, String);
}

impl TupleHelper for (String, String) {
    fn into_tuple(self, fam: SynthesisModelFamily) -> (SynthesisModelFamily, String, String) {
        (fam, self.0, self.1)
    }
}
```

#### File 4: `crates/quorum-coordinator/src/agentic/mod.rs` (State Machine Runner)
```rust
use crate::hardware::HardwareTelemetry;
use crate::model_router::AutoFusionRouter;
use crate::telemetry::AgenticTelemetryEvent;
use crate::types::*;
use quorum_types::{Citation, ScoredChunk};
use tokio::sync::mpsc::Sender;
use std::time::Instant;

pub struct AgenticStateGraphRunner {
    pub max_hops: usize,
    pub relevance_threshold: f32,
}

impl Default for AgenticStateGraphRunner {
    fn default() -> Self {
        Self {
            max_hops: 3,
            relevance_threshold: 0.60,
        }
    }
}

impl AgenticStateGraphRunner {
    pub async fn execute(
        &self,
        prompt: &str,
        requested_family: SynthesisModelFamily,
        telemetry_tx: Option<Sender<AgenticTelemetryEvent>>,
        retriever: impl Fn(&str, usize) -> tokio::task::JoinHandle<Vec<ScoredChunk>>,
        synthesizer: impl Fn(&str, &[ScoredChunk], &str) -> tokio::task::JoinHandle<String>,
    ) -> Result<String, String> {
        let mut state = AgenticStateName::ClassifyIntent;
        let mut hop_count = 0;
        let mut current_query = prompt.to_string();
        let mut query_history = vec![current_query.clone()];
        let mut retrieved_chunks: Vec<ScoredChunk> = Vec::new();

        // 1. Hardware probe
        let hw = HardwareTelemetry::probe();
        if let Some(ref tx) = telemetry_tx {
            let _ = tx.send(AgenticTelemetryEvent::HardwareProbed {
                free_ram_gb: hw.free_ram_gb,
                free_vram_mb: hw.free_vram_mb,
                gpu_name: hw.gpu_name.clone(),
                selected_tier: format!("RAM: {:.1}GB free, VRAM: {}MB", hw.free_ram_gb, hw.free_vram_mb),
            }).await;
        }

        // Loop until Complete or Failed
        loop {
            match state {
                AgenticStateName::ClassifyIntent => {
                    let next_state = AgenticStateName::RetrieveMesh;
                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx.send(AgenticTelemetryEvent::StateTransition {
                            from_state: state,
                            to_state: next_state,
                            hop: hop_count,
                            timestamp_ms: 0,
                        }).await;
                    }
                    state = next_state;
                }

                AgenticStateName::RetrieveMesh => {
                    let start = Instant::now();
                    let handle = retriever(&current_query, hop_count);
                    retrieved_chunks = handle.await.map_err(|e| e.to_string())?;
                    let duration = start.elapsed().as_millis() as u64;

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx.send(AgenticTelemetryEvent::RetrievalCompleted {
                            hop: hop_count,
                            query: current_query.clone(),
                            search_mode: quorum_storage::SearchMode::Hybrid,
                            retrieved_count: retrieved_chunks.len(),
                            local_count: retrieved_chunks.iter().filter(|c| !c.is_remote).count(),
                            remote_count: retrieved_chunks.iter().filter(|c| c.is_remote).count(),
                            latency_ms: duration,
                        }).await;
                    }

                    let next_state = AgenticStateName::GradeRelevance;
                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx.send(AgenticTelemetryEvent::StateTransition {
                            from_state: state,
                            to_state: next_state,
                            hop: hop_count,
                            timestamp_ms: duration,
                        }).await;
                    }
                    state = next_state;
                }

                AgenticStateName::GradeRelevance => {
                    // Score relevance: max score of retrieved chunks
                    let max_score = retrieved_chunks.iter().map(|c| c.score).fold(0.0f32, f32::max);
                    let is_satisfactory = max_score >= self.relevance_threshold || retrieved_chunks.is_empty();

                    let will_rewrite = !is_satisfactory && hop_count < self.max_hops;

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx.send(AgenticTelemetryEvent::RelevanceGraded {
                            hop: hop_count,
                            grade: RelevanceGrade {
                                is_relevant: is_satisfactory,
                                score: max_score,
                                reasoning: if is_satisfactory { "Sufficient relevance" } else { "Low relevance" }.to_string(),
                                satisfactory: is_satisfactory,
                            },
                            score: max_score,
                            reasoning: format!("Top chunk score: {:.3}", max_score),
                            will_rewrite,
                        }).await;
                    }

                    // Strict convergence guard: if max hops reached, force SynthesizeWithCitations
                    if will_rewrite {
                        let next_state = AgenticStateName::RewriteQuery;
                        if let Some(ref tx) = telemetry_tx {
                            let _ = tx.send(AgenticTelemetryEvent::StateTransition {
                                from_state: state,
                                to_state: next_state,
                                hop: hop_count,
                                timestamp_ms: 0,
                            }).await;
                        }
                        state = next_state;
                    } else {
                        let next_state = AgenticStateName::SynthesizeWithCitations;
                        if let Some(ref tx) = telemetry_tx {
                            let _ = tx.send(AgenticTelemetryEvent::StateTransition {
                                from_state: state,
                                to_state: next_state,
                                hop: hop_count,
                                timestamp_ms: 0,
                            }).await;
                        }
                        state = next_state;
                    }
                }

                AgenticStateName::RewriteQuery => {
                    hop_count += 1;
                    let rewritten = format!("{} (focused context hop {})", prompt, hop_count);
                    
                    // Anti-cycle check
                    if query_history.contains(&rewritten) {
                        // Cycle detected, break to synthesis
                        state = AgenticStateName::SynthesizeWithCitations;
                        continue;
                    }

                    query_history.push(rewritten.clone());
                    current_query = rewritten.clone();

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx.send(AgenticTelemetryEvent::QueryRewritten {
                            hop: hop_count,
                            original_query: prompt.to_string(),
                            rewritten_query: rewritten,
                            rewrite_strategy: "Semantic disambiguation".to_string(),
                        }).await;
                    }

                    let next_state = AgenticStateName::RetrieveMesh;
                    state = next_state;
                }

                AgenticStateName::SynthesizeWithCitations => {
                    let (chosen_family, model_tag, rationale) = AutoFusionRouter::select_model(
                        requested_family,
                        prompt,
                        &retrieved_chunks,
                        &hw,
                    );

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx.send(AgenticTelemetryEvent::IntentClassified {
                            intent: QueryIntentType::LocalDocumentSearch,
                            search_mode: quorum_storage::SearchMode::Hybrid,
                            model_family: chosen_family,
                            model_name: model_tag.clone(),
                            routing_rationale: rationale,
                            is_retrieval_required: true,
                        }).await;

                        let _ = tx.send(AgenticTelemetryEvent::SynthesisStarted {
                            model_name: model_tag.clone(),
                            model_family: chosen_family,
                            context_chunks_count: retrieved_chunks.len(),
                            estimated_prompt_tokens: 500,
                        }).await;
                    }

                    let handle = synthesizer(prompt, &retrieved_chunks, &model_tag);
                    let result = handle.await.map_err(|e| e.to_string())?;

                    if let Some(ref tx) = telemetry_tx {
                        for chunk in &retrieved_chunks {
                            let _ = tx.send(AgenticTelemetryEvent::CitationEmitted {
                                citation: chunk.to_citation(),
                            }).await;
                        }

                        let _ = tx.send(AgenticTelemetryEvent::ExecutionFinished {
                            total_hops: hop_count,
                            total_duration_ms: 0,
                            total_tokens: result.len() / 4,
                            final_state: AgenticStateName::Complete,
                            model_name: model_tag,
                        }).await;
                    }

                    return Ok(result);
                }

                AgenticStateName::Complete | AgenticStateName::Failed => {
                    break;
                }
            }
        }

        Err("Graph terminated unexpectedly".to_string())
    }
}
```

---

## 5. Verification Method

To independently verify the architecture, execution contracts, and failure handling, execute the following commands and assertions:

### 5.1 Verification Commands
```powershell
# 1. Compile workspace crates to verify clean boundary dependencies
cargo check --workspace

# 2. Verify storage isolation and zero Arrow type leakage
cargo test -p quorum-storage
cargo test --test test_storage_isolation

# 3. Unit test hardware probe and model sizing
cargo test -p quorum-coordinator test_hardware_sizing_matrix

# 4. Agentic State Graph convergence test (max 3 hops circuit breaker)
cargo test -p quorum-coordinator test_agentic_loop_convergence_max_hops

# 5. Telemetry SSE serialization roundtrip
cargo test -p quorum-coordinator test_telemetry_event_serialization
```

### 5.2 Test Assertions & Expected Invariants
1. **Convergence Invariant**:
   Assert that for an adversarial test query with mocked relevance score `0.1` (< threshold 0.6), the graph transitions:
   `ClassifyIntent (hop 0) -> RetrieveMesh -> GradeRelevance -> RewriteQuery (hop 1) -> RetrieveMesh -> GradeRelevance -> RewriteQuery (hop 2) -> RetrieveMesh -> GradeRelevance -> RewriteQuery (hop 3) -> RetrieveMesh -> GradeRelevance -> SynthesizeWithCitations`.
   Total rewrite loops must equal exactly `3`, and execution must terminate cleanly without stack overflow.

2. **Ollama Dynamic Memory Law Invariant**:
   ```rust
   let hw_high = HardwareTelemetry { free_ram_gb: 50.0, free_vram_mb: 26_000, ..Default::default() };
   assert_eq!(AutoFusionRouter::size_qwen_by_hardware(&hw_high), "qwen2.5:32b");

   let hw_low = HardwareTelemetry { free_ram_gb: 4.0, free_vram_mb: 500, ..Default::default() };
   assert_eq!(AutoFusionRouter::size_qwen_by_hardware(&hw_low), "qwen2.5:1.5b");
   ```

3. **Arrow Isolation Invariant**:
   Run `cargo tree -p quorum-coordinator` and assert that neither `arrow-array` nor `lancedb` appears in the dependency tree.

---
*Report completed and delivered by explorer_m4_3.*
