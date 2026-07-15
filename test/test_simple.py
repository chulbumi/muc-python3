#!/usr/bin/env python3
"""Simple test to debug server responses"""

import socket
import time
import sys

def recv_with_timeout(sock, timeout=3.0):
    sock.settimeout(timeout)
    data = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
            # If we see a prompt, stop
            if b":" in data or len(data) > 1000:
                break
    except socket.timeout:
        pass
    return data

print("Connecting to localhost:9999...")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 9999))

# Get initial banner
print("\n=== Waiting for initial banner ===")
data = recv_with_timeout(s)
print(f"Received {len(data)} bytes")
print(data.decode('utf-8', errors='ignore')[:500])
print("--- END ---")

# Send username
print("\n=== Sending username 'testchar' ===")
s.sendall(b"testchar\n")
time.sleep(1)
data = recv_with_timeout(s)
print(f"Received {len(data)} bytes")
print(data.decode('utf-8', errors='ignore')[:500])
print("--- END ---")

# Send password
print("\n=== Sending password 'testchar' ===")
s.sendall(b"testchar\n")
time.sleep(2)
data = recv_with_timeout(s)
print(f"Received {len(data)} bytes")
text = data.decode('utf-8', errors='ignore')
print(text[:1000])
print("--- END ---")

# Try look command
print("\n=== Sending 'look' command ===")
s.sendall(b"look\n")
time.sleep(1)
data = recv_with_timeout(s)
print(f"Received {len(data)} bytes")
text = data.decode('utf-8', errors='ignore')
print(text[:800])
print("--- END ---")

s.close()
print("\nDone!")
