// 아이템제작.rhai - 아이템 제작 (관리자)
// Python: cmds/아이템제작.py

fn cmd(ob, line) {
    let words = split(line, " ");
    if len(words) < 1 {
        send_line(ob, "☞ 사용법: [파일명] 아이템제작");
        return;
    }
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^");
        return;
    }
    let target_path = "item/" + words[0].to_string() + ".json";
    begin_file_edit(ob, target_path, "작성을 마치시려면 '.' 를 입력하세요.\r\n:");
}

fn main(ob, line) {
    cmd(ob, line);
}
