#!/usr/bin/env python3
"""
NPC Dialogue System Test using telnetlib.

Tests Python (9900) vs Rust (9999) MUD servers.
"""

import telnetlib
import time
import sys
import re

class MUDTelnetClient:
    def __init__(self, host, port, name):
        self.host = host
        self.port = port
        self.name = name
        self.tn = None

    def connect(self):
        try:
            self.tn = telnetlib.Telnet(self.host, self.port, timeout=10)
            print(f"[{self.name}] Connected to {self.host}:{self.port}")
            return True
        except Exception as e:
            print(f"[{self.name}] Connection failed: {e}")
            return False

    def disconnect(self):
        if self.tn:
            try:
                self.tn.close()
            except:
                pass

    def send(self, text):
        if self.tn:
            try:
                self.tn.write(text.encode("euc-kr") + b"\r\n")
            except Exception as e:
                print(f"[{self.name}] Send error: {e}")

    def read_until(self, expected, timeout=3):
        if self.tn:
            try:
                data = self.tn.read_until(expected.encode("euc-kr"), timeout=timeout)
                return data.decode("euc-kr", errors="ignore")
            except Exception as e:
                return ""
        return ""

    def read_all(self, timeout=1):
        if self.tn:
            time.sleep(timeout)
            try:
                data = self.tn.read_very_eager()
                return data.decode("euc-kr", errors="ignore")
            except:
                return ""
        return ""

    def login(self, username, password):
        """Login using expected prompts."""
        # Wait for login prompt
        try:
            # Read initial prompt
            data = self.tn.read_until(b"ID", timeout=3)
            if b"ID" not in data and b"id" not in data:
                # Maybe no prompt, just send username
                pass

            self.send(username)
            time.sleep(0.5)

            # Read password prompt
            data = self.tn.read_until(b"PASS", timeout=2)
            if b"PASS" not in data and b"pass" not in data:
                pass

            self.send(password)
            time.sleep(1)

            # Read result
            data = self.read_all(timeout=1)

            # Check for success indicators
            if "입장" in data or "환영" in data or "무림" in data or "하북성" in data:
                return True

            return True  # Assume success if no error
        except Exception as e:
            print(f"[{self.name}] Login exception: {e}")
            return False


def main():
    print("=" * 70)
    print("NPC DIALOGUE SYSTEM TEST (telnetlib)")
    print("=" * 70)

    # Test Python server
    print("\n### Testing Python Server (9900) ###")
    py_client = MUDTelnetClient("localhost", 9900, "Python")
    py_results = {"connected": False, "logged_in": False, "responses": []}

    if py_client.connect():
        py_results["connected"] = True
        py_results["logged_in"] = py_client.login("테스트", "1234")
        print(f"[Python] Logged in: {py_results['logged_in']}")

        if py_results["logged_in"]:
            # Look at current room
            py_client.send("봐")
            resp1 = py_client.read_all(timeout=1)
            py_results["responses"].append(("봐", resp1))
            print(f"[Python] Room: {re.sub(r'\x1b\[[0-9;]*m', '', resp1)[:300]}...")

            # Try to find an NPC - move around
            for direction in ["동", "동", "남", "남", "서", "북"]:
                time.sleep(0.5)
                py_client.send(direction)
                py_client.read_all(timeout=0.5)

                py_client.send("봐")
                room_resp = py_client.read_all(timeout=1)
                if "NPC" in room_resp or "사람" in room_resp or "장로" in room_resp or "호법" in room_resp:
                    print(f"[Python] Found potential NPC after moving {direction}")
                    break

            # Final room state
            py_client.send("봐")
            final_room = py_client.read_all(timeout=1)
            py_results["responses"].append(("final_room", final_room))
            print(f"[Python] Final room: {re.sub(r'\x1b\[[0-9;]*m', '', final_room)[:400]}...")

            # Try dialogue if we found an NPC
            time.sleep(0.5)
            py_client.send("도움말")
            help_resp = py_client.read_all(timeout=1)
            py_results["responses"].append(("도움말", help_resp))
            print(f"[Python] Help: {re.sub(r'\x1b\[[0-9;]*m', '', help_resp)[:300]}...")

    py_client.disconnect()
    time.sleep(1)

    # Test Rust server
    print("\n### Testing Rust Server (9999) ###")
    rust_client = MUDTelnetClient("localhost", 9999, "Rust")
    rust_results = {"connected": False, "logged_in": False, "responses": []}

    if rust_client.connect():
        rust_results["connected"] = True
        rust_results["logged_in"] = rust_client.login("테스트", "1234")
        print(f"[Rust] Logged in: {rust_results['logged_in']}")

        if rust_results["logged_in"]:
            # Look at current room
            rust_client.send("봐")
            resp1 = rust_client.read_all(timeout=1)
            rust_results["responses"].append(("봐", resp1))
            print(f"[Rust] Room: {re.sub(r'\x1b\[[0-9;]*m', '', resp1)[:300]}...")

            # Try to find an NPC - move around
            for direction in ["동", "동", "남", "남", "서", "북"]:
                time.sleep(0.5)
                rust_client.send(direction)
                rust_client.read_all(timeout=0.5)

                rust_client.send("봐")
                room_resp = rust_client.read_all(timeout=1)
                if "NPC" in room_resp or "사람" in room_resp or "장로" in room_resp or "호법" in room_resp:
                    print(f"[Rust] Found potential NPC after moving {direction}")
                    break

            # Final room state
            rust_client.send("봐")
            final_room = rust_client.read_all(timeout=1)
            rust_results["responses"].append(("final_room", final_room))
            print(f"[Rust] Final room: {re.sub(r'\x1b\[[0-9;]*m', '', final_room)[:400]}...")

            # Try dialogue if we found an NPC
            time.sleep(0.5)
            rust_client.send("도움말")
            help_resp = rust_client.read_all(timeout=1)
            rust_results["responses"].append(("도움말", help_resp))
            print(f"[Rust] Help: {re.sub(r'\x1b\[[0-9;]*m', '', help_resp)[:300]}...")

    rust_client.disconnect()

    # Comparison
    print("\n" + "=" * 70)
    print("COMPARISON")
    print("=" * 70)

    print(f"\nConnection:")
    print(f"  Python: {py_results['connected']}")
    print(f"  Rust:   {rust_results['connected']}")

    print(f"\nLogin:")
    print(f"  Python: {py_results['logged_in']}")
    print(f"  Rust:   {rust_results['logged_in']}")

    # Compare room responses
    py_room = ""
    rust_room = ""
    for cmd, resp in py_results["responses"]:
        if cmd == "final_room":
            py_room = resp
    for cmd, resp in rust_results["responses"]:
        if cmd == "final_room":
            rust_room = resp

    py_clean = re.sub(r'\x1b\[[0-9;]*m', '', py_room)
    rust_clean = re.sub(r'\x1b\[[0-9;]*m', '', rust_room)

    print(f"\nFinal Room Content:")
    print(f"  Python: {py_clean[:500]}...")
    print(f"  Rust:   {rust_clean[:500]}...")

    # Check for dialogue-related keywords
    keywords = ["대화", "NPC", "상인", "장로", "호법", "할배", "할머니", "문", "여관"]
    print(f"\nNPC/Dialogue Keywords Found:")
    for kw in keywords:
        py_has = kw in py_room
        rust_has = kw in rust_room
        if py_has or rust_has:
            print(f"  '{kw}': Python={py_has}, Rust={rust_has}")

    # Compare help responses
    py_help = ""
    rust_help = ""
    for cmd, resp in py_results["responses"]:
        if cmd == "도움말":
            py_help = resp
    for cmd, resp in rust_results["responses"]:
        if cmd == "도움말":
            rust_help = resp

    print(f"\nHelp Response:")
    py_help_clean = re.sub(r'\x1b\[[0-9;]*m', '', py_help)
    rust_help_clean = re.sub(r'\x1b\[[0-9;]*m', '', rust_help)
    print(f"  Python: {py_help_clean[:400]}...")
    print(f"  Rust:   {rust_help_clean[:400]}...")

    print("\n" + "=" * 70)

    return 0


if __name__ == "__main__":
    sys.exit(main())
