from pathlib import Path

path = Path('scripts/apply_technical_futures_phase3.py')
text = path.read_text()
old = r'''            return rows\n                .collect::<Result<Vec<_>, _>>()\n                .map_err(storage_error);\n        }\n\n        let like_query = format!(\"%{}%\", escape_like(raw_query.trim()));\n'''
new = r'''            let ids = rows\n                .collect::<Result<Vec<_>, _>>()\n                .map_err(storage_error)?;\n            if !ids.is_empty() {\n                return Ok(ids);\n            }\n        }\n\n        let like_query = format!(\"%{}%\", escape_like(raw_query.trim()));\n'''
if old not in text:
    raise SystemExit('expected FTS return block was not found in phase three staging script')
path.write_text(text.replace(old, new, 1))
