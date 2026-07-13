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
    let result = append_obj_attr_value(ob, a[0], a[1], a[2]);
    if result == "no_target" { send_line(ob, "☞ 그런 대상이 없어요!"); }
    else if result == "failed" { send_line(ob, "☞ 속성추가를 실패했습니다."); }
    else { send_line(ob, "☞ 속성이 추가 되었습니다."); }
}

fn main(ob, line) {
    cmd(ob, line);
}
