// Python `cmds/방어지정.py`.

fn party_combat_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/" + person["max_hp"].to_string()
        + ", " + person["mp"].to_string() + "/" + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    if line == "" { send_line(ob, "☞ 사용법 : [무리원이름] 방어지정"); }
    let ctx = get_party_context();
    let self = ctx["self"];
    let leader = ctx["party_leader"];
    if leader["id"] == "" { send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^"); return; }
    if combat_room_has_attr(ob, "전투금지") { send_line(ob, "☞ 이곳에선 모든 전투가 금지되어 있어요. ^^"); return; }
    if line.contains("시체") { send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!"); return; }
    if leader["id"] != self["id"] { send_line(ob, "☞ 당신은 무리의 대장이 아니에요. ^^"); return; }

    let tanker = self;
    if line != leader["name"] {
        tanker = ();
        for member in ctx["party_members"] {
            if member["name"] == line { tanker = member; break; }
        }
    }
    if tanker.is_unit() { send_line(ob, "☞ 당신의 무리원이 아니에요."); return; }

    let fighter = ();
    if leader["act"] == 1 { fighter = leader; }
    if fighter.is_unit() {
        for member in ctx["party_members"] {
            if member["act"] == 1 { fighter = member; break; }
        }
    }
    if fighter.is_unit() { send_line(ob, "☞ 전투중인 무리원이 없어요. ^^"); return; }
    party_request_set_group_combat_targets(ob, [tanker["id"]], fighter["combat_target_ids"], tanker["id"]);

    let message = "\x1b[1m" + tanker["name"] + "\x1b[0;37m"
        + han_iga(tanker["name"]) + " 무리의 방어로 지정 되었습니다.";
    if leader["id"] != tanker["id"] { send_line(ob, message); }
    for member in ctx["party_members"] {
        if member["id"] == tanker["id"] { continue; }
        party_send_raw(ob, member["id"], "\r\n" + message + "\r\n" + party_combat_prompt(member));
    }
    if tanker["id"] == self["id"] { send_line(ob, "\r\n당신이 무리의 방어로 지정 되었습니다."); }
    else { party_send_raw(ob, tanker["id"], "\r\n당신이 무리의 방어로 지정 되었습니다.\r\n"); }
}

fn main(ob, line) { cmd(ob, line); }
