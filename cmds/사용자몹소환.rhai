// 사용자몹소환.rhai - 사용자 생성 몹 소환
// Python: cmds/사용자몹소환.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 사용자몹소환");
        return;
    }
    let result = create_user_mob(ob, line);
    if result == "존재하지않는 사용자입니다."
        || result == "월드 상태를 가져올 수 없습니다."
        || result == "위치 정보를 찾을 수 없습니다."
        || result == "관리자 권한이 없습니다." {
        send_line(ob, result);
    } else {
        send_line(ob, "\x1b[1m" + result + "\x1b[0;37m" + han_eul(result) + " 소환하였습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
