#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MUD Server Test using telnetlib (standard library)
"""

import telnetlib
import time
import re
import sys

ANSI_ESCAPE = re.compile(r'\x1b\[[0-9;]*m')

def strip_ansi(text):
    """Remove ANSI escape codes"""
    return ANSI_ESCAPE.sub('', text)

def normalize(text):
    """Normalize text for comparison"""
    text = strip_ansi(text)
    text = text.replace('\r\n', '\n')
    text = text.replace('\r', '\n')
    lines = text.split('\n')
    # Remove empty lines at start and end
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return '\n'.join(lines)

def test_server_telnetlib(host, port, username, password, commands):
    """Test server using telnetlib"""
    try:
        tn = telnetlib.Telnet(host, port, timeout=5)

        # Wait for login prompt
        time.sleep(0.5)

        # Login
        tn.write(username.encode('euc-kr') + b'\n')
        time.sleep(0.3)

        tn.write(password.encode('euc-kr') + b'\n')
        time.sleep(0.5)

        # Skip welcome text
        tn.read_very_eager()

        results = []

        for cmd in commands:
            # Send command
            tn.write(cmd.encode('euc-kr') + b'\n')
            time.sleep(0.4)

            # Read output
            output = tn.read_very_eager().decode('euc-kr', errors='ignore')
            results.append({
                'command': cmd,
                'raw': output,
                'normalized': normalize(output)
            })

        tn.close()
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
    print("MUD Server Comparison Test (telnetlib)")
    print("=" * 80)

    # Test Python server
    print(f"\n[1/2] Testing Python server on port {python_port}...")
    py_results = test_server_telnetlib(host, python_port, python_user, password, commands)

    if py_results:
        print(f"Got {len(py_results)} results from Python server")
        for r in py_results:
            print(f"  - {r['command']}: {len(r['raw'])} bytes")

    time.sleep(1)

    # Test Rust server
    print(f"\n[2/2] Testing Rust server on port {rust_port}...")
    rust_results = test_server_telnetlib(host, rust_port, rust_user, password, commands)

    if rust_results:
        print(f"Got {len(rust_results)} results from Rust server")
        for r in rust_results:
            print(f"  - {r['command']}: {len(r['raw'])} bytes")

    if not py_results or not rust_results:
        print("\nFailed to get results from one or both servers")
        return

    # Compare
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
    report.append(f"\n## Commands Tested: {', '.join(commands)}")
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

        if not match:
            all_match = False

            # Show Python output
            report.append("**Python Output (normalized):**\n```\n")
            py_lines = py_norm.split('\n')
            for line in py_lines[:100]:
                report.append(line)
            report.append("\n```\n")

            # Show Rust output
            report.append("**Rust Output (normalized):**\n```\n")
            rust_lines = rust_norm.split('\n')
            for line in rust_lines[:100]:
                report.append(line)
            report.append("\n```\n")

            # Line-by-line diff
            report.append("**Differences:**\n")
            max_lines = max(len(py_lines), len(rust_lines))
            for j in range(min(max_lines, 50)):
                pl = py_lines[j] if j < len(py_lines) else "(missing)"
                rl = rust_lines[j] if j < len(rust_lines) else "(missing)"
                if pl != rl:
                    report.append(f"Line {j+1}:")
                    report.append(f"  Python: {repr(pl)}")
                    report.append(f"  Rust:   {repr(rl)}")

            if len(py_lines) != len(rust_lines):
                report.append(f"\nLength: Python={len(py_lines)}, Rust={len(rust_lines)}")

            report.append("\n")
        else:
            report.append("Outputs match.\n")

    print("\n" + "=" * 80)
    if all_match:
        print("✓ ALL COMMANDS MATCH!")
    else:
        print("✗ DIFFERENCES FOUND")
    print("=" * 80)

    # Write report
    with open('/home/ubuntu/muc-python3/final_comparison_report.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(report))

    print(f"\nReport: /home/ubuntu/muc-python3/final_comparison_report.md")

if __name__ == '__main__':
    main()
