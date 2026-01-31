#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MUD Server Test using raw sockets
"""

import socket
import time
import re
import sys

ANSI_ESCAPE = re.compile(r'\x1b\[[0-9;]*m')

def strip_ansi(text):
    return ANSI_ESCAPE.sub('', text)

def normalize(text):
    text = strip_ansi(text)
    text = text.replace('\r\n', '\n')
    text = text.replace('\r', '\n')
    lines = text.split('\n')
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return '\n'.join(lines)

def recv_all(sock, timeout=0.5):
    """Receive all available data with timeout"""
    sock.setblocking(False)
    data = b''
    start = time.time()
    while time.time() - start < timeout:
        try:
            chunk = sock.recv(4096)
            if chunk:
                data += chunk
                start = time.time()  # Reset timeout on new data
            else:
                break
        except BlockingIOError:
            time.sleep(0.01)
    return data

def test_server_socket(host, port, username, password, commands):
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((host, port))
        sock.setblocking(True)

        # Wait for prompt
        time.sleep(0.3)
        recv_all(sock, 0.2)

        # Login
        sock.send((username + '\n').encode('euc-kr'))
        time.sleep(0.2)
        recv_all(sock, 0.2)

        sock.send((password + '\n').encode('euc-kr'))
        time.sleep(0.3)
        recv_all(sock, 0.3)

        results = []

        for cmd in commands:
            sock.send((cmd + '\n').encode('euc-kr'))
            time.sleep(0.3)

            data = recv_all(sock, 0.4)

            try:
                output = data.decode('euc-kr', errors='ignore')
            except:
                output = str(data)

            results.append({
                'command': cmd,
                'raw': output,
                'normalized': normalize(output)
            })

        sock.close()
        return results

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return None

def main():
    python_port = 9900
    rust_port = 9999
    host = 'localhost'

    commands = ['능력치', '무공', '소지품', '누구', '지도']

    python_user = '테스터파이썬'
    rust_user = '테스터러스트'
    password = '1234'

    print("=" * 80)
    print("MUD Server Comparison Test (socket)")
    print("=" * 80)

    print(f"\n[1/2] Testing Python server on port {python_port}...")
    py_results = test_server_socket(host, python_port, python_user, password, commands)

    if py_results:
        print(f"Got {len(py_results)} results")
        for r in py_results:
            print(f"  - {r['command']}: {len(r['raw'])} bytes, {len(r['normalized'])} chars normalized")

    time.sleep(1)

    print(f"\n[2/2] Testing Rust server on port {rust_port}...")
    rust_results = test_server_socket(host, rust_port, rust_user, password, commands)

    if rust_results:
        print(f"Got {len(rust_results)} results")
        for r in rust_results:
            print(f"  - {r['command']}: {len(r['raw'])} bytes, {len(r['normalized'])} chars normalized")

    if not py_results or not rust_results:
        print("\nFailed to get results")
        return

    print("\n" + "=" * 80)
    print("COMPARISON RESULTS")
    print("=" * 80)

    report = []
    report.append("# MUD Server Output Comparison Report")
    report.append(f"\nGenerated: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    report.append("\n## Configuration")
    report.append(f"- Python Server: localhost:{python_port}")
    report.append(f"- Rust Server: localhost:{rust_port}")
    report.append(f"- Python Character: {python_user}")
    report.append(f"- Rust Character: {rust_user}")
    report.append(f"\n## Commands: {', '.join(commands)}")
    report.append("\n" + "-" * 80 + "\n")

    all_match = True

    for i, cmd in enumerate(commands):
        py_norm = py_results[i]['normalized'] if i < len(py_results) else ""
        rust_norm = rust_results[i]['normalized'] if i < len(rust_results) else ""

        match = py_norm == rust_norm
        status = "✓ MATCH" if match else "✗ DIFFER"

        print(f"\n[{status}] {cmd}")

        report.append(f"### Command: `{cmd}`")
        report.append(f"**Status:** {status}\n")

        # Show raw output lengths
        py_raw_len = len(py_results[i]['raw']) if i < len(py_results) else 0
        rust_raw_len = len(rust_results[i]['raw']) if i < len(rust_results) else 0
        report.append(f"- Python raw: {py_raw_len} bytes")
        report.append(f"- Rust raw: {rust_raw_len} bytes")
        report.append(f"- Python normalized: {len(py_norm)} chars")
        report.append(f"- Rust normalized: {len(rust_norm)} chars\n")

        if not match:
            all_match = False

            report.append("**Python Output (normalized):**\n```\n")
            py_lines = py_norm.split('\n')
            for line in py_lines[:50]:
                report.append(line)
            report.append("\n```\n")

            report.append("**Rust Output (normalized):**\n```\n")
            rust_lines = rust_norm.split('\n')
            for line in rust_lines[:50]:
                report.append(line)
            report.append("\n```\n")

            # Line diff
            report.append("**Differences:**\n")
            max_lines = max(len(py_lines), len(rust_lines))
            for j in range(min(max_lines, 30)):
                pl = py_lines[j] if j < len(py_lines) else "(missing)"
                rl = rust_lines[j] if j < len(rust_lines) else "(missing)"
                if pl != rl:
                    report.append(f"Line {j+1}: Python={repr(pl)} Rust={repr(rl)}")
            report.append("\n")
        else:
            report.append("Outputs match.\n")

    print("\n" + "=" * 80)
    if all_match:
        print("✓ ALL COMMANDS MATCH!")
    else:
        print("✗ DIFFERENCES FOUND")
    print("=" * 80)

    with open('/home/ubuntu/muc-python3/final_comparison_report.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(report))

    print(f"\nReport: /home/ubuntu/muc-python3/final_comparison_report.md")

if __name__ == '__main__':
    main()
