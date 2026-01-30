#!/usr/bin/env python3
"""
Create two test characters and test all MUD commands systematically.
Tests both Python MUD (9900) and Rust MUD (9990) for comparison.
"""

import socket
import time
import threading
import re
from dataclasses import dataclass
from typing import List, Tuple

@dataclass
class TestResult:
    server: str
    command: str
    output: str
    success: bool
    response_time: float

class MudTester:
    def __init__(self, host, port, name):
        self.host = host
        self.port = port
        self.name = name
        self.sock = None
        self.connected = False

    def connect(self):
        """Connect to MUD server"""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(5)
            self.sock.connect((self.host, self.port))
            time.sleep(0.5)
            self.connected = True
            return True
        except Exception as e:
            print(f"[{self.name}] Connection failed: {e}")
            return False

    def receive(self, timeout=0.5):
        """Receive data from server"""
        if not self.connected:
            return ""
        self.sock.setblocking(False)
        data = b""
        start = time.time()
        while time.time() - start < timeout:
            try:
                chunk = self.sock.recv(4096)
                if chunk:
                    data += chunk
                else:
                    break
            except:
                if data:
                    break
                time.sleep(0.05)
        self.sock.setblocking(True)
        return data.decode('utf-8', errors='ignore')

    def send(self, command):
        """Send command to server"""
        if not self.connected:
            return False
        try:
            self.sock.sendall((command + "\n").encode('utf-8'))
            time.sleep(0.3)
            return True
        except:
            return False

    def login(self, username, password="test1234"):
        """Login to MUD"""
        # Get initial banner
        initial = self.receive()

        # Send username
        self.send(username)
        response = self.receive()

        # Handle password/new char prompt
        if "암호" in response or "password" in response.lower():
            self.send(password)
            response = self.receive()
        elif "새" in response or "new" in response.lower() or "없는" in response:
            # New character - send password twice
            self.send(password)
            time.sleep(0.3)
            self.send(password)
            response = self.receive()

        # Try to enter game
        self.send("")
        time.sleep(0.3)
        return self.receive()

    def test_command(self, command):
        """Test a command and return result"""
        start = time.time()
        self.send(command)
        output = self.receive(timeout=0.5)
        elapsed = time.time() - start
        return output

    def disconnect(self):
        """Disconnect from server"""
        if self.sock:
            self.sock.close()
        self.connected = False

def clean_ansi(text):
    """Remove ANSI escape codes for comparison"""
    return re.sub(r'\x1b\[[0-9;]*[mHJK]', '', text)

def test_server(port, server_name, char_names):
    """Test a server with two characters"""
    results = []

    for i, char_name in enumerate(char_names, 1):
        print(f"\n{'='*60}")
        print(f"{server_name} - Testing Character {i}: {char_name}")
        print('='*60)

        tester = MudTester('localhost', port, f"{server_name}_Char{i}")

        if not tester.connect():
            continue

        # Login
        login_output = tester.login(char_name)
        print(f"Login response (first 200 chars):")
        print(clean_ansi(login_output)[:200])

        # Test commands systematically
        test_commands = [
            ("보기", "Look around"),
            ("인벤토리", "Show inventory"),
            ("무공", "Show skills"),
            ("비전", "Show vision"),
            ("상태", "Show status"),
            ("who", "Show who's online"),
            ("help", "Show help"),
            ("지도", "Show map"),
            ("8", "Move North (numeric)"),
            ("북", "Move North (Korean)"),
            ("2", "Move South"),
            ("남", "Move South (Korean)"),
            ("6", "Move East"),
            ("동", "Move East (Korean)"),
            ("4", "Move West"),
            ("서", "Move West (Korean)"),
            ("보기", "Look after movement"),
            ("말 테스트 메시지", "Say something"),
            ("외치기 테스트", "Shout"),
            ("비전목록", "Show learned visions"),
            ("비전수련", "Show vision training"),
        ]

        for cmd, description in test_commands:
            output = tester.test_command(cmd)
            clean = clean_ansi(output)

            # Extract key info from output
            if len(clean) > 50:
                # Look for specific patterns
                if "체력" in clean or "HP" in clean:
                    print(f"✓ {cmd:15} ({description}): Contains stats")
                elif "소지품" in clean or "인벤토리" in clean:
                    print(f"✓ {cmd:15} ({description}): Shows inventory")
                elif "무공" in clean or "스킬" in clean:
                    print(f"✓ {cmd:15} ({description}): Shows skills")
                elif "비전" in clean:
                    print(f"✓ {cmd:15} ({description}): Shows vision info")
                elif "출구" in clean or "Exits" in clean or "방" in clean:
                    print(f"✓ {cmd:15} ({description}): Shows room")
                elif "없습니다" in clean or "없음" in clean:
                    print(f"  {cmd:15} ({description}): Empty response")
                else:
                    # Show first 80 chars
                    preview = clean[:80].replace('\n', ' ').replace('\r', ' ')
                    print(f"  {cmd:15} ({description}): {preview}...")
            else:
                print(f"✗ {cmd:15} ({description}): No response")

            results.append(TestResult(
                server=server_name,
                command=cmd,
                output=clean[:200],  # Store first 200 chars
                success=len(clean) > 10,
                response_time=0
            ))

            time.sleep(0.2)

        tester.disconnect()
        time.sleep(0.5)

    return results

def compare_servers():
    """Test both servers and compare results"""
    char_names = ["테스터1", "테스터2"]

    print("="*60)
    print("MUD COMPARISON TEST - Two Characters")
    print("="*60)
    print(f"Characters: {', '.join(char_names)}")
    print(f"Python MUD: port 9900")
    print(f"Rust MUD: port 9990")

    # Test Python MUD
    py_results = test_server(9900, "Python", char_names)

    # Test Rust MUD
    rust_results = test_server(9990, "Rust", char_names)

    # Compare results
    print(f"\n{'='*60}")
    print("COMPARISON SUMMARY")
    print('='*60)

    # Count successful commands
    py_success = sum(1 for r in py_results if r.success)
    rust_success = sum(1 for r in rust_results if r.success)

    print(f"\nPython MUD: {py_success}/{len(py_results)} commands responded")
    print(f"Rust MUD: {rust_success}/{len(rust_results)} commands responded")

    # Compare specific commands
    print(f"\n{'Command':<20} {'Python':<10} {'Rust':<10} {'Match'}")
    print('-' * 50)

    for py_r, rust_r in zip(py_results, rust_results):
        if py_r.command == rust_r.command:
            match = "✓" if py_r.success == rust_r.success else "✗"
            py_status = "✓" if py_r.success else "✗"
            rust_status = "✓" if rust_r.success else "✗"
            print(f"{py_r.command:<20} {py_status:<10} {rust_status:<10} {match}")

if __name__ == "__main__":
    compare_servers()
