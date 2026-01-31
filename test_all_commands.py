#!/usr/bin/env python3
"""
MUD Command Comparison Test
Tests commands on both Python (9900) and Rust (9999) servers and compares outputs
"""

import telnetlib
import time
import re
import json
from datetime import datetime

class MUDTester:
    def __init__(self, host='localhost', port=9900, character_name=None):
        self.host = host
        self.port = port
        self.character_name = character_name or f"테스터{port}"
        self.tn = None
        self.connected = False

    def connect(self):
        """Connect to the MUD server"""
        try:
            self.tn = telnetlib.Telnet(self.host, self.port, timeout=10)
            self.connected = True
            time.sleep(0.5)
            return True
        except Exception as e:
            print(f"Connection error to {self.host}:{self.port} - {e}")
            return False

    def login(self):
        """Login to the MUD"""
        if not self.connected:
            return False

        # Wait for login prompt
        time.sleep(1)

        # Send character name
        self.send_command(self.character_name)
        time.sleep(1)

        # Send password (assuming empty or default)
        self.send_command("")
        time.sleep(1)

        # Skip through any initial prompts
        for _ in range(3):
            self.send_command("")
            time.sleep(0.5)

        return True

    def send_command(self, command):
        """Send a command to the MUD"""
        if self.tn:
            try:
                cmd_bytes = command.encode('utf-8') + b'\n'
                self.tn.write(cmd_bytes)
            except Exception as e:
                print(f"Send command error: {e}")

    def get_output(self, wait_time=1):
        """Get output from the MUD"""
        if self.tn:
            time.sleep(wait_time)
            try:
                output = self.tn.read_very_eager().decode('utf-8', errors='ignore')
                return output
            except Exception as e:
                print(f"Get output error: {e}")
                return ""
        return ""

    def execute_command(self, command, wait_time=1):
        """Execute a command and return the output"""
        self.send_command(command)
        return self.get_output(wait_time)

    def cleanup_output(self, output):
        """Clean up output for comparison"""
        # Remove extra whitespace
        output = re.sub(r'\s+', ' ', output)
        # Remove special characters that might differ
        output = output.strip()
        return output

    def disconnect(self):
        """Disconnect from the MUD"""
        if self.tn:
            try:
                self.tn.close()
            except:
                pass
            self.connected = False


def test_command(tester, command, wait_time=1):
    """Test a single command and return the output"""
    print(f"  Testing: {command}")
    output = tester.execute_command(command, wait_time)
    return output


def compare_outputs(py_output, rust_output, command):
    """Compare outputs from Python and Rust servers"""
    result = {
        'command': command,
        'same': True,
        'differences': [],
        'py_output': py_output,
        'rust_output': rust_output
    }

    # Clean outputs for comparison
    py_clean = py_output.strip()
    rust_clean = rust_output.strip()

    # Normalize whitespace for comparison
    py_normalized = re.sub(r'\s+', ' ', py_clean)
    rust_normalized = re.sub(r'\s+', ' ', rust_clean)

    if py_normalized != rust_normalized:
        result['same'] = False

        # Find differences
        py_lines = py_clean.split('\n')
        rust_lines = rust_clean.split('\n')

        if len(py_lines) != len(rust_lines):
            result['differences'].append(f"Line count differs: Python={len(py_lines)}, Rust={len(rust_lines)}")

        # Compare line by line
        max_lines = max(len(py_lines), len(rust_lines))
        for i in range(max_lines):
            py_line = py_lines[i] if i < len(py_lines) else "(missing)"
            rust_line = rust_lines[i] if i < len(rust_lines) else "(missing)"

            if py_line.strip() != rust_line.strip():
                py_norm = re.sub(r'\s+', ' ', py_line.strip())
                rust_norm = re.sub(r'\s+', ' ', rust_line.strip())
                if py_norm != rust_norm:
                    result['differences'].append(f"Line {i+1}:")
                    result['differences'].append(f"  Python: {py_line}")
                    result['differences'].append(f"  Rust:   {rust_line}")

    return result


def main():
    """Main test function"""
    results = []

    # Commands to test
    commands = [
        ('능력치', 1, 'Stats'),
        ('점수', 1, 'Score'),
        ('무공', 1, 'Martial Arts'),
        ('소지품', 1, 'Inventory'),
        ('누구', 1, 'Who'),
        ('봐', 1, 'Look'),
        ('말 테스트', 1, 'Say'),
        ('지도', 1, 'Map'),
        ('어디', 1, 'Where'),
        ('동', 2, 'East'),
        ('서', 2, 'West'),
        ('남', 2, 'South'),
        ('북', 2, 'North'),
    ]

    print("=" * 60)
    print("MUD Command Comparison Test")
    print("=" * 60)
    print(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print()

    # Test each command
    for cmd, wait_time, description in commands:
        print(f"\n[{description}] Testing command: {cmd}")
        print("-" * 40)

        # Connect to Python server
        print("  Connecting to Python server (9900)...")
        py_tester = MUDTester(host='localhost', port=9900, character_name='테스터파이썬')
        if py_tester.connect():
            py_tester.login()
            time.sleep(1)
            py_output = test_command(py_tester, cmd, wait_time)
            py_tester.disconnect()
            print(f"  Python output length: {len(py_output)} chars")
        else:
            print("  Failed to connect to Python server")
            py_output = ""

        # Connect to Rust server
        print("  Connecting to Rust server (9999)...")
        rust_tester = MUDTester(host='localhost', port=9999, character_name='테스터러스트')
        if rust_tester.connect():
            rust_tester.login()
            time.sleep(1)
            rust_output = test_command(rust_tester, cmd, wait_time)
            rust_tester.disconnect()
            print(f"  Rust output length: {len(rust_output)} chars")
        else:
            print("  Failed to connect to Rust server")
            rust_output = ""

        # Compare outputs
        print("  Comparing outputs...")
        comparison = compare_outputs(py_output, rust_output, cmd)
        results.append(comparison)

        if comparison['same']:
            print(f"  Result: Outputs are IDENTICAL")
        else:
            print(f"  Result: Outputs DIFFER")
            print(f"  Differences found: {len(comparison['differences'])}")

    # Generate report
    generate_report(results)

    print("\n" + "=" * 60)
    print("Test completed!")
    print("=" * 60)


def generate_report(results):
    """Generate a comparison report"""
    report_path = '/home/ubuntu/muc-python3/COMMAND_COMPARISON.md'

    with open(report_path, 'w', encoding='utf-8') as f:
        f.write("# MUD Command Comparison Report\n\n")
        f.write(f"**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        f.write("**Servers:**\n")
        f.write("- Python Server: localhost:9900\n")
        f.write("- Rust Server: localhost:9999\n\n")
        f.write("**Characters:**\n")
        f.write("- Python: 테스터파이썬\n")
        f.write("- Rust: 테스터러스트\n\n")
        f.write("---\n\n")

        # Summary
        same_count = sum(1 for r in results if r['same'])
        diff_count = len(results) - same_count

        f.write("## Summary\n\n")
        f.write(f"- **Total Commands Tested:** {len(results)}\n")
        f.write(f"- **Identical Outputs:** {same_count} \n")
        f.write(f"- **Different Outputs:** {diff_count} x\n\n")
        f.write(f"**Pass Rate:** {same_count/len(results)*100:.1f}%\n\n")
        f.write("---\n\n")

        # Detailed results
        f.write("## Detailed Results\n\n")

        for i, result in enumerate(results, 1):
            f.write(f"### {i}. {result['command']}\n\n")

            if result['same']:
                f.write("**Status:** IDENTICAL\n\n")
            else:
                f.write("**Status:** DIFFERENT\n\n")
                f.write("**Differences:**\n\n")
                for diff in result['differences']:
                    f.write(f"- {diff}\n")
                f.write("\n")

            # Python output
            f.write("**Python Server Output:**\n\n")
            f.write("```\n")
            f.write(result['py_output'] if result['py_output'] else "(No output)")
            f.write("\n```\n\n")

            # Rust output
            f.write("**Rust Server Output:**\n\n")
            f.write("```\n")
            f.write(result['rust_output'] if result['rust_output'] else "(No output)")
            f.write("\n```\n\n")

            f.write("---\n\n")

        # Recommendations
        if diff_count > 0:
            f.write("## Recommendations\n\n")
            f.write("The following commands need to be aligned between Python and Rust implementations:\n\n")

            for result in results:
                if not result['same']:
                    f.write(f"- **{result['command']}**")
                    if result['differences']:
                        f.write(f": {result['differences'][0] if result['differences'] else 'See details above'}")
                    f.write("\n")

    print(f"\nReport saved to: {report_path}")


if __name__ == '__main__':
    main()
