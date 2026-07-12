// 제이슨.rhai - 현재 사용자 객체 속성을 서버 콘솔에 JSON으로 출력
// Python: cmds/제이슨.py

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    print(get_body_attrs_json(ob));
}

fn main(ob, line) {
    cmd(ob, line);
}
