#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
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

# Clear login messages
sock.settimeout(1)
try:
    while True:
        chunk = sock.recv(8192)
        if not chunk:
            break
except socket.timeout:
    pass

sock.settimeout(10)
time.sleep(1)

# Try 능력치 command
print("=== Sending: 능력치 (after deleting file) ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(2)

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

print(f"Response (len={len(all_data)}):")
print(all_data.decode('utf-8', errors='replace'))

sock.close()
