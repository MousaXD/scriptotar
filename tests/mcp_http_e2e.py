"""Black-box localhost HTTP MCP transport and authorization certification."""
import http.client
import json
import os
import pathlib
import socket
import subprocess
import tempfile
import time

BIN = pathlib.Path('target/debug/scriptotar-mcp')
assert BIN.is_file(), BIN
with tempfile.TemporaryDirectory() as work:
    root = pathlib.Path(work)
    policy_file = root / 'mcp-permissions.json'
    token = 'a' * 64
    other_token = 'b' * 64

    def policy(enabled=True, read_data=False, bearer=token):
        payload = {'enabled': enabled, 'readData': read_data, 'httpToken': bearer}
        temp = root / 'policy-tmp'
        fd = os.open(temp, os.O_CREAT | os.O_WRONLY | os.O_TRUNC, 0o600)
        with os.fdopen(fd, 'w') as f:
            json.dump(payload, f)
            f.flush()
            os.fsync(f.fileno())
        os.replace(temp, policy_file)

    def request(port, *, method='POST', path='/mcp', auth=token, origin=None, body=None,
                content_type='application/json', accept='application/json, text/event-stream'):
        conn = http.client.HTTPConnection('127.0.0.1', port, timeout=8)
        headers = {'Accept': accept, 'Content-Type': content_type}
        if auth is not None:
            headers['Authorization'] = 'Bearer ' + auth
        if origin is not None:
            headers['Origin'] = origin
        if body is None and method == 'POST':
            body = json.dumps({'jsonrpc':'2.0','id':1,'method':'tools/list'})
        try:
            conn.request(method, path, body=body, headers=headers)
            result = conn.getresponse()
            raw = result.read()
            return result.status, json.loads(raw) if result.getheader('Content-Type','').startswith('application/json') else raw
        finally:
            conn.close()

    policy()
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env = {**os.environ, 'SCRIPTOTAR_DB_PATH': str(root / 'scriptotar.sqlite3'),
           'SCRIPTOTAR_MCP_PORT': str(port)}
    with subprocess.Popen([str(BIN), '--mcp-http'], stdin=subprocess.DEVNULL,
                          stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, env=env) as proc:
        for _ in range(100):
            if proc.poll() is not None:
                raise RuntimeError(f'HTTP process exited: {proc.stderr.read().decode()}')
            try:
                with socket.create_connection(('127.0.0.1', port), timeout=0.1):
                    break
            except OSError:
                time.sleep(0.05)
        else:
            raise AssertionError('MCP HTTP gateway did not start')

        assert request(port, auth=None)[0] == 401
        assert request(port, auth='wrong')[0] == 401
        assert request(port, origin='https://evil.example')[0] == 403
        assert request(port, path='/not-mcp')[0] == 404
        assert request(port, method='GET')[0] == 405
        assert request(port, content_type='text/plain')[0] == 415
        assert request(port, accept='text/plain')[0] == 406
        status, result = request(port)
        assert status == 200, result
        assert {item['name'] for item in result['result']['tools']} >= {'execute_command', 'download_video', 'search_creator'}
        status, result = request(port, body=json.dumps({'jsonrpc':'2.0','id':2,'method':'tools/call',
                                                         'params':{'name':'list_projects','arguments':{}}}))
        assert status == 200 and result['result']['isError'] is True
        policy(read_data=True)
        status, result = request(port, body=json.dumps({'jsonrpc':'2.0','id':3,'method':'tools/call',
                                                         'params':{'name':'list_projects','arguments':{}}}))
        assert status == 200 and not result['result'].get('isError', False), result
        assert json.loads(result['result']['content'][0]['text']) == []
        policy(read_data=True, bearer=other_token)
        assert request(port, auth=token)[0] == 401
        assert request(port, auth=other_token)[0] == 200
        policy(enabled=False, read_data=True, bearer=other_token)
        assert request(port, auth=other_token)[0] == 403
        assert proc.poll() is None
        proc.terminate()
        proc.wait(timeout=5)
print('PASS: localhost HTTP auth, origin, content limits, dynamic policy and token revocation')
