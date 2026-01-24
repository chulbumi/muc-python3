// 앞앞.rhai - 앞앞. Python: cmds/앞앞.py (앞와 동일)
// [대상] 채널.players에서 이름 찾아 ob.env=ply.env. 관리자 2000.

fn cmd(ob, line) {
    if require_admin(ob, 2000) {
        if line == "" || is_empty(line) {
            send_line(ob, "☞ 운영자 명령: [대상] 앞");
            return;
        }
        send_line(ob, "☞ 활동중인 그런 무림인이 없어요. ^^ (channel.players/enterRoom 연동 예정)");
    } else {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
