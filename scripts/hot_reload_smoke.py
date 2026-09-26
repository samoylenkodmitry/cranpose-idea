#!/usr/bin/env python3
"""Exercise compiled hot patches through a real Cranpose GPU preview connection."""
import argparse
import json
import os
from pathlib import Path
import queue
import secrets
import signal
import socket
import struct
import subprocess
import threading
import time


def string(value):
    data = value.encode()
    return struct.pack('<I', len(data)) + data


class Host:
    def __init__(self, connection, token):
        self.connection = connection
        self.lock = threading.Lock()
        self.messages = queue.Queue()
        self.frames = 0
        hello = self.read_packet()
        assert hello[0] == 1 and struct.unpack_from('<I', hello, 1)[0] == 2
        assert hello[9:].decode() == token
        threading.Thread(target=self.read, daemon=True).start()
        self.send(1, struct.pack('<IIIff', 0, 320, 240, 1.0, 60.0))
        self.send(13, struct.pack('<IB', 0, 1))

    def read_exact(self, length):
        data = bytearray()
        while len(data) < length:
            piece = self.connection.recv(length - len(data))
            if not piece:
                raise EOFError('preview connection closed')
            data.extend(piece)
        return data

    def read_packet(self):
        length, = struct.unpack('<I', self.read_exact(4))
        if not 0 < length <= 256 * 1024 * 1024:
            raise ValueError('invalid frame length')
        return self.read_exact(length)

    def read(self):
        try:
            while True:
                packet = self.read_packet()
                if packet[0] == 2:
                    self.frames += 1
                    self.send(11, packet[1:9])
                elif packet[0] == 4:
                    length, = struct.unpack_from('<I', packet, 1)
                    channel = packet[5:5+length].decode()
                    payload = packet[9+length:].decode()
                    self.messages.put((channel, json.loads(payload)))
        except (OSError, EOFError, ValueError) as error:
            self.messages.put(('error', str(error)))

    def send(self, kind, body=b''):
        packet = bytes([kind]) + body
        with self.lock:
            self.connection.sendall(struct.pack('<I', len(packet)) + packet)

    def message(self, channel, payload):
        self.send(10, string(channel) + string(json.dumps(payload)))

    def click(self, x, y):
        body = struct.pack('<Iff', 0, x, y)
        self.send(2, body)
        self.send(3, body)
        self.send(4, body)

    def snapshot_with(self, expected, deadline):
        next_request = 0
        while time.monotonic() < deadline:
            if time.monotonic() >= next_request:
                self.message('cranpose.inspector.v2.request', {'requestId': int(time.monotonic() * 1000)})
                next_request = time.monotonic() + 0.2
            try:
                channel, payload = self.messages.get(timeout=0.2)
            except queue.Empty:
                continue
            if channel == 'error':
                raise AssertionError(payload)
            if channel == 'cranpose.dev.applied':
                print(json.dumps({'runtime': payload}), flush=True)
                self.runtime = payload
            if channel == 'cranpose.inspector.v2.snapshot':
                texts = [node.get('text') for node in payload['nodes']]
                if all(value in texts for value in expected) and hasattr(self, 'runtime'):
                    return payload
        raise AssertionError(f'preview never displayed {expected}; frames={self.frames}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--dx', type=Path)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--log', type=Path, required=True)
    parser.add_argument('--runner', type=Path)
    parser.add_argument('--cache', type=Path)
    parser.add_argument('--source', default='src/main.rs')
    args = parser.parse_args()
    source = args.workspace / args.source
    original = source.read_text()
    token = secrets.token_hex(24)
    with socket.socket() as server, args.log.open('w') as log:
        server.bind(('127.0.0.1', 0))
        server.listen(1)
        server.settimeout(1)
        environment = dict(os.environ, CRANPOSE_EMBED_ADDRESS=f'127.0.0.1:{server.getsockname()[1]}', CRANPOSE_EMBED_TOKEN=token)
        command = [str(args.dx.resolve()), 'serve', '--hot-patch', '--platform', 'desktop', '--renderer', 'native', '--interactive', 'false', '--open', 'false', '--json-output', '--raw-json-diagnostics'] if args.dx else []
        if args.runner:
            if args.dx:
                environment['CRANPOSE_DX'] = str(args.dx.resolve())
            options = {'root': str(args.workspace.resolve()), 'cache': str(args.cache.resolve()), 'package': 'cranpose-hot-counter', 'target': 'cranpose-hot-counter', 'kind': 'bin', 'hotReload': True}
            command = [str(args.runner.resolve()), json.dumps(options)]
        if not command:
            parser.error('--runner or --dx is required')
        process = subprocess.Popen(command, cwd=args.workspace, env=environment, stdout=log, stderr=log, start_new_session=True)
        try:
            deadline = time.monotonic() + 900
            while True:
                try:
                    connection, _ = server.accept()
                    break
                except socket.timeout:
                    if process.poll() is not None:
                        raise AssertionError(f'compiler exited {process.returncode}; see {args.log}')
                    if time.monotonic() > deadline:
                        raise AssertionError(f'preview did not connect; see {args.log}')
            connection.settimeout(None)
            host = Host(connection, token)
            snapshot = host.snapshot_with(['Count: 0', 'Increment'], time.monotonic() + 30)
            initial_pid = host.runtime['pid']
            label = next(node for node in snapshot['nodes'] if node.get('text') == 'Increment')
            x, y = label['x'] + label['width'] / 2, label['y'] + label['height'] / 2
            for expected in range(1, 4):
                host.click(x, y)
                host.snapshot_with([f'Count: {expected}'], time.monotonic() + 10)
            source.write_text(original.replace('Increment', 'Add two!!').replace('count.get() + 1', 'count.get() + 2'))
            host.snapshot_with(['Count: 3', 'Add two!!'], time.monotonic() + 120)
            assert host.runtime['pid'] == initial_pid, 'application restarted'
            assert host.runtime['generation'] > 0, 'no patch acknowledgement'
            host.click(x, y)
            host.snapshot_with(['Count: 5', 'Add two!!'], time.monotonic() + 10)
            if args.runner:
                patched = source.read_text()
                def wait_log(marker, offset):
                    deadline = time.monotonic() + 90
                    while time.monotonic() < deadline:
                        if marker in args.log.read_text()[offset:]:
                            return
                        if process.poll() is not None:
                            raise AssertionError('compiler stopped during recovery test')
                        time.sleep(0.1)
                    raise AssertionError(f'missing compiler event: {marker}')
                offset = args.log.stat().st_size
                source.write_text(patched.replace('count.get() + 2', 'count.get() + 999999999999999999999'))
                wait_log('literal out of range', offset)
                host.snapshot_with(['Count: 5', 'Add two!!'], time.monotonic() + 10)
                offset = args.log.stat().st_size
                source.write_text(patched + '\nfn incompatible( {')
                wait_log('restartRequired', offset)
                host.click(x, y)
                host.snapshot_with(['Count: 7', 'Add two!!'], time.monotonic() + 10)
                offset = args.log.stat().st_size
                source.write_text(patched.replace('0_i32', '0_i64'))
                wait_log('restartRequired', offset)
                host.snapshot_with(['Count: 7', 'Add two!!'], time.monotonic() + 10)
                source.write_text(original)
                host.snapshot_with(['Count: 7', 'Increment'], time.monotonic() + 120)
                host.click(x, y)
                host.snapshot_with(['Count: 8', 'Increment'], time.monotonic() + 10)
                assert host.runtime['pid'] == initial_pid, 'recovery restarted the application'
                print(json.dumps({'recovery': 'compiler error, invalid syntax, and state-type change kept the previous preview; reverting resumed hot reload', 'state': 8}), flush=True)
            print(json.dumps({'result': 'passed', 'pid': initial_pid, 'frames': host.frames, 'state': '3 preserved; updated handler produced 5', 'connection': 'unchanged'}), flush=True)
        finally:
            source.write_text(original)
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()


if __name__ == '__main__':
    main()
