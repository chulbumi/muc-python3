#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Comprehensive test script for comparing Python and Rust MUD server outputs.

This script:
1. Starts the Python MUD server on port 9998
2. Starts the Rust MUD server on port 9999
3. Connects test clients to both servers
4. Creates a test player with admin privileges
5. Tests all commands from cmds/*.rhai
6. Compares outputs between Python and Rust servers
7. Reports any differences found
"""

import os
import sys
import re
import time
import socket
import telnetlib
import subprocess
import signal
import json
import threading
from pathlib import Path
from datetime import datetime
from typing import Optional, Tuple, List, Dict

# Configuration
PYTHON_PORT = 9998
RUST_PORT = 9999
TEST_PLAYER_NAME = "테스터비교"
TEST_PLAYER_PASS = "test1234"
RESPONSE_TIMEOUT = 3.0
COMMAND_DELAY = 0.5
WORK_DIR = Path("/home/ubuntu/muc-python3")

# ANSI code stripping pattern
ANSI_PATTERN = re.compile(r'\x1b\[[0-9;]*[mGKH]|\x1b\[[0-9;]*[m]|\r|\n')

def strip_ansi(text: str) -> str:
    """Remove ANSI escape codes from text."""
    return ANSI_PATTERN.sub('', text)

def normalize_output(text: str) -> str:
    """Normalize output for comparison by removing ANSI codes and extra whitespace."""
    text = strip_ansi(text)
    # Normalize whitespace
    text = re.sub(r'\s+', ' ', text)
    text = text.strip()
    return text

class ServerProcess:
    """Manages a server subprocess."""

    def __init__(self, name: str, cmd: List[str], port: int, work_dir: Path):
        self.name = name
        self.cmd = cmd
        self.port = port
        self.work_dir = work_dir
        self.process: Optional[subprocess.Popen] = None
        self.log_file = None

    def start(self) -> bool:
        """Start the server process."""
        print(f"[{self.name}] Starting server on port {self.port}...")
        print(f"[{self.name}] Command: {' '.join(self.cmd)}")

        log_path = self.work_dir / f"server_{self.name.lower()}.log"
        self.log_file = open(log_path, 'w')

        try:
            self.process = subprocess.Popen(
                self.cmd,
                cwd=self.work_dir,
                stdout=self.log_file,
                stderr=subprocess.STDOUT,
                preexec_fn=os.setsid
            )

            # Wait for server to start
            max_wait = 10
            for i in range(max_wait):
                time.sleep(1)
                if self.is_port_open():
                    print(f"[{self.name}] Server started successfully on port {self.port}")
                    return True
                if self.process.poll() is not None:
                    print(f"[{self.name}] Server process exited with code {self.process.returncode}")
                    return False

            print(f"[{self.name}] Server did not start within {max_wait} seconds")
            return False

        except Exception as e:
            print(f"[{self.name}] Failed to start server: {e}")
            return False

    def is_port_open(self) -> bool:
        """Check if the server port is open."""
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(1)
            result = sock.connect_ex(('127.0.0.1', self.port))
            sock.close()
            return result == 0
        except:
            return False

    def stop(self):
        """Stop the server process."""
        if self.process:
            print(f"[{self.name}] Stopping server...")
            try:
                os.killpg(os.getpgid(self.process.pid), signal.SIGTERM)
                self.process.wait(timeout=5)
            except:
                try:
                    os.killpg(os.getpgid(self.process.pid), signal.SIGKILL)
                except:
                    pass
            self.process = None

        if self.log_file:
            self.log_file.close()
            self.log_file = None

class TestClient:
    """Test client for connecting to MUD server."""

    def __init__(self, name: str, host: str, port: int):
        self.name = name
        self.host = host
        self.port = port
        self.conn: Optional[telnetlib.Telnet] = None
        self.buffer = ""

    def connect(self) -> bool:
        """Connect to the server."""
        try:
            print(f"[{self.name}] Connecting to {self.host}:{self.port}...")
            self.conn = telnetlib.Telnet(self.host, self.port, timeout=10)
            time.sleep(0.5)
            return True
        except Exception as e:
            print(f"[{self.name}] Connection failed: {e}")
            return False

    def disconnect(self):
        """Disconnect from the server."""
        if self.conn:
            try:
                self.conn.close()
            except:
                pass
            self.conn = None

    def send(self, text: str):
        """Send a command to the server."""
        if self.conn:
            try:
                self.conn.write((text + "\r\n").encode('utf-8'))
            except Exception as e:
                print(f"[{self.name}] Send error: {e}")

    def receive(self, timeout: float = RESPONSE_TIMEOUT) -> str:
        """Receive data from the server."""
        if not self.conn:
            return ""

        try:
            data = self.conn.read_very_eager()
            if data:
                return data.decode('utf-8', errors='ignore')
        except:
            pass

        try:
            data = self.conn.read_until(b"\n", timeout=timeout)
            return data.decode('utf-8', errors='ignore')
        except:
            return ""

    def receive_all(self, timeout: float = RESPONSE_TIMEOUT) -> str:
        """Receive all available data with timeout."""
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
        """Clear any pending data."""
        if self.conn:
            try:
                self.conn.read_very_eager()
            except:
                pass

class CommandTestResult:
    """Stores the result of a command test."""

    def __init__(self, command: str):
        self.command = command
        self.python_output = ""
        self.rust_output = ""
        self.python_success = False
        self.rust_success = False
        self.match = False
        self.error = None

class ComparisonTest:
    """Main test comparison class."""

    def __init__(self):
        self.python_server: Optional[ServerProcess] = None
        self.rust_server: Optional[ServerProcess] = None
        self.python_client: Optional[TestClient] = None
        self.rust_client: Optional[TestClient] = None
        self.commands: List[str] = []
        self.results: List[CommandTestResult] = []
        self.test_player_created = False

    def load_commands(self) -> List[str]:
        """Load all commands from cmds/*.rhai files."""
        commands = []
        cmds_dir = WORK_DIR / "cmds"

        for rhai_file in sorted(cmds_dir.glob("*.rhai")):
            # Command name is the filename without extension
            cmd_name = rhai_file.stem
            commands.append(cmd_name)

        print(f"Loaded {len(commands)} commands from {cmds_dir}")
        return commands

    def categorize_commands(self) -> Dict[str, List[str]]:
        """Categorize commands by type."""
        categories = {
            'basic': [],
            'movement': [],
            'combat': [],
            'item': [],
            'admin': [],
            'social': [],
            'communication': [],
            'skill': [],
            'guild': [],
            'other': []
        }

        basic_keywords = ['look', 'help', 'say', '도움말', '봐', '말', '명령어리스트',
                         '안시', '설정', '상태보기', '점수', '소지품', 'inventory',
                         '장비', '숙련도', '무공상태']

        movement_keywords = ['이동', '귀환', '점프', '앞', '올려', '내려', '어디',
                            '지도', '맵', '자동경로', '위치각인', '추적']

        combat_keywords = ['attack', '도망', '죽여', '쳐', '시전', '무공', '회복',
                          '분노', '자동무공', '호위']

        item_keywords = ['가져', '먹어', '버려', '입어', '벗어', '구입', '판매',
                        '장비', '소지품', '아이템', '생성', '부셔', '분해',
                        '똥파말', '낚시', '대여', '반납', '세트', '옵설정',
                        '옵랜덤', '비교', '넣어', '꺼내']

        admin_keywords = ['생성', '삭제', '제거', '설정', '초기화', '리부팅',
                         '리젠', '소환', '몹', '방', '출구', '이동동', '이동이동',
                         '값', '순위', '업데이트', '저장', '오브젝트', '정리',
                         '투명', '체인지', '디버그', '테스트명령', 'test',
                         'master', '소각', '특정방파', '순위초기화', '몹회복',
                         '기연', '이벤트', '명칭', '방파초기화']

        social_keywords = ['표현', '소소', '기부', '쉬어', '일어나', '낚시',
                          '무릉별호', '성올려']

        communication_keywords = ['say', '외쳐', '전음', '반전음', '쪽지', '공지',
                                 '지난', '채널', '무리말', '방파말', '꼬리말',
                                 '머리말', '줄임말', 'comm', '트윗']

        skill_keywords = ['무공', '시전', '내공주입', '비전', '무공전수', '조제',
                         '이형환위']

        guild_keywords = ['방파', '입문', '직위', '방주권한', '문파', '무리']

        for cmd in self.commands:
            cmd_lower = cmd.lower()
            categorized = False

            if any(kw in cmd_lower for kw in basic_keywords):
                categories['basic'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in movement_keywords):
                categories['movement'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in combat_keywords):
                categories['combat'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in item_keywords):
                categories['item'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in admin_keywords):
                categories['admin'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in social_keywords):
                categories['social'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in communication_keywords):
                categories['communication'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in skill_keywords):
                categories['skill'].append(cmd)
                categorized = True
            if any(kw in cmd_lower for kw in guild_keywords):
                categories['guild'].append(cmd)
                categorized = True

            if not categorized:
                categories['other'].append(cmd)

        return categories

    def setup_servers(self) -> bool:
        """Start both Python and Rust servers."""
        # Create test player data file first
        self.create_test_player_data()

        # Start Python server on port 9900 (hardcoded in server.py)
        # We'll use the default port since modifying it would require code changes
        global PYTHON_PORT
        PYTHON_PORT = 9900  # Python server uses hardcoded port

        python_cmd = [sys.executable, "server.py"]

        self.python_server = ServerProcess(
            "Python",
            python_cmd,
            PYTHON_PORT,
            WORK_DIR
        )

        # Start Rust server on port 9999
        rust_binary = WORK_DIR / "target" / "release" / "murim_server"
        if not rust_binary.exists():
            print(f"Rust server binary not found: {rust_binary}")
            return False

        # Rust server accepts port via command line argument
        self.rust_server = ServerProcess(
            "Rust",
            [str(rust_binary), str(RUST_PORT)],
            RUST_PORT,
            WORK_DIR
        )

        # Start servers
        if not self.python_server.start():
            print("Failed to start Python server")
            return False

        if not self.rust_server.start():
            print("Failed to start Rust server")
            self.python_server.stop()
            return False

        return True

    def create_test_player_data(self):
        """Create test player data file with admin privileges."""
        player_data = {
            "사용자오브젝트": {
                "이름": TEST_PLAYER_NAME,
                "암호": TEST_PLAYER_PASS,
                "성별": "남",
                "나이": 25,
                "레벨": 100,
                "체력": 1000,
                "최대체력": 1000,
                "최고체력": 1000,
                "내공": 5000,
                "최고내공": 5000,
                "최대내공": 5000,
                "힘": 100,
                "맷집": 100,
                "민첩성": 100,
                "지력": 100,
                "운": 100,
                "현재경험치": 1000000,
                "은전": 100000,
                "금전": 1000,
                "위치": "낙양성:42",
                "현재방": "낙양성:42",
                "귀환지맵": "낙양성:42",
                "관리자등급": 2000,
                "무공이름": [],
                "무공숙련도": [],
                "아이템": []
            }
        }

        player_file = WORK_DIR / "data" / "user" / f"{TEST_PLAYER_NAME}.json"
        with open(player_file, 'w', encoding='utf-8') as f:
            json.dump(player_data, f, ensure_ascii=False, indent=2)

        print(f"Created test player data: {player_file}")

    def setup_clients(self) -> bool:
        """Connect test clients to both servers."""
        self.python_client = TestClient("Python", "127.0.0.1", 9900)  # Python uses hardcoded port
        self.rust_client = TestClient("Rust", "127.0.0.1", RUST_PORT)

        if not self.python_client.connect():
            return False

        if not self.rust_client.connect():
            self.python_client.disconnect()
            return False

        # Wait for welcome message
        time.sleep(1)
        self.python_client.receive_all(2)
        self.rust_client.receive_all(2)

        return True

    def login_player(self) -> bool:
        """Login with test player on both servers."""
        print(f"Logging in as {TEST_PLAYER_NAME}...")

        for client in [self.python_client, self.rust_client]:
            # Clear buffer
            client.clear_buffer()

            # Try to create new character first (in case server expects that)
            client.send(TEST_PLAYER_NAME)
            time.sleep(0.5)
            response = client.receive_all(1)

            # Check if we need to enter password
            if "암호" in response or "password" in response.lower():
                client.send(TEST_PLAYER_PASS)
                time.sleep(0.5)
                response = client.receive_all(1)

            # Handle any additional prompts
            if "다시입력" in response or "confirm" in response.lower():
                client.send(TEST_PLAYER_PASS)
                time.sleep(0.5)
                response = client.receive_all(1)

            # Clear any remaining prompts
            for _ in range(3):
                client.send("")
                time.sleep(0.3)
                client.receive_all(0.5)

        print("Login completed")
        self.test_player_created = True
        return True

    def test_command(self, command: str) -> CommandTestResult:
        """Test a single command on both servers and compare outputs."""
        result = CommandTestResult(command)

        try:
            # Clear buffers
            self.python_client.clear_buffer()
            self.rust_client.clear_buffer()

            # Send command to both servers
            self.python_client.send(command)
            self.rust_client.send(command)

            # Wait for responses
            time.sleep(COMMAND_DELAY)

            # Receive responses
            result.python_output = self.python_client.receive_all(RESPONSE_TIMEOUT)
            result.rust_output = self.rust_client.receive_all(RESPONSE_TIMEOUT)

            # Check for success indicators
            result.python_success = len(result.python_output) > 0
            result.rust_success = len(result.rust_output) > 0

            # Normalize and compare outputs
            py_normalized = normalize_output(result.python_output)
            rust_normalized = normalize_output(result.rust_output)

            result.match = py_normalized == rust_normalized

        except Exception as e:
            result.error = str(e)

        return result

    def run_tests(self):
        """Run all command tests."""
        print("\n" + "="*60)
        print("Starting Command Comparison Tests")
        print("="*60)

        categories = self.categorize_commands()

        # Test commands by category
        test_order = ['basic', 'movement', 'item', 'skill', 'combat',
                     'communication', 'social', 'guild', 'admin', 'other']

        for category in test_order:
            cmds = categories.get(category, [])
            if not cmds:
                continue

            print(f"\n--- Testing {category.upper()} commands ({len(cmds)} total) ---")

            for cmd in cmds:
                print(f"  Testing: {cmd}", end=" ")

                result = self.test_command(cmd)
                self.results.append(result)

                if result.match:
                    print("PASS (match)")
                elif result.python_success and result.rust_success:
                    print("DIFF (outputs differ)")
                elif result.python_success:
                    print("RUST_FAIL (no output)")
                elif result.rust_success:
                    print("PYTHON_FAIL (no output)")
                else:
                    print("BOTH_FAIL (no output)")

                # Small delay between commands
                time.sleep(0.2)

    def generate_report(self) -> str:
        """Generate a detailed test report."""
        report = []
        report.append("\n" + "="*60)
        report.append("TEST COMPARISON REPORT")
        report.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        report.append("="*60)

        # Summary statistics
        total = len(self.results)
        matched = sum(1 for r in self.results if r.match)
        python_only = sum(1 for r in self.results if r.python_success and not r.rust_success)
        rust_only = sum(1 for r in self.results if r.rust_success and not r.python_success)
        both_failed = sum(1 for r in self.results if not r.python_success and not r.rust_success)
        diff = total - matched - python_only - rust_only - both_failed

        report.append(f"\nSUMMARY:")
        report.append(f"  Total commands tested: {total}")
        report.append(f"  Matched outputs: {matched} ({100*matched/total:.1f}%)")
        report.append(f"  Different outputs: {diff} ({100*diff/total:.1f}%)")
        report.append(f"  Python only (Rust failed): {python_only}")
        report.append(f"  Rust only (Python failed): {rust_only}")
        report.append(f"  Both failed: {both_failed}")

        # Detailed differences
        report.append("\n" + "-"*60)
        report.append("DETAILED DIFFERENCES:")
        report.append("-"*60)

        for result in self.results:
            if not result.match and result.python_success and result.rust_success:
                report.append(f"\nCommand: {result.command}")
                report.append(f"  Python output ({len(result.python_output)} chars):")
                py_preview = normalize_output(result.python_output)[:200]
                report.append(f"    {py_preview}")
                report.append(f"  Rust output ({len(result.rust_output)} chars):")
                rust_preview = normalize_output(result.rust_output)[:200]
                report.append(f"    {rust_preview}")

        # Failed commands
        report.append("\n" + "-"*60)
        report.append("COMMANDS WITH ERRORS:")
        report.append("-"*60)

        for result in self.results:
            if result.error:
                report.append(f"\n{result.command}: {result.error}")

        return "\n".join(report)

    def save_report(self, report: str):
        """Save report to file."""
        report_file = WORK_DIR / f"test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.txt"
        with open(report_file, 'w', encoding='utf-8') as f:
            f.write(report)
        print(f"\nReport saved to: {report_file}")

    def cleanup(self):
        """Clean up resources."""
        print("\nCleaning up...")

        if self.python_client:
            self.python_client.disconnect()
        if self.rust_client:
            self.rust_client.disconnect()

        if self.python_server:
            self.python_server.stop()
        if self.rust_server:
            self.rust_server.stop()

        # Remove test player data
        player_file = WORK_DIR / "data" / "user" / f"{TEST_PLAYER_NAME}.json"
        if player_file.exists():
            player_file.unlink()
            print(f"Removed test player data: {player_file}")

    def run(self):
        """Main test execution."""
        try:
            # Load commands
            self.commands = self.load_commands()
            if not self.commands:
                print("No commands found to test!")
                return

            # Setup and start servers
            print("\nSetting up servers...")
            if not self.setup_servers():
                print("Failed to setup servers!")
                return

            # Setup clients
            print("\nSetting up clients...")
            if not self.setup_clients():
                print("Failed to setup clients!")
                return

            # Login
            print("\nLogging in test player...")
            if not self.login_player():
                print("Failed to login!")
                return

            # Run tests
            self.run_tests()

            # Generate and save report
            report = self.generate_report()
            print(report)
            self.save_report(report)

        except KeyboardInterrupt:
            print("\nTest interrupted by user")
        except Exception as e:
            print(f"\nTest failed with error: {e}")
            import traceback
            traceback.print_exc()
        finally:
            self.cleanup()


def main():
    """Main entry point."""
    print("="*60)
    print("MUD Server Comparison Test")
    print("Python server: port 9998")
    print("Rust server: port 9999")
    print("="*60)

    test = ComparisonTest()
    test.run()

    print("\nTest completed!")


if __name__ == "__main__":
    main()
