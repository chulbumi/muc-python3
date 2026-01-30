// 방파초기화.rhai - 관리자가 방파를 강제로 해체
// Python: cmds/방파초기화.py
// Uses: guild_remove(guild_id) - 관리자용 방파 삭제

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 사용법: [방파이름] 방파초기화");
        return;
    }

    let id = trim(line);

    // 방파 존재 확인
    if !guild_has(id) {
        send_line(ob, "* 그런 방파가 없습니다.");
        return;
    }

    // 방파 데이터 삭제
    if guild_remove(id) {
        // TODO: 방파원들의 소속과 직위를 제거해야 함
        // TODO: 방파 방들의 '방파주인' 속성을 제거해야 함
        send_line(ob, "* 방파가 초기화되었습니다.");
    } else {
        send_line(ob, "☞ 방파 초기화에 실패했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
