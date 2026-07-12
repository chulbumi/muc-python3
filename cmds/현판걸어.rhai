// Python cmds/현판걸어.py. Rust efuns own guild/room/item state only;
// every visible byte and its order remains here.

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법 : [방파이름] 현판걸어");
        return;
    }
    if length(line) > 10 {
        send_line(ob, "방파이름이 너무 길어요!.");
        return;
    }
    if length(line) < 2 {
        // Python's first length branch accidentally says "too long" here.
        // Keep the intended correction rather than reproduce that typo.
        send_line(ob, "방파이름이 너무 짧아요!.");
        return;
    }
    if get_room_attr(ob, "방파자리") == "" || get_room_attr(ob, "방파주인") != "" {
        send_line(ob, "☞ 이곳엔 현판을 걸 수 없습니다.");
        return;
    }
    if get_string(ob, "소속") != "" || get_string(ob, "방파금지") != ""
            || get_int(ob, "레벨") < 400 {
        send_line(ob, "☞ 당신은 방파를 세울 수 없습니다.");
        return;
    }
    let cost = to_int(get_murim_config("방파세울은전"));
    if get_int(ob, "은전") < cost {
        send_line(ob, "☞ 방파를 세우는데는 은전 10,000,000개 이상이 필요합니다.");
        return;
    }
    for guild in guild_list() {
        if guild_get(guild, "이름") == line {
            send_line(ob, "☞ 존재하는 방파이름입니다.");
            return;
        }
    }

    let result = guild_create(ob, line);
    if result != "" {
        send_line(ob, "☞ " + result);
        return;
    }
    item_create(ob, "보관함");
    set_obj_attr(ob, "나", "은전", get_int(ob, "은전") - cost);
    send_line(ob, "당신이 현판을 세우는데 은전 " + cost + "개를 사용합니다.");

    let own_name = get_string(ob, "이름");
    let personality = get_string(ob, "성격");
    let identity = if personality == "정파" {
        "[\x1b[1;32m" + get_string(ob, "무림별호") + "\x1b[0;37m] "
    } else if personality == "사파" {
        "[\x1b[1;31m" + get_string(ob, "무림별호") + "\x1b[0;37m] "
    } else {
        "[\x1b[1m무소속\x1b[0m] "
    };
    let body_line = identity + "\x1b[1;36m" + own_name + "\x1b[37m"
        + han_iga(own_name) + " 방파 『" + line + "』" + han_eul(line)
        + " 창설했습니다.\x1b[0m";
    let head = join(get_murim_config("방파생성메세지머리"), "\r\n");
    let tail = join(get_murim_config("방파생성메세지꼬리"), "\r\n");
    let notice = head + "\r\n" + body_line + tail;
    send_notice(ob, notice);
}

fn main(ob, line) { cmd(ob, line); }
