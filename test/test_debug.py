#!/usr/bin/env python3
"""Debug test - see raw data from server"""

import socket
import time

print("Connecting to localhost:9999...")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 9999))
s.settimeout(10)  # Longer timeout

# Get initial banner
print("\n=== Waiting for initial banner ===")
data = b""
try:
    while True:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
        if b":" in data:
            break
except socket.timeout:
    pass
print(f"Received {len(data)} bytes")
print(data.decode('utf-8', errors='replace')[-200:])

# Send username
print("\n=== Sending username '비교테스터' ===")
s.sendall("비교테스터\n".encode('utf-8'))

# Wait for response
print("Waiting for response...")
data = b""
try:
    while True:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
        if b":" in data or len(data) > 100:
            break
except socket.timeout:
    pass
print(f"After username: {len(data)} bytes")
if data:
    print(data.decode('utf-8', errors='replace'))

# Send password
print("\n=== Sending password 'test1234' ===")
s.sendall("test1234\n".encode('utf-8'))

# Wait for response - use longer timeout
print("Waiting for response (10s timeout)...")
s.settimeout(10)
data = b""
try:
    while True:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
        if len(data) > 500:
            break
except socket.timeout:
    print("Timeout - no response received")
except Exception as e:
    print(f"Error: {e}")
print(f"After password: {len(data)} bytes")
if data:
    print(data.decode('utf-8', errors='replace')[:1500])

s.close()
print("\nDone!")
