// Python `cmds/내공주입.py`. State work is performed by inject_guard_qi.

fn cmd(ob, line) {
    let result = inject_guard_qi(ob);
    if result["status"] == "no_guards" {
        send_line(ob, "☞ 호위를 거느리지 않고 있습니다.");
        return;
    }
    if result["status"] == "mp_shortage" {
        send_line(ob, "☞ 내가진기를 주입할 내공이 부족합니다.");
        return;
    }
    if result["status"] == "full" {
        send_line(ob, "☞ 회복할 호위가 없습니다.");
        return;
    }
    let lines = "";
    for entry in result["healed"] {
        lines += "당신이 " + entry["이름"] + "에게 내가진기를 주입하여 체력을 회복 시킵니다. (\x1b[1;36m+"
            + entry["회복"].to_string() + "\x1b[0;37m)\r\n";
    }
    send_line(ob, lines);
    send_line(ob, "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-" + result["spent"].to_string() + "\x1b[0;37m)");
}

fn main(ob, line) { cmd(ob, line); }
