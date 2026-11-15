# Agent 04 — P1 claim verification and source provenance

**Repo:** `MousaXD/scriptotar` · **Branch:** `agent-04/p1-fact-verification-20261116` · **Baseline:** `3009989a3aa73248b6432ba0cd84821c59110993`

**Scope:** Add a deterministic, zero-network verification core and read-only canonical Study adapter. Agent 00 owns schemas, production API, user interface wiring, merge and release. Do not merge this PR or authorize a release on the basis of these unit tests.

## Delivered

- `scriptotar_ai::writing::verification`: typed claim proposals (fact/opinion/prediction/unsupported), consequential risk, exact `StudySource`/evidence identity, declared content version, excerpt equality, segment/time and modality, source state and freshness, independent publisher lineage, contradiction and reviewer decision handling.
- Fail-closed statuses: **verified, unverified, contradicted, blocked, opinion, prediction**. Verified requires TWO distinct independently owned, reviewer-assessed documentary sources, an exact current independent human review and unexpired version-matched evidence. A model label never suffices. Revoked, untrusted, disputed, unavailable, foreign-project, duplicate, forged locator/quote and stale-version citations prevent verification.
- Structured `VerificationReport` with per-claim `FindingCode`, precise source/excerpt citations and version-bound `review_lineage`. Older review records are preserved in the output but cannot certify a different draft revision. A host must persist this lineage in an append-only ledger before claiming durability.
- `AppServices::verify_studio_claims_read_only`: resolves draft claim evidence IDs against the currently attached project-/Study-scoped SQLite records. Missing IDs are flagged. Two reads detect common stale changes. **It deliberately sets all creator observations to unreviewed/unassessed and supplies no human approvals** even when legacy records carry `corroborated` metadata. No provider call, network fetch, data egress or persistent changes.
- Offline adversarial unit tests and malicious JSON fixture. Minimal additive module registrations only; no existing DTO or migration is changed.

## Contract to Agent 00, Agent 03 and Agent 05

1. **Agent 00 / schema:** If factual "verified" is to be surfaced, approve an additive *authenticated append-only* claim-review ledger bound to `(project_id, study_id, draft_id, draft_version, claim_id, exact_claim_text, exact_evidence_ids, source_content_versions)`. Require reviewer identity and authenticated human action, immutable decision/audit history, transactionally consistent source snapshot, revoke/expiry invalidation, and canonical source **content hashes**, not UUID-v5 identity fingerprints or user-supplied `source_revision`.
2. **Agent 03 / Studio:** Consume per-claim `VerificationReport.claims[].findings` with exact source locator, status and review needs. Do not display `Verified` until authenticated storage and endpoint integration are complete. Keep old version reports visible without treating them as current certificates. Require consequential human-review gates.
3. **Agent 05 / research:** Supply *observed* document/transcript/visual provenance separately. Transcript-derived words are `spoken_transcript`; user-entered "segment 1" and engagement metrics are not verified visual frames. In future supply real segment IDs + bounded start/end timestamps validated against stored segment JSON and independently owned factual documents with provenance/access constraints.
4. **Agent 06 / writing:** `FactualClaim.status == Supported` is a provider proposal only. The new adapter resolves exact evidence IDs; never map model-emitted supported/corroborated labels into an authenticated reviewer attestation.

## Known limitations (fail-closed)

- No network fact-checking or independent external-source retrieval. Merely having multiple social transcripts does **not** establish factual corroboration.
- SQLite v9 `study_evidence` captures creator observations and human unverified↔disputed transitions, not independent factual review decisions. The production adapter cannot produce Verified today; this is deliberate. It does not persist a revision verification ledger or expose a new Tauri/TS command.
- Source `fingerprint` is an identity/deduplication digest; `source_revision` is caller-declared, not an authenticated content hash. Neither proves source authenticity or freshness. Unknown versions stay blocked.
- Existing `EvidenceInput.locator` is editable free text, not verified transcript segment storage. Adapter preserves raw locator/timecode, but does NOT fabricate a segment ID. Timed transcript verification therefore remains insufficient until canonical segment resolution is added.
- Existing saved observations contain a claim, not an independently retrieved verbatim excerpt. The adapter preserves it as an unreviewed observation, not primary-source proof.
- Claim extraction is typed over existing `ScriptDraft.claims`; it cannot confidently detect every implied factual statement in arbitrary free-form narration. Unannotated beats are conservatively reported as unsupported. Users/LLMs proposing `Opinion` or `Prediction` is not proof of classification, and no publication should rely solely on model classification.
- This adapter's repeated scoped reads are NOT a transactionally atomic snapshot; it is advisory and cannot serve as a final publication authorization gate. Agent 00 must add atomic snapshot/CAS integration with revision and review ledger.
- No local Rust toolchain is available in the editing environment. CI must independently run fmt, check, Clippy, tests and integrated end-to-end gates **on the exact PR SHA**. Fixture tests are not evidence of real-world factual accuracy.

**Safety state:** draft, no merge, no release. Issue #116 and parent PR #114 remain open.
