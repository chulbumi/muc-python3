#!/usr/bin/env python3
import socket
import time

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(5)
s.connect(('localhost', 9999))

# Wait for initial message
time.sleep(1)

# Login with existing character
s.sendall('무명객\n'.encode('utf-8'))
time.sleep(1)

# Read initial messages
data = s.recv(8192)
print("After login:")
print(data.decode('utf-8', errors='replace'))

# Send 능력치 command
s.sendall('능력치\n'.encode('utf-8'))
time.sleep(1)

# Read response
data = s.recv(8192)
print("\nAfter 능력치:")
print(data.decode('utf-8', errors='replace'))

s.close()
