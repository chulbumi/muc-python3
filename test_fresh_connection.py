#!/usr/bin/env python3
import socket
import time

def test_command(command, expected_in_output=None):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(10)
    sock.connect(('localhost', 9990))

    # Get banner
    time.sleep(1)
    sock.recv(8192)

    # Login as 점수
    sock.sendall("점수\r\n".encode('utf-8'))
    time.sleep(1)
    sock.recv(8192)

    # Empty password
    sock.sendall(b"\r\n")
    time.sleep(2)

    # Drain buffered data
    sock.settimeout(1)
    try:
        while True:
            chunk = sock.recv(8192)
            if not chunk:
                break
    except socket.timeout:
        pass

    # Send command
    sock.sendall((command + "\r\n").encode('utf-8'))
    time.sleep(3)

    # Read response
    sock.settimeout(3)
    all_data = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            all_data += chunk
    except socket.timeout:
        pass

    sock.close()

    output = all_data.decode('utf-8', errors='replace')
    if expected_in_output and expected_in_output in output:
        print(f"✓ {command}: Found expected output!")
        return True
    elif "오류" in output:
        print(f"✗ {command}: Error in output")
        for line in output.split('\n'):
            if "오류" in line:
                print(f"  {line}")
        return False
    else:
        print(f"✗ {command}: No expected output (got {len(all_data)} bytes)")
        # Show last 200 chars
        print(f"  Last 200 chars: {output[-200:]}")
        return False

# Test multiple commands
print("Testing commands:")
test_command("봐", "낙양성")  # Should show room
test_command("능력치", "┏")  # Should show score table
test_command("점수", "┏")    # Alias for 능력치
