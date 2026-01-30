#!/usr/bin/env python3
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(5)
sock.connect(('localhost', 9990))

# Just get the banner
time.sleep(1)
data = sock.recv(8192)

print("=== Banner received ({} bytes) ===".format(len(data)))
print(data.decode('utf-8', errors='ignore'))

sock.close()
print("\nConnection closed successfully")
