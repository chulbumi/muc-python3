// 구입.rhai - 구입 명령어
// Python: cmds/구입.py

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [물품이름] [수량] 구입") { return; }
    // ob.env, findMerchant, alias, 상인 거래 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
