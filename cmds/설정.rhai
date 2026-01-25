// 설정.rhai - 설정 명령어. ob["설정"][키]=값. get_user_config/set_user_config 사용.
// Python: cmds/설정.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: 설정 [키] [값]  예: 설정 외침거부 1");
        return;
    }
    let words = line_args(line);
    let key = words[0];
    let val = if len(words) >= 2 { to_int(trim(words[1])) } else { 1 };
    let v = if val == 1 { "1" } else { "0" };
    set_user_config(ob, key, v);
    send_line(ob, "☞ 설정되었습니다. (" + key + " = " + int_to_str(val) + ")");
}

fn main(ob, line) {
    cmd(ob, line);
}
