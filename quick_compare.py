#!/usr/bin/env python3
"""Quick comparison test - just 20 key commands"""

import socket
import time
import re

def clean_ansi(text):
    ansi_escape = re.compile(r'\x1b\[[0-9;]*[a-zA-Z]|\[0m|\[\d+m|\[\d+;\d+m')
    return ansi_escape.sub('', text)

def recv_all(sock, timeout=1.5):
    sock.settimeout(timeout)
    data = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
    except socket.timeout:
        pass
    return data.decode('utf-8', errors='ignore')

def connect(port):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', port))
    time.sleep(0.5)
    recv_all(s, timeout=2)
    s.sendall("비교테스터\n".encode('utf-8'))
    time.sleep(0.3)
    recv_all(s, timeout=1)
    s.sendall("비교테스터\n".encode('utf-8'))
    time.sleep(1)
    recv_all(s, timeout=2)
    return s

def send_cmd(sock, cmd):
    sock.sendall((cmd + "\n").encode('utf-8'))
    time.sleep(1)
    return clean_ansi(recv_all(sock)).strip()

# Test commands
TEST_CMDS = [
    "help", "look", "inventory", "점수", "소지품", "장비",
    "도움말", "봐", "누구", "어디", "무공", "숙련도",
    "기연리스트", "방파리스트", "저장", "회복", "안시",
    "줄임말", "호위", "분노"
]

print("Quick Comparison Test")
print("=" * 50)

# Connect
print("Connecting to Python (9903)...")
py = connect(9903)
print("Connecting to Rust (9999)...")
rs = connect(9999)

print("\nTesting commands...\n")

matches = 0
diffs = 0
results = []

for cmd in TEST_CMDS:
    py_out = send_cmd(py, cmd)[:100]
    rs_out = send_cmd(rs, cmd)[:100]

    # Normalize for comparison (remove HP/MP)
    py_clean = re.sub(r'\[\s*\d+/\d+,\s*\d+/\d+\s*\]', '', py_out)
    rs_clean = re.sub(r'\[\s*\d+/\d+,\s*\d+/\d+\s*\]', '', rs_out)
    py_clean = re.sub(r'\s+', ' ', py_clean).strip()
    rs_clean = re.sub(r'\s+', ' ', rs_clean).strip()

    if py_clean == rs_clean:
        matches += 1
        status = "✓"
    else:
        diffs += 1
        status = "✗"

    results.append((cmd, status, py_clean[:50], rs_clean[:50]))
    print(f"{status} {cmd}")

print("\n" + "=" * 50)
print(f"Results: {matches}/{len(TEST_CMDS)} matched ({matches/len(TEST_CMDS)*100:.0f}%)")

if diffs > 0:
    print("\nDifferences:")
    for cmd, st, py, rs in results:
        if st == "✗":
            print(f"\n  {cmd}:")
            print(f"    PY: {py}")
            print(f"    RS: {rs}")

py.close()
rs.close()
