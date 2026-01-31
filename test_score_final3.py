#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9999))

# Get banner
time.sleep(1)
sock.recv(8192)

# Login as 점수
print("=== Login as 테스트 ===")
sock.sendall("테스트\r\n".encode('utf-8'))
time.sleep(1)
sock.recv(8192)

# Password
sock.sendall("1234\r\n".encode('utf-8'))
time.sleep(1)

# Get all login messages
all_login = b""
sock.settimeout(2)
try:
    while True:
        chunk = sock.recv(8192)
        if not chunk:
            break
        all_login += chunk
except socket.timeout:
    pass

print(f"Login output: {len(all_login)} bytes")

# Wait a bit more for things to settle
time.sleep(1)

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(3)

# Read response
all_data = b""
sock.settimeout(2)
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
except socket.timeout:
    pass

print(f"\n=== 능력치 Output ({len(all_data)} bytes) ===")
output = all_data.decode('utf-8', errors='replace')

# Print relevant parts
for line in output.split('\n'):
    if '┏' in line or '┝' in line or '├' in line or '┕' in line or '레  벨' in line or '체  력' in line or '은  전' in line or '능력치' in line:
        print(line)
    elif '▷▶▷▶▷' in line or '◁◀◁◀◁' in line:
        print(line)

print("\n--- Full output ---")
print(output)

sock.close()
