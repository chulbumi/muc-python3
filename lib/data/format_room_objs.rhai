// lib/data/format_room_objs.rhai
// 바닥 아이템 이름별 묶음 포맷. 파이썬 viewMapData nStr.
// items: [ #{ name, count, desc1 }, ... ]
// 반환: "\r\n줄1\r\n줄2..." 또는 ""

fn format_room_objs(items) {
    let out = [];
    for i in 0..len(items) {
        let it = items[i];
        let name = it["name"];
        let cnt = it["count"];
        let d1 = it["desc1"];
        let name_a = name_ansi(name);
        let line = if d1 == "" {
            if cnt == 1 {
                "○ " + name_a + han_iga(name) + " 바닥에 떨어져 있다."
            } else {
                "○ " + name_a + " " + int_to_str(cnt) + "개가 바닥에 떨어져 있다."
            }
        } else {
            if cnt == 1 {
                replace_str(d1, "$아이템$", name_a)
            } else {
                replace_str(d1, "$아이템$", name_a + " " + int_to_str(cnt) + "개")
            }
        };
        out.push(line);
    }
    if len(out) == 0 { "" } else { "\r\n" + join(out, "\r\n") }
}

return format_room_objs(items);
