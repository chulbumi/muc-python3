// 사용자몹소환.rhai - 사용자 생성 몹 소환
// Python: cmds/사용자몹소환.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 운영자 명령: 사용자몹소환 [몹이름]");
        return;
    }
    let result = create_user_mob(ob, line);
    if result == "" {
        send_line(ob, line + " 소환하였습니다.");
    } else {
        send_line(ob, result);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
