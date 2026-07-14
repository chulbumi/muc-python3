#!/usr/bin/env python3
"""Test with CRLF line endings"""

import socket
import select
import time

def recv_nonblock(sock, timeout=3.0):
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

try:
    s.connect(('localhost', 9999))
except BlockingIOError:
    pass

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

# Send username with CRLF
print("\n=== Sending username (CRLF) ===")
s.setblocking(True)
s.sendall("비교테스터\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(1)

# Read response
print("Reading response...")
data = recv_nonblock(s, timeout=3)
print(f"Response ({len(data)} bytes)")
if data:
    print(data.decode('utf-8', errors='replace'))

# Send password with CRLF
print("\n=== Sending password (CRLF) ===")
s.setblocking(True)
s.sendall("test1234\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(2)

# Read response
print("Reading response...")
data = recv_nonblock(s, timeout=5)
print(f"Response ({len(data)} bytes)")
if data:
    text = data.decode('utf-8', errors='replace')
    print(text[:1500])

# Try look command
print("\n=== Sending '봐' (CRLF) ===")
s.setblocking(True)
s.sendall("봐\r\n".encode('utf-8'))
s.setblocking(False)
time.sleep(1)

data = recv_nonblock(s, timeout=3)
print(f"Response ({len(data)} bytes)")
if data:
    print(data.decode('utf-8', errors='replace')[:1000])

s.close()
print("\nDone!")
