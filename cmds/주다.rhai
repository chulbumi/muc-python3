// 주다.rhai - 주다(줘/give). [대상] [물품] [개수] 주다. 물품=은전|금전|아이템이름
// Python: cmds/줘.py. request_give_silver, request_give_gold, request_give_item → GiveToPlayer

fn cmd(ob, line) {
    if !require_arg(ob, line, "☞ 사용법: [대상] [물품] [개수] 주다") { return; }
    let args = line_args(line);
    let target = args[0];
    let thing = args[1];
    let cnt = if len(args) >= 3 { to_int(args[2]) } else { 1 };
    let cnt = max(1, min(50, cnt));

    if get_string(ob, "이름") == target {
        if thing == "은전" || thing == "금전" {
            send_line(ob, "☞ 자기 자신에게 돈을 줄 수 없어요. ^^");
            return;
        }
        send_line(ob, "☞ 자기 자신에게 물품을 줄 수 없어요. ^^");
        return;
    }

    if thing == "은전" {
        let err = request_give_silver(ob, target, cnt);
        if length(err) > 0 { send_line(ob, err); }
        return;
    }
    if thing == "금전" {
        let err = request_give_gold(ob, target, cnt);
        if length(err) > 0 { send_line(ob, err); }
        return;
    }

    let arr = parse_name_order(thing);
    let iname = arr[0];
    let iord = arr[1];
    if length(iname) == 0 {
        send_line(ob, "☞ 사용법: [대상] [물품] [개수] 주다");
        return;
    }
    let c = if iord > 1 { 1 } else { cnt };
    let err = request_give_item(ob, target, iname, iord, c);
    if length(err) > 0 { send_line(ob, err); }
}

fn main(ob, line) {
    cmd(ob, line);
}
