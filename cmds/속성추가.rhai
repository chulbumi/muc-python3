// 속성추가.rhai - 아이템 속성 추가
// Python: cmds/속성추가.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let a = line_args(line);
    if a.len() < 3 {
        send_line(ob, "☞ 사용법: [대상] [키] [값] 키값설정");
        return;
    }
    let target = a[0];
    let key = a[1];
    let value = a[2];
    let current = get_obj_attr(ob, target, key);
    if current.is_unit() {
        if set_obj_attr(ob, target, key, value) {
            send_line(ob, "☞ 속성이 추가 되었습니다.");
        } else { send_line(ob, "☞ 그런 대상이 없어요!"); }
        return;
    }
    if current.is_string() {
        if set_obj_attr(ob, target, key, current + "\r\n" + value) {
            send_line(ob, "☞ 속성이 추가 되었습니다.");
        } else { send_line(ob, "☞ 속성추가를 실패했습니다."); }
    } else {
        send_line(ob, "☞ 속성추가를 실패했습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
