#!/usr/bin/env python3
import socket
import time
import re

def clean_ansi(text):
    return re.sub(r'\x1b\[\?[0-9;]*[A-Za-z]', '', re.sub(r'\x1b\[[0-9;]*[mHJK]', '', text))

def test_port(port, name):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(3)
    sock.connect(('localhost', port))
    time.sleep(0.5)
    data = sock.recv(4096).decode('utf-8', errors='ignore')
    print(f"\n=== {name} (port {port}) ===")
    print("Initial:", clean_ansi(data)[:150])
    
    sock.sendall("테스터\n".encode('utf-8'))
    time.sleep(0.3)
    data = sock.recv(4096).decode('utf-8', errors='ignore')
    print("After name:", clean_ansi(data)[:200])
    
    sock.sendall("\n".encode('utf-8'))
    time.sleep(0.3)
    data = sock.recv(4096).decode('utf-8', errors='ignore')
    print("After enter:", clean_ansi(data)[:200])
    sock.close()

test_port(9900, "Python")
test_port(9990, "Rust")
