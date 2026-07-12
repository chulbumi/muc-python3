// Private target-side Python enterRoom(..., "소환", "소환") handler.
fn main(ob, line) {
    let words = line_args(line);
    if len(words) != 2 { return; }
    perform_summon_move(ob, words[0], words[1]);
}
