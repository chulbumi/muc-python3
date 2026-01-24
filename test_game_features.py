#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
게임(9999 Rust murim_server) 접속 후 cmds/ 전역 명령 검증
- 로그인: 나만바라바 → 빠른도우미 (이름/암호/성별/엔터)
- 검증: 구현된 명령(OK), 스텁(아직 준비 중→STUB), 실패(FAIL), 미등록(NOCMD)
- --quick: 구현된 것 + 숙련도 + 일부 스텁만. 기본: 전부.
"""

import socket
import time
import sys
import os

HOST = 'localhost'
PORT = 9999
ENCODING = 'utf-8'

def enc(s):
    return s.encode(ENCODING)

def dec(b):
    return b.decode(ENCODING, errors='replace')

def send(sock, msg):
    sock.sendall(enc(msg + "\r\n"))
    time.sleep(0.2)

def recv(sock, timeout=2.0, until_idle=0.3):
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

def run_quick_helper(sock, user="테스트캐", pw="test1234"):
    raw = recv(sock, timeout=4.0, until_idle=0.6)
    if "무림존함" not in raw and "존함" not in raw:
        time.sleep(0.5)
        raw += recv(sock, timeout=2.0, until_idle=0.4)
    send(sock, "나만바라바")
    time.sleep(0.6)
    raw = recv_until_contains(sock, "케릭터 이름", timeout=5.0)
    send(sock, user)
    time.sleep(0.5)
    raw = recv_until_contains(sock, "비밀번호", timeout=5.0)
    send(sock, pw)
    time.sleep(0.5)
    raw = recv_until_contains(sock, "성별", "남/여", "엔터", "자고로", timeout=5.0)
    if "성별" in raw or "남/여" in raw:
        send(sock, "남")
        time.sleep(0.6)
        raw = recv_until_contains(sock, "엔터", "누르세요", "자고로", timeout=5.0)
    send(sock, "")
    time.sleep(1.0)
    raw = recv_until_contains(sock, "]", ">", "무공", "체력", "내공", timeout=6.0)
    return raw

# 구현된 명령 (실제 로직 있음). (입력, 설명)
IMPL_CMDS = [
    ("봐", "look"),
    ("상태", "능력치"),
    ("점", "능력치(점)"),
    ("도움말", "help"),
    ("소지품", "inventory"),
    ("장비", "equip"),
    ("숙련도", "숙련도(구현됨)"),
    ("북", "north"),
    ("남", "south"),
    ("동", "east"),
    ("서", "west"),
    ("귀환", "return"),
    ("하이 말", "say"),
    ("테스트 외쳐", "shout"),
    ("웃음 표현", "emote"),
    ("아무거나 하이 전음", "whisper"),
    ("왕대협 은전 1 주다", "give"),
    ("웃음", "emotion(Rust)"),
    ("능력치", "능력치"),
    ("점수", "능력치(점수별칭)"),
    ("어디", "어디"),
    ("누구", "누구"),
    ("맵", "맵(관리자)"),
    ("지도", "지도"),
    ("뭔가 가져", "가져"),
    ("점프", "점프"),
    ("앞", "앞"),
    ("이동", "이동"),
    ("저장", "저장"),
    ("도움말 봐", "도움말봐"),
]

# 스텁 명령 (아직 준비 중). (입력, 설명) — cmds/*.rhai 중 "아직 준비 중" 있는 것. 숙련도 제외.
STUB_CMDS = [
    ("값값", "값값"), ("값설정 x y", "값설정"), ("값삭제 x", "값삭제"),
    ("공지말 x", "공지말"), ("공지사항", "공지사항"),
    ("기부", "기부"), ("기연", "기연"), ("기연리스트", "기연리스트"), ("기연삭제 x", "기연삭제"),
    ("기연삭제1 x", "기연삭제1"), ("기연정리", "기연정리"), ("기연정리리", "기연정리리"), ("기연초기화", "기연초기화"),
    ("꺼내 x", "꺼내"), ("꼬리말 x", "꼬리말"), ("꼬리말제거", "꼬리말제거"),
    ("낚시", "낚시"), ("내공주입", "내공주입"), ("내려", "내려"), ("넣어 x", "넣어"),
    ("누가주나", "누가주나"), ("뒤져", "뒤져"), ("등록 x", "등록"), ("등록삭제 x", "등록삭제"), ("등록취소 x", "등록취소"),
    ("대여 x", "대여"), ("대여목록", "대여목록"), ("따라 x", "따라"), ("똥파말 x", "똥파말"),
    ("리부팅", "리부팅"), ("리젠", "리젠"),
    ("머리말 x", "머리말"), ("머리말제거", "머리말제거"), ("맴돌이", "맴돌이"),
    ("명령 x", "명령"), ("명령어리스트", "명령어리스트"), ("명칭설정 x", "명칭설정"),
    ("모두끝", "모두끝"), ("모두소환", "모두소환"), ("모두저장", "모두저장"),
    ("몹삭제 x", "몹삭제"), ("몹생성 x", "몹생성"), ("몹제거 x", "몹제거"), ("몹제작 x", "몹제작"), ("몹찾기 x", "몹찾기"), ("몹회복 x", "몹회복"),
    ("무공", "무공"), ("무공리스트", "무공리스트"), ("무공상태", "무공상태"), ("무공전수 x", "무공전수"), ("무공전수2 x", "무공전수2"), ("무공제거 x", "무공제거"),
    ("무리", "무리"), ("무리말 x", "무리말"), ("무리제외 x", "무리제외"), ("무리합 x", "무리합"), ("무림별호 x", "무림별호"),
    ("반납 x", "반납"), ("반전음 x", "반전음"),
    ("방설명 x", "방설명"), ("방어구찾기", "방어구찾기"), ("방어지정 x", "방어지정"), ("방이름 x", "방이름"), ("방제거 x", "방제거"), ("방제작 x", "방제작"), ("방주권한양도 x", "방주권한양도"),
    ("방파리스트", "방파리스트"), ("방파말 x", "방파말"), ("방파방설명 x", "방파방설명"), ("방파방이름 x", "방파방이름"), ("방파별호 x", "방파별호"), ("방파상태", "방파상태"), ("방파입문 x", "방파입문"), ("방파초기화", "방파초기화"), ("방파파문 x", "방파파문"),
    ("분노", "분노"), ("분해 x", "분해"), ("비교 x", "비교"), ("비전", "비전"), ("비전삭제 x", "비전삭제"),
    ("사용자몹소환 x", "사용자몹소환"), ("사용자몹제거 x", "사용자몹제거"), ("사용자몹제거1 x", "사용자몹제거1"),
    ("상태보기", "상태보기"), ("설정 x", "설정"), ("설치 x", "설치"), ("성올려 x", "성올려"),
    ("세트기억 x", "세트기억"), ("세트착용 x", "세트착용"), ("소소", "소소"), ("소켓 x", "소켓"), ("소환 x", "소환"),
    ("속성 x", "속성"), ("속성제거 x", "속성제거"), ("속성추가 x", "속성추가"),
    ("수령", "수령"), ("순위", "순위"), ("순위초기화", "순위초기화"),
    ("아이템삭제 x", "아이템삭제"), ("아이템제작 x", "아이템제작"), ("아이템찾기 x", "아이템찾기"),
    ("안시 x", "안시"), ("앞앞", "앞앞"), ("오브젝트저장 x", "오브젝트저장"), ("올려", "올려"), ("올숙리스트", "올숙리스트"),
    ("옵랜덤 x", "옵랜덤"), ("옵설정 x", "옵설정"), ("외쳐2 x", "외쳐2"), ("위치각인", "위치각인"),
    ("이벤트", "이벤트"), ("이벤트삭제 x", "이벤트삭제"), ("이벤트설정 x", "이벤트설정"), ("이형환위", "이형환위"),
    ("입금 x", "입금"), ("입문신청 x", "입문신청"),
    ("자동경로 x", "자동경로"), ("자동무공", "자동무공"), ("자동무공삭제 x", "자동무공삭제"),
    ("정리", "정리"), ("정렬", "정렬"), ("제이슨 x", "제이슨"), ("조제 x", "조제"), ("조회 x", "조회"),
    ("죽여 x", "죽여"), ("줄임말 x", "줄임말"), ("줘 x", "줘"), ("줘줘 x", "줘줘"),
    ("지난대화", "지난대화"), ("지난잡담", "지난잡담"), ("지연입력 x", "지연입력"), ("지워지워 x", "지워지워"),
    ("직위임명 x", "직위임명"), ("쪽지 x", "쪽지"), ("찾아라 x", "찾아라"),
    ("채널누구", "채널누구"), ("채널입장 x", "채널입장"), ("채널잡담 x", "채널잡담"), ("채널퇴장 x", "채널퇴장"),
    ("청소", "청소"), ("체인지 x", "체인지"), ("추적 x", "추적"), ("출구숨김 x", "출구숨김"), ("출구제거 x", "출구제거"),
    ("투명", "투명"), ("트윗 x", "트윗"), ("특정방파초기화 x", "특정방파초기화"),
    ("호위 x", "호위"), ("회복", "회복"), ("현판걸어 x", "현판걸어"),
]

# 관리자 전용 명령: 비관리자 시 "무슨 말인지" 정상
ADMIN_ONLY_DESC = {"맵(관리자)", "앞", "이동", "점프"}

def classify(out, expect_stub, desc=""):
    if not out:
        return "FAIL", "응답없음"
    if "알 수 없는" in out or "알수없는" in out:
        return "NOCMD", "미등록"
    if "아직 준비 중" in out or "아직  준비 중" in out:
        return "STUB", "스텁"
    if expect_stub:
        return "OK", "구현됨(예상스텁)"
    if desc in ADMIN_ONLY_DESC and "무슨 말인지" in out:
        return "OK", "관리자전용"
    if "할 수 없" in out or "무슨 말인지" in out:
        return "FAIL", "에러"
    return "OK", "구현됨"

def run_tests(sock, quick=False):
    results = []  # (status, tag, cmd, note, out_preview)
    all_cases = list(IMPL_CMDS)
    if not quick:
        all_cases.extend(STUB_CMDS)
    else:
        # quick: 구현 + 숙련도 이미 포함 + 스텁 10개만
        for i, s in enumerate(STUB_CMDS):
            if i < 10:
                all_cases.append(s)

    for inp, desc in all_cases:
        expect_stub = (inp, desc) in STUB_CMDS
        send(sock, inp)
        out = recv(sock, timeout=2.0, until_idle=0.3)
        status, note = classify(out, expect_stub, desc)
        preview = (out or "(없음)").strip().replace("\r", "").replace("\n", " ")[:70]
        results.append((status, desc, inp, note, preview))
    return results

def main():
    quick = "--quick" in sys.argv
    use_ming = "--밍밍" in sys.argv or "--ming" in sys.argv
    user, pw = ("밍밍", "밍밍") if use_ming else ("테스트캐", "test1234")
    print("=== 게임 명령 검증 (localhost:{}) {} [{}] ===\n".format(
        PORT, "[QUICK]" if quick else "[FULL]", user))
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(20.0)
        sock.connect((HOST, PORT))
        print("[1] 로그인(나만바라바) {} / ***".format(user))
        run_quick_helper(sock, user, pw)
        print("    입장 완료.\n")
        print("[2] 명령 검증\n")
        results = run_tests(sock, quick=quick)
        by_status = {}
        for status, desc, inp, note, preview in results:
            by_status[status] = by_status.get(status, 0) + 1
            sym = "✓" if status == "OK" else ("○" if status == "STUB" else "✗")
            print("  [{}] {} {}  [{}]  {}".format(status, sym, desc, repr(inp)[:36], preview))
        sock.close()
        print("\n--- 요약 ---")
        print("  OK:{}  STUB:{}  FAIL:{}  NOCMD:{}".format(
            by_status.get("OK", 0), by_status.get("STUB", 0),
            by_status.get("FAIL", 0), by_status.get("NOCMD", 0)))
        fail = [r for r in results if r[0] == "FAIL"]
        nocmd = [r for r in results if r[0] == "NOCMD"]
        if fail:
            print("  FAIL: " + ", ".join(r[1] for r in fail))
        if nocmd:
            print("  NOCMD: " + ", ".join(r[1] for r in nocmd))
        stub_list = [r[1] for r in results if r[0] == "STUB"]
        if stub_list and not quick:
            print("  STUB(미구현) 수: {}".format(len(stub_list)))
        print("\n=== 검증 종료 ===")
    except Exception as e:
        print("오류:", e)
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
