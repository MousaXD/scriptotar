# Agent 07 — exact-source installed native acceptance ledger (2026-11-21)

**Disposition: RELEASE NO-GO. No new installed acceptance was executed by this handoff.** This document records original GitHub-hosted evidence and specifies the *next independently permitted* acceptance run. It is a documentation-only handoff; do not use it to advance any trusted-source pin, enable provider egress, mark a case PASS or publish a package.

## Immutable checkpoints

| Scope | Exact identity | Observation / limitation |
| --- | --- | --- |
| Frozen owner integration | `b12843c75764601248a69c0dd89ff753af8df366` / PR #114 | Existing protected review policy |
| Independently approved seven-file source stage | `b6ec7b914b55007417e8191695e2213d4117a61c` / PR #185 | Limited source-only scope; not application certification |
| Approved baseline app/package source | `5aadc5b685a1a0e3ef79fe7e1f128a5f343fc48c` / PR #200 | Original installed run [44840881509](https://github.com/MousaXD/scriptotar/actions/runs/44840881509): **Linux 11 PASS / 5 NOT TESTED; Windows 11 PASS / 5 NOT TESTED** |
| Agent07 focus instrumentation | PR #196 `d9225202e70cfb26aed6b04fce3ba0a256a49c13` | **NOT SOURCE-APPROVED**. Exactly 3 QA files changed against frozen integration; new installed run [44895730777](https://github.com/MousaXD/scriptotar/actions/runs/44895730777) stopped correctly at pre-execution source guard on both OS, before native packaging |
| Agent07 bounded visual ZIP parser | PR #198 `db584639dbf89b0b26c90054b31c4564f9935c1b` | Agent10 **SCOPED SOURCE-APPROVE** of parser blob `c49521e8baf7b32c21c6c839eb5ab6d5c2d49432` and test blob `bd1bddabeff3b330404e7897d26afa8d2048e3d4` only; original independent test-only [run 44909282388](https://github.com/MousaXD/scriptotar/actions/runs/44909282388), 4 original + 13 source-owned tests PASS per OS |
| Offline Studio candidate | PR #212 `31648bc71cc12a8d32924c0581cfb2849bbac62a` at this checkpoint | Offline cross-component candidate; not yet final source/security approved, not an installed app acceptance or paid-provider authorization |

The frozen installed baseline has the five **NOT TESTED** cases on **each** platform: `wrong_project_ipc_denial`, `multi_source_transcript`, `source_revocation`, `arabic_rtl` and `light_dark_visual`. A failed *workflow* because some cases are NOT TESTED is not five functional product failures. Do not propagate old exact-head PASS results onto a different application commit.

## Required original media inputs — presently missing

The optional Linux [original acquisition attempt in run 44840881509](https://github.com/MousaXD/scriptotar/actions/runs/44840881509) reported `INCOMPLETE` at `origin-acquisition`: no independently authenticated offline media bundle was supplied, and automatic Wikimedia acquisition was disabled after HTTP 429. The existing optional `real_source_fixtures.py` prepares **exactly two** named LibriVox origins; it does **not** satisfy a three-distinct-original-source Studio writing gate. Its unit fixtures contain FAKE bytes and cannot count as native evidence.

Before attempting native transcription, obtain by an authorized offline channel **at least three genuinely distinct recordings** with independently documentable rights for this QA use. The supplying operator records per origin: source attribution and canonical origin URL/recording ID; license/permission and acquisition date; original SHA-256/byte length; the exact verified original recording bytes; any allowed transcoded bytes with their own hash, tool/version and commands; independently measured duration/codec; and distinct original content identity. Pin the entire manifest SHA-256 and isolate the files from untrusted network downloads during acceptance. Reject missing, duplicate, unverifiable, altered or disallowed origins. Never manufacture transcripts, source records, usage, rights or approvals.

Keep audio in a controlled QA location. Do **not** upload personal or licensed-private recordings into public Actions logs/artifacts. Capture safe redacted metadata and cryptographic identities, not secret material. The verified bundle is a *precondition*, never itself proof of installed ingestion.

## Five acceptance contracts — EACH OS, one final exact reviewed source

| Case | Minimum directly observed installed evidence | Fail-closed status until observed |
| --- | --- | --- |
| `wrong_project_ipc_denial` | Real installed project A/B, a genuine **mutating** Tauri IPC request carrying a foreign project/Study/source or draft ID, authoritative application denial/request identity, immutable canonical SQLite before/after, and no foreign mutation after restart. Read-only foreign navigation and UI switching are insufficient. | NOT TESTED |
| `multi_source_transcript` | Three independent, licensed source origins; actual installed Queue/sidecar transcript ingestion through native GUI and WebKitGTK/WebView2; source UUIDs, origin digests, independent original segment IDs/timestamps; immutable SQLite and displayed Study evidence linked back to those same three sources. Neither synthetic fixture transcripts nor two-source provenance qualify. | NOT TESTED |
| `source_revocation` | Revoke one of those genuinely attached sources while research/approval is pending; prove stale research/writing denied through installed controls; observe exact app denial, no stale candidate/use/receipt, canonical DB before/after, quit/restart/reopen denial, and reapproval requires newly valid source context. | NOT TESTED |
| `arabic_rtl` | Real EN/AR switch, quit/reopen, mixed Arabic/Latin/numbers/URLs/IDs; native AT-SPI or UIA role/name/bounds and genuine Tab/Shift+Tab/Enter/Space/Escape focus order. Inspect correct reading order, direction, overlays and error announcements. Source/mock DOM and screenshots alone do not certify this case. | NOT TESTED |
| `light_dark_visual` | Installed light, dark, system-light and system-dark modes, native screenshots plus independent human visual review, real controls/focus/contrast and persisted theme after relaunch at desktop and narrow widths. Correlate each screenshot to exact package/executable/source and OS scheme. A ZIP parser boolean is not visual certification. | NOT TESTED |

## Agent07 focus-source review contract (PR #196)

Exact Git blobs against frozen integration:
- `tests/defragmentation/native-e2e/linux_webkit.py` — `8331d612ebda387db651502abd2f137396f484e4`
- `tests/defragmentation/native-e2e/windows_webview2.ps1` — `a52d0643cb41de4a98e17f36def7ea70b7c4b877`
- `tests/defragmentation/native-e2e/test_native_tab_focus_guard.py` — `507493b25bce282aa11b98a404ad6f2939f62a3c`

Agent10 must independently inspect exact source scope, Linux X11 PID/window ownership and BOTH active-window + keyboard focus guards before each key, Windows process-owned HWND/foreground verification before each key, AT-SPI/UIA focus ancestry, inter-key focus theft, misdelivery negatives and the fact that `certifies_rtl_or_visual=false` cannot upgrade any of 16 statuses. Six synthetic Linux source tests are asserted in the PR but are *not* installed/native proof. Agent07 cannot self-approve this source. The source guard must remain enabled and fail closed until exact-source approval and Agent00-controlled baseline roll-forward.

## Combined source and run requirements

1. Agent10 independently approves PR #196's exact Git blobs and lineage (or REJECTs with evidence). PR #198 source approval is limited to its exact two blobs.
2. Agent00 stages **only** individually approved QA changes with reviewed product/Studio/research changes. Preserve original commit lineage, immutable guard, dependencies, credentials and approved exclusions; perform independently approved workflow source-pin transition before executable QA runs.
3. Verify final package SHA-256, installed executable SHA-256, final **application commit** (the SAME for both OS), CI run URL, OS build/version, native runtime versions, exact QA driver Git blob identities, package installation proof, process PID/start identity and actual WebKitGTK/WebView2 document.
4. For each case capture raw native control action/element identity, timestamped focus trace, canonical SQLite read-only before/after and restart proof, negative/positive observations, screenshot/frame hashes and human review notes. Separate unsupported/missing evidence from a genuine functional failure.
5. Repeat existing 11 cases on **that exact new source**; previous PASS never carries over automatically.
6. After Agent00 integrates the authenticated Research → approved writing context → v11 receipt → Studio journey, exercise three real attributable sources, exact saved research artifact and citation identities, source invalidation, and canonical three-draft IDs. Verify manual edits, stale compare and export/reopen. Paid provider execution remains disabled unless its separate host security boundary is independently enabled; an intentionally disabled path cannot be relabeled a completed paid generation test.
7. Publish the two original complete `result.json` matrices, installed package/executable hashes, immutable evidence artifact identifiers, independent visual/accessibility review and Agent10 exact-SHA security verdict. Advance to **16/16 PASS per OS** *only when every case is genuinely observed*.

## Handoff verdict

**Source:** PR #198 scoped ZIP-reader approval only; PR #196 focus instrumentation awaiting independent exact-SHA review. **Native installed:** 11/16 Linux and 11/16 Windows, preserved from the older source `5aadc5b...`; five untested per OS. **Studio:** integrated candidate does not yet establish native final-source receipt flow. **Media:** three genuine original sources not supplied. **Release/merge/publishing:** NO-GO. There is no authorized evidence to call this 16/16, and this document does not change release policy.
