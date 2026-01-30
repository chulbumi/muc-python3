#!/usr/bin/env python3
import socket, time

sock = socket.socket()
sock.connect(('localhost', 9999))
sock.setblocking(False)

def recv_all(timeout=1):
    data = b''
    start = time.time()
    while time.time() - start < timeout:
        try:
            chunk = sock.recv(8192)
            if chunk:
                data += chunk
            else:
                break
        except BlockingIOError:
            time.sleep(0.05)
    return data

# Get initial banner
time.sleep(1)
d = recv_all(1)
print(f"Initial: {len(d)} bytes")

# Use special name "나만바라바" to trigger DOUMI character creation (simpler script)
sock.sendall(b'\xeb\x82\x98\xeb\xa7\x8c\xeb\xb0\x94\xeb\x9d\xbc\xeb\xb0\x94\xec\x95\xbc\r\n')  # 나만바라바
time.sleep(1)
d = recv_all(1)
print(f"After 나만바라바: {len(d)} bytes")

# DOUMI should ask for name - send a name
sock.sendall(b'\xec\x98\xa4\xeb\x8a\x98\xec\xb9\x98\xec\x8a\xa4\r\n')  # 오늘치스
time.sleep(1)
d = recv_all(1)
print(f"After name: {len(d)} bytes")

# DOUMI asks for password
sock.sendall(b'1234\r\n')
time.sleep(1)
d = recv_all(1)
print(f"After password: {len(d)} bytes")

# DOUMI asks for gender
sock.sendall(b'\xeb\x82\xa8\r\n')  # 남 (male)
time.sleep(2)
d = recv_all(2)
print(f"After gender: {len(d)} bytes")
text = d.decode('utf-8', errors='replace')
print(f"After gender preview: {text[:500]}")

# Now try score command
sock.sendall(b'\xec\xa0\x90\xec\x88\x98\r\n')  # 점수
time.sleep(2.0)
d = recv_all(2)
print(f"\nAfter 점수: {len(d)} bytes")
text = d.decode('utf-8', errors='replace')
print(f"Score preview:\n{text[:500]}")

sock.close()
