// 대여목록.rhai - 대여목록. Python: cmds/대여목록.py

fn cmd(ob, line) {
    if !room_has_mob(ob, "진영") {
        send_line(ob, "☞ 이곳에서는 불가능해요.");
        return;
    }
    let entries = book_entries(ob);
    if entries.len() == 0 {
        send_line(ob, "☞ 대여가능한 품목이 없어요.");
        return;
    }
    // Python compares the command argument verbatim.  In particular, a
    // leading/trailing space does not match an otherwise identical name.
    let wanted = line;
    let shown = 0;
    for entry in entries {
        let number = entry["번호"].to_int();
        let name = entry["이름"].to_string();
        let plain_name = strip_ansi(name);
        if wanted != "" && wanted != plain_name {
            continue;
        }
        let state = if entry["대여가능"] == true {
            "대여가능".to_string()
        } else {
            "대여중(" + entry["대여"].to_string() + ")"
        };
        let plain_len = plain_name.len();
        let pad = if plain_len < 8 { "\t" } else { "" };
        let msg = number.to_string() + "\t" + name + pad + "\t(" + entry["등록자"].to_string() + ")\t" + state;
        if get_int(ob, "관리자등급") > 1000 {
            msg += "\t" + entry["고유번호"].to_string();
        }
        send_line(ob, msg);
        shown += 1;
    }
    if shown == 0 {
        send_line(ob, "☞ 대여가능한 품목이 없어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
