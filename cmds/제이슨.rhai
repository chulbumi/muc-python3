// 제이슨.rhai - 제이슨 명령어 (config JSON 조회)
// Python: cmds/제이슨.py

fn cmd(ob, line) {
    if is_empty(line) {
        send_line(ob, "☞ 사용법: 제이슨 [config 파일명]");
        return;
    }
    let path = "data/config/" + trim(line) + ".json";
    let s = read_text_file(path);
    if s == "" {
        send_line(ob, "☞ 해당 파일이 없습니다: " + path);
        return;
    }
    send_line(ob, s);
}

fn main(ob, line) {
    cmd(ob, line);
}
