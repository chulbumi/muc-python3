// 오브젝트저장.rhai - 오브젝트 저장
// Python: cmds/오브젝트저장.py

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 2000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let result = save_object_python(ob, line);
    if result["status"] == "ok" {
        send_line(ob, "* " + result["path"] + " 저장되었습니다.");
    } else if result["status"] == "missing" {
        send_line(ob, "☞ 그런 대상이 없어요!");
    } else if result["status"] == "cannot_save" {
        send_line(ob, "☞ 저장할 수 없어요!");
    } else {
        send_line(ob, "* 파일 열기를 실패하였습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
