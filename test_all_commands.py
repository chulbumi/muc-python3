#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import socket
import time
import sys

def test_command(port, char_name, password, command):
    """Test single command on server"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect(("127.0.0.1", port))

        time.sleep(0.2)
        sock.recv(4096)  # Welcome

        sock.send(f"{char_name}\r\n".encode("utf-8"))
        time.sleep(0.2)
        sock.recv(4096)

        sock.send(f"{password}\r\n".encode("utf-8"))
        time.sleep(0.4)
        login_resp = sock.recv(4096).decode("utf-8", errors="ignore")

        if "암호" in login_resp or "틀렸습니다" in login_resp:
            sock.close()
            return False, "login_failed"

        # Send command
        sock.send(f"{command}\r\n".encode("utf-8"))
        time.sleep(0.3)

        response = sock.recv(8192).decode("utf-8", errors="ignore")

        sock.close()

        # Check if command was recognized
        has_content = len(response) > 10
        has_error = "알 수 없는" in response or "Unknown" in response or "없는 명령" in response

        return True, {
            "has_content": has_content,
            "has_error": has_error,
            "length": len(response)
        }

    except Exception as e:
        return False, str(e)

def test_all_commands(port, commands):
    """Test all commands on a server"""
    results = {}

    for cmd in commands:
        success, result = test_command(port, "검사", "9999", cmd)

        if success:
            if isinstance(result, dict):
                results[cmd] = result
            else:
                results[cmd] = {"has_content": False, "has_error": True}
        else:
            results[cmd] = {"has_content": False, "has_error": True}

        # Small delay between commands
        time.sleep(0.1)

    return results

def main():
    # Read all commands
    with open("/tmp/all_commands.txt", "r") as f:
        commands = [line.strip() for line in f if line.strip()]

    # Remove test/debug commands
    exclude = ["test", "test_output", "test_simple", "test_syntax", "debug_test", "comm", "master"]
    commands = [c for c in commands if c not in exclude]

    print("=" * 80)
    print(f"전체 명령어 테스트 - 총 {len(commands)}개")
    print("=" * 80)

    all_results = {}

    for port, server_name in [(9900, "Python"), (9999, "Rust")]:
        print(f"\n[{server_name} 서버 (포트 {port})] 테스트 중...")
        print("-" * 60)

        results = test_all_commands(port, commands)

        # Count results
        total = len(results)
        ok = sum(1 for r in results.values() if r.get("has_content") and not r.get("has_error"))
        errors = sum(1 for r in results.values() if r.get("has_error"))
        empty = sum(1 for r in results.values() if not r.get("has_content"))

        print(f"  총: {total}")
        print(f"  정상: {ok}")
        print(f"  오류/알 수 없음: {errors}")
        print(f"  응답 없음: {empty}")

        all_results[server_name] = results

    # Comparison
    print("\n" + "=" * 80)
    print("[명령어별 비교]")
    print("=" * 80)

    py_results = all_results.get("Python", {})
    rs_results = all_results.get("Rust", {})

    py_ok = sum(1 for r in py_results.values() if r.get("has_content") and not r.get("has_error"))
    rs_ok = sum(1 for r in rs_results.values() if r.get("has_content") and not r.get("has_error"))

    print(f"\nPython: {py_ok}/{len(py_results)} 정상 작동")
    print(f"Rust:   {rs_ok}/{len(rs_results)} 정상 작동")

    # Show commands that work on both
    both_ok = []
    for cmd in commands:
        py = py_results.get(cmd, {})
        rs = rs_results.get(cmd, {})
        if (py.get("has_content") and not py.get("has_error") and
            rs.get("has_content") and not rs.get("has_error")):
            both_ok.append(cmd)

    print(f"\n양쪽 모두 정상 작동: {len(both_ok)}/{len(commands)} ({100*len(both_ok)//len(commands)}%)")

    # Show differences
    py_only = []
    rs_only = []

    for cmd in commands:
        py = py_results.get(cmd, {})
        rs = rs_results.get(cmd, {})
        py_works = py.get("has_content") and not py.get("has_error")
        rs_works = rs.get("has_content") and not rs.get("has_error")

        if py_works and not rs_works:
            py_only.append(cmd)
        elif rs_works and not py_works:
            rs_only.append(cmd)

    if py_only:
        print(f"\nPython만 작동 ({len(py_only)}): {', '.join(py_only[:10])}")
        if len(py_only) > 10:
            print(f"  ... 외 {len(py_only)-10}개")

    if rs_only:
        print(f"\nRust만 작동 ({len(rs_only)}): {', '.join(rs_only[:10])}")
        if len(rs_only) > 10:
            print(f"  ... 외 {len(rs_only)-10}개")

    print("\n" + "=" * 80)

if __name__ == "__main__":
    main()
