#!/usr/bin/env python3
"""
Test Python MUD server commands with proper CRLF line endings
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
    '능력치',
    '무공',
    '소지품',
    '점수',
    '누구',
    '봐',
    '말 hello',
    '지도',
]

def send_line(sock, line):
    """Send a line with CRLF ending"""
    sock.sendall((line + '\r\n').encode('utf-8'))

def receive_data(sock, timeout=2):
    """Receive data with timeout"""
    sock.settimeout(timeout)
    data = b''
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
            # Check for common prompt indicators
            decoded = data.decode('utf-8', errors='ignore')
            # Look for HP/MP prompt like "[ 450/900, 18/18 ]"
            if '[ ' in decoded and '/900' in decoded:
                time.sleep(0.3)
                try:
                    extra = sock.recv(4096)
                    if extra:
                        data += extra
                except socket.timeout:
                    pass
                break
            # If we see command prompt >
            if '>' in decoded:
                time.sleep(0.2)
                try:
                    extra = sock.recv(4096)
                    if extra:
                        data += extra
                except socket.timeout:
                    pass
                break
    except socket.timeout:
        pass
    return data

def test_mud_server():
    """Connect and test all commands"""
    output = []
    output.append("=" * 70)
    output.append("Python MUD Server Test Output - Port 9900")
    output.append(f"Character: {CHARACTER_NAME}")
    output.append(f"Time: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    output.append("=" * 70)
    output.append("")

    sock = None
    try:
        # Connect to server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(15)
        sock.connect((HOST, PORT))
        time.sleep(0.5)

        # Receive initial greeting
        output.append("=" * 70)
        output.append("STEP 1: Initial Connection")
        output.append("=" * 70)
        initial = receive_data(sock, timeout=3)
        output.append(initial.decode('utf-8', errors='replace'))
        output.append("")

        # Send character name
        output.append("=" * 70)
        output.append(f"STEP 2: Sending Character Name: {CHARACTER_NAME}")
        output.append("=" * 70)
        send_line(sock, CHARACTER_NAME)
        time.sleep(0.5)
        response = receive_data(sock, timeout=3)
        output.append(response.decode('utf-8', errors='replace'))
        output.append("")

        # Check if password is requested
        response_str = response.decode('utf-8', errors='replace')
        if '암호' in response_str or '비밀번호' in response_str or 'password' in response_str.lower():
            output.append("=" * 70)
            output.append("STEP 3: Sending Password")
            output.append("=" * 70)
            send_line(sock, PASSWORD)
            time.sleep(0.5)
            response = receive_data(sock, timeout=5)
            output.append(response.decode('utf-8', errors='replace'))
            output.append("")

        # Check for "엔터키를 누르세요" prompt
        if '엔터키를 누르세요' in response_str or '[엔터' in response_str:
            output.append("=" * 70)
            output.append("STEP 4: Pressing Enter to continue")
            output.append("=" * 70)
            send_line(sock, '')
            time.sleep(0.5)
            response = receive_data(sock, timeout=3)
            output.append(response.decode('utf-8', errors='replace'))
            output.append("")

        # Test each command
        for i, cmd in enumerate(COMMANDS, 1):
            output.append("=" * 70)
            output.append(f"COMMAND {i}: {cmd}")
            output.append("=" * 70)

            send_line(sock, cmd)
            time.sleep(0.5)
            response = receive_data(sock, timeout=3)
            output.append(response.decode('utf-8', errors='replace'))
            output.append("")

        # Send quit command
        send_line(sock, '종료')
        time.sleep(0.5)
        receive_data(sock, timeout=2)

    except Exception as e:
        output.append("")
        output.append("=" * 70)
        output.append(f"ERROR: {e}")
        output.append("=" * 70)
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
