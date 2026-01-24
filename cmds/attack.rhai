// attack.rhai - 공격 명령어
//
// 사용법: attack
// 대상을 공격합니다.

fn main(ob, line) {
    let hp = get_int(ob, "hp");
    let name = get_string(ob, "이름");

    if hp < 10 {
        send_line(ob, "☞ 체력이 너무 부족하여 공격할 수 없습니다.");
        return;
    }
    if is_empty(line) {
        send_line(ob, "☞ 사용법: [대상] 공격");
        return;
    }

    let iga = ob_iga(ob);
    send_line(ob, "당신이 공격 자세를 취합니다!");
    send_room(ob, name + iga + " 공격 자세를 취합니다.");
    // ob.env.findObjName, 전투 루프, 데미지 등 서버 연동 필요
}
