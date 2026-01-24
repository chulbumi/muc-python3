// 전음.rhai - 전음(귓속말). [대상] [내용] 전음(/). send_tell(ob, target, msg) → Tell
// Python: cmds/전음.py. 전음거부/자기자신(중얼) 검사는 efun에서

fn cmd(ob, line) {
    let parts = line_args(line);
    if len(parts) < 2 {
        send_line(ob, "☞ 사용법: [대상] [내용] 전음(/)");
        return;
    }
    let target = parts[0];
    let msg = join_after(parts, 1);
    send_tell(ob, target, trim(msg));
}

fn join_after(arr, from) {
    let s = "";
    let i = from;
    while i < len(arr) {
        s = s + arr[i];
        if i + 1 < len(arr) { s = s + " "; }
        i = i + 1;
    }
    s
}

fn main(ob, line) {
    cmd(ob, line);
}
