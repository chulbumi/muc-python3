#!/usr/bin/env python3
"""Test script for MUD mob spawning - go up to room 4000"""
import socket
import time

def utf8_encode(s):
    """Encode string to UTF-8 (EUC-KR 미지원)"""
    return s.encode('utf-8')

def test_mobs():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 9999))

    # Wait for initial screen
    time.sleep(1)
    sock.recv(4096)  # Clear initial screen

    # Login
    sock.sendall(utf8_encode("멍멍멍\r\n"))
    time.sleep(0.5)
    sock.recv(4096)  # Clear password prompt

    sock.sendall(utf8_encode("멍멍멍\r\n"))
    time.sleep(1)
    sock.recv(8192)  # Clear login screen

    sock.sendall(b"1\r\n")  # Enter game
    time.sleep(1)
    data = sock.recv(8192)
    print("=== Initial room (낙양성:1) ===")
    print(repr(data))
    print()

    # Go Up to room 4000 where mobs should be
    sock.sendall(utf8_encode("위\r\n"))
    time.sleep(0.5)
    data = sock.recv(8192)
    print("=== After going Up (room 4000 - should have mobs) ===")
    print(repr(data))
    print()

    # Look around
    sock.sendall(utf8_encode("봐\r\n"))
    time.sleep(0.5)
    data = sock.recv(8192)
    print("=== Look at room 4000 ===")
    print(repr(data))
    print()

    sock.close()

if __name__ == "__main__":
    test_mobs()
