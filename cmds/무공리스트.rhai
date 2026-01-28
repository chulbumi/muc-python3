// 무공리스트.rhai - 무공리스트 (관리자)
// Python: cmds/무공리스트.py
// MUGONG 데이터의 모든 키 목록 표시

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^^*");
        return;
    }

    // MUGONG 데이터의 모든 키 가져오기
    let keys = get_global_keys("MUGONG");
    if len(keys) == 0 {
        send_line(ob, "☞ 무공 데이터가 없습니다.");
        return;
    }

    let mut msg = "";
    let mut c = 0;
    for k in keys {
        msg = msg + format("%14s,", k);
        c += 1;
        if c % 5 == 0 {
            send_line(ob, msg);
            msg = "";
        }
    }
    if msg != "" {
        send_line(ob, msg);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
