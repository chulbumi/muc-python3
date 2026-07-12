// 몹찾기.rhai - 몹찾기 명령어 (관리자)
// Python: cmds/몹찾기.py
// 사용법: [몹이름] 몹찾기

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [몹이름]|[몹종류] 몹찾기\r\n ex) 산적 몹찾기 or 6 몹찾기");
        return;
    }

    let search_term = trim(line);
    let results = find_mobs(search_term);

    if len(results) == 0 {
        send_line(ob, "☞ '" + search_term + "'(이/가) 몹 데이터에서 발견되지 않았습니다.");
        return;
    }

    send_line(ob, "☞ '" + search_term + "' 검색 결과 (" + to_string(len(results)) + "개 발견):");
    send_line(ob, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let i = 0;
    for r in results {
        let zone = r["zone"];
        let room = r["room"];
        let mob_name = r["mob_name"];
        let display_name = r["display_name"];
        let hp = r["hp"];
        let max_hp = r["max_hp"];

        let loc_str = zone + ":" + room;
        let hp_str = "(" + to_string(hp) + "/" + to_string(max_hp) + ")";

        let line = "  [" + to_string(i + 1) + "] " + display_name + " (" + mob_name + ") " + hp_str;
        send_line(ob, line);
        send_line(ob, "       위치: " + loc_str);
        send_line(ob, "──────────────────────────────────────────────────────────");

        i = i + 1;
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
