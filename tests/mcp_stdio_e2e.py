"""Real Rust MCP stdio process contract and fail-closed permission E2E (no network)."""
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import uuid

BIN = pathlib.Path('target/debug/scriptotar-mcp')
assert BIN.is_file(), f'build the binary before running: {BIN}'

with tempfile.TemporaryDirectory() as directory:
    data = pathlib.Path(directory)
    allowed = data / 'approved'
    allowed.mkdir()
    outside = data / 'outside.txt'
    outside.write_text('private', encoding='utf-8')
    policy_file = data / 'mcp-permissions.json'

    def grant(**overrides):
        policy = {
            'enabled': False, 'readData': False, 'writeData': False,
            'readFiles': False, 'writeFiles': False, 'overwriteFiles': False,
            'executeCommands': False, 'searchCreators': False, 'downloadVideos': False,
            'allowedPaths': [], 'allowedPrograms': [],
        }
        policy.update(overrides)
        staging = data / f'.policy-{uuid.uuid4()}.tmp'
        fd = os.open(staging, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, 'w', encoding='utf-8') as stream:
            json.dump(policy, stream)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(staging, policy_file)

    env = {**os.environ, 'SCRIPTOTAR_DB_PATH': str(data / 'scriptotar.sqlite3')}
    with subprocess.Popen([str(BIN)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, text=True, env=env) as process:
        request_id = 0

        def rpc(method, params=None):
            global request_id
            request_id += 1
            payload = {'jsonrpc': '2.0', 'id': request_id, 'method': method}
            if params is not None:
                payload['params'] = params
            process.stdin.write(json.dumps(payload) + '\n')
            process.stdin.flush()
            raw = process.stdout.readline()
            assert raw, f'MCP server exited unexpectedly: {process.poll()}'
            reply = json.loads(raw)
            assert reply['id'] == request_id, reply
            return reply

        def call(tool_name, **arguments):
            return rpc('tools/call', {'name': tool_name, 'arguments': arguments})

        def content(reply):
            assert not reply['result'].get('isError', False), reply
            return json.loads(reply['result']['content'][0]['text'])

        def denied(reply):
            assert reply['result']['isError'] is True, reply
            return reply['result']['content'][0]['text']

        assert rpc('initialize', {'protocolVersion': '2025-11-25'})['result']['protocolVersion'] == '2025-11-25'
        assert rpc('ping')['result'] == {}
        tools = rpc('tools/list')['result']['tools']
        expected = {
            'list_projects', 'list_research', 'list_transcripts', 'get_transcript',
            'search_transcripts', 'list_jobs', 'create_project', 'read_file',
            'write_file', 'execute_command', 'search_creator', 'download_video',
            'list_directory', 'enqueue_local_media'
        }
        assert {tool['name'] for tool in tools} == expected
        denied(call('list_projects'))
        denied(call('write_file', path=str(allowed / 'new.txt'), content='no'))
        denied(call('execute_command', program='/bin/echo', args=['no']))
        assert rpc('nonexistent')['error']['code'] == -32601
        assert denied(call('change_permissions', enabled=True)) == 'unknown tool'

        # Capabilities are independently app granted, no implicit privilege escalation.
        grant(enabled=True, readData=True, writeData=True)
        assert content(call('list_projects')) == []
        project = content(call('create_project', name='MCP test'))
        pid = project['id']
        assert len(content(call('list_projects'))) == 1
        assert content(call('list_jobs', project_id=pid)) == []
        assert content(call('list_research', project_id=pid)) == []
        assert content(call('list_transcripts', project_id=pid)) == []
        assert content(call('search_transcripts', project_id=pid, query='hook')) == []
        denied(call('list_jobs', project_id='bad'))
        denied(call('get_transcript', project_id=pid, transcript_id=str(uuid.uuid4())))
        denied(call('create_project', name='bad', dangerous='ignored'))
        denied(call('read_file', path=str(outside)))
        denied(call('write_file', path=str(allowed / 'new.txt'), content='no'))
        denied(call('search_creator', project_id=pid, profile_url='https://youtube.com/@abc'))
        denied(call('download_video', project_id=pid, url='https://youtube.com/watch?v=abc'))
        denied(call('enqueue_local_media', project_id=pid, path=str(outside)))
        denied(call('execute_command', program='/bin/echo', args=['no']))

        # Filesystem is explicitly scoped; symlink escape is denied.
        executable = pathlib.Path(shutil.which('echo')).resolve()
        grant(enabled=True, readData=True, writeData=True, readFiles=True, writeFiles=True,
              executeCommands=True, searchCreators=True, downloadVideos=True,
              allowedPaths=[str(allowed)], allowedPrograms=[str(executable)])
        created = allowed / 'new.txt'
        assert content(call('write_file', path=str(created), content='hello'))['bytes'] == 5
        assert content(call('read_file', path=str(created)))['text'] == 'hello'
        assert any(entry['name'] == 'new.txt' for entry in content(call('list_directory', path=str(allowed)))['entries'])
        denied(call('read_file', path=str(outside)))
        denied(call('write_file', path=str(outside), content='x'))
        denied(call('write_file', path=str(created), content='replace', overwrite=True))
        allowed.joinpath('escape.txt').symlink_to(outside)
        denied(call('read_file', path=str(allowed / 'escape.txt')))
        denied(call('write_file', path=str(allowed / 'escape.txt'), content='x', overwrite=True))
        assert outside.read_text() == 'private'
        # A write grant does not permit rewriting the policy, even if its parent is scoped.
        grant(enabled=True, readFiles=True, writeFiles=True, overwriteFiles=True,
              allowedPaths=[str(data)])
        denied(call('write_file', path=str(policy_file), content='{}', overwrite=True))
        denied(call('read_file', path=str(policy_file)))
        grant(enabled=True, readFiles=True, writeFiles=True, overwriteFiles=True,
              allowedPaths=[str(allowed)])
        assert content(call('write_file', path=str(created), content='updated', overwrite=True))['bytes'] == 7
        assert content(call('read_file', path=str(created)))['text'] == 'updated'

        # Command execution only permits exact executables, not arbitrary shells.
        grant(enabled=True, executeCommands=True, allowedPaths=[str(allowed)],
              allowedPrograms=[str(executable)])
        output = content(call('execute_command', program=str(executable), args=['MCP_OK'], cwd=str(allowed)))
        assert output['exit_code'] == 0 and output['stdout'].strip() == 'MCP_OK'
        denied(call('execute_command', program='/bin/sh', args=['-c', 'echo unauthorized']))
        denied(call('execute_command', program=str(executable), cwd=str(data), args=['no']))
        denied(call('execute_command', program=str(executable), args=['\x00']))

        # Network-capable tools reject disallowed URL hosts before accessing the network.
        grant(enabled=True, searchCreators=True, downloadVideos=True)
        denied(call('search_creator', project_id=pid, profile_url='http://127.0.0.1/private'))
        denied(call('download_video', project_id=pid, url='https://example.com/movie'))

        # Invalid policy and hard revocation must take effect without restarting MCP.
        policy_file.write_text('{broken', encoding='utf-8')
        assert 'invalid' in denied(call('list_projects')).lower()
        grant(enabled=False, readData=True, writeData=True)
        assert 'disabled' in denied(call('list_projects')).lower()
        assert 'disabled' in denied(call('create_project', name='not created')).lower()
        grant(enabled=True, readData=True)
        assert len(content(call('list_projects'))) == 1
        assert rpc('tools/call', {'name': 'list_projects', 'arguments': []})['error']['code'] == -32602

        process.stdin.write('{bad json}\n')
        process.stdin.flush()
        assert json.loads(process.stdout.readline())['error']['code'] == -32700
        process.stdin.write('X' * (1024 * 1024 + 200) + '\n')
        process.stdin.flush()
        assert json.loads(process.stdout.readline())['error']['code'] == -32600
        assert len(content(call('list_projects'))) == 1
        process.stdin.close()
        assert process.wait(timeout=10) == 0
print('PASS: real MCP process, permissions, files, commands, validation, revocation, URL safety')
