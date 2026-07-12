// 값설정.rhai - [대상] [키] [값]. set_obj_attr(ob, target, key, val). 관리자 2000.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    let a = line_args(line);
    if len(a) < 3 {
        send_line(ob, "☞ 사용법: [대상] [키] [값] 값설정");
        return;
    }
    let target = a[0];
    let key = a[1];
    let val = a[2];
    if length(val.to_string()) > 50 {
        send_line(ob, "☞ 너무 길어요!");
        return;
    }
    if set_obj_attr(ob, target, key, val) {
        send_line(ob, "☞ 값이 설정되었습니다.");
    } else {
        send_line(ob, "☞ 그런 대상이 없어요!");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
