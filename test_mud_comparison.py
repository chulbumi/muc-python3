#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MUD Server Output Comparison Script
Compares outputs between Python (9900) and Rust (9999) MUD servers
"""

import asyncio
import telnetlib3
import re
import sys
from datetime import datetime

# ANSI escape code removal for comparison
ANSI_ESCAPE = re.compile(r'\x1b\[[0-9;]*m')

def strip_ansi(text):
    """Remove ANSI escape codes from text"""
    return ANSI_ESCAPE.sub('', text)

def normalize_output(text):
    """Normalize output for comparison"""
    text = strip_ansi(text)
    text = text.replace('\r\n', '\n')
    text = text.replace('\r', '\n')
    # Remove trailing whitespace
    lines = text.split('\n')
    lines = [line.rstrip() for line in lines]
    return '\n'.join(lines)

async def test_server(host, port, username, password, commands):
    """Connect to a server and execute commands"""
    results = []
    try:
        reader, writer = await telnetlib3.open_connection(host, port, encoding='euc-kr')

        # Wait for login prompt
        await asyncio.sleep(0.5)

        # Login sequence
        writer.write(f'{username}\n')
        await asyncio.sleep(0.3)
        writer.write(f'{password}\n')
        await asyncio.sleep(0.5)

        # Skip initial text
        while True:
            try:
                line = await asyncio.wait_for(reader.read(1024), timeout=0.2)
                if not line:
                    break
            except asyncio.TimeoutError:
                break

        # Execute each command and capture output
        for cmd in commands:
            # Send command
            writer.write(f'{cmd}\n')
            await asyncio.sleep(0.3)

            # Capture output
            output = ""
            max_reads = 10
            for _ in range(max_reads):
                try:
                    chunk = await asyncio.wait_for(reader.read(4096), timeout=0.2)
                    if chunk:
                        output += chunk
                    else:
                        break
                except asyncio.TimeoutError:
                    break

            results.append({
                'command': cmd,
                'output': output,
                'normalized': normalize_output(output)
            })

        writer.close()
        return results

    except Exception as e:
        print(f"Error connecting to {host}:{port} - {e}")
        return None

async def compare_servers():
    """Compare outputs between Python and Rust servers"""
    python_port = 9900
    rust_port = 9999
    host = 'localhost'

    # Test commands
    commands = ['능력치', '무공', '소지품', '누구', '봐', '말 테스트', '지도']

    # Test characters
    python_user = '테스터파이썬'
    rust_user = '테스터러스트'
    password = '1234'

    print("=" * 80)
    print("MUD Server Output Comparison Test")
    print(f"Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 80)
    print()

    # Test Python server
    print(f"Testing Python server on port {python_port}...")
    python_results = await test_server(host, python_port, python_user, password, commands)

    await asyncio.sleep(1)

    # Test Rust server
    print(f"Testing Rust server on port {rust_port}...")
    rust_results = await test_server(host, rust_port, rust_user, password, commands)

    if not python_results or not rust_results:
        print("Failed to connect to one or both servers")
        return

    # Compare results
    print("\n" + "=" * 80)
    print("COMPARISON RESULTS")
    print("=" * 80)

    report_lines = []
    report_lines.append(f"# MUD Server Output Comparison Report")
    report_lines.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report_lines.append("")
    report_lines.append("## Test Configuration")
    report_lines.append(f"- Python Server: localhost:{python_port}")
    report_lines.append(f"- Rust Server: localhost:{rust_port}")
    report_lines.append(f"- Python Character: {python_user}")
    report_lines.append(f"- Rust Character: {rust_user}")
    report_lines.append("")

    all_match = True

    for i, cmd in enumerate(commands):
        py_out = python_results[i]['normalized'] if i < len(python_results) else ""
        rust_out = rust_results[i]['normalized'] if i < len(rust_results) else ""

        # Basic comparison
        if py_out == rust_out:
            status = "✓ MATCH"
        else:
            status = "✗ DIFFER"
            all_match = False

        print(f"\n[{status}] Command: {cmd}")
        print("-" * 40)

        report_lines.append(f"## Command: {cmd}")
        report_lines.append(f"**Status:** {status}")
        report_lines.append("")

        if py_out != rust_out:
            # Show differences
            report_lines.append("### Python Output:")
            report_lines.append("```")
            py_lines = py_out.split('\n')
            for line in py_lines[:50]:  # Limit output
                report_lines.append(line)
            report_lines.append("```")
            report_lines.append("")

            report_lines.append("### Rust Output:")
            report_lines.append("```")
            rust_lines = rust_out.split('\n')
            for line in rust_lines[:50]:  # Limit output
                report_lines.append(line)
            report_lines.append("```")
            report_lines.append("")

            # Line-by-line comparison
            report_lines.append("### Differences:")
            py_lines = py_out.split('\n')
            rust_lines = rust_out.split('\n')
            max_lines = max(len(py_lines), len(rust_lines))

            diff_found = False
            for j in range(max_lines):
                py_line = py_lines[j] if j < len(py_lines) else "(missing)"
                rust_line = rust_lines[j] if j < len(rust_lines) else "(missing)"

                if py_line != rust_line:
                    diff_found = True
                    report_lines.append(f"Line {j+1}:")
                    report_lines.append(f"  Python: {py_line}")
                    report_lines.append(f"  Rust:   {rust_line}")

            if not diff_found:
                report_lines.append("(No differences found in line-by-line comparison)")
            report_lines.append("")
        else:
            report_lines.append("Outputs match perfectly.")
            report_lines.append("")

    print("\n" + "=" * 80)
    if all_match:
        print("ALL COMMANDS MATCH!")
    else:
        print("SOME COMMANDS HAVE DIFFERENCES")
    print("=" * 80)

    # Write report to file
    with open('/home/ubuntu/muc-python3/final_comparison_report.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(report_lines))

    print(f"\nReport saved to: /home/ubuntu/muc-python3/final_comparison_report.md")

if __name__ == '__main__':
    asyncio.run(compare_servers())
