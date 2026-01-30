#!/usr/bin/env python3
"""Test login and character creation on both Python and Rust MUD"""

import socket
import time
import re

def clean_ansi(text):
    """Remove ANSI escape codes"""
    return re.sub(r'\x1b\[[0-9;]*m', '', text)

def connect_and_test(port, server_name):
    """Connect to server and test login"""
    print(f"\n{'='*60}")
    print(f"Testing {server_name} (port {port})")
    print('='*60)

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5)
    try:
        sock.connect(('localhost', port))

        # Get initial banner
        time.sleep(0.5)
        initial = sock.recv(8192).decode('utf-8', errors='ignore')

        print(f"\n[1] Initial Banner (first 200 chars, no ANSI):")
        print(clean_ansi(initial)[:200])

        # Try to login with test character
        test_name = "비교테스터"
        sock.sendall(f"{test_name}\n".encode('utf-8'))
        time.sleep(0.3)

        # Check for password prompt or new character prompt
        response = sock.recv(8192).decode('utf-8', errors='ignore')
        print(f"\n[2] After username (first 300 chars):")
        print(clean_ansi(response)[:300])

        # Check if it's asking for password or new character
        if "없는" in response or "새로" in response or "새" in response:
            print("\n-> Creating new character")
            sock.sendall("test1234\n".encode('utf-8'))
            time.sleep(0.3)
            response = sock.recv(8192).decode('utf-8', errors='ignore')
            print(clean_ansi(response)[:300])
        elif "암호" in response or "password" in response.lower():
            print("\n-> Password prompt detected")
            sock.sendall("test1234\n".encode('utf-8'))
            time.sleep(0.3)
            response = sock.recv(8192).decode('utf-8', errors='ignore')
            print(clean_ansi(response)[:300])

        # Try to enter game
        sock.sendall("\n".encode('utf-8'))
        time.sleep(0.3)
        response = sock.recv(8192).decode('utf-8', errors='ignore')

        print(f"\n[3] After entering game (first 400 chars):")
        print(clean_ansi(response)[:400])

        sock.close()
        return clean_ansi(initial), clean_ansi(response)
    except Exception as e:
        print(f"Error: {e}")
        return None, None

# Test both servers
py_initial, py_game = connect_and_test(9900, "Python MUD")
rust_initial, rust_game = connect_and_test(9990, "Rust MUD")

# Compare
print(f"\n{'='*60}")
print("COMPARISON")
print('='*60)

if py_initial and rust_initial:
    print("\nInitial banner match:", py_initial[:100] == rust_initial[:100])
if py_game and rust_game:
    print("Game screen match:", py_game[:100] == rust_game[:100])

    # Check for key elements
    for key in ["체력", "내공", "은전", "레벨", "보기"]:
        py_has = key in py_game
        rust_has = key in rust_game
        print(f"  {key}: Python={py_has}, Rust={rust_has}, Match={py_has==rust_has}")
