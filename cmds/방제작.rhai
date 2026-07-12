// 방제작.rhai - 방 제작 (관리자)
// Python: cmds/방제작.py

fn cmd(ob, line) {
    let words = split(line, " ");
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [존이름] [방이름] 방제작");
        return;
    }
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^");
        return;
    }
    let target_path = "map/" + words[0].to_string() + "/" + words[1].to_string() + ".json";
    begin_file_edit(ob, target_path, "작성을 마치시려면 '.' 를 입력하세요.\r\n:");
}

fn main(ob, line) {
    cmd(ob, line);
}
