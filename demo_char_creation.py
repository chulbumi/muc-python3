#!/usr/bin/env python3
"""
Live interactive demonstration of character creation flow
This script will connect and go through the full creation process automatically
"""

import socket
import time

def send_line(sock, msg):
    """Send a line to the server"""
    if msg:
        print(f"\n>>> SENDING: {repr(msg)}")
    sock.sendall((msg + "\n").encode('utf-8'))
    time.sleep(0.8)

def recv_data(sock, timeout=2):
    """Receive and print data"""
    sock.settimeout(timeout)
    data = b""
    start = time.time()

    while time.time() - start < timeout:
        try:
            chunk = sock.recv(8192)
            if chunk:
                data += chunk
                time.sleep(0.2)
            else:
                break
        except socket.timeout:
            break

    if data:
        text = data.decode('utf-8', errors='replace')
        # Print in a readable format
        lines = text.split('\n')
        for line in lines:
            # Strip ANSI codes for cleaner output
            clean = line
            import re
            ansi_escape = re.compile(r'\x1b\[[0-9;]*m')
            clean = ansi_escape.sub('', clean)
            if clean.strip():
                print(clean)

    return len(data) > 0

def main():
    print("=" * 80)
    print("MUD CHARACTER CREATION DEMONSTRATION")
    print("=" * 80)
    print("\n[Connecting to localhost:9900...]")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(60)

    try:
        sock.connect(('localhost', 9900))
        print("[Connected!]\n")

        # Get initial screen
        print("-" * 80)
        print("INITIAL SCREEN")
        print("-" * 80)
        recv_data(sock, timeout=3)

        # Enter name as "나만바라바" (fast path - skips long story)
        print("\n" + "-" * 80)
        print("STEP: Entering '나만바라바' (Quick Creation Path)")
        print("-" * 80)
        send_line(sock, "나만바라바")
        recv_data(sock, timeout=2)

        # Quick path should ask for name immediately
        print("\n" + "-" * 80)
        print("STEP: Entering character name '테스트'")
        print("-" * 80)
        send_line(sock, "테스트")
        recv_data(sock, timeout=2)

        # Password
        print("\n" + "-" * 80)
        print("STEP: Entering password")
        print("-" * 80)
        send_line(sock, "test1234")
        recv_data(sock, timeout=2)

        # Confirm password
        print("\n" + "-" * 80)
        print("STEP: Confirming password")
        print("-" * 80)
        send_line(sock, "test1234")
        recv_data(sock, timeout=2)

        # Gender
        print("\n" + "-" * 80)
        print("STEP: Selecting gender '남' (Male)")
        print("-" * 80)
        send_line(sock, "남")
        recv_data(sock, timeout=2)

        # Continue pressing Enter to go through tutorial
        print("\n" + "-" * 80)
        print("STEP: Going through tutorial (pressing Enter multiple times)")
        print("-" * 80)

        for i in range(30):
            print(f"\n[Enter #{i+1}]")
            send_line(sock, "")
            got_data = recv_data(sock, timeout=2)

            # Check if we need to execute specific commands
            time.sleep(0.5)

        print("\n" + "=" * 80)
        print("DEMONSTRATION COMPLETE")
        print("=" * 80)

    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()
    finally:
        sock.close()
        print("\n[Connection closed]")

if __name__ == "__main__":
    main()
