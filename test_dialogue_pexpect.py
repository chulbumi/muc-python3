#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
NPC Dialogue System Test using pexpect.

Tests Python (9900) vs Rust (9999) MUD servers.
"""

import pexpect
import time
import sys
import re
import os

def clean_ansi(text):
    """Remove ANSI color codes."""
    return re.sub(r'\x1b\[[0-9;]*m', '', text)

def test_server(port, name):
    """Test a single MUD server."""
    print(f"\n### Testing {name} Server ({port}) ###")
    results = {
        "connected": False,
        "logged_in": False,
        "responses": {},
        "room_content": "",
        "command_list": "",
        "help_output": ""
    }

    try:
        # Connect to server
        child = pexpect.spawn('nc localhost %d' % port, encoding='euc-kr', timeout=10)
        results["connected"] = True
        print(f"[{name}] Connected")

        # Wait for prompt and login
        time.sleep(1)
        child.sendline("테스트")
        time.sleep(0.5)
        child.sendline("1234")
        time.sleep(1.5)

        # Read response
        try:
            data = child.read_nonblocking(size=2000, timeout=1)
        except:
            data = ""

        results["logged_in"] = True
        print(f"[{name}] Login complete")

        # Look at current room
        child.sendline("봐")
        time.sleep(0.8)
        try:
            room_resp = child.read_nonblocking(size=2000, timeout=1)
        except:
            room_resp = ""
        results["room_content"] = room_resp
        print(f"[{name}] Room: {clean_ansi(room_resp)[:200]}...")

        # Try to move around and find NPCs
        for direction in ["동", "동", "남", "남", "서", "북"]:
            child.sendline(direction)
            time.sleep(0.5)
            try:
                child.read_nonblocking(size=1000, timeout=0.3)
            except:
                pass

            child.sendline("봐")
            time.sleep(0.5)
            try:
                room_resp = child.read_nonblocking(size=2000, timeout=0.5)
            except:
                room_resp = ""

            if any(kw in room_resp for kw in ["사람", "장로", "호법", "상인", "무인", "여관", "NPC"]):
                print(f"[{name}] Found potential NPC after moving {direction}")
                results["room_content"] = room_resp
                break

        # Get help
        child.sendline("도움말")
        time.sleep(0.8)
        try:
            help_resp = child.read_nonblocking(size=2000, timeout=1)
        except:
            help_resp = ""
        results["help_output"] = help_resp
        print(f"[{name}] Help: {clean_ansi(help_resp)[:200]}...")

        # Get command list
        child.sendline("명령어리스트")
        time.sleep(0.8)
        try:
            cmd_resp = child.read_nonblocking(size=3000, timeout=1)
        except:
            cmd_resp = ""
        results["command_list"] = cmd_resp

        # Save output to file
        filename = f"dialogue_output_{name}_{port}.txt"
        with open(filename, "w", encoding="utf-8", errors="replace") as f:
            f.write("=== ROOM ===\n")
            f.write(results["room_content"])
            f.write("\n\n=== HELP ===\n")
            f.write(results["help_output"])
            f.write("\n\n=== COMMANDS ===\n")
            f.write(results["command_list"])
        print(f"[{name}] Saved output to {filename}")

        child.close()

    except Exception as e:
        print(f"[{name}] Error: {e}")
        import traceback
        traceback.print_exc()

    return results


def main():
    print("=" * 70)
    print("NPC DIALOGUE SYSTEM TEST (pexpect)")
    print("=" * 70)

    py_results = test_server(9900, "Python")
    time.sleep(1)
    rust_results = test_server(9999, "Rust")

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

    py_room = py_results["room_content"]
    rust_room = rust_results["room_content"]

    print(f"\nFinal Room Content (cleaned, first 400 chars):")
    print(f"  Python: {clean_ansi(py_room)[:400]}...")
    print(f"  Rust:   {clean_ansi(rust_room)[:400]}...")

    # Check for dialogue-related keywords
    keywords = ["대화", "사람", "장로", "호법", "할배", "할머니", "여관", "상인", "무공"]
    print(f"\nNPC/Dialogue Keywords Found:")
    for kw in keywords:
        py_has = kw in py_room
        rust_has = kw in rust_room
        if py_has or rust_has:
            print(f"  '{kw}': Python={py_has}, Rust={rust_has}")

    # Compare command list
    py_cmd = py_results["command_list"]
    rust_cmd = rust_results["command_list"]

    print(f"\nCommand List (first 500 chars):")
    print(f"  Python: {clean_ansi(py_cmd)[:500]}...")
    print(f"  Rust:   {clean_ansi(rust_cmd)[:500]}...")

    # Check for dialogue-related commands
    dialogue_cmds = ["대화", "말", "정보", "물어", "전음", "외쳐", "속삭여"]
    print(f"\nDialogue Commands Available:")
    for dc in dialogue_cmds:
        py_has = dc in py_cmd
        rust_has = dc in rust_cmd
        print(f"  '{dc}': Python={py_has}, Rust={rust_has}")

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

    # Check for encoding issues
    if "湲" in rust_room or "λ" in rust_room:
        print("\n  WARNING: Rust server has encoding issues (mojibake detected)")

    print("\n" + "=" * 70)

    return 0


if __name__ == "__main__":
    sys.exit(main())
