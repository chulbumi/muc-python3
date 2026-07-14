#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Simple test script for testing MUD server commands.
This script connects to a running Rust MUD server and tests all commands.
"""

import os
import re
import time
import socket
import telnetlib
from pathlib import Path
from datetime import datetime
from typing import Optional, List, Tuple

# Configuration
RUST_PORT = 9999
TEST_PLAYER_NAME = "비교테스터"
TEST_PLAYER_PASS = "test1234"
RESPONSE_TIMEOUT = 2.0
COMMAND_DELAY = 0.3
WORK_DIR = Path("/home/ubuntu/muc-python3")

# ANSI code stripping pattern
ANSI_PATTERN = re.compile(r'\x1b\[[0-9;]*[mGKH]|\x1b\[[0-9;]*[m]|\r')

def strip_ansi(text: str) -> str:
    """Remove ANSI escape codes from text."""
    return ANSI_PATTERN.sub('', text)

def normalize_output(text: str) -> str:
    """Normalize output for comparison."""
    text = strip_ansi(text)
    text = re.sub(r'\s+', ' ', text)
    text = text.strip()
    return text

class SimpleMUDClient:
    """Simple MUD client for testing."""

    def __init__(self, host: str, port: int):
        self.host = host
        self.port = port
        self.conn: Optional[telnetlib.Telnet] = None

    def connect(self) -> bool:
        """Connect to server."""
        try:
            print(f"Connecting to {self.host}:{self.port}...")
            self.conn = telnetlib.Telnet(self.host, self.port, timeout=10)
            time.sleep(0.5)
            return True
        except Exception as e:
            print(f"Connection failed: {e}")
            return False

    def disconnect(self):
        """Disconnect from server."""
        if self.conn:
            try:
                self.conn.close()
            except:
                pass
            self.conn = None

    def send(self, text: str):
        """Send command."""
        if self.conn:
            try:
                self.conn.write((text + "\r\n").encode('utf-8'))
            except Exception as e:
                print(f"Send error: {e}")

    def receive_all(self, timeout: float = RESPONSE_TIMEOUT) -> str:
        """Receive all available data."""
        if not self.conn:
            return ""
        all_data = ""
        end_time = time.time() + timeout
        while time.time() < end_time:
            try:
                data = self.conn.read_very_eager()
                if data:
                    all_data += data.decode('utf-8', errors='ignore')
                else:
                    time.sleep(0.1)
            except:
                break
        return all_data

    def clear_buffer(self):
        """Clear buffer."""
        if self.conn:
            try:
                self.conn.read_very_eager()
            except:
                pass

def load_commands() -> List[str]:
    """Load all commands from cmds/*.rhai."""
    commands = []
    cmds_dir = WORK_DIR / "cmds"
    for rhai_file in sorted(cmds_dir.glob("*.rhai")):
        commands.append(rhai_file.stem)
    print(f"Loaded {len(commands)} commands")
    return commands

def login(client: SimpleMUDClient, name: str, password: str) -> bool:
    """Login to the server."""
    print(f"Logging in as {name}...")

    # Get initial welcome
    response = client.receive_all(2)
    print(f"Welcome: {normalize_output(response)[:100]}...")

    # Try to login
    client.send(name)
    time.sleep(0.5)
    response = client.receive_all(1)
    print(f"After name: {normalize_output(response)[:100]}...")

    # Enter password if prompted
    if "암호" in response or "password" in response.lower() or len(response) > 0:
        client.send(password)
        time.sleep(0.5)
        response = client.receive_all(1)
        print(f"After password: {normalize_output(response)[:100]}...")

    # Handle additional prompts
    for _ in range(3):
        client.send("")
        time.sleep(0.3)
        client.receive_all(0.5)

    return True

def test_command(client: SimpleMUDClient, command: str) -> Tuple[bool, str]:
    """Test a single command."""
    client.clear_buffer()
    client.send(command)
    time.sleep(COMMAND_DELAY)
    response = client.receive_all(RESPONSE_TIMEOUT)

    success = len(response) > 0
    normalized = normalize_output(response)

    return success, normalized

def main():
    """Main test function."""
    print("="*60)
    print("Rust MUD Server Command Test")
    print("="*60)

    # Load commands
    commands = load_commands()

    # Connect
    client = SimpleMUDClient("127.0.0.1", RUST_PORT)
    if not client.connect():
        print("Failed to connect!")
        return

    # Login
    login(client, TEST_PLAYER_NAME, TEST_PLAYER_PASS)

    # Test commands
    results = []
    print("\n" + "-"*60)
    print("Testing Commands")
    print("-"*60)

    for i, cmd in enumerate(commands):
        print(f"[{i+1}/{len(commands)}] Testing: {cmd:<25}", end=" ")
        success, output = test_command(client, cmd)

        if success:
            preview = output[:60] if len(output) > 60 else output
            print(f"OK - {preview}...")
            results.append((cmd, True, output))
        else:
            print("NO RESPONSE")
            results.append((cmd, False, ""))

        time.sleep(0.1)

    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)

    successful = sum(1 for _, s, _ in results if s)
    failed = sum(1 for _, s, _ in results if not s)

    print(f"Total: {len(results)}")
    print(f"Successful: {successful} ({100*successful/len(results):.1f}%)")
    print(f"Failed (no response): {failed}")

    # List failed commands
    if failed > 0:
        print("\nFailed commands:")
        for cmd, success, _ in results:
            if not success:
                print(f"  - {cmd}")

    # Save report
    report_file = WORK_DIR / f"rust_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.txt"
    with open(report_file, 'w', encoding='utf-8') as f:
        f.write("Rust MUD Server Command Test Report\n")
        f.write(f"Generated: {datetime.now()}\n")
        f.write("="*60 + "\n\n")

        for cmd, success, output in results:
            status = "OK" if success else "FAIL"
            f.write(f"[{status}] {cmd}\n")
            if output:
                f.write(f"  Output: {output[:200]}...\n")
            f.write("\n")

    print(f"\nReport saved to: {report_file}")

    # Disconnect
    client.disconnect()
    print("\nTest completed!")

if __name__ == "__main__":
    main()
