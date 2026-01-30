// 몹제작.rhai - 사용자 정의 몹 생성
// Python: cmds/몹제작.py
// Admin level 1000 (Python: 1000)

fn cmd(ob, line) {
    let admin_level = get_int(ob, "관리자등급");
    if admin_level < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^");
        return;
    }

    let words = split(line, " ");
    if len(words) < 2 {
        send_line(ob, "☞ 사용법: [존이름] [몹이름] 몹제작");
        return;
    }

    let zone_name = words[0];
    let mob_name = words[1];

    // 몹 데이터 파일 경로 생성
    let file_path = "mob/" + zone_name + "/" + mob_name + ".json";

    // 몹 데이터 템플릿 생성
    send_line(ob, "작성을 마치시려면 '.' 를 입력하세요.");
    send_line(ob, "대상 파일: " + file_path);
    send_line(ob, "");
    send_line(ob, "☞ 몹 데이터는 JSON 형식으로 작성해야 합니다.");
    send_line(ob, "   현재 버전에서는 대화형 입력이 제한적입니다.");
    send_line(ob, "   몹 템플릿을 생성하려면: [존이름] [몹이름] [체력] [공격력] 몹생성");

    // TODO: 대화형 입력 모드 구현 필요
    // 현재는 안내 메시지만 출력
}

fn main(ob, line) {
    cmd(ob, line);
}
