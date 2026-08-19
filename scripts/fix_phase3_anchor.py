from pathlib import Path

path = Path('scripts/apply_technical_futures_phase3.py')
text = path.read_text()
old = '''"""  function buildSearchResults(snapshot: BootstrapData, rawQuery: string): WorkspaceSearchResult[] {\n        const query = rawQuery.trim().toLocaleLowerCase();\n"""'''
new = '''"""  function buildSearchResults(snapshot: BootstrapData, rawQuery: string): WorkspaceSearchResult[] {\n    const query = rawQuery.trim().toLocaleLowerCase();\n"""'''
if old not in text:
    raise SystemExit('expected phase three search-result anchor was not found')
path.write_text(text.replace(old, new, 1))
