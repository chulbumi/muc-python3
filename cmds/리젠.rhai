// 리젠.rhai - [이름] 시체 리젠. mob_regen(ob, name). 인자 없으면 "리젠될 몹이 없어요". 관리자 1000.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 리젠될 몹이 없어요!!");
        return;
    }
    if mob_regen(ob, trim(line)) {
        send_line(ob, "☞ 리젠되었습니다.");
    } else {
        send_line(ob, "☞ 시체만 가능합니다. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
