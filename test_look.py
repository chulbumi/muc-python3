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

# Try 봐 (look) - this should work
print("=== Sending: 봐 (look) ===")
sock.sendall("봐\r\n".encode('utf-8'))
time.sleep(2)

# Read response
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

print(f"Response ({len(all_data)} bytes):")
output = all_data.decode('utf-8', errors='replace')
# Check if it's different from the default room description
if "낙양성" in output and len(output) < 1000:
    print("SUCCESS: Got room description!")
elif "이름" in output or "Name" in output:
    print("SUCCESS: Got different output!")
else:
    print("Got standard room description (might be cached)")

sock.close()
