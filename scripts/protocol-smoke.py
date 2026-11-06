#!/usr/bin/env python3
"""Real process + narrow fake host tests. Never accesses the desktop or host profile."""
import argparse
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import uuid

ROOT = Path(__file__).resolve().parents[1]
MAX_FRAME = 16 * 1024 * 1024
METHODS = {
    'host.selection.read': ('core:platform:selected-text', 'selection', ''),
    'host.clipboard.read': ('clipboard:read', 'clipboard', ''),
    'host.clipboard.pasteText': ('core:platform:paste-text', 'paste', {'ok': True, 'message': 'Pasted text'}),
}


class Client:
    def __init__(self, binary, manifest, directory, grants=None, package_dir=None):
        self.manifest = manifest
        # Run with no Node/Bun/Cargo or sibling repo on PATH. All paths are private.
        env = {k: v for k, v in os.environ.items() if k in ('SystemRoot', 'WINDIR', 'TEMP', 'TMP')}
        env['PATH'] = os.path.join(env.get('SystemRoot', 'C:/Windows'), 'System32') if os.name == 'nt' else '/usr/bin:/bin'
        package_dir = package_dir or ROOT
        self.p = subprocess.Popen([str(binary.resolve())], cwd=package_dir, env=env,
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.messages = queue.Queue()
        self.calls = []
        self.events = []
        self.config = {}
        def reader():
            try:
                while True:
                    line = self.p.stdout.readline(MAX_FRAME + 2)
                    if not line:
                        self.messages.put(EOFError('Unexpected protocol EOF'))
                        return
                    assert len(line.rstrip(b'\n')) <= MAX_FRAME and line.endswith(b'\n')
                    self.messages.put(json.loads(line))
            except Exception as error:
                self.messages.put(error)
        threading.Thread(target=reader, daemon=True).start()
        hello = self.receive()
        assert hello['type'] == 'hello' and hello['protocol'] == 1
        assert hello['id'] == manifest['id'] and hello['version'] == manifest['version']
        assert hello['sdkVersion'] == '0.1.1'
        init = self.request('initialize', {
            'protocol': 1, 'extensionId': manifest['id'],
            'platform': 'Windows' if os.name == 'nt' else 'macOS',
            'packageDir': str(package_dir), 'dataDir': str(directory / 'data'),
            'tempDir': str(directory / 'temp'), 'migrationPaths': {}, 'reason': 'command',
            'grantedCapabilities': manifest['capabilities'] if grants is None else grants,
        })
        assert init['ready'] is True

    def send(self, frame):
        data = json.dumps(frame, ensure_ascii=False, separators=(',', ':')).encode() + b'\n'
        assert len(data) - 1 <= MAX_FRAME
        self.p.stdin.write(data)
        self.p.stdin.flush()

    def receive(self):
        value = self.messages.get(timeout=15)
        if isinstance(value, Exception):
            raise value
        return value

    def begin(self, method, params):
        ident = 'h:' + str(uuid.uuid4())
        self.send({'type': 'request', 'id': ident, 'method': method, 'params': params})
        return ident

    def respond(self, frame):
        assert frame['type'] == 'request' and frame['method'] in METHODS, frame
        assert frame['id'].startswith('e:')
        method = frame['method']
        channel, key, default = METHODS[method]
        params = frame['params']
        if key == 'paste':
            assert set(params) == {'text'} and isinstance(params['text'], str)
            args = [params['text']]
        else:
            assert params == {}, frame
            args = []
        self.calls.append([channel, args])
        spec = self.config.get(key, {})
        reply = {'type': 'response', 'id': frame['id']}
        if 'error' in spec:
            reply['error'] = {'code': 'stub_error', 'message': 'Recording host transport failure'}
        else:
            reply['result'] = spec.get('value', default)
        self.send(reply)

    def finish(self, ident, error=False):
        while True:
            frame = self.receive()
            if frame['type'] == 'request':
                self.respond(frame)
            elif frame['type'] == 'event':
                self.events.append(frame)
            elif frame['type'] == 'cancel':
                self.events.append(frame)
            else:
                assert frame['type'] == 'response' and frame['id'] == ident, frame
                assert ('error' in frame) == error, frame
                return frame.get('error') if error else frame['result']

    def request(self, method, params, error=False):
        return self.finish(self.begin(method, params), error)

    def open(self, session, payload=None):
        params = {'sessionId': session, 'commandName': 'index', 'argumentValues': {}, 'preferences': {}}
        params.update(payload or {})
        return self.request('session.open', params)

    def call(self, session, operation, payload=None, error=False):
        return self.request('session.call', {'sessionId': session, 'operation': operation, 'payload': payload or {}}, error)

    def dispose(self, session):
        self.request('session.dispose', {'sessionId': session})

    def close(self):
        if self.p.poll() is None:
            self.p.kill()
        self.p.wait(timeout=5)
        self.p.stdin.close()
        self.p.stdout.close()
        self.p.stderr.close()


def exercise(binary, manifest, oracle=True, limits=True, package_dir=None):
    info = json.loads(subprocess.check_output([str(binary.resolve()), '--protocol-info'], timeout=10))
    assert info['id'] == manifest['id'] and info['version'] == manifest['version']
    assert info['protocol'] == 1 and info['commands'] == [c['name'] for c in manifest['commands']]
    with tempfile.TemporaryDirectory(prefix='markdown-process-') as temporary:
        client = Client(binary, manifest, Path(temporary), package_dir=package_dir)
        try:
            cases = json.loads((ROOT / 'v2/tests/fixtures/oracle.json').read_text()) if oracle else []
            cases += [
                {'name': 'literal multiline', 'payload': {'argumentValues': {'markdownImage': 'before\n![a](u)\nafter'}},
                 'expected': {'status': 'ok', 'calls': [['core:platform:paste-text', ['before\n<img width="300" alt="a" src="u"/>\nafter']]]}},
                {'name': 'actual paste reply', 'payload': {'argumentValues': {'markdownImage': '![a](u)'}},
                 'paste': {'value': {'ok': True, 'message': 'Pasted text'}},
                 'expected': {'status': 'ok', 'calls': [['core:platform:paste-text', ['<img width="300" alt="a" src="u"/>']]]}},
            ]
            for number, case in enumerate(cases):
                client.config = case
                client.calls = []
                session = 'case-' + str(number)
                result = client.open(session, case.get('payload'))
                assert result['mode'] == 'no-view' and isinstance(result['message'], str), case['name']
                assert result['ok'] == (case['expected']['status'] == 'ok'), (case['name'], result)
                assert client.calls == case['expected']['calls'], case['name']
                assert result['message']
                calls = list(client.calls)
                assert client.call(session, 'extension:refresh-session') == {'ok': True, 'mode': 'unchanged'}
                # Actual renderer action payload shapes cannot cause paste/retry.
                client.call(session, 'extension:invoke-action', {'actionId': 'paste', 'formValues': {'markdownImage': '![forged](u)'}}, error=True)
                client.call(session, 'extension:run-command', {}, error=True)
                assert client.calls == calls, case['name']
                client.dispose(session)  # Required even when result.ok is false.
            # Separate malformed-envelope probes from the real host reply tests.
            for number, value in enumerate([None, True, {}, {'ok': False}, {'ok': True, 'error': 'failure'}, {'ok': True, 'success': False}]):
                client.config = {'paste': {'value': value}}
                client.calls = []
                session = 'bad-ack-' + str(number)
                result = client.open(session, {'argumentValues': {'markdownImage': '![a](u)'}})
                assert result['ok'] is False and 'may already have occurred' in result['message']
                assert len(client.calls) == 1
                client.call(session, 'extension:invoke-action', {'actionId': 'retry'}, error=True)
                client.dispose(session)
                assert len(client.calls) == 1
            if limits:
                # Encoded payload stays below 16 MiB while source exceeds its 4 MiB bound.
                client.config = {}
                client.calls = []
                result = client.open('oversized-input', {'argumentValues': {'markdownImage': 'x' * (4 * 1024 * 1024 + 1)}})
                assert result['ok'] is False and '4 MiB' in result['message'] and not client.calls
                client.dispose('oversized-input')
                # One legal 4 MiB preference expands to a >16 MiB JSON-escaped reverse
                # request. SDK frame budget rejects it before the fake host sees paste.
                result = client.open('escaped-frame', {'argumentValues': {'markdownImage': '![a](u) ![b](v)'},
                                                       'preferences': {'imageWidth': '\\' * (4 * 1024 * 1024)}})
                assert result['ok'] is False and not client.calls
                client.dispose('escaped-frame')
            # Unknown commands and operations are protocol rejections, not domain errors.
            client.request('session.open', {'sessionId': 'unknown', 'commandName': 'other'}, error=True)
            # Cancellation while waiting on a read propagates reverse cancel and never pastes.
            client.calls = []
            opening = client.begin('session.open', {'sessionId': 'cancelled', 'commandName': 'index', 'argumentValues': {}, 'preferences': {}})
            reverse = client.receive()
            assert reverse['type'] == 'request' and reverse['method'] == 'host.selection.read'
            client.send({'type': 'cancel', 'requestId': opening, 'reason': 'test cancellation'})
            client.finish(opening, error=True)
            assert not client.calls
            # Cancelled opens are removed by the SDK. Actual host may dispose idempotently
            # at its own layer; a direct redundant SDK dispose correctly rejects unknown ID.
            client.request('session.dispose', {'sessionId': 'cancelled'}, error=True)
            client.request('shutdown', {})
            assert client.p.wait(timeout=5) == 0
            assert not client.p.stderr.read(), 'Unexpected stderr'
        finally:
            client.close()
        # An ungranted paste never reaches the fake host and never reports success.
        denied = Client(binary, manifest, Path(temporary), grants=[], package_dir=package_dir)
        try:
            result = denied.open('denied', {'argumentValues': {'markdownImage': '![a](u)'}})
            assert result['ok'] is False and not denied.calls
            denied.dispose('denied')
            denied.request('shutdown', {})
            assert denied.p.wait(timeout=5) == 0
        finally:
            denied.close()
    print(f'Protocol lifecycle + {len(cases)} semantic cases + malformed acknowledgements + cancellation + grant denial passed; desktop untouched')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    args = parser.parse_args()
    exercise(args.binary, json.loads((ROOT / 'extension.json').read_text()))
