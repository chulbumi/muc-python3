#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
게임(9999 Rust murim_server) 접속 후 cmds/ 전역 명령 검증
- 로그인: 나만바라바 → 빠른도우미 (이름/암호/성별/엔터)
- 검증: 구현(OK), 스텁(STUB), 실패(FAIL), 미등록(NOCMD)
- 스텁 판별: 출력뿐 아니라 cmds/*.rhai 코드 스캔 (STUB_PATTERNS: 아직 준비 중, 연동 예정, 미구현, 추후 구현)
- --quick: 구현 목록 + 스텁 10개만. --scan-only: 접속 없이 코드 스캔만.
- --all: cmds/*.rhai 스캔으로 전체 명령 목록 사용. --all-max=N: 상위 N개. --all-skip=N: 앞 N개 건너뛰기 (다음 50개: --all-skip=50 --all-max=50).
- --two: 테스트캐·밍밍 이인 동시 접속(주다/전음/감정).
- 구현 완료로 코드상 스텁에서 제외된 예: 점수, 머리말, 꼬리말, 기연정리, 기연정리리

아이템 흐름: 가져(줍기), 버려(버리기), 주다(은전/아이템 주기), 생성(관리자). [대상/물품] [명령] 식 한글 어순.
"""

import socket
import select
import time
import sys
import os

# 프로젝트 루트 (이 스크립트 기준)
_ROOT = os.path.dirname(os.path.abspath(__file__))
CMDS_DIR = os.path.join(_ROOT, "cmds")

# cmds/*.rhai 전체 스캔 시 제외 (라이브러리/보조)
CMDS_EXCLUDE = frozenset(("comm", "master", "look", "help", "say", "inventory", "attack", "test"))

# 코드에 아래 문자열이 있으면 스텁(미구현/간단 출력)으로 간주
STUB_PATTERNS = ("아직 준비 중", "연동 예정", "미구현", "추후 구현")

HOST = 'localhost'
PORT = 9999
ENCODING = 'utf-8'

def enc(s):
    return s.encode(ENCODING)

def dec(b):
    return b.decode(ENCODING, errors='replace')

def send(sock, msg):
    sock.sendall(enc(msg + "\r\n"))
    time.sleep(0.15)

def recv(sock, timeout=1.8, until_idle=0.25):
    """타임아웃 내 수신. 데드락 방지: 0.4초 단위 recv, deadline=timeout 강제 종료."""
    sock.settimeout(0.35)
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
    except (BrokenPipeError, ConnectionResetError, OSError):
        pass
    return dec(data)


def recv_both(sock_a, sock_b, timeout=3.0, until_idle=0.35):
    """
    두 소켓에서 동시에 수신. select로 데이터 있는 쪽부터 읽어 데드락·한쪽만 블로킹을 피함.
    연결 끊김(recv b'') 시 즉시 break. 반환: (str_a, str_b)
    """
    buf_a, buf_b = [], []
    last_any = time.time()
    deadline = time.time() + timeout
    for s in (sock_a, sock_b):
        s.settimeout(0.4)
    done = False
    while time.time() < deadline and not done:
        r, _, _ = select.select([sock_a, sock_b], [], [], 0.2)
        now = time.time()
        for s in r:
            try:
                chunk = s.recv(4096)
                if chunk:
                    (buf_a if s is sock_a else buf_b).append(chunk)
                    last_any = now
                else:
                    done = True
                    break
            except (socket.timeout, BlockingIOError, OSError):
                pass
        if not r and (buf_a or buf_b) and (now - last_any) >= until_idle:
            break
    return dec(b"".join(buf_a)), dec(b"".join(buf_b))


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
    ("표현 웃음", "emote"),  # [인용구] [명령]: 한국어 어법상 명령이 마지막
    ("아무거나 하이 전음", "whisper"),
    ("왕대협 은전 1 주다", "give(은전)"),
    ("왕대협 검 1 주다", "아이템주기"),
    ("웃음", "emotion(Rust)"),
    ("능력치", "능력치"),
    ("점수", "능력치(점수별칭)"),
    ("어디", "어디"),
    ("누구", "누구"),
    ("맵", "맵(관리자)"),
    ("지도", "지도"),
    ("뭔가 가져", "가져(줍기)"),
    ("뭔가 버려", "버리기"),
    ("검 1 생성", "아이템생성(관리자)"),
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
ADMIN_ONLY_DESC = {"맵(관리자)", "앞", "이동", "점프", "아이템생성(관리자)"}


def script_path_for(inp):
    """입력의 첫 토큰으로 cmds/XXX.rhai 경로 반환."""
    name = (inp or "").split()[0] if inp else ""
    return os.path.join(CMDS_DIR, name + ".rhai") if name else ""


def get_all_cmd_stems():
    """cmds/*.rhai 파일명 stem 목록. CMDS_EXCLUDE 제외."""
    stems = []
    if not os.path.isdir(CMDS_DIR):
        return stems
    for f in os.listdir(CMDS_DIR):
        if f.endswith(".rhai") and f != ".rhai":
            s = f[:-5]
            if s not in CMDS_EXCLUDE:
                stems.append(s)
    return sorted(stems)


def script_path_for_cmd(inp):
    """한글 어순 [인수] [명령]: 마지막 단어로 cmds/XXX.rhai 탐색, 없으면 첫 단어."""
    parts = (inp or "").strip().split()
    if not parts:
        return ""
    for name in (parts[-1], parts[0]):
        path = os.path.join(CMDS_DIR, name + ".rhai")
        if os.path.isfile(path):
            return path
    return os.path.join(CMDS_DIR, parts[-1] + ".rhai")


def stem_for_inp(inp):
    """입력에서 담당 스크립트 stem 추출 (get_full_cmd_list 중복 제거용)."""
    path = script_path_for_cmd(inp)
    if path:
        return os.path.basename(path).replace(".rhai", "")
    return (inp or "").strip().split()[-1] if (inp or "").strip() else ""


# get_full_cmd_list에서 신규 stem용 샘플 (없으면 stem 그대로). 필요한 것만.
CMD_SAMPLES = {}

# 명령별 필요한 인자를 포함한 테스트 입력. [인수...] [명령] 한글 어순. 없으면 stem만.
# 각 .rhai 사용법/require_arg 기준. 테스트는 '사용법'/'조건 불만'까지 도달하면 OK.
CMD_TEST_INPUTS = {
    "가져": "바람 가져",
    "값설정": "나 t v 값설정",
    "값삭제": "나 k 값삭제",
    "구입": "은전 1 구입",
    "귀환": "귀환",
    "비교": "검 검 비교",
    "부셔": "은전 1 부셔",
    "벗어": "검 벗어",
    "뒤져": "밍밍 뒤져",
    "등록": "x 등록",
    "등록삭제": "x 등록삭제",
    "등록취소": "x 등록취소",
    "도망": "도망",
    "도움말": "도움말",
    "동": "동",
    "똥파말": "x 똥파말",
    "땅": "땅",
    "땅파": "땅파",
    "대여": "x 대여",
    "대여목록": "대여목록",
    "따라": "x 따라",
    "맵": "맵",
    "먹어": "은전 먹어",
    "멍": "멍",
    "모두끝": "모두끝",
    "모두소환": "모두소환",
    "모두저장": "모두저장",
    "무공": "무공",
    "무공리스트": "무공리스트",
    "무공상태": "무공상태",
    "무공전수": "x 무공전수",
    "무공전수2": "x 무공전수2",
    "무공제거": "x 무공제거",
    "무리": "무리",
    "무리말": "x 무리말",
    "무리제외": "x 무리제외",
    "무리합": "x 무리합",
    "무림별호": "x 무림별호",
    "반납": "x 반납",
    "반전음": "x 반전음",
    "방파리스트": "방파리스트",
    "방파말": "x 방파말",
    "방파방설명": "x 방파방설명",
    "방파방이름": "x 방파방이름",
    "방파별호": "x 방파별호",
    "방파상태": "방파상태",
    "방파입문": "x 방파입문",
    "방파초기화": "테스트방파 방파초기화",
    "방파파문": "x 방파파문",
    "방설명": "x 방설명",
    "방어구찾기": "방어구찾기",
    "방어지정": "x 방어지정",
    "방이름": "x 방이름",
    "방제거": "x 방제거",
    "방제작": "x 방제작",
    "방주권한양도": "x 방주권한양도",
    "버려": "은전 버려",
    "분노": "분노",
    "분해": "x 분해",
    "비전": "비전",
    "비전삭제": "x 비전삭제",
    "사용자몹소환": "x 사용자몹소환",
    "사용자몹제거": "x 사용자몹제거",
    "사용자몹제거1": "x 사용자몹제거1",
    "생성": "검 1 생성",
    "상태": "상태",
    "상태보기": "밍밍 상태보기",
    "설정": "x 1 설정",
    "설치": "x 설치",
    "성올려": "x 성올려",
    "세트기억": "x 세트기억",
    "세트착용": "x 세트착용",
    "소각": "은전 소각",
    "소소": "소소",
    "소지품": "소지품",
    "소켓": "x 소켓",
    "소환": "x 소환",
    "수령": "1 수령",
    "순위": "힘 순위",
    "순위초기화": "순위초기화",
    "숙련도": "숙련도",
    "쉬어": "쉬어",
    "시전": "시전",
    "속성": "x 속성",
    "속성제거": "x 속성제거",
    "속성추가": "x 속성추가",
    "아이템삭제": "x 아이템삭제",
    "아이템제작": "x 아이템제작",
    "아이템찾기": "x 아이템찾기",
    "안시": "1 안시",
    "앞": "밍밍 앞",
    "앞앞": "밍밍 앞앞",
    "암호변경": "암호변경",
    "업데이트": "업데이트",
    "어디": "어디",
    "외쳐": "x 외쳐",
    "외쳐2": "x 외쳐2",
    "올려": "힘 올려",
    "올숙리스트": "올숙리스트",
    "옵랜덤": "x 옵랜덤",
    "옵설정": "x 옵설정",
    "위": "위",
    "위치각인": "밍밍 위치각인",
    "이동": "낙양성:1 이동",
    "이동동": "낙양성:1 이동동",
    "이동이동": "낙양성:1 이동이동",
    "이벤트": "밍밍 이벤트",
    "이벤트삭제": "x 이벤트삭제",
    "이벤트설정": "x 이벤트설정",
    "이형환위": "밍밍 이형환위",
    "입금": "x 입금",
    "입문신청": "x 입문신청",
    "입어": "검 입어",
    "자동경로": "x 자동경로",
    "자동무공": "자동무공",
    "자동무공삭제": "x 자동무공삭제",
    "장비": "장비",
    "저장": "저장",
    "전음": "밍밍 하이 전음",
    "정리": "밍밍 정리",
    "정렬": "소지품 힘 정렬",
    "제이슨": "제이슨 global",
    "조제": "x 조제",
    "조회": "x 조회",
    "죽여": "x 죽여",
    "줄임말": "1 줄임말",
    "주다": "밍밍 은전 1 주다",
    "줘": "밍밍 x 줘",
    "줘줘": "x 줘줘",
    "지도": "지도",
    "지난대화": "지난대화",
    "지난잡담": "지난잡담",
    "지연입력": "x 지연입력",
    "지워지워": "x 지워지워",
    "직위임명": "x 직위임명",
    "쪽지": "x 쪽지",
    "찾아라": "x 찾아라",
    "채널누구": "채널누구",
    "채널입장": "x 채널입장",
    "채널잡담": "x 채널잡담",
    "채널퇴장": "x 채널퇴장",
    "청소": "청소",
    "체인지": "x 체인지",
    "추적": "x 추적",
    "출구숨김": "북 출구숨김",
    "출구제거": "북 출구제거",
    "트윗": "x 트윗",
    "특정방파초기화": "x 특정방파초기화",
    "판매": "은전 1 판매",
    "품목표": "품목표",
    "호위": "x 호위",
    "회복": "회복",
    "현판걸어": "x 현판걸어",
    "공지말": "x 공지말",
    "공지사항": "공지사항",
    "기부": "1 기부",
    "기연": "기연",
    "기연리스트": "기연리스트",
    "기연삭제": "x 기연삭제",
    "기연삭제1": "x 기연삭제1",
    "기연정리": "x 기연정리",
    "기연정리리": "x 기연정리리",
    "기연초기화": "기연초기화",
    "꺼내": "소지품 은전 꺼내",
    "꼬리말": "x 꼬리말",
    "꼬리말제거": "꼬리말제거",
    "낚시": "낚시",
    "내공주입": "내공주입",
    "내려": "힘 내려",
    "넣어": "소지품 은전 넣어",
    "누가주나": "1 누가주나",
    "누구": "누구",
    "능력치": "능력치",
    "리부팅": "리부팅",
    "리젠": "리젠",
    "말": "하이 말",
    "머리말": "x 머리말",
    "머리말제거": "머리말제거",
    "맴돌이": "북 맴돌이",
    "명령": "x 명령",
    "명령어리스트": "명령어리스트",
    "명칭설정": "x 명칭설정",
    "몹삭제": "x 몹삭제",
    "몹생성": "x 몹생성",
    "몹제거": "x 몹제거",
    "몹제작": "x 몹제작",
    "몹찾기": "x 몹찾기",
    "몹회복": "x 몹회복",
    "남": "남",
    "북": "북",
    "서": "서",
    "점": "점",
    "점수": "점수",
    "점프": "점프",
    "쳐": "밍밍 쳐",
    "일어나": "일어나",
}


def _test_input_for(inp, desc):
    """(inp, desc)에 대해 인자 포함 테스트 입력 반환. CMD_TEST_INPUTS에 있으면 사용."""
    stem = stem_for_inp(inp)
    return CMD_TEST_INPUTS.get(stem, inp)


def get_full_cmd_list():
    """IMPL + STUB + (cmds 스캔 중 아직 없는 stem). (입력, 설명) 리스트."""
    result = list(IMPL_CMDS) + list(STUB_CMDS)
    covered = {stem_for_inp(inp) for inp, _ in IMPL_CMDS + STUB_CMDS}
    all_stems = set(get_all_cmd_stems())
    new_stems = sorted(all_stems - covered)
    result.extend((CMD_TEST_INPUTS.get(s, CMD_SAMPLES.get(s, s)), s) for s in new_stems)
    return result


def scan_rhai_stub(path):
    """
    cmds/*.rhai 내용에 스텁 패턴이 있으면 True.
    - 아직 준비 중, 연동 예정, 미구현, 추후 구현 (간단 출력만/추후 구현 표시)
    """
    if not path or not os.path.isfile(path):
        return False
    try:
        with open(path, "r", encoding="utf-8") as f:
            c = f.read()
    except Exception:
        return False
    return any(p in c for p in STUB_PATTERNS)


def is_stub_by_code(inp, desc=""):
    """테스트 입력에 해당하는 스크립트가 코드상 스텁이면 True. [인수] [명령] 어순 지원."""
    path = script_path_for_cmd(inp)
    return scan_rhai_stub(path)


def classify(out, expect_stub, desc=""):
    if not out:
        return "FAIL", "응답없음"
    if "Script storage unavailable" in out:
        return "FAIL", "스크립트저장소_lock"
    if "알 수 없는" in out or "알수없는" in out:
        return "NOCMD", "미등록"
    # 출력 기준 스텁: 아직 준비 중, 연동 예정, 미구현, 추후 구현
    if "아직 준비 중" in out or "아직  준비 중" in out:
        return "STUB", "스텁(출력)"
    if "연동 예정" in out or "미구현" in out or "추후 구현" in out:
        return "STUB", "스텁(출력)"
    if expect_stub:
        return "OK", "구현됨(예상스텁)"
    if desc in ADMIN_ONLY_DESC and ("무슨 말인지" in out or "관리자만" in out or "권한이 없" in out):
        return "OK", "관리자전용"
    if "할 수 없" in out or "무슨 말인지" in out:
        return "FAIL", "에러"
    return "OK", "구현됨"

def run_tests(sock, quick=False, cases=None, progress_every=25):
    """cases: None이면 IMPL+(STUB 전부 또는 10개). (입력, 설명) 리스트. send/recv 예외 시 FAIL 기록 후 계속."""
    results = []  # (status, tag, cmd, note, out_preview)
    if cases is not None:
        all_cases = list(cases)
    else:
        all_cases = list(IMPL_CMDS)
        if not quick:
            all_cases.extend(STUB_CMDS)
        else:
            for i, s in enumerate(STUB_CMDS):
                if i < 10:
                    all_cases.append(s)

    for i, (inp, desc) in enumerate(all_cases):
        inp = _test_input_for(inp, desc)  # 인자 포함 테스트 입력으로 치환
        expect_stub = is_stub_by_code(inp, desc)  # 코드 스캔 기준 (출력만 보지 않음)
        try:
            send(sock, inp)
            out = recv(sock, timeout=1.8, until_idle=0.25)
        except (socket.timeout, TimeoutError, BrokenPipeError, ConnectionResetError, OSError) as e:
            out = ""
            results.append(("FAIL", desc, inp, "타임아웃/연결끊김", str(e)[:50]))
            if (i + 1) % progress_every == 0:
                print("  ... 진행 {}/{}".format(i + 1, len(all_cases)), flush=True)
            continue
        status, note = classify(out, expect_stub, desc)
        preview = (out or "(없음)").strip().replace("\r", "").replace("\n", " ")[:70]
        results.append((status, desc, inp, note, preview))
        if progress_every and (i + 1) % progress_every == 0:
            print("  ... 진행 {}/{}".format(i + 1, len(all_cases)), flush=True)
    return results

def run_scan_only(quick=False):
    """접속 없이 cmds/*.rhai 코드 스캔만. 스텁 패턴 보유 명령 출력."""
    all_cases = list(IMPL_CMDS)
    if not quick:
        all_cases.extend(STUB_CMDS)
    else:
        for i, s in enumerate(STUB_CMDS):
            if i < 10:
                all_cases.append(s)
    stubs = [(inp, desc) for inp, desc in all_cases if is_stub_by_code(inp, desc)]
    print("=== 코드 스캔 (cmds/*.rhai) 스텁 패턴: {}\n".format(", ".join(STUB_PATTERNS)))
    print("  코드상 스텁: {} 개\n".format(len(stubs)))
    for inp, desc in stubs:
        p = script_path_for_cmd(inp)
        print("    [{}]  {}".format(desc, p if (p and os.path.isfile(p)) else "(없음)"))
    return stubs


def run_two_char_tests(sock_a, sock_b, name_a="테스트캐", name_b="밍밍"):
    """
    이인 동시 접속: 주다(은전), 전음, 감정(대상지정). A→B, B→A 양방향.
    같은 방 아니면 '대상 없음' 등이 나와도 데드락/타임아웃 없으면 OK.
    recv_both(select)로 한쪽만 블로킹되는 데드락 회피.
    반환: [(테스트명, "OK"|"FAIL", 메모), ...]
    """
    results = []
    time.sleep(0.3)

    # 같은 방 시도 (귀환; 시작 위치가 다를 수 있음)
    try:
        send(sock_a, "귀환")
        time.sleep(0.4)
        send(sock_b, "귀환")
        time.sleep(0.8)
        recv_both(sock_a, sock_b, timeout=2.0, until_idle=0.3)
    except Exception as e:
        results.append(("귀환(같은방)", "FAIL", str(e)))
        return results

    def step(label, sender, cmd):
        try:
            send(sender, cmd)
            time.sleep(0.35)
            # select로 두 소켓 동시 대기 → 한쪽만 데이터 와도 바로 수신, 데드락 완화
            recv_both(sock_a, sock_b, timeout=3.0, until_idle=0.35)
            results.append((label, "OK", "recv OK"))
        except Exception as e:
            results.append((label, "FAIL", str(e)))

    # A → B
    step("A→B 주다(은전)", sock_a, "{} 은전 1 주다".format(name_b))
    step("A→B 전음", sock_a, "{} 하이 전음".format(name_b))
    step("A→B 감정(웃음)", sock_a, "{} 웃음".format(name_b))

    # B → A
    step("B→A 주다(은전)", sock_b, "{} 은전 1 주다".format(name_a))
    step("B→A 전음", sock_b, "{} 하이 전음".format(name_a))
    step("B→A 감정(웃음)", sock_b, "{} 웃음".format(name_a))

    return results


def _parse_opt(name, eq_name):
    """--name=N 또는 --name N 파싱. 없으면 None."""
    for i, a in enumerate(sys.argv):
        if a.startswith(eq_name + "="):
            try:
                return int(a.split("=", 1)[1])
            except ValueError:
                return None
        if a == name and i + 1 < len(sys.argv):
            try:
                return int(sys.argv[i + 1])
            except ValueError:
                return None
    return None


def _parse_all_max():
    return _parse_opt("--all-max", "--all-max")


def _parse_all_skip():
    return _parse_opt("--all-skip", "--all-skip")


def main():
    quick = "--quick" in sys.argv
    use_ming = "--밍밍" in sys.argv or "--ming" in sys.argv
    scan_only = "--scan-only" in sys.argv
    do_all = "--all" in sys.argv
    do_two = "--two" in sys.argv
    all_max = _parse_all_max()
    all_skip = _parse_all_skip()
    user, pw = ("밍밍", "밍밍") if use_ming else ("테스트캐", "test1234")

    if scan_only:
        run_scan_only(quick=quick)
        print("\n=== 스캔 종료 ===")
        return

    if do_two:
        # 이인 동시 접속. --밍밍이면 A,B 모두 밍밍/밍밍; 아니면 테스트캐(A), 밍밍(B).
        name_a = "밍밍" if use_ming else "테스트캐"
        name_b = "밍밍"
        pw_a = "밍밍" if use_ming else "test1234"
        pw_b = "밍밍"
        title = "밍밍 (이중접속)" if use_ming else "테스트캐 + 밍밍"
        print("=== 게임 이인 검증 (localhost:{}) {} ===\n".format(PORT, title))
        try:
            sock_a = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock_a.settimeout(20.0)
            sock_a.connect((HOST, PORT))
            sock_b = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock_b.settimeout(20.0)
            sock_b.connect((HOST, PORT))
            print("[1] 로그인 A: {} / ***".format(name_a))
            run_quick_helper(sock_a, name_a, pw_a)
            print("    입장 완료.")
            print("[2] 로그인 B: {} / ***".format(name_b))
            run_quick_helper(sock_b, name_b, pw_b)
            print("    입장 완료.\n")
            print("[3] 이인 테스트 (주다/전음/감정)\n")
            two_res = run_two_char_tests(sock_a, sock_b, name_a, name_b)
            for label, status, note in two_res:
                sym = "✓" if status == "OK" else "✗"
                print("  [{}] {} {}  {}".format(status, sym, label, note))

            if do_all:
                ran = " {}개".format(all_max) if all_max else " 전체"
                if all_skip:
                    ran = " {}-{}번".format(all_skip + 1, all_skip + (all_max or 999)) if all_max else " {}번~".format(all_skip + 1)
                print("\n[4] 명령 검증 (--all{})\n".format(ran))
                cases = get_full_cmd_list()
                if all_skip:
                    cases = cases[all_skip:]
                if all_max:
                    cases = cases[:all_max]
                prog = 10 if all_skip else 25  # 다음 N개 구간: 진행 더 자주 (데드락 감지)
                results = run_tests(sock_a, cases=cases, progress_every=prog)
                by_status = {}
                for status, desc, inp, note, preview in results:
                    by_status[status] = by_status.get(status, 0) + 1
                    sym = "✓" if status == "OK" else ("○" if status == "STUB" else "✗")
                    print("  [{}] {} {}  [{}]  {}".format(status, sym, desc, repr(inp)[:36], preview))
                code_stub_n = sum(1 for inp, d in cases if is_stub_by_code(inp, d))
                print("\n--- 요약 ---")
                print("  OK:{}  STUB:{}  FAIL:{}  NOCMD:{}".format(
                    by_status.get("OK", 0), by_status.get("STUB", 0),
                    by_status.get("FAIL", 0), by_status.get("NOCMD", 0)))
                print("  코드상 스텁 패턴 보유: {} 개".format(code_stub_n))
                fail = [r for r in results if r[0] == "FAIL"]
                nocmd = [r for r in results if r[0] == "NOCMD"]
                if fail:
                    print("  FAIL: " + ", ".join(r[1] for r in fail))
                if nocmd:
                    print("  NOCMD: " + ", ".join(r[1] for r in nocmd))

            sock_a.close()
            sock_b.close()
            print("\n=== 검증 종료 ===")
        except Exception as e:
            print("오류:", e)
            import traceback
            traceback.print_exc()
            sys.exit(1)
        return

    # 단일 접속
    mode = "[ALL]" if do_all else ("[QUICK]" if quick else "[FULL]")
    if do_all:
        if all_skip and all_max:
            mode = "[ALL 51-100]" if all_skip == 50 and all_max == 50 else "[ALL skip={} max={}]".format(all_skip, all_max)
        elif all_max:
            mode = "[ALL-MAX={}]".format(all_max)
        elif all_skip:
            mode = "[ALL skip={}]".format(all_skip)
    print("=== 게임 명령 검증 (localhost:{}) {} [{}] ===\n".format(PORT, mode, user))
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(15.0)
        sock.connect((HOST, PORT))
        print("[1] 로그인(나만바라바) {} / ***".format(user))
        run_quick_helper(sock, user, pw)
        print("    입장 완료.\n")
        print("[2] 명령 검증\n")
        if do_all:
            cases = get_full_cmd_list()
            if all_skip:
                cases = cases[all_skip:]
            if all_max:
                cases = cases[:all_max]
            prog = 10 if all_skip else 25  # 다음 N개: 진행 10회마다 (데드락 감지)
        else:
            cases = None
            prog = 25
        results = run_tests(sock, quick=quick, cases=cases, progress_every=prog)
        by_status = {}
        for status, desc, inp, note, preview in results:
            by_status[status] = by_status.get(status, 0) + 1
            sym = "✓" if status == "OK" else ("○" if status == "STUB" else "✗")
            print("  [{}] {} {}  [{}]  {}".format(status, sym, desc, repr(inp)[:36], preview))
        sock.close()

        if do_all:
            all_cases = cases
        else:
            all_cases = list(IMPL_CMDS)
            if not quick:
                all_cases.extend(STUB_CMDS)
            else:
                for i, s in enumerate(STUB_CMDS):
                    if i < 10:
                        all_cases.append(s)
        code_stub_n = sum(1 for inp, d in all_cases if is_stub_by_code(inp, d))

        print("\n--- 요약 ---")
        print("  OK:{}  STUB:{}  FAIL:{}  NOCMD:{}".format(
            by_status.get("OK", 0), by_status.get("STUB", 0),
            by_status.get("FAIL", 0), by_status.get("NOCMD", 0)))
        print("  코드상 스텁 패턴 보유: {} 개".format(code_stub_n))
        fail = [r for r in results if r[0] == "FAIL"]
        nocmd = [r for r in results if r[0] == "NOCMD"]
        if fail:
            print("  FAIL: " + ", ".join(r[1] for r in fail))
        if nocmd:
            print("  NOCMD: " + ", ".join(r[1] for r in nocmd))
        stub_list = [r[1] for r in results if r[0] == "STUB"]
        if stub_list and not quick:
            print("  STUB(출력/코드) 수: {}".format(len(stub_list)))
        print("\n=== 검증 종료 ===")
    except Exception as e:
        print("오류:", e)
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
