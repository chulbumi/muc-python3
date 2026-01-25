// 명령어리스트.rhai - 명령어리스트. Python: cmds/명령어리스트.py
// ob.cmdList 순회, level==1000인 명령만. cmdList efun 없음 → get_help 사용.

fn cmd(ob, line) {
    if require_admin(ob, 1000) {
        let h = get_help("명령어");
        if h != "" {
            send_line(ob, h);
        } else {
            send_line(ob, "☞ 도움말 명령으로 확인하세요.");
        }
    } else {
        send_line(ob, "☞ 관리자만 사용할 수 있어요.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
