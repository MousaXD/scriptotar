from pathlib import Path

path = Path('apps/desktop-ui/src/views/TranscriptView.svelte')
text = path.read_text()
replacements = {
    "downloadFile('txt', selected.text)": "downloadFile('txt', selected?.text || '')",
    "timestampedText(selected.segments)": "timestampedText(selected?.segments || [])",
    "srt(selected.segments)": "srt(selected?.segments || [])",
    "vtt(selected.segments)": "vtt(selected?.segments || [])",
}
for old, new in replacements.items():
    if old not in text:
        raise SystemExit(f'expected TranscriptView export closure was not found: {old}')
    text = text.replace(old, new, 1)
path.write_text(text)
