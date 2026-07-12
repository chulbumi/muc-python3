// comm.rhai - Python comm.py의 follower/group 해제 helper
// Python: cmds/comm.py

fn cmd(ob, line) {
    let ctx = get_party_context();
    let followers = ctx["followers"];
    if len(followers) == 0 {
        send_line(ob, "☞ 당신을 따르는 무리가 없어요. ^^");
        return;
    }
    let ids = [];
    let i = 0;
    while i < len(followers) {
        let member = followers[i];
        if is_empty(line) || line == "모두" || member["name"] == line {
            ids.push(member["id"]);
        }
        i = i + 1;
    }
    if len(ids) == 0 { send_line(ob, "☞ 당신을 따르는 그런 대상이 없어요. ^^"); return; }
    if party_request_remove_followers(ob, ids) {
        send_line(ob, "☞ 무리에서 대상을 제외시켰습니다.");
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
