// Python `cmds/몹회복.py`.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 운영자 명령: [대상] 몹회복");
        return;
    }
    let target = find_attack_target(ob, line);
    if target.is_unit() || target["kind"] != "mob"
        || !recover_room_mob(ob, target["id"], target["room_index"]) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    send_line(ob, "* 회복되었습니다.");
}

fn main(ob, line) { cmd(ob, line); }
