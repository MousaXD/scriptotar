"""Safely stage a synthetic migrated SQLite fixture for installed desktop trials.

This tool never starts Scriptotar or certifies a native GUI. It refuses existing
installed app data and requires both source and destination scratch sentinels.
"""
from __future__ import annotations

import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3

SHA = re.compile(r'[0-9a-f]{40}\Z')
STUDY_TABLES = ('content_studies','study_sources','study_briefs','creative_drafts','draft_versions')
INTEGRATION = ('legacy_import_support','job_transcript_lineage','safe_job_transcript_backfill',
               'job_transcript_project_guard','parent_lineage_reassignment_guards')
SENTINEL = '.scriptotar-benchmark-only'


def digest(path: Path) -> str:
    hash_ = hashlib.sha256()
    with path.open('rb') as inp:
        for block in iter(lambda: inp.read(1024*1024), b''):
            hash_.update(block)
    return hash_.hexdigest()


def scratch_dir(path: Path) -> Path:
    if not path.is_absolute() or path.is_symlink() or not path.is_dir():
        raise ValueError('scratch directory must be an existing real absolute directory')
    path = path.resolve(strict=True)
    if path == Path.home().resolve() or path == Path('/'):
        raise ValueError('refusing home or filesystem root as scratch directory')
    marker = path / SENTINEL
    if marker.is_symlink() or not marker.is_file() or marker.read_text(encoding='utf-8').strip() != 'SYNTHETIC_ONLY':
        raise ValueError('scratch directory requires SYNTHETIC_ONLY sentinel')
    return path


def verify_fixture(db: Path, manifest: Path, count: int, source_sha: str) -> dict:
    if not SHA.fullmatch(source_sha) or count not in (100,1000,10000):
        raise ValueError('source SHA or canonical Study count invalid')
    if not db.is_file() or db.is_symlink() or not manifest.is_file() or manifest.is_symlink():
        raise ValueError('fixture and manifest must be real files')
    scratch_dir(db.parent)
    if db.parent.resolve() != manifest.parent.resolve():
        raise ValueError('manifest and fixture must share one guarded source directory')
    if any(Path(str(db)+suffix).exists() for suffix in ('-wal','-shm')):
        raise ValueError('fixture has outstanding WAL/SHM sidecars; refuse inconsistent snapshot')
    meta = json.loads(manifest.read_text(encoding='utf-8'))
    if (meta.get('dataset') != 'SYNTHETIC_ONLY' or meta.get('per_collection') != count
            or meta.get('source_sha_operator_supplied') != source_sha
            or meta.get('schema_version') != 12 or meta.get('study_tables') != list(STUDY_TABLES)):
        raise ValueError('fixture manifest source, dataset, size or v12 schema mismatch')
    db_hash = digest(db)
    if meta.get('db_sha256') != db_hash or meta.get('db_file_bytes') != db.stat().st_size:
        raise ValueError('fixture checksum/size disagrees with manifest')
    with closing(sqlite3.connect(db.resolve().as_uri() + '?mode=ro',uri=True)) as con:
        con.execute('PRAGMA query_only=ON')
        version = con.execute('PRAGMA user_version').fetchone()[0]
        if version != 12:
            raise ValueError('fixture is not current schema v12')
        if [x[0] for x in con.execute('SELECT version FROM schema_migrations ORDER BY version')] != list(range(1,13)):
            raise ValueError('fixture core migration ledger mismatch')
        if list(con.execute('SELECT version, name FROM integration_schema_migrations ORDER BY version')) != list(enumerate(INTEGRATION,1)):
            raise ValueError('fixture integration migration ledger mismatch')
        for table in STUDY_TABLES:
            if con.execute(f'SELECT COUNT(*) FROM {table}').fetchone()[0] != count:
                raise ValueError(f'fixture row-count mismatch for {table}')
        if con.execute("SELECT COUNT(*) FROM content_studies WHERE title NOT LIKE 'SYNTHETIC %'").fetchone()[0]:
            raise ValueError('fixture contains non-synthetic Study titles')
        projects=list(con.execute('SELECT id, name FROM projects'))
        if len(projects)!=1 or projects[0][0] != meta.get('project_id') or projects[0][1] != f'SYNTHETIC QA {count}':
            raise ValueError('fixture project identity mismatch')
        if con.execute('PRAGMA foreign_key_check').fetchall() or con.execute('PRAGMA quick_check').fetchone()[0] != 'ok':
            raise ValueError('fixture failed SQLite integrity check')
    return {'fixture_sha256':db_hash, 'fixture_bytes':db.stat().st_size,
            'study_count':count, 'source_sha':source_sha, 'project_id':meta['project_id'],
            'manifest_sha256':digest(manifest)}


def stage(fixture: Path, manifest: Path, data_dir: Path, count: int,
          source_sha: str, output: Path) -> dict:
    # Validate both sentinel directories and input provenance before ANY write.
    info=verify_fixture(fixture,manifest,count,source_sha)
    data_dir=scratch_dir(data_dir)
    if fixture.resolve(strict=True).parent == data_dir:
        raise ValueError('source and installed scratch locations must be distinct')
    target=data_dir/'scriptotar.sqlite3'
    if target.exists() or target.is_symlink() or Path(str(target)+'-wal').exists() or Path(str(target)+'-shm').exists():
        raise ValueError('refusing to overwrite any existing installed-app database or SQLite sidecar')
    if not output.is_absolute() or output.parent.resolve() != data_dir or output.exists() or output.is_symlink():
        raise ValueError('fresh report must be inside destination scratch directory')
    created=False
    try:
        fd=os.open(target,os.O_CREAT | os.O_EXCL | os.O_WRONLY | getattr(os,'O_NOFOLLOW',0),0o600)
        created=True
        with os.fdopen(fd,'wb') as dest, fixture.open('rb') as inp:
            shutil.copyfileobj(inp,dest,1024*1024)
            dest.flush()
            os.fsync(dest.fileno())
        if digest(target)!=info['fixture_sha256']:
            raise ValueError('staged DB differs from source')
        proof={**info, 'scope':'SYNTHETIC_INSTALLED_DB_STAGE_ONLY_NOT_NATIVE_CERTIFICATION',
               'staged_db':str(target), 'staged_sha256':digest(target),
               'certification':'PENDING_REAL_INSTALLED_APP_READBACK',
               'limitations':'Matching staged files do not prove that installed WebKitGTK/WebView2 opened this DB; independently observe real app process, exact Study counts and schema after startup.'}
        with output.open('x',encoding='utf-8') as writer:
            json.dump(proof,writer,indent=2)
            writer.write('\n')
        return proof
    except Exception:
        if created:
            target.unlink(missing_ok=True)
        raise


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--fixture',type=Path,required=True)
    p.add_argument('--manifest',type=Path,required=True)
    p.add_argument('--data-dir',type=Path,required=True)
    p.add_argument('--study-count',type=int,choices=(100,1000,10000),required=True)
    p.add_argument('--source-sha',required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    try:
        res=stage(a.fixture,a.manifest,a.data_dir,a.study_count,a.source_sha,a.output)
        print('Staged synthetic file only; installed UI NOT TESTED: '+res['staged_db'])
    except (ValueError,OSError,sqlite3.Error,KeyError,json.JSONDecodeError) as err:
        p.exit(2,f'staging refused: {err}\n')

if __name__=='__main__':
    main()
