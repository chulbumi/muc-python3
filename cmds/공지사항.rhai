// 공지사항.rhai - 공지사항. Python: cmds/공지사항.py
// cat(ob, 'data/text/notice.txt')

fn cmd(ob, line) {
    let txt = read_text_file("data/text/notice.txt");
    if txt != "" {
        let arr = split(txt, "\n");
        for a in arr {
            let s = trim(a);
            if s != "" {
                send_line(ob, s);
            }
        }
    } else {
        send_line(ob, "☞ 등록된 공지사항이 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
