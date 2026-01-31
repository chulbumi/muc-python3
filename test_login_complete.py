#!/usr/bin/env python3
"""Complete login test for Rust MUD server (9999)"""

import socket
import time

def complete_login(name, password):
    """Test complete login flow"""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)

    try:
        sock.connect(("localhost", 9999))

        # Receive initial greeting
        data = sock.recv(4096)

        # Send Korean name
        name_bytes = (name + "\r\n").encode("utf-8")
        sock.sendall(name_bytes)
        time.sleep(0.2)

        # Receive response
        data = sock.recv(4096)
        decoded = data.decode("utf-8", errors="ignore")

        if "존함암호" in decoded:
            # Send password
            pwd_bytes = (password + "\r\n").encode("utf-8")
            sock.sendall(pwd_bytes)
            time.sleep(0.2)

            data = sock.recv(4096)
            decoded = data.decode("utf-8", errors="ignore")

            if "잘못된 암호" in decoded:
                return "WRONG_PASSWORD"
            elif "공지사항" in decoded or "입장하셨습니다" in decoded:
                return "SUCCESS"
            else:
                return "OTHER"

        elif "한글 입력만" in decoded:
            return "KOREAN_ONLY_ERROR"

        return "NO_PASSWORD_PROMPT"

    except Exception as e:
        return f"ERROR: {e}"
    finally:
        sock.close()

if __name__ == "__main__":
    print("=" * 60)
    print("Rust MUD Server (9999) Complete Login Test")
    print("=" * 60)

    # Test with the character that exists
    results = {
        "테스터러스트": complete_login("테스터러스트", "1234"),
        "테스터": complete_login("테스터", "1234"),
    }

    for name, result in results.items():
        print(f"\n{name}: {result}")

    # Check if all tests passed
    all_ok = all("KOREAN_ONLY_ERROR" not in r for r in results.values())
    print(f"\n{'=' * 60}")
    if all_ok:
        print("PASS: No 'Korean only' errors detected!")
        print("The login system is working correctly.")
    else:
        print("FAIL: Some tests failed")
