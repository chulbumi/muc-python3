#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test Rust server with UTF-8"""

import socket
import time

IAC = b'\xff'

def handle_iac(data):
    response = b""
    processed = b""
    i = 0
    while i < len(data):
        if data[i:i+1] == IAC and i + 1 < len(data):
            cmd = data[i+1:i+2]
            if cmd in [b'\xf1', b'\xf2', b'\xf3', b'\xf4', b'\xf5', b'\xf6', b'\xf7', b'\xf8', b'\xf9']:
                i += 2
            elif cmd in [b'\xfb', b'\xfc', b'\xfd', b'\xfe']:
                if i + 2 < len(data):
                    opt = data[i+2:i+3]
                    if cmd == b'\xfb':  # WILL
                        response += IAC + b'\xfc' + opt  # WONT
                    elif cmd == b'\xfd':  # DO
                        response += IAC + b'\xfc' + opt  # WONT
                    i += 3
                else:
                    i += 2
            else:
                i += 2
        else:
            processed += data[i:i+1]
            i += 1
    return processed, response

def test_rust_utf8():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(10)
    sock.connect(('localhost', 9999))

    data = sock.recv(8192)
    print(f"Banner: {len(data)} bytes")
    processed, response = handle_iac(data)
    if response:
        sock.send(response)

    time.sleep(0.5)

    # Send username in UTF-8
    sock.send('테스터러스트'.encode('utf-8') + b'\r\n')
    time.sleep(0.5)

    data = sock.recv(4096)
    print(f"After username: {len(data)} bytes")
    processed, response = handle_iac(data)
    print(f"UTF-8 decoded: {processed.decode('utf-8', errors='ignore')[:100]}")
    if response:
        sock.send(response)

    # Check for password
    pw_check = processed.decode('utf-8', errors='ignore')
    if b'assword' in data or '암호' in pw_check:
        sock.send('1234\r\n'.encode('utf-8'))
        time.sleep(0.5)
        data = sock.recv(4096)
        print(f"After password: {len(data)} bytes")
        processed, response = handle_iac(data)

    time.sleep(1)

    # Send command
    cmd = '능력치'
    sock.send(cmd.encode('utf-8') + b'\r\n')
    time.sleep(1)

    data = sock.recv(8192)
    print(f"After command: {len(data)} bytes")
    processed, response = handle_iac(data)

    output = processed.decode('utf-8', errors='ignore')
    print(f"Output ({len(output)} chars):")
    print(output[:500])

    sock.close()

test_rust_utf8()
