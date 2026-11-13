# Agent 04 — durable agent runtime handoff

**Repository:** https://github.com/MousaXD/scriptotar
**Base:** `plan/scriptotar-defragmentation`
**Working branch:** `agent-04/defragmentation-agent-runtime`
**Task:** P1-01 (isolated foundation, **not** production agent activation)
**Date:** 2026-11-13

## Completed on assigned branch

- Implemented `crates/scriptotar-agent` as a separate Rust crate; no root workspace changes.
- Closed typed tool vocabulary and validated Plan/StepInput contracts.
- Added deny-by-default tool registry, separately injected project/provider policy gate and service-only execution/probe/cancellation interfaces.
- Added user approval/denial, pause/resume, cancellation intent, pending external job status, stable idempotency keys and bounded retries.
- Added CAS-before-side-effect checkpoint port, versioned serializable snapshots and metadata-only audit events with safe error-code redaction.
- Added token/API cost/tool-call/CPU/time/output-byte/memory budgets and fail-closed receipt validation.
- Added truthful queued-versus-completed semantics and non-replaying crash recovery requiring status probe.
- Added **34 deterministic fake-service tests** for every run state, retries, simultaneous CAS claims, stale completion races, duplicate dispatch, approval denial, cancellation races, paused queue progress, provider failure, resource limits and snapshot corruption. These tests are authored but **not yet executed**.
- Added architecture and proposed integration contract in `docs/defragmentation/proposals/agent-04-runtime.md`.

## Independent runtime functions

These functions are implemented without changing any P0 crate: `Engine::create`, `get`, `approve`, `deny`, `tick`, `pause`, `resume`, `cancel`, `recover`, and `reconcile`; typed `ToolRegistry`, `ToolExecutor`, `PolicyGate`, `CheckpointStore` and test `MemoryStore`.

This is a **source-level implementation status**, not a claim of executed unit tests or a usable AI agent.

## Exact validation status

- Source inspection and isolated code/test authoring: **done**.
- Local `cargo test`, `cargo fmt`, and `cargo clippy`: **NOT RUN** — the available container has no Cargo, rustc or rustfmt, and outbound GitHub clone/network access is unavailable from that container. The PR cannot be considered source-compiled or format-clean.
- Hosted PR CI: workflows are triggered by PR updates, but the existing Rust foundation and CI YAML invoke `cargo ... --workspace` only. The agent crate deliberately has its own nested `[workspace]` and therefore **those workflows do not validate this crate**. The isolated checks remain a separate required gate.
- Native packaged app, SQLite restart and provider/media E2E: **NOT RUN / OUT OF SCOPE** before P0 integration.
- Existing media/job/orchestrator code was only inspected; it was not modified.

## What is blocked on Agent 00 and P0

1. Approve/freeze P0 Study/Source/Evidence/Draft identities and reference ownership with Agents 01 and 02.
2. Choose the production SQLite checkpoint/effect ledger and ensure transactionally atomic `revision` CAS and audit append. `MemoryStore` is a **test double**, not durable storage.
3. Ensure each policy-checked AppServices tool returns a real committed result or stable queued job ID, and supports status by idempotency key. The runtime cannot guarantee provider-side exactly-once effects by itself.
4. Implement actual provider-side max cost/token, timeout, memory and egress enforcement. Reservations here are an upper-bound contract.
5. Wire Tauri/UI only after Agent 00 approves P0 and registers the crate in the root workspace (remove its nested `[workspace]` and update Cargo.lock).
6. Run the isolated Rust test commands and exact-head CI before integration approval, then test real SQLite interruption and native installs.

## Clock and resource note

`started_ms` and `now_ms` are restart-stable UTC epoch milliseconds; do not supply a monotonic uptime counter across processes. The host is still responsible for deadline enforcement on running services using its monotonic clock and for true CPU/RAM hard limits. Agent 00 must explicitly certify these adapters, not equate coordinator reservations with real metering.

## Review verdict

**Ready for Agent 00 design and code review as a draft; not ready for integration, merge or autonomous release.** No modifications to `main` or shared P0-owned files. See proposal for complete API and security details.
