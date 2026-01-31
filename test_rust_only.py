#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test just Rust server to see what output we get"""

import socket
import time
import re

IAC = b'\xff'
DO = b'\xfd'
DONT = b'\xfe'
WILL = b'\xfb'
WONT = b'\xfc'
SE = b'\xf0'
NOP = b'\xf1'
SB = b'\xfa'

def handle_iac(data):
    response = b""
    processed = b""
    i = 0
    while i < len(data):
        if data[i:i+1] == IAC and i + 1 < len(data):
            cmd = data[i+1:i+2]
            if cmd in [NOP]:
                i += 2
            elif cmd in [WILL, WONT, DO, DONT]:
                if i + 2 < len(data):
                    opt = data[i+2:i+3]
                    if cmd == WILL:
                        response += IAC + DONT + opt
                    elif cmd == DO:
                        response += IAC + WONT + opt
                    i += 3
                else:
                    i += 2
            elif cmd == SB:
                j = i + 2
                while j < len(data):
                    if data[j:j+1] == IAC and j + 1 < len(data) and data[j+1:j+2] == SE:
                        j += 2
                        break
                    j += 1
                i = j
            else:
                i += 2
        else:
            processed += data[i:i+1]
            i += 1
    return processed, response

def test_rust():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(10)
    sock.connect(('localhost', 9999))

    # Read banner
    data = sock.recv(8192)
    print(f"Banner: {len(data)} bytes")
    processed, response = handle_iac(data)
    print(f"Processed: {len(processed)} bytes")
    if response:
        sock.send(response)

    time.sleep(0.5)

    # Send username
    sock.send('테스터러스트'.encode('euc-kr') + b'\r\n')
    time.sleep(0.5)

    data = sock.recv(4096)
    print(f"After username: {len(data)} bytes")
    processed, response = handle_iac(data)
    print(f"Content preview: {processed[:100]}")
    if response:
        sock.send(response)

    # Send password
    sock.send('1234\r\n'.encode('euc-kr'))
    time.sleep(0.5)

    data = sock.recv(4096)
    print(f"After password: {len(data)} bytes")
    processed, response = handle_iac(data)
    if response:
        sock.send(response)

    # Newlines to get to prompt
    for _ in range(2):
        sock.send(b'\r\n')
        time.sleep(0.3)
        data = sock.recv(4096)
        processed, response = handle_iac(data)
        if response:
            sock.send(response)

    # Send command
    cmd = '능력치'
    sock.send(cmd.encode('euc-kr') + b'\r\n')
    time.sleep(1)

    data = sock.recv(8192)
    print(f"After command: {len(data)} bytes")
    processed, response = handle_iac(data)
    if response:
        sock.send(response)

    output = processed.decode('euc-kr', errors='ignore')
    print(f"Decoded output ({len(output)} chars):")
    print(repr(output[:500]))

    sock.close()

test_rust()
