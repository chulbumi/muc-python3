#!/usr/bin/env python3
# """data/mob/{zone}/*.json 이벤트 배열을 data/script/{zone}/{mob_id}_{suffix}.rhai 로 변환하고 JSON을 문자열 참조로 바꿉니다."""

import json
import os
import re
from pathlib import Path

SCRIPT_BASE = Path("data/script")
MOB_BASE = Path("data/mob")

# 변환 시 스킵할 지시어 (아직 Rhai efun 미지원)
SKIP_IF_CONTAINS = ["$스크립트호출", "$엔터$"]


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")


def suffix_from_key(key: str) -> str:
    toks = key.split()
    # "이벤트" / "이벤트:" 건너뛰기
    i = 0
    if i < len(toks) and toks[i] in ("이벤트", "이벤트:"):
        i += 1
    out = []
    while i < len(toks):
        t = toks[i]
        if t.startswith("$"):
            t = t[1:]
        if t and t != ":":
            out.append(re.sub(r'[^\w\uac00-\ud7a3]', "_", t))
        i += 1
    return "_".join(out) if out else "기본"


def parse_get_next_words(line: str) -> str:
    parts = line.split(None, 1)
    return parts[1].strip() if len(parts) > 1 else ""


def get_str_cnt(line: str) -> tuple[str, int]:
    toks = line.split()
    if len(toks) < 2:
        return ("", 1)
    idx = toks[1]
    cnt = 1
    if len(toks) >= 3:
        try:
            cnt = int(toks[-1])
        except ValueError:
            pass
    return (idx, cnt)


def convert_lines(lines: list[str], indent: str = "") -> list[str]:
    out = []
    i = 0
    while i < len(lines):
        raw = lines[i]
        s = raw.strip()
        i += 1

        if not s:
            out.append(f'{indent}output("");')
            continue
        if s in ("{", "}"):
            continue

        if s.startswith("$"):
            nw = parse_get_next_words(s)
            if s.startswith("$종료"):
                out.append(f"{indent}end_event();")
                return out  # 뒤는 실행 안 함
            if s.startswith("$출력"):
                out.append(f'{indent}output("{esc(nw)}");')
                continue
            if s.startswith("$이벤트설정"):
                v = nw.split()
                if len(v) >= 2:
                    out.append(f'{indent}set_event("{esc(v[0])}", "{esc(v[1])}");')
                elif len(v) == 1:
                    out.append(f'{indent}set_event("{esc(v[0])}", "1");')
                continue
            if s.startswith("$이벤트삭제"):
                if nw:
                    out.append(f'{indent}del_event("{esc(nw)}");')
                continue
            if s.startswith("$아이템삭제"):
                idx, cnt = get_str_cnt(s)
                if idx:
                    out.append(f'{indent}delete_item("{esc(idx)}", {cnt});')
                continue
            if s.startswith("$아이템주기"):
                idx, cnt = get_str_cnt(s)
                if idx:
                    out.append(f'{indent}give_item("{esc(idx)}", {cnt});')
                continue
            if s.startswith("$위치이동"):
                if ":" in nw:
                    z, r = nw.split(":", 1)
                    out.append(f'{indent}set_position("{esc(z.strip())}", "{esc(r.strip())}");')
                    out.append(f"{indent}end_event();")
                    return out
                continue
            if s.startswith("$정사전환"):
                out.append(f"{indent}tendency_switch();")
                continue
            if s.startswith("$소오강호설정"):
                out.append(f"{indent}set_giin();")
                continue

            # 블록을 쓰는 $ : 다음에 { 가 와야 함
            if s.startswith("$이벤트확인!") and nw:
                block = []
                depth = 0
                while i < len(lines):
                    line = lines[i]
                    i += 1
                    t = line.strip()
                    if t == "{":
                        depth += 1
                        if depth == 1:
                            continue
                    elif t == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    if depth > 0:
                        block.append(line)
                conv = convert_lines(block, indent + "    ")
                out.append(f'{indent}if !check_event("{esc(nw)}") {{')
                out.extend(conv)
                out.append(f"{indent}}}")
                continue

            if s.startswith("$이벤트확인") and nw:
                block = []
                depth = 0
                while i < len(lines):
                    line = lines[i]
                    i += 1
                    t = line.strip()
                    if t == "{":
                        depth += 1
                        if depth == 1:
                            continue
                    elif t == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    if depth > 0:
                        block.append(line)
                conv = convert_lines(block, indent + "    ")
                out.append(f'{indent}if check_event("{esc(nw)}") {{')
                out.extend(conv)
                out.append(f"{indent}}}")
                continue

            if s.startswith("$무림별호조건") and nw:
                block = []
                depth = 0
                while i < len(lines):
                    line = lines[i]
                    i += 1
                    t = line.strip()
                    if t == "{":
                        depth += 1
                        if depth == 1:
                            continue
                    elif t == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    if depth > 0:
                        block.append(line)
                conv = convert_lines(block, indent + "    ")
                out.append(f'{indent}if get_tendency("{esc(nw)}") {{')
                out.extend(conv)
                out.append(f"{indent}}}")
                continue

            if s.startswith("$아이템확인") and nw:
                block = []
                depth = 0
                while i < len(lines):
                    line = lines[i]
                    i += 1
                    t = line.strip()
                    if t == "{":
                        depth += 1
                        if depth == 1:
                            continue
                    elif t == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    if depth > 0:
                        block.append(line)
                conv = convert_lines(block, indent + "    ")
                out.append(f'{indent}if has_item("{esc(nw)}") {{')
                out.extend(conv)
                out.append(f"{indent}}}")
                continue

            if s.startswith("$변수확인"):
                v = s.split()
                if len(v) >= 3:
                    try:
                        c = int(v[1])
                        val = v[2]
                        idx = c + 1  # words[0]=대상, words[1]=명령, words[2]=첫인자
                        block = []
                        depth = 0
                        while i < len(lines):
                            line = lines[i]
                            i += 1
                            t = line.strip()
                            if t == "{":
                                depth += 1
                                if depth == 1:
                                    continue
                            elif t == "}":
                                depth -= 1
                                if depth == 0:
                                    break
                            if depth > 0:
                                block.append(line)
                        conv = convert_lines(block, indent + "    ")
                        out.append(f'{indent}if words({idx}) == "{esc(val)}" {{')
                        out.extend(conv)
                        out.append(f"{indent}}}")
                    except ValueError:
                        pass
                continue

            # 미지원 $ 는 주석으로 남기고 무시
            out.append(f"{indent}// legacy: {esc(s[:60])}")
            continue

        # 일반 출력
        out.append(f'{indent}output("{esc(s)}");')
    return out


def migrate_file(zone: str, mob_path: Path) -> tuple[int, int]:
    with open(mob_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    info = data.get("몹정보") or {}
    mob_id = mob_path.stem
    written = 0
    skipped = 0
    changes = {}

    for key in list(info.keys()):
        if not key.startswith("이벤트"):
            continue
        val = info[key]
        if not isinstance(val, list):
            continue
        arr = [x if isinstance(x, str) else str(x) for x in val]
        if any(s in " ".join(arr) for s in SKIP_IF_CONTAINS):
            skipped += 1
            continue

        suf = suffix_from_key(key)
        if not suf or suf == "_":
            suf = "기본"
        safe = re.sub(r"[^\w\uac00-\ud7a3\-.]", "_", suf)
        script_name = f"{mob_id}_{safe}.rhai"
        if script_name.startswith("_"):
            script_name = "기본" + script_name

        try:
            body = convert_lines(arr)
        except Exception as e:
            print(f"  convert err {zone}/{mob_path.name} [{key[:40]}]: {e}")
            skipped += 1
            continue

        if not body:
            body = ['output("");', "end_event();"]

        if "end_event()" not in "\n".join(body):
            body.append("end_event();")

        out_dir = SCRIPT_BASE / zone
        out_dir.mkdir(parents=True, exist_ok=True)
        out_path = out_dir / script_name
        header = f"// {key}\n// efun: output, set_event, del_event, delete_item, give_item, set_position, check_event, has_item, get_tendency, tendency_switch, set_giin, words, end_event\n\n"
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(header + "\n".join(body) + "\n")
        written += 1
        # JSON 에서 배열을 스크립트 파일명 문자열로 교체 (확장자 포함해도 되고, 엔진이 .rhai 붙이므로 뺴도 됨. 여기선 포함)
        changes[key] = script_name

    for k, v in changes.items():
        info[k] = v

    if changes:
        with open(mob_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=4)
    return written, skipped


def main():
    total_w = 0
    total_s = 0
    for zone in sorted(MOB_BASE.iterdir()):
        if not zone.is_dir():
            continue
        zone_name = zone.name
        for j in sorted(zone.glob("*.json")):
            w, s = migrate_file(zone_name, j)
            total_w += w
            total_s += s
            if w or s:
                print(f"{zone_name}/{j.name}: +{w} rhai, skip {s}")
    print(f"Total: {total_w} scripts written, {total_s} skipped.")


if __name__ == "__main__":
    main()
