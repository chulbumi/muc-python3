#!/usr/bin/env python3
"""Debug script to trace what's happening with stat loading"""
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
            if b">> " in chunk:
                break
    except socket.timeout:
        pass
    return response.decode('utf-8', errors='ignore')

def test_stats():
    """Test character stats loading"""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 9999))

    # Login
    send_and_recv(sock, "\r\n")
    send_and_recv(sock, "테스트\r\n")
    response = send_and_recv(sock, "1234\r\n")

    # Enter game (if there's a notice)
    if "Enter" in response or "계속" in response:
        response = send_and_recv(sock, "\r\n")

    # Give time for login to complete
    time.sleep(1)

    # Check server logs - the issue is on the server side
    # Let's check what the body actually has
    print("Connected to server. Check server logs for debug output.")

    # Try a simple command
    response = send_and_recv(sock, "test\r\n")
    print(response[-200:])

    sock.close()

if __name__ == "__main__":
    test_stats()
