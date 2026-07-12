// 찾아라.rhai - 오브젝트 찾기
// Python: cmds/찾아라.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    for room_name in find_object_rooms(ob, line) {
        send_line(ob, room_name);
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
