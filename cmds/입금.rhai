// 입금.rhai - 입금 (보관함에 돈 넣기의 별칭)
// Python: cmds/입금.py

fn cmd(ob, line) {
    send_line(ob, "☞ 입금 기능은 '넣어' 명령을 사용하세요.");
}

fn main(ob, line) {
    cmd(ob, line);
}
