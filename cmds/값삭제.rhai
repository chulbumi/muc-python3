// 값삭제.rhai - [대상] [키]. del_obj_attr(ob, target, key). 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let result = admin_delete_space_value(ob, line);
    if result == "usage" {
        send_line(ob, "☞ 사용법: [대상] [키] 값삭제");
    } else if result == "missing" {
        send_line(ob, "☞ 그런 대상이 없어요!");
    } else if result == "ok" {
        send_line(ob, "☞ 값이 삭제되었습니다.");
    } else {
        send_line(ob, "☞ 해당 키가 없습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
