// master.rhai - Master Object for Driver/Mudlib Coordination
//
// The master object is the central coordinator between the Rust driver
// and the Rhai mudlib. It implements LPMUD "applies" pattern.
//
// Based on MudOS/FluffOS architecture:
// - https://www.fluffos.info/
//
// This script is loaded at server startup and provides callback functions
// that the driver calls at specific events.

// ============================================================================
// CONNECTION APPLIES
// ============================================================================

/// CONNECT apply - Called when a new player connects
///
/// @param player_name - Name of the connecting player
/// @return Login object path or error message
fn connect(player_name) {
    // Return the login object for new players
    // The driver will load this object and call its logon() function
    return "cmds/login.rhai";
}

/// LOGON apply - Called after player is logged in
///
/// @param player_name - Name of the logged in player
/// @return Player object path
fn logon(player_name) {
    print("[MASTER] logon() - " + player_name);

    // Return the player object
    return "objs/player/" + player_name + ".rhai";
}

// ============================================================================
// ERROR HANDLING APPLIES
// ============================================================================

/// ERROR_HANDLER apply - Called when an error occurs
///
/// @param error - The error message
/// @param source - Source file where error occurred
/// @return true to continue, false to shutdown
fn error_handler(error, source) {
    // Log the error
    // In a full implementation, would write to log file

    // Return true to continue running (don't shutdown)
    return true;
}

/// CRASH apply - Called when the driver crashes
///
/// @param error - The crash error message
fn crash(error) {
    // Last chance to save state before shutdown
    // In a full implementation, would:
    // 1. Save all player data
    // 2. Save world state
    // 3. Write crash log
    return "";
}

// ============================================================================
// COMPILATION APPLIES
// ============================================================================

/// VALID_COMPILE apply - Check if a file can be compiled
///
/// @param path - File path to check
/// @return true to allow, false to deny
fn valid_compile(path) {
    // Security checks
    // Don't allow compiling files outside of game directories
    if path.contains("..") {
        print("[MASTER] Security: Attempted to compile path with ..");
        return false;
    }

    // Only allow cmds/ and lib/ directories
    if !starts_with(path, "cmds/") && !starts_with(path, "lib/") {
        print("[MASTER] Security: Attempted to compile outside allowed dirs");
        return false;
    }

    return true;
}

// ============================================================================
// OBJECT LIFECYCLE APPLIES
// ============================================================================

/// CREATE apply - Called when an object is first created
///
/// @param object_path - Path of the object being created
fn create(object_path) {
    // In a full implementation, would:
    // 1. Initialize object properties
    // 2. Set up object state
    // 3. Register with appropriate managers
    return "";
}

/// RESET apply - Called periodically to refresh objects
///
/// @param object_path - Path of the object being reset
fn reset(object_path) {
    // In a full implementation, would:
    // 1. Regenerate mobs/items in rooms
    // 2. Refresh object state
    // 3. Handle timed events
    return "";
}

/// INIT apply - Called when two objects meet
///
/// @param object - The object being encountered
/// @param living - The living object encountering it
fn init(object, living) {
    // In a full implementation, would:
    // 1. Call object's init() function
    // 2. Allow object to add actions to living
    // 3. Set up any interactions
    return "";
}

/// MOVE_OR_DESTRUCT apply - Called when an object is being moved
///
/// @param object - The object being moved
/// @param destination - Where it's being moved to
/// @return true to allow move, false to destruct instead
fn move_or_destruct(object, destination) {
    // Default: allow the move
    true
}

// ============================================================================
// UID/SECURITY APPLIES
// ============================================================================

/// GET_BB_UID apply - Get the base UID for an object
///
/// @param object - Object to get UID for
/// @return Base UID
fn get_bb_uid(object) {
    return "mudlib";
}

/// GET_ROOT_UID apply - Get the root UID
///
/// @return Root UID string
fn get_root_uid() {
    return "root";
}

/// AUTHOR_FILE apply - Get the author file for a domain
///
/// @param domain - Domain name
/// @return Author file path
fn author_file(domain) {
    "domains/" + domain + "/AUTHOR"
}

/// DOMAIN_FILE apply - Get the domain file for a path
///
/// @param path - File path
/// @return Domain name or empty string
fn domain_file(path) {
    // Extract domain from path
    // domains/xxx/yyy -> xxx
    if starts_with(path, "domains/") {
        let parts = split(path, "/");
        if len(parts) >= 2 {
            return "domains/" + parts[1];
        }
    }
    ""
}

// ============================================================================
// STARTUP APPLIES
// ============================================================================

/// PRELOAD apply - Get list of objects to preload at startup
///
/// @return Array of object paths to preload
fn preload() {
    [
        "cmds/도움말.rhai",
        "cmds/저장.rhai",
        "cmds/누구.rhai",
        "cmds/상태보기.rhai",
    ]
}

/// EPILOG apply - Called after all objects are loaded
///
/// This is the final step of server initialization
fn epilog() {
    return "";
}

// ============================================================================
// MISC APPLIES
// ============================================================================

/// GET_SAVE_FILE_NAME apply - Get save file name for an object
///
/// @param object - Object to save
/// @return Save file path
fn get_save_file_name(object) {
    "save/" + object + ".json"
}

/// CLEAN_UP apply - Called when an object might need cleanup
///
/// @param object - Object to check
/// @return true if object should be destructed
fn clean_up(object) {
    print("[MASTER] clean_up() - " + object);

    // Default: don't destruct
    false
}

/// FLAG apply - Check if a flag is set for a domain
///
/// @param domain - Domain name
/// @param flag - Flag to check
/// @return true if flag is set
fn flag(domain, flag) {
    print("[MASTER] flag() - " + domain + ": " + flag);

    // Check domain flags
    false
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Check if a string starts with a prefix
fn starts_with(s, prefix) {
    len(s) >= len(prefix) && substring(s, 0, len(prefix)) == prefix
}

/// Split a string by delimiter
fn split(s, delim) {
    // Simple split implementation
    let result = [];
    let current = "";
    for ch in s {
        if ch == delim {
            result = result + [current];
            current = "";
        } else {
            current = current + ch;
        }
    }
    if len(current) > 0 {
        result = result + [current];
    }
    result
}

/// Get substring
fn substring(s, start, end) {
    let result = "";
    let i = 0;
    for ch in s {
        if i >= start && i < end {
            result = result + ch;
        }
        i = i + 1;
    }
    result
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Main function called when master is loaded
fn _init() {
    print("[MASTER] Master object loaded");
    print("[MASTER] Driver/Mudlib coordination initialized");
}

/// Command entry point - master.rhai는 명령어가 아니라 시스템 파일입니다
fn cmd(ob, line) {
    send_line(ob, "☞ master.rhai는 관리자용 시스템 파일입니다.");
    send_line(ob, "   직접 실행할 수 없습니다.");
}

fn main(ob, line) {
    cmd(ob, line);
}
