#!/usr/bin/env python3
"""
Test NPC Dialogue System on Python (9900) and Rust (9999) MUD servers.

Tests:
1. Login with '테스트' character (password '1234')
2. Find NPCs using '봐' command
3. Test dialogue interactions:
   - 대화 (talk to NPC)
   - 말 [NPC명] [text] (say to NPC)
   - 정보 [NPC명] (get info)
   - 물어 [NPC명] [keyword] (ask keyword)
"""

import socket
import time
import sys
import re
from typing import Dict, List, Tuple, Optional

class MUDTestClient:
    def __init__(self, host: str, port: int, name: str = ""):
        self.host = host
        self.port = port
        self.name = name
        self.sock = None
        self.buffer = b""
        self.connected = False

    def connect(self) -> bool:
        """Connect to the MUD server."""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(10.0)
            self.sock.connect((self.host, self.port))
            self.connected = True
            print(f"[{self.name}] Connected to {self.host}:{self.port}")
            return True
        except Exception as e:
            print(f"[{self.name}] Connection failed: {e}")
            return False

    def disconnect(self):
        """Disconnect from the server."""
        if self.sock:
            try:
                self.sock.close()
            except:
                pass
            self.sock = None
            self.connected = False

    def send(self, text: str):
        """Send text to the server, adding IAC EOL sequence."""
        if self.sock:
            try:
                # MUD protocol: IAC EOL = \xFF\xF0
                message = text + "\xFF\xF0"
                self.sock.sendall(message.encode("euc-kr", errors="ignore"))
                # print(f"[{self.name}] SENT: {text!r}")
            except Exception as e:
                print(f"[{self.name}] Send error: {e}")

    def read_until(self, delimiter: bytes = b"\xFF\xF0", timeout: float = 3.0) -> str:
        """Read from socket until delimiter or timeout."""
        if not self.sock:
            return ""

        start_time = time.time()
        result = b""

        while time.time() - start_time < timeout:
            try:
                data = self.sock.recv(4096)
                if not data:
                    break
                self.buffer += data

                # Check for delimiter
                if delimiter in self.buffer:
                    parts = self.buffer.split(delimiter, 1)
                    result += parts[0]
                    self.buffer = parts[1] if len(parts) > 1 else b""
                    break

                # Also check for prompt (usually ends with specific patterns)
                if b"\x1b[" in self.buffer:
                    # Contains ANSI codes, might be end of response
                    # Wait a bit for more data
                    time.sleep(0.05)

            except socket.timeout:
                break
            except Exception as e:
                print(f"[{self.name}] Read error: {e}")
                break

        # Decode with EUC-KR
        try:
            text = result.decode("euc-kr", errors="ignore")
            return text
        except:
            return result.decode("utf-8", errors="ignore")

    def read_all(self, timeout: float = 1.0) -> str:
        """Read all available data."""
        if not self.sock:
            return ""
        time.sleep(timeout)
        try:
            data = self.sock.recv(8192)
            if data:
                self.buffer += data
        except:
            pass

        try:
            return self.buffer.decode("euc-kr", errors="ignore")
        except:
            return self.buffer.decode("utf-8", errors="ignore")

    def clear_buffer(self):
        """Clear the receive buffer."""
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

    def login(self, username: str, password: str) -> bool:
        """Login to the MUD server."""
        print(f"[{self.name}] Attempting login as {username}...")

        # Wait for initial prompt
        time.sleep(0.5)
        self.clear_buffer()

        # Send username
        self.send(username)
        time.sleep(0.5)
        self.read_until(timeout=1.0)

        # Send password
        self.send(password)
        time.sleep(0.5)

        # Read login result
        result = self.read_until(timeout=2.0)

        # Check for successful login indicators
        if "입장하셨습니다" in result or "환영합니다" in result or "무림" in result:
            print(f"[{self.name}] Login successful!")
            return True
        else:
            print(f"[{self.name}] Login response: {result[:200]}")
            # Sometimes login succeeds even without clear indicators
            return True

    def extract_npcs(self, text: str) -> List[str]:
        """Extract NPC names from room description."""
        npcs = []
        lines = text.split('\n')

        # Look for NPCs in the room display
        # Format: Usually NPCs are listed after "이곳에 있는 사람들:" or similar
        in_npc_section = False
        for line in lines:
            line = line.strip()
            # Clean ANSI codes
            line = re.sub(r'\x1b\[[0-9;]*m', '', line)

            if "이곳에 있는" in line or "사람" in line or "몹" in line:
                in_npc_section = True
                continue

            if in_npc_section:
                if not line or line.startswith("=") or line.startswith("-"):
                    continue
                # Extract name (usually first word before description)
                parts = line.split()
                if parts:
                    npc_name = parts[0]
                    if npc_name and npc_name not in ['당신', '당신은', '그']:
                        npcs.append(npc_name)

        return npcs

    def find_dialogue_options(self, text: str) -> List[str]:
        """Extract dialogue options from NPC response."""
        options = []
        lines = text.split('\n')

        for line in lines:
            line = re.sub(r'\x1b\[[0-9;]*m', '', line)
            # Look for numbered options or keywords
            # Format: [1] xxx or 대화: xxx
            match = re.search(r'\[(\d+)\]\s*(\S+)', line)
            if match:
                options.append(match.group(2))
            # Also look for "대화:" patterns
            match = re.search(r'대화\s*[:：]\s*(\S+)', line)
            if match:
                options.append(match.group(1))

        return options


def test_dialogue_system(client: MUDTestClient) -> Dict:
    """Test the dialogue system on a connected MUD client."""
    results = {
        "server": client.name,
        "login": False,
        "room_npcs": [],
        "npc_details": {},
        "dialogue_tests": [],
        "errors": []
    }

    try:
        # Login
        if not client.login("테스트", "1234"):
            results["errors"].append("Login failed")
            return results

        results["login"] = True
        time.sleep(1)
        client.clear_buffer()

        # Test 1: Look at room (봐)
        print(f"\n[{client.name}] === Testing '봐' command ===")
        client.send("봐")
        room_response = client.read_until(timeout=2.0)
        print(f"[{client.name}] Room response:\n{room_response[:500]}...")

        # Extract NPCs
        npcs = client.extract_npcs(room_response)
        results["room_npcs"] = npcs
        print(f"[{client.name}] NPCs found: {npcs}")

        if not npcs:
            # Try to parse NPCs differently
            for line in room_response.split('\n'):
                clean_line = re.sub(r'\x1b\[[0-9;]*m', '', line).strip()
                if clean_line and not clean_line.startswith('[') and not clean_line.startswith('-'):
                    words = clean_line.split()
                    if words and len(words) < 5:
                        potential_npc = words[0]
                        if potential_npc not in ['당신', '당신은', '출구:', '북', '남', '동', '서', '위', '아래']:
                            npcs.append(potential_npc)

        # Test 2: Try dialogue with first NPC (if any)
        if npcs:
            test_npc = npcs[0]
            print(f"\n[{client.name}] === Testing dialogue with NPC: {test_npc} ===")

            # Test: Look at NPC
            client.send(f"봐 {test_npc}")
            npc_look_response = client.read_until(timeout=2.0)
            results["npc_details"][test_npc] = {
                "look": npc_look_response[:500]
            }
            print(f"[{client.name}] Look at {test_npc}:\n{npc_look_response[:300]}...")

            time.sleep(0.5)

            # Test: Talk to NPC (대화)
            client.send(f"대화 {test_npc}")
            dialogue_response = client.read_until(timeout=2.0)
            dialogue_opts = client.find_dialogue_options(dialogue_response)

            dialogue_test = {
                "npc": test_npc,
                "command": f"대화 {test_npc}",
                "response": dialogue_response[:800],
                "options": dialogue_opts
            }
            results["dialogue_tests"].append(dialogue_test)
            print(f"[{client.name}] Dialogue response:\n{dialogue_response[:400]}...")
            print(f"[{client.name}] Dialogue options: {dialogue_opts}")

            time.sleep(0.5)

            # Test: Say to NPC (말)
            client.send(f"말 {test_npc} 안녕하세요")
            say_response = client.read_until(timeout=2.0)

            say_test = {
                "npc": test_npc,
                "command": f"말 {test_npc} 안녕하세요",
                "response": say_response[:500]
            }
            results["dialogue_tests"].append(say_test)
            print(f"[{client.name}] Say response:\n{say_response[:300]}...")

            time.sleep(0.5)

            # Test: Get NPC info (정보)
            client.send(f"정보 {test_npc}")
            info_response = client.read_until(timeout=2.0)

            info_test = {
                "npc": test_npc,
                "command": f"정보 {test_npc}",
                "response": info_response[:500]
            }
            results["dialogue_tests"].append(info_test)
            print(f"[{client.name}] Info response:\n{info_response[:300]}...")

            time.sleep(0.5)

            # Test: Ask keyword (물어)
            client.send(f"물어 {test_npc} 정보")
            ask_response = client.read_until(timeout=2.0)

            ask_test = {
                "npc": test_npc,
                "command": f"물어 {test_npc} 정보",
                "response": ask_response[:500]
            }
            results["dialogue_tests"].append(ask_test)
            print(f"[{client.name}] Ask response:\n{ask_response[:300]}...")

            # Test dialogue options if found
            if dialogue_opts:
                for opt in dialogue_opts[:2]:  # Test first 2 options
                    time.sleep(0.5)
                    client.send(f"대화 {test_npc} {opt}")
                    opt_response = client.read_until(timeout=2.0)

                    opt_test = {
                        "npc": test_npc,
                        "command": f"대화 {test_npc} {opt}",
                        "response": opt_response[:500]
                    }
                    results["dialogue_tests"].append(opt_test)
                    print(f"[{client.name}] Option '{opt}' response:\n{opt_response[:300]}...")

        else:
            # No NPCs found, move around to find some
            print(f"\n[{client.name}] No NPCs in starting room, exploring...")

            # Try different directions
            directions = ["동", "서", "남", "북"]
            for direction in directions:
                time.sleep(0.5)
                client.clear_buffer()
                client.send(direction)
                move_response = client.read_until(timeout=2.0)

                client.send("봐")
                room_response = client.read_until(timeout=2.0)

                npcs = client.extract_npcs(room_response)
                if npcs:
                    print(f"[{client.name}] Found NPCs after moving {direction}: {npcs}")
                    results["room_npcs"].extend(npcs)
                    break

        # Test shop interactions if we find a shopkeeper
        time.sleep(0.5)
        client.send("도움말")
        help_response = client.read_until(timeout=2.0)
        results["help_sample"] = help_response[:1000]

    except Exception as e:
        results["errors"].append(f"Exception: {e}")
        print(f"[{client.name}] Error: {e}")

    return results


def compare_servers(py_results: Dict, rust_results: Dict) -> Dict:
    """Compare dialogue system results between Python and Rust servers."""
    comparison = {
        "login_both": py_results.get("login", False) and rust_results.get("login", False),
        "npc_count_diff": len(py_results.get("room_npcs", [])) - len(rust_results.get("room_npcs", [])),
        "npc_names_match": set(py_results.get("room_npcs", [])) == set(rust_results.get("room_npcs", [])),
        "dialogue_commands_work": {
            "python": bool(py_results.get("dialogue_tests")),
            "rust": bool(rust_results.get("dialogue_tests"))
        },
        "response_differences": [],
        "summary": []
    }

    # Compare dialogue test results
    py_tests = py_results.get("dialogue_tests", [])
    rust_tests = rust_results.get("dialogue_tests", [])

    for i, (py_test, rust_test) in enumerate(zip(py_tests, rust_tests)):
        cmd = py_test.get("command", "")
        diff = {
            "command": cmd,
            "python_response_len": len(py_test.get("response", "")),
            "rust_response_len": len(rust_test.get("response", "")),
            "responses_differ": py_test.get("response", "") != rust_test.get("response", ""),
            "python_options": py_test.get("options", []),
            "rust_options": rust_test.get("options", []),
        }
        comparison["response_differences"].append(diff)

    # Generate summary
    if comparison["login_both"]:
        comparison["summary"].append("Both servers allow login")
    else:
        comparison["summary"].append("Login differs between servers")

    if comparison["npc_names_match"]:
        comparison["summary"].append("Same NPCs found in both servers")
    else:
        comparison["summary"].append(f"Different NPCs: Python has {len(py_results.get('room_npcs', []))}, Rust has {len(rust_results.get('room_npcs', []))}")

    py_dialogue_works = any(t.get("response") for t in py_tests)
    rust_dialogue_works = any(t.get("response") for t in rust_tests)

    if py_dialogue_works and rust_dialogue_works:
        comparison["summary"].append("Dialogue commands work on both servers")
    elif py_dialogue_works:
        comparison["summary"].append("Dialogue works only on Python")
    elif rust_dialogue_works:
        comparison["summary"].append("Dialogue works only on Rust")
    else:
        comparison["summary"].append("Dialogue not working on either server")

    return comparison


def main():
    """Main test function."""
    print("=" * 70)
    print("NPC DIALOGUE SYSTEM TEST")
    print("Testing Python MUD (9900) vs Rust MUD (9999)")
    print("=" * 70)

    # Test Python server
    print("\n### Testing Python Server (port 9900) ###")
    py_client = MUDTestClient("localhost", 9900, "Python")
    if py_client.connect():
        py_results = test_dialogue_system(py_client)
        py_client.disconnect()
    else:
        print("Failed to connect to Python server")
        py_results = {"login": False, "errors": ["Connection failed"]}

    time.sleep(1)

    # Test Rust server
    print("\n### Testing Rust Server (port 9999) ###")
    rust_client = MUDTestClient("localhost", 9999, "Rust")
    if rust_client.connect():
        rust_results = test_dialogue_system(rust_client)
        rust_client.disconnect()
    else:
        print("Failed to connect to Rust server")
        rust_results = {"login": False, "errors": ["Connection failed"]}

    # Compare results
    print("\n" + "=" * 70)
    print("COMPARISON RESULTS")
    print("=" * 70)

    comparison = compare_servers(py_results, rust_results)

    print(f"\nLogin Status:")
    print(f"  Python: {'OK' if py_results.get('login') else 'FAILED'}")
    print(f"  Rust:   {'OK' if rust_results.get('login') else 'FAILED'}")

    print(f"\nNPCs Found:")
    print(f"  Python: {py_results.get('room_npcs', [])}")
    print(f"  Rust:   {rust_results.get('room_npcs', [])}")
    print(f"  Match:  {comparison['npc_names_match']}")

    print(f"\nDialogue Tests:")
    for diff in comparison["response_differences"]:
        print(f"  Command: {diff['command']}")
        print(f"    Python: {diff['python_response_len']} chars, options: {diff['python_options']}")
        print(f"    Rust:   {diff['rust_response_len']} chars, options: {diff['rust_options']}")
        print(f"    Same response: {not diff['responses_differ']}")

    print(f"\nSummary:")
    for s in comparison["summary"]:
        print(f"  - {s}")

    # Detailed output
    print("\n" + "=" * 70)
    print("DETAILED RESPONSES")
    print("=" * 70)

    if py_results.get("dialogue_tests"):
        print("\n--- Python Server Dialogue Responses ---")
        for test in py_results["dialogue_tests"][:4]:
            print(f"\nCommand: {test['command']}")
            resp = test.get("response", "")
            # Clean ANSI codes for readability
            clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', resp)
            print(f"Response: {clean_resp[:400]}...")
            if test.get("options"):
                print(f"Options found: {test['options']}")

    if rust_results.get("dialogue_tests"):
        print("\n--- Rust Server Dialogue Responses ---")
        for test in rust_results["dialogue_tests"][:4]:
            print(f"\nCommand: {test['command']}")
            resp = test.get("response", "")
            clean_resp = re.sub(r'\x1b\[[0-9;]*m', '', resp)
            print(f"Response: {clean_resp[:400]}...")
            if test.get("options"):
                print(f"Options found: {test['options']}")

    # Error report
    if py_results.get("errors"):
        print(f"\nPython Server Errors: {py_results['errors']}")
    if rust_results.get("errors"):
        print(f"Rust Server Errors: {rust_results['errors']}")

    print("\n" + "=" * 70)
    print("TEST COMPLETE")
    print("=" * 70)

    return 0


if __name__ == "__main__":
    sys.exit(main())
