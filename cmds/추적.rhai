// 추적.rhai - 몹 추적 (관리자 lv 1000)
// Python: cmds/추적.py

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if is_empty(line) {
        send_line(ob, "몹이름 존이름 추적");
        return;
    }

    let words = split(line, " ");
    if len(words) < 2 {
        send_line(ob, "몹이름 존이름 추적");
        return;
    }

    let mob_name = words[0];
    let zone_name = words[1];

    // get_room_data로 존의 방 정보 확인
    // 존 내 모든 방을 순회하며 몹 찾기
    // 현재 구현: 존의 존재 여부만 확인 (전체 순회는 추가 efun 필요)

    // 테스트용: 몇 가지 일반적인 방 번호로 확인
    // 실제 구현에서는 존의 모든 방 목록을 가져오는 efun이 필요
    let test_rooms = ["1", "100", "200", "300"];
    let mut found = false;
    let mut i = 0;

    while i < len(test_rooms) {
        let room_spec = zone_name + ":" + test_rooms[i];
        let room_data = get_room_data(room_spec);

        if !room_data.is_unit() {
            // 방 데이터가 있으면 몹 확인
            // TODO: 방의 몹 목록을 확인하는 기능이 추가되면 구현
            // 현재는 방이 존재함을 확인만 가능
        }

        i = i + 1;
    }

    // 현재 버전에서는 제한적인 기능만 제공
    send_line(ob, "☞ 추적 기능: 존 '" + zone_name + "' 확인 중...");
    send_line(ob, "   (몹: " + mob_name + ")");
    send_line(ob, "   현재 버전에서는 방별 몹 검색이 제한적입니다.");
    send_line(ob, "   몹위치 파일 확인 또는 관리자 도구를 사용하세요.");
}

fn main(ob, line) {
    cmd(ob, line);
}
