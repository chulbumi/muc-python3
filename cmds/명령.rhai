// 명령.rhai - NPC/몹 명령 설정 (관리자 레벨 2000)
// Python: cmds/명령.py
// 대상 플레이어에게 명령을 실행시키는 관리자 명령어

fn cmd(ob, line) {
    // 관리자 권한 확인
    let admin_level = get_admin_level(ob);
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    // 명령어 파싱: [대상] [내용]
    let words = parse_note_header(line);
    if len(words) < 2 {
        send_line(ob, "☞ 운영자 명령: [대상] [내용] 명령");
        return;
    }

    let target_name = resolve_room_player_name(ob, words[0]);
    let command = words[1];
    if target_name == "" {
        // Python folds missing, item, box and mob targets into one message.
        send_line(ob, "☞ 그런 대상이 없어요. *^_^*");
        return;
    }

    // 대상에게 명령 실행
    // Python: obj.do_command(words[1])
    // Python has no administrator-side success/failure text after resolving
    // the same-room Player; operational failures are logged by the driver.
    admin_force_command(ob, target_name, command);
}

fn main(ob, line) {
    cmd(ob, line);
}
