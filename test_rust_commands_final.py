#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Rust MUD 서버(9999) 명령어 테스트 스크립트"""

import telnetlib
import time
import sys

# Telnet 연결 설정
HOST = "localhost"
PORT = 9999
CHAR_NAME = "테스터러스트"
PASSWORD = ""  # 빈 비밀번호

# 명령어 목록
COMMANDS = [
    "능력치",   # 1
    "점수",     # 2
    "무공",     # 3
    "소지품",   # 4
    "누구",     # 5
    "봐",       # 6
]

def read_until(tn, expected, timeout=5):
    """expected 문자열이 나올 때까지 읽기"""
    output = b""
    start_time = time.time()
    while time.time() - start_time < timeout:
        try:
            chunk = tn.read_very_eager()
            if chunk:
                output += chunk
                if expected.encode('utf-8') in output:
                    return output.decode('utf-8', errors='ignore')
        except Exception:
            pass
        time.sleep(0.1)
    return output.decode('utf-8', errors='ignore')

def send_command(tn, cmd, wait_time=1):
    """명령어 전송 및 응답 수신"""
    tn.write(cmd.encode('utf-8') + b"\r\n")
    time.sleep(wait_time)
    try:
        response = tn.read_very_eager().decode('utf-8', errors='ignore')
        return response
    except Exception:
        return ""

def main():
    results = []

    try:
        # Telnet 연결
        print(f"{HOST}:{PORT}에 연결 중...")
        tn = telnetlib.Telnet(HOST, PORT, timeout=10)

        # 초기 화면 대기
        time.sleep(1)
        initial = tn.read_very_eager().decode('utf-8', errors='ignore')
        print("=== 초기 연결 응답 ===")
        print(initial)
        print()

        # 로그인 시도
        print("=== 로그인 시도 ===")
        tn.write(CHAR_NAME.encode('utf-8') + b"\r\n")
        time.sleep(0.5)

        if PASSWORD:  # 비밀번호가 있으면 전송
            tn.write(PASSWORD.encode('utf-8') + b"\r\n")
            time.sleep(1)
        else:
            # 빈 비밀번호 - 그냥 엔터
            tn.write(b"\r\n")
            time.sleep(1)

        # 로그인 후 응답 확인
        login_response = tn.read_very_eager().decode('utf-8', errors='ignore')
        print(login_response)
        print()

        # 캐릭터가 없으면 생성해야 할 수도 있음
        if "없습니다" in login_response or "새로운" in login_response or "new" in login_response.lower():
            print("캐릭터 생성 필요...")
            time.sleep(2)

        # 메인 화면 대기
        time.sleep(1)

        results.append("# Rust MUD 서버(9999) 명령어 테스트 결과\n")
        results.append(f"캐릭터: {CHAR_NAME}\n")
        results.append(f"테스트 시간: {time.strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        results.append("---\n\n")

        # 각 명령어 테스트
        for i, cmd in enumerate(COMMANDS, 1):
            print(f"테스트 중: {cmd}...")
            results.append(f"## {i}. 명령어: {cmd}\n\n")
            results.append("```\n")

            # 명령어 전송
            tn.write(cmd.encode('utf-8') + b"\r\n")
            time.sleep(1.5)  # 응답 대기

            # 응답 수신
            response = tn.read_very_eager().decode('utf-8', errors='ignore')

            # 응답 정리 (프롬프트 제거 등)
            if response:
                results.append(response)
            else:
                results.append("(응답 없음)")

            results.append("\n```\n\n")
            print(f"  완료: {len(response)} bytes\n")

            # 명령어 간 간격
            time.sleep(0.5)

        # 연결 종료
        tn.write(b"quit\r\n")
        time.sleep(0.5)
        tn.close()

        print("테스트 완료!")

    except Exception as e:
        results.append(f"\n\n오류 발생: {str(e)}\n")
        print(f"오류: {e}")

    # 결과 저장
    output_file = "/home/ubuntu/muc-python3/rust_test_final.md"
    with open(output_file, "w", encoding="utf-8") as f:
        f.writelines(results)

    print(f"\n결과가 {output_file}에 저장되었습니다.")

if __name__ == "__main__":
    main()
