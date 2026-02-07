#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Direct comparison between Python and Rust MUD servers using raw socket
"""

import socket
import time
import sys

class MUDConnection:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.sock = None

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((self.host, self.port))
        time.sleep(0.5)

    def send(self, cmd):
        self.sock.sendall((cmd + "\r\n").encode('utf-8'))

    def recv_all(self, timeout=0.5):
        time.sleep(timeout)
        data = b""
        self.sock.settimeout(0.1)
        while True:
            try:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                data += chunk
            except socket.timeout:
                break
        return data.decode('utf-8', errors='ignore')

    def close(self):
        if self.sock:
            self.sock.close()

def create_character(conn, name):
    """Create character using DOUMI"""
    conn.send("나만바라바")
    time.sleep(0.5)
    resp = conn.recv_all()

    if '이름' in resp or '무엇' in resp:
        conn.send(name)
        time.sleep(0.5)
        resp = conn.recv_all()

        if '빠른도우미' in resp:
            conn.send("1")
            time.sleep(0.3)

            for _ in range(20):
                resp = conn.recv_all()
                if '낙양성' in resp or '입장' in resp:
                    return True
                conn.send("")
                time.sleep(0.2)
    return False

def test_command(conn, cmd):
    """Test a single command and return response"""
    conn.send(cmd)
    time.sleep(0.5)
    return conn.recv_all()

def main():
    print("=" * 70)
    print("Python vs Rust MUD Server Comparison")
    print("=" * 70)

    # Test Python server
    print("\n[Python Server :9900]")
    py_conn = MUDConnection('localhost', 9900)
    py_conn.connect()
    create_character(py_conn, "파이썬테스트")
    time.sleep(0.5)

    # Test Rust server
    print("\n[Rust Server :9999]")
    rust_conn = MUDConnection('localhost', 9999)
    rust_conn.connect()
    create_character(rust_conn, "러스트테스트")
    time.sleep(0.5)

    # Compare commands
    commands = ['능력치', '점수', '소지품', '봐', '저장']

    print("\n" + "=" * 70)
    print("Command Comparison")
    print("=" * 70)

    for cmd in commands:
        print(f"\n{'='*70}")
        print(f"Command: {cmd}")
        print(f"{'='*70}")

        # Python response
        py_resp = test_command(py_conn, cmd)
        print(f"\n[Python] {len(py_resp)} bytes")
        # Show first 200 chars
        preview = py_resp.replace('\r', '').replace('\n', '\\n')[:200]
        print(f"Preview: {preview}...")

        # Rust response
        rust_resp = test_command(rust_conn, cmd)
        print(f"\n[Rust]   {len(rust_resp)} bytes")
        # Show first 200 chars
        preview = rust_resp.replace('\r', '').replace('\n', '\\n')[:200]
        print(f"Preview: {preview}...")

        # Comparison
        if py_resp.strip() == rust_resp.strip():
            print("\n✓ MATCH")
        else:
            print("\n✗ DIFFER")

    py_conn.close()
    rust_conn.close()

    print("\n" + "=" * 70)
    print("Test Complete")
    print("=" * 70)

if __name__ == '__main__':
    main()
