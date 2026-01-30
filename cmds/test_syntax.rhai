// Test syntax
fn main(ob, line) {
    let name = get_string(ob, "이름");
    let level = get_int(ob, "레벨");
    let age = get_int(ob, "나이");
    send_line(ob, "Name: " + name);
    send_line(ob, "Level: " + int_to_str(level));
}
