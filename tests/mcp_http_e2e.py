"""Black-box localhost HTTP MCP transport and authorization certification."""
import http.client
from contextlib import contextmanager
import json
import os
import pathlib
import socket
import subprocess
import tempfile
import time

BIN = pathlib.Path(os.environ.get('SCRIPTOTAR_MCP_BIN', 'target/debug/scriptotar-mcp'))
assert BIN.is_file(), BIN


@contextmanager
def managed_gateway(binary, env):
    """Always reap a long-running MCP child, even when an assertion fails.

    Popen's own context manager waits forever on a still-running child when
    the test body raises. A temporary stderr file also prevents PIPE backpressure
    from deadlocking the gateway before HTTP requests can be answered.
    """
    with tempfile.TemporaryFile() as gateway_stderr:
        proc = subprocess.Popen(
            [str(binary), '--mcp-http'],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=gateway_stderr, env=env,
        )
        try:
            yield proc
        finally:
            if proc.poll() is None:
                proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=5)


def checkpoint(label):
    print(f'MCP HTTP E2E: {label}', flush=True)
with tempfile.TemporaryDirectory() as work:
    root = pathlib.Path(work)
    policy_file = root / 'mcp-permissions.json'
    token = 'a' * 64
    other_token = 'b' * 64

    def policy(enabled=True, read_data=False, bearer=token, path_token='c' * 64):
        payload = {'enabled': enabled, 'readData': read_data, 'httpToken': bearer, 'urlToken': path_token}
        temp = root / 'policy-tmp'
        fd = os.open(temp, os.O_CREAT | os.O_WRONLY | os.O_TRUNC, 0o600)
        with os.fdopen(fd, 'w') as f:
            json.dump(payload, f)
            f.flush()
            os.fsync(f.fileno())
        os.replace(temp, policy_file)

    def request(port, *, method='POST', path='/mcp', auth=token, origin=None, body=None,
                content_type='application/json', accept='application/json, text/event-stream',
                declared_content_length=None):
        conn = http.client.HTTPConnection('127.0.0.1', port, timeout=8)
        headers = {'Accept': accept, 'Content-Type': content_type}
        if auth is not None:
            headers['Authorization'] = 'Bearer ' + auth
        if origin is not None:
            headers['Origin'] = origin
        if declared_content_length is not None:
            headers['Content-Length'] = str(declared_content_length)
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
    checkpoint('launching installed/local gateway')
    with managed_gateway(BIN, env) as proc:
        for _ in range(100):
            if proc.poll() is not None:
                raise RuntimeError(f'HTTP process exited before readiness (status {proc.returncode})')
            try:
                with socket.create_connection(('127.0.0.1', port), timeout=0.1):
                    break
            except OSError:
                time.sleep(0.05)
        else:
            raise AssertionError('MCP HTTP gateway did not start')

        checkpoint('gateway listening; validating authentication')
        assert request(port, auth=None)[0] == 401
        assert request(port, path='/mcp/p/' + 'wrong' * 13, auth=None)[0] == 401
        assert request(port, path='/mcp/p/' + 'c' * 64, auth=None)[0] == 200
        assert request(port, path='/mcp/p/' + 'c' * 64, auth=None, origin='https://chatgpt.com')[0] == 200
        assert request(port, path='/mcp/p/' + 'c' * 64, auth=None, origin='https://evil.example')[0] == 403
        assert request(port, auth='wrong')[0] == 401
        assert request(port, origin='https://evil.example')[0] == 403
        assert request(port, path='/not-mcp')[0] == 404
        assert request(port, method='GET')[0] == 405
        assert request(port, content_type='text/plain')[0] == 415
        assert request(port, content_type='application/jsonp')[0] == 415
        # The server rejects an oversized declared Content-Length before reading
        # any body bytes. Sending a megabyte after that early rejection can cause
        # BrokenPipeError on Linux even when HTTP 413 is correctly enforced.
        assert request(port, body=b'', declared_content_length=1024 * 1024 + 50)[0] == 413
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
        status, result = request(port, path='/mcp/p/' + 'c' * 64, auth=None,
                                 body=json.dumps({'jsonrpc':'2.0','id':4,'method':'tools/call',
                                                  'params':{'name':'list_projects','arguments':{}}}))
        assert status == 200 and not result['result'].get('isError', False), result
        assert json.loads(result['result']['content'][0]['text']) == []
        # Authenticate before the complete body arrives, rotate BOTH credentials,
        # then finish the old request. It must not dispatch on stale credentials.
        checkpoint('testing credential rotation during in-flight request')
        payload = json.dumps({'jsonrpc':'2.0','id':87,'method':'tools/call',
                              'params':{'name':'list_projects','arguments':{}}})
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=8)
        try:
            connection.putrequest('POST', '/mcp')
            connection.putheader('Authorization', 'Bearer ' + token)
            connection.putheader('Accept', 'application/json')
            connection.putheader('Content-Type', 'application/json')
            connection.putheader('Content-Length', str(len(payload)))
            connection.endheaders()
            connection.send(payload[:12].encode())
            time.sleep(0.1)
            policy(read_data=True, bearer=other_token, path_token='d' * 64)
            connection.send(payload[12:].encode())
            assert connection.getresponse().status == 401
        finally:
            connection.close()
        assert request(port, path='/mcp/p/' + 'c' * 64, auth=None)[0] == 401
        assert request(port, path='/mcp/p/' + 'd' * 64, auth=None)[0] == 200
        assert request(port, auth=token)[0] == 401
        assert request(port, auth=other_token)[0] == 200
        policy(enabled=False, read_data=True, bearer=other_token)
        assert request(port, auth=other_token)[0] == 403
        assert request(port, path='/mcp/p/' + 'd' * 64, auth=None)[0] == 403
        assert proc.poll() is None
        checkpoint('revocation verified; cleaning up gateway')
print('PASS: localhost HTTP auth, origin, content limits, dynamic policy and token revocation')
