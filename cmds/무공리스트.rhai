// 무공리스트.rhai - 무공리스트 (관리자)
// Python: cmds/무공리스트.py
// MUGONG 데이터의 모든 키 목록 표시

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    // MUGONG 데이터의 모든 키 가져오기
    // GlobalData stores data/config/skill.json under the file stem `skill`.
    let keys = skill_definition_names();
    let msg = "";
    let c = 0;
    for k in keys {
        msg = msg + pad_start(k, 14, " ") + ",";
        c += 1;
        if c % 5 == 0 {
            msg += "\r\n";
        }
    }
    // Python은 빈 목록도 포함해 조립한 문자열을 sendLine 한 번으로 보낸다.
    send_line(ob, msg);
}

fn main(ob, line) {
    cmd(ob, line);
}
