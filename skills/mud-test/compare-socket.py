#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Socket-based direct comparison between Python and Rust MUD servers
Both servers use raw socket connection (no telnetlib)
"""

import socket
import time
import sys
from typing import Dict, List, Tuple

class SocketMUDClient:
    def __init__(self, host: str, port: int):
        self.host = host
        self.port = port
        self.sock = None

    def connect(self) -> bool:
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.host, self.port))
            time.sleep(0.3)
            return True
        except Exception as e:
            print(f"Connection failed to {self.host}:{self.port} - {e}")
            return False

    def send(self, command: str):
        if self.sock:
            self.sock.sendall((command + "\r\n").encode('utf-8'))

    def recv_all(self, timeout: float = 0.5) -> str:
        if not self.sock:
            return ""

        time.sleep(timeout)
        data = b""
        self.sock.settimeout(0.15)

        try:
            while True:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                data += chunk
        except socket.timeout:
            pass
        except:
            pass

        return data.decode('utf-8', errors='ignore')

    def recv_until_contains(self, text: str, max_tries: int = 30) -> str:
        """Receive data until it contains specific text"""
        all_data = ""
        for _ in range(max_tries):
            self.sock.settimeout(0.1)
            try:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                all_data += chunk.decode('utf-8', errors='ignore')
                if text in all_data:
                    return all_data
            except socket.timeout:
                pass
        return all_data

    def clear_buffer(self):
        self.sock.settimeout(0.1)
        try:
            while self.sock.recv(4096):
                pass
        except:
            pass

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass

def create_character_doumi(client: SocketMUDClient, name: str) -> bool:
    """Create character using DOUMI system"""
    client.clear_buffer()

    # Trigger DOUMI
    client.send("나만바라바")
    time.sleep(0.5)
    resp = client.recv_all(0.3)

    if '이름' in resp or '무엇' in resp:
        client.send(name)
        time.sleep(0.5)
        resp = client.recv_all(0.3)

        if '빠른도우미' in resp:
            client.send("1")
            time.sleep(0.3)

            # Press Enter through DOUMI flow
            for _ in range(25):
                resp = client.recv_all(0.1)
                if '낙양성' in resp or '입장' in resp or '하남성' in resp:
                    return True
                client.send("")
                time.sleep(0.12)

    return True

def test_commands(client: SocketMUDClient, commands: List[str]) -> Dict[str, str]:
    """Test commands and return responses"""
    results = {}

    for cmd in commands:
        client.send(cmd)
        time.sleep(0.6)
        resp = client.recv_all(0.4)
        results[cmd] = resp

    return results

def compare_servers():
    """Compare Python and Rust servers"""
    print("=" * 70)
    print("Python vs Rust MUD Server - Socket Comparison Test")
    print("=" * 70)

    # Test commands
    commands = [
        '능력치',
        '점수',
        '소지품',
        '봐',
        '저장',
        '무공',
        '누구',
        '어디',
        '도움말'
    ]

    # Test Python server
    print("\n[Python Server :9900]")
    py_client = SocketMUDClient('localhost', 9900)
    if py_client.connect():
        create_character_doumi(py_client, '파이썬비교')
        py_results = test_commands(py_client, commands)
        py_client.close()
        print(f"  Tested {len(py_results)} commands")
    else:
        print("  Failed to connect")
        py_results = {}

    # Test Rust server
    print("\n[Rust Server :9999]")
    rust_client = SocketMUDClient('localhost', 9999)
    if rust_client.connect():
        create_character_doumi(rust_client, '러스트비교')
        rust_results = test_commands(rust_client, commands)
        rust_client.close()
        print(f"  Tested {len(rust_results)} commands")
    else:
        print("  Failed to connect")
        rust_results = {}

    # Compare results
    print("\n" + "=" * 70)
    print("Comparison Results")
    print("=" * 70)

    match_count = 0
    differ_count = 0
    both_fail_count = 0

    for cmd in commands:
        print(f"\n{'─' * 70}")
        print(f"Command: {cmd}")
        print(f"{'─' * 70}")

        py_resp = py_results.get(cmd, "").replace('\r', '').strip()
        rust_resp = rust_results.get(cmd, "").replace('\r', '').strip()

        py_len = len(py_results.get(cmd, ""))
        rust_len = len(rust_results.get(cmd, ""))

        # Check for key indicators
        py_has_hp = '체력' in py_resp or 'HP' in py_resp
        rust_has_hp = '체력' in rust_resp or 'HP' in rust_resp
        py_has_gold = '은전' in py_resp or 'gold' in py_resp.lower()
        rust_has_gold = '은전' in rust_resp or 'gold' in rust_resp.lower()
        py_has_loc = '낙양성' in py_resp or '하남성' in py_resp or '위치' in py_resp
        rust_has_loc = '낙양성' in rust_resp or '하남성' in rust_resp or '위치' in rust_resp

        print(f"Python: {py_len}b | Rust: {rust_len}b")

        # Show preview
        py_preview = py_resp[:80].replace('\n', '\\n')
        rust_preview = rust_resp[:80].replace('\n', '\\n')

        if py_preview:
            print(f"  Py:  {py_preview}...")
        if rust_preview:
            print(f"  Rust: {rust_preview}...")

        # Functional comparison
        if not py_resp and not rust_resp:
            print("  ✗ Both empty")
            both_fail_count += 1
        elif py_resp == rust_resp:
            print("  ✓ EXACT MATCH")
            match_count += 1
        elif (py_has_hp == rust_has_hp and
              py_has_gold == rust_has_gold and
              py_has_loc == rust_has_loc):
            print("  ✓ Similar content (both have HP/Gold/Location)")
            match_count += 1
        else:
            print("  ✗ Different output")
            differ_count += 1

    print("\n" + "=" * 70)
    print(f"Summary: {match_count} Similar/Match, {differ_count} Different, {both_fail_count} Both Empty")
    print("=" * 70)

if __name__ == '__main__':
    compare_servers()
