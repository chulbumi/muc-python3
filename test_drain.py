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
all_buffered = b""
try:
    while True:
        chunk = sock.recv(8192)
        if not chunk:
            break
        all_buffered += chunk
        print(f"Drained {len(chunk)} bytes")
except socket.timeout:
    pass

print(f"Total drained: {len(all_buffered)} bytes")

# Wait before sending command
time.sleep(1)

# Try 능력치
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(2)

# Read response - set non-blocking to get all available
sock.settimeout(3)
all_data = b""
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
        print(f"Received {len(chunk)} bytes")
except socket.timeout:
    pass

print(f"\n=== Total Response ({len(all_data)} bytes) ===")
output = all_data.decode('utf-8', errors='replace')
print(output)

sock.close()
