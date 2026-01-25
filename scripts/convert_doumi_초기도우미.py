#!/usr/bin/env python3
"""data/config/doumi.json의 초기도우미 배열을 lib/doumi/초기도우미.rhai로 변환"""
import json
import re

def esc(s: str) -> str:
    # Rhai 문자열 내부: \ -> \\, " -> \", ESC(0x1b) -> \x1b
    s = s.replace("\\", "\\\\").replace('"', '\\"').replace("\x1b", "\\x1b")
    return s

def main():
    with open("data/config/doumi.json", "r", encoding="utf-8") as f:
        data = json.load(f)
    arr = data["도우미메인설정"]["초기도우미"]

    out = [
        "// 도우미 스크립트: 초기도우미 (풀 스토리)",
        "// doumi.json 초기도우미 배열에서 마이그레이션",
        "",
        "start_script(ob);",
        ""
    ]

    for line in arr:
        s = line.strip() if isinstance(line, str) else str(line)
        if s.startswith("$틱:"):
            n = s.split(":", 1)[-1].strip()
            out.append(f"set_tick({n});")
        elif s in ("$출력시작", "$출력끝"):
            continue
        elif s == "$키입력":
            out.append("get_enter();")
        elif s.startswith("$키입력:"):
            expected = s.split(":", 1)[-1].strip()  # "$키입력:흑백쌍괴 봐" -> "흑백쌍괴 봐"
            out.append(f'get_key_input("{esc(expected)}");')
        elif s == "$이름획득":
            out.append('ob["이름"] = get_name();')
        elif s == "$암호획득":
            out.append('ob["암호"] = get_password();')
        elif s == "$성별획득":
            out.append('ob["성별"] = get_sex();')
        else:
            out.append(f'send_line(ob, "{esc(s)}");')
        out.append("")

    # 마지막 get_enter() 다음에 finish_script(ob)
    # out 끝이 get_enter();\n\n 이므로, 그 뒤에 finish 추가. 뒤에서 get_enter가 있으면 그 다음이 비어있고 끝.
    # 실제로 마지막 지시가 $키입력 -> get_enter() 라서, 그 다음 빈줄 뒤에 finish_script(ob)를 넣으면 됨.
    # 지금 out에는 ... get_enter(); \n \n 으로 끝남. 마지막 \n\n를 제거하고 finish_script(ob); \n 을 넣자.
    while out and out[-1] == "":
        out.pop()
    out.append("finish_script(ob);")
    out.append("")

    with open("lib/doumi/초기도우미.rhai", "w", encoding="utf-8") as f:
        f.write("\n".join(out))

    print("wrote lib/doumi/초기도우미.rhai")

if __name__ == "__main__":
    main()
