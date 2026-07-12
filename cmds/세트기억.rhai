// 세트기억.rhai - 장비 세트 저장
// Python: cmds/세트기억.py

fn cmd(ob, line) {
    let count = remember_equipment_set(ob);
    if count == 0 {
        send_line(ob, "☞ 무엇을 기억하시려구요?.");
    } else {
        send_line(ob, "☞ 기억 되었습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
