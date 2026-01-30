#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get banner
time.sleep(1)
sock.recv(8192)

# Send simple Korean username
print("=== Sending: 김 (simple Korean, CRLF) ===")
sock.sendall("김\r\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
resp_str = response.decode('utf-8', errors='ignore')
print(f"Response (len={len(response)}):")
print(resp_str[:500])

# Send password prompt response
print("\n=== Sending: 김 (as password) ===")
sock.sendall("김\r\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
resp_str = response.decode('utf-8', errors='ignore')
print(f"Response (len={len(response)}):")
print(resp_str[:500])

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(2)

# Read all response
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

print(f"\n=== Response ({len(all_data)} bytes) ===")
print(all_data.decode('utf-8', errors='replace'))

sock.close()
