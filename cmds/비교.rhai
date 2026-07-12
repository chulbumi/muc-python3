// Python `cmds/비교.py`: 같은 방 플레이어/몹과 예상 전투 횟수를 비교한다.

fn cmd(ob, line) {
    if line == "" {
        send_line(ob, "☞ 사용법: [대상] 비교");
        return;
    }
    let result = compare_combat_target(ob, line);
    if result["status"] == "invalid" || result["status"] == "self" {
        send_line(ob, "자신의 상태를 통탄해 합니다. @_@");
        return;
    }
    if result["status"] == "hidden" {
        send_line(ob, "☞ 그런 비교대상이 없어요. ^^");
        return;
    }
    if result["status"] == "refused" {
        send_line(ob, "☞ 진정한 승부란 비무를 통해서 알 수 있는 것 이지");
        return;
    }
    let name = result["name"].to_string();
    send_line(ob, "━━━━━━━━━━━━━━━");
    send_line(ob, "▶ \x1b[1m" + name + "\x1b[0;37m" + han_wa(name) + "의 상대비교");
    send_line(ob, "───────────────");
    send_line(ob, "☞ 당신의 승률 오차ː" + result["mine"].to_string());
    send_line(ob, "☞ 상대의 승률 오차ː" + result["other"].to_string());
    send_line(ob, "☞ 승  률 오차 결과ː" + (result["mine"].to_int() - result["other"].to_int()).to_string());
    send_line(ob, "━━━━━━━━━━━━━━━");
}

fn main(ob, line) { cmd(ob, line); }
