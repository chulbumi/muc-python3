#!/usr/bin/env python3
"""Test using existing character with correct password"""

import socket
import time
import re

def clean_ansi(text):
    ansi_escape = re.compile(r'\x1b\[[0-9;]*[a-zA-Z]|\[0m|\[\d+m|\[\d+;\d+m')
    return ansi_escape.sub('', text)

def recv_with_timeout(sock, timeout=3.0):
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

print("Connecting to localhost:9999...")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 9999))

# Get initial banner
print("\n=== Waiting for initial banner ===")
data = recv_with_timeout(s)
print(f"Received {len(data)} bytes")

# Send username
print("\n=== Sending username '비교테스터' ===")
s.sendall("비교테스터\n".encode('utf-8'))
time.sleep(1)
data = recv_with_timeout(s)
print(f"After username: {clean_ansi(data)[:200]}")

# Send correct password
print("\n=== Sending password 'test1234' ===")
s.sendall("test1234\n".encode('utf-8'))
time.sleep(2)
data = recv_with_timeout(s)
text = clean_ansi(data)
print(f"After password ({len(text)} chars):")
print(text[:800])

# If login successful, try commands
if len(text) > 0:
    print("\n=== Sending '봐' command ===")
    s.sendall("봐\n".encode('utf-8'))
    time.sleep(1)
    data = recv_with_timeout(s)
    print(clean_ansi(data)[:800])

    print("\n=== Sending '이동 낙양성 4000' command ===")
    s.sendall("이동 낙양성 4000\n".encode('utf-8'))
    time.sleep(1)
    data = recv_with_timeout(s)
    print(clean_ansi(data)[:500])

    print("\n=== Sending '봐' command at room 4000 ===")
    s.sendall("봐\n".encode('utf-8'))
    time.sleep(1)
    data = recv_with_timeout(s)
    print(clean_ansi(data)[:800])

    print("\n=== Sending '곤륜선인 대화 난이도1' command ===")
    s.sendall("곤륜선인 대화 난이도1\n".encode('utf-8'))
    time.sleep(2)
    data = recv_with_timeout(s)
    print(clean_ansi(data)[:800])

    print("\n=== Sending '봐' command after zone change ===")
    s.sendall("봐\n".encode('utf-8'))
    time.sleep(1)
    data = recv_with_timeout(s)
    print(clean_ansi(data)[:800])

    print("\n=== Sending '점수' command ===")
    s.sendall("점수\n".encode('utf-8'))
    time.sleep(1)
    data = recv_with_timeout(s)
    print(clean_ansi(data)[:600])

s.close()
print("\nDone!")
