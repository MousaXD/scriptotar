# Agent 04 — P1-01 bounded agent coordinator design

**Status:** isolated prototype. **Branch:** `agent-04/defragmentation-agent-runtime` targeting `plan/scriptotar-defragmentation`. **Owner:** Agent 04. Production integration remains **blocked** on Agent 00 approval of P0 Content Study, Draft, provenance, and AppServices contracts.

## Source inspection and architectural fit

Read the current defragmentation README, ROADMAP_P0_P3, CONTINUATION, Rust foundation and technical-futures audit, plus current `scriptotar-jobs`, `scriptotar-orchestrator`, `scriptotar-mcp`, `scriptotar-ai`, `scriptotar-research`, and `scriptotar-db` code on the planning branch.

- Existing `scriptotar-jobs` provides typed, persisted media states and explicit `mark_active_jobs_interrupted`; do **not** treat job enqueue as transcript completion.
- Existing `scriptotar-orchestrator` owns sidecar lifecycle, media progress, transcript commit and interruption. The agent must call existing application services, not a second sidecar.
- Existing `McpPermissions` defaults to disabled with granular filesystem/network/command controls. **Never** expose MCP's command-execution capability to the agent, even when independently enabled for MCP.
- `scriptotar-ai` provides request-time BYOK, endpoint validation and provider controls. No API keys belong in an agent plan, snapshot or audit event.
- `scriptotar-research` validates supported URLs; access restrictions and unavailability are real outcomes.
- `scriptotar-db` owns SQLite migrations, WAL, foreign keys and project relationships. Agent 04 owns **no** migrations.

Old audit documents describe older baselines; this is not a new P0 certification.

## Current isolated crate

`crates/scriptotar-agent` has its own `[workspace]` solely to permit stand-alone compilation without touching the root manifest. Agent 00 must remove that nested stanza when registering it in the root Cargo workspace.

### Plan and state contracts

- `Plan`: opaque `run_id`, `project_id`, `study_id`, ordered typed `StepPlan`, and `Limits`. IDs intentionally remain opaque; cross-project membership must be checked by the future app adapter.
- `StepInput` is a closed enum: research discovery/selection, transcript queue/read, evidence analysis, draft/revision, artifact save. No arbitrary tool name, shell, HTTP URL, browser cookie API or publishing command.
- `StepPlan`: stable ID, tool-specific input, up-front token/cost/CPU/output/memory ceiling, max 1–5 attempts and approval flag. Mutating/network and any costed tool require per-step approval, regardless of the caller's flag.
- `RunSnapshot`: versioned, serialized, revisioned checkpoint with current step, per-step attempt, approval, idempotency key, opaque output ID and metadata-only audit events.
- `RunState`: `planned`, `awaiting_approval`, `running`, `cancelling`, `paused`, `needs_input`, `blocked`, `completed`, `failed`, `cancelled`.
- `StepState`: `pending`, `running`, `awaiting_result`, `completed`, `failed`, `cancelled`.

### Dispatch and durability invariant

```text
Plan -> pending step -> approval (if required) -> policy recheck
      -> budget reservation -> CAS checkpoint "running"
      -> service adapter execute(idempotency_key)
          | actual committed result -> completed step
          | accepted/queued job     -> awaiting_result + needs_input
          | known not applied       -> paused for bounded retry / failed
          | uncertain effect        -> blocked; probe only
```

- `CheckpointStore::compare_and_swap(expected_revision, snapshot)` MUST atomically persist a snapshot **and** its audit events before an external effect. The fixture `MemoryStore` implements this atomically under a mutex. **It is not durable across an OS/process restart.** The production SQLite adapter and schema belong to Agent 01/00.
- One `tick()` dispatches at most one step. Concurrent ticks contend on the same persisted CAS claim; a failed CAS never executes the tool.
- Idempotency key `<run_id>:<ordinal>:<step_id>` is stable across retries/restarts. The application service must deduplicate it transactionally or maintain an effect ledger; the runtime cannot guarantee exactly-once execution against non-idempotent providers by itself.
- `ToolReceipt::Applied` is **only** legal after the actual action is committed. `ToolReceipt::Accepted` is a queued request, not a completed transcript or saved draft. `reconcile()` must verify the exact effect using the existing job or application-service ledger.
- On crash, `recover()` never replays a running or cancelling effect; it marks it blocked for `probe()`. `NotApplied` permits bounded retry; `Unknown` forbids automatic retry. Retrying an API call with unknown provider status is unsafe without provider-side idempotency.
- `cancel()` commits intent first and calls the adapter's best-effort `request_cancel`. A concurrent successful side effect is recorded truthfully as a completed **step** inside a cancelled **run**, not discarded or portrayed as unperformed.
- `pause()` prevents the next dispatch; for a running request, it pauses after the result. An accepted background job may still run while the agent is paused; pause is not a job cancel.
- Audit events only contain step ID, event type, sequence and timestamp; raw provider responses, bearer tokens and error strings must not appear. Unknown error messages become `tool_failure`.

### Budget/permission contract

Limits include cumulative tool calls, reserved tokens, reserved micro-USD, reserved CPU milliseconds, elapsed milliseconds, maximum response bytes and peak memory MiB. Dispatch checks the estimates, records reservations before execution, and validates reported receipt totals. These are **adapter-side upper bounds**, not measured provider bills or enforceable OS quotas: the real provider/media service must enforce timeout, output length, tokens, network egress and resource caps, and report actual amounts conservatively. Persisted `started_ms` and callback `now_ms` must use the **same restart-stable UTC epoch-millisecond clock**. A backward clock jump blocks new dispatches; the application service must additionally enforce a monotonic in-flight deadline. The `ExecutionContext.deadline_ms` field is an epoch-millisecond deadline derived from the saved run start. Absolute time budgets cannot be reliably enforced by the coordinator alone if the host clock changes.

**Approval is necessary but not sufficient.** `PolicyGate` must separately validate active project, Study membership, source IDs, current provider/session access, user-granted egress and paid-provider consent. `ToolExecutor` should call the policy-checked `AppServices` functions, never raw database writes or unscoped MCP execution.

## Proposed adapter contract for Agent 00, 01 and 02

No shared DTO/schema was assumed or changed. Proposed forward-only app integration:

| Runtime operation | Future AppServices / P0 boundary | Completion evidence |
| --- | --- | --- |
| DiscoverResearch | policy-checked creator/source discovery with project/study context | real persisted research batch ID |
| SelectResearch | attach canonical selected source IDs to the same project/study | committed study-source relationship |
| QueueTranscript | queue existing media job with idempotency key | **accepted job ID**, pending until terminal media job + transcript |
| ReadTranscript | project-checked on-demand transcript fetch | observed persisted transcript ID/revision |
| AnalyzeEvidence | bounded selected-source analysis; independent fact references | persisted evidence/finding ID |
| DraftScript | approved BYOK/Copy Prompt via existing AI service | saved, versioned draft ID, not model text alone |
| ReviseScript | draft revision with optimistic concurrency | persisted draft-version ID |
| SaveArtifact | existing approved export/save service | durable artifact ID and verified path/status |

All service methods need `project_id`, `study_id`, `idempotency_key`, request/approval context, and an operation-status lookup by the same key. Tools must not internally call `execute_command`, scrape browser cookies, use shell or auto-publish. User-supplied source/transcript text remains data and is never interpreted as tool instructions.

## Validation and limits of this PR

The fixture suite contains **34 authored tests** covering plan validation, approval/denial, missing tool, policy revocation, actual-versus-queued completion, retries, stable keys, uncertain effects, failures, partial progress, resource/time/cost/output limits, cancellation and pause races, serialization/restart, corrupted checkpoint, competing CAS claims and late receipt attribution. They have **not** been executed in a Rust-capable environment. Test code is deterministic and makes no network or paid requests.

Run after checkout on a Rust-capable worker:

```sh
cargo fmt --manifest-path crates/scriptotar-agent/Cargo.toml -- --check
cargo test --manifest-path crates/scriptotar-agent/Cargo.toml --all-targets
cargo clippy --manifest-path crates/scriptotar-agent/Cargo.toml --all-targets -- -D warnings
```

**Not claimed:** Rust test execution or lint success in this authoring environment (no Rust/Cargo installed); production SQLite durability; provider-side idempotency; real app-source authorization; Tauri/Studio integration; shipped autonomous workflow. Root workspace CI does not necessarily compile a separately isolated crate until Agent 00 registers it.

## P0 integration gates

1. Freeze Content Study/Draft/Evidence identity and project authorization contracts with Agents 00–02.
2. Agent 00 reviews the enum/API boundary; no production wiring without approval.
3. Agent 01/00 adds transactional SQLite checkpoint/effect-ledger migrations and durable CAS implementation, with migration/reopen tests. Do not use `MemoryStore` in production.
4. Agent 02 provides policy-checked typed AppServices adapters with service-side deduplication and actual completion probes.
5. Agent 00 registers crate in root workspace and updates lockfile, schedules exact-head Rust tests and network/provider opt-in integration gates.
6. Independent QA verifies crash after every CAS boundary, project switch, credential egress, cancel/restart races, repeated paid request, queued-job completion, EN/AR UI audit and installed-app behavior before any P1 GO.

**Readiness: isolated design/fixtures only; integration NO-GO.**
