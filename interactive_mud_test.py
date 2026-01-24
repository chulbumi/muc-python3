#!/usr/bin/env python3
"""
Interactive script to go through the entire character creation flow
"""

import socket
import time

def send_line(sock, message):
    """Send a line to the server"""
    print(f"[SENDING] {repr(message)}")
    sock.sendall(message.encode('utf-8') + b"\r\n")
    time.sleep(0.5)

def receive_all(sock, timeout=1):
    """Receive all available data"""
    sock.settimeout(timeout)
    data = b""
    start_time = time.time()

    while time.time() - start_time < timeout:
        try:
            chunk = sock.recv(4096)
            if chunk:
                data += chunk
                time.sleep(0.2)
            else:
                break
        except socket.timeout:
            break

    return data.decode('utf-8', errors='replace')

def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(60.0)

    try:
        print("=" * 80)
        print("MUD CHARACTER CREATION FLOW DOCUMENTATION")
        print("=" * 80)

        print("\n[1] CONNECTING...")
        sock.connect(('localhost', 9900))

        print("[2] RECEIVING INITIAL SCREEN...")
        data = receive_all(sock, timeout=3)
        print_data("INITIAL SCREEN", data)

        # Enter 무명객 to start creation
        print("\n[3] ENTERING '무명객' (GUEST)...")
        send_line(sock, "무명객")
        data = receive_all(sock, timeout=2)
        print_data("AFTER NAME", data)

        # The intro sequence will play, requiring multiple Enter presses
        # Let's press Enter a few times to advance the story
        for i in range(50):
            print(f"\n[{4+i}] PRESSING ENTER #{i+1}...")
            send_line(sock, "")
            data = receive_all(sock, timeout=2)
            print_data(f"AFTER ENTER #{i+1}", data)

            # If we see a prompt for name, password, etc., we need to respond
            if "무림존함" in data:
                print("\n*** NAME PROMPT DETECTED ***")
                send_line(sock, "테스트")
                data = receive_all(sock, timeout=2)
                print_data("AFTER NAME 입력", data)
            elif "존함암호" in data or "암호" in data:
                print("\n*** PASSWORD PROMPT DETECTED ***")
                send_line(sock, "test1234")
                data = receive_all(sock, timeout=2)
                print_data("AFTER PASSWORD", data)
            elif "암호확인" in data:
                print("\n*** PASSWORD CONFIRMATION PROMPT ***")
                send_line(sock, "test1234")
                data = receive_all(sock, timeout=2)
                print_data("AFTER PASSWORD CONFIRM", data)
            elif "성별" in data:
                print("\n*** GENDER PROMPT ***")
                send_line(sock, "남")
                data = receive_all(sock, timeout=2)
                print_data("AFTER GENDER", data)

            # If we see the game prompt, we're done
            if "[" in data and "/" in data and "," in data and "]" in data:
                if any(x in data for x in ["명령", ">", "무공"]):
                    print("\n*** ENTERED GAME ***")
                    break

            # If connection seems closed
            if not data and i > 5:
                print("\n*** NO MORE DATA RECEIVED ***")
                break

        print("\n" + "=" * 80)
        print("DOCUMENTATION COMPLETE")
        print("=" * 80)

    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()
    finally:
        sock.close()

def print_data(label, data):
    """Print received data"""
    print(f"\n--- {label} ---")
    print(f"LENGTH: {len(data)} bytes")
    if data:
        print("CONTENT:")
        print(data)
    else:
        print("(NO DATA)")
    print("-" * 40)

if __name__ == "__main__":
    main()
