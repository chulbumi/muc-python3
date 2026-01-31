#!/usr/bin/env python3
"""
MUD Server Test Script
Python MUD (9900)와 Rust MUD (9999) 서버에 접속하여 명령어를 테스트하고 비교합니다.

사용법:
    python3 test_servers.py                    # 기본 캐릭터로 테스트
    python3 test_servers.py --python-user 유저명 --rust-user 유저명
    python3 test_servers.py --create-new       # 새 캐릭터 생성 모드
    python3 test_servers.py --commands score,help  # 특정 명령어만 테스트
"""

import socket
import time
import sys
import argparse
import re
from datetime import datetime
from typing import Dict, List, Tuple, Optional


class MUDConnection:
    """MUD 서버 접속을 위한 클래스"""

    def __init__(self, host: str, port: int, name: str):
        self.host = host
        self.port = port
        self.name = name
        self.sock: Optional[socket.socket] = None
        self.connected = False

    def connect(self, timeout: float = 10.0) -> bool:
        """서버에 접속"""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(timeout)
            self.sock.connect((self.host, self.port))
            self.connected = True
            return True
        except Exception as e:
            print(f"[ERROR] {self.host}:{self.port} 접속 실패: {e}")
            return False

    def disconnect(self):
        """접속 해제"""
        if self.sock:
            try:
                self.sock.close()
            except:
                pass
        self.connected = False

    def send(self, message: str):
        """메시지 전송"""
        if self.sock and self.connected:
            self.sock.sendall(message.encode('utf-8') + b"\r\n")

    def receive(self, timeout: float = 2.0) -> str:
        """데이터 수신"""
        if not self.sock or not self.connected:
            return ""

        self.sock.settimeout(timeout)
        data = b""
        start_time = time.time()

        while time.time() - start_time < timeout:
            try:
                chunk = self.sock.recv(4096)
                if chunk:
                    data += chunk
                    time.sleep(0.1)
                else:
                    break
            except socket.timeout:
                break

        return self._decode(data)

    def receive_until_prompt(self, prompt_markers: List[str], timeout: float = 5.0) -> str:
        """프롬프트가 나타날 때까지 수신"""
        if not self.sock or not self.connected:
            return ""

        self.sock.settimeout(timeout)
        data = b""
        start_time = time.time()

        while time.time() - start_time < timeout:
            try:
                chunk = self.sock.recv(4096)
                if chunk:
                    data += chunk
                    decoded = self._decode(data)
                    for marker in prompt_markers:
                        if marker in decoded:
                            time.sleep(0.2)
                            try:
                                more = self.sock.recv(4096)
                                if more:
                                    data += more
                            except:
                                pass
                            return self._decode(data)
            except socket.timeout:
                break

        return self._decode(data)

    def _decode(self, data: bytes) -> str:
        """바이트를 문자열로 디코딩"""
        try:
            return data.decode('utf-8', errors='replace')
        except:
            return str(data)

    def login(self, create_new: bool = False) -> Tuple[bool, str]:
        """
        로그인 과정 처리
        returns: (성공 여부, 초기 화면 출력)
        """
        if not self.connected:
            return False, ""

        # 초기 화면 수신
        initial_screen = self.receive_until_prompt([":", "이름", "명:", "Name"], timeout=5)
        time.sleep(0.5)

        # 이름 전송
        self.send(self.name)
        time.sleep(0.5)

        # 응답 수신
        response = self.receive(timeout=2)

        # 신규 캐릭터 생성 확인
        if create_new or "새로운" in response or "new" in response.lower():
            # 캐릭터 생성 시퀀스
            time.sleep(1)
            response = self.receive(timeout=3)

            # Enter키 여러 번 눌러서 건너뛰기
            for _ in range(10):
                self.send("")
                time.sleep(0.5)
                data = self.receive(timeout=1)
                if "명령" in data or "입력" in data or ">" in data:
                    break

        # 비밀번호가 있을 경우 처리 (없으면 건너뜀)
        if "비번" in response or "암호" in response or "password" in response.lower():
            # 빈 비밀번호로 시도
            self.send("")
            time.sleep(0.5)
            response = self.receive(timeout=2)

        # 로그인 완료까지 대기
        time.sleep(1)
        final_response = self.receive(timeout=2)

        return True, initial_screen + "\n" + response + "\n" + final_response


class MUDTester:
    """MUD 서버 테스터"""

    # 테스트할 명령어 목록
    DEFAULT_COMMANDS = [
        "능력치",
        "무공",
        "소지품",
        "점수",
        "도움말",
        "누구",
        "여기",
        "맵",
        "정보",
    ]

    def __init__(self, python_port: int = 9900, rust_port: int = 9999):
        self.python_port = python_port
        self.rust_port = rust_port
        self.results: Dict[str, Dict] = {}

    def test_command(self, conn: MUDConnection, command: str) -> Dict:
        """단일 명령어 테스트"""
        print(f"  테스트: {command}... ", end="", flush=True)

        # 명령어 전송
        conn.send(command)
        time.sleep(0.5)

        # 응답 수신
        response = conn.receive(timeout=3)

        # ANSI 코드 제거
        clean_response = self._remove_ansi(response)

        print(f"({len(response)} bytes)")

        return {
            "raw": response,
            "clean": clean_response,
            "length": len(response),
            "line_count": len(clean_response.split('\n')),
        }

    def _remove_ansi(self, text: str) -> str:
        """ANSI 이스케이프 시퀀스 제거"""
        ansi_escape = re.compile(r'\x1b\[[0-9;]*m')
        return ansi_escape.sub('', text)

    def compare_responses(self, python_result: Dict, rust_result: Dict) -> Dict:
        """두 서버의 응답 비교"""
        py_clean = python_result["clean"]
        rust_clean = rust_result["clean"]

        # 기본 통계
        py_lines = py_clean.split('\n')
        rust_lines = rust_clean.split('\n')

        comparison = {
            "python_length": python_result["length"],
            "rust_length": rust_result["length"],
            "length_diff": rust_result["length"] - python_result["length"],
            "python_lines": len(py_lines),
            "rust_lines": len(rust_lines),
            "line_diff": len(rust_lines) - len(py_lines),
            "content_match": py_clean.strip() == rust_clean.strip(),
            "similar_lines": self._count_similar_lines(py_lines, rust_lines),
        }

        return comparison

    def _count_similar_lines(self, lines1: List[str], lines2: List[str]) -> int:
        """유사한 라인 수 계산"""
        set1 = set(line.strip() for line in lines1 if line.strip())
        set2 = set(line.strip() for line in lines2 if line.strip())
        return len(set1 & set2)

    def run_tests(
        self,
        python_user: str,
        rust_user: str,
        commands: Optional[List[str]] = None,
        create_new: bool = False
    ) -> Dict:
        """
        전체 테스트 실행

        Args:
            python_user: Python 서버 접속용 캐릭터명
            rust_user: Rust 서버 접속용 캐릭터명
            commands: 테스트할 명령어 목록 (None이면 기본 명령어 사용)
            create_new: 신규 캐릭터 생성 여부

        Returns:
            테스트 결과 딕셔너리
        """
        if commands is None:
            commands = self.DEFAULT_COMMANDS

        results = {
            "timestamp": datetime.now().isoformat(),
            "python_user": python_user,
            "rust_user": rust_user,
            "commands": commands,
            "results": {},
        }

        print("=" * 80)
        print("MUD 서버 비교 테스트")
        print("=" * 80)
        print(f"Python 서버: localhost:{self.python_port} (캐릭터: {python_user})")
        print(f"Rust 서버:   localhost:{self.rust_port} (캐릭터: {rust_user})")
        print("=" * 80)
        print()

        # Python 서버 테스트
        print("[1/2] Python 서버 접속 중...")
        py_conn = MUDConnection("localhost", self.python_port, python_user)
        if not py_conn.connect():
            print("[ERROR] Python 서버 접속 실패")
            return results

        py_conn.login(create_new=create_new)
        print("접속 완료")

        # Python 서버에서 명령어 테스트
        py_results = {}
        print("\nPython 서버 명령어 테스트:")
        print("-" * 40)
        for cmd in commands:
            py_results[cmd] = self.test_command(py_conn, cmd)

        py_conn.disconnect()

        # Rust 서버 테스트
        print("\n[2/2] Rust 서버 접속 중...")
        rust_conn = MUDConnection("localhost", self.rust_port, rust_user)
        if not rust_conn.connect():
            print("[ERROR] Rust 서버 접속 실패")
            return results

        rust_conn.login(create_new=create_new)
        print("접속 완료")

        # Rust 서버에서 명령어 테스트
        rust_results = {}
        print("\nRust 서버 명령어 테스트:")
        print("-" * 40)
        for cmd in commands:
            rust_results[cmd] = self.test_command(rust_conn, cmd)

        rust_conn.disconnect()

        # 비교 분석
        print("\n" + "=" * 80)
        print("비교 분석 결과")
        print("=" * 80)

        for cmd in commands:
            comparison = self.compare_responses(py_results[cmd], rust_results[cmd])
            results["results"][cmd] = {
                "python": py_results[cmd],
                "rust": rust_results[cmd],
                "comparison": comparison,
            }

            # 출력
            print(f"\n명령어: {cmd}")
            print("-" * 40)
            print(f"  Python: {comparison['python_length']} bytes, {comparison['python_lines']} lines")
            print(f"  Rust:   {comparison['rust_length']} bytes, {comparison['rust_lines']} lines")
            print(f"  차이:   {comparison['length_diff']:+d} bytes, {comparison['line_diff']:+d} lines")
            print(f"  내용 일치: {'O' if comparison['content_match'] else 'X'}")
            print(f"  유사 라인: {comparison['similar_lines']}")

        return results

    def save_results(self, results: Dict, filename: str):
        """결과를 파일로 저장"""
        with open(filename, 'w', encoding='utf-8') as f:
            f.write("=" * 80 + "\n")
            f.write("MUD 서버 비교 테스트 결과\n")
            f.write("=" * 80 + "\n\n")
            f.write(f"시간: {results['timestamp']}\n")
            f.write(f"Python 캐릭터: {results['python_user']}\n")
            f.write(f"Rust 캐릭터: {results['rust_user']}\n\n")

            for cmd, result in results["results"].items():
                f.write("=" * 80 + "\n")
                f.write(f"명령어: {cmd}\n")
                f.write("=" * 80 + "\n\n")

                comp = result["comparison"]

                f.write(f"Python: {comp['python_length']} bytes, {comp['python_lines']} lines\n")
                f.write(f"Rust:   {comp['rust_length']} bytes, {comp['rust_lines']} lines\n")
                f.write(f"차이:   {comp['length_diff']:+d} bytes, {comp['line_diff']:+d} lines\n")
                f.write(f"내용 일치: {'O' if comp['content_match'] else 'X'}\n\n")

                f.write("-" * 40 + "\n")
                f.write("Python 응답:\n")
                f.write("-" * 40 + "\n")
                f.write(result["python"]["clean"])
                f.write("\n\n")

                f.write("-" * 40 + "\n")
                f.write("Rust 응답:\n")
                f.write("-" * 40 + "\n")
                f.write(result["rust"]["clean"])
                f.write("\n\n")

        print(f"\n결과가 {filename}에 저장되었습니다.")


def main():
    parser = argparse.ArgumentParser(
        description="MUD 서버 비교 테스트 스크립트",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
예시:
  %(prog)s                              # 기본 설정으로 테스트
  %(prog)s --python-user tester1 --rust-user tester2
  %(prog)s --commands score,help,누구
  %(prog)s --create-new --output results.txt
        """
    )

    parser.add_argument(
        "--python-port",
        type=int,
        default=9900,
        help="Python MUD 서버 포트 (기본값: 9900)"
    )

    parser.add_argument(
        "--rust-port",
        type=int,
        default=9999,
        help="Rust MUD 서버 포트 (기본값: 9999)"
    )

    parser.add_argument(
        "--python-user",
        type=str,
        default="테스터유저",
        help="Python 서버 접속용 캐릭터명 (기본값: 테스터유저)"
    )

    parser.add_argument(
        "--rust-user",
        type=str,
        default="러스트테스터",
        help="Rust 서버 접속용 캐릭터명 (기본값: 러스트테스터)"
    )

    parser.add_argument(
        "--commands",
        type=str,
        help="테스트할 명령어 목록 (쉼표로 구분, 예: score,help,누구)"
    )

    parser.add_argument(
        "--create-new",
        action="store_true",
        help="신규 캐릭터 생성 모드"
    )

    parser.add_argument(
        "--output",
        type=str,
        default=None,
        help="결과 저장 파일명 (기본값: mud_test_results_YYYYMMDD_HHMMSS.txt)"
    )

    parser.add_argument(
        "--quiet",
        action="store_true",
        help="간략 출력 모드"
    )

    args = parser.parse_args()

    # 명령어 목록 파싱
    commands = None
    if args.commands:
        commands = [c.strip() for c in args.commands.split(",")]

    # 출력 파일명 생성
    output_file = args.output
    if output_file is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_file = f"mud_test_results_{timestamp}.txt"

    # 테스터 생성 및 실행
    tester = MUDTester(python_port=args.python_port, rust_port=args.rust_port)
    results = tester.run_tests(
        python_user=args.python_user,
        rust_user=args.rust_user,
        commands=commands,
        create_new=args.create_new
    )

    # 결과 저장
    tester.save_results(results, output_file)

    return 0


if __name__ == "__main__":
    sys.exit(main())
