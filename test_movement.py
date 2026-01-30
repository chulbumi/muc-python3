#!/usr/bin/env python3
import socket
import time

def send_and_receive(sock, cmd, wait=0.3):
    sock.sendall((cmd + "\n").encode('utf-8'))
    time.sleep(wait)
    sock.setblocking(False)
    data = b""
    start = time.time()
    while time.time() - start < 0.5:
        try:
            chunk = sock.recv(4096)
            if chunk:
                data += chunk
            else:
                break
        except:
            break
    sock.setblocking(True)
    return data.decode('utf-8', errors='ignore')

def test_movement(port, name):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(3)
    sock.connect(('localhost', port))
    time.sleep(0.5)
    
    # Get initial
    sock.setblocking(False)
    data = b""
    start = time.time()
    while time.time() - start < 0.5:
        try:
            data += sock.recv(4096)
        except:
            if data: break
    sock.setblocking(True)
    
    # Login
    send_and_receive(sock, "테스터")
    send_and_receive(sock, "")
    send_and_receive(sock, "")
    
    # Test 보기 first
    print(f"\n=== {name} - Initial Room (보기) ===")
    response = send_and_receive(sock, "보기")
    # Clean and print key parts
    for line in response.split('\n'):
        if '출구' in line or 'Exits' in line or '보기' in line or '☞' in line:
            print(line)

    # Test movement - directional commands
    directions = [
        ("북", "North"),
        ("남", "South"),
        ("동", "East"),
        ("서", "West"),
    ]
    
    for cmd, eng in directions:
        response = send_and_receive(sock, cmd)
        # Check if movement succeeded
        if "이동" in response or "갈" in response or "없" in response:
            print(f"{cmd} ({eng}): {response[:100]}")
        else:
            # Maybe room description
            if len(response) > 50:
                print(f"{cmd} ({eng}): Room changed (response: {len(response)} chars)")
    
    sock.close()

test_movement(9900, "Python")
test_movement(9990, "Rust")
