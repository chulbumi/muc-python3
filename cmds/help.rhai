// help.rhai - 도움말 (영문/한글 공통). 사용법: help [주제]

fn show_general_help(ob) {
    send_line(ob, "==================================================");
    send_line(ob, "                    도  움  말");
    send_line(ob, "==================================================");
    send_line(ob, "");
    send_line(ob, "[기본 명령어]");
    send_line(ob, "  look <대상>     - 대상이나 방을 봅니다");
    send_line(ob, "  say <내용>      - 주변 사람들에게 말합니다");
    send_line(ob, "  inventory      - 소지하고 있는 아이템을 봅니다");
    send_line(ob, "  attack <대상>   - 대상을 공격합니다");
    send_line(ob, "");
    send_line(ob, "[이동 명령어]");
    send_line(ob, "  north / n      - 북쪽으로 이동합니다");
    send_line(ob, "  south / s      - 남쪽으로 이동합니다");
    send_line(ob, "  east / e       - 동쪽으로 이동합니다");
    send_line(ob, "  west / w       - 서쪽으로 이동합니다");
    send_line(ob, "");
    send_line(ob, "상세 도움말: help <명령어>");
    send_line(ob, "==================================================");
}

fn show_command_help(ob, topic) {
    let help_text = if topic == "look" {
        "look <대상>\n대상이나 방을 자세히 봅니다."
    } else if topic == "say" {
        "say <내용>\n같은 방에 있는 사람들에게 말합니다."
    } else if topic == "inventory" {
        "inventory\n소지하고 있는 아이템 목록을 보여줍니다."
    } else if topic == "attack" {
        "attack <대상>\n대상을 공격하여 전투를 시작합니다."
    } else {
        "☞ 해당 도움말이 없어요. ^^"
    };
    send_line(ob, help_text);
}

fn main(ob, line) {
    let args = line_args(line);
    if len(args) == 0 || is_empty(line) {
        show_general_help(ob);
    } else {
        let topic = args[0];
        show_command_help(ob, topic);
    }
}
