#!/usr/bin/env python3
"""Test mob spawning and difficulty zone"""

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
print("Mob Spawn and Difficulty Zone Test")
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

# Move to room 4000
print("\n2. Moving to room 4000...")
result = send_cmd(s, "이동 낙양성 4000")
print(result[:300])

# Look for mobs
print("\n3. Looking for mobs in room 4000...")
result = send_cmd(s, "봐")
print(result)

# Check if we can interact with 곤륜선인 directly
print("\n4. Trying to list mobs...")
result = send_cmd(s, "몹찾기 곤륜선인")
print(result[:500])

# Try the correct command format for 대화
print("\n5. Trying '곤륜선인에게 말하기 난이도1'...")
result = send_cmd(s, "곤륜선인에게 말하기 난이도1")
print(result[:500])

# Try different command formats
print("\n6. Trying '대화 곤륜선인 난이도1'...")
result = send_cmd(s, "대화 곤륜선인 난이도1")
print(result[:500])

# Try saying to the NPC
print("\n7. Trying '말 난이도1' to 곤륜선인...")
result = send_cmd(s, "말 난이도1")
print(result[:500])

# Check available commands
print("\n8. Trying '도움말 대화'...")
result = send_cmd(s, "도움말 대화")
print(result[:500])

# Try the $대 trigger
print("\n9. Trying direct '$대화 난이도1'...")
result = send_cmd(s, "$대화 난이도1")
print(result[:500])

# Go to a mob room and test combat
print("\n10. Moving to room 1 (has mobs)...")
result = send_cmd(s, "이동 낙양성 1")
print(result[:300])

print("\n11. Looking for mobs in room 1...")
result = send_cmd(s, "봐")
print(result)

# Try to attack
print("\n12. Trying '쳐' to see attackable targets...")
result = send_cmd(s, "쳐")
print(result[:500])

s.close()
print("\n" + "=" * 60)
print("Test completed!")
print("=" * 60)
