#!/usr/bin/env python3
"""Test using raw socket with select"""

import socket
import select
import time

def recv_nonblock(sock, timeout=3.0):
    """Receive data with non-blocking select"""
    data = b""
    end_time = time.time() + timeout
    while time.time() < end_time:
        ready, _, _ = select.select([sock], [], [], 0.5)
        if ready:
            try:
                chunk = sock.recv(4096)
                if not chunk:
                    break
                data += chunk
            except:
                break
        else:
            if data:
                break
    return data

print("Connecting to localhost:9999...")
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setblocking(False)

# Connect
try:
    s.connect(('localhost', 9999))
except BlockingIOError:
    pass

# Wait for connection
_, writable, _ = select.select([], [s], [], 5.0)
if not writable:
    print("Connection failed")
    exit(1)

print("Connected!")

# Read banner
print("\n=== Reading banner ===")
time.sleep(1)
data = recv_nonblock(s, timeout=3)
print(f"Banner ({len(data)} bytes)")
print(data.decode('utf-8', errors='replace')[-300:])

# Send username
print("\n=== Sending username ===")
s.setblocking(True)
s.sendall("비교테스터\n".encode('utf-8'))
s.setblocking(False)
time.sleep(1)

# Read response
print("Reading response...")
data = recv_nonblock(s, timeout=3)
print(f"Response ({len(data)} bytes)")
if data:
    print(data.decode('utf-8', errors='replace'))
else:
    print("No response received")

# Send password
print("\n=== Sending password ===")
s.setblocking(True)
s.sendall("test1234\n".encode('utf-8'))
s.setblocking(False)
time.sleep(2)

# Read response
print("Reading response...")
data = recv_nonblock(s, timeout=5)
print(f"Response ({len(data)} bytes)")
if data:
    text = data.decode('utf-8', errors='replace')
    print(text[:1500])
else:
    print("No response received")

# Try 봐 command
print("\n=== Sending '봐' ===")
s.setblocking(True)
s.sendall("봐\n".encode('utf-8'))
s.setblocking(False)
time.sleep(1)

data = recv_nonblock(s, timeout=3)
print(f"Response ({len(data)} bytes)")
if data:
    print(data.decode('utf-8', errors='replace')[:1000])

s.close()
print("\nDone!")
