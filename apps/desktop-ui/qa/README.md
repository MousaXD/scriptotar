# Browser-rendered GUI QA

This directory owns QA infrastructure, not the product API, CSS, or runtime.

## Run

From apps/desktop-ui on Node 22:

1. npm ci && npm run check && npm run test && npm run build
2. npm install --no-save --package-lock=false --ignore-scripts playwright@1.56.1
3. npx playwright install --with-deps chromium (Linux CI)
4. node qa/visual-regression.mjs
5. node qa/verify-snapshots.mjs

The runner starts the actual Vite production preview on 127.0.0.1:4173 and stops it. It outputs 116 real Chromium PNG screenshots and a measured JSON report into qa/evidence/. The CI artifact is named for the exact GitHub commit SHA. Screenshots are not synthetic images.

## Matrix and checks

At 1440, 1100, and 760 widths with English LTR / Arabic RTL and dark / system-light themes: Dashboard, Research, Jobs, Transcript, AI Studio, Library, Settings, and scrolled MCP setup. Include 12 library empty-search screenshots plus 8 loading/error screenshots captured from rendered real Svelte components with injected pending/rejected bootstrap. Total: 116 screenshots.

Checks cover focus-visible style, command-palette focus trapping and Escape, actual native selector/range appearance against a narrow allowlist, navigation shortcuts, overflow, browser exceptions, direction and stored appearance. Browser performance metrics report time to useful shell, view navigation and optional JS heap memory; the labels explicitly say browser mock client.

## Baseline promotion

Until qa/baselines/manifest.json is approved, the image-diff step explicitly says NO APPROVED VISUAL BASELINE. CI is capture-and-interaction only, never pixel-regression-certified. After reviewing every rendered screenshot from a successful exact-commit run, the coordinator may set QA_APPROVED_SHA to that exact 40-character commit and run node qa/approve-baseline.mjs with its full artifact. The manifest must be independently reviewed before it is committed. CI then rejects any missing, extra or differing image hashes. Updating the baseline is never automatic or a way to suppress failures.

## Scope limits

The normal browser flow uses Scriptotar's existing built-in mock API. Loading/error states inject only pending or failed bootstrap IPC for visualization; they do not mock screenshot pixels. Neither flow exercises actual Tauri, SQLite, MCP auth, downloads, Whisper or native installers. Measure native cold/warm startup, backend bootstrap payload, large-database query/memory and installed-app acceptance separately. Do not compare mock UI timings across machines as evidence of native performance gains.

The allowlist is qa/native-control-allowlist.json: genuine native file/color inputs only. Unstyled selects/ranges elsewhere are regressions for the owning designer or view owner to fix.
