// Python cmds/똥파말.py is byte-for-byte the same command as 방파말.py.
fn cmd(ob, line) {
    if get_string(ob, "소속") == "" {
        send_line(ob, "☞ 당신은 소속이 없습니다.");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법 : [내용] 방파말(])");
        return;
    }
    if get_user_config(ob, "방파말거부") == "1" {
        send_line(ob, "☞ 방파말 거부중 이에요. *^^*");
        return;
    }
    append_guild_chat_log(ob, line);
    send_python_guild_group(ob, line);
}
fn main(ob, line) { cmd(ob, line); }
