#!/usr/bin/env python3
"""
Interactive test for Python MUD server with proper timing
"""

import socket
import time
import sys

HOST = 'localhost'
PORT = 9900
CHARACTER_NAME = '테스터파이썬'
PASSWORD = '1234'

# Commands to test
COMMANDS = [
    ('능력치', 2),
    ('무공', 2),
    ('소지품', 2),
    ('점수', 2),
    ('누구', 2),
    ('봐', 2),
    ('말 hello', 2),
    ('지도', 2),
]

def send_and_receive(sock, data, timeout=3):
    """Send data and receive response"""
    sock.settimeout(timeout)
    sock.sendall((data + '\n').encode('utf-8'))
    time.sleep(0.3)

    response = b''
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            # Check if we have a prompt
            if b'>' in response:
                time.sleep(0.2)
                try:
                    extra = sock.recv(4096)
                    if extra:
                        response += extra
                except socket.timeout:
                    pass
                break
    except socket.timeout:
        pass

    return response.decode('utf-8', errors='replace')

def test_mud_server():
    """Connect and test all commands"""
    output = []
    output.append("=" * 60)
    output.append("Python MUD Server Test Output - Port 9900")
    output.append(f"Character: {CHARACTER_NAME}")
    output.append(f"Time: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    output.append("=" * 60)
    output.append("")

    sock = None
    try:
        # Connect to server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(10)
        sock.connect((HOST, PORT))
        time.sleep(1)

        # Receive initial greeting
        sock.settimeout(5)
        initial = sock.recv(4096)
        output.append("=" * 60)
        output.append("STEP 1: Initial Connection")
        output.append("=" * 60)
        output.append(initial.decode('utf-8', errors='replace'))
        output.append("")

        # Send character name
        output.append("=" * 60)
        output.append(f"STEP 2: Sending Character Name: {CHARACTER_NAME}")
        output.append("=" * 60)
        response = send_and_receive(sock, CHARACTER_NAME, timeout=5)
        output.append(response)
        output.append("")

        # Check if password is requested
        if '암호' in response or '비밀번호' in response or 'password' in response.lower():
            output.append("=" * 60)
            output.append("STEP 3: Sending Password")
            output.append("=" * 60)
            response = send_and_receive(sock, PASSWORD, timeout=5)
            output.append(response)
            output.append("")
        else:
            output.append("(No password requested)")
            output.append("")

        # Test each command
        for i, (cmd, wait_time) in enumerate(COMMANDS, 1):
            output.append("=" * 60)
            output.append(f"COMMAND {i}: {cmd}")
            output.append("=" * 60)

            response = send_and_receive(sock, cmd, timeout=wait_time + 1)
            output.append(response)
            output.append("")

        # Send quit
        send_and_receive(sock, '종료', timeout=2)

    except Exception as e:
        output.append("")
        output.append("=" * 60)
        output.append(f"ERROR: {e}")
        output.append("=" * 60)
        import traceback
        output.append(traceback.format_exc())

    finally:
        if sock:
            try:
                sock.close()
            except:
                pass

    return '\n'.join(output)

if __name__ == '__main__':
    result = test_mud_server()

    # Save to file
    output_file = '/home/ubuntu/muc-python3/python_test_output.md'
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(result)

    print(result)
    print(f"\n\nOutput saved to: {output_file}")
