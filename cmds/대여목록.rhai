// 대여목록.rhai - 대여목록. Python: cmds/대여목록.py
// ob.env.findObjName('진영'), data/config/book.dat (pickle). findObjName(진영), read pickle 없음.

fn cmd(ob, line) {
    send_line(ob, "☞ 대여가능한 품목이 없어요.");
}

fn main(ob, line) {
    cmd(ob, line);
}
