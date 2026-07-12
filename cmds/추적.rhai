// Python: cmds/추적.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    let words = line_args(line);
    if len(words) < 2 {
        send_line(ob, "몹이름 존이름 추적");
        return;
    }

    let result = track_mob_in_zone(words[1], words[0]);
    if !result["zone_exists"] {
        send_line(ob, "그런 존은 없어요!");
        return;
    }

    if result["room"] != "" {
        send_line(ob, result["room"]);
        return;
    }
    send_line(ob, "못찾았음");
}

fn main(ob, line) {
    cmd(ob, line);
}
