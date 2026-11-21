# Agent 05 — Approved research to canonical three-script writing (21 Nov 2026)

Status: **DRAFT, offline integration only, no paid dispatch, not certified for merge/release.**

## Source of authority

The generation plan must be created with
`AppServices::prepare_three_writing_plan_from_research(project, study, run, artifact_uuid)`.
The private `CurrentStudy::load_for_research` reopens Agent03's exact approved
`AnalyzeEvidence` artifact, requires the saved Completed run + approved step +
Applied effect, and rejects changed/forged/revoked original source content.
The original run ID is recovered from the **same checked report envelope**.
Agent03's verified evidence and findings replace the independently reconstructed
Study excerpts; the persisted Study brief remains canonical.

The serialized writing context includes artifact UUID, approved run ID,
distinct canonical attachment IDs, distinct original transcript IDs, original
content SHA-256, attachment fingerprint/revision/collection date and the
original bounded evidence/finding locators. The writing validator rejects
nonmatching attachment identity, missing/invalid digests, an unknown research
evidence ID or fewer than three genuinely attributable original sources.
Only the host produces a trusted research-bound context; a client DTO cannot
selectively replace it.

`generate_three` reloads the exact original artifact before `angles`, after
`angles`, before/after each of `draft-1`, `draft-2`, `draft-3`, and
before commit. The existing v11 IMMEDIATE transaction repeats the same
research-bound loader after acquiring its write reservation. A changed original
or Study revision fails closed with no draft/receipt. The already committed
v11 receipt can be inspected read-only after restart even if the Study has
since changed or a human has edited a newer draft version.

Legacy `prepare_three_writing_plan` and the unapproved host fixture path are
compiled only with `cfg(test)`. The production host generation and direct
batch-save entrypoints reject unapproved research. They are still **private**
and have no registered provider transport/Tauri command.

## Required Agent02 + Agent00 post-v13 paid-result contract (NOT IMPLEMENTED)

The existing `StepAdmission::Recorded { response_json }` is only an
offline/fixture interface, **not proof of real paid response durability**.
No claim of exactly-once paid recovery is made on this branch. Agent02/00
must provide one reviewed additive migration (v14 if v13 is registered):

1. Preserve four distinct immutable effect records: `angles`,
   `draft-1`, `draft-2`, `draft-3`, each bound to exact project,
   Study, writing run, research artifact/run, original Study revision and
   source digest set, step, provider account/model, sealed request bytes and
   attempt/idempotency key.
2. Atomically settle the *authenticated* provider receipt, usage, charge and
   bounded original response bytes + SHA-256 in the same IMMEDIATE transaction,
   after the exact request crossed its v13 send boundary. Never treat receipt
   status `Completed` alone as durable response content.
3. Expose a typed `load_four_sealed_effect_outcomes(tx,...)` that validates
   all four distinct original effect/receipt/byte records under the existing
   v11 batch transaction, and refuses missing bytes, digest mismatch, wrong
   provider/account, wrong run, substituted prompt or stale research.
4. A response lost before atomic sealing becomes **UncertainEffect / charged
   outcome unresolved**. Never call the provider a fifth time, regenerate a
   synthetic answer, or silently accept a partial three-script batch.
5. Preserve independent retention/erasure policy for bounded provider bytes
   and never persist API credentials with project/Study data.

The v11 `GeneratedBatch`, three deterministic UUIDs, immutable initial
versions, exact source links, strict angle/draft parsers, original-copy guard,
short EN/AR near-duplicate guard, CTA allowance, unverified factual claims,
idempotent read-only receipt and human-edit targeted CAS remain unchanged.

## Owners and acceptance

- **Agent 00:** independently review source ancestry and combine approved
  Agent01 v13 + Agent02 v14 with this branch. Do not move owner PR114 or main
  until security/installed-app acceptance.
- **Agent 02:** implement typed authenticated four-outcome persistence and
  crash-safe read-only replay without a fifth paid request.
- **Agent 06:** expose an explicitly selected approved research artifact in the
  Studio journey; stale research must be a clear recoverable denial, and
  completed batch reopen must never trigger paid egress.
- **Agent 10:** independent source/receipt/cross-process race certification.

Required exact-head checks: Rustfmt, strict Clippy, Rust workspace and writing
tests, research/SQLite migrations, hostile replay/provenance tests, security
and native independently approved installed-app acceptance. Protected
packaging source pins must veto unreviewed heads. **Paid egress stays OFF.**
