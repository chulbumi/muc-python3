#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get initial banner
time.sleep(1)
initial = sock.recv(8192)
print("=== Connected! ===")

# Send username character by character with delays
username = "test"
print(f"\n=== Sending username: {username} ===")
for ch in username:
    sock.sendall(ch.encode('utf-8'))
    time.sleep(0.05)  # Small delay between characters

sock.sendall(b"\n")
time.sleep(1)

response = sock.recv(8192)
resp_str = response.decode('utf-8', errors='ignore')
print(f"Response (len={len(response)}):")
print(resp_str[:500])

sock.close()
