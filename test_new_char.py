#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get initial banner
time.sleep(1)
initial = sock.recv(8192)
print("=== Initial Banner (first 500 chars) ===")
print(initial.decode('utf-8', errors='ignore')[:500])

# Create new character
print("\n=== Sending: 능력치테스터 (new character) ===")
sock.sendall("능력치테스터\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
resp_str = response.decode('utf-8', errors='ignore')
print(f"Response (len={len(response)}):")
print(resp_str[:500])

# Send empty password/enter
print("\n=== Sending: Enter (empty) ===")
sock.sendall(b"\n")
time.sleep(1)

response = sock.recv(8192)
resp_str = response.decode('utf-8', errors='ignore')
print(resp_str[:500])

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\n".encode('utf-8'))
time.sleep(2)

# Read all available data
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

print("=== Response ===")
print(all_data.decode('utf-8', errors='replace'))

sock.close()
