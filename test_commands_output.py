#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Test script to compare command outputs between Python (9900) and Rust (9999) servers
"""

import telnetlib
import time
import sys

def test_server(host, port, commands):
    """Connect to server, login, and run commands"""
    try:
        print(f"  Connecting to {host}:{port}...", end=" ", flush=True)
        tn = telnetlib.Telnet(host, port, timeout=10)
        print("Connected!")
        time.sleep(1)

        # Login sequence
        output = tn.read_very_eager().decode('euc-kr', errors='ignore')
        print(f"  Initial output: {repr(output[:200])}")

        # Send login name
        tn.write("테스터러스트\n".encode('euc-kr'))
        time.sleep(0.5)

        output = tn.read_very_eager().decode('euc-kr', errors='ignore')
        print(f"  After name output: {repr(output[:200])}")

        # Send password if needed
        if "비번" in output or "assword" in output or "암호" in output:
            tn.write("1234\n".encode('euc-kr'))
            time.sleep(0.5)
            output = tn.read_very_eager().decode('euc-kr', errors='ignore')
            print(f"  After password output: {repr(output[:200])}")

        # Clear buffer
        tn.write(b"\n")
        time.sleep(0.3)
        tn.read_very_eager()

        results = {}
        for cmd in commands:
            print(f"  Sending command: {cmd}")
            tn.write((cmd + "\n").encode('euc-kr'))
            time.sleep(1)
            output = tn.read_very_eager().decode('euc-kr', errors='ignore')
            results[cmd] = output
            print(f"    Output length: {len(output)} chars")

            tn.write(b"\n")
            time.sleep(0.3)
            tn.read_very_eager()

        tn.close()
        return results

    except Exception as e:
        print(f"  Error: {e}")
        import traceback
        traceback.print_exc()
        return {}


def main():
    commands = ["점수", "능력치", "무공"]

    print("=" * 80)
    print("Testing PYTHON Server (port 9900)")
    print("=" * 80)
    python_results = test_server("localhost", 9900, commands)

    print("\n" + "=" * 80)
    print("Testing RUST Server (port 9999)")
    print("=" * 80)
    rust_results = test_server("localhost", 9999, commands)

    # Compare results
    for cmd in commands:
        print("\n" + "=" * 80)
        print(f"COMMAND: {cmd}")
        print("=" * 80)

        python_output = python_results.get(cmd, "NO OUTPUT")
        rust_output = rust_results.get(cmd, "NO OUTPUT")

        print("\n--- PYTHON (9900) OUTPUT ---")
        print(python_output)
        print("\n--- RUST (9999) OUTPUT ---")
        print(rust_output)

        # Check if they match
        if python_output.strip() == rust_output.strip():
            print("\n✓ OUTPUTS MATCH")
        else:
            print("\n✗ OUTPUTS DIFFER")


if __name__ == "__main__":
    main()
