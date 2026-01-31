#!/usr/bin/env python3
import subprocess
import time
import socket
import os

# Kill any existing server
try:
    subprocess.run(['killall', '-9', 'murim_server'], stderr=subprocess.DEVNULL)
except FileNotFoundError:
    pass
time.sleep(2)

# Start server
server_proc = subprocess.Popen(
    ['./target/debug/murim_server'],
    stdout=open('/tmp/murim_server.log', 'w'),
    stderr=subprocess.STDOUT,
    cwd='/home/ubuntu/muc-python3'
)
print(f"Started server PID: {server_proc.pid}")

# Wait for server to start
time.sleep(5)

# Show server log
print("=== Server Log ===")
with open('/tmp/murim_server.log', 'r') as f:
    print(f.read())

# Test connection
print("\n=== Testing Connection ===")
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(('localhost', 9999))

# Receive initial output
time.sleep(1)
data = sock.recv(4096)
print(data.decode('utf-8', errors='ignore'))

# Send username
sock.sendall('test\n'.encode('utf-8'))
time.sleep(1)
data = sock.recv(4096)
print(data.decode('utf-8', errors='ignore'))

# Send password
sock.sendall('1234\n'.encode('utf-8'))
time.sleep(4)
data = sock.recv(8192)
print(data.decode('utf-8', errors='ignore'))

# Press enter to enter game
sock.sendall('\n\n'.encode('utf-8'))
time.sleep(3)
data = sock.recv(8192)
print(data.decode('utf-8', errors='ignore'))

# Look at room
sock.sendall('봐\n'.encode('utf-8'))
time.sleep(2)
data = sock.recv(8192)
output = data.decode('utf-8', errors='ignore')
print(output)

# Check if mobs are present
if '밍밍' in output or '포졸' in output or '범죄자' in output:
    print("\n>>> MOBS FOUND IN ROOM!")
else:
    print("\n>>> NO MOBS FOUND IN ROOM!")

sock.close()

# Kill server
server_proc.terminate()
time.sleep(1)
server_proc.kill()
print("\nServer stopped")
