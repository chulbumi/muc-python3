#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Comprehensive MUD Test Script

This script tests both Python (port 9900) and Rust (port 9999) MUD servers.
It supports creating characters with Korean names, executing commands,
capturing responses, comparing outputs, and testing various scenarios.

Features:
- Connect to both Python and Rust servers
- Create test characters with Korean names
- Execute commands and capture responses
- Compare outputs between servers
- Test multiple scenarios in sequence
- Handle reconnections and error recovery
- Generate detailed test reports

Usage:
    python3 test_mud_comprehensive.py [options]

    Options:
        --host HOST         Server host (default: localhost)
        --py-port PORT      Python server port (default: 9900)
        --rust-port PORT    Rust server port (default: 9999)
        --test TEST         Specific test to run (basic, movement, combat, items, npc, all)
        --chars N           Number of test characters (default: 2)
        --report FILE       Report file path (default: test_results.md)
        --verbose           Enable verbose output
        --no-report         Skip generating report
"""

import telnetlib
import time
import re
import json
import sys
import os
from datetime import datetime
from typing import Optional, Dict, List, Tuple, Any
from dataclasses import dataclass, field, asdict
from enum import Enum


# ============================================================================
# Configuration
# ============================================================================

class ServerType(Enum):
    """Enum for server types"""
    PYTHON = "Python"
    RUST = "Rust"


@dataclass
class TestConfig:
    """Configuration for test runs"""
    host: str = "localhost"
    py_port: int = 9900
    rust_port: int = 9999
    num_characters: int = 2
    base_password: str = "test1234"
    encoding: str = "euc-kr"
    connection_timeout: int = 15
    command_timeout: int = 5
    report_path: str = "/home/ubuntu/muc-python3/test_results.md"
    verbose: bool = False


@dataclass
class TestResult:
    """Data class for test results"""
    server_type: ServerType
    character_name: str
    command: str
    success: bool
    output: str
    output_length: int
    timestamp: str
    error_message: str = ""
    execution_time: float = 0.0


@dataclass
class ComparisonResult:
    """Data class for comparison results"""
    command: str
    py_output: str
    rust_output: str
    match: bool
    differences: List[str] = field(default_factory=list)
    keywords_match: Dict[str, bool] = field(default_factory=dict)


@dataclass
class TestReport:
    """Data class for complete test report"""
    test_start: str
    test_end: str
    total_tests: int
    passed_tests: int
    failed_tests: int
    test_results: List[TestResult] = field(default_factory=list)
    comparison_results: List[ComparisonResult] = field(default_factory=list)
    server_status: Dict[str, bool] = field(default_factory=dict)


# ============================================================================
# ANSI Code Utilities
# ============================================================================

def strip_ansi_codes(text: str) -> str:
    """
    Remove ANSI escape codes from text.

    Args:
        text: Text containing ANSI codes

    Returns:
        Clean text without ANSI codes
    """
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    return ansi_escape.sub('', text)


def normalize_whitespace(text: str) -> str:
    """
    Normalize whitespace in text for comparison.

    Args:
        text: Input text

    Returns:
        Text with normalized whitespace
    """
    return re.sub(r'\s+', ' ', text.strip())


# ============================================================================
# Connection Management
# ============================================================================

class MUDConnection:
    """
    Handles connection to a MUD server.

    This class manages telnet connections, login flow, character creation,
    and command execution for MUD servers.
    """

    def __init__(self, config: TestConfig, server_type: ServerType, port: int,
                 character_name: Optional[str] = None):
        """
        Initialize a MUD connection.

        Args:
            config: Test configuration
            server_type: Type of server (Python or Rust)
            port: Port number to connect to
            character_name: Name of the character to use/create
        """
        self.config = config
        self.server_type = server_type
        self.host = config.host
        self.port = port
        self.character_name = character_name or f"테스터{server_type.value}"
        self.password = config.base_password
        self.tn: Optional[telnetlib.Telnet] = None
        self.connected = False
        self.logged_in = False
        self.buffer = ""

    def connect(self) -> bool:
        """
        Establish connection to the MUD server.

        Returns:
            True if connection successful, False otherwise
        """
        try:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Connecting to {self.host}:{self.port}...")

            self.tn = telnetlib.Telnet(self.host, self.port, timeout=self.config.connection_timeout)
            self.connected = True

            # Wait for initial banner
            time.sleep(1)
            self.buffer = self._read_output()

            if self.config.verbose:
                print(f"[{self.server_type.value}] Connected successfully")

            return True

        except Exception as e:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Connection failed: {e}")
            self.connected = False
            return False

    def disconnect(self) -> None:
        """Disconnect from the MUD server."""
        if self.tn:
            try:
                self.tn.close()
            except Exception:
                pass
        self.connected = False
        self.logged_in = False

    def reconnect(self) -> bool:
        """
        Reconnect to the server.

        Returns:
            True if reconnection successful, False otherwise
        """
        self.disconnect()
        time.sleep(1)
        return self.connect()

    def _read_output(self, timeout: Optional[float] = None) -> str:
        """
        Read output from the server.

        Args:
            timeout: Optional timeout in seconds

        Returns:
            Decoded output string
        """
        if not self.tn:
            return ""

        try:
            timeout = timeout or self.config.command_timeout
            output = self.tn.read_very_eager().decode(self.config.encoding, errors='ignore')
            return output
        except Exception as e:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Read error: {e}")
            return ""

    def _wait_for_prompt(self, prompt_patterns: List[str], timeout: int = 5) -> bool:
        """
        Wait for a prompt pattern to appear in the output.

        Args:
            prompt_patterns: List of patterns to wait for
            timeout: Timeout in seconds

        Returns:
            True if prompt found, False otherwise
        """
        start_time = time.time()

        while time.time() - start_time < timeout:
            try:
                data = self.tn.read_some()
                if data:
                    self.buffer += data.decode(self.config.encoding, errors='ignore')
                    for pattern in prompt_patterns:
                        if pattern in self.buffer:
                            return True
            except Exception:
                pass
            time.sleep(0.1)

        return False

    def _send(self, command: str) -> bool:
        """
        Send a command to the server.

        Args:
            command: Command string to send

        Returns:
            True if send successful, False otherwise
        """
        if not self.tn:
            return False

        try:
            cmd_bytes = command.encode(self.config.encoding) + b'\n'
            self.tn.write(cmd_bytes)
            return True
        except Exception as e:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Send error: {e}")
            return False


# ============================================================================
# Character Creation and Login
# ============================================================================

    def login_or_create(self) -> bool:
        """
        Handle login and character creation flow.

        Returns:
            True if login/creation successful, False otherwise
        """
        if not self.connected:
            return False

        try:
            # Wait for name prompt
            name_patterns = ['무림존함', '이 름', 'Username', 'ID', 'login', '>']
            self._wait_for_prompt(name_patterns, timeout=5)

            # Send character name
            self._send(self.character_name)
            time.sleep(1)
            self.buffer = self._read_output()

            # Check if password is requested
            if any(p in self.buffer for p in ['Password', '비밀번호', '비번', '암호']):
                self._send(self.password)
                time.sleep(1)
                self.buffer = self._read_output()

            # Check if character needs to be created
            if any(p in self.buffer for p in ['없습니다', 'create', '새로운', '없는', 'Unknown']):
                return self._create_character()

            # Check if already logged in
            if any(p in self.buffer for p in ['명령', 'Commands', '무공', '능력치', '>']):
                self.logged_in = True
                return True

            # Try to clear any remaining prompts
            for _ in range(3):
                self._send("")
                time.sleep(0.3)

            self.buffer = self._read_output()
            self.logged_in = True
            return True

        except Exception as e:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Login error: {e}")
            return False

    def _create_character(self) -> bool:
        """
        Handle character creation flow.

        Returns:
            True if creation successful, False otherwise
        """
        try:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Creating character: {self.character_name}")

            # Send confirmation to create
            self._send("y")
            time.sleep(1)
            self.buffer = self._read_output()

            # Answer creation prompts with default values
            # Different servers may have different creation flows
            max_prompts = 15
            prompts_answered = 0

            while prompts_answered < max_prompts:
                self._send("")
                time.sleep(0.3)
                self.buffer = self._read_output()

                # Check if we're at the main game prompt
                if any(p in self.buffer for p in ['명령', 'Commands', '무공', '능력치', '낙양성', '접속']):
                    self.logged_in = True
                    if self.config.verbose:
                        print(f"[{self.server_type.value}] Character created successfully")
                    return True

                prompts_answered += 1

            self.logged_in = True
            return True

        except Exception as e:
            if self.config.verbose:
                print(f"[{self.server_type.value}] Character creation error: {e}")
            return False


# ============================================================================
# Command Execution
# ============================================================================

    def execute_command(self, command: str, wait_time: float = 1.0) -> str:
        """
        Execute a command and return the output.

        Args:
            command: Command string to execute
            wait_time: Time to wait for response

        Returns:
            Command output string
        """
        if not self.logged_in:
            return ""

        start_time = time.time()

        # Clear buffer first
        self.buffer = ""

        # Send command
        if not self._send(command):
            return ""

        # Wait for response
        time.sleep(wait_time)

        # Read output
        output = self._read_output()

        execution_time = time.time() - start_time

        if self.config.verbose:
            print(f"[{self.server_type.value}] Command '{command}' executed in {execution_time:.2f}s")

        return output

    def execute_commands(self, commands: List[str], wait_time: float = 1.0) -> Dict[str, str]:
        """
        Execute multiple commands and return outputs.

        Args:
            commands: List of command strings
            wait_time: Time to wait for each response

        Returns:
            Dictionary mapping commands to their outputs
        """
        results = {}

        for cmd in commands:
            output = self.execute_command(cmd, wait_time)
            results[cmd] = output

            # Small delay between commands
            time.sleep(0.2)

        return results


# ============================================================================
# Test Functions
# ============================================================================

def test_basic_commands(conn: MUDConnection) -> List[TestResult]:
    """
    Test basic MUD commands.

    Args:
        conn: MUD connection to use

    Returns:
        List of test results
    """
    commands = [
        ('능력치', 'Stats command'),
        ('점수', 'Score command'),
        ('무공', 'Martial arts command'),
        ('소지품', 'Inventory command'),
        ('누구', 'Who command'),
        ('봐', 'Look command'),
        ('지도', 'Map command'),
        ('어디', 'Where command'),
        ('도움말', 'Help command'),
        ('저장', 'Save command'),
    ]

    results = []

    for cmd, description in commands:
        start_time = time.time()
        output = conn.execute_command(cmd, wait_time=1.5)
        execution_time = time.time() - start_time

        result = TestResult(
            server_type=conn.server_type,
            character_name=conn.character_name,
            command=cmd,
            success=len(output) > 0,
            output=output,
            output_length=len(output),
            timestamp=datetime.now().isoformat(),
            execution_time=execution_time
        )
        results.append(result)

        if conn.config.verbose:
            print(f"[{conn.server_type.value}] {description}: {len(output)} bytes")

    return results


def test_movement(conn: MUDConnection) -> List[TestResult]:
    """
    Test movement commands.

    Args:
        conn: MUD connection to use

    Returns:
        List of test results
    """
    # Movement directions
    movements = [
        ('동', 'East'),
        ('서', 'West'),
        ('남', 'South'),
        ('북', 'North'),
        ('위', 'Up'),
        ('아래', 'Down'),
        ('봐', 'Look after movement'),
    ]

    results = []

    for cmd, description in movements:
        start_time = time.time()
        output = conn.execute_command(cmd, wait_time=1.0)
        execution_time = time.time() - start_time

        result = TestResult(
            server_type=conn.server_type,
            character_name=conn.character_name,
            command=f"move_{cmd}",
            success=len(output) > 0,
            output=output,
            output_length=len(output),
            timestamp=datetime.now().isoformat(),
            execution_time=execution_time
        )
        results.append(result)

        if conn.config.verbose:
            print(f"[{conn.server_type.value}] Movement {description}: {len(output)} bytes")

    return results


def test_combat(conn: MUDConnection) -> List[TestResult]:
    """
    Test combat-related commands.

    Args:
        conn: MUD connection to use

    Returns:
        List of test results
    """
    combat_commands = [
        ('상태', 'Status command'),
        ('공격', 'Attack command'),
        ('습득', 'Learn skill command'),
        ('시전', 'Cast skill command'),
        ('도망', 'Flee command'),
    ]

    results = []

    for cmd, description in combat_commands:
        start_time = time.time()
        output = conn.execute_command(cmd, wait_time=1.5)
        execution_time = time.time() - start_time

        result = TestResult(
            server_type=conn.server_type,
            character_name=conn.character_name,
            command=cmd,
            success=len(output) > 0,
            output=output,
            output_length=len(output),
            timestamp=datetime.now().isoformat(),
            execution_time=execution_time
        )
        results.append(result)

        if conn.config.verbose:
            print(f"[{conn.server_type.value}] Combat {description}: {len(output)} bytes")

    return results


def test_items(conn: MUDConnection) -> List[TestResult]:
    """
    Test item-related commands.

    Args:
        conn: MUD connection to use

    Returns:
        List of test results
    """
    item_commands = [
        ('장비', 'Equipment command'),
        ('품목표', 'Item list command'),
        ('버려 검', 'Drop item command'),
        ('줘', 'Give command'),
        ('구입', 'Buy command'),
        ('판매', 'Sell command'),
        ('먹어', 'Eat command'),
        ('입어', 'Wear command'),
        ('벗어', 'Remove command'),
    ]

    results = []

    for cmd, description in item_commands:
        start_time = time.time()
        output = conn.execute_command(cmd, wait_time=1.5)
        execution_time = time.time() - start_time

        result = TestResult(
            server_type=conn.server_type,
            character_name=conn.character_name,
            command=cmd,
            success=len(output) > 0,
            output=output,
            output_length=len(output),
            timestamp=datetime.now().isoformat(),
            execution_time=execution_time
        )
        results.append(result)

        if conn.config.verbose:
            print(f"[{conn.server_type.value}] Item {description}: {len(output)} bytes")

    return results


def test_npc_dialogue(conn: MUDConnection) -> List[TestResult]:
    """
    Test NPC interaction commands.

    Args:
        conn: MUD connection to use

    Returns:
        List of test results
    """
    dialogue_commands = [
        ('말 안녕', 'Say hello'),
        ('대화', 'Dialogue command'),
        ('물어', 'Ask command'),
        ('정보', 'Info command'),
        ('퀘스트', 'Quest command'),
    ]

    results = []

    for cmd, description in dialogue_commands:
        start_time = time.time()
        output = conn.execute_command(cmd, wait_time=1.5)
        execution_time = time.time() - start_time

        result = TestResult(
            server_type=conn.server_type,
            character_name=conn.character_name,
            command=cmd,
            success=len(output) > 0,
            output=output,
            output_length=len(output),
            timestamp=datetime.now().isoformat(),
            execution_time=execution_time
        )
        results.append(result)

        if conn.config.verbose:
            print(f"[{conn.server_type.value}] Dialogue {description}: {len(output)} bytes")

    return results


# ============================================================================
# Comparison Functions
# ============================================================================

def compare_outputs(py_output: str, rust_output: str, command: str) -> ComparisonResult:
    """
    Compare outputs from Python and Rust servers.

    Args:
        py_output: Output from Python server
        rust_output: Output from Rust server
        command: Command that was executed

    Returns:
        Comparison result
    """
    result = ComparisonResult(
        command=command,
        py_output=py_output,
        rust_output=rust_output,
        match=False,
        differences=[],
        keywords_match={}
    )

    # Clean outputs for comparison
    py_clean = strip_ansi_codes(py_output).strip()
    rust_clean = strip_ansi_codes(rust_output).strip()

    # Normalize for comparison
    py_normalized = normalize_whitespace(py_clean)
    rust_normalized = normalize_whitespace(rust_clean)

    # Check if outputs match
    result.match = py_normalized == rust_normalized

    # If they don't match, find differences
    if not result.match:
        py_lines = py_clean.split('\n')
        rust_lines = rust_clean.split('\n')

        if len(py_lines) != len(rust_lines):
            result.differences.append(f"Line count differs: Python={len(py_lines)}, Rust={len(rust_lines)}")

        # Compare line by line
        max_lines = min(max(len(py_lines), len(rust_lines)), 50)
        for i in range(max_lines):
            py_line = py_lines[i].strip() if i < len(py_lines) else "(missing)"
            rust_line = rust_lines[i].strip() if i < len(rust_lines) else "(missing)"

            if py_line != rust_line:
                py_norm = normalize_whitespace(py_line)
                rust_norm = normalize_whitespace(rust_line)
                if py_norm != rust_norm:
                    result.differences.append(f"Line {i+1}: Python='{py_line[:80]}', Rust='{rust_line[:80]}'")

    # Check for important keywords
    important_keywords = [
        '체력', '내력', '은전', '경험치', '레벨',
        'HP', 'MP', 'Gold', 'EXP', 'Level',
        '낙양성', '방파', '무공', '아이템'
    ]

    for keyword in important_keywords:
        py_has = keyword in py_output
        rust_has = keyword in rust_output
        result.keywords_match[keyword] = (py_has, rust_has)

    return result


def compare_results(py_results: List[TestResult], rust_results: List[TestResult]) -> List[ComparisonResult]:
    """
    Compare test results from Python and Rust servers.

    Args:
        py_results: Results from Python server
        rust_results: Results from Rust server

    Returns:
        List of comparison results
    """
    comparisons = []

    # Create a dictionary for easy lookup
    rust_dict = {r.command: r for r in rust_results}

    for py_result in py_results:
        rust_result = rust_dict.get(py_result.command)

        if rust_result:
            comparison = compare_outputs(
                py_result.output,
                rust_result.output,
                py_result.command
            )
            comparisons.append(comparison)

    return comparisons


# ============================================================================
# Report Generation
# ============================================================================

def generate_report(report: TestReport, config: TestConfig) -> None:
    """
    Generate a detailed test report.

    Args:
        report: Test report data
        config: Test configuration
    """
    os.makedirs(os.path.dirname(config.report_path) or ".", exist_ok=True)

    with open(config.report_path, 'w', encoding='utf-8') as f:
        # Header
        f.write("# MUD Server Test Report\n\n")
        f.write(f"**Generated:** {report.test_end}\n\n")
        f.write(f"**Test Duration:** {report.test_start} to {report.test_end}\n\n")

        f.write("## Test Configuration\n\n")
        f.write(f"- **Host:** {config.host}\n")
        f.write(f"- **Python Server Port:** {config.py_port}\n")
        f.write(f"- **Rust Server Port:** {config.rust_port}\n")
        f.write(f"- **Number of Characters:** {config.num_characters}\n")
        f.write(f"- **Base Password:** {config.base_password}\n\n")

        f.write("---\n\n")

        # Summary
        f.write("## Test Summary\n\n")
        f.write(f"- **Total Tests:** {report.total_tests}\n")
        f.write(f"- **Passed Tests:** {report.passed_tests}\n")
        f.write(f"- **Failed Tests:** {report.failed_tests}\n\n")

        if report.total_tests > 0:
            pass_rate = (report.passed_tests / report.total_tests) * 100
            f.write(f"**Pass Rate:** {pass_rate:.1f}%\n\n")

        # Server Status
        f.write("## Server Status\n\n")
        for server, status in report.server_status.items():
            status_str = "ONLINE" if status else "OFFLINE"
            f.write(f"- **{server}:** {status_str}\n\n")

        f.write("---\n\n")

        # Comparison Results
        if report.comparison_results:
            f.write("## Comparison Results\n\n")

            match_count = sum(1 for c in report.comparison_results if c.match)
            total_comparisons = len(report.comparison_results)

            f.write(f"- **Total Comparisons:** {total_comparisons}\n")
            f.write(f"- **Matching Outputs:** {match_count}\n")
            f.write(f"- **Different Outputs:** {total_comparisons - match_count}\n\n")

            # Detailed comparisons
            f.write("### Detailed Comparisons\n\n")

            for comp in report.comparison_results:
                status = "MATCH" if comp.match else "DIFFER"
                f.write(f"#### Command: `{comp.command}` - {status}\n\n")

                if not comp.match and comp.differences:
                    f.write("**Differences:**\n\n")
                    for diff in comp.differences[:10]:
                        f.write(f"- {diff}\n")
                    f.write("\n")

                # Keyword comparison
                if comp.keywords_match:
                    f.write("**Keywords Present:**\n\n")
                    f.write("| Keyword | Python | Rust |\n")
                    f.write("|---------|--------|------|\n")
                    for kw, (py_has, rust_has) in comp.keywords_match.items():
                        py_str = "X" if py_has else ""
                        rust_str = "X" if rust_has else ""
                        f.write(f"| {kw} | {py_str} | {rust_str} |\n")
                    f.write("\n")

                f.write("---\n\n")

        # Detailed Test Results (Python)
        py_results = [r for r in report.test_results if r.server_type == ServerType.PYTHON]
        if py_results:
            f.write("## Python Server Test Results\n\n")
            for result in py_results:
                status = "PASS" if result.success else "FAIL"
                f.write(f"### `{result.command}` - {status}\n\n")
                f.write(f"- **Output Length:** {result.output_length} bytes\n")
                f.write(f"- **Execution Time:** {result.execution_time:.2f}s\n")
                f.write(f"- **Timestamp:** {result.timestamp}\n\n")

                if result.output:
                    clean_output = strip_ansi_codes(result.output)
                    preview = clean_output[:500] + ("..." if len(clean_output) > 500 else "")
                    f.write(f"**Output Preview:**\n```\n{preview}\n```\n\n")

                f.write("---\n\n")

        # Detailed Test Results (Rust)
        rust_results = [r for r in report.test_results if r.server_type == ServerType.RUST]
        if rust_results:
            f.write("## Rust Server Test Results\n\n")
            for result in rust_results:
                status = "PASS" if result.success else "FAIL"
                f.write(f"### `{result.command}` - {status}\n\n")
                f.write(f"- **Output Length:** {result.output_length} bytes\n")
                f.write(f"- **Execution Time:** {result.execution_time:.2f}s\n")
                f.write(f"- **Timestamp:** {result.timestamp}\n\n")

                if result.output:
                    clean_output = strip_ansi_codes(result.output)
                    preview = clean_output[:500] + ("..." if len(clean_output) > 500 else "")
                    f.write(f"**Output Preview:**\n```\n{preview}\n```\n\n")

                f.write("---\n\n")

    print(f"\nReport saved to: {config.report_path}")


# ============================================================================
# Main Test Runner
# ============================================================================

class MUDTestRunner:
    """
    Main test runner for MUD server testing.

    This class orchestrates the entire testing process including
    connecting to servers, running tests, and generating reports.
    """

    def __init__(self, config: TestConfig):
        """
        Initialize the test runner.

        Args:
            config: Test configuration
        """
        self.config = config
        self.report = TestReport(
            test_start=datetime.now().isoformat(),
            test_end="",
            total_tests=0,
            passed_tests=0,
            failed_tests=0,
            server_status={}
        )
        self.py_conn: Optional[MUDConnection] = None
        self.rust_conn: Optional[MUDConnection] = None

    def run_all_tests(self) -> None:
        """Run all test scenarios."""
        print("=" * 70)
        print("MUD Server Comprehensive Test Suite")
        print("=" * 70)
        print(f"Starting at: {self.report.test_start}")
        print()

        # Check server status
        self._check_servers()

        # Connect to servers
        if not self._connect_servers():
            print("Failed to connect to one or more servers")
            return

        # Run test scenarios
        test_scenarios = [
            ("Basic Commands", test_basic_commands),
            ("Movement", test_movement),
            ("Combat", test_combat),
            ("Items", test_items),
            ("NPC Dialogue", test_npc_dialogue),
        ]

        for scenario_name, test_func in test_scenarios:
            print(f"\n{'=' * 70}")
            print(f"Running: {scenario_name}")
            print('=' * 70)

            self._run_test_scenario(scenario_name, test_func)

        # Finalize report
        self.report.test_end = datetime.now().isoformat()
        self._finalize_report()

        # Generate report
        generate_report(self.report, self.config)

        # Cleanup
        self._cleanup()

    def run_specific_test(self, test_name: str) -> None:
        """
        Run a specific test scenario.

        Args:
            test_name: Name of the test to run
        """
        test_map = {
            'basic': ('Basic Commands', test_basic_commands),
            'movement': ('Movement', test_movement),
            'combat': ('Combat', test_combat),
            'items': ('Items', test_items),
            'npc': ('NPC Dialogue', test_npc_dialogue),
        }

        if test_name not in test_map:
            print(f"Unknown test: {test_name}")
            print(f"Available tests: {', '.join(test_map.keys())}")
            return

        scenario_name, test_func = test_map[test_name]

        print("=" * 70)
        print(f"MUD Server Test: {scenario_name}")
        print("=" * 70)

        self._check_servers()
        self._connect_servers()
        self._run_test_scenario(scenario_name, test_func)

        self.report.test_end = datetime.now().isoformat()
        self._finalize_report()
        generate_report(self.report, self.config)
        self._cleanup()

    def _check_servers(self) -> None:
        """Check if servers are accessible."""
        import socket

        print("Checking server availability...")

        for name, port in [("Python", self.config.py_port), ("Rust", self.config.rust_port)]:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(2)
                result = sock.connect_ex((self.config.host, port))
                sock.close()
                self.report.server_status[name] = (result == 0)
                status = "ONLINE" if result == 0 else "OFFLINE"
                print(f"  {name} server ({self.config.host}:{port}): {status}")
            except Exception as e:
                self.report.server_status[name] = False
                print(f"  {name} server: OFFLINE ({e})")

        print()

    def _connect_servers(self) -> bool:
        """
        Connect to both servers.

        Returns:
            True if both connections successful, False otherwise
        """
        # Connect to Python server
        print("Connecting to Python server...")
        self.py_conn = MUDConnection(
            self.config,
            ServerType.PYTHON,
            self.config.py_port,
            f"테스터파이썬"
        )

        if self.py_conn.connect():
            if self.py_conn.login_or_create():
                print("  Python server: Connected and logged in")
            else:
                print("  Python server: Connected but login failed")
        else:
            print("  Python server: Connection failed")

        # Connect to Rust server
        print("\nConnecting to Rust server...")
        self.rust_conn = MUDConnection(
            self.config,
            ServerType.RUST,
            self.config.rust_port,
            f"테스터러스트"
        )

        if self.rust_conn.connect():
            if self.rust_conn.login_or_create():
                print("  Rust server: Connected and logged in")
            else:
                print("  Rust server: Connected but login failed")
        else:
            print("  Rust server: Connection failed")

        print()

        # Check if at least one server is connected
        return (self.py_conn.logged_in or self.rust_conn.logged_in)

    def _run_test_scenario(self, scenario_name: str, test_func) -> None:
        """
        Run a test scenario on both servers.

        Args:
            scenario_name: Name of the scenario
            test_func: Test function to execute
        """
        py_results = []
        rust_results = []

        # Run on Python server
        if self.py_conn and self.py_conn.logged_in:
            print(f"\n[Python Server] Running {scenario_name}...")
            try:
                py_results = test_func(self.py_conn)
                self.report.test_results.extend(py_results)
                print(f"  Completed {len(py_results)} tests")
            except Exception as e:
                print(f"  Error: {e}")

        # Run on Rust server
        if self.rust_conn and self.rust_conn.logged_in:
            print(f"\n[Rust Server] Running {scenario_name}...")
            try:
                rust_results = test_func(self.rust_conn)
                self.report.test_results.extend(rust_results)
                print(f"  Completed {len(rust_results)} tests")
            except Exception as e:
                print(f"  Error: {e}")

        # Compare results
        if py_results and rust_results:
            print(f"\n[Comparison] Comparing outputs...")
            comparisons = compare_results(py_results, rust_results)
            self.report.comparison_results.extend(comparisons)

            match_count = sum(1 for c in comparisons if c.match)
            print(f"  {match_count}/{len(comparisons)} outputs match")

            # Show differences
            for comp in comparisons:
                if not comp.match:
                    print(f"  - '{comp.command}': Outputs differ")

    def _finalize_report(self) -> None:
        """Finalize the test report."""
        self.report.total_tests = len(self.report.test_results)
        self.report.passed_tests = sum(1 for r in self.report.test_results if r.success)
        self.report.failed_tests = self.report.total_tests - self.report.passed_tests

    def _cleanup(self) -> None:
        """Clean up connections."""
        if self.py_conn:
            self.py_conn.disconnect()
        if self.rust_conn:
            self.rust_conn.disconnect()

        print("\n" + "=" * 70)
        print("Test completed!")
        print(f"Total tests: {self.report.total_tests}")
        print(f"Passed: {self.report.passed_tests}")
        print(f"Failed: {self.report.failed_tests}")
        print("=" * 70)


# ============================================================================
# Standalone Functions (for direct use without the class)
# ============================================================================

def connect_to_server(host: str, port: int, encoding: str = "euc-kr",
                      timeout: int = 15) -> Optional[telnetlib.Telnet]:
    """
    Connect to a MUD server.

    Args:
        host: Server hostname or IP
        port: Server port
        encoding: Character encoding (default: euc-kr)
        timeout: Connection timeout in seconds

    Returns:
        Telnet connection object or None if failed

    Example:
        >>> conn = connect_to_server("localhost", 9900)
        >>> if conn:
        ...     print("Connected!")
    """
    try:
        tn = telnetlib.Telnet(host, port, timeout=timeout)
        time.sleep(1)
        return tn
    except Exception as e:
        print(f"Connection failed: {e}")
        return None


def send_command(sock: telnetlib.Telnet, cmd: str, encoding: str = "euc-kr") -> bool:
    """
    Send a command to the MUD server.

    Args:
        sock: Telnet socket
        cmd: Command string to send
        encoding: Character encoding (default: euc-kr)

    Returns:
        True if send successful, False otherwise

    Example:
        >>> send_command(conn, "능력치")
        True
    """
    try:
        cmd_bytes = cmd.encode(encoding) + b'\n'
        sock.write(cmd_bytes)
        return True
    except Exception as e:
        print(f"Send failed: {e}")
        return False


def create_character(host: str, port: int, name: str, password: str,
                     encoding: str = "euc-kr") -> bool:
    """
    Create a new character on the MUD server.

    Args:
        host: Server hostname or IP
        port: Server port
        name: Character name (Korean supported)
        password: Character password
        encoding: Character encoding (default: euc-kr)

    Returns:
        True if creation successful, False otherwise

    Example:
        >>> create_character("localhost", 9900, "테스터", "1234")
        True
    """
    try:
        conn = telnetlib.Telnet(host, port, timeout=15)
        time.sleep(1)

        # Wait for name prompt and send name
        conn.read_very_eager().decode(encoding, errors='ignore')
        send_command(conn, name, encoding)
        time.sleep(1)

        # Check if we need to create
        response = conn.read_very_eager().decode(encoding, errors='ignore')
        if any(p in response for p in ['없습니다', 'create', '새로운']):
            send_command(conn, "y", encoding)
            time.sleep(1)

            # Answer creation prompts
            for _ in range(10):
                send_command(conn, "", encoding)
                time.sleep(0.3)

        conn.close()
        return True

    except Exception as e:
        print(f"Character creation failed: {e}")
        return False


# ============================================================================
# Main Entry Point
# ============================================================================

def parse_arguments() -> TestConfig:
    """
    Parse command line arguments.

    Returns:
        Test configuration
    """
    config = TestConfig()

    for arg in sys.argv[1:]:
        if arg.startswith("--host="):
            config.host = arg.split("=", 1)[1]
        elif arg.startswith("--py-port="):
            config.py_port = int(arg.split("=", 1)[1])
        elif arg.startswith("--rust-port="):
            config.rust_port = int(arg.split("=", 1)[1])
        elif arg.startswith("--chars="):
            config.num_characters = int(arg.split("=", 1)[1])
        elif arg.startswith("--report="):
            config.report_path = arg.split("=", 1)[1]
        elif arg.startswith("--test="):
            config.test_type = arg.split("=", 1)[1]
        elif arg == "--verbose":
            config.verbose = True
        elif arg == "--no-report":
            config.no_report = True

    return config


def main() -> int:
    """
    Main entry point.

    Returns:
        Exit code (0 for success, 1 for failure)
    """
    config = parse_arguments()

    # Override for standalone function usage
    config.test_type = getattr(config, 'test_type', 'all')

    runner = MUDTestRunner(config)

    try:
        if config.test_type == 'all':
            runner.run_all_tests()
        else:
            runner.run_specific_test(config.test_type)
        return 0
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
        runner._cleanup()
        return 1
    except Exception as e:
        print(f"\n\nTest error: {e}")
        import traceback
        traceback.print_exc()
        runner._cleanup()
        return 1


if __name__ == '__main__':
    sys.exit(main())
