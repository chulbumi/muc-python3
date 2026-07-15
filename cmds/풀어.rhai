// 포장(묶음) 아이템을 하나 풀어 원본 아이템 수량으로 되돌린다.
// 아이템 이벤트는 명령 dispatch에서 이 스크립트보다 먼저 조사된다.
fn cmd(ob, line) {
    let query = trim(line);
    if query == "" {
        send_line(ob, "☞ 사용법: [포장아이템] 풀어");
        return;
    }
    let result = unpack_item(ob, query);
    if result["status"] == "usage" {
        send_line(ob, "☞ 사용법: [포장아이템] 풀어");
    } else if result["status"] == "not_found" {
        send_line(ob, "☞ 그런 포장 아이템이 소지품에 없어요.");
    } else if result["status"] == "invalid_original" || result["status"] == "invalid_package" {
        send_line(ob, "☞ 그 포장은 풀 수 없는 상태입니다.");
    } else {
        send_line(ob, "당신이 " + result["package"] + "을(를) 풀어 "
            + result["original"] + " " + result["quantity"].to_string() + "개를 꺼냅니다.");
    }
}

fn main(ob, line) { cmd(ob, line); }
