// 공지사항.rhai - 공지사항. Python: cmds/공지사항.py
// cat(ob, 'data/text/notice.txt')

fn cmd(ob, line) {
    let txt = read_text_file("data/text/notice.txt");
    if text_file_exists("data/text/notice.txt") {
        send_line(ob, return_replace(txt, "\n", "\r\n"));
    } else {
        send_line(ob, "[filename] 열 수 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
