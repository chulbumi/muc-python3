#!/usr/bin/env python3
"""Test Rust MUD server (9999) login with Korean name"""

import socket
import time

def test_login(name, password="1234"):
    """Test login to MUD server"""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)

    try:
        sock.connect(("localhost", 9999))

        # Receive initial greeting
        data = sock.recv(4096)
        print(f"[RECV] {len(data)} bytes: {repr(data[:200])}")

        # Send Korean name encoded in UTF-8
        name_bytes = (name + "\r\n").encode("utf-8")
        print(f"[SEND] Sending name: {name} ({len(name_bytes)} bytes)")
        print(f"[SEND] Bytes: {name_bytes}")
        sock.sendall(name_bytes)

        # Receive response
        data = sock.recv(4096)
        print(f"[RECV] {len(data)} bytes")
        print(f"[RECV] Data: {repr(data)}")
        decoded = data.decode("utf-8", errors="ignore")
        print(f"[DECODED] {decoded[:100]}")

        # Check if password prompt is received (존함암호ː or 비밀번호)
        if "존함암호" in decoded or "비밀번호" in decoded or "Password" in decoded:
            print("[SUCCESS] Password prompt received - login is working!")
            # Send password
            pwd_bytes = (password + "\r\n").encode("utf-8")
            sock.sendall(pwd_bytes)

            data = sock.recv(4096)
            print(f"[RECV] After password: {len(data)} bytes")
            print(f"[RECV] Data: {repr(data[:200])}")
        elif "한글 입력만" in data.decode("utf-8", errors="ignore"):
            print("[ERROR] '한글 입력만' message received - validation failed!")
            print(f"[DEBUG] is_han('{name}') should return True")
        else:
            print("[UNKNOWN] Unexpected response")

    except Exception as e:
        print(f"[ERROR] {e}")
    finally:
        sock.close()

if __name__ == "__main__":
    print("=" * 60)
    print("Testing Rust MUD Server (port 9999) login")
    print("=" * 60)

    # Test with various Korean names
    test_names = ["테스터러스트", "테스터", "철수"]

    for name in test_names:
        print(f"\n{'=' * 60}")
        print(f"Testing with name: {name}")
        print("=" * 60)
        test_login(name)
        time.sleep(1)
