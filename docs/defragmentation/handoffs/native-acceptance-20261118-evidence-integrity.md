# Agent 07 — installed native acceptance evidence integrity (2026-11-18)

**Status: NO-GO.** This is a forensic checkpoint, not production certification and not a substitute for independent installed-app execution on a final integrated commit. PR #131 remains draft. No production application, database, dependency, security test, or paid-egress code was modified in this QA work.

## Immutable installed run reviewed

- Source: `9c272a92de8df42a1ecfee5211a4f7ea3bb74e44` (Agent 07 QA branch, **not final integrated SHA**).
- [Installed run #44302808654](https://github.com/MousaXD/scriptotar/actions/runs/44302808654).
- Linux installed .deb WebKitGTK/AT-SPI: **10 PASS, 1 FAIL, 5 NOT TESTED**; job overall failed.
- Windows installed NSIS/WebView2/UIA: **11 PASS, 0 FAIL, 5 NOT TESTED**; job overall failed closed.
- Linux .deb SHA-256: `268a22b87d3ea90e82af40a1e318d27fd9fe499aab906ae8de59e79d9cc9df93`.
- Linux installed executable SHA-256: `9d66d7b02480533b2d8d4d8dd2905a9362361dc73a436191534643608265665a`.
- Windows NSIS installer SHA-256: `ccfb178fbcf4d02f41794ec7ea801bed179a71dc332301247f520610619b958d`.
- Windows installed executable SHA-256: `4c2ccd3c78b0394cd3e54e99ba720e6f6b3c32680f9efebde28567bc36fc4f27`.

### All 16 required cases on this source

| Mandatory installed case | Linux | Windows |
|---|---|---|
| installed_webview | PASS | PASS |
| create_switch_projects | PASS | PASS |
| canonical_project_persistence | PASS | PASS |
| create_study | PASS | PASS |
| edit_save_duplicate_candidate | PASS | PASS |
| quit_relaunch_persistence | PASS | PASS |
| wrong_project_ui_isolation | PASS | PASS |
| wrong_project_ipc_denial | NOT TESTED | NOT TESTED |
| multi_source_transcript | NOT TESTED | NOT TESTED |
| source_revocation | NOT TESTED | NOT TESTED |
| error_recovery | **FAIL** | PASS |
| keyboard_custom_controls | PASS | PASS |
| arabic_locale_control | PASS | PASS |
| theme_mode_control | PASS | PASS |
| arabic_rtl | NOT TESTED | NOT TESTED |
| light_dark_visual | NOT TESTED | NOT TESTED |

### Reproduced Linux Queue accessibility gap

Actual installed Queue actions: click **Queue** -> type `file:///scriptotar-native-qa-denied-media.mp4` in **Media URL** -> click **Queue URL**. The Linux screenshot `failed-denied-url-no-error.png` visibly shows the red error text:

> validation failed: research URL is invalid: only http and https are supported

The installed AT-SPI snapshot `a11y-diagnostics.json` contains the Queue controls `Media URL` and `Queue URL` but no named error/alert node or matching error text. Windows UIA exposes the same actual validation error and passes the error/recovery case. Thus this is a verified **semantic native accessibility defect/gap**, not evidence of a backend acceptance of the unsupported URL.

The failed Linux source checked alert semantics **before** `count_canonical_jobs` following the rejected Queue action. The screenshot displays **All 0**, but that is *not* a completed independent read-only SQLite no-write proof for this failed step. Report the original negative-recovery gate as FAIL, not PASS.

- [Small Linux screenshot/AT-SPI report archive](https://github.com/MousaXD/scriptotar/actions/runs/44302808654/artifacts/14427142757) (ID 14427142757, ZIP digest `sha256:6a9e6b1715dbced0a8e390274d33305c43caf2bed42849aca6a92c9572ce53b3`).
- [Full Linux .deb/evidence archive](https://github.com/MousaXD/scriptotar/actions/runs/44302808654/artifacts/14427273474).
- [Small Windows UIA/report archive](https://github.com/MousaXD/scriptotar/actions/runs/44302808654/artifacts/14428460649).
- [Full Windows NSIS/evidence archive](https://github.com/MousaXD/scriptotar/actions/runs/44302808654/artifacts/14427229971).

### Adversarial evidence-integrity finding

Agent 10 demonstrated that `checked(report, "error_recovery", fn)` previously accepted successful callback return and wrote PASS even when a screenshot-derived `angle_rejection_visual_proof` explicitly had `atspi_error_text_exposed: false` and `human_review_required: true`. A new red border is neither semantic IPC rejection nor independent human adjudication.

QA-only commits:
- [ab1e9e88](https://github.com/MousaXD/scriptotar/commit/ab1e9e88b728256895f329c902470d97a62b8b1e): `checked` makes `error_recovery` NOT TESTED without semantic native rejection proof; capture read-only canonical Queue before/after count even when no accessible Queue error appears.
- [93a3b7ae](https://github.com/MousaXD/scriptotar/commit/93a3b7ae8b27d58dbbbe551a3d38e22654e379b8): regression fixtures for unresolved human review, flag flipping without semantics, absent proof, valid helper-only semantic proof, and Queue evidence ordering.

**These unit assertions are not native certification.** [Fresh native run #44307630119](https://github.com/MousaXD/scriptotar/actions/runs/44307630119), [CI #44307630160](https://github.com/MousaXD/scriptotar/actions/runs/44307630160), [security hygiene #44307630106](https://github.com/MousaXD/scriptotar/actions/runs/44307630106) were dispatched by the new QA commits; do not copy verdicts from source `9c272a92` to these runs. The independent Agent 10 fixture must also be rerun against exact Agent 07 source.

### Five mandatory native gaps (both systems)

1. Forged foreign-project IPC: send forged project, Study, draft and/or receipt IDs through **real installed-host Tauri commands**; check explicit rejection and canonical SQLite ownership/no writes. UI isolation alone is insufficient.
2. Genuine multiple-source transcript: ingest at least two different actual permitted sources through production acquisition/transcription; bind real source IDs, transcript/provenance timestamps and canonical records. No simulated transcript or seeded fake rows.
3. Source revocation: establish real allowed source authorization, revoke, retry stale capability through the host, and verify refusal plus read-only source/Study/candidate records. Detaching a Study is not source revocation.
4. Arabic RTL: independent end-to-end visual/accessibility inspection across navigation, forms, dialogs, mixed Arabic/Latin and numbers, focus order, editor alignment, and persistence on both installed platforms. The captured Arabic menu alone does not certify the whole interface.
5. Light/Dark/System appearance: independently inspect all requested appearances on both native platforms, persistence, operating-system setting response, contrast/focus and dialogs. Exact Linux screenshot `dark-theme-menu-open.png` shows only **Dark** and **System** choices, with no explicit Light option; Agent 00 must resolve the intended product contract.

### Completion / next integration gate

Agent 00 owns the Queue AT-SPI error naming and any missing production contracts; [handoff on PR #114](https://github.com/MousaXD/scriptotar/pull/114#issuecomment-6979076191). Agent 10 owns independent adversarial evidence checks; [coordination on PR #124](https://github.com/MousaXD/scriptotar/pull/124#issuecomment-6979091336).

For a future GO recommendation require **16/16 true installed PASS on Linux and Windows on one final integrated SHA**, immutable package/executable hashes, AT-SPI/UIA semantics, real IPC/backend denial and recovery, independent read-only canonical SQLite evidence, screenshot review with reviewer identity and disposition, and no hidden skips. Source-only or mocked CI cannot substitute. Keep PR #131 draft, no main merge, no release, no paid provider egress.

## Continuing independent Agent 07 evidence — current P0 closure attempt

The sections above document **historical sources**, not the current release
readiness. The QA branch is preserved atop production integration
`5ed385d09092d2c94a1bd86cb56888d96df0bb5e` through two-parent merge
`f0e2e1c0fece4956bdd6847286d10ffc20a18aa2`.
All QA commits remain individually reachable and none alter production code,
paid egress or security-gate thresholds.

### Latest independently completed installed baselines

- Linux [#44313593525](https://github.com/MousaXD/scriptotar/actions/runs/44313593525),
  QA `6981df8`: **10 PASS / 1 FAIL / 5 NOT TESTED**. Unsupported file URL
  did not create a canonical job, but no fresh named AT-SPI Queue denial.
  Later evidence from integration [#44313725742](https://github.com/MousaXD/scriptotar/actions/runs/44313725742)
  found the exact Studio native error as an AT-SPI `image map`; the QA locator
  now admits this role **only** for exact official localized errors. No screenshot
  or image-map text independently authenticates Rust rejection.
- Windows [#44333456021](https://github.com/MousaXD/scriptotar/actions/runs/44333456021),
  QA `f771ec7`: **11 PASS / 0 FAIL / 5 NOT TESTED** using actual installed UIA
  and NSIS package (executable SHA-256
  `6fdf081a386c64926a244278144d8d527cfe490eea5df0e73142b96060be8080`).
  That 11th `error_recovery` PASS preceded the later independent evidence
  challenge and must **not** be copied onto the newer QA source.

### Independent security proof defect and remediation scope

Agent 10's [exact independent security run #44334626785](https://github.com/MousaXD/scriptotar/actions/runs/44334626785)
demonstrated a genuine *certification* false positive: a **new yet unrelated**
`angle invalid` AT-SPI message could satisfy the prior QA helper despite
exact user-input readback, monotonic timestamps and unchanged SQLite. The
production may still correctly reject malformed JSON; this finding concerns
the evidence's inability to identify *which request* failed.

QA now requires an authoritative per-invocation Tauri denial identity and
typed error code observed from the **real production host and corresponding
native accessible error**, not created by the QA collector. Both OS runners
report `NOT TESTED` on the rejection gate when production lacks that contract;
a genuine failed native action still reports `FAIL`. Tests reject stale,
screenshot-only, semantically plausible unrelated and unbound proof. QA also
hashes every canonical SQLite table's row contents before/after invalid
actions, not just counts. These fixtures do not certify native IPC.

### New non-certifying optional installed probes

- Candidate production [PR #138](https://github.com/MousaXD/scriptotar/pull/138)
  introduces normal `Open Study reference` navigation. Agent 07 now probes
  canonical UI-created Project A / Study / draft IDs while Project B is active,
  exact field readback, **fresh native denial**, and all-table read-only
  before/after fingerprints. Both runners retain
  `wrong_project_ipc_denial=NOT TESTED` when only the normal **read**
  command was challenged. Forged **mutating** IPC remains required.
  Feature absence leaves the probe NOT TESTED; a present but failing native
  preflight is a FAIL. This production PR has **not** been merged into the
  installed integration source reviewed here.
- Candidate production [PR #136](https://github.com/MousaXD/scriptotar/pull/136)
  adds a distinct Light choice. QA now checks for the **actually visible**
  native Light menu item, selects/saves/captures it only if installed,
  then returns to Dark for independent relaunch persistence. With Light absent,
  `System` is **not** counted as Light. Full appearance inspection remains
  NOT TESTED pending verified screenshots and contrast/focus review.
- Candidate [PR #137](https://github.com/MousaXD/scriptotar/pull/137)
  supplies actual Study-specific source-detachment and revision fencing but
  not global origin permission revocation. A real two-source ingest/transcript
  journey, authorization withdrawal/stale retry, cross-project denial and
  immutable provenance must be exercised on both installed OSes before PASS.
- Arabic UI native navigation/persisted locale has been observed and screenshot
  captured, but form-direction, mixed-script text, dialog/focus order and
  complete product visual inspection are not yet certified as `arabic_rtl`.

**Current gate:** BLOCKED / DRAFT / NO-GO. Re-run on one exact final integrated
production SHA after Agent 00's approved contracts are merged, with all 16
real cases PASS per OS, immutable package/executable hashes, native UI trees,
canonical read-only evidence and independent Agent 10 verification. PR
[#131](https://github.com/MousaXD/scriptotar/pull/131) remains the exact-head
status authority; never extrapolate from this historical text alone.
