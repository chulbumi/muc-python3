// Python `cmds/죽여.py`. Successful execution emits only Mob.die room output.

fn cmd(ob, line) {
    if !require_admin(ob, 2000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 죽여");
        return;
    }
    let target = find_attack_target(ob, line);
    if target.is_unit() || target["kind"] != "mob" || target["attack_forbidden"] {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }
    let result = admin_kill_mob(ob, target["id"], target["room_index"]);
    if result["status"] != "ok" {
        send_line(ob, "☞ 강호에는 공격할 수 있는것과 없는것이 있지!");
        return;
    }
    let text = if result["death_script"] == "" {
        "\r\n\x1b[1;37m" + result["name"] + han_iga(result["name"])
            + " 쓰러집니다. '쿠웅~~ 철퍼덕~~'\x1b[0;37m"
    } else {
        "\r\n\x1b[1;37m" + result["death_script"] + "\x1b[0;37m"
    };
    // Python Room.writeRoom writes these bytes verbatim, including the
    // issuer, and then writes a second CRLF. Generic send_to_user would wrap
    // the payload in another CRLF pair, so use the opaque raw delivery path.
    let context = get_party_context();
    for player in context["room_players"] {
        party_send_raw(ob, player["id"], text + "\r\n");
    }
}

fn main(ob, line) { cmd(ob, line); }
