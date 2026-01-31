#!/usr/bin/env python3
"""
Combat System Test - Socket-based MUD client
"""

import socket
import time
import re
import json

class MUDClient:
    def __init__(self, host, port, name):
        self.host = host
        self.port = port
        self.name = name
        self.sock = None

    def connect(self):
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.host, self.port))
            return True
        except Exception as e:
            print(f"[{self.name}] Connect error: {e}")
            return False

    def send(self, text):
        if self.sock:
            try:
                data = (text + '\r\n').encode('utf-8')
                self.sock.sendall(data)
                return True
            except:
                return False
        return False

    def recv(self, timeout=1.0):
        if not self.sock:
            return ""
        data = b""
        self.sock.setblocking(False)
        start = time.time()
        while time.time() - start < timeout:
            try:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                data += chunk
                if len(chunk) < 4096:  # Got all data
                    break
            except BlockingIOError:
                time.sleep(0.05)
        self.sock.setblocking(True)
        return data.decode('utf-8', errors='ignore')

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass
            self.sock = None

def test_server(host, port, name, username, password):
    print(f"\n{'='*50}")
    print(f"Testing {name} ({host}:{port})")
    print(f"{'='*50}\n")

    client = MUDClient(host, port, name)
    if not client.connect():
        return None

    # Initial read
    time.sleep(0.5)
    init = client.recv(timeout=1.0)
    print(f"[{name}] Initial banner received: {len(init)} chars")

    # Login
    client.send(username)
    time.sleep(0.5)
    resp = client.recv(timeout=1.0)

    if '비번' in resp or 'assword' in resp.lower():
        client.send(password)
        time.sleep(1.0)
        resp = client.recv(timeout=1.0)

    # Check for duplicate login
    if '기존 접속' in resp or '종료' in resp:
        print(f"[{name}] Already logged in!")
        client.close()
        return {'server': name, 'error': 'already_logged_in'}

    # Check if we're in
    client.send('')
    time.sleep(0.5)
    resp = client.recv(timeout=1.0)

    results = {'server': name, 'responses': {}}

    # Status command
    client.send('상태')
    time.sleep(1.0)
    status = client.recv(timeout=1.0)
    results['responses']['status'] = status

    # Parse HP from prompt [450/900, 18/18]
    hp_match = re.search(r'\[\s*(\d+)\s*/\s*(\d+)\s*,\s*(\d+)\s*/\s*(\d+)\s*\]', status)
    if hp_match:
        results['hp'] = int(hp_match.group(1))
        results['max_hp'] = int(hp_match.group(2))
        results['level'] = int(hp_match.group(3))
        print(f"[{name}] HP: {results['hp']}/{results['max_hp']}, Level: {results['level']}")
    else:
        print(f"[{name}] Status output: {status[:200]}...")

    # Look command
    client.send('봐')
    time.sleep(1.0)
    look = client.recv(timeout=1.0)
    results['responses']['look'] = look
    print(f"[{name}] Look output: {look[:150]}...")

    # Attack command
    targets = ['토끼', '쥐', 'mob']
    for target in targets:
        client.send(f'공격 {target}')
        time.sleep(1.5)
        attack = client.recv(timeout=1.5)
        results['responses'][f'attack_{target}'] = attack

        # Parse damage
        damage_matches = re.findall(r'(\d+)\s*(?:피해|데미지|damage)', attack)
        if damage_matches:
            results['damage_dealt'] = [int(d) for d in damage_matches]
            print(f"[{name}] Attack {target}: Damage {damage_matches}")

            # Check for kill
            if '쓰러뜨렸' in attack or '경험치' in attack:
                print(f"[{name}] KILLED {target}!")
                results['kill'] = target
            break
        elif len(attack) > 20:
            print(f"[{name}] Attack {target}: {attack[:100]}...")
            results['last_attack_response'] = attack[:200]
            break

    # Flee command
    client.send('도망')
    time.sleep(1.0)
    flee = client.recv(timeout=1.0)
    results['responses']['flee'] = flee
    print(f"[{name}] Flee: {flee[:80]}...")

    # Learn command
    client.send('습득')
    time.sleep(1.0)
    learn = client.recv(timeout=1.0)
    results['responses']['learn'] = learn
    print(f"[{name}] Learn: {learn[:80]}...")

    client.close()
    return results

def main():
    print("="*50)
    print("MUD COMBAT SYSTEM TEST")
    print("="*50)

    # Test Python with 테스트파이 character
    py = test_server('localhost', 9900, 'PYTHON', '테스트파이', '1234')
    time.sleep(2)

    # Test Rust with 테스트러스트 character
    rust = test_server('localhost', 9999, 'RUST', '테스트러스트', '1234')

    # Report
    print("\n" + "="*60)
    print("COMBAT SYSTEM COMPARISON")
    print("="*60 + "\n")

    if py and rust:
        print("1. HP/STATUS:")
        print(f"   Python: HP {py.get('hp')}/{py.get('max_hp')}, Level {py.get('level')}")
        print(f"   Rust:   HP {rust.get('hp')}/{rust.get('max_hp')}, Level {rust.get('level')}")

        print("\n2. ATTACK DAMAGE:")
        print(f"   Python: {py.get('damage_dealt', [])}")
        print(f"   Rust:   {rust.get('damage_dealt', [])}")

        print("\n3. KILLS:")
        print(f"   Python: {py.get('kill', 'None')}")
        print(f"   Rust:   {rust.get('kill', 'None')}")

        print("\n4. FLEE RESPONSE:")
        print(f"   Python: {py.get('responses', {}).get('flee', '')[:80]}")
        print(f"   Rust:   {rust.get('responses', {}).get('flee', '')[:80]}")

        print("\n5. LEARN RESPONSE:")
        print(f"   Python: {py.get('responses', {}).get('learn', '')[:80]}")
        print(f"   Rust:   {rust.get('responses', {}).get('learn', '')[:80]}")

    with open('/home/ubuntu/muc-python3/combat_test_results.json', 'w') as f:
        json.dump({'python': py, 'rust': rust}, f, indent=2, ensure_ascii=False)
    print("\nResults saved to combat_test_results.json")

if __name__ == '__main__':
    main()
