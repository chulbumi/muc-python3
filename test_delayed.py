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
print("Login...")
sock.sendall("점수\r\n".encode('utf-8'))
time.sleep(2)

response = sock.recv(8192)
print(f"After username: {len(response)} bytes")

# Empty password
sock.sendall(b"\r\n")
time.sleep(3)

# Get all login messages
all_login = b""
sock.settimeout(3)
try:
    while True:
        chunk = sock.recv(8192)
        if not chunk:
            break
        all_login += chunk
        print(f"Received {len(chunk)} bytes, total {len(all_login)}")
except socket.timeout:
    pass

print(f"Total login output: {len(all_login)} bytes")
print(all_login.decode('utf-8', errors='replace')[:500])

# Wait before sending command
time.sleep(2)

# Try 도움말
print("\n=== Sending: 도움말 ===")
sock.sendall("도움말\r\n".encode('utf-8'))
time.sleep(3)

all_data = b""
sock.settimeout(3)
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
