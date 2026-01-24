// 암호변경.rhai - 암호변경 명령어
// Python: cmds/암호변경.py, objs/player.py get_oldpass/change_password/change_password1
// 사용법: [현재암호] [바꿀암호] 암호변경

fn cmd(ob, line) {
    let p = parse_two_args(line);
    let old = p[0];
    let new = p[1];
    if new == "" {
        send_line(ob, "☞ 사용법: [현재암호] [바꿀암호] 암호변경");
        return;
    }
    if !verify_password(ob, old) {
        send_line(ob, "☞ 현재의 암호가 맞지 않아요. ^^");
        return;
    }
    if len(new) < 3 {
        send_line(ob, "☞ 3자 이상 입력하세요.");
        return;
    }
    set_value(ob, "암호", password_hash(new));
    let res = body_save(ob);
    if res {
        send_line(ob, "☞ 암호가 변경되었습니다.");
    } else {
        send_line(ob, "☞ 암호 변경 후 저장에 실패했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
