#!/usr/bin/env python3
"""Test difficulty zone functionality - simple version"""

import socket
import time
import re

def clean_ansi(text):
    ansi_escape = re.compile(r'\x1b\[[0-9;]*[a-zA-Z]|\[0m|\[\d+m|\[\d+;\d+m')
    return ansi_escape.sub('', text)

def recv_all(sock, timeout=2.0):
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

print("Connecting to server...")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 9999))
s.settimeout(2)

# Wait for initial prompt
print("Waiting for initial prompt...")
data = recv_all(s, timeout=3)
print(f"Initial: {len(data)} bytes")

# Send username
print("Sending username...")
s.sendall("난이도테스터\n".encode('utf-8'))
time.sleep(1)
data = recv_all(s, timeout=2)
print(f"After username: {clean_ansi(data)[:300]}")

# Send password (if needed)
print("Sending password...")
s.sendall("난이도테스터\n".encode('utf-8'))
time.sleep(2)
data = recv_all(s, timeout=3)
print(f"After password: {clean_ansi(data)[:500]}")

# Test 봐 command
print("\n--- Testing '봐' command ---")
s.sendall("봐\n".encode('utf-8'))
time.sleep(1)
data = recv_all(s, timeout=2)
clean = clean_ansi(data)
print(f"Result:\n{clean[:800]}")

# Test 이동 command to go to room 4000 (곤륜선인 location)
print("\n--- Testing '이동 낙양성 4000' command ---")
s.sendall("이동 낙양성 4000\n".encode('utf-8'))
time.sleep(1)
data = recv_all(s, timeout=2)
clean = clean_ansi(data)
print(f"Result:\n{clean[:500]}")

# Look around after moving
print("\n--- Testing '봐' command after move ---")
s.sendall("봐\n".encode('utf-8'))
time.sleep(1)
data = recv_all(s, timeout=2)
clean = clean_ansi(data)
print(f"Result:\n{clean[:800]}")

# Test 대화 command with 곤륜선인
print("\n--- Testing '곤륜선인 대화 난이도1' command ---")
s.sendall("곤륜선인 대화 난이도1\n".encode('utf-8'))
time.sleep(2)
data = recv_all(s, timeout=3)
clean = clean_ansi(data)
print(f"Result:\n{clean[:800]}")

# Check position after zone change
print("\n--- Testing '봐' command after zone change ---")
s.sendall("봐\n".encode('utf-8'))
time.sleep(1)
data = recv_all(s, timeout=2)
clean = clean_ansi(data)
print(f"Result:\n{clean[:800]}")

# Test 점수 to check difficulty
print("\n--- Testing '점수' command ---")
s.sendall("점수\n".encode('utf-8'))
time.sleep(1)
data = recv_all(s, timeout=2)
clean = clean_ansi(data)
print(f"Result:\n{clean[:600]}")

s.close()
print("\nTest completed!")
