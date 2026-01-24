// 쉬어.rhai - 쉬기. Python: cmds/쉬어.py. has_room_property(쉼금지), set_act(ob,"휴식")

fn cmd(ob, line) {
    let pos = get_my_position(ob);
    let z = pos["zone"];
    let r = pos["room"];
    if has_room_property(z, r, "쉼금지") {
        send_line(ob, "☞ 이곳에서 운기조식하기엔 적당하지 않군요.");
        return;
    }
    let a = get_int(ob, "act");
    if a == 2 {
        send_line(ob, "☞ 벌써 쉬고 있어요. ^^");
    } else if a == 0 {
        set_act(ob, "휴식");
        send_line(ob, "당신이 자세를 편안히 하며 운기조식에 들어갑니다.");
    } else {
        send_line(ob, "☞ 지금 쉬기에는 좋지가 않네요. ^^");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
