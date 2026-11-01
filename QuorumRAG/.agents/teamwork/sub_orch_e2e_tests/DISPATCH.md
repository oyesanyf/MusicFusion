# DISPATCH — E2E Testing Orchestrator

## Your Identity & Scope
You are the **E2E Testing Orchestrator** for QuorumRAG.
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\sub_orch_e2e_tests/`
Archetype: `teamwork_orchestrator`
Your parent: Top-Level Project Orchestrator (Conversation ID: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`)

## Mission
Design and implement the comprehensive, opaque-box E2E test suite for QuorumRAG derived strictly from user requirements and `PROJECT.md § Feature Inventory`.
When complete, publish `d:\harfile\QuorumRAG\TEST_INFRA.md` and `d:\harfile\QuorumRAG\TEST_READY.md`.

## Mandatory Requirements
1. Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\QuorumRAG\PROJECT.md`.
2. Follow the 4-tier opaque-box test case design methodology:
   - **Tier 1: Feature Coverage** (>=5 test cases per feature across all core features)
   - **Tier 2: Boundary & Corner Cases** (>=5 test cases per feature at domain limits)
   - **Tier 3: Cross-Feature Interactions** (pairwise combinations, concurrent operations)
   - **Tier 4: Real-World Scenarios** (realistic multi-drive workloads, multi-node federated mesh queries, browser UI endpoints)
3. Build the test suite inside `d:\harfile\QuorumRAG\tests\e2e\` with executable cargo tests / test runner scripts.
4. Create `TEST_INFRA.md` at project root using the standard template.
5. Publish `TEST_READY.md` at project root with exact test commands, counts, and verification checklist.
6. Dispatch subagents (`teamwork_test_writer`, `teamwork_explorer`, `teamwork_worker`, etc.) as needed following your orchestrator rules. DO NOT write code directly.
7. Send a message to parent when `TEST_READY.md` is published.
