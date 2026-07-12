// 이동.rhai - 이동 명령어 (관리자 1000: [존:방] 순간이동)
// Python: cmds/이동.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if !require_arg(ob, line) { send_line(ob, "☞ 사용법: [존이름:맵번호] 이동"); return; }
    let spec = parse_room_spec(line);
    if length(spec["zone"]) == 0 {
        send_line(ob, "* 이동 실패!!!");
        return;
    }
    let err = perform_summon_move(ob, spec["zone"], spec["room"]);
    if err == "fail" { send_line(ob, "* 이동 실패!!!"); return; }
    if err == "same_place" { send_line(ob, "☞ 같은 자리에요. ^^"); return; }
    if err != "" { return; }
}

fn main(ob, line) {
    cmd(ob, line);
}
