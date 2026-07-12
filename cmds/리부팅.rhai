// Python cmds/리부팅.py
// 방 갱신과 서버 stop은 Rust efun이 수행하고, 권한 문구는 여기에만 둔다.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    // Python has no success output.
    request_reboot(ob);
}

fn main(ob, line) {
    cmd(ob, line);
}
