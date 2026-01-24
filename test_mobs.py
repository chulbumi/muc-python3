#!/usr/bin/env python3
"""Test script for MUD mob spawning"""
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

    # Move South to room 5
    sock.sendall(utf8_encode("남\r\n"))
    time.sleep(0.5)
    data = sock.recv(8192)
    print("=== After moving South (room 5) ===")
    print(repr(data))
    print()

    # Move North to go back to room 1, then to room 35
    # We need to navigate to room 42
    # Path from 1: South -> 5, then need to find path to 42
    # For now let's just look around room 5
    sock.sendall(utf8_encode("봐\r\n"))
    time.sleep(0.5)
    data = sock.recv(8192)
    print("=== Look at room 5 ===")
    print(repr(data))
    print()

    sock.close()

if __name__ == "__main__":
    test_mobs()
