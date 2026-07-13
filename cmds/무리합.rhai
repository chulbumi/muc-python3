// Python `cmds/무리합.py`.

fn group_prompt(person) {
    if !person["show_prompt"] { return ""; }
    "\r\n\x1b[0;37;40m[ " + person["hp"].to_string() + "/" + person["max_hp"].to_string()
        + ", " + person["mp"].to_string() + "/" + person["max_mp"].to_string() + " ] "
}

fn cmd(ob, line) {
    let ctx = get_party_context();
    let self = ctx["self"];
    let leader = ctx["party_leader"];
    if leader["id"] == "" { send_line(ob, "☞ 당신이 속한 무리가 없어요. ^^"); return; }
    if combat_room_has_attr(ob, "전투금지") { send_line(ob, "☞ 이곳에선 모든 전투가 금지되어 있어요. ^^"); return; }
    if line.contains("시체") { send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!"); return; }
    if line != "" { send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*"); return; }
    if leader["id"] != self["id"] { send_line(ob, "☞ 당신은 무리의 대장이 아니에요. ^^"); return; }

    let fighter = ();
    if self["act"] == 1 { fighter = self; }
    if fighter.is_unit() {
        for member in ctx["party_members"] {
            if member["act"] == 1 { fighter = member; break; }
        }
    }
    if fighter.is_unit() { send_line(ob, "☞ 무리 합동공격할 대상이 없어요. ^^"); return; }
    let owners = [self["id"]];
    for member in ctx["party_members"] {
        if member["act"] != 1 { owners.push(member["id"]); }
    }
    // Python compares the player object itself with ACT_FIGHT, so this branch
    // always runs. setAllTarget also moves every participant to the front of
    // each enemy's target list in assignment order.
    party_request_set_group_combat_targets(
        ob,
        owners,
        fighter["combat_target_ids"],
        fighter["combat_target_instance_ids"],
        "",
        true
    );

    let message = "당신이 속한 무리가 무리합동 공격을 시작 합니다.";
    send_line(ob, message);
    for member in ctx["party_members"] {
        party_send_raw(ob, member["id"], "\r\n" + message + "\r\n" + group_prompt(member));
    }
}

fn main(ob, line) { cmd(ob, line); }
