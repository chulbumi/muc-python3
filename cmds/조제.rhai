// 조제.rhai - 약초 조제
// Python: cmds/조제.py

fn cmd(ob, line) {
    if is_empty(line) { send_line(ob, "☞ 사용법: [이름] 조제"); return; }
    let mobs = get_room_mobs(ob);
    let doctor = false;
    let recipe = "";
    let doctor_name = "의원";
    let i = 0;
    while i < len(mobs) {
        let mob = mobs[i];
        let reactions = mob["reaction_names"];
        let j = 0;
        while j < len(reactions) {
            if contains(reactions[j], "의원") {
                doctor = true;
                doctor_name = mob["desc1"];
            }
            j = j + 1;
        }
        if doctor && mob["events"].contains_key("조제 " + line) {
            recipe = mob["events"]["조제 " + line];
            break;
        }
        i = i + 1;
    }
    if !doctor { send_line(ob, "☞ 이곳에 약을 조제할만한 의원이 없어요. ^^"); return; }
    if recipe == "" { send_line(ob, "☞ 그러한 것을 조제할 의원이 없어요. ^^"); return; }
    let lines = recipe.split("\r\n");
    let give = ""; let amount = 1; let k = 0;
    let needs = [];
    while k < len(lines) {
        let w = lines[k].split(" ");
        if len(w) >= 2 {
            if starts_with(w[0], "+") { give = substring(w[0], 1, len(w[0])); amount = to_int(w[1]); }
            else { needs.push([w[0], to_int(w[1])]); }
        }
        k = k + 1;
    }
    let inv = list_inventory(ob);
    let n = 0;
    while n < len(needs) {
        let have = 0; let q = 0;
        while q < len(inv) { if inv[q][0] == needs[n][0] { have = inv[q][1]; break; } q = q + 1; }
        if have < needs[n][1] { send_line(ob, doctor_name + " 말합니다. \"음.. 부족한게 있다네... 재료를 더 구해오게나\""); return; }
        n = n + 1;
    }
    n = 0; while n < len(needs) { item_destroy(ob, needs[n][0], 1, needs[n][1]); n = n + 1; }
    if give == "" { send_line(ob, "☞ 조제 결과물이 없습니다."); return; }
    n = 0;
    while n < amount {
        if item_create(ob, give) == "" {
            send_line(ob, doctor_name + " 말합니다. \"음.. 재료가 다 떨어져서 한동안 조제가 힘들겠어...\"");
            return;
        }
        n = n + 1;
    }
    send_line(ob, "당신이 " + line + "을(를) 조제할 재료를 건넵니다.");
    send_line(ob, doctor_name + " 재료들을 가지고 작업합니다.");
    send_line(ob, doctor_name + "이(가) 당신에게 " + give + " " + amount + "개 줍니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
