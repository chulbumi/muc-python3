#!/usr/bin/env python3
"""
Detailed NPC Dialogue Test - Navigate to specific room with NPC and test dialogue.

Tests Python (9900) vs Rust (9999) MUD servers.
"""

import socket
import time
import sys
import re

class MUDClient:
    def __init__(self, host, port, name):
        self.host = host
        self.port = port
        self.name = name
        self.sock = None
        self.buffer = b""

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(10.0)
        self.sock.connect((self.host, self.port))
        print(f"[{self.name}] Connected")

    def disconnect(self):
        if self.sock:
            self.sock.close()

    def send(self, text):
        if self.sock:
            try:
                message = text + "\xFF\xF0"
                self.sock.sendall(message.encode("euc-kr", errors="ignore"))
            except:
                pass

    def read(self, timeout=2.0):
        if not self.sock:
            return ""
        time.sleep(timeout)
        try:
            data = self.sock.recv(8192)
        except socket.timeout:
            return ""
        if data:
            self.buffer += data

        try:
            return self.buffer.decode("euc-kr", errors="ignore")
        except:
            return self.buffer.decode("utf-8", errors="ignore")

    def clear_buffer(self):
        self.buffer = b""
        try:
            self.sock.settimeout(0.1)
            while True:
                data = self.sock.recv(4096)
                if not data:
                    break
        except:
            pass
        finally:
            self.sock.settimeout(5.0)

    def login(self, username, password):
        time.sleep(0.5)
        self.clear_buffer()
        self.send(username)
        time.sleep(0.5)
        self.read(timeout=0.5)
        self.send(password)
        time.sleep(1.0)
        result = self.read(timeout=1.0)
        return "입장" in result or "환영" in result or "무림" in result


def navigate_to_npc(client, server_name):
    """Navigate to the room with 무성호법 (room 106)."""
    print(f"\n[{server_name}] === Navigating to NPC location ===")

    # Path from starting room (1) to room 106
    # 1 -> 동(2) -> ...
    # Let's try a simpler approach - teleport if admin, or walk

    commands = []

    # Try teleport command first
    commands.append("이동 하북성 106")  # Try zone:room teleport
    commands.append("106이동")          # Try simple teleport
    commands.append("봐")              # Look around

    results = {}

    for cmd in commands:
        time.sleep(0.5)
        client.clear_buffer()
        client.send(cmd)
        response = client.read(timeout=1.5)
        clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
        results[cmd] = response
        print(f"[{server_name}] Command '{cmd}' response preview: {clean_resp[:200]}...")

        # Check if we found NPC
        if "무성호법" in response or "호법" in response:
            print(f"[{server_name}] Found NPC '무성호법'!")
            return True, response

        # Check if we're in the right room
        if "무극전" in response or "석가장" in response:
            print(f"[{server_name}] Arrived at 석가장 area")
            client.send("봐")
            response = client.read(timeout=1.5)
            if "무성호법" in response:
                print(f"[{server_name}] Found NPC '무성호법'!")
                return True, response

    # Try walking path
    print(f"[{server_name}] Teleport failed, trying to walk...")
    client.clear_buffer()

    # Walk from room 1: 1 -> 동(2) -> 남(24) -> ... toward 석가장
    walk_commands = ["동", "남", "동", "동", "동", "동", "동", "북", "북"]

    for cmd in walk_commands:
        time.sleep(0.5)
        client.send(cmd)
        client.read(timeout=0.8)

    client.send("봐")
    response = client.read(timeout=1.5)

    if "무성호법" in response:
        print(f"[{server_name}] Found NPC after walking!")
        return True, response

    return False, response


def test_dialogue(client, server_name, npc_name="무성호법"):
    """Test dialogue commands with NPC."""
    print(f"\n[{server_name}] === Testing Dialogue with {npc_name} ===")

    tests = []

    # Test 1: Look at NPC
    time.sleep(0.5)
    client.clear_buffer()
    client.send(f"봐 {npc_name}")
    response = client.read(timeout=2.0)
    tests.append(("look", f"봐 {npc_name}", response))
    clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
    print(f"[{server_name}] LOOK response: {clean_resp[:300]}...")

    # Test 2: Try dialogue command (format: [NPC] 대화)
    time.sleep(0.5)
    client.clear_buffer()
    client.send(f"{npc_name} 대화")
    response = client.read(timeout=2.0)
    tests.append(("dialogue", f"{npc_name} 대화", response))
    clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
    print(f"[{server_name}] DIALOGUE response: {clean_resp[:400]}...")

    # Test 3: Try "예" response (if prompted)
    if "예" in response or "아니오" in response:
        time.sleep(0.5)
        client.clear_buffer()
        client.send(f"{npc_name} 대화 예")
        response = client.read(timeout=2.0)
        tests.append(("dialogue_yes", f"{npc_name} 대화 예", response))
        clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
        print(f"[{server_name}] DIALOGUE YES response: {clean_resp[:400]}...")

    # Test 4: Try "말" command (say to NPC)
    time.sleep(0.5)
    client.clear_buffer()
    client.send(f"말 {npc_name} 안녕하세요")
    response = client.read(timeout=2.0)
    tests.append(("say", f"말 {npc_name} 안녕하세요", response))
    clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
    print(f"[{server_name}] SAY response: {clean_resp[:300]}...")

    # Test 5: Try "정보" command
    time.sleep(0.5)
    client.clear_buffer()
    client.send(f"정보 {npc_name}")
    response = client.read(timeout=2.0)
    tests.append(("info", f"정보 {npc_name}", response))
    clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
    print(f"[{server_name}] INFO response: {clean_resp[:300]}...")

    # Test 6: Try different dialogue formats
    for cmd_format in [f"대화 {npc_name}", f"{npc_name} 말 정보"]:
        time.sleep(0.5)
        client.clear_buffer()
        client.send(cmd_format)
        response = client.read(timeout=2.0)
        tests.append(("other", cmd_format, response))
        clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', response)
        print(f"[{server_name}] Command '{cmd_format}': {clean_resp[:200]}...")

    return tests


def main():
    print("=" * 70)
    print("NPC DIALOGUE SYSTEM DETAILED TEST")
    print("=" * 70)

    # Test Python server
    print("\n### Testing Python Server (9900) ###")
    py_client = MUDClient("localhost", 9900, "Python")
    py_client.connect()
    py_login = py_client.login("테스트", "1234")
    print(f"[Python] Login: {'OK' if py_login else 'FAILED'}")

    py_found_npc = False
    py_room_response = ""
    py_tests = []

    if py_login:
        py_found_npc, py_room_response = navigate_to_npc(py_client, "Python")
        if py_found_npc:
            py_tests = test_dialogue(py_client, "Python")
        else:
            # Try anyway with current location
            py_tests = test_dialogue(py_client, "Python")

    py_client.disconnect()
    time.sleep(1)

    # Test Rust server
    print("\n### Testing Rust Server (9999) ###")
    rust_client = MUDClient("localhost", 9999, "Rust")
    rust_client.connect()
    rust_login = rust_client.login("테스트", "1234")
    print(f"[Rust] Login: {'OK' if rust_login else 'FAILED'}")

    rust_found_npc = False
    rust_room_response = ""
    rust_tests = []

    if rust_login:
        rust_found_npc, rust_room_response = navigate_to_npc(rust_client, "Rust")
        if rust_found_npc:
            rust_tests = test_dialogue(rust_client, "Rust")
        else:
            rust_tests = test_dialogue(rust_client, "Rust")

    rust_client.disconnect()

    # Compare results
    print("\n" + "=" * 70)
    print("COMPARISON RESULTS")
    print("=" * 70)

    print(f"\nLogin Status:")
    print(f"  Python: {py_login}")
    print(f"  Rust:   {rust_login}")

    print(f"\nNPC Found (무성호법):")
    print(f"  Python: {py_found_npc}")
    print(f"  Rust:   {rust_found_npc}")

    print(f"\nRoom Response Preview:")
    py_clean = re.sub(r'\x1b\[[0-9;]*m', '', py_room_response)
    rust_clean = re.sub(r'\x1b\[[0-9;]*m', '', rust_room_response)
    print(f"  Python: {py_clean[:300]}...")
    print(f"  Rust:   {rust_clean[:300]}...")

    print(f"\nDialogue Test Comparison:")
    print(f"  Python tests: {len(py_tests)}")
    print(f"  Rust tests: {len(rust_tests)}")

    for i, (py_test, rust_test) in enumerate(zip(py_tests, rust_tests)):
        py_type, py_cmd, py_resp = py_test
        rust_type, rust_cmd, rust_resp = rust_test

        py_clean = re.sub(r'\x1b\[[0-9;]*m', '', py_resp)
        rust_clean = re.sub(r'\x1b\[[0-9;]*m', '', rust_resp)

        print(f"\n  Test {i+1}: {py_cmd}")
        print(f"    Python ({len(py_resp)} chars): {py_clean[:200]}...")
        print(f"    Rust   ({len(rust_resp)} chars): {rust_clean[:200]}...")

        if py_resp == rust_resp:
            print(f"    Status: IDENTICAL")
        else:
            print(f"    Status: DIFFERENT")

            # Check for specific dialogue elements
            for element in ["시험", "무성호법이 말합니다", "예", "아니오", "장주"]:
                py_has = element in py_resp
                rust_has = element in rust_resp
                if py_has != rust_has:
                    print(f"      Element '{element}': Python={py_has}, Rust={rust_has}")

    # Final summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    py_dialogue_works = any("시험" in t[2] or "무성호법이 말" in t[2] for t in py_tests)
    rust_dialogue_works = any("시험" in t[2] or "무성호법이 말" in t[2] for t in rust_tests)

    print(f"Dialogue system working:")
    print(f"  Python: {py_dialogue_works}")
    print(f"  Rust:   {rust_dialogue_works}")

    print("\n" + "=" * 70)

    return 0


if __name__ == "__main__":
    sys.exit(main())
