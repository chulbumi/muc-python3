// 현판걸어.rhai - 현판 걸기 (방파 생성)
// Python: cmds/현판걸어.py
// Uses: guild_create(ob, guild_name)

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법 : [방파이름] 현판걸어");
        return;
    }

    let guild_name = trim(line);

    if length(guild_name) > 10 {
        send_line(ob, "방파이름이 너무 길어요!.");
        return;
    }

    if length(guild_name) < 2 {
        send_line(ob, "방파이름이 너무 짧아요!.");
        return;
    }

    let my_level = get_attr(ob, "레벨");
    if to_int(my_level) < 400 {
        send_line(ob, "☞ 당신은 방파를 세울 수 없습니다.");
        return;
    }

    let my_guild = get_attr(ob, "소속");
    if my_guild != "" {
        send_line(ob, "☞ 당신은 이미 방파에 소속되어 있습니다.");
        return;
    }

    let admin_level = get_attr(ob, "관리자등급");
    if to_int(admin_level) < 1000 {
        send_line(ob, "☞ 방파 생성에는 관리자 등급 1000 이상이 필요합니다.");
        return;
    }

    // TODO: 방 위치 확인 (현판을 걸 수 있는 곳인지)
    // TODO: 은전 확인 (10,000,000은전 이상 필요)
    // TODO: 중복 방파이름 확인
    // TODO: 방들의 '방파주인' 속성 설정

    // guild_create efun 호출
    let result = guild_create(ob, guild_name);
    if result != "" {
        send_line(ob, "☞ " + result);
        return;
    }

    // 성공 메시지
    let my_name = get_attr(ob, "이름");
    let my_nickname = get_attr(ob, "무림별호");
    let my_personality = get_attr(ob, "성격");

    let color = if my_personality == "정파" { "1;32" } else { "1;31" };
    let nickname = if my_nickname == "" { "무소속" } else { my_nickname };

    send_line(ob, "\x1b[" + color + "m[" + nickname + "]\x1b[0m \x1b[1;36m" + my_name + "\x1b[0;37m 방파 \x1b[1m『" + guild_name + "』\x1b[0m을 창설했습니다.");

    // TODO: 보관함 아이템 지급
    // TODO: 은전 차감
    // TODO: 전체 공지
}

fn main(ob, line) {
    cmd(ob, line);
}
