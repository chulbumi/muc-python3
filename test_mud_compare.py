#!/usr/bin/env python3
"""
Test script to compare Python MUD (localhost:9900) with Rust MUD (localhost:9990)
"""

import socket
import time
import sys

# ANSI color codes to strip for comparison
ANSI_ESCAPE = '\x1b['

def strip_ansi(text):
    """Remove ANSI escape codes"""
    import re
    return re.sub(r'\x1b\[[0-9;]*m', '', text)

def connect_to_mud(host, port):
    """Connect to MUD server"""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5)
    try:
        sock.connect((host, port))
        return sock
    except Exception as e:
        print(f"Failed to connect to {host}:{port} - {e}")
        return None

def send_command(sock, command):
    """Send command to MUD server"""
    try:
        sock.sendall((command + "\n").encode('utf-8', errors='ignore'))
        time.sleep(0.3)  # Wait for response
        response = sock.recv(8192).decode('utf-8', errors='ignore')
        return response
    except Exception as e:
        return f"Error: {e}"

def get_response(sock):
    """Get current response buffer"""
    try:
        sock.setblocking(False)
        response = sock.recv(8192).decode('utf-8', errors='ignore')
        sock.setblocking(True)
        return response
    except:
        sock.setblocking(True)
        return ""

def test_login(sock, username):
    """Test login flow"""
    print(f"\n=== Testing login for {username} ===")

    # Get initial screen
    initial = send_command(sock, "")
    print("Initial screen (first 500 chars):")
    print(strip_ansi(initial)[:500])

    # Send username
    send_command(sock, username)
    time.sleep(0.5)

    # Check for password prompt
    response = get_response(sock)
    if "암호" in response or "password" in response.lower():
        print("Password prompt detected")

        # For new character, might need to create
        send_command(sock, "newpass123")
        time.sleep(0.5)

    # Try to enter game
    send_command(sock, "")
    time.sleep(0.5)

    return get_response(sock)

def test_basic_commands(python_sock, rust_sock):
    """Test basic MUD commands and compare outputs"""

    commands_to_test = [
        ("보기", "look around"),
        ("지도", "show map"),
        ("인벤토리", "show inventory"),
        ("무공", "show skills"),
        ("비전", "show vision"),
        ("상태", "show status"),
        ("통신", "show communication"),
        ("help", "show help"),
        ("who", "show who's online"),
    ]

    results = []

    for cmd, description in commands_to_test:
        print(f"\n{'='*60}")
        print(f"Testing: {description} ({cmd})")
        print('='*60)

        # Test Python
        python_response = send_command(python_sock, cmd)
        python_clean = strip_ansi(python_response)

        # Test Rust
        rust_response = send_command(rust_sock, cmd)
        rust_clean = strip_ansi(rust_response)

        # Compare
        print(f"\nPython response (first 300 chars):")
        print(python_clean[:300])

        print(f"\nRust response (first 300 chars):")
        print(rust_clean[:300])

        # Store comparison result
        match = "SIMILAR" if python_clean[:100] == rust_clean[:100] else "DIFFERENT"
        results.append((cmd, description, match))

        time.sleep(0.3)

    return results

def main():
    print("MUD Comparison Test")
    print("=" * 60)

    # Connect to Python MUD (port 9900)
    print("\nConnecting to Python MUD (localhost:9900)...")
    python_sock = connect_to_mud('localhost', 9900)

    # Connect to Rust MUD (port 9990)
    print("Connecting to Rust MUD (localhost:9990)...")
    rust_sock = connect_to_mud('localhost', 9990)

    if not python_sock:
        print("Failed to connect to Python MUD")
        return
    if not rust_sock:
        print("Failed to connect to Rust MUD")
        return

    print("Connected to both servers!")

    # Test character 1 on Python
    print("\n" + "="*60)
    print("Creating Character 1 on Python MUD")
    print("="*60)
    test_login(python_sock, "테스터1")

    # Test character 2 on Rust
    print("\n" + "="*60)
    print("Creating Character 2 on Rust MUD")
    print("="*60)
    test_login(rust_sock, "테스터2")

    # Test basic commands
    print("\n" + "="*60)
    print("Testing Basic Commands")
    print("="*60)
    results = test_basic_commands(python_sock, rust_sock)

    # Print summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    for cmd, desc, match in results:
        print(f"{cmd:15} ({desc:20}): {match}")

    # Close connections
    python_sock.close()
    rust_sock.close()

if __name__ == "__main__":
    main()
