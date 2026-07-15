#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MUD Test Skill - Python/Rust Server Comparison

This skill provides comprehensive testing and comparison between Python and Rust MUD servers.
It wraps the test/test_mud_comprehensive.py functionality with skill-specific enhancements.
"""

import sys
import os
import json
import subprocess
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional, Tuple

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# Import the comprehensive test module
try:
    from test.test_mud_comprehensive import (
        MUDConnection, TestConfig, TestResult, ServerType,
        test_basic_commands, test_movement, test_combat,
        test_items, test_npc_dialogue, compare_outputs,
        strip_ansi_codes, MUDTestRunner
    )
except ImportError:
    print("Error: Cannot import test_mud_comprehensive module")
    sys.exit(1)


class MUDTestSkill:
    """
    MUD Test Skill - Main handler for MUD server testing

    This class provides a unified interface for testing MUD servers
    with various test scenarios and reporting capabilities.
    """

    def __init__(self):
        """Initialize the MUD Test skill."""
        self.config = TestConfig()
        self.results_cache = []

    def show_help(self) -> None:
        """Display help information for the MUD test skill."""
        help_text = """
╔══════════════════════════════════════════════════════════════════╗
║                    MUD Test Skill - Help                        ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  Usage: /mud-test [command] [options]                           ║
║                                                                  ║
║  Commands:                                                       ║
║    all           Run all test scenarios (default)               ║
║    basic         Test basic commands (stats, inventory, etc.)   ║
║    movement      Test movement commands (directions)             ║
║    combat        Test combat system (attack, skills)            ║
║    items         Test item interactions (buy, sell, drop)       ║
║    npc           Test NPC dialogue and interactions              ║
║    quick         Quick comparison test (essential commands)      ║
║    status        Show server connection status                   ║
║    report        Generate and show last test report              ║
║    help          Show this help message                         ║
║                                                                  ║
║  Options:                                                        ║
║    --py-port=N   Python server port (default: 9900)              ║
║    --rust-port=N Rust server port (default: 9999)               ║
║    --host=HOST   Server host (default: localhost)               ║
║    --verbose     Enable verbose output                          ║
║    --report=FILE Custom report file path                        ║
║                                                                  ║
║  Examples:                                                       ║
║    /mud-test all                      # Run all tests           ║
║    /mud-test basic --verbose          # Test basics with detail ║
║    /mud-test combat --py-port=9901    # Test combat on custom   ║
║    /mud-test quick                    # Quick comparison        ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
        """
        print(help_text)

    def run_test(self, test_type: str = "all", **options) -> int:
        """
        Run MUD server tests.

        Args:
            test_type: Type of test to run
            **options: Additional options (ports, host, verbose, etc.)

        Returns:
            Exit code (0 for success, 1 for failure)
        """
        # Update config with options
        if 'py_port' in options:
            self.config.py_port = options['py_port']
        if 'rust_port' in options:
            self.config.rust_port = options['rust_port']
        if 'host' in options:
            self.config.host = options['host']
        if 'verbose' in options:
            self.config.verbose = options['verbose']
        if 'report_path' in options:
            self.config.report_path = options['report_path']

        print(f"\n{'='*70}")
        print(f"MUD Server Test: {test_type.upper()}")
        print(f"Python Port: {self.config.py_port} | Rust Port: {self.config.rust_port}")
        print(f"Host: {self.config.host}")
        print(f"{'='*70}\n")

        try:
            runner = MUDTestRunner(self.config)

            if test_type == "all":
                succeeded = runner.run_all_tests()
            elif test_type == "quick":
                succeeded = self._run_quick_test(runner)
            elif test_type == "status":
                return self._check_server_status()
            elif test_type == "report":
                return self._show_last_report()
            else:
                succeeded = runner.run_specific_test(test_type)

            return 0 if succeeded else 1

        except KeyboardInterrupt:
            print("\n\nTest interrupted by user")
            return 1
        except Exception as e:
            print(f"\n\nTest error: {e}")
            import traceback
            traceback.print_exc()
            return 1

    def _run_quick_test(self, runner: MUDTestRunner) -> bool:
        """
        Run a quick comparison test with essential commands only.

        Args:
            runner: MUDTestRunner instance
        """
        print("\n[Quick Test] Testing essential commands...\n")

        runner._check_servers()
        if not runner._connect_servers():
            print("Failed to connect to servers")
            runner._cleanup()
            return False

        # Quick test commands
        quick_commands = [
            ('능력치', 'Stats'),
            ('소지품', 'Inventory'),
            ('봐', 'Look'),
            ('저장', 'Save'),
        ]

        py_results = []
        rust_results = []

        if runner.py_conn and runner.py_conn.logged_in:
            print("[Python Server] Testing...")
            for cmd, desc in quick_commands:
                output = runner.py_conn.execute_command(cmd, wait_time=1.0)
                py_results.append(TestResult(
                    server_type=ServerType.PYTHON,
                    character_name=runner.py_conn.character_name,
                    command=cmd,
                    success=len(output) > 0,
                    output=output,
                    output_length=len(output),
                    timestamp=datetime.now().isoformat()
                ))
                print(f"  {desc}: {len(output)} bytes")

        if runner.rust_conn and runner.rust_conn.logged_in:
            print("\n[Rust Server] Testing...")
            for cmd, desc in quick_commands:
                output = runner.rust_conn.execute_command(cmd, wait_time=1.0)
                rust_results.append(TestResult(
                    server_type=ServerType.RUST,
                    character_name=runner.rust_conn.character_name,
                    command=cmd,
                    success=len(output) > 0,
                    output=output,
                    output_length=len(output),
                    timestamp=datetime.now().isoformat()
                ))
                print(f"  {desc}: {len(output)} bytes")

        # Compare results
        exact_matches = True
        if py_results and rust_results:
            print("\n[Comparison]")
            for py_res, rust_res in zip(py_results, rust_results):
                py_output = py_res.output
                rust_output = rust_res.output
                py_len = len(py_output)
                rust_len = len(rust_output)

                if py_len > 0 and rust_len > 0:
                    matched = py_output == rust_output
                    exact_matches = exact_matches and matched
                    status = "EXACT MATCH" if matched else "DIFFER"
                    print(f"  {py_res.command}: Python={py_len}b, Rust={rust_len}b [{status}]")
                else:
                    exact_matches = False
                    print(f"  {py_res.command}: Python={py_len}b, Rust={rust_len}b [NO OUTPUT]")

        runner._cleanup()
        return (
            bool(py_results)
            and bool(rust_results)
            and all(result.success for result in py_results + rust_results)
            and exact_matches
        )

    def _check_server_status(self) -> int:
        """
        Check the status of both servers.

        Returns:
            Exit code
        """
        import socket

        print("\n[Server Status Check]")
        print("="*50)

        results = {}
        for name, port in [("Python", self.config.py_port), ("Rust", self.config.rust_port)]:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(2)
                result = sock.connect_ex((self.config.host, port))
                sock.close()
                status = "ONLINE" if result == 0 else "OFFLINE"
                results[name] = (result == 0)
                print(f"  {name} ({self.config.host}:{port}): {status}")
            except Exception as e:
                results[name] = False
                print(f"  {name}: OFFLINE ({e})")

        print("="*50)
        return 0 if all(results.values()) else 1

    def _show_last_report(self) -> int:
        """
        Show the last generated test report.

        Returns:
            Exit code
        """
        report_path = self.config.report_path

        if not os.path.exists(report_path):
            print(f"\nNo report found at: {report_path}")
            print("Run a test first to generate a report.")
            return 1

        print(f"\n[Last Test Report: {report_path}]")
        print("="*70)

        with open(report_path, 'r', encoding='utf-8') as f:
            content = f.read()
            # Show first 100 lines
            lines = content.split('\n')[:100]
            print('\n'.join(lines))

            if len(content.split('\n')) > 100:
                print(f"\n... (truncated, full report at {report_path})")

        return 0


def parse_skill_args(args: List[str]) -> Tuple[str, Dict]:
    """
    Parse skill command arguments.

    Args:
        args: List of command line arguments

    Returns:
        Tuple of (command, options_dict)
    """
    command = "all"
    options = {}

    if args and not args[0].startswith('--'):
        command = args[0]
        args = args[1:]

    for arg in args:
        if arg.startswith('--py-port='):
            options['py_port'] = int(arg.split('=')[1])
        elif arg.startswith('--rust-port='):
            options['rust_port'] = int(arg.split('=')[1])
        elif arg.startswith('--host='):
            options['host'] = arg.split('=')[1]
        elif arg.startswith('--report='):
            options['report_path'] = arg.split('=')[1]
        elif arg == '--verbose':
            options['verbose'] = True

    return command, options


def main():
    """Main entry point for the MUD test skill."""
    if len(sys.argv) > 1 and sys.argv[1] in ['help', '--help', '-h']:
        skill = MUDTestSkill()
        skill.show_help()
        return 0

    command, options = parse_skill_args(sys.argv[1:])

    skill = MUDTestSkill()
    return skill.run_test(command, **options)


if __name__ == '__main__':
    sys.exit(main())
