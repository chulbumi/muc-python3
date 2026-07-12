// Python objs/player.py parse_command: `끝`/`종료` one-word connection close.
// User-visible text stays here so hot reload preserves the Python command model.

fn cmd(ob, line) {
    // Python applies this special branch only when argc == 1.
    if !is_empty(line) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if !can_leave_murim(ob) {
        send_line(ob, "☞ 지금은 무림을 떠나기에 좋은 상황이 아니네요. ^_^");
        return;
    }

    request_connection_close(ob, "\r\n다음에 또 만나요~!!!\r\n");
}

fn main(ob, line) {
    cmd(ob, line);
}
