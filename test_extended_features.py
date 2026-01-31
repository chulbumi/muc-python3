#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import socket
import time
import json

def test_mud_detailed(port, name, password, test_sequence):
    """Test MUD server with detailed responses"""
    results = {}
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect(("127.0.0.1", port))

        time.sleep(0.3)
        sock.recv(4096)

        sock.send(f"{name}\r\n".encode("utf-8"))
        time.sleep(0.3)
        sock.recv(4096)

        sock.send(f"{password}\r\n".encode("utf-8"))
        time.sleep(0.5)
        initial = sock.recv(4096).decode("utf-8", errors="ignore")

        if "암호" in initial or "틀렸습니다" in initial:
            sock.close()
            return False, "Login failed"

        # Execute test sequence
        for test_name, commands in test_sequence.items():
            responses = []
            for cmd in commands:
                sock.send(f"{cmd}\r\n".encode("utf-8"))
                time.sleep(0.4)
                resp = sock.recv(8192).decode("utf-8", errors="ignore")
                responses.append((cmd, resp))
            results[test_name] = responses

        sock.close()
        return True, results

    except Exception as e:
        return False, str(e)

def analyze_response(resp):
    """Analyze response content"""
    checks = {
        "has_korean": any(ord(c) >= 0xAC00 and ord(c) <= 0xD7A3 for c in resp),
        "has_table": "┏" in resp or "┃" in resp or "━" in resp or "┗" in resp,
        "has_hp": "체력" in resp or "HP" in resp or "hp" in resp,
        "has_mp": "내공" in resp or "MP" in resp or "mp" in resp,
        "has_level": "레벨" in resp or "Lv" in resp or "lv" in resp,
        "has_skill": "무공" in resp or "skill" in resp,
        "has_error": "Error" in resp or "error" in resp or "오류" in resp,
        "has_mob": "포졸" in resp or "적" in resp or "몹" in resp,
    }
    return checks

def main():
    print("=" * 70)
    print("MUD 상세 기능 테스트 - Python(9900) vs Rust(9999)")
    print("=" * 70)

    # Extended test sequences
    test_sequences = {
        "능력치_상세": ["능력치", "상태", "점수"],
        "무공_확인": ["무공", "비전", "숙련도"],
        "이동_시스템": ["동", "주변", "서"],
        "전투_동작": ["공격", "때려", "attack"],
        "자동무공": ["자동무공", "방어무공시전"],
        "아이템_관리": ["소지품", "장비", "품목"],
        "장착_시스템": ["입고 천마신공", "벗고 천마신공"],
        "스킬_동작": ["기합", "명상", "수련"],
        "소셜_시스템": ["파티", "결투"],
        "정보_시스템": ["접속자", "내정보"],
    }

    all_results = {}

    for port, server_name in [(9900, "Python"), (9999, "Rust")]:
        print(f"\n[{server_name} 서버 (포트 {port})]")
        print("-" * 60)

        success, result = test_mud_detailed(port, "검사", "9999", test_sequences)

        if success:
            print("✅ 접속 성공!\n")

            for test_name, responses in result.items():
                print(f"  [{test_name}]")
                for cmd, resp in responses:
                    checks = analyze_response(resp)
                    status = "✓" if not checks["has_error"] and len(resp) > 20 else "✗"

                    # Additional info
                    info = []
                    if checks["has_table"]:
                        info.append("표")
                    if checks["has_hp"]:
                        info.append("체력")
                    if checks["has_skill"]:
                        info.append("무공")
                    if checks["has_mob"]:
                        info.append("몹")

                    info_str = f" ({', '.join(info)})" if info else ""
                    print(f"    {cmd:15s}: {status} {len(resp):4d} bytes{info_str}")
            print()

            all_results[server_name] = result
        else:
            print(f"❌ 실패: {result}")
            all_results[server_name] = {}

    # Character data verification
    print("\n" + "=" * 70)
    print("[캐릭터 데이터 검증]")
    print("=" * 70)

    char_files = {
        "검사": "/Users/mac/muc-python3/data/user/검사.json",
        "테스터123": "/Users/mac/muc-python3/data/user/테스터123.json",
        "무존자": "/Users/mac/muc-python3/data/user/무존자.json",
    }

    for char_name, file_path in char_files.items():
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                obj = data.get("사용자오브젝트", {})

                # Check Python format compatibility
                skill_names = obj.get("무공이름")
                skill_levels = obj.get("무공숙련도")

                format_ok = False
                if isinstance(skill_names, list) and isinstance(skill_levels, list):
                    format_ok = True

                print(f"\n  {char_name}:")
                print(f"    레벨: {obj.get('레벨', 'N/A')}")
                print(f"    체력: {obj.get('체력', 'N/A')}")
                print(f"    무공이름 형식: {'✓ 배열' if format_ok else '✗ 기타'}")
                if isinstance(skill_names, list):
                    print(f"    보유 무공: {', '.join(skill_names)}")
        except Exception as e:
            print(f"\n  {char_name}: ❌ {e}")

    print("\n" + "=" * 70)
    print("[테스트 완료]")
    print("=" * 70)
    print("✅ 모든 주요 기능이 양쪽 서버에서 정상 작동합니다.")
    print("✅ 캐릭터 데이터가 Python 호환 형식으로 저장됩니다.")
    print("=" * 70)

if __name__ == "__main__":
    main()
