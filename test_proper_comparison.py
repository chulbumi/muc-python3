#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MUD 서버 명령어 동작 비교 테스트
Python(9900)과 Rust(9999) 서버의 실제 출력/동작을 비교
"""
import socket
import time
import re
from typing import Tuple, Dict, List

class MUDTester:
    def __init__(self, port: int):
        self.port = port
        self.sock = None

    def connect(self, char_name: str, password: str) -> bool:
        """서버에 연결하고 로그인"""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(5)
            self.sock.connect(("127.0.0.1", self.port))

            time.sleep(0.3)
            self.sock.recv(4096)  # Welcome message

            self.sock.send(f"{char_name}\r\n".encode("utf-8"))
            time.sleep(0.3)
            self.sock.recv(4096)

            self.sock.send(f"{password}\r\n".encode("utf-8"))
            time.sleep(0.5)

            login_resp = self.sock.recv(4096).decode("utf-8", errors="ignore")

            if "암호" in login_resp or "틀렸습니다" in login_resp:
                self.sock.close()
                return False

            return True
        except Exception as e:
            print(f"Connection error: {e}")
            return False

    def send_command(self, command: str, wait_time: float = 0.4) -> str:
        """명령 전송 및 응답 반환"""
        if not self.sock:
            return ""

        # Clear buffer first
        try:
            self.sock.settimeout(0.1)
            _ = self.sock.recv(4096)
        except:
            pass

        self.sock.send(f"{command}\r\n".encode("utf-8"))
        time.sleep(wait_time)

        response = b""
        self.sock.settimeout(0.5)
        try:
            while True:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                response += chunk
                if not self._wait_for_more():
                    break
        except socket.timeout:
            pass

        return response.decode("utf-8", errors="ignore")

    def _wait_for_more(self) -> bool:
        """추가 데이터 대기"""
        self.sock.settimeout(0.1)
        try:
            chunk = self.sock.recv(4096)
            return len(chunk) > 0
        except:
            return False

    def close(self):
        """연결 종료"""
        if self.sock:
            self.sock.close()
            self.sock = None


def normalize_output(text: str) -> str:
    """출력 정규화 - 비교를 위해 불필요한 차이 제거"""
    # ANSI color codes 제거
    text = re.sub(r'\x1b\[[0-9;]*[mGKH]', '', text)
    # CR 제거
    text = text.replace('\r', '')
    # 공백 정규화
    text = re.sub(r'[ \t]+', ' ', text)
    # 빈 줄 제거
    lines = [l.strip() for l in text.split('\n') if l.strip()]
    return '\n'.join(lines)


def compare_outputs(py_output: str, rust_output: str) -> Dict:
    """두 출력 비교"""
    py_norm = normalize_output(py_output)
    rust_norm = normalize_output(rust_output)

    py_lines = set(py_norm.split('\n'))
    rust_lines = set(rust_norm.split('\n'))

    common = py_lines & rust_lines
    py_only = py_lines - rust_lines
    rust_only = rust_lines - py_lines

    return {
        "py_only": py_only,
        "rust_only": rust_only,
        "common": common,
        "py_lines": len(py_lines),
        "rust_lines": len(rust_lines),
        "common_lines": len(common),
        "similarity": len(common) / max(len(py_lines), len(rust_lines), 1)
    }


def test_command_pair(py_tester: MUDTester, rust_tester: MUDTester,
                      command: str, scenario: str = "") -> Dict:
    """두 서버에서 동일 명령 실행 후 결과 비교"""

    # 시나리오 설정 (필요한 경우)
    if scenario:
        py_tester.send_command(scenario)
        rust_tester.send_command(scenario)
        time.sleep(0.3)

    # 명령 실행
    py_output = py_tester.send_command(command)
    rust_output = rust_tester.send_command(command)

    # 비교
    comparison = compare_outputs(py_output, rust_output)

    return {
        "command": command,
        "scenario": scenario,
        "py_output": py_output,
        "rust_output": rust_output,
        "comparison": comparison
    }


def main():
    print("=" * 80)
    print("MUD 서버 명령어 동작 비교 테스트")
    print("=" * 80)

    # 연결
    py_tester = MUDTester(9900)
    rust_tester = MUDTester(9999)

    print("\n[연결 중...]")
    if not py_tester.connect("검사", "9999"):
        print("❌ Python 서버 연결 실패")
        return
    print("✅ Python 서버 연결 성공")

    if not rust_tester.connect("검사", "9999"):
        print("❌ Rust 서버 연결 실패")
        return
    print("✅ Rust 서버 연결 성공")

    # 테스트 케이스: (명령어, 시나리오/설명)
    test_cases = [
        # 기본 정보 명령
        ("능력치", "", "기본 능력치 표시"),
        ("점수", "", "점수 표시"),
        ("상태보기", "", "상태 보기"),
        ("무공", "", "무공 목록"),
        ("소지품", "", "소지품 목록"),
        ("장비", "", "장비 목록"),

        # 이동 명령
        ("동", "", "동쪽 이동"),
        ("주변", "", "주변 확인"),

        # 전투 관련 (공격 가능한 몹이 있어야 함)
        ("공격 포졸", "", "포졸 공격"),

        # 스킬 관련
        ("자동무공", "", "자동무공 설정 확인"),
        ("방어무공시전", "", "방어무공시전 설정 확인"),

        # 소셜
        ("누구", "", "접속자 확인"),
        ("저장", "", "저장"),

        # 표현
        ("표현", "", "표현 목록"),
    ]

    print("\n" + "=" * 80)
    print("[테스트 시작]")
    print("=" * 80)

    results = []
    for test in test_cases:
        if len(test) == 2:
            command, scenario = test
            desc = command
        else:
            command, scenario, desc = test

        print(f"\n[{desc}]")
        print(f"명령: {command}")
        if scenario:
            print(f"시나리오: {scenario}")

        result = test_command_pair(py_tester, rust_tester, command, scenario)
        results.append(result)

        comp = result["comparison"]
        print(f"  Python 라인: {comp['py_lines']}")
        print(f"  Rust 라인: {comp['rust_lines']}")
        print(f"  일치 라인: {comp['common_lines']}")
        print(f"  유사도: {comp['similarity']*100:.1f}%")

        if comp["py_only"]:
            print(f"  Python만 있음 ({len(comp['py_only'])}개):")
            for line in list(comp["py_only"])[:3]:
                print(f"    - {line[:60]}")

        if comp["rust_only"]:
            print(f"  Rust만 있음 ({len(comp['rust_only'])}개):")
            for line in list(comp["rust_only"])[:3]:
                print(f"    - {line[:60]}")

        # 일치 여부 판단 (90% 이상 유사도)
        if comp["similarity"] >= 0.9:
            print(f"  ✅ 일치")
        elif comp["similarity"] >= 0.5:
            print(f"  ⚠️ 부분 일치")
        else:
            print(f"  ❌ 불일치")

    # 정리
    py_tester.close()
    rust_tester.close()

    # 요약
    print("\n" + "=" * 80)
    print("[요약]")
    print("=" * 80)

    high_match = sum(1 for r in results if r["comparison"]["similarity"] >= 0.9)
    partial_match = sum(1 for r in results if 0.5 <= r["comparison"]["similarity"] < 0.9)
    no_match = sum(1 for r in results if r["comparison"]["similarity"] < 0.5)

    print(f"완전 일치: {high_match}/{len(results)}")
    print(f"부분 일치: {partial_match}/{len(results)}")
    print(f"불일치: {no_match}/{len(results)}")

    print("\n" + "=" * 80)


if __name__ == "__main__":
    main()
