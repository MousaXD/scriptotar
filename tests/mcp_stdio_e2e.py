"""Black-box JSON-RPC MCP over real Rust process (no network)."""
import json
import os
import pathlib
import subprocess
import tempfile

BIN = pathlib.Path('target/debug/scriptotar-mcp')
with tempfile.TemporaryDirectory() as directory:
    env = {**os.environ, 'SCRIPTOTAR_DB_PATH': str(pathlib.Path(directory) / 'test.sqlite3')}
    with subprocess.Popen([str(BIN)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env) as p:
        def rpc(method, params=None, id=1):
            payload = {'jsonrpc':'2.0', 'id':id, 'method':method}
            if params is not None: payload['params'] = params
            p.stdin.write(json.dumps(payload)+'\n')
            p.stdin.flush()
            reply = json.loads(p.stdout.readline())
            assert reply['id'] == id, reply
            return reply
        assert rpc('initialize')['result']['serverInfo']['name'] == 'scriptotar'
        tools = rpc('tools/list')['result']['tools']
        assert {tool['name'] for tool in tools} == {'list_projects','list_research','list_transcripts'}
        assert json.loads(rpc('tools/call', {'name':'list_projects','arguments':{}})['result']['content'][0]['text']) == []
        assert rpc('tools/call', {'name':'list_research','arguments':{'project_id':'bad'}})['result']['isError']
        assert rpc('tools/call', {'name':'list_projects','arguments':{'unexpected':'x'}})['result']['isError']
        assert rpc('tools/call', {'name':'execute_shell'})['error']['code'] == -32602
        assert rpc('nonexistent')['error']['code'] == -32601
        p.stdin.close()
        assert p.wait(timeout=5) == 0
print('MCP stdio process E2E passed')
