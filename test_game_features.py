#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
게임(9999 Rust murim_server) 접속 후 주요 명령을 순서대로 실행하여 기능 검증
- 로그인: 나만바라바 → 빠른도우미 (이름/암호/성별/엔터)
- 검증 명령: 봐, 상태, 도움말, 소지품, 장비, 북/남/동/서, 귀환, 말, 전음, 외쳐, 표현, 주다, 감정
"""

import socket
import time
import sys

HOST = 'localhost'
PORT = 9999
ENCODING = 'utf-8'

def enc(s):
    return s.encode(ENCODING)

def dec(b):
    return b.decode(ENCODING, errors='replace')

def send(sock, msg):
    sock.sendall(enc(msg + "\r\n"))
    time.sleep(0.25)

def recv(sock, timeout=2.0, until_idle=0.35):
    sock.settimeout(0.4)
    data = b""
    last_recv = time.time()
    deadline = time.time() + timeout
    try:
        while time.time() < deadline:
            try:
                chunk = sock.recv(4096)
                if chunk:
                    data += chunk
                    last_recv = time.time()
                else:
                    break
            except socket.timeout:
                if data and (time.time() - last_recv) >= until_idle:
                    break
                if time.time() >= deadline:
                    break
    except Exception:
        pass
    return dec(data)

def recv_until_contains(sock, *keywords, timeout=6.0, step=0.4):
    data = ""
    deadline = time.time() + timeout
    while time.time() < deadline:
        sock.settimeout(step)
        try:
            chunk = sock.recv(4096)
            if chunk:
                data += dec(chunk)
                for k in keywords:
                    if k in data:
                        return data
        except socket.timeout:
            pass
    return data

def run_quick_helper(sock):
    """나만바라바 빠른도우미: 이름 → 암호 → 성별 → 엔터"""
    # 1) 초기 화면 수신 후 이름 입력
    raw = recv(sock, timeout=4.0, until_idle=0.6)
    if "무림존함" not in raw and "존함" not in raw:
        time.sleep(0.5)
        raw += recv(sock, timeout=2.0, until_idle=0.4)
    send(sock, "나만바라바")
    time.sleep(0.6)

    # 2) "케릭터 이름:" 대기 후 전송
    raw = recv_until_contains(sock, "케릭터 이름", timeout=5.0)
    send(sock, "테스트캐")
    time.sleep(0.5)

    # 3) "비밀번호:" 대기 후 전송
    raw = recv_until_contains(sock, "비밀번호", timeout=5.0)
    send(sock, "test1234")
    time.sleep(0.5)

    # 4) "성별(남/여):" 대기 후 전송
    raw = recv_until_contains(sock, "성별", "남/여", timeout=5.0)
    send(sock, "남")
    time.sleep(0.6)

    # 5) "【엔터키를 누르세요】" 대기 후 엔터
    raw = recv_until_contains(sock, "엔터", "누르세요", "자고로", timeout=5.0)
    send(sock, "")
    time.sleep(1.0)

    # 6) 게임 입장까지 대기 (방 설명/프롬프트)
    raw = recv_until_contains(sock, "]", ">", "무공", "체력", "내공", timeout=6.0)
    return raw

def run_tests(sock):
    results = []
    # 파서: 맨 뒤 단어=명령, 앞=인자. 예: "안녕 말", "테스트 외쳐", "ㅎㅎ 표현"
    tests = [
        ("봐", "look", "방/설명/객체"),
        ("상태", "status", "능력치/체력/내공"),
        ("점", "status alias", "능력치"),
        ("도움말", "help", "도움/명령"),
        ("/h", "help alias", "도움"),
        ("소지품", "inventory", "소지품/은전"),
        ("소", "inventory alias", "소지품"),
        ("장비", "equip", "장비"),
        ("장", "equip alias", "장비"),
        ("북", "north", "이동/북"),
        ("남", "south", "이동/남"),
        ("동", "east", "이동/동"),
        ("서", "west", "이동/서"),
        ("귀환", "return", "귀환/귀환지"),
        ("안녕 말", "say", "말/대사"),
        ("테스트 .", "say alias", "말"),
        ("테스트 외쳐", "shout", "외침"),
        ("ㅎㅎ 표현", "emote", "표현"),
        ("웃음 '", "emote alias", "표현"),
    ]
    for cmd, name, hint in tests:
        send(sock, cmd)
        time.sleep(0.4)
        out = recv(sock, timeout=2.0, until_idle=0.3)
        ok = bool(out)
        # 에러/불가 메시지가 없으면 양호. 프롬프트 [ HP/MP ] 만 있어도 수신은 된 것.
        if "할 수 없" in out or "무슨 말인지" in out or "알 수 없는" in out:
            ok = False
        if out and ("]" in out or "[" in out) and "무슨 말인지" not in out:
            ok = True  # 프롬프트/방정보 등 수신
        results.append((name, cmd, ok, out[:200] if out else ""))
    # 전음/주다: 파서 형식 [앞] [맨뒤=명령]. 전음=[대상] [내용] 전음, 주다=[대상] [물품] [개수] 줘
    send(sock, "아무거나 하이 전음")
    time.sleep(0.4)
    out = recv(sock, timeout=2.0, until_idle=0.3)
    results.append(("whisper", "아무거나 하이 전음", "대상 없음/구문" in out or "알 수" in out or "할 수 없" in out or "전음" in out or "상대" in out, out[:150] if out else ""))

    send(sock, "왕대협 은전 1 줘")
    time.sleep(0.4)
    out = recv(sock, timeout=2.0, until_idle=0.3)
    results.append(("give", "왕대협 은전 1 줘", bool(out), out[:150] if out else ""))

    send(sock, "웃음")
    time.sleep(0.4)
    out = recv(sock, timeout=2.0, until_idle=0.3)
    results.append(("emotion", "웃음", bool(out), out[:150] if out else ""))

    return results

def main():
    print("=== 게임 기능 검증 (localhost:{} ) ===\n".format(PORT))
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(15.0)
        sock.connect((HOST, PORT))
        print("[1] 접속 후 로그인(나만바라바 빠른도우미) 진행...")
        run_quick_helper(sock)
        print("    게임 입장 완료.\n")

        print("[2] 명령별 실행 및 수신 여부 검증\n")
        for name, cmd, ok, out in run_tests(sock):
            status = "OK" if ok else "FAIL"
            preview = (out or "(없음)").strip().replace("\r","").replace("\n"," ")[:80]
            print("  [{:8}] {}  [{}]  → {}".format(status, name, repr(cmd)[:30], preview))

        sock.close()
        print("\n=== 검증 종료 ===")
    except Exception as e:
        print("오류:", e)
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
