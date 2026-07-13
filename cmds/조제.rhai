// 조제.rhai - 약초 조제
// Python: cmds/조제.py

fn cmd(ob, line) {
    if line == "" { send_line(ob, "☞ 사용법: [이름] 조제"); return; }
    let mobs = get_room_mobs(ob);
    let doctor = false;
    let recipe = "";
    let doctor_name = "의원";
    let i = 0;
    while i < len(mobs) {
        let mob = mobs[i];
        let reactions = mob["reaction_names"];
        let is_doctor = false;
        let j = 0;
        while j < len(reactions) {
            if reactions[j] == "의원" {
                doctor = true;
                is_doctor = true;
                doctor_name = mob["name"];
            }
            j = j + 1;
        }
        // Python checks the recipe only inside the same `is_mob` + 의원
        // branch. A non-doctor following a doctor must not lend its attrs to
        // the previously seen doctor.
        if is_doctor && map_has_key(mob["recipes"], "조제 " + line) {
            recipe = mob["recipes"]["조제 " + line];
            break;
        }
        i = i + 1;
    }
    if !doctor { send_line(ob, "☞ 이곳에 약을 조제할만한 의원이 없어요. ^^"); return; }
    if recipe == "" { send_line(ob, "☞ 그러한 것을 조제할 의원이 없어요. ^^"); return; }
    let lines = split(recipe, "\r\n");
    let give = ""; let amount = 1; let k = 0;
    let needs = [];
    while k < len(lines) {
        let w = line_args(lines[k]);
        if len(w) >= 2 {
            if starts_with(w[0], "+") { give = substring(w[0], 1, len(w[0])); amount = to_int(w[1]); }
            else {
                // Python removes matched indices from its temporary list as
                // each recipe line is checked. Repeated lines for one index
                // therefore require the sum, not the same items repeatedly.
                let merged = false;
                let p = 0;
                while p < len(needs) {
                    if needs[p][0] == w[0] {
                        needs[p] = [needs[p][0], needs[p][1] + to_int(w[1])];
                        merged = true;
                        break;
                    }
                    p = p + 1;
                }
                if !merged { needs.push([w[0], to_int(w[1])]); }
            }
        }
        k = k + 1;
    }
    let inv = list_inventory_indices(ob);
    let n = 0;
    while n < len(needs) {
        let have = 0; let q = 0;
        while q < len(inv) { if inv[q][0] == needs[n][0] { have = inv[q][1]; break; } q = q + 1; }
        if have < needs[n][1] {
            send_line(ob, "\x1b[33m" + doctor_name + "\x1b[37m" + han_iga(doctor_name)
                + " 말합니다. \"음.. 부족한게 있다네... 재료를 더 구해오게나\"");
            return;
        }
        n = n + 1;
    }
    if give == "" { send_line(ob, "☞ 조제 결과물이 없습니다."); return; }
    if !item_template_exists(give) {
        send_line(ob, "\x1b[33m" + doctor_name + "\x1b[37m" + han_iga(doctor_name)
            + " 말합니다. \"음.. 재료가 다 떨어져서 한동안 조제가 힘들겠어...\"");
        return;
    }
    n = 0; while n < len(needs) { destroy_inventory_index(ob, needs[n][0], needs[n][1]); n = n + 1; }
    n = 0;
    while n < amount {
        if item_create(ob, give) == "" {
            send_line(ob, "\x1b[33m" + doctor_name + "\x1b[37m" + han_iga(doctor_name)
                + " 말합니다. \"음.. 재료가 다 떨어져서 한동안 조제가 힘들겠어...\"");
            return;
        }
        n = n + 1;
    }
    let result_name = give;
    let result_color = "";
    for catalog_item in get_item_catalog() {
        if catalog_item["index"] == give {
            result_name = catalog_item["name"];
            result_color = catalog_item["ansi"];
            break;
        }
    }
    let result_ansi = if result_color == "" {
        "\x1b[0;36m" + result_name + "\x1b[37m"
    } else {
        result_color + result_name + "\x1b[0;37m"
    };
    let doctor_post = "\x1b[33m" + doctor_name + "\x1b[37m" + han_iga(doctor_name);
    send_line(ob, "당신이 \x1b[33m" + doctor_name + "\x1b[37m에게 \x1b[36m" + line
        + "\x1b[37m" + han_eul(line) + " 만들수 있는 재료들을 건네줍니다.");
    send_line(ob, doctor_post + " 재료들을 가지고 심오한 기를 불어 넣으며 작업합니다.");
    send_line(ob, doctor_post + " 당신에게 " + result_ansi + han_eul(result_name)
        + " " + amount + "개 줍니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
