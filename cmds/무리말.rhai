// 무리말.rhai - 무리/파티 채팅
// Python: cmds/무리말.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [내용] 무리말(;)");
        return;
    }

    // 메시지 길이 제한
    if len(line) > 160 {
        send_line(ob, "☞ 너무 길어요. ^^");
        return;
    }

    // TODO: noComm() 확인 - 현재 방에서 통신 가능한지
    // 이 기능을 위한 별도 efun 필요

    // 파티 소속 확인
    let my_party = get_string(ob, "소속파티");
    if my_party == "" {
        send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^");
        return;
    }

    let my_name = get_string(ob, "이름");

    // 파티장 여부에 따라 다른 프리픽스
    let prefix;
    if my_party == my_name {
        prefix = "\x1b[1m\x1b[40m\x1b[32m◀\x1b[0m\x1b[40m\x1b[37m" + my_name + "\x1b[1m\x1b[40m\x1b[32m▶\x1b[0m\x1b[40m\x1b[37m ";
    } else {
        prefix = "\x1b[1m\x1b[40m\x1b[32m◁\x1b[0m\x1b[40m\x1b[37m" + my_name + "\x1b[1m\x1b[40m\x1b[32m▷\x1b[0m\x1b[40m\x1b[37m ";
    }

    let msg = prefix + line;

    // 파티 메시지 전송
    if send_party_message(ob, msg) {
        // 성공 - send_party_message가 모든 파티 멤버에게 전송함
    } else {
        send_line(ob, "☞ 무리 메시지 전송에 실패했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
