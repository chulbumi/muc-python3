// Private entry-event runner shared by login and new-character creation.
// Movement and return run the same efun inline so their Python output order is
// preserved.  Keeping this as a command script lets delayed event requests
// use the normal call_out-aware Rhai command boundary.

fn main(ob, _line) {
    let events = run_return_entry_events(ob);
    if events["status"] != "ok" {
        return;
    }
    for line in events["lines"] {
        send_line(ob, line);
    }
}
