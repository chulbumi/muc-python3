#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(15)
sock.connect(('localhost', 9990))

# Get banner
time.sleep(1)
sock.recv(8192)

# Send 무명객
sock.sendall("무명객\r\n".encode('utf-8'))
time.sleep(1)
sock.recv(8192)

# Send empty password
sock.sendall(b"\r\n")
time.sleep(1)
sock.recv(8192)

# Press Enter multiple times to skip intro
for i in range(5):
    sock.sendall(b"\r\n")
    time.sleep(0.5)

# Clear any pending output
sock.settimeout(1)
try:
    while True:
        data = sock.recv(8192)
        if not data:
            break
except socket.timeout:
    pass

sock.settimeout(15)

# Send 봐 (look) first to get into normal mode
print("=== Sending: 봐 (look) ===")
sock.sendall("봐\r\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
print(f"After look: {len(response)} bytes")

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(2)

# Read response
all_data = b""
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
        if len(all_data) > 5000:
            break
except socket.timeout:
    pass

print(f"\n=== 능력치 Output ({len(all_data)} bytes) ===")
output = all_data.decode('utf-8', errors='replace')
print(output)

sock.close()
