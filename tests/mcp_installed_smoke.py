"""Fail-closed stdio MCP handshake using an actual installed Scriptotar binary.

Runs with a disposable application-data directory, and does not touch the
user's settings, real database, or media. Cross-platform Python standard lib.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True)
args = parser.parse_args()
app = Path(args.binary).resolve()
assert app.is_file(), f'installed executable not found: {app}'

with tempfile.TemporaryDirectory(prefix='scriptotar-installed-mcp-') as temporary:
    data_dir = Path(temporary) / 'private-test-data'
    env = {**os.environ, 'SCRIPTOTAR_DATA_DIR': str(data_dir)}
    env.pop('SCRIPTOTAR_DB_PATH', None)
    with subprocess.Popen([str(app), '--mcp-stdio'], env=env,
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, text=True) as proc:
        commands = [
            {'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-11-25'}},
            {'jsonrpc':'2.0','id':2,'method':'tools/list'},
            {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'list_projects','arguments':{}}},
        ]
        for command in commands:
            proc.stdin.write(json.dumps(command) + '\n')
            proc.stdin.flush()
            line = proc.stdout.readline()
            assert line, f'MCP process died at {command["method"]}: {proc.poll()}'
            response = json.loads(line)
            assert response['id'] == command['id'], response
            if command['method'] == 'initialize':
                identity = response['result']['serverInfo']
                assert identity['name'] == 'scriptotar', response
                expected = json.loads((Path(__file__).resolve().parents[1] / 'apps/desktop/src-tauri/tauri.conf.json').read_text())['version']
                assert identity['version'] == expected, f'Installed MCP binary is {identity["version"]}; expected {expected}'
            if command['method'] == 'tools/list':
                assert any(tool['name'] == 'download_video' for tool in response['result']['tools']), response
            if command['method'] == 'tools/call':
                assert response['result']['isError'] is True, response
                assert 'disabled' in response['result']['content'][0]['text'], response
        proc.stdin.close()
        assert proc.wait(timeout=10) == 0, f'MCP child exit code: {proc.returncode}'
    assert (data_dir / 'scriptotar.sqlite3').is_file()
    assert not (data_dir / 'mcp-permissions.json').exists(), 'MCP smoke must not grant access'
print('PASS: installed Scriptotar CLI MCP handshake, tools, and default-deny permissions')
