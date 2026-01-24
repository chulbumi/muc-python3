// 장비.rhai - 장비 명령어
// Python: cmds/장비.py

fn cmd(ob, line) {
    // ob.objs 내 inUse 아이템, get_arm_script, postPosition1, 방어력/공격력 등 서버 연동 필요
    send_line(ob, "☞ 이 기능은 서버 기능 연동 후 사용할 수 있습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
