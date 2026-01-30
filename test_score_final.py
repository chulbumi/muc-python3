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
sock.sendall("무명객\r\n".encode('utf-8'))
time.sleep(1)
sock.recv(8192)

# Send empty password
sock.sendall(b"\r\n")
time.sleep(1)
sock.recv(8192)

# Press Enter to skip intro
sock.sendall(b"\r\n")
time.sleep(1)

# Clear intro output
sock.settimeout(1)
try:
    while True:
        data = sock.recv(8192)
        if not data:
            break
except socket.timeout:
    pass

sock.settimeout(15)

# Fight the mob to complete the event
print("=== Fighting mob: 흑백쌍괴 쳐 ===")
sock.sendall("흑백쌍괴 쳐\r\n".encode('utf-8'))
time.sleep(2)

# Clear combat output
sock.settimeout(1)
try:
    while True:
        data = sock.recv(8192)
        if not data:
            break
except socket.timeout:
    pass

sock.settimeout(15)

# Send 봐 (look) to get room description
print("=== Sending: 봐 (look) ===")
sock.sendall("봐\r\n".encode('utf-8'))
time.sleep(1)
response = sock.recv(8192)
print(response.decode('utf-8', errors='replace')[:500])

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
