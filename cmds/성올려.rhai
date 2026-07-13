// 성올려.rhai - 성향 수치 올리기 (관리자)
// Python: cmds/성올려.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let words = line_args(line);
    if words.len() < 3 {
        send_line(ob, "☞ 사용법: [대상] [무공] [성] 성올려");
        return;
    }
    // Python first checks line.split(), then parses the complete third field
    // from split(None, 2) with int(). Extra text raises before any mutation.
    if words.len() > 3 { return; }
    let parsed = strict_int(words[2]);
    if !parsed["valid"] { return; }
    let level = parsed["value"];
    let result = set_skill_training(ob, words[0], words[1], level);
    if result == "ok" { send_line(ob, "☞ 값이 설정되었습니다."); }
    else { send_line(ob, "☞ 그런 대상이 없어요!"); }
}

fn main(ob, line) {
    cmd(ob, line);
}
