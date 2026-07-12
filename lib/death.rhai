// Shared presentation for Python Body.dropAllItem() followed by coma.

fn death_item_post(item) {
    let name_a = if item["ansi"] != "" {
        item["ansi"] + item["name"] + "\x1b[0;37m"
    } else {
        "\x1b[0;36m" + item["name"] + "\x1b[37m"
    };
    name_a + han_eul(item["particle_source"])
}

fn finish_python_death(ob) {
    let result = death_drop_inventory_python(ob);
    let last_post = "";
    for item in result["dropped"] {
        let count = item["count"];
        if count == 1 {
            last_post = death_item_post(item);
            send_line(ob, "당신이 \x1b[36m" + last_post + "\x1b[37m 떨어뜨립니다.");
        } else {
            send_line(ob, "당신이 \x1b[36m" + item["name"] + "\x1b[37m "
                + count.to_string() + "개를 떨어뜨립니다..");
        }
    }
    for item in result["failed"] {
        let count = item["count"];
        if count == 1 {
            let post = if last_post == "" { death_item_post(item) } else { last_post };
            send_line(ob, "당신이 \x1b[36m" + post + "\x1b[37m 떨어뜨리자 바로 부서집니다.");
        } else {
            send_line(ob, "당신이 \x1b[36m" + item["name"] + "\x1b[37m "
                + count.to_string() + "개를 떨어뜨리자 바로 부서집니다.");
        }
    }
    send_line(ob, "당신은 정신이 혼미합니다.");
}
