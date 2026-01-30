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
print("=== Sending: 무명객 ===")
sock.sendall("무명객\r\n".encode('utf-8'))
time.sleep(1)
sock.recv(8192)

# Send empty password to continue
print("=== Pressing Enter to continue ===")
sock.sendall(b"\r\n")
time.sleep(1)
sock.recv(8192)

# Press Enter again to continue past intro
print("=== Pressing Enter to skip intro ===")
sock.sendall(b"\r\n")
time.sleep(1)

# Read and discard intro output
try:
    sock.settimeout(2)
    while True:
        data = sock.recv(8192)
        if not data:
            break
        print(f"Discarding {len(data)} bytes...")
except socket.timeout:
    pass

sock.settimeout(15)

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
        if "┕".encode('utf-8') in all_data:  # End of the box drawing
            break
except socket.timeout:
    pass

print(f"\n=== 능력치 Output ({len(all_data)} bytes) ===")
print(all_data.decode('utf-8', errors='replace'))

sock.close()
