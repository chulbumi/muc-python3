// 한 Soul이 소유한 캐릭터 사이에서 조종 대상을 바꾼다.
fn cmd(ob, line) {
    let roster = get_soul_roster(ob);
    let query = trim(line);
    if query == "" {
        send_line(ob, "━━━━━━━━━━━━━━━━━");
        send_line(ob, "  ◁   캐 릭 터 전 환   ▷");
        send_line(ob, "─────────────────");
        for member in roster["members"] {
            let mark = if member["active"] { " *" } else { "" };
            let role = if member["kind"] == "main" { "주 캐릭터" }
                else { if member["kind"] == "mercenary" { "용병" } else { "보조 캐릭터" } };
            send_line(ob, member["number"].to_string() + ". " + member["name"]
                + " [" + role + "]" + mark);
        }
        send_line(ob, "─────────────────");
        send_line(ob, "☞ 사용법: [번호|이름] 전환");
        return;
    }

    let target = "";
    let number = parse_int_prefix(query);
    for member in roster["members"] {
        if member["name"] == query || (number > 0 && member["number"] == number) {
            target = member["name"];
            break;
        }
    }
    if target == "" {
        send_line(ob, "☞ 그 캐릭터는 당신의 무리에 속해 있지 않습니다.");
        return;
    }
    if target == roster["active"] {
        send_line(ob, "☞ 이미 그 캐릭터를 조종하고 있습니다.");
        return;
    }
    if request_soul_switch(ob, target) {
        send_line(ob, "조종 대상을 " + target + " 캐릭터로 전환합니다.");
    }
}

fn main(ob, line) { cmd(ob, line); }
