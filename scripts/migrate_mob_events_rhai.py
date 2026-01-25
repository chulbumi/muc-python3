#!/usr/bin/env python3
# """data/mob/{zone}/*.json 이벤트 배열을 data/script/{zone}/{mob_id}_{suffix}.rhai 로 변환하고 JSON을 문자열 참조로 바꿉니다."""

import json
import os
import re
from pathlib import Path

SCRIPT_BASE = Path("data/script")
MOB_BASE = Path("data/mob")

# 변환 시 스킵할 지시어 (아직 Rhai efun 미지원). $엔터$는 wait_enter/event/stepN 구조로 변환하므로 제외.
SKIP_IF_CONTAINS = ["$스크립트호출"]


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


def split_by_enter(lines: list) -> list:
    """$엔터$로 구획. (블록, 그 다음 엔터의 prompt) 쌍 리스트. 마지막 블록은 None."""
    segments = []
    current = []
    for line in lines:
        s = line.strip()
        if s.startswith("$엔터$"):
            # " $엔터$ [엔터키를 누르세요]" 또는 "$엔터$[엔터키를 누르세요]" 모두 처리
            prompt = s[len("$엔터$"):].strip() or parse_get_next_words(s)
            segments.append((current, prompt))
            current = []
        else:
            current.append(line)
    segments.append((current, None))
    return segments


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
            has_enter = any(line.strip().startswith("$엔터$") for line in arr)
            if has_enter:
                segments = split_by_enter(arr)
                body = []
                for i, (block, prompt_after) in enumerate(segments):
                    blk = convert_lines(block)
                    if i == 0:
                        body.append("fn event() {")
                        body.extend("    " + ln for ln in blk)
                        if prompt_after is not None:
                            body.append(f'    wait_enter("step1", "{esc(prompt_after)}");')
                        body.append("}")
                        body.append("")
                    else:
                        step_name = f"step{i}"
                        body.append(f"fn {step_name}() {{")
                        body.extend("    " + ln for ln in blk)
                        if prompt_after is not None:
                            next_step = f"step{i+1}"
                            body.append(f'    wait_enter("{next_step}", "{esc(prompt_after)}");')
                        else:
                            if "end_event()" not in "\n".join(blk):
                                body.append("    end_event();")
                        body.append("}")
                        body.append("")
                if body and body[-1] == "":
                    body.pop()
            else:
                # $엔터$ 없어도 fn event() { ... } 형태로 통일
                blk = convert_lines(arr)
                body = ["fn event() {"]
                body.extend("    " + ln for ln in blk)
                body.append("}")
        except Exception as e:
            print(f"  convert err {zone}/{mob_path.name} [{key[:40]}]: {e}")
            skipped += 1
            continue

        if not body:
            body = ['output("");', "end_event();"]

        # $엔터$ 구조(event/stepN)가 아니고, fn event() 안에 end_event() 없으면 보충
        if not has_enter and "end_event()" not in "\n".join(body):
            # "fn event() {" 다음부터 "}" 전 마지막에 삽입. body가 ["fn event() {", "    ...", "}"] 형태.
            idx = len(body) - 1  # "}" 바로 앞
            if idx >= 1:
                body.insert(idx, "    end_event();")

        out_dir = SCRIPT_BASE / zone
        out_dir.mkdir(parents=True, exist_ok=True)
        out_path = out_dir / script_name
        header = f"// {key}\n// efun: output, set_event, del_event, delete_item, give_item, set_position, check_event, has_item, get_tendency, tendency_switch, set_giin, words, wait_enter, end_event\n\n"
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


def wrap_existing_rhai_in_event() -> int:
    """이미 존재하는 .rhai 중 fn event()가 없으면 fn event() { ... }로 감쌈. 감싼 파일 수 반환."""
    import sys
    wrapped = 0
    for zone_dir in sorted(SCRIPT_BASE.iterdir()):
        if not zone_dir.is_dir():
            continue
        for rhai_path in sorted(zone_dir.glob("*.rhai")):
            try:
                raw = rhai_path.read_text(encoding="utf-8")
            except Exception as e:
                print(f"  read err {rhai_path}: {e}", file=sys.stderr)
                continue
            lines = raw.split("\n")
            if any(ln.strip().startswith("fn event") for ln in lines):
                continue
            new_lines = ["fn event() {"] + ["    " + ln for ln in lines] + ["}"]
            try:
                rhai_path.write_text("\n".join(new_lines) + "\n", encoding="utf-8")
            except Exception as e:
                print(f"  write err {rhai_path}: {e}", file=sys.stderr)
                continue
            wrapped += 1
            print(f"  wrap: {zone_dir.name}/{rhai_path.name}")
    return wrapped


def main():
    import sys
    wrap_only = "--wrap-event" in sys.argv
    if wrap_only:
        n = wrap_existing_rhai_in_event()
        print(f"Total: {n} .rhai wrapped in fn event().")
        return

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
