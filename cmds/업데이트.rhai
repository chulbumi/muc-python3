// Python cmds/업데이트.py parity.
// Reload mechanics are efuns; every user-visible string remains here.

fn cmd(ob, line) {
    if get_int(ob, "관리자등급") < 1000 {
        send_line(ob, "☞ 무슨 말인지 모르겠어요. *^_^*");
        return;
    }

    if line == "" {
        send_line(ob, "* 명령어, 무림별호, 도움말, 무공, 표현, 도우미, 메인설정, 스크립트 중에 선택하세요");
    } else if line == "명령어" {
        if reload_runtime_data(ob, "명령어") {
            send_line(ob, "* 명령어가 업데이트 되었습니다.");
        }
    } else if line == "무림별호" {
        if reload_runtime_data(ob, "무림별호") {
            send_line(ob, "* 무림별호가 업데이트 되었습니다.");
        }
    } else if line == "도움말" {
        if reload_runtime_data(ob, "도움말") {
            send_line(ob, "* 도움말이 업데이트 되었습니다.");
        }
    } else if line == "무공" {
        if reload_runtime_data(ob, "무공") {
            send_line(ob, "* 무공이 업데이트 되었습니다.");
        }
    } else if line == "표현" {
        if reload_runtime_data(ob, "표현") {
            send_line(ob, "* 표현이 업데이트 되었습니다.");
        }
    } else if line == "도우미" {
        if reload_runtime_data(ob, "도우미") {
            send_line(ob, "* 도우미가 업데이트 되었습니다.");
        }
    } else if line == "메인설정" {
        if reload_runtime_data(ob, "메인설정") {
            send_line(ob, "* 메인설정이 업데이트 되었습니다.");
        }
    } else if line == "스크립트" {
        if reload_runtime_data(ob, "스크립트") {
            send_line(ob, "* 스크립트가 업데이트 되었습니다.");
        }
    }
}

fn main(ob, line) {
    cmd(ob, line);
}
