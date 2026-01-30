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
sock.sendall("점수\r\n".encode('utf-8'))
time.sleep(1)
sock.recv(8192)

# Empty password
sock.sendall(b"\r\n")
time.sleep(1)

# Drain all buffered data
sock.settimeout(1)
try:
    while True:
        chunk = sock.recv(8192)
        if not chunk:
            break
except socket.timeout:
    pass

time.sleep(1)

# Try 능력치
print("=== Sending: 능력치 ===")
sock.sendall("능력치\r\n".encode('utf-8'))
time.sleep(3)

# Read response - wait longer for more data
sock.settimeout(5)
all_data = b""
try:
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        all_data += chunk
        print(f"Received {len(chunk)} bytes, total {len(all_data)}")
        # Check if we have enough data
        if len(all_data) > 2000:
            break
except socket.timeout:
    pass

print(f"\n=== Total Response ({len(all_data)} bytes) ===")
output = all_data.decode('utf-8', errors='replace')

# Look for the score table
if "┏" in output or "레  벨" in output or "능력치" in output:
    print("Found score table output!")
elif "오류" in output or "Error" in output:
    print("Found error message!")
    # Print error line
    for line in output.split('\n'):
        if "오류" in line or "Error" in line:
            print(f"Error: {line}")
else:
    print("No score table found, just room description")

sock.close()
