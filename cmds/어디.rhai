// 어디.rhai - 어디 명령어. Python: cmds/어디.py
// 인자 없음: 같은 zone 접속자 "이름 ▷ 방이름". 인자(이름): 해당 유저 "이름 ▷ 방이름". 없으면 "활동중인 그런 무림인이 없어요"
// rhai unchecked feature로 표현 깊이 제한 비활성화 필요

fn cmd(ob, line) {
    let name = trim(line);
    if name != "" {
        let arr = get_all_online_players();
        for c in arr {
            if c["이름"] == name {
                let z = c["zone"];
                let r = c["room"];
                let rn = get_room_name(z, r);
                send_line(ob, "\x1b[1m" + fill_space(10, name) + "\x1b[0;37m ▷ " + rn);
                return;
            }
        }
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
        return;
    }
    let pos = get_my_position(ob);
    let my_zone = pos["zone"];
    let arr = get_all_online_players();
    for c in arr {
        if c["zone"] == my_zone {
            let z = c["zone"];
            let r = c["room"];
            let rn = get_room_name(z, r);
            send_line(ob, "\x1b[1m" + fill_space(10, c["이름"]) + "\x1b[0;37m ▷ " + rn);
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
