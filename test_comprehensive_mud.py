#!/usr/bin/env python3
"""Comprehensive MUD comparison test between Python (9900) and Rust (9999)"""
import socket
import time
import sys
import re

class MudTester:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.sock = None

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.setblocking(False)
        try:
            self.sock.connect((self.host, self.port))
        except BlockingIOError:
            pass  # Expected for non-blocking
        time.sleep(0.5)
        data = self.recv_all()
        return len(data) > 0

    def recv_all(self, timeout=1):
        data = b""
        start = time.time()
        while time.time() - start < timeout:
            try:
                chunk = self.sock.recv(8192)
                if not chunk:
                    break
                data += chunk
            except BlockingIOError:
                time.sleep(0.05)
        return data

    def send(self, cmd, wait_time=0.5):
        try:
            self.sock.send((cmd + "\r\n").encode('utf-8'))
        except:
            return b""
        time.sleep(wait_time)
        return self.recv_all()

    def clean_output(self, data):
        text = data.decode('utf-8', errors='replace')
        ansi_escape = re.compile(r'\x1b\[[0-9;]*[mKH]')
        return ansi_escape.sub('', text)

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass

    def reset(self):
        self.close()
        time.sleep(0.5)
        return self.connect()

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass

def test_login(host, port, name, password="1234"):
    """Test basic login flow"""
    print(f"\n{'='*50}")
    print(f"Testing {host}:{port} - Login as '{name}'")
    print('='*50)

    m = MudTester(host, port)
    if not m.connect():
        print(f"FAIL: Could not connect")
        return None

    # Send name
    data = m.send(name)

    # Check if asking for password
    text = data.decode('utf-8', errors='replace')
    if '암호' in text or '존함암호' in text:
        data = m.send(password)
        text = data.decode('utf-8', errors='replace')

    clean = m.clean_output(data)
    lines = [l.strip() for l in clean.split('\n') if l.strip() and len(l.strip()) > 2]

    # Check for success indicators
    success = 'HP' in text or '체력' in text or 'MP' in text or '내공' in text or '은전' in text
    print(f"Success: {success}")
    print(f"Response lines: {len(lines)}")

    # Show first few content lines
    content_lines = [l for l in lines if not l.startswith('=') and not l.startswith('[0m')]
    for line in content_lines[:5]:
        print(f"  {line[:70]}")

    m.close()
    return success, text

def test_commands(host, port, name, password):
    """Test various commands after login"""
    print(f"\n{'='*50}")
    print(f"Testing {host}:{port} - Commands")
    print('='*50)

    m = MudTester(host, port)
    m.connect()
    m.send(name)
    data = m.send(password)

    commands_to_test = [
        ("보기", "look around"),
        ("점수", "score/status"),
        ("인벤토리", "inventory"),
        ("장비", "equipment"),
        ("무공", "skills"),
        ("도움말", "help"),
        ("말", "say"),
        ("외침", "shout"),
        ("관리자설정 1000", "set admin level"),
        ("관리자도움말", "admin help"),
    ]

    results = {}
    for cmd, desc in commands_to_test:
        data = m.send(cmd, wait_time=0.5)
        text = data.decode('utf-8', errors='replace')
        # Check if command was recognized
        has_output = len(data) > 20
        is_error = '무슨 말인지 모르겠' in text or '알 수 없는' in text
        results[cmd] = {'has_output': has_output, 'is_error': is_error, 'bytes': len(data)}

        status = "OK" if has_output and not is_error else "ERROR" if is_error else "NO OUTPUT"
        print(f"  {cmd:15s} ({desc:15s}): {status:10s} ({len(data):4d} bytes)")

    m.close()
    return results

def compare_servers():
    """Run comprehensive comparison between Python and Rust MUD"""

    print("\n" + "="*60)
    print("COMPREHENSIVE MUD COMPARISON TEST")
    print("="*60)

    # Test login with existing user
    name = "test"
    password = "1234"

    print("\n" + "="*60)
    print("PHASE 1: LOGIN TEST")
    print("="*60)

    py_login, py_text = test_login('localhost', 9900, name, password)
    rust_login, rust_text = test_login('localhost', 9999, name, password)

    print(f"\nLogin Results:")
    print(f"  Python MUD (9900): {'PASS' if py_login else 'FAIL'}")
    print(f"  Rust MUD (9999):   {'PASS' if rust_login else 'FAIL'}")

    # Test commands
    print("\n" + "="*60)
    print("PHASE 2: COMMAND TESTS")
    print("="*60)

    py_commands = test_commands('localhost', 9900, name, password)
    rust_commands = test_commands('localhost', 9999, name, password)

    # Compare command results
    print(f"\nCommand Comparison:")
    for cmd in py_commands:
        py_res = py_commands[cmd]
        rust_res = rust_commands.get(cmd, {})
        py_status = "OK" if py_res['has_output'] and not py_res['is_error'] else "ERR"
        rust_status = "OK" if rust_res.get('has_output') and not rust_res.get('is_error') else "ERR"
        match = "MATCH" if (py_status == rust_status) else "DIFF"
        print(f"  {cmd:15s}: Python={py_status:5s} Rust={rust_status:5s} [{match}]")

    return True

if __name__ == "__main__":
    try:
        compare_servers()
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
