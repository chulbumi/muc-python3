// 몹삭제.rhai - 몹 삭제 (관리자)
// Python: cmds/몹삭제.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "사용법: [몹 인덱스] 몹삭제");
        return;
    }
    if delete_mob_definition(line) {
        send_line(ob, "몹이 삭제되었습니다.");
    } else {
        send_line(ob, "존재하지않는 몹입니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
