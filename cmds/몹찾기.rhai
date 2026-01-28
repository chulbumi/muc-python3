// 몹찾기.rhai - 몹찾기 명령어 (관리자)
// Python: cmds/몹찾기.py
// 사용법: [몹이름] 몹찾기

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "☞ 운영자 명령: [몹이름]|[몹종류] 몹찾기\r\n ex) 산적 몹찾기 or 6 몹찾기");
        return;
    }

    send_line(ob, "☞ 몹 찾기 기능은 현재 구현 중입니다.");
    // TODO: 몹 위치 검색 기능 구현 필요
}

fn main(ob, line) {
    cmd(ob, line);
}
