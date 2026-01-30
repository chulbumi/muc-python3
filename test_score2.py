#!/usr/bin/env python3
import socket
import time
import signal
import sys

def timeout_handler(signum, frame):
    print("Test timed out")
    sys.exit(1)

signal.signal(signal.SIGALRM, timeout_handler)
signal.alarm(10)  # 10 second timeout

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('localhost', 9999))
time.sleep(0.5)

# Login
s.send('testuser\n'.encode('utf-8'))
time.sleep(0.5)

# Send 능력치 command
s.send('능력치\n'.encode('utf-8'))
time.sleep(1.0)

# Read response
data = b''
while True:
    try:
        s.settimeout(2)
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
    except socket.timeout:
        break

s.close()
print(data.decode('utf-8', errors='replace'))
signal.alarm(0)  # Cancel alarm
