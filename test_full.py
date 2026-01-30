#!/usr/bin/env python3
import socket, time, sys

sock = socket.socket()
sock.connect(('localhost', 9999))
sock.setblocking(False)
time.sleep(1.5)
sock.recv(8192)
# Create new character via quick helper
sock.send(b'\xeb\x82\x98\xeb\xa7\x84\xeb\xb0\x94\xeb\xb0\x94\xec\x95\xbc\r\n')
time.sleep(0.8)
sock.recv(8192)
sock.send(b'\xea\xb9\x80\xec\xb2\xa0\xec\x88\x98\r\n')
time.sleep(0.8)
sock.recv(8192)
sock.send(b'7777\r\n')
time.sleep(0.8)
sock.recv(8192)
sock.send(b'\xeb\x82\xa1\r\n')
time.sleep(0.8)
sock.recv(8192)
sock.send(b'\r\n')
time.sleep(1.0)
d = sock.recv(8192)
print(f"After creation: {len(d)} bytes")
text = d.decode('utf-8', errors='replace')
print(f"Creation preview: {text[:200]}")

# Now try score command
sock.send(b'\xec\xa0\x90\xec\x88\x98\r\n')
time.sleep(2.0)
d = sock.recv(8192)
print(f"After 점수: {len(d)} bytes")
text = d.decode('utf-8', errors='replace')
print(f"Score preview: {text[:500]}")

sock.close()
