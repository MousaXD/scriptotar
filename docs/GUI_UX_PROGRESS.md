# Scriptotar GUI / UX Overhaul Progress

Branch: `feature/gui-ux-overhaul`
Base: `feature/arabic-gui`
PR: #47

## Implemented

### Design foundation
- Semantic color, spacing, typography, radius, elevation, control, and motion tokens.
- Shared local SVG icon primitive.
- Consistent keyboard focus treatment and reduced-motion behavior.
- Creator Control Room design contract in `apps/desktop-ui/DESIGN.md`.

### Localization foundation
- Explicit keyed English/Arabic translations for redesigned shell and new GUI chrome.
- Existing DOM translator retained only as a compatibility bridge for views not yet fully migrated.
- New explicit-localized controls are excluded from DOM rewriting.
- Transcript/user data remains excluded from localization.

### Application shell
- Work / Create / Utility navigation hierarchy.
- Persisted collapsible sidebar rail.
- Compact language switcher and project/activity controls.
- Direct workspace search retained.
- Ctrl/Cmd+K command palette with keyboard navigation, app commands, project switching, and local workspace results.

### Home
- Action-first hierarchy.
- Failed/interrupted and active work prioritized.
- Real creator avatars used when available.
- Metrics demoted to a compact supporting row.

### Queue / Jobs
- Clear local-file and URL ingestion paths.
- Manual path entry moved into Advanced.
- Active and attention counts surfaced.
- Existing cancel/retry/open-transcript behavior preserved.
- Native Tauri file-drop events select dropped local media without bypassing the Rust enqueue validation path.
- Drag-over state provides clear visual feedback and dropped files are never auto-enqueued.
- Multiple-file drops explicitly select only the first path and report that choice to the user.

### Transcript workspace
- Persisted collapsible transcript and metadata rails.
- Search result count plus next/previous navigation.
- Current matching segment emphasis without modifying or fragmenting transcript text.
- Timestamp jump returns to full transcript context.
- Export actions consolidated into a compact menu.

### Research
- Real research thumbnails used when present.
- Locale-aware compact metrics and dates.
- Stronger selection feedback.
- Narrow-window table-to-card layout.

### Library
- Artifact type counts.
- Stronger item/source/signal/date hierarchy.
- Explicit artifact-kind labels.
- Responsive column reduction.

### AI Studio
- Progressive Configure → Source → Direction → Artifact flow.
- Copy Prompt / BYOK semantics preserved.
- Privacy boundary visible before API execution.
- Sticky action bar and result hierarchy.

### Settings
- Sticky section navigation at desktop widths.
- Transcription, media, storage, local behavior, watchlists, migration, and interface anchors.
- Unsaved-settings indicator.
- Interface language is discoverable beside Theme as well as in the top bar.
- Existing migration/output/theme behavior remains visible rather than hidden behind tabs.

### Regression coverage
Added GUI-specific tests for:
- sidebar preference persistence,
- keyboard command-palette execution,
- transcript rail persistence and content preservation,
- unsaved-settings feedback,
- Arabic switching from Settings and explicit new chrome translation,
- normalization and cleanup of native Tauri file-drop events.

The pre-existing desktop interaction suite continues to cover project switching, queue operations, job refresh, transcript search, RTL transcript content, research/watchlist behavior, Library navigation, settings persistence, appearance, and legacy migration states.

## Deliberately not faked in the frontend

### Transcript media playback
The current frontend model exposes a transcript `source` string, but not a safe Tauri media-resource URL contract. Do not render arbitrary local filesystem paths through `file://` or assume the WebView may access them.

A playback implementation should first add a Rust/Tauri-owned media access contract that returns a safe resource reference suitable for the WebView. Once that exists, add:
- play/pause,
- seek,
- speed control,
- active-segment following,
- timestamp-to-player synchronization,
- persisted playback UI preferences where useful.

Tauri's asset protocol can expose local files, but enabling it changes the filesystem-to-WebView security boundary and should be scoped deliberately rather than broadly allowing home-directory media paths.

## Next implementation targets

1. Complete migration of remaining redesigned view strings from the DOM translator to explicit translation keys.
2. Add a narrowly scoped Rust/Tauri media-resource contract, then implement transcript playback and timestamp synchronization.
3. Add rendered visual regression coverage for English dark, English system-light, Arabic dark RTL, and narrow-window layouts.
4. Verify packaged Linux/Windows WebViews for sticky rails, command palette focus behavior, native drag/drop, font fallback, and RTL geometry.
