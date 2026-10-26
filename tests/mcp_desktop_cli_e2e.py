"""Installed-desktop-binary MCP entrypoint: SQLite, yt-dlp research, job/sidecar E2E.

All providers are deterministic local fixtures. No actual platform traffic or
model downloads are performed. Runs in the GitHub runner, not the developer PC.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

APP = Path('target/debug/scriptotar-desktop').resolve()
assert APP.is_file(), f'Missing built desktop binary: {APP}'
ROOT = Path.cwd().resolve()
SIDECAR = ROOT / 'sidecars/transcription/sidecar.py'
ENGINE = ROOT / 'sidecars/transcription/tests/installed_app_fixture_engine.py'
EXPECTED_TEXT = 'installed fixture transcript survives restart'

with tempfile.TemporaryDirectory(prefix='scriptotar-mcp-desktop-e2e-') as raw:
    root = Path(raw)
    data_dir = root / 'isolated-app-data'
    data_dir.mkdir()
    media_root = root / 'allowed-media'
    media_root.mkdir()
    media = media_root / 'fixture.mp4'
    media.write_bytes(b'isolated E2E fixture video')

    engine_launcher = root / 'fake-transcription-engine'
    engine_launcher.write_text(
        '#!/bin/sh\nset -eu\nexec python3 "$SCRIPTOTAR_E2E_FIXTURE_SCRIPT"\n', encoding='utf-8'
    )
    engine_launcher.chmod(0o700)
    research_provider = root / 'fixture-ytdlp'
    research_provider.write_text(
        '#!/usr/bin/env python3\n'
        'import json\n'
        'print(json.dumps({"webpage_url":"https://www.youtube.com/watch?v=fixture123",'
        '"id":"fixture123","title":"Fixture researched post","extractor":"youtube",'
        '"view_count":43210,"like_count":900,"upload_date":"20261020"}))\n',
        encoding='utf-8'
    )
    research_provider.chmod(0o700)
    policy_file = data_dir / 'mcp-permissions.json'

    def policy(**overrides):
        fields = {
            'enabled': True, 'readData': True, 'writeData': True,
            'readFiles': True, 'writeFiles': False, 'overwriteFiles': False,
            'executeCommands': False, 'searchCreators': True, 'downloadVideos': True,
            'allowedPaths': [str(media_root)], 'allowedPrograms': [],
        }
        fields.update(overrides)
        tmp = data_dir / '.temp-policy'
        fd = os.open(tmp, os.O_CREAT | os.O_WRONLY | os.O_EXCL, 0o600)
        with os.fdopen(fd, 'w', encoding='utf-8') as stream:
            json.dump(fields, stream)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(tmp, policy_file)

    environment = {
        **os.environ,
        'SCRIPTOTAR_DB_PATH': str(data_dir / 'scriptotar.sqlite3'),
        'SCRIPTOTAR_SIDECAR_PYTHON': sys.executable,
        'SCRIPTOTAR_SIDECAR_SCRIPT': str(SIDECAR),
        'SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE': str(engine_launcher),
        'SCRIPTOTAR_E2E_FIXTURE_SCRIPT': str(ENGINE),
        'SCRIPTOTAR_YTDLP_EXECUTABLE': str(research_provider),
        'PYTHONPATH': str(ROOT / 'sidecars/transcription'),
    }

    # A disabled server must start without granting any access.
    with subprocess.Popen([str(APP), '--mcp-stdio'], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, env=environment) as process:
        request_id = 0
        def rpc(method, params=None):
            global request_id
            request_id += 1
            call = {'jsonrpc':'2.0', 'id':request_id, 'method':method}
            if params is not None:
                call['params'] = params
            process.stdin.write(json.dumps(call) + '\n')
            process.stdin.flush()
            raw_reply = process.stdout.readline()
            assert raw_reply, f'Executable exited: {process.poll()}'
            reply = json.loads(raw_reply)
            assert reply['id'] == request_id, reply
            return reply

        def call(tool_name, **arguments):
            return rpc('tools/call', {'name': tool_name, 'arguments': arguments})

        def content(reply):
            assert not reply['result'].get('isError'), reply
            return json.loads(reply['result']['content'][0]['text'])

        def denied(reply):
            assert reply['result'].get('isError') is True, reply

        identity = rpc('initialize', {'protocolVersion':'2025-11-25'})['result']['serverInfo']
        assert identity['name'] == 'scriptotar' and identity['version'] == '1.1.0', identity
        denied(call('list_projects'))
        denied(call('download_video', project_id=str(uuid.uuid4()), url='https://youtube.com/watch?v=123'))

        policy()
        project = content(call('create_project', name='MCP Desktop E2E'))
        pid = project['id']
        assert any(p['id'] == pid for p in content(call('list_projects')))

        scanned = content(call('search_creator', project_id=pid, profile_url='https://www.youtube.com/@fixture', limit=3))
        assert scanned['saved'] == 1, scanned
        research = content(call('list_research', project_id=pid))
        assert len(research) == 1 and research[0]['view_count'] == 43210, research

        denied(call('enqueue_local_media', project_id=pid, path=str(root / 'outside.mp4')))
        local = content(call('enqueue_local_media', project_id=pid, path=str(media)))
        remote = content(call('download_video', project_id=pid, url='https://www.youtube.com/watch?v=fixture123'))
        ids = {local['job_id'], remote['job_id']}

        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            jobs = {job['id']: job for job in content(call('list_jobs', project_id=pid))}
            failures = {job_id: jobs[job_id] for job_id in ids if job_id in jobs and jobs[job_id]['state'] in ('failed','cancelled','interrupted')}
            assert not failures, failures
            if all(job_id in jobs and jobs[job_id]['state'] == 'completed' for job_id in ids):
                break
            time.sleep(0.1)
        else:
            raise AssertionError(f'Fixture jobs did not finish in 30 seconds: {jobs}')

        transcripts = content(call('list_transcripts', project_id=pid))
        assert len(transcripts) == 2, transcripts
        assert content(call('search_transcripts', project_id=pid, query='installed', limit=10)), 'Completed transcripts must be searchable'
        for item in transcripts:
            saved = content(call('get_transcript', project_id=pid, transcript_id=item['id']))
            assert saved['text'] == EXPECTED_TEXT, saved

        # Explicit file-write denial remains in place after granting media download.
        denied(call('write_file', path=str(media_root / 'write.txt'), content='disallowed'))
        policy(enabled=False)
        denied(call('list_projects'))
        denied(call('download_video', project_id=pid, url='https://www.youtube.com/watch?v=fixture123'))
        process.stdin.close()
        assert process.wait(timeout=10) == 0

    # Verify completed jobs, research and transcripts survive process restart.
    policy(readData=True, writeData=False, downloadVideos=False, searchCreators=False, readFiles=False)
    with subprocess.Popen([str(APP), '--mcp-stdio'], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, env=environment) as second:
        queries = [
            {'jsonrpc':'2.0','id':1,'method':'tools/call','params':{'name':'list_projects','arguments':{}}},
            {'jsonrpc':'2.0','id':2,'method':'tools/call','params':{'name':'list_research','arguments':{'project_id':pid}}},
            {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'list_transcripts','arguments':{'project_id':pid}}},
        ]
        for request in queries:
            second.stdin.write(json.dumps(request) + '\n')
            second.stdin.flush()
            response = json.loads(second.stdout.readline())
            assert len(json.loads(response['result']['content'][0]['text'])) >= 1, response
        second.stdin.close()
        assert second.wait(timeout=10) == 0

print('PASS: desktop binary MCP entrypoint, local permissions, research provider, two job types, transcript persistence and restart')
