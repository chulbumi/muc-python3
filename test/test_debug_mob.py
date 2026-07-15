#!/usr/bin/env python3
"""Test mob spawning with debug output"""

import socket
import select
import time
import re

def clean_ansi(text):
    ansi_escape = re.compile(r'\x1b\[[0-9;]*[a-zA-Z]|\[0m|\[\d+m|\[\d+;\d+m')
    return ansi_escape.sub('', text)

def recv_nonblock(sock, timeout=3.0):
    data = b""
    end_time = time.time() + timeout
    while time.time() < end_time:
        ready, _, _ = select.select([sock], [], [], 0.5)
        if ready:
            try:
                chunk = sock.recv(4096)
                if not chunk:
                    break
                data += chunk
            except:
                break
        else:
            if data:
                break
    return data

def send_cmd(sock, cmd):
    sock.setblocking(True)
    sock.sendall((cmd + "\r\n").encode('utf-8'))
    sock.setblocking(False)
    time.sleep(1.0)
    data = recv_nonblock(sock, timeout=2)
    return clean_ansi(data.decode('utf-8', errors='replace'))

print("=" * 60)
print("Mob Spawn Debug Test")
print("=" * 60)

# Connect and login
print("\n1. Connecting and logging in...")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setblocking(False)
try:
    s.connect(('localhost', 9999))
except BlockingIOError:
    pass
_, writable, _ = select.select([], [s], [], 5.0)
if not writable:
    print("Connection failed!")
    exit(1)

time.sleep(1)
recv_nonblock(s, timeout=2)
s.setblocking(True)
s.sendall("비교테스터\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(0.5)
recv_nonblock(s, timeout=1)
s.setblocking(True)
s.sendall("test1234\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(2)
recv_nonblock(s, timeout=3)
s.setblocking(True)
s.sendall("\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(1)
recv_nonblock(s, timeout=2)
print("   Logged in!")

# Move to room with mobs (room 42 has mobs)
print("\n2. Moving to room 42...")
result = send_cmd(s, "이동 낙양성 42")
print(result[:400])

# Look for mobs
print("\n3. Looking for mobs...")
result = send_cmd(s, "봐")
print(result)

# Try to spawn a mob manually using 몹생성 command (admin)
print("\n4. Trying '몹생성 밍밍' (admin command)...")
result = send_cmd(s, "몹생성 밍밍")
print(result[:500])

# Look again
print("\n5. Looking again after spawn attempt...")
result = send_cmd(s, "봐")
print(result)

# Move to another room and check
print("\n6. Moving to room 3001 (has mobs)...")
result = send_cmd(s, "이동 낙양성 3001")
print(result[:400])

# Look for mobs
print("\n7. Looking for mobs in room 3001...")
result = send_cmd(s, "봐")
print(result)

s.close()
print("\n" + "=" * 60)
print("Test completed! Check server logs for mob spawn messages.")
print("=" * 60)
