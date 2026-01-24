#!/usr/bin/env python3
"""Test script for MUD login and movement"""
import socket
import time

def utf8_encode(s):
    """Encode string to UTF-8 (EUC-KR 미지원)"""
    return s.encode('utf-8')

def utf8_decode(b):
    """Decode bytes from UTF-8"""
    try:
        return b.decode('utf-8')
    except Exception:
        return b.decode('utf-8', errors='replace')

def test_mud():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 9999))

    # Wait for initial screen
    time.sleep(1)
    data = sock.recv(4096)
    print("=== Initial screen (first 500 bytes) ===")
    print(repr(data[:500]))
    print()

    # Send username (UTF-8, EUC-KR 미지원)
    username = "멍멍멍"
    print(f"Sending username: {repr(username)}")
    sock.sendall(utf8_encode(username + "\r\n"))
    time.sleep(0.5)

    data = sock.recv(4096)
    print("=== After username ===")
    print(repr(data))
    print()

    # Send password
    password = "멍멍멍"
    print(f"Sending password: {repr(password)}")
    sock.sendall(utf8_encode(password + "\r\n"))
    time.sleep(1)

    data = sock.recv(8192)
    print("=== After password (login screen) ===")
    print(repr(data))
    print()

    # Send '1' to select "연결" (Connect)
    print("Sending '1'")
    sock.sendall(b"1\r\n")
    time.sleep(1)

    data = sock.recv(8192)
    print("=== After selecting '1' (enter game) ===")
    print(repr(data))
    print()

    # Test look command
    look_cmd = "봐"
    print(f"Sending look command: {repr(look_cmd)}")
    sock.sendall(utf8_encode(look_cmd + "\r\n"))
    time.sleep(0.5)

    data = sock.recv(8192)
    print("=== After '봐' (look) command ===")
    print(repr(data))
    print()

    # Test movement south
    south_cmd = "남"
    print(f"Sending south command: {repr(south_cmd)}")
    sock.sendall(utf8_encode(south_cmd + "\r\n"))
    time.sleep(0.5)

    data = sock.recv(8192)
    print("=== After '남' (south) movement ===")
    print(repr(data))
    print()

    sock.close()

if __name__ == "__main__":
    test_mud()
