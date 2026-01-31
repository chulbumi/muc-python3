#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
상세 차이 분석 - 각 명령어의 실제 출력을 자세히 비교
"""
import socket
import time

def test_and_capture(port: int, commands: list) -> dict:
    """명령 실행 후 전체 출력 캡처"""
    results = {}

    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect(("127.0.0.1", port))

        time.sleep(0.3)
        sock.recv(8192)  # Welcome

        sock.send("검사\r\n".encode("utf-8"))
        time.sleep(0.3)
        sock.recv(4096)

        sock.send("9999\r\n".encode("utf-8"))
        time.sleep(0.5)
        login = sock.recv(8192).decode("utf-8", errors="ignore")

        for cmd in commands:
            sock.send(f"{cmd}\r\n".encode("utf-8"))
            time.sleep(0.4)
            resp = sock.recv(16384).decode("utf-8", errors="ignore")
            results[cmd] = resp

        sock.close()
        return results

    except Exception as e:
        return {"error": str(e)}


def print_detailed_output(title: str, py_output: str, rust_output: str):
    """양쪽 출력을 상세히 표시"""
    print(f"\n{'='*80}")
    print(f"[{title}]")
    print('='*80)

    print("\n[Python 서버 출력]")
    print("-"*80)
    print(py_output[:1000])

    print("\n[Rust 서버 출력]")
    print("-"*80)
    print(rust_output[:1000])


def main():
    print("="*80)
    print("MUD 서버 상세 차이 분석")
    print("="*80)

    # 능력치 비교
    py_results = test_and_capture(9900, ["능력치"])
    rust_results = test_and_capture(9999, ["능력치"])

    if "능력치" in py_results and "능력치" in rust_results:
        print_detailed_output("능력치 명령", py_results["능력치"], rust_results["능력치"])

    # 무공 비교
    py_results = test_and_capture(9900, ["무공"])
    rust_results = test_and_capture(9999, ["무공"])

    if "무공" in py_results and "무공" in rust_results:
        print_detailed_output("무공 명령", py_results["무공"], rust_results["무공"])

    # 소지품 비교
    py_results = test_and_capture(9900, ["소지품"])
    rust_results = test_and_capture(9999, ["소지품"])

    if "소지품" in py_results and "소지품" in rust_results:
        print_detailed_output("소지품 명령", py_results["소지품"], rust_results["소지품"])

    # 공격 비교 (같은 방에 몹이 있는지 확인)
    py_results = test_and_capture(9900, ["주변", "공격 포졸"])
    rust_results = test_and_capture(9999, ["주변", "공격 포졸"])

    if "주변" in py_results and "주변" in rust_results:
        print_detailed_output("주변 명령 (몹 확인)", py_results["주변"], rust_results["주변"])

    if "공격 포졸" in py_results and "공격 포졸" in rust_results:
        print_detailed_output("공격 포졸 명령", py_results["공격 포졸"], rust_results["공격 포졸"])

    # 점수 비교
    py_results = test_and_capture(9900, ["점수"])
    rust_results = test_and_capture(9999, ["점수"])

    if "점수" in py_results and "점수" in rust_results:
        print_detailed_output("점수 명령", py_results["점수"], rust_results["점수"])

    # 이동 비교
    py_results = test_and_capture(9900, ["동"])
    rust_results = test_and_capture(9999, ["동"])

    if "동" in py_results and "동" in rust_results:
        print_detailed_output("동(이동) 명령", py_results["동"], rust_results["동"])


if __name__ == "__main__":
    main()
