// 값삭제.rhai - [대상] [키]. del_obj_attr(ob, target, key). 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let a = line_args(line);
    if len(a) < 2 {
        send_line(ob, "☞ 사용법: [대상] [키] 값삭제");
        return;
    }
    let target = a[0];
    let key = a[1];
    if del_obj_attr(ob, target, key) {
        send_line(ob, "☞ 값이 삭제되었습니다.");
    } else {
        send_line(ob, "☞ 해당 키가 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
