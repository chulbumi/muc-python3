#!/usr/bin/env python3
"""
Combat System Test Script
Manual interactive test for comparing Python (9900) and Rust (9999) MUD combat
"""

import socket
import time
import re
import json

# Colors for output
class Colors:
    PYTHON = '\033[94m'  # Blue
    RUST = '\033[92m'    # Green
    BOTH = '\033[93m'    # Yellow
    RESET = '\033[0m'
    BOLD = '\033[1m'

class SimpleMUDClient:
    def __init__(self, host='localhost', port=9900, name='PYTHON'):
        self.host = host
        self.port = port
        self.name = name
        self.sock = None

    def connect(self):
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.host, self.port))
            print(f"[{self.name}] Connected to {self.host}:{self.port}")
            return True
        except Exception as e:
            print(f"[{self.name}] Connection failed: {e}")
            return False

    def send(self, data):
        try:
            self.sock.sendall((data + '\r\n').encode('utf-8'))
        except:
            pass

    def recv(self, timeout=0.5):
        try:
            self.sock.setblocking(False)
            data = b""
            deadline = time.time() + timeout
            while time.time() < deadline:
                try:
                    chunk = self.sock.recv(4096)
                    if not chunk:
                        break
                    data += chunk
                except BlockingIOError:
                    time.sleep(0.05)
            self.sock.setblocking(True)
            return data.decode('utf-8', errors='ignore')
        except:
            return ""

    def close(self):
        if self.sock:
            self.sock.close()

def test_server(port, name, username, password):
    """Test a single server"""
    print(f"\n{Colors.BOLD}{'='*50}")
    print(f"Testing {name} Server (port {port})")
    print(f"{'='*50}{Colors.RESET}\n")

    client = SimpleMUDClient('localhost', port, name)
    if not client.connect():
        return None

    # Initial read
    time.sleep(0.5)
    client.recv()

    # Login
    client.send(username)
    time.sleep(0.5)
    client.recv()

    client.send(password)
    time.sleep(1.0)

    # Read login response
    output = client.recv(timeout=1.0)

    results = {
        'server': name,
        'port': port,
        'login_output': output,
        'status_output': '',
        'look_output': '',
        'attack_outputs': [],
        'flee_output': '',
        'learn_output': '',
    }

    # Check for duplicate login
    if '기존 접속' in output or '로그인' in output:
        if '기존 접속' in output:
            print(f"[{name}] Character already logged in!")
            client.close()
            return results

    # Get status
    client.send('상태')
    time.sleep(1.0)
    status = client.recv(timeout=1.0)
    results['status_output'] = status

    # Parse HP and Level from prompt
    prompt_match = re.search(r'\[\s*(\d+)\s*/\s*(\d+)\s*,\s*(\d+)\s*/\s*(\d+)\s*\]', status)
    if prompt_match:
        hp = int(prompt_match.group(1))
        max_hp = int(prompt_match.group(2))
        level = int(prompt_match.group(3))
        print(f"[{name}] HP: {hp}/{max_hp}, Level: {level}")
        results['hp'] = hp
        results['max_hp'] = max_hp
        results['level'] = level

    # Look for mobs
    client.send('봐')
    time.sleep(1.0)
    look = client.recv(timeout=1.0)
    results['look_output'] = look
    print(f"[{name}] Room preview: {look[:150]}...")

    # Try attack commands
    attack_targets = ['토끼', '늑대', '쥐', '고양이', '좀비', '사슴', '쥐', 'rat', 'rabbit', 'mob']

    for target in attack_targets:
        client.send(f'공격 {target}')
        time.sleep(1.5)
        attack_output = client.recv(timeout=1.5)
        results['attack_outputs'].append({'target': target, 'output': attack_output})

        # Check for damage numbers
        damage_nums = re.findall(r'(\d+)\s*(?:데미지|피해|damage|DMG)', attack_output)
        if damage_nums:
            print(f"[{name}] Attack '{target}': damage numbers found: {damage_nums}")
            # Check for kill messages
            if '쓰러뜨렸' in attack_output or '죽였' in attack_output:
                print(f"[{name}] KILLED the target!")
                results['kill'] = target
                break
        else:
            print(f"[{name}] Attack '{target}': {attack_output[:80]}...")

        # Check if in combat
        if '이미' in attack_output or '없는것' in attack_output or '강호' in attack_output:
            continue

        # If we got a combat response, break
        if len(attack_output) > 20:
            break

    # Check status after combat
    client.send('상태')
    time.sleep(1.0)
    after_status = client.recv(timeout=1.0)
    prompt_match = re.search(r'\[\s*(\d+)\s*/\s*(\d+)\s*,\s*(\d+)\s*/\s*(\d+)\s*\]', after_status)
    if prompt_match:
        hp = int(prompt_match.group(1))
        level = int(prompt_match.group(3))
        print(f"[{name}] After combat - HP: {hp}, Level: {level}")
        results['after_hp'] = hp
        results['after_level'] = level

    # Test flee
    client.send('도망')
    time.sleep(1.0)
    flee = client.recv(timeout=1.0)
    results['flee_output'] = flee
    print(f"[{name}] Flee: {flee[:80]}...")

    # Test learn
    client.send('습득')
    time.sleep(1.0)
    learn = client.recv(timeout=1.0)
    results['learn_output'] = learn
    print(f"[{name}] Learn: {learn[:80]}...")

    client.close()
    return results

def main():
    print(f"{Colors.BOLD}{Colors.BOTH}MUD COMBAT SYSTEM TEST")
    print(f"Testing Python (9900) vs Rust (9999){Colors.RESET}")

    # Test Python server
    py_results = test_server(9900, 'PYTHON', '테스트', '1234')

    # Wait a bit
    time.sleep(2)

    # Test Rust server
    rust_results = test_server(9999, 'RUST', '테스트', '1234')

    # Comparison
    print(f"\n{Colors.BOLD}{Colors.BOTH}{'='*60}")
    print(f"COMBAT SYSTEM COMPARISON REPORT")
    print(f"{'='*60}{Colors.RESET}\n")

    if py_results and rust_results:
        print(f"{Colors.BOLD}1. STATUS COMMAND{Colors.RESET}")
        print(f"  Python HP: {py_results.get('hp')}/{py_results.get('max_hp')}")
        print(f"  Rust HP:   {rust_results.get('hp')}/{rust_results.get('max_hp')}")

        print(f"\n{Colors.BOLD}2. ATTACK COMMAND{Colors.RESET}")
        print(f"  Python attack attempts:")
        for attempt in py_results.get('attack_outputs', [])[:3]:
            print(f"    - {attempt['target']}: {attempt['output'][:60]}...")

        print(f"  Rust attack attempts:")
        for attempt in rust_results.get('attack_outputs', [])[:3]:
            print(f"    - {attempt['target']}: {attempt['output'][:60]}...")

        print(f"\n{Colors.BOLD}3. FLEE COMMAND{Colors.RESET}")
        print(f"  Python: {py_results.get('flee_output', '')[:80]}")
        print(f"  Rust:   {rust_results.get('flee_output', '')[:80]}")

        print(f"\n{Colors.BOLD}4. LEARN COMMAND{Colors.RESET}")
        print(f"  Python: {py_results.get('learn_output', '')[:80]}")
        print(f"  Rust:   {rust_results.get('learn_output', '')[:80]}")

    # Save results
    with open('/home/ubuntu/muc-python3/combat_test_results.json', 'w', encoding='utf-8') as f:
        json.dump({'python': py_results, 'rust': rust_results}, f, indent=2, ensure_ascii=False)

if __name__ == '__main__':
    main()
