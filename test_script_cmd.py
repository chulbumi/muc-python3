#!/usr/bin/env python3
import socket, time

sock = socket.socket()
sock.connect(('localhost', 9999))
sock.setblocking(False)
time.sleep(1.5)
sock.recv(8198)
sock.send(b'\xed\x85\x90\xec\x8a\xa4\xed\x84\xb0\r\n')
time.sleep(0.8)
sock.recv(8198)
sock.send(b'1234\r\n')  # Correct password
time.sleep(0.8)
d = sock.recv(8198)
print(f"After password: {len(d)} bytes")
sock.send(b'\xec\xa0\x90\xec\x88\x98\r\n')
time.sleep(2.0)
d = sock.recv(8192)
print(f"After 점수: {len(d)} bytes")
text = d.decode('utf-8', errors='replace')
print(f"Text preview: {text[:200]}")
sock.close()
