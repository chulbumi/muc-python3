#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Improved MUD Command Comparison Test
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
        self.character_name = character_name or "테스터{}".format(port)
        self.tn = None
        self.connected = False
        self.buffer = ""

    def connect(self):
        """Connect to the MUD server"""
        try:
            self.tn = telnetlib.Telnet(self.host, self.port, timeout=15)
            self.connected = True
            # Clear initial buffer
            time.sleep(1)
            self.buffer = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
            return True
        except Exception as e:
            print("Connection error to {}:{} - {}".format(self.host, self.port, e))
            return False

    def wait_for_prompt(self, timeout=5):
        """Wait for a prompt pattern"""
        start_time = time.time()
        while time.time() - start_time < timeout:
            try:
                data = self.tn.read_some()
                if data:
                    self.buffer += data.decode('euc-kr', errors='ignore')
                    # Check for common Korean prompts
                    if '무림존함' in self.buffer:
                        return True
                    if 'Password' in self.buffer or '비밀번호' in self.buffer:
                        return True
                    if '명령' in self.buffer:
                        return True
                    if '>' in self.buffer:
                        return True
            except:
                pass
            time.sleep(0.1)
        return False

    def login(self):
        """Login to the MUD with proper flow handling"""
        if not self.connected:
            return False

        try:
            # Wait for name prompt
            self.wait_for_prompt(timeout=5)
            
            # Send character name
            self.send_command(self.character_name)
            time.sleep(1)
            self.buffer = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
            
            # Check if password is requested
            if 'Password' in self.buffer or '비밀번호' in self.buffer:
                self.send_command("")
                time.sleep(1)
                self.buffer = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
            
            # Check if character needs to be created
            if '없습니다' in self.buffer or 'create' in self.buffer.lower() or '새로운' in self.buffer:
                # Try to create character
                self.send_command("y")
                time.sleep(1)
                self.buffer = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
                
                # Answer creation prompts with default values
                for _ in range(10):
                    self.send_command("")
                    time.sleep(0.3)
                    self.buffer = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
                    if '명령' in self.buffer or '>' in self.buffer:
                        break
            
            # Clear any remaining prompts
            for _ in range(3):
                self.send_command("")
                time.sleep(0.3)
            
            self.buffer = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
            return True
            
        except Exception as e:
            print("Login error: {}".format(e))
            return False

    def send_command(self, command):
        """Send a command to the MUD"""
        if self.tn:
            try:
                cmd_bytes = command.encode('euc-kr') + b'\n'
                self.tn.write(cmd_bytes)
            except Exception as e:
                print("Send command error: {}".format(e))

    def get_output(self, wait_time=1):
        """Get output from the MUD"""
        if self.tn:
            time.sleep(wait_time)
            try:
                output = self.tn.read_very_eager().decode('euc-kr', errors='ignore')
                return output
            except Exception as e:
                print("Get output error: {}".format(e))
                return ""
        return ""

    def execute_command(self, command, wait_time=1.5):
        """Execute a command and return the output"""
        # Clear buffer first
        self.buffer = ""
        self.send_command(command)
        time.sleep(wait_time)
        output = self.get_output(0.2)
        return output

    def disconnect(self):
        """Disconnect from the MUD"""
        if self.tn:
            try:
                self.tn.close()
            except:
                pass
            self.connected = False


def test_command(tester, command, wait_time=1.5):
    """Test a single command and return the output"""
    print("  Testing: {}".format(command))
    output = tester.execute_command(command, wait_time)
    return output


def strip_ansi(text):
    """Remove ANSI escape codes"""
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    return ansi_escape.sub('', text)


def compare_outputs(py_output, rust_output, command):
    """Compare outputs from Python and Rust servers"""
    result = {
        'command': command,
        'same': True,
        'differences': [],
        'py_output': py_output,
        'rust_output': rust_output,
        'py_clean': strip_ansi(py_output),
        'rust_clean': strip_ansi(rust_output)
    }

    # Clean outputs for comparison - remove ANSI codes
    py_clean = result['py_clean'].strip()
    rust_clean = result['rust_clean'].strip()

    # Normalize whitespace for comparison
    py_normalized = re.sub(r'\s+', ' ', py_clean)
    rust_normalized = re.sub(r'\s+', ' ', rust_clean)

    if py_normalized != rust_normalized:
        result['same'] = False

        # Find differences
        py_lines = py_clean.split('\n')
        rust_lines = rust_clean.split('\n')

        if len(py_lines) != len(rust_lines):
            result['differences'].append("Line count differs: Python={}, Rust={}".format(len(py_lines), len(rust_lines)))

        # Compare line by line
        max_lines = max(len(py_lines), len(rust_lines))
        for i in range(min(max_lines, 20)):  # Limit to first 20 lines
            py_line = py_lines[i].strip() if i < len(py_lines) else "(missing)"
            rust_line = rust_lines[i].strip() if i < len(rust_lines) else "(missing)"

            if py_line != rust_line:
                py_norm = re.sub(r'\s+', ' ', py_line)
                rust_norm = re.sub(r'\s+', ' ', rust_line)
                if py_norm != rust_norm:
                    result['differences'].append("Line {}:".format(i+1))
                    result['differences'].append("  Python: {}".format(py_line[:100]))
                    result['differences'].append("  Rust:   {}".format(rust_line[:100]))

    return result


def main():
    """Main test function"""
    results = []

    # Commands to test
    commands = [
        ('능력치', 2, 'Stats'),
        ('점수', 2, 'Score'),
        ('무공', 2, 'Martial Arts'),
        ('소지품', 2, 'Inventory'),
        ('누구', 2, 'Who'),
        ('봐', 2, 'Look'),
        ('말 테스트', 2, 'Say'),
        ('지도', 2, 'Map'),
        ('어디', 2, 'Where'),
        ('동', 3, 'East'),
        ('서', 3, 'West'),
        ('남', 3, 'South'),
        ('북', 3, 'North'),
    ]

    print("=" * 60)
    print("MUD Command Comparison Test")
    print("=" * 60)
    print("Start time: {}".format(datetime.now().strftime('%Y-%m-%d %H:%M:%S')))
    print()

    # Test each command with fresh connections
    for cmd, wait_time, description in commands:
        print("\n[{}] Testing command: {}".format(description, cmd))
        print("-" * 40)

        # Connect to Python server
        print("  Connecting to Python server (9900)...")
        py_tester = MUDTester(host='localhost', port=9900, character_name='테스터파이썬')
        if py_tester.connect():
            if py_tester.login():
                time.sleep(1)
                py_output = test_command(py_tester, cmd, wait_time)
                print("  Python output length: {} chars".format(len(py_output)))
            else:
                print("  Login failed")
                py_output = ""
            py_tester.disconnect()
        else:
            print("  Failed to connect to Python server")
            py_output = ""

        # Connect to Rust server
        print("  Connecting to Rust server (9999)...")
        rust_tester = MUDTester(host='localhost', port=9999, character_name='테스터러스트')
        if rust_tester.connect():
            if rust_tester.login():
                time.sleep(1)
                rust_output = test_command(rust_tester, cmd, wait_time)
                print("  Rust output length: {} chars".format(len(rust_output)))
            else:
                print("  Login failed")
                rust_output = ""
            rust_tester.disconnect()
        else:
            print("  Failed to connect to Rust server")
            rust_output = ""

        # Compare outputs
        print("  Comparing outputs...")
        comparison = compare_outputs(py_output, rust_output, cmd)
        results.append(comparison)

        if comparison['same']:
            print("  Result: Outputs are IDENTICAL")
        else:
            print("  Result: Outputs DIFFER")
            print("  Differences found: {}".format(len(comparison['differences'])))
            # Show first few differences
            for diff in comparison['differences'][:6]:
                print("    {}".format(diff))

        # Small delay between tests
        time.sleep(0.5)

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
        f.write("**Generated:** {}\n\n".format(datetime.now().strftime('%Y-%m-%d %H:%M:%S')))
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
        f.write("- **Total Commands Tested:** {}\n".format(len(results)))
        f.write("- **Identical Outputs:** {}\n".format(same_count))
        f.write("- **Different Outputs:** {}\n\n".format(diff_count))
        f.write("**Pass Rate:** {:.1f}%\n\n".format(same_count/len(results)*100 if len(results) > 0 else 0))
        f.write("---\n\n")

        # Detailed results
        f.write("## Detailed Results\n\n")

        for i, result in enumerate(results, 1):
            f.write("### {}. {}\n\n".format(i, result['command']))

            if result['same']:
                f.write("**Status:** IDENTICAL\n\n")
            else:
                f.write("**Status:** DIFFERENT\n\n")
                f.write("**Differences:**\n\n")
                for diff in result['differences'][:20]:  # Limit output
                    f.write("- {}\n".format(diff))
                f.write("\n")

            # Python output (cleaned - no ANSI)
            f.write("**Python Server Output:**\n\n")
            f.write("```\n")
            py_display = result['py_output'][:2000] if result['py_output'] else "(No output)"
            f.write(py_display)
            if len(result['py_output'] or "") > 2000:
                f.write("\n... (truncated)")
            f.write("\n```\n\n")

            # Rust output (cleaned - no ANSI)
            f.write("**Rust Server Output:**\n\n")
            f.write("```\n")
            rust_display = result['rust_output'][:2000] if result['rust_output'] else "(No output)"
            f.write(rust_display)
            if len(result['rust_output'] or "") > 2000:
                f.write("\n... (truncated)")
            f.write("\n```\n\n")

            f.write("---\n\n")

        # Recommendations
        if diff_count > 0:
            f.write("## Recommendations\n\n")
            f.write("The following commands need to be aligned between Python and Rust implementations:\n\n")

            for result in results:
                if not result['same']:
                    f.write("- **{}**\n".format(result['command']))
                    if result['differences']:
                        for diff in result['differences'][:3]:
                            f.write("  - {}\n".format(diff))
                    f.write("\n")

    print("\nReport saved to: {}".format(report_path))


if __name__ == '__main__':
    main()
