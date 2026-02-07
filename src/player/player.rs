//! Player module for MUD engine
//!
//! This module provides the Player structure for managing player characters
//! with network connectivity, items, party system, and commands.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::mpsc;

use crate::object::Object;
use crate::player::body::{ActState, Body, SendLine};

/// Login state enum - tracks the player's state during the login/creation process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoginState {
    /// Player has just connected, waiting for name input
    #[default]
    GetName,
    /// Player has entered name, waiting for password
    GetPassword,
    /// Showing notice/message to player
    ShowNotice,
    /// Player is creating a new character
    CreatingCharacter,
    /// Player is fully logged in and playing
    Playing,
}

/// Player state constants
pub const STATE_INACTIVE: i32 = 0;
pub const STATE_DOUMI: i32 = 1;
pub const STATE_NOTICE: i32 = 2;
pub const STATE_ACTIVE: i32 = 3;

/// Configuration options for player
pub const CFG_OPTIONS: &[&str] = &[
    "자동습득",
    "비교거부",
    "접촉거부",
    "동행거부",
    "전음거부",
    "외침거부",
    "방파말거부",
    "간략설명",
    "엘피출력",
    "나침반제거",
    "운영자안시거부",
    "사용자안시거부",
    "입출입메세지거부",
    "타인전투출력거부",
    "자동무공시전",
    "순위거부",
    "수련모드",
    "잡담시간보기",
    "자동채널입장",
];

/// Party/Group structure for players
#[derive(Debug)]
pub struct Party {
    /// Party leader
    pub leader: Weak<Mutex<Player>>,
    /// Party members
    pub members: Vec<Weak<Mutex<Player>>>,
}

impl Party {
    pub fn new(leader: Arc<Mutex<Player>>) -> Self {
        Self {
            leader: Arc::downgrade(&leader),
            members: Vec::new(),
        }
    }

    /// Broadcast a message to all party members
    pub fn broadcast(&self, message: &str, exclude: Option<&Arc<Mutex<Player>>>) {
        // Send to leader
        if let Some(leader) = self.leader.upgrade() {
            if exclude.map_or(true, |e| Arc::as_ptr(&leader) != Arc::as_ptr(e)) {
                if let Ok(p) = leader.lock() {
                    p.send_line(message);
                }
            }
        }

        // Send to members
        for member_weak in &self.members {
            if let Some(member) = member_weak.upgrade() {
                if exclude.map_or(true, |e| Arc::as_ptr(&member) != Arc::as_ptr(e)) {
                    if let Ok(p) = member.lock() {
                        p.send_line(message);
                    }
                }
            }
        }
    }
}

/// Channel for network communication
#[derive(Debug)]
pub struct Channel {
    /// Network sender for messages to client
    pub sender: Option<mpsc::UnboundedSender<Vec<u8>>>,
    /// Weak reference to the player
    pub player: Option<Weak<Mutex<Player>>>,
    /// Connected players list
    pub players: Vec<Weak<Mutex<Player>>>,
}

impl Channel {
    pub fn new() -> Self {
        Self {
            sender: None,
            player: None,
            players: Vec::new(),
        }
    }

    /// Write data to the network connection
    pub fn write(&self, data: &[u8]) {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(data.to_vec());
        }
    }

    /// Send a line to the client
    pub fn send_line(&self, line: &str) {
        let message = format!("{}\r\n", line);
        self.write(message.as_bytes());
    }

    /// Close the connection
    pub fn lose_connection(&self) {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(Vec::new()); // Empty signal to close
        }
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}

/// Player structure - Main player character
///
/// Contains the Body (stats/combat) plus:
/// - Network connection (Channel)
/// - Party system
/// - Configuration options
/// - Alias/Shortcut system
/// - Item management
/// - Room entry/exit
/// - Login session state tracking
#[derive(Debug)]
pub struct Player {
    /// Body component (stats, combat, skills)
    pub body: Body,

    /// Player state (INACTIVE, DOUMI, NOTICE, ACTIVE)
    pub state: i32,
    /// Login state - tracks where the player is in the login/creation process
    pub login_state: LoginState,
    /// Login retry count
    pub login_retry: u32,
    /// Stored name input during login process
    pub login_name: String,
    /// Stored password input during login process
    pub login_password: String,
    /// Death step counter
    pub step_death: i32,
    /// Configuration flags
    pub configs: HashMap<String, bool>,
    /// Command alias/shortcuts
    pub alias: HashMap<String, String>,
    /// Talk history for anti-spam
    pub talk_history: Vec<String>,
    /// Previous command for '!' repeat
    pub prev_cmd: String,
    /// Command counter for spam detection
    pub cmd_cnt: u32,
    /// Idle tick counter
    pub idle: u64,
    /// Auto-move command queue
    pub auto_move_list: Vec<String>,
    /// Interactive flag
    pub interactive: i32,
    /// Fight mode flag
    pub fight_mode: bool,

    /// Party reference
    pub party: Option<Weak<Mutex<Player>>>,
    /// Party members list (when leader)
    pub party_members: Vec<Weak<Mutex<Player>>>,

    /// Followers (other players following this player)
    pub followers: Vec<Weak<Mutex<Player>>>,
    /// Who this player is following
    pub following: Option<Weak<Mutex<Player>>>,

    /// Memo/messages from other players
    pub memos: HashMap<String, String>,

    /// Network channel
    pub channel: Channel,

    /// Input handler (for async input processing)
    pub input_handler: Option<String>,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    /// Creates a new Player with default values
    pub fn new() -> Self {
        let mut player = Player {
            body: Body::new(),
            state: STATE_INACTIVE,
            login_state: LoginState::default(),
            login_retry: 0,
            login_name: String::new(),
            login_password: String::new(),
            step_death: 0,
            configs: HashMap::new(),
            alias: HashMap::new(),
            talk_history: Vec::new(),
            prev_cmd: String::new(),
            cmd_cnt: 0,
            idle: 0,
            auto_move_list: Vec::new(),
            interactive: 0,
            fight_mode: false,
            party: None,
            party_members: Vec::new(),
            followers: Vec::new(),
            following: None,
            memos: HashMap::new(),
            channel: Channel::new(),
            input_handler: None,
        };

        // Initialize with default configs disabled
        for &cfg in CFG_OPTIONS {
            player.configs.insert(cfg.to_string(), false);
        }

        player
    }

    /// Creates a Player from a Body
    pub fn from_body(body: Body) -> Self {
        let mut player = Self::new();
        player.body = body;
        player
    }

    // ==================== Session State Methods ====================

    /// Sets the current login state
    pub fn set_login_state(&mut self, state: LoginState) {
        self.login_state = state;
    }

    /// Gets the current login state
    pub fn get_login_state(&self) -> LoginState {
        self.login_state
    }

    /// Checks if the player is in a specific login state
    pub fn is_login_state(&self, state: LoginState) -> bool {
        self.login_state == state
    }

    /// Checks if the player is currently playing (logged in)
    pub fn is_playing(&self) -> bool {
        self.login_state == LoginState::Playing
    }

    /// Stores the name input during login
    pub fn set_name_input(&mut self, name: String) {
        self.login_name = name;
    }

    /// Gets the stored name input during login
    pub fn get_name_input(&self) -> &str {
        &self.login_name
    }

    /// Clears the stored name input
    pub fn clear_name_input(&mut self) {
        self.login_name.clear();
    }

    /// Stores the password input during login
    pub fn set_password_input(&mut self, password: String) {
        self.login_password = password;
    }

    /// Gets the stored password input during login
    pub fn get_password_input(&self) -> &str {
        &self.login_password
    }

    /// Clears the stored password input
    pub fn clear_password_input(&mut self) {
        self.login_password.clear();
    }

    /// Shows a notice/message to the player and transitions to ShowNotice state
    pub fn show_notice(&mut self, message: &str) {
        self.login_state = LoginState::ShowNotice;
        self.send_line(message);
    }

    /// Enters the game - transitions to Playing state
    pub fn enter_game(&mut self) -> bool {
        // Clear sensitive login data
        self.login_name.clear();
        self.login_password.clear();

        // Set state to playing
        self.login_state = LoginState::Playing;
        self.state = STATE_ACTIVE;

        // Move to starting room (this would be implemented with actual room system)
        // For now, this is a placeholder that can be called when the room system is ready
        true
    }

    /// Starts character creation process
    pub fn start_character_creation(&mut self) {
        self.login_state = LoginState::CreatingCharacter;
        self.send_line("\x1b[1;37m캐릭터를 생성합니다.");
        self.send_line("");
        self.send_line("강호에 펼쳐질 당신의 이야기를 입력해주세요.");
    }

    /// Resets the login session to initial state
    pub fn reset_login_session(&mut self) {
        self.login_state = LoginState::GetName;
        self.login_retry = 0;
        self.login_name.clear();
        self.login_password.clear();
        self.state = STATE_INACTIVE;
    }

    // ==================== Network Methods ====================

    /// Sets the network channel sender
    pub fn set_sender(&mut self, sender: mpsc::UnboundedSender<Vec<u8>>) {
        self.channel.sender = Some(sender);
    }

    /// Writes raw data to the client
    pub fn write(&self, data: &[u8]) {
        self.channel.write(data);
    }

    /// Sends a line to the player
    pub fn send_line(&self, line: &str) {
        self.channel.send_line(line);
    }

    /// Sends a prompt to the player
    pub fn prompt(&self) {
        if self.interactive != 1 {
            return;
        }

        let hp = self.body.get_hp();
        let max_hp = self.body.get_max_hp();
        let mp = self.body.get_mp();
        let max_mp = self.body.get_max_mp();

        let line = format!("\x1b[0;37;40m[ {}/{} , {}/{} ] ", hp, max_hp, mp, max_mp);
        self.write(line.as_bytes());
    }

    /// Sends prompt with optional newline
    pub fn lp_prompt(&self, mode: bool) {
        if !self.check_config("엘피출력") {
            self.prompt();
            if mode {
                self.send_line("");
            }
        }
    }

    // ==================== Room Methods ====================

    /// Enters a room
    pub fn enter_room(&mut self, room: Arc<Mutex<Object>>, move_dir: &str, mode: &str) -> bool {
        if !self.body.is_movable() && mode != "소환" && mode != "도망" {
            self.send_line("☞ 지금 이동하기에는 좋은 상황이 아니네요. ^_^");
            return false;
        }

        // Check level restrictions
        let room_guard = room.lock().unwrap();
        let level_limit = room_guard.getInt("레벨상한");
        let level_req = room_guard.getInt("레벨제한");
        let my_level = self.body.get_int("레벨");

        if level_limit > 0 && level_limit < my_level {
            self.send_line("강한 무형의 기운이 당신을 압박합니다.");
            return false;
        }

        if level_req > my_level {
            self.send_line("강한 무형의 기운이 당신을 압박합니다.");
            return false;
        }

        drop(room_guard);

        // Exit current room first
        self.exit_room(move_dir, mode);

        // Enter new room
        let mut room_guard = room.lock().unwrap();
        room_guard.append(Arc::new(Mutex::new(Object::new()))); // Placeholder

        // Send entry message
        if self.body.get_int("투명상태") != 1 {
            match mode {
                "시작" => {
                    let _msg = format!("{} 무림지존을 꿈꾸며 강호에 출두합니다.",
                        self.body.han_iga());
                    // self.channel.send_to_all_in_out(&msg, self);
                }
                "귀환" => {
                    let msg = format!("{} 하늘에서 사뿐히 내려 앉습니다. '척~~~'",
                        self.body.han_iga());
                    self.send_room(&msg);
                }
                "도망" => {
                    let msg = format!("{} 신형을 비틀거리며 간신히 도망옵니다. '헉헉~~'",
                        self.body.han_iga());
                    self.send_room(&msg);
                }
                _ => {
                    let msg = format!("{} 왔습니다.", self.body.han_iga());
                    self.send_room(&msg);
                }
            }
        }

        true
    }

    /// Exits current room
    pub fn exit_room(&mut self, move_dir: &str, mode: &str) {
        if self.body.get_int("투명상태") == 1 {
            return;
        }

        let msg = match mode {
            "귀환" => {
                format!("당신이 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'")
            }
            "소환" => {
                format!("당신이 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'")
            }
            "도망" => {
                format!("당신이 신형을 비틀거리며 간신히 도망갑니다. '살리도~~'")
            }
            _ => {
                format!("{} {}쪽으로 갔습니다.", self.body.han_iga(), move_dir)
            }
        };

        self.send_line(&msg);
        self.send_room(&msg);
    }

    /// Sends a message to everyone in the room
    pub fn send_room(&self, message: &str) {
        // This would iterate through room.objs and send to other players
        // For now, just a placeholder
        let _ = message;
    }

    /// Sends a message to room excluding self
    pub fn write_room(&self, message: &str) {
        let _ = message;
        // Placeholder implementation
    }

    // ==================== Party Methods ====================

    /// Sends a message to party members
    pub fn send_to_party(&self, message: &str, prompt: bool) {
        if let Some(party_leader) = &self.party {
            if let Some(leader) = party_leader.upgrade() {
                if let Ok(leader_player) = leader.lock() {
                    leader_player.send_line(message);
                    if prompt {
                        leader_player.lp_prompt(false);
                    }

                    // Send to party members
                    for member_weak in &leader_player.party_members {
                        if let Some(member) = member_weak.upgrade() {
                            if let Ok(member_player) = member.lock() {
                                member_player.send_line(message);
                                if prompt {
                                    member_player.lp_prompt(false);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Creates a party with this player as leader
    pub fn create_party(&mut self, self_ref: Arc<Mutex<Player>>) {
        self.party = Some(Arc::downgrade(&self_ref));
        self.party_members.clear();
    }

    /// Adds a member to the party
    pub fn add_party_member(&mut self, member: Arc<Mutex<Player>>) {
        self.party_members.push(Arc::downgrade(&member));

        // Set member's party to us
        if let Ok(mut member_player) = member.lock() {
            member_player.party = self.party.clone();
        }
    }

    /// Removes a member from the party
    pub fn remove_party_member(&mut self, member: &Arc<Mutex<Player>>) {
        let ptr = Arc::as_ptr(member);
        self.party_members.retain(|m| {
            m.upgrade()
                .map(|m| Arc::as_ptr(&m) != ptr)
                .unwrap_or(false)
        });
    }

    /// Leaves the current party
    pub fn leave_party(&mut self) {
        self.party = None;
        self.party_members.clear();
    }

    // ==================== Item Methods ====================

    /// Adds an item to inventory
    pub fn add_item(&mut self, item: Arc<Mutex<Object>>) {
        self.body.object.objs.push(item);
    }

    /// Removes an item from inventory
    pub fn del_item(&mut self, item: &Arc<Mutex<Object>>) -> bool {
        let ptr = Arc::as_ptr(item);
        let len_before = self.body.object.objs.len();
        self.body.object.objs.retain(|obj| {
            Arc::as_ptr(obj) != ptr
        });
        self.body.object.objs.len() < len_before
    }

    /// Gets item by index (1-based)
    pub fn get_item_index(&self, index: usize) -> Option<Arc<Mutex<Object>>> {
        if index == 0 || index > self.body.object.objs.len() {
            return None;
        }
        self.body.object.objs.get(index - 1).cloned()
    }

    /// Gets item by name
    pub fn get_item_name(&self, name: &str, order: usize) -> Option<Arc<Mutex<Object>>> {
        let mut count = 0;
        for obj in &self.body.object.objs {
            if let Ok(item) = obj.lock() {
                if item.getString("이름") == name {
                    count += 1;
                    if count == order {
                        return Some(obj.clone());
                    }
                }
            }
        }
        None
    }

    /// Checks if player has an item by name
    pub fn check_item_name(&self, name: &str, count: usize) -> bool {
        let mut found = 0;
        for obj in &self.body.object.objs {
            if let Ok(item) = obj.lock() {
                if item.getString("이름") == name {
                    found += 1;
                    if found >= count {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Checks if player has money
    pub fn check_money(&self, amount: i64) -> bool {
        self.body.get_int("은전") >= amount
    }

    /// Adds money
    pub fn add_money(&mut self, amount: i64) {
        let current = self.body.get_int("은전");
        self.body.set("은전", current + amount);
    }

    /// Removes money
    pub fn spend_money(&mut self, amount: i64) -> bool {
        let current = self.body.get_int("은전");
        if current >= amount {
            self.body.set("은전", current - amount);
            true
        } else {
            false
        }
    }

    // ==================== Config Methods ====================

    /// Checks if a config option is enabled
    pub fn check_config(&self, config: &str) -> bool {
        self.configs.get(config).copied().unwrap_or(false)
    }

    /// Sets a config option
    pub fn set_config(&mut self, config: &str, value: bool) {
        self.configs.insert(config.to_string(), value);
    }

    /// Toggles a config option
    pub fn toggle_config(&mut self, config: &str) -> bool {
        let current = self.check_config(config);
        self.set_config(config, !current);
        !current
    }

    // ==================== Welcome/Logout ====================

    /// Sends welcome message and prompts for name
    pub fn welcome(&mut self) {
        // Reset to GetName state
        self.login_state = LoginState::GetName;

        // Logo and name prompt would go here
        self.send_line("\x1b[1;37m무림에서 불리우는 존함을 알려주세요.");
        self.send_line("(처음 오시는 분은 \x1b[1m무명객\x1b[0;37m이라고 하세요)");
        self.write("무림존함ː".as_bytes());
    }

    /// Logs out the player
    pub fn logout(&mut self) {
        // Clear party
        self.leave_party();

        // Clear followers
        self.followers.clear();

        // Clear targets
        self.body.clear_all_targets();

        // Reset body state
        self.body.act = ActState::Stand;

        // Reset login session state
        self.reset_login_session();
    }

    // ==================== Command Processing ====================

    /// Processes a command from the player
    pub fn do_command(&mut self, command: &str) {
        // Store previous command
        if command != "!" {
            self.prev_cmd = command.to_string();
        }

        // Add to talk history for spam control
        self.talk_history.push(command.to_string());
        if self.talk_history.len() > 10 {
            self.talk_history.remove(0);
        }

        // Command processing would go here
        // For now, just acknowledge
    }

    /// Parses and executes a command
    pub fn parse_command(&mut self, line: &str) {
        let line = line.trim();

        if line.is_empty() {
            return;
        }

        // Handle command repeat
        let prev_cmd = self.prev_cmd.clone();
        let command = if line == "!" {
            prev_cmd.as_str()
        } else {
            line
        };

        self.do_command(command);
    }

    // ==================== Utility Methods ====================

    /// Gets the player's name with ANSI color
    pub fn get_name_a(&self) -> String {
        format!("\x1b[1m{}\x1b[0;37m", self.body.get_name())
    }

    /// Gets player description
    pub fn get_desc(&self) -> String {
        let _name = self.body.get_name();
        let nickname = self.body.get_string("무림별호");

        let title = if nickname.is_empty() {
            "무명객".to_string()
        } else {
            nickname
        };

        let personality = self.body.get_string("성격");
        let prefix = match personality.as_str() {
            "선인" => "[선인]",
            "기인" => "[기인이사]",
            "정파" => "[정파]",
            "사파" => "[사파]",
            "은둔칩거" => "[은둔칩거]",
            _ => "",
        };

        format!("{} 『{}』", prefix, title)
    }

    /// Dies (drops items, etc.)
    pub fn die(&mut self) {
        self.body.act = ActState::Stand; // Will be set to DEATH by caller
        self.body.unwear_all();
        self.send_line("\r\n\x1b[1;37m당신이 쓰러집니다. '쿠웅~~ 철퍼덕~~'\x1b[0;37m");

        // Drop items logic would go here

        self.body.clear_all_targets();
        self.body.clear_skills();
    }

    // ==================== Follower System ====================

    /// Adds a follower
    pub fn add_follower(&mut self, follower: Arc<Mutex<Player>>) {
        let ptr = Arc::as_ptr(&follower);
        for f in &self.followers {
            if f.upgrade().map(|f| Arc::as_ptr(&f) == ptr).unwrap_or(false) {
                return; // Already following
            }
        }

        self.followers.push(Arc::downgrade(&follower));

        if let Ok(f) = follower.lock() {
            f.send_line(&format!("당신은 {} 따라다니기 시작합니다.", self.body.han_obj()));
        }
    }

    /// Removes a follower
    pub fn remove_follower(&mut self, follower: &Arc<Mutex<Player>>) {
        let ptr = Arc::as_ptr(follower);
        self.followers.retain(|f| {
            if let Some(strong) = f.upgrade() {
                Arc::as_ptr(&strong) != ptr
            } else {
                false
            }
        });
    }

    /// Clears all followers
    pub fn clear_followers(&mut self) {
        self.followers.clear();
    }

    /// Sets this player to follow another
    pub fn follow(&mut self, _target: Arc<Mutex<Player>>) -> bool {
        if !self.body.is_movable() {
            self.send_line("지금은 따라다니기를 시작할 수 없습니다.");
            return false;
        }

        // Note: This would set following but requires external Arc management
        self.send_line("당신이 따라다니기 시작합니다.");
        true
    }

    /// Stops following
    pub fn unfollow(&mut self) {
        self.following = None;
        self.send_line("당신이 따라다니는 것을 그만둡니다.");
    }

    // ==================== Save/Load ====================

    /// Saves player data
    pub fn save(&self) -> Result<(), String> {
        // Save logic would serialize to JSON
        Ok(())
    }

    /// Loads player data
    pub fn load(&mut self, _path: &str) -> Result<(), String> {
        // Load logic would deserialize from JSON
        Ok(())
    }

    // ==================== Alias/Shortcuts ====================

    /// Sets an alias/shortcut
    pub fn set_alias(&mut self, name: &str, command: &str) {
        self.alias.insert(name.to_string(), command.to_string());
    }

    /// Gets an alias
    pub fn get_alias(&self, name: &str) -> Option<&String> {
        self.alias.get(name)
    }

    /// Removes an alias
    pub fn remove_alias(&mut self, name: &str) -> bool {
        self.alias.remove(name).is_some()
    }
}

impl SendLine for Player {
    fn send_line(&self, line: &str) {
        self.send_line(line);
    }

    fn write(&self, data: &[u8]) {
        self.write(data);
    }
}

// Allow accessing Body methods through Player
impl std::ops::Deref for Player {
    type Target = Body;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl std::ops::DerefMut for Player {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_new() {
        let player = Player::new();
        assert_eq!(player.state, STATE_INACTIVE);
        assert_eq!(player.login_state, LoginState::GetName);
        assert_eq!(player.login_retry, 0);
        assert!(player.channel.sender.is_none());
        assert!(player.followers.is_empty());
        assert!(player.party.is_none());
        assert!(player.login_name.is_empty());
        assert!(player.login_password.is_empty());
    }

    #[test]
    fn test_login_state_transitions() {
        let mut player = Player::new();

        // Initial state
        assert_eq!(player.get_login_state(), LoginState::GetName);

        // Transition to GetPassword
        player.set_login_state(LoginState::GetPassword);
        assert_eq!(player.get_login_state(), LoginState::GetPassword);

        // Transition to Playing
        player.set_login_state(LoginState::Playing);
        assert!(player.is_playing());
    }

    #[test]
    fn test_is_login_state() {
        let mut player = Player::new();

        assert!(player.is_login_state(LoginState::GetName));
        assert!(!player.is_login_state(LoginState::Playing));

        player.set_login_state(LoginState::Playing);
        assert!(!player.is_login_state(LoginState::GetName));
        assert!(player.is_login_state(LoginState::Playing));
    }

    #[test]
    fn test_name_input() {
        let mut player = Player::new();

        assert_eq!(player.get_name_input(), "");

        player.set_name_input("TestPlayer".to_string());
        assert_eq!(player.get_name_input(), "TestPlayer");

        player.clear_name_input();
        assert_eq!(player.get_name_input(), "");
    }

    #[test]
    fn test_password_input() {
        let mut player = Player::new();

        assert_eq!(player.get_password_input(), "");

        player.set_password_input("secret123".to_string());
        assert_eq!(player.get_password_input(), "secret123");

        player.clear_password_input();
        assert_eq!(player.get_password_input(), "");
    }

    #[test]
    fn test_show_notice() {
        let mut player = Player::new();

        player.show_notice("Test notice message");

        assert_eq!(player.get_login_state(), LoginState::ShowNotice);
    }

    #[test]
    fn test_enter_game() {
        let mut player = Player::new();

        // Set some login data first
        player.set_name_input("TestPlayer".to_string());
        player.set_password_input("password".to_string());

        let result = player.enter_game();

        assert!(result);
        assert_eq!(player.get_login_state(), LoginState::Playing);
        assert_eq!(player.state, STATE_ACTIVE);
        assert_eq!(player.get_name_input(), ""); // Should be cleared
        assert_eq!(player.get_password_input(), ""); // Should be cleared
    }

    #[test]
    fn test_start_character_creation() {
        let mut player = Player::new();

        player.start_character_creation();

        assert_eq!(player.get_login_state(), LoginState::CreatingCharacter);
    }

    #[test]
    fn test_reset_login_session() {
        let mut player = Player::new();

        // Set some state
        player.set_login_state(LoginState::Playing);
        player.set_name_input("TestPlayer".to_string());
        player.set_password_input("password".to_string());
        player.login_retry = 3;
        player.state = STATE_ACTIVE;

        // Reset
        player.reset_login_session();

        assert_eq!(player.get_login_state(), LoginState::GetName);
        assert_eq!(player.login_retry, 0);
        assert_eq!(player.get_name_input(), "");
        assert_eq!(player.get_password_input(), "");
        assert_eq!(player.state, STATE_INACTIVE);
    }

    #[test]
    fn test_welcome_sets_login_state() {
        let mut player = Player::new();

        // Set a different state first
        player.set_login_state(LoginState::Playing);

        // Call welcome
        player.welcome();

        // Should be back to GetName
        assert_eq!(player.get_login_state(), LoginState::GetName);
    }

    #[test]
    fn test_logout_resets_login_session() {
        let mut player = Player::new();

        // Set up player as if logged in
        player.set_login_state(LoginState::Playing);
        player.set_name_input("TestPlayer".to_string());
        player.state = STATE_ACTIVE;

        // Logout
        player.logout();

        // Should be reset
        assert_eq!(player.get_login_state(), LoginState::GetName);
        assert_eq!(player.get_name_input(), "");
        assert_eq!(player.state, STATE_INACTIVE);
    }

    #[test]
    fn test_check_config_default() {
        let player = Player::new();
        // All configs should be false by default
        for &cfg in CFG_OPTIONS {
            assert!(!player.check_config(cfg));
        }
    }

    #[test]
    fn test_set_config() {
        let mut player = Player::new();
        assert!(!player.check_config("간략설명"));

        player.set_config("간략설명", true);
        assert!(player.check_config("간략설명"));
    }

    #[test]
    fn test_toggle_config() {
        let mut player = Player::new();
        assert!(!player.check_config("자동습득"));

        let result = player.toggle_config("자동습득");
        assert!(result);
        assert!(player.check_config("자동습득"));

        let result = player.toggle_config("자동습득");
        assert!(!result);
        assert!(!player.check_config("자동습득"));
    }

    #[test]
    fn test_get_name_a() {
        let mut player = Player::new();
        player.body.set("이름", "테스트");

        assert_eq!(player.get_name_a(), "\x1b[1m테스트\x1b[0;37m");
    }

    #[test]
    fn test_get_desc() {
        let mut player = Player::new();
        player.body.set("이름", "용사");
        player.body.set("무림별호", "전사");

        let desc = player.get_desc();
        assert!(desc.contains("전사"));
    }

    #[test]
    fn test_check_money() {
        let mut player = Player::new();
        player.body.init_body(); // 100000 은전

        assert!(player.check_money(50000));
        assert!(player.check_money(100000));
        assert!(!player.check_money(100001));
    }

    #[test]
    fn test_add_money() {
        let mut player = Player::new();
        player.body.init_body();

        player.add_money(50000);
        assert_eq!(player.body.get_int("은전"), 150000);
    }

    #[test]
    fn test_spend_money() {
        let mut player = Player::new();
        player.body.init_body();

        assert!(player.spend_money(50000));
        assert_eq!(player.body.get_int("은전"), 50000);

        assert!(!player.spend_money(100000));
        assert_eq!(player.body.get_int("은전"), 50000); // Unchanged
    }

    #[test]
    fn test_alias() {
        let mut player = Player::new();

        player.set_alias("l", "바라보기");
        assert_eq!(player.get_alias("l"), Some(&"바라보기".to_string()));

        player.remove_alias("l");
        assert_eq!(player.get_alias("l"), None);
    }

    #[test]
    fn test_add_item() {
        let mut player = Player::new();

        let item = Arc::new(Mutex::new(Object::new()));
        item.lock().unwrap().set("이름", "검");

        player.add_item(item);
        assert_eq!(player.body.object.objs.len(), 1);
    }

    #[test]
    fn test_get_item_index() {
        let mut player = Player::new();

        let item1 = Arc::new(Mutex::new(Object::new()));
        item1.lock().unwrap().set("이름", "검");
        let item2 = Arc::new(Mutex::new(Object::new()));
        item2.lock().unwrap().set("이름", "방패");

        player.add_item(item1);
        player.add_item(item2);

        assert!(player.get_item_index(1).is_some());
        assert!(player.get_item_index(2).is_some());
        assert!(player.get_item_index(3).is_none());
        assert!(player.get_item_index(0).is_none());
    }

    #[test]
    fn test_get_item_name() {
        let mut player = Player::new();

        let item1 = Arc::new(Mutex::new(Object::new()));
        item1.lock().unwrap().set("이름", "검");
        let item2 = Arc::new(Mutex::new(Object::new()));
        item2.lock().unwrap().set("이름", "검");

        player.add_item(item1);
        player.add_item(item2);

        assert!(player.get_item_name("검", 1).is_some());
        assert!(player.get_item_name("검", 2).is_some());
        assert!(player.get_item_name("검", 3).is_none());
    }

    #[test]
    fn test_check_item_name() {
        let mut player = Player::new();

        let item1 = Arc::new(Mutex::new(Object::new()));
        item1.lock().unwrap().set("이름", "포션");
        let item2 = Arc::new(Mutex::new(Object::new()));
        item2.lock().unwrap().set("이름", "포션");

        player.add_item(item1);
        player.add_item(item2);

        assert!(player.check_item_name("포션", 1));
        assert!(player.check_item_name("포션", 2));
        assert!(!player.check_item_name("포션", 3));
        assert!(!player.check_item_name("없는아이템", 1));
    }

    #[test]
    fn test_del_item() {
        let mut player = Player::new();

        let item = Arc::new(Mutex::new(Object::new()));
        item.lock().unwrap().set("이름", "검");

        player.add_item(item.clone());
        assert_eq!(player.body.object.objs.len(), 1);

        assert!(player.del_item(&item));
        assert_eq!(player.body.object.objs.len(), 0);

        assert!(!player.del_item(&item)); // Already removed
    }

    #[test]
    fn test_prev_cmd() {
        let mut player = Player::new();

        player.do_command("북");
        assert_eq!(player.prev_cmd, "북");

        player.do_command("!");
        // "!" doesn't update prev_cmd, it stays as "북"
        assert_eq!(player.prev_cmd, "북");
    }

    #[test]
    fn test_talk_history() {
        let mut player = Player::new();

        for i in 0..15 {
            player.do_command(&format!("cmd{}", i));
        }

        // Should only keep last 10
        assert_eq!(player.talk_history.len(), 10);
        assert_eq!(player.talk_history[0], "cmd5");
        assert_eq!(player.talk_history[9], "cmd14");
    }

    #[test]
    fn test_create_party() {
        let mut player = Player::new();
        let self_ref = Arc::new(Mutex::new(Player::new()));

        player.create_party(self_ref);
        assert!(player.party.is_some());
        assert!(player.party_members.is_empty());
    }

    #[test]
    fn test_logout_clears_state() {
        let mut player = Player::new();
        player.body.act = ActState::Fight;
        player.body.targets.push(Weak::new()); // Add a placeholder target

        player.logout();

        assert_eq!(player.body.act, ActState::Stand);
        assert!(player.body.targets.is_empty());
        assert!(player.party.is_none());
    }

    #[test]
    fn test_dereference_to_body() {
        let mut player = Player::new();
        player.body.init_body();

        // Can access Body methods through Player
        assert_eq!(player.get_int("레벨"), 1);
        assert_eq!(player.get_hp(), 450);
    }

    #[test]
    fn test_unfollow() {
        let mut player = Player::new();

        let target = Arc::new(Mutex::new(Player::new()));
        player.following = Some(Arc::downgrade(&target));

        player.unfollow();

        assert!(player.following.is_none());
    }

    #[test]
    fn test_clear_followers() {
        let mut player = Player::new();

        let follower1 = Arc::new(Mutex::new(Player::new()));
        let follower2 = Arc::new(Mutex::new(Player::new()));

        player.followers.push(Arc::downgrade(&follower1));
        player.followers.push(Arc::downgrade(&follower2));

        assert_eq!(player.followers.len(), 2);

        player.clear_followers();

        assert_eq!(player.followers.len(), 0);
    }
}
