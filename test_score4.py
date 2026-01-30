#!/usr/bin/env python3
import socket
import time

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(5)
s.connect(('localhost', 9999))

# Wait for initial message
time.sleep(1)

# Login with existing character "test"
s.sendall('test\n'.encode('utf-8'))
time.sleep(1)

# Send 능력치 command
s.sendall('능력치\n'.encode('utf-8'))
time.sleep(1)

# Read response
data = b''
while True:
    try:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
    except socket.timeout:
        break

print(data.decode('utf-8', errors='replace'))

s.close()
