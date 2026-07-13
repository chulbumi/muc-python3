// Python `cmds/안시.py`: current HP as a ten-cell ANSI bar.

fn cmd(ob, line) {
    let hp = get_int(ob, "체력");
    let max_hp = get_int(ob, "최고체력");
    // Normal Player bodies always have positive max HP, matching Python's
    // direct integer division contract.
    let green = python_floor_div(10 * hp, max_hp);
    send_line(ob,
        "\x1b[32m" + repeat("━", green)
        + "\x1b[37m" + repeat("━", 10 - green));
}

fn main(ob, line) { cmd(ob, line); }
