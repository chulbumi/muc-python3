#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Socket-based MUD Test for Python Server

This script uses raw socket to connect to Python MUD server,
creating a character via DOUMI system and running test commands.
"""

import socket
import time
import sys
from typing import List, Tuple, Dict

class SocketMUDTest:
    def __init__(self, host='localhost', port=9900):
        self.host = host
        self.port = port
        self.sock = None
        self.encoding = 'utf-8'
        self.connected = False

    def connect(self) -> bool:
        """Connect to the MUD server."""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.host, self.port))
            time.sleep(0.5)
            self.connected = True
            return True
        except Exception as e:
            print(f"Connection failed: {e}")
            return False

    def disconnect(self):
        """Disconnect from the server."""
        if self.sock:
            try:
                self.sock.close()
            except:
                pass
        self.connected = False

    def send(self, command: str):
        """Send a command to the server."""
        if not self.sock:
            return False
        try:
            cmd_bytes = command.encode(self.encoding) + b'\r\n'
            self.sock.sendall(cmd_bytes)
            return True
        except Exception as e:
            print(f"Send error: {e}")
            return False

    def recv(self, timeout=1.0) -> str:
        """Receive data from the server."""
        if not self.sock:
            return ""
        try:
            self.sock.settimeout(timeout)
            data = self.sock.recv(8192)
            return data.decode(self.encoding, errors='ignore')
        except socket.timeout:
            return ""
        except Exception as e:
            return ""

    def recv_all(self, max_reads=20, delay=0.1) -> str:
        """Receive all available data."""
        all_data = ""
        for _ in range(max_reads):
            try:
                self.sock.settimeout(0.1)
                data = self.sock.recv(4096)
                if not data:
                    break
                all_data += data.decode(self.encoding, errors='ignore')
            except:
                break
        return all_data

    def create_character_doumi(self, character_name: str) -> bool:
        """Create character using DOUMI 빠른도우미."""
        print(f"Creating character: {character_name}")

        # Step 1: Send 나만바라바 to trigger DOUMI
        self.send("나만바라바")
        time.sleep(0.5)

        response = self.recv_all()
        print(f"After 나만바라바: {len(response)} bytes")

        # Step 2: Send character name
        if '이름' in response or '무엇' in response:
            self.send(character_name)
            time.sleep(0.5)

            response = self.recv_all()
            print(f"After name: {len(response)} bytes")

            # Step 3: Look for DOUMI menu
            if '빠른도우미' in response:
                # Select option 1
                self.send("1")
                time.sleep(0.5)

                # Continue through DOUMI flow (press Enter for defaults)
                for i in range(20):
                    response = self.recv_all()

                    if '입장하셨습니다' in response or '낙양성' in response:
                        print("Character created successfully!")
                        return True

                    # If still in DOUMI menu or prompt, send Enter
                    self.send("")
                    time.sleep(0.2)

        print("Character creation completed")
        return True

    def test_commands(self) -> List[Tuple[str, str, int]]:
        """Run test commands and return results."""
        commands = [
            ('능력치', 'Stats command'),
            ('소지품', 'Inventory command'),
            ('봐', 'Look command'),
            ('저장', 'Save command'),
        ]

        results = []

        for cmd, desc in commands:
            print(f"Testing: {cmd}")
            self.send(cmd)
            time.sleep(0.5)

            response = self.recv_all()
            results.append((cmd, desc, len(response)))
            print(f"  Response: {len(response)} bytes")

        return results

    def run_test(self, character_name: str = "테스터"):
        """Run the full test."""
        print("=" * 60)
        print(f"Socket MUD Test - Python Server")
        print(f"Host: {self.host}:{self.port}")
        print("=" * 60)

        if not self.connect():
            print("Connection failed!")
            return False

        print("\nConnected to server")

        # Create character
        self.create_character_doumi(character_name)

        # Run tests
        print("\nRunning tests...")
        results = self.test_commands()

        # Print summary
        print("\n" + "=" * 60)
        print("Test Results:")
        print("=" * 60)

        total_bytes = 0
        for cmd, desc, bytes_received in results:
            status = "OK" if bytes_received > 0 else "FAIL"
            print(f"  {desc:30} {bytes_received:5d} bytes [{status}]")
            total_bytes += bytes_received

        print(f"\nTotal: {total_bytes} bytes received")

        self.disconnect()
        return total_bytes > 0


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description='Socket MUD Test')
    parser.add_argument('--host', default='localhost', help='Server host')
    parser.add_argument('--port', type=int, default=9900, help='Server port')
    parser.add_argument('--name', default='테스터', help='Character name')
    parser.add_argument('--verbose', '-v', action='store_true', help='Verbose output')

    args = parser.parse_args()

    test = SocketMUDTest(args.host, args.port)
    return 0 if test.run_test(args.name) else 1


if __name__ == '__main__':
    sys.exit(main())
