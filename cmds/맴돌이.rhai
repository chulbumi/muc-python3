// 맴돌이.rhai - Python: cmds/맴돌이.py
// [출구]: ob.env['출구']를 파싱해 해당 출구(이름 또는 이름$)의 목적지를 ob.env.index(자기 방)로 바꿈. c>=1이면 "출구가 맴돌이 되었습니다", else "그런 출구가 없습니다". 출구/방 수정 efun 연동 예정.

fn cmd(ob, line) {
    if !require_admin(ob, 1000) {
        return;
    }
    if line == "" || is_empty(line) {
        send_line(ob, "☞ 사용법: [출구] 맴돌이");
        return;
    }
    // ob.env['출구'] 파싱, ex.split(None,1), x[0][:-1]==line && x[0][-1]=='$' or x[0]==line → 목적지를 ob.env.index로, nexists에 반영, ob.env['출구']=nexists, init (연동 예정)
    send_line(ob, "☞ 그런 출구가 없습니다. (출구/방 데이터 수정 연동 예정)");
}

fn main(ob, line) {
    cmd(ob, line);
}
