// 외쳐.rhai - 외침. [내용] 외쳐. 포맷을 스크립트에서 정하고, send_to_user로 조건(외침거부) 확인하며 전송.
// Python: cmds/외쳐.py

fn cmd(ob, line) {
    let msg = trim(line);
    if length(msg) == 0 {
        send_line(ob, "☞ 사용법: [내용] 외침(,)");
        return;
    }
    if length(msg) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }
    if get_user_config(ob, "외침거부") == "1" {
        send_line(ob, "☞ 외침거부중엔 외칠 수 없어요. ^^");
        return;
    }
    if get_bool(ob, "운기조식") {
        send_line(ob, "☞ 운기조식중에 외치게 되면 기가 흐트러집니다.");
        return;
    }
    let name = get_string(ob, "이름");
    let adm = get_int(ob, "관리자등급");
    let personality = get_string(ob, "성격");
    let shout_type = if adm >= 2000 {
        "\x1b[0;35m사자후\x1b[0;37m"
    } else if personality == "선인" {
        "\x1b[1;36m창룡후\x1b[0;37m"
    } else if personality == "기인" {
        "\x1b[1;32m사자후\x1b[0;37m"
    } else {
        "\x1b[32m외 침\x1b[37m"
    };
    let formatted = name + "(" + shout_type + ") : " + msg;
    for n in get_online_names() {
        if !user_refuses_shout(n) {
            send_to_user(n, formatted);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
