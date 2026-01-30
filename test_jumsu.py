#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(10)
sock.connect(('localhost', 9990))

# Get banner
time.sleep(1)
sock.recv(8192)

# Send 점수 (new character without password)
print("=== Sending: 점수 ===")
sock.sendall("점수\r\n".encode('utf-8'))
time.sleep(1)

response = sock.recv(8192)
resp_str = response.decode('utf-8', errors='ignore')
print(f"Response (len={len(response)}):")
print(resp_str[:500])

# Check if password is needed
if "암호" in resp_str or "password" in resp_str.lower():
    print("\n=== Sending empty password ===")
    sock.sendall(b"\r\n")
    time.sleep(1)
    response = sock.recv(8192)
    resp_str = response.decode('utf-8', errors='ignore')
    print(resp_str[:500])

# Send 능력치 command
print("\n=== Sending: 능력치 ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(2)

# Read response
all_data = b""
sock.settimeout(2)
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
except socket.timeout:
    pass

print(f"\n=== 능력치 Output ({len(all_data)} bytes) ===")
output = all_data.decode('utf-8', errors='replace')
print(output)

sock.close()
