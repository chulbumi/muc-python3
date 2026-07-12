// 앞.rhai - 앞으로 이동 (관리자 명령: 대상 플레이어 위치로 순간이동)
// Python: cmds/앞.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if !require_arg(ob, line) { send_line(ob, "☞ 운영자 명령: [대상] 앞"); return; }
    let target = trim(line);
    let pos = get_position_of(target);
    if length(pos["zone"]) == 0 {
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^");
        return;
    }
    let my = get_my_position(ob);
    if my["zone"] == pos["zone"] && my["room"] == pos["room"] {
        send_line(ob, "☞ 같은 자리에요.");
        return;
    }
    let err = perform_summon_move(ob, pos["zone"], pos["room"]);
    if err != "" { send_line(ob, "☞ 공간이동에 실패하였습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
