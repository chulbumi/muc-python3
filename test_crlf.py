#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get banner
time.sleep(1)
sock.recv(8192)

# Send username with CRLF
print("=== Sending: test\\r\\n ===")
sock.sendall(b"test\r\n")
time.sleep(1)

response = sock.recv(8192)
print(f"Response (len={len(response)}):")
print(response.decode('utf-8', errors='ignore')[:500])

sock.close()
