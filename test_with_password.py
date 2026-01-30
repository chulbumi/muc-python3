#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get initial banner
time.sleep(1)
initial = sock.recv(8192)
print("=== Initial Banner ===")
print(initial.decode('utf-8', errors='ignore'))

# Login with test user
print("\n=== Sending: test ===")
sock.sendall(b"test\n")
time.sleep(1)

response = sock.recv(8192)
print(response.decode('utf-8', errors='ignore'))

# Check if password is needed
if "암호" in response.decode('utf-8', errors='ignore') or "password" in response.decode('utf-8', errors='ignore').lower():
    print("\n=== Sending password: 1234 ===")
    sock.sendall(b"1234\n")
    time.sleep(1)

    response = sock.recv(8192)
    print(response.decode('utf-8', errors='ignore'))

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
print("=== Response ===")
print(response.decode('utf-8', errors='replace'))

sock.close()
