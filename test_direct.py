#!/usr/bin/env python3
"""Direct test of MUD servers"""

import socket
import time
import sys
import re

def test_server(port, name):
    print(f"\n{'='*60}")
    print(f"Testing {name} (port {port})")
    print('='*60)

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(3)
    try:
        sock.connect(('localhost', port))

        # Get initial banner
        time.sleep(0.5)
        try:
            data = sock.recv(4096).decode('utf-8', errors='ignore')
            print(f"Initial banner:\n{data[:500]}")
        except:
            pass

        # Send login
        sock.sendall("테스터\n".encode('utf-8'))
        time.sleep(0.5)

        # Try password
        sock.sendall("test1234\n".encode('utf-8'))
        time.sleep(0.5)

        # Send enter
        sock.sendall("\n".encode('utf-8'))
        time.sleep(0.5)

        # Get game screen
        try:
            data = sock.recv(4096).decode('utf-8', errors='ignore')
            print(f"\nAfter login:\n{data[:500]}")
        except:
            pass

        # Test 보기 command
        print(f"\n--- Testing '보기' command ---")
        sock.sendall("보기\n".encode('utf-8'))
        time.sleep(0.5)
        try:
            data = sock.recv(4096).decode('utf-8', errors='ignore')
            # Strip ANSI
            clean = re.sub(r'\x1b\[[0-9;]*m', '', data)
            print(clean[:500])
        except:
            pass

        sock.close()
        return True
    except Exception as e:
        print(f"Error: {e}")
        return False

# Test both servers
test_server(9900, "Python MUD")
test_server(9990, "Rust MUD")
