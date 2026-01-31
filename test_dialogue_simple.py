#!/usr/bin/env python3
"""
NPC Dialogue System Test using telnetlib.

Tests Python (9900) vs Rust (9999) MUD servers.
"""

import telnetlib
import time
import sys
import re

def clean_ansi(text):
    return re.sub(r'\x1b\[[0-9;]*m', '', text)

def test_server(port, name):
    """Test a single MUD server."""
    print(f"\n### Testing {name} Server ({port}) ###")
    results = {"connected": False, "logged_in": False, "responses": []}

    try:
        tn = telnetlib.Telnet("localhost", port, timeout=10)
        results["connected"] = True
        print(f"[{name}] Connected")

        # Login
        tn.write(b"테스트\r\n")
        time.sleep(0.5)
        tn.write(b"1234\r\n")
        time.sleep(1.5)

        # Read response
        data = tn.read_very_eager().decode("euc-kr", errors="ignore")
        results["logged_in"] = True
        print(f"[{name}] Login complete")

        # Look at current room
        tn.write(b"봐\r\n")
        time.sleep(0.8)
        resp1 = tn.read_very_eager().decode("euc-kr", errors="ignore")
        results["responses"].append(("봐", resp1))
        print(f"[{name}] Room: " + clean_ansi(resp1)[:300] + "...")

        # Try different directions to find NPCs
        for direction in ["동", "동", "남", "남", "서", "북"]:
            tn.write(direction.encode("euc-kr") + b"\r\n")
            time.sleep(0.5)
            tn.read_very_eager()

            tn.write(b"봐\r\n")
            time.sleep(0.5)
            room_resp = tn.read_very_eager().decode("euc-kr", errors="ignore")
            if any(kw in room_resp for kw in ["NPC", "사람", "장로", "호법", "상인", "무인", "여관"]):
                print(f"[{name}] Found potential NPC after moving {direction}")
                break

        # Get final room
        tn.write(b"봐\r\n")
        time.sleep(0.8)
        final_room = tn.read_very_eager().decode("euc-kr", errors="ignore")
        results["responses"].append(("final_room", final_room))
        print(f"[{name}] Final room: " + clean_ansi(final_room)[:400] + "...")

        # Get help
        tn.write(b"도움말\r\n")
        time.sleep(0.8)
        help_resp = tn.read_very_eager().decode("euc-kr", errors="ignore")
        results["responses"].append(("도움말", help_resp))
        print(f"[{name}] Help: " + clean_ansi(help_resp)[:300] + "...")

        # Try command list
        tn.write(b"명령어리스트\r\n")
        time.sleep(0.8)
        cmd_resp = tn.read_very_eager().decode("euc-kr", errors="ignore")
        results["responses"].append(("명령어리스트", cmd_resp))

        tn.close()

    except Exception as e:
        print(f"[{name}] Error: {e}")

    return results


def main():
    print("=" * 70)
    print("NPC DIALOGUE SYSTEM TEST (telnetlib)")
    print("=" * 70)

    py_results = test_server(9900, "Python")
    time.sleep(1)
    rust_results = test_server(9999, "Rust")

    # Comparison
    print("\n" + "=" * 70)
    print("COMPARISON")
    print("=" * 70)

    print(f"\nConnection:")
    print(f"  Python: " + str(py_results['connected']))
    print(f"  Rust:   " + str(rust_results['connected']))

    print(f"\nLogin:")
    print(f"  Python: " + str(py_results['logged_in']))
    print(f"  Rust:   " + str(rust_results['logged_in']))

    # Get room responses
    py_room = ""
    rust_room = ""
    for cmd, resp in py_results["responses"]:
        if cmd == "final_room":
            py_room = resp
    for cmd, resp in rust_results["responses"]:
        if cmd == "final_room":
            rust_room = resp

    print(f"\nFinal Room Content:")
    print(f"  Python: " + clean_ansi(py_room)[:500] + "...")
    print(f"  Rust:   " + clean_ansi(rust_room)[:500] + "...")

    # Check for dialogue-related keywords
    keywords = ["대화", "NPC", "상인", "장로", "호법", "할배", "할머니", "문", "여관", "무림인"]
    print(f"\nNPC/Dialogue Keywords Found:")
    for kw in keywords:
        py_has = kw in py_room
        rust_has = kw in rust_room
        if py_has or rust_has:
            print(f"  '" + kw + "': Python=" + str(py_has) + ", Rust=" + str(rust_has))

    # Compare command list
    py_cmd = ""
    rust_cmd = ""
    for cmd, resp in py_results["responses"]:
        if cmd == "명령어리스트":
            py_cmd = resp
    for cmd, resp in rust_results["responses"]:
        if cmd == "명령어리스트":
            rust_cmd = resp

    print(f"\nCommand List (first 500 chars):")
    print(f"  Python: " + clean_ansi(py_cmd)[:500] + "...")
    print(f"  Rust:   " + clean_ansi(rust_cmd)[:500] + "...")

    # Check for dialogue-related commands
    dialogue_cmds = ["대화", "말", "정보", "물어", "전음", "외쳐"]
    print(f"\nDialogue Commands Available:")
    for dc in dialogue_cmds:
        py_has = dc in py_cmd
        rust_has = dc in rust_cmd
        print(f"  '" + dc + "': Python=" + str(py_has) + ", Rust=" + str(rust_has))

    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    # Summary
    py_npc_count = sum(1 for kw in keywords if kw in py_room)
    rust_npc_count = sum(1 for kw in keywords if kw in rust_room)

    print(f"\nNPC-related keywords found:")
    print(f"  Python: {py_npc_count}/{len(keywords)}")
    print(f"  Rust:   {rust_npc_count}/{len(keywords)}")

    py_dialogue_cmds = sum(1 for dc in dialogue_cmds if dc in py_cmd)
    rust_dialogue_cmds = sum(1 for dc in dialogue_cmds if dc in rust_cmd)

    print(f"\nDialogue commands available:")
    print(f"  Python: {py_dialogue_cmds}/{len(dialogue_cmds)}")
    print(f"  Rust:   {rust_dialogue_cmds}/{len(dialogue_cmds)}")

    print("\n" + "=" * 70)

    return 0


if __name__ == "__main__":
    sys.exit(main())
