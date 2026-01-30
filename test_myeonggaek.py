#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get banner
time.sleep(1)
sock.recv(8192)

# Send 무명객 (the default name)
print("=== Sending: 무명객 ===")
sock.sendall("무명객\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
print(f"Response (len={len(response)}):")
print(response.decode('utf-8', errors='ignore')[:500])

# Send password (empty/enter)
print("\n=== Sending: Enter ===")
sock.sendall(b"\n")
time.sleep(1)

response = sock.recv(8192)
print(f"Response (len={len(response)}):")
print(response.decode('utf-8', errors='ignore')[:500])

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\n".encode('utf-8'))
time.sleep(2)

# Read all response
all_data = b""
sock.settimeout(1)
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
