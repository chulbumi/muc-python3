start_script(ob);
set_tick(1);
send_line(ob, "Testing get_name...");
let name = get_name();
send_line(ob, "Got name!");
finish_script(ob);
