#!/usr/bin/env python3
"""
Python vs Rust MUD Server Command Comparison Test
Tests all commands and compares outputs between the two servers.
"""

import socket
import time
import re
import os
import sys

def clean_ansi(text):
    """Remove ANSI escape codes from text"""
    ansi_escape = re.compile(r'\x1b\[[0-9;]*[a-zA-Z]|\[0m|\[\d+m|\[\d+;\d+m')
    return ansi_escape.sub('', text)

def recv_all(sock, timeout=2):
    """Receive all data with timeout"""
    sock.settimeout(timeout)
    data = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
    except socket.timeout:
        pass
    return data.decode('utf-8', errors='ignore')

def connect_to_server(port, username="비교테스터", password="비교테스터"):
    """Connect to a MUD server and login"""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', port))
    time.sleep(0.5)
    recv_all(s, timeout=2)  # Clear banner

    # Login
    s.sendall((username + "\n").encode('utf-8'))
    time.sleep(0.5)
    recv_all(s, timeout=1)

    s.sendall((password + "\n").encode('utf-8'))
    time.sleep(1)
    response = recv_all(s, timeout=2)

    return s, response

def send_command(sock, cmd, wait=1.5):
    """Send a command and get response"""
    sock.sendall((cmd + "\n").encode('utf-8'))
    time.sleep(wait)
    return recv_all(sock, timeout=2)

def normalize_output(text):
    """Normalize output for comparison"""
    # Remove ANSI codes
    text = clean_ansi(text)
    # Remove extra whitespace
    text = re.sub(r'\s+', ' ', text)
    # Remove HP/MP/Exp status lines
    text = re.sub(r'\[\s*\d+/\d+,\s*\d+/\d+\s*\]', '', text)
    # Strip leading/trailing whitespace
    text = text.strip()
    return text

def get_command_list():
    """Get list of all commands from cmds/*.rhai"""
    commands = []
    for f in os.listdir('cmds'):
        if f.endswith('.rhai'):
            cmd = f[:-5]  # Remove .rhai
            commands.append(cmd)
    return sorted(commands)

def main():
    print("=" * 60)
    print("Python vs Rust MUD Server Comparison Test")
    print("=" * 60)

    PYTHON_PORT = 9903
    RUST_PORT = 9999

    # Connect to both servers
    print("\n[1] Connecting to servers...")

    try:
        py_sock, py_resp = connect_to_server(PYTHON_PORT)
        print(f"  Python server (port {PYTHON_PORT}): Connected")
    except Exception as e:
        print(f"  Python server connection failed: {e}")
        return

    try:
        rs_sock, rs_resp = connect_to_server(RUST_PORT)
        print(f"  Rust server (port {RUST_PORT}): Connected")
    except Exception as e:
        print(f"  Rust server connection failed: {e}")
        py_sock.close()
        return

    # Get command list
    commands = get_command_list()
    print(f"\n[2] Found {len(commands)} commands to test")

    # Test results
    results = {
        'pass': 0,
        'fail': 0,
        'differences': []
    }

    print("\n[3] Testing commands...")
    print("-" * 60)

    for i, cmd in enumerate(commands):
        # Send command to both servers
        py_out = send_command(py_sock, cmd)
        rs_out = send_command(rs_sock, cmd)

        # Normalize outputs
        py_norm = normalize_output(py_out)
        rs_norm = normalize_output(rs_out)

        # Compare
        if py_norm == rs_norm:
            results['pass'] += 1
            status = "[PASS]"
        else:
            results['fail'] += 1
            status = "[FAIL]"
            results['differences'].append({
                'cmd': cmd,
                'python': py_norm[:200],
                'rust': rs_norm[:200]
            })

        # Print progress every 20 commands
        if (i + 1) % 20 == 0:
            print(f"  Tested {i+1}/{len(commands)} commands...")

    # Print results
    print("\n" + "=" * 60)
    print("RESULTS")
    print("=" * 60)
    print(f"Total commands: {len(commands)}")
    print(f"Passed: {results['pass']}")
    print(f"Failed: {results['fail']}")
    print(f"Match rate: {results['pass']/len(commands)*100:.1f}%")

    if results['differences']:
        print("\n" + "-" * 60)
        print("DIFFERENCES FOUND:")
        print("-" * 60)
        for diff in results['differences'][:20]:  # Show first 20
            print(f"\nCommand: {diff['cmd']}")
            print(f"  Python: {diff['python'][:100]}...")
            print(f"  Rust:   {diff['rust'][:100]}...")

    # Save full report
    report_path = 'comparison_report.txt'
    with open(report_path, 'w') as f:
        f.write("Python vs Rust MUD Server Comparison Report\n")
        f.write("=" * 60 + "\n\n")
        f.write(f"Total commands: {len(commands)}\n")
        f.write(f"Passed: {results['pass']}\n")
        f.write(f"Failed: {results['fail']}\n")
        f.write(f"Match rate: {results['pass']/len(commands)*100:.1f}%\n\n")

        if results['differences']:
            f.write("All Differences:\n")
            f.write("-" * 60 + "\n")
            for diff in results['differences']:
                f.write(f"\nCommand: {diff['cmd']}\n")
                f.write(f"Python: {diff['python']}\n")
                f.write(f"Rust:   {diff['rust']}\n")

    print(f"\nFull report saved to: {report_path}")

    # Cleanup
    py_sock.close()
    rs_sock.close()

    return results['fail'] == 0

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
