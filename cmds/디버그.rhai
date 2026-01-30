fn main(ob, line) {
    send_line(ob, "DEBUG: Script executed!");
    send_line(ob, "DEBUG: Name is " + get_string(ob, "이름"));
}
