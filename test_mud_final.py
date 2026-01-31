#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Final MUD comparison test using telnetlib3
"""

import asyncio
import sys

async def test_mud(host, port, username, password, commands):
    """Test a MUD server"""
    try:
        import telnetlib3

        reader, writer = await telnetlib3.open_connection(
            host, port, encoding='euc-kr'
        )

        # Wait for login
        await asyncio.sleep(1)

        # Send username
        writer.write(username + '\n')
        await asyncio.sleep(0.5)

        # Send password
        writer.write(password + '\n')
        await asyncio.sleep(1)

        # Clear any remaining output
        try:
            await asyncio.wait_for(reader.read(4096), timeout=0.5)
        except:
            pass

        outputs = {}

        for cmd in commands:
            # Send command
            writer.write(cmd + '\n')
            await asyncio.sleep(0.5)

            # Read response
            output = ""
            for _ in range(10):
                try:
                    chunk = await asyncio.wait_for(reader.read(8192), timeout=0.2)
                    if chunk:
                        output += chunk
                    else:
                        break
                except asyncio.TimeoutError:
                    break

            outputs[cmd] = output
            await asyncio.sleep(0.2)

        writer.close()
        return outputs

    except Exception as e:
        print(f"Error testing {host}:{port} - {e}")
        import traceback
        traceback.print_exc()
        return {}

async def main():
    python_port = 9900
    rust_port = 9999
    host = 'localhost'

    commands = ['능력치', '무공', '소지품', '누구', '지도']

    python_user = '테스터파이썬'
    rust_user = '테스터러스트'
    password = '1234'

    print("=" * 70)
    print("MUD Server Output Comparison")
    print("=" * 70)
    print()

    # Test Python
    print(f"[1/2] Testing Python server on port {python_port}...")
    py_outputs = await test_mud(host, python_port, python_user, password, commands)
    print(f"Got {len(py_outputs)} responses")

    await asyncio.sleep(1)

    # Test Rust
    print(f"[2/2] Testing Rust server on port {rust_port}...")
    rust_outputs = await test_mud(host, rust_port, rust_user, password, commands)
    print(f"Got {len(rust_outputs)} responses")

    print()
    print("=" * 70)
    print("Comparison Results")
    print("=" * 70)
    print()

    import re
    ansi = re.compile(r'\x1b\[[0-9;]*m')

    def strip_ansi(text):
        return ansi.sub('', text)

    report_lines = []
    report_lines.append("# MUD Server Output Comparison Report\n")
    report_lines.append(f"Generated: {asyncio.get_event_loop().time()}\n")
    report_lines.append(f"- Python Server: localhost:{python_port}\n")
    report_lines.append(f"- Rust Server: localhost:{rust_port}\n")
    report_lines.append(f"- Python Character: {python_user}\n")
    report_lines.append(f"- Rust Character: {rust_user}\n")
    report_lines.append("\n---\n\n")

    all_match = True

    for cmd in commands:
        py_out = py_outputs.get(cmd, "")
        rust_out = rust_outputs.get(cmd, "")

        py_clean = strip_ansi(py_out).strip()
        rust_clean = strip_ansi(rust_out).strip()

        match = py_clean == rust_clean
        status = "MATCH" if match else "DIFFER"

        print(f"[{status}] {cmd}")
        print(f"  Python: {len(py_out)} bytes, {len(py_clean)} chars (stripped)")
        print(f"  Rust:   {len(rust_out)} bytes, {len(rust_clean)} chars (stripped)")

        report_lines.append(f"### Command: {cmd}\n")
        report_lines.append(f"**Status:** {status}\n")
        report_lines.append(f"- Python: {len(py_out)} bytes\n")
        report_lines.append(f"- Rust: {len(rust_out)} bytes\n\n")

        if not match and py_clean and rust_clean:
            all_match = False
            report_lines.append("**Python Output:**\n```\n")
            report_lines.append(py_clean[:500] + ("..." if len(py_clean) > 500 else ""))
            report_lines.append("\n```\n\n")
            report_lines.append("**Rust Output:**\n```\n")
            report_lines.append(rust_clean[:500] + ("..." if len(rust_clean) > 500 else ""))
            report_lines.append("\n```\n\n")

            # Show first differences
            py_lines = py_clean.split('\n')
            rust_lines = rust_clean.split('\n')
            for i, (pl, rl) in enumerate(zip(py_lines, rust_lines)):
                if pl != rl:
                    report_lines.append(f"First difference at line {i+1}:\n")
                    report_lines.append(f"- Python: {pl[:100]}\n")
                    report_lines.append(f"- Rust:   {rl[:100]}\n\n")
                    break
        elif match and py_clean:
            report_lines.append("Outputs match.\n\n")
        else:
            if not py_clean and not rust_clean:
                report_lines.append("(No output captured from either server)\n\n")
            elif not py_clean:
                report_lines.append("(No output from Python server)\n\n")
                all_match = False
            else:
                report_lines.append("(No output from Rust server)\n\n")
                all_match = False

        print()

    print("=" * 70)
    if all_match:
        print("ALL COMMANDS MATCH!")
    else:
        print("SOME DIFFERENCES FOUND")
    print("=" * 70)

    # Write report
    with open('/home/ubuntu/muc-python3/final_comparison_report.md', 'w', encoding='utf-8') as f:
        f.write(''.join(report_lines))

    print(f"\nReport: /home/ubuntu/muc-python3/final_comparison_report.md")

if __name__ == '__main__':
    asyncio.run(main())
