// Python: cmds/암호변경.py, objs/player.py get_oldpass/change_password/change_password1

fn cmd(ob, line) {
    begin_password_change(
        ob,
        "이전암호ː ",
        "☞ 현재의 암호가 맞지 않아요. ^^\r\n",
        "☞ 변경 하실 암호를 입력해주세요. \r\n존함암호ː",
        "☞ 한번 더 암호를 입력해주세요. \r\n암호확인ː",
        "☞ 이전 입력과 다릅니다. 암호변경을 취소합니다.\r\n",
        "☞ 암호가 변경되었습니다."
    );
}

fn main(ob, line) {
    cmd(ob, line);
}
