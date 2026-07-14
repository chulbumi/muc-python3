#!/usr/bin/env python3
"""Test difficulty zone functionality"""

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

def send_cmd(sock, cmd):
    sock.sendall((cmd + "\n").encode('utf-8'))
    time.sleep(0.8)
    return clean_ansi(recv_all(sock, timeout=1.5)).strip()

def connect():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', 9999))
    time.sleep(0.5)
    recv_all(s, timeout=2)
    s.sendall("난이도테스터\n".encode('utf-8'))
    time.sleep(0.3)
    recv_all(s, timeout=1)
    s.sendall("난이도테스터\n".encode('utf-8'))
    time.sleep(1)
    recv_all(s, timeout=2)
    return s

print("=" * 60)
print("Difficulty Zone Test")
print("=" * 60)

# Connect
print("\n1. Connecting to server...")
s = connect()
print("   Connected!")

# Check current position
print("\n2. Checking current position...")
result = send_cmd(s, "봐")
print(result[:500])

# Go to 곤륜선인 location (room 4000)
print("\n3. Moving to 곤륜선인 location (room 4000)...")
# First check if we can use a warp or goto command
result = send_cmd(s, "이동 낙양성 4000")
print(result[:500])

# Look around
print("\n4. Looking around...")
result = send_cmd(s, "봐")
print(result[:800])

# Check for 곤륜선인
print("\n5. Talking to 곤륜선인 about 난이도1...")
result = send_cmd(s, "곤륜선인 대화 난이도1")
print(result[:800])

# Check new position
print("\n6. Checking new position after zone change...")
result = send_cmd(s, "봐")
print(result[:800])

# Check mobs in the room
print("\n7. Looking at mobs in difficulty zone...")
result = send_cmd(s, "봐")
if "몹" in result or "NPC" in result or "몬스터" in result.lower():
    print("Found mob references")
print(result[:800])

# Try to attack a mob
print("\n8. Trying to attack a mob...")
result = send_cmd(s, "공격")
print(result[:500])

# Check score/stats
print("\n9. Checking player score...")
result = send_cmd(s, "점수")
print(result[:600])

s.close()
print("\n" + "=" * 60)
print("Test completed")
print("=" * 60)
