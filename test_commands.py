#!/usr/bin/env python3
"""Test specific commands and compare outputs"""
import socket
import time
import subprocess
import sys

def run_command_sequence(port, commands):
    """Run a sequence of commands and return output"""
    results = []
    for cmd in commands:
        try:
            result = subprocess.run(
                ['bash', '-c', f'echo "{cmd}" | nc -w 1 localhost {port}'],
                capture_output=True,
                text=True,
                timeout=3
            )
            results.append((cmd, result.stdout))
        except Exception as e:
            results.append((cmd, f"Error: {e}"))
    return results

# Commands to test
test_commands = [
    "무명객",  # Login as guest
]

# Test both servers
print("Testing command responses on both servers...")
for port, name in [(9900, "Python"), (9990, "Rust")]:
    print(f"\n{'='*50}\n{name} MUD (port {port})\n{'='*50}")
    results = run_command_sequence(port, test_commands)
    for cmd, output in results:
        # Clean ANSI
        lines = []
        for line in output.split('\n'):
            if '무명' in line or '없는' in line or '갈' in line or '출구' in line:
                lines.append(line[:80])
        if lines:
            print(f"Command '{cmd}': {lines[:3]}")
