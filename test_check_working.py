#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(15)
sock.connect(('localhost', 9990))

# Get banner
time.sleep(1)
sock.recv(8192)

# Login as 점수
sock.sendall("점수\r\n".encode('utf-8'))
time.sleep(1)
sock.recv(8192)

# Empty password
sock.sendall(b"\r\n")
time.sleep(1)

# Drain all buffered data
sock.settimeout(1)
try:
    while True:
        chunk = sock.recv(8192)
        if not chunk:
            break
except socket.timeout:
    pass

time.sleep(1)

# Try 봐
print("=== Sending: 봐 (should work) ===")
sock.sendall("봐\r\n".encode('utf-8'))
time.sleep(2)

sock.settimeout(3)
all_data = b""
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
except socket.timeout:
    pass

print(f"봐 response ({len(all_data)} bytes)")

# Try 점수 (score)
print("\n=== Sending: 점수 (alias for 능력치) ===")
sock.sendall("점수\r\n".encode('utf-8'))
time.sleep(2)

sock.settimeout(3)
all_data = b""
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
except socket.timeout:
    pass

print(f"점수 response ({len(all_data)} bytes)")
output = all_data.decode('utf-8', errors='replace')
if "┏" in output or "레  벨" in output:
    print("SUCCESS: Score output found!")
elif "오류" in output:
    print("ERROR: Found error message")

sock.close()
