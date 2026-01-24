// look.rhai - 방과 주변을 보는 명령어
//
// 사용법: look
// 현재 방을 봅니다.

fn main(ob, line) {
    // line: 대상이 있으면 대상을, 없으면 방을 표시. ob.env, ob.channel 미지원으로 방/대상 조회는 제한적.
    send_line(ob, "=== ??? ===");
    send_line(ob, "☞ 봐/지도 명령은 현재 맵·방 데이터 조회 efun 연동 후 구현됩니다.");
}
