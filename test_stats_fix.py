#!/usr/bin/env python3
"""Test script to verify character stat loading fix"""
import socket
import time

def send_and_recv(sock, data, timeout=2):
    """Send data and receive response"""
    sock.sendall(data.encode('utf-8'))
    time.sleep(0.3)
    response = b""
    sock.settimeout(timeout)
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            # Check if we have a complete response (looking for prompt)
            if b">> " in chunk or "암호:".encode('euc-kr') in chunk:
                break
    except socket.timeout:
        pass
    return response.decode('utf-8', errors='ignore')

def test_stats():
    """Test character stats loading"""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 9999))

    # Login
    response = send_and_recv(sock, "\r\n")
    print("--- After welcome ---")
    print(response[-500:])

    # Send name
    response = send_and_recv(sock, "테스트\r\n")
    print("--- After name ---")
    print(response[-500:])

    # Send password
    response = send_and_recv(sock, "1234\r\n")
    print("--- After password ---")
    print(response[-1000:])

    # Enter game (if there's a notice)
    if "아무거나" in response or "계속" in response or "Enter" in response:
        response = send_and_recv(sock, "\r\n")
        print("--- After continue ---")
        print(response[-500:])

    # Try ability command (능력치)
    time.sleep(0.5)
    response = send_and_recv(sock, "능력치\r\n")
    print("\n=== ABILITY STATS (능력치) ===")
    # Extract the stats table
    lines = response.split('\n')
    in_table = False
    for line in lines:
        if '━━━━' in line or '┏' in line:
            in_table = True
        if in_table:
            print(line)
        if '┕' in line and in_table:
            break

    # Also try score command
    time.sleep(0.3)
    response = send_and_recv(sock, "점수\r\n")
    print("\n=== SCORE (점수) ===")
    lines = response.split('\n')
    in_table = False
    for line in lines:
        if '━━━━' in line or '┏' in line:
            in_table = True
        if in_table:
            print(line)
        if '┕' in line and in_table:
            break

    sock.close()

if __name__ == "__main__":
    test_stats()
