#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Detailed MUD Server Output Test with Full Output Display
"""

import asyncio
import telnetlib3
import re
from datetime import datetime

ANSI_ESCAPE = re.compile(r'\x1b\[[0-9;]*m')

def strip_ansi(text):
    """Remove ANSI escape codes from text for comparison"""
    return ANSI_ESCAPE.sub('', text)

async def test_server_detailed(host, port, username, password, commands):
    """Connect to a server and execute commands, return raw and stripped output"""
    results = []
    try:
        reader, writer = await telnetlib3.open_connection(host, port, encoding='euc-kr')

        await asyncio.sleep(0.5)

        # Login
        writer.write(f'{username}\n')
        await asyncio.sleep(0.3)
        writer.write(f'{password}\n')
        await asyncio.sleep(0.5)

        # Clear initial buffer
        while True:
            try:
                line = await asyncio.wait_for(reader.read(1024), timeout=0.2)
                if not line:
                    break
            except asyncio.TimeoutError:
                break

        # Execute commands
        for cmd in commands:
            writer.write(f'{cmd}\n')
            await asyncio.sleep(0.3)

            output = ""
            for _ in range(15):
                try:
                    chunk = await asyncio.wait_for(reader.read(8192), timeout=0.2)
                    if chunk:
                        output += chunk
                    else:
                        break
                except asyncio.TimeoutError:
                    break

            results.append({
                'command': cmd,
                'raw': output,
                'stripped': strip_ansi(output),
                'lines': output.split('\r\n') if '\r\n' in output else output.split('\n')
            })

        writer.close()
        return results

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return None

def print_section(title, content, width=80):
    """Print a section with a header"""
    print("\n" + "=" * width)
    print(f" {title}")
    print("=" * width)
    print(content)
    print("=" * width)

async def main():
    python_port = 9900
    rust_port = 9999
    host = 'localhost'

    commands = ['능력치', '무공', '소지품', '누구', '지도']

    python_user = '테스터파이썬'
    rust_user = '테스터러스트'
    password = '1234'

    print("=" * 80)
    print("DETAILED MUD SERVER OUTPUT COMPARISON")
    print(f"Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 80)

    print("\n[1/2] Connecting to Python server...")
    python_results = await test_server_detailed(host, python_port, python_user, password, commands)

    await asyncio.sleep(1)

    print("[2/2] Connecting to Rust server...")
    rust_results = await test_server_detailed(host, rust_port, rust_user, password, commands)

    if not python_results or not rust_results:
        print("Failed to connect to servers")
        return

    # Generate detailed report
    report_lines = []
    report_lines.append("# MUD Server Output Comparison - Detailed Report")
    report_lines.append(f"\nGenerated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report_lines.append("\n## Test Configuration")
    report_lines.append(f"- Python Server: localhost:{python_port}")
    report_lines.append(f"- Rust Server: localhost:{rust_port}")
    report_lines.append(f"- Python Character: {python_user}")
    report_lines.append(f"- Rust Character: {rust_user}")
    report_lines.append(f"\n## Commands Tested: {', '.join(commands)}")
    report_lines.append("\n" + "-" * 80 + "\n")

    all_match = True

    for i, cmd in enumerate(commands):
        py_raw = python_results[i]['raw'] if i < len(python_results) else ""
        rust_raw = rust_results[i]['raw'] if i < len(rust_results) else ""
        py_stripped = python_results[i]['stripped'] if i < len(python_results) else ""
        rust_stripped = rust_results[i]['stripped'] if i < len(rust_results) else ""

        # Compare stripped outputs
        match = py_stripped == rust_stripped
        status = "✓ MATCH" if match else "✗ DIFFER"

        print(f"\n[{status}] Command: {cmd}")

        report_lines.append(f"### Command: `{cmd}`")
        report_lines.append(f"**Status:** {status}\n")

        if not match:
            all_match = False
            report_lines.append("**Python Output (stripped):**\n```\n")
            report_lines.append(py_stripped)
            report_lines.append("\n```\n")
            report_lines.append("**Rust Output (stripped):**\n```\n")
            report_lines.append(rust_stripped)
            report_lines.append("\n```\n")

            # Line-by-line diff
            py_lines = py_stripped.split('\n')
            rust_lines = rust_stripped.split('\n')
            report_lines.append("**Line-by-line differences:**\n")

            for j, (pl, rl) in enumerate(zip(py_lines, rust_lines), 1):
                if pl != rl:
                    report_lines.append(f"- Line {j}:")
                    report_lines.append(f"  - Python: `{repr(pl)}`")
                    report_lines.append(f"  - Rust:   `{repr(rl)}`")

            # Handle different length
            if len(py_lines) != len(rust_lines):
                report_lines.append(f"- Length mismatch: Python={len(py_lines)} lines, Rust={len(rust_lines)} lines")
                if len(py_lines) > len(rust_lines):
                    for j in range(len(rust_lines), len(py_lines)):
                        report_lines.append(f"  - Python line {j+1} (missing in Rust): `{repr(py_lines[j])}`")
                else:
                    for j in range(len(py_lines), len(rust_lines)):
                        report_lines.append(f"  - Rust line {j+1} (missing in Python): `{repr(rust_lines[j])}`")

            report_lines.append("\n")
        else:
            report_lines.append("Outputs match perfectly.\n")

    print("\n" + "=" * 80)
    if all_match:
        print("RESULT: ALL COMMAND OUTPUTS MATCH!")
    else:
        print("RESULT: DIFFERENCES FOUND - Check report")
    print("=" * 80)

    # Write report
    with open('/home/ubuntu/muc-python3/final_comparison_report.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(report_lines))

    print(f"\nReport saved: /home/ubuntu/muc-python3/final_comparison_report.md")

if __name__ == '__main__':
    asyncio.run(main())
