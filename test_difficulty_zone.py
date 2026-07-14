#!/usr/bin/env python3
"""Test difficulty zone functionality"""

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
    time.sleep(0.8)
    data = recv_nonblock(sock, timeout=2)
    return clean_ansi(data.decode('utf-8', errors='replace'))

print("=" * 60)
print("Difficulty Zone Test")
print("=" * 60)

# Connect
print("\n1. Connecting to server...")
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
print("   Connected!")

# Login
print("\n2. Logging in as '비교테스터'...")
time.sleep(1)
recv_nonblock(s, timeout=2)  # Clear banner

s.setblocking(True)
s.sendall("비교테스터\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(0.5)
recv_nonblock(s, timeout=1)  # Clear password prompt

s.setblocking(True)
s.sendall("test1234\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(2)
data = recv_nonblock(s, timeout=3)  # Notice

# Press Enter to continue
s.setblocking(True)
s.sendall("\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(1)
data = recv_nonblock(s, timeout=2)
print("   Login successful!")

# Check current position
print("\n3. Checking current position...")
result = send_cmd(s, "봐")
print(result[:500])

# Go to 곤륜선인 location (room 4000)
print("\n4. Moving to room 4000 (곤륜선인 location)...")
result = send_cmd(s, "이동 낙양성 4000")
print(result[:400])

# Look around
print("\n5. Looking around room 4000...")
result = send_cmd(s, "봐")
print(result[:800])

# Talk to 곤륜선인 about 난이도1
print("\n6. Talking to 곤륜선인 about 난이도1...")
result = send_cmd(s, "곤륜선인 대화 난이도1")
print(result[:800])

# Check new position (should be in 낙양성1:1)
print("\n7. Checking new position after zone change...")
result = send_cmd(s, "봐")
if "낙양성1" in result:
    print("   SUCCESS: Moved to difficulty zone 낙양성1!")
else:
    print("   Note: Checking for zone name...")
print(result[:800])

# Look at mobs in the difficulty zone
print("\n8. Looking for mobs...")
result = send_cmd(s, "봐")
print(result[:800])

# Try to attack a mob
print("\n9. Trying to attack a mob...")
result = send_cmd(s, "공격")
print(result[:500])

# Check player score to see stats
print("\n10. Checking player score...")
result = send_cmd(s, "점수")
print(result[:600])

# Return to base zone (test)
print("\n11. Testing return to base zone...")
result = send_cmd(s, "이동 낙양성 42")
print(result[:400])

s.close()
print("\n" + "=" * 60)
print("Test completed!")
print("=" * 60)
