#!/usr/bin/env python3
"""
Automated character creation that follows the doumi.json script exactly
"""

import socket
import time
import json

def load_doumi_script():
    """Load the doumi script"""
    with open('data/config/doumi.json', 'r', encoding='utf-8') as f:
        data = json.load(f)
    return data['도우미메인설정']['초기도우미']

def send(sock, message):
    """Send message to server"""
    print(f"[SEND] {repr(message)}")
    sock.sendall(message.encode('utf-8') + b"\r\n")
    time.sleep(0.5)

def recv_all(sock, timeout=1):
    """Receive all available data"""
    sock.settimeout(timeout)
    data = b""
    start_time = time.time()

    while time.time() - start_time < timeout:
        try:
            chunk = sock.recv(8192)
            if chunk:
                data += chunk
                time.sleep(0.1)
            else:
                break
        except socket.timeout:
            break

    return data.decode('utf-8', errors='replace')

def main():
    script = load_doumi_script()

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(60.0)

    try:
        print("=" * 80)
        print("AUTOMATED CHARACTER CREATION")
        print("=" * 80)

        # Connect
        print("\n[CONNECTING]")
        sock.connect(('localhost', 9900))
        data = recv_all(sock, timeout=3)
        print_data("INITIAL", data)

        # Enter 무명객
        print("\n[ENTERING '무명객']")
        send(sock, "무명객")
        data = recv_all(sock, timeout=2)
        print_data("AFTER NAME", data)

        # Now follow the script
        # The script has special markers like $키입력, $키입력:command
        # We need to parse these and respond appropriately

        line_num = 0
        while line_num < len(script):
            line = script[line_num]
            print(f"\n[SCRIPT LINE {line_num}] {repr(line[:50])}")

            if line.startswith('$'):
                # Special command
                parts = line.strip().split()
                cmd = parts[0]

                if cmd == '$키입력':
                    # Just press Enter
                    print("  -> PRESSING ENTER")
                    send(sock, "")
                    data = recv_all(sock, timeout=2)
                    if data:
                        print_data("RESPONSE", data[:500])

                elif cmd.startswith('$키입력:'):
                    # Press specific command
                    command = line.split(':', 1)[1].strip()
                    print(f"  -> SENDING COMMAND: {command}")
                    send(sock, command)
                    data = recv_all(sock, timeout=2)
                    if data:
                        print_data("RESPONSE", data[:500])

                elif cmd == '$이름획득':
                    print("  -> NAME PROMPT")
                    send(sock, "테스트")
                    data = recv_all(sock, timeout=2)
                    print_data("AFTER NAME", data)

                elif cmd == '$암호획득':
                    print("  -> PASSWORD PROMPT")
                    send(sock, "test1234")
                    data = recv_all(sock, timeout=2)
                    print_data("AFTER PASSWORD", data)

                elif cmd == '$성별획득':
                    print("  -> GENDER PROMPT")
                    send(sock, "남")
                    data = recv_all(sock, timeout=2)
                    print_data("AFTER GENDER", data)

                elif cmd == '$틱:':
                    # Tick setting - just continue
                    print("  -> TICK SET (continuing)")
                    pass

                elif cmd in ['$출력시작', '$출력끝']:
                    # Output mode - just continue
                    print(f"  -> {cmd}")
                    pass

                line_num += 1

            elif line.startswith('\u001b['):
                # ANSI escape sequence - screen clear
                print("  -> SCREEN CLEAR/ANSI")
                data = recv_all(sock, timeout=2)
                if data:
                    print_data("AFTER ANSI", data[:300])
                line_num += 1

            elif line.strip() == '':
                # Empty line
                print("  -> EMPTY LINE")
                line_num += 1

            else:
                # Regular text output - server will send this
                print("  -> WAITING FOR OUTPUT")
                data = recv_all(sock, timeout=2)
                if data:
                    print_data("OUTPUT", data[:300])
                line_num += 1

            # Check if we've entered the game
            if data and ('[' in data and '/' in data and ']' in data):
                if '귀환' in data or '명령' in data or '무공' in data:
                    print("\n*** APPEARS TO BE IN GAME ***")
                    break

        print("\n" + "=" * 80)
        print("COMPLETE")
        print("=" * 80)

    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()
    finally:
        sock.close()

def print_data(label, data):
    """Print data"""
    if data:
        print(f"  [{label}] {len(data)} bytes")
        # Print first few lines
        lines = data.split('\r\n')[:5]
        for line in lines:
            print(f"    {line[:80]}")

if __name__ == "__main__":
    main()
