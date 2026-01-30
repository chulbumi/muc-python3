#!/usr/bin/env python3
import socket
import time

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 9999))
time.sleep(0.5)

# Login
s.send('testuser\n'.encode('utf-8'))
time.sleep(0.5)

# Send 능력치 command
s.send('능력치\n'.encode('utf-8'))
time.sleep(0.5)

# Read response
data = b''
while True:
    chunk = s.recv(4096)
    if not chunk:
        break
    data += chunk
    if len(data) > 5000:  # Read enough
        break

s.close()
print(data.decode('utf-8', errors='replace'))
