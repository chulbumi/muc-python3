//! Room module for MUD world
//!
//! This module provides room loading and management functionality.
//! Rooms are loaded from JSON files in the data/map/ directory.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Direction types for room exits. 파이썬 objs/room.sortExit·longExitStr와 동일(동서남북위아래 + 대각 4방향).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
    NorthWest, // 북서 ↖
    NorthEast, // 북동 ↗
    SouthWest, // 남서 ↙
    SouthEast, // 남동 ↘
}

impl Direction {
    /// Parse a Korean direction string (파이썬 출구/exitList 형식)
    pub fn from_korean(s: &str) -> Option<Direction> {
        match s.trim() {
            "북" => Some(Direction::North),
            "남" => Some(Direction::South),
            "동" => Some(Direction::East),
            "서" => Some(Direction::West),
            "위" => Some(Direction::Up),
            "아래" => Some(Direction::Down),
            "북서" => Some(Direction::NorthWest),
            "북동" => Some(Direction::NorthEast),
            "남서" => Some(Direction::SouthWest),
            "남동" => Some(Direction::SouthEast),
            _ => None,
        }
    }

    /// Get the Korean name for this direction
    pub fn korean_name(&self) -> &'static str {
        match self {
            Direction::North => "북",
            Direction::South => "남",
            Direction::East => "동",
            Direction::West => "서",
            Direction::Up => "위",
            Direction::Down => "아래",
            Direction::NorthWest => "북서",
            Direction::NorthEast => "북동",
            Direction::SouthWest => "남서",
            Direction::SouthEast => "남동",
        }
    }

    /// Get the full direction description
    pub fn description(&self) -> &'static str {
        match self {
            Direction::North => "북쪽",
            Direction::South => "남쪽",
            Direction::East => "동쪽",
            Direction::West => "서쪽",
            Direction::Up => "위로",
            Direction::Down => "아래로",
            Direction::NorthWest => "북서쪽",
            Direction::NorthEast => "북동쪽",
            Direction::SouthWest => "남서쪽",
            Direction::SouthEast => "남동쪽",
        }
    }
}

/// Room exit information.
/// 출구: 방향명(동서남북위아래, 대각), 숨겨진 출구(이름 끝 $), 고유 명칭(예: 초보수련장, 출구) 지원.
#[derive(Debug, Clone)]
pub struct Exit {
    /// 표시명(방향 "북" 또는 고유명 "초보수련장"). 이동/표시에 사용.
    pub display_name: String,
    /// 방향(있으면 나침반·방향이동에 사용). 고유명만 있는 출구는 None.
    pub direction: Option<Direction>,
    /// 목적지 (zone, room_id). room_id는 "1" 또는 사용자맵 "이름". 없으면 출구 없음.
    pub destination: Option<(String, String)>,
    /// 숨겨진 출구(표시 안 함, 이름으로는 이동 가능). JSON에서 이름 끝 `$`.
    pub hidden: bool,
}

impl Exit {
    /// Get the direction of this exit (if it has one)
    pub fn direction(&self) -> Option<Direction> {
        self.direction
    }

    /// Check if this exit leads somewhere
    pub fn has_destination(&self) -> bool {
        self.destination.is_some()
    }

    /// Get the destination as (zone, room_id). room_id는 "1" 또는 사용자맵 "이름".
    pub fn destination(&self, _current_zone: &str) -> Option<(String, String)> {
        self.destination.clone()
    }

    /// 이동 메시지용 문자열: 방향이 있으면 "북쪽", 없으면 "초보수련장" 등
    pub fn exit_message_name(&self) -> &str {
        self.direction
            .as_ref()
            .map(|d| d.description())
            .unwrap_or(self.display_name.as_str())
    }
}

/// Raw room data from JSON
#[derive(Debug, Clone)]
pub struct RawRoomData {
    /// Map properties (맵속성)
    pub properties: Vec<String>,
    /// Description lines (설명)
    pub description: Vec<String>,
    /// Room name (이름)
    pub name: String,
    /// Zone name (존이름)
    pub zone: String,
    /// Exits (출구)
    pub exits: Vec<String>,
    /// Mob IDs in this room (몹) — 방 입장 시에만 해당 몹 로드
    pub mob_ids: Vec<String>,
}

/// Room structure representing a game room
#[derive(Debug, Clone)]
pub struct Room {
    /// Zone name this room belongs to
    pub zone: String,
    /// Room identifier (name or index)
    pub name: String,
    /// Room display name
    pub display_name: String,
    /// Description lines
    pub description: Vec<String>,
    /// Map properties
    pub properties: Vec<String>,
    /// Parsed exits. key=display_name(방향 또는 고유명, 숨겨진은 $ 제거). value=Exit.
    pub exits: HashMap<String, Exit>,
    /// Players currently in this room
    pub players: Vec<String>,
    /// NPCs in this room
    pub npcs: Vec<String>,
    /// Mob IDs to spawn (맵정보.몹) — 입장 시에만 로드, locations 기반 대체
    pub mob_ids: Vec<String>,
    /// Items in this room
    pub items: Vec<String>,
    /// Level restriction (lower bound)
    pub level_limit: i64,
    /// Level restriction (upper bound)
    pub level_upper: i64,
    /// Whether this is a safe zone (no combat)
    pub safe_zone: bool,
    /// Whether PK is allowed in this room
    pub pk_allowed: bool,
}

impl Room {
    /// Create a new empty room
    pub fn new(zone: String, name: String) -> Self {
        Self {
            zone,
            name,
            display_name: String::new(),
            description: Vec::new(),
            properties: Vec::new(),
            exits: HashMap::new(),
            players: Vec::new(),
            npcs: Vec::new(),
            mob_ids: Vec::new(),
            items: Vec::new(),
            level_limit: 0,
            level_upper: 0,
            safe_zone: false,
            pk_allowed: true,
        }
    }

    /// Get a description of the room for display
    pub fn get_description(&self) -> String {
        let mut desc = format!("[{}] {}", self.zone, self.display_name);
        if !self.description.is_empty() {
            desc.push_str("\r\n");
            desc.push_str(&self.description.join("\r\n"));
        }
        desc
    }

    /// Get exit information as a display string
    pub fn get_exits_display(&self) -> String {
        let mut names: Vec<String> = self
            .exits
            .values()
            .filter(|e| e.has_destination() && !e.hidden)
            .map(|e| e.display_name.clone())
            .collect();
        names.sort();
        format!("출구: {}", names.join(", "))
    }

    /// 방향으로 출구 조회 (이동용). destination 있는 것만.
    pub fn get_exit(&self, direction: Direction) -> Option<&Exit> {
        self.exits
            .values()
            .find(|e| e.direction == Some(direction) && e.has_destination())
    }

    /// 고유명/방향명으로 출구 조회 (이동용). "초보수련장", "북" 등.
    pub fn get_exit_by_name(&self, name: &str) -> Option<&Exit> {
        self.exits.get(name).filter(|e| e.has_destination())
    }

    /// Check if a player can enter based on level restrictions
    pub fn can_enter(&self, player_level: i64) -> bool {
        if self.level_limit > 0 && player_level < self.level_limit {
            return false;
        }
        if self.level_upper > 0 && player_level > self.level_upper {
            return false;
        }
        true
    }

    /// Add a player to this room
    pub fn add_player(&mut self, player_name: String) {
        if !self.players.contains(&player_name) {
            self.players.push(player_name);
        }
    }

    /// Remove a player from this room
    pub fn remove_player(&mut self, player_name: &str) {
        self.players.retain(|p| p != player_name);
    }

    /// 출구숨김: 출구의 hidden 플래그 설정. true=숨김, false=드러냄. 있으면 true.
    pub fn set_exit_hidden(&mut self, name: &str, hidden: bool) -> bool {
        if let Some(e) = self.exits.get_mut(name) {
            e.hidden = hidden;
            true
        } else {
            false
        }
    }

    /// 출구제거: 출구 삭제. 있으면 true.
    pub fn remove_exit(&mut self, name: &str) -> bool {
        self.exits.remove(name).is_some()
    }

    /// 맴돌이: 출구 목적지를 (zone, room_id)로 변경. room_id는 "1" 또는 사용자맵 "이름".
    pub fn set_exit_destination(&mut self, name: &str, zone: &str, room: &str) -> bool {
        if let Some(e) = self.exits.get_mut(name) {
            e.destination = Some((zone.to_string(), room.to_string()));
            true
        } else {
            false
        }
    }
}

/// Room cache for storing loaded rooms
#[derive(Debug)]
pub struct RoomCache {
    /// Cached rooms indexed by zone:name
    rooms: HashMap<String, Arc<RwLock<Room>>>,
    /// Data directory path
    data_dir: PathBuf,
}

impl RoomCache {
    /// Create a new room cache
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            data_dir: PathBuf::from("data/map"),
        }
    }

    /// Create a new room cache with a custom data directory
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            rooms: HashMap::new(),
            data_dir: PathBuf::from(data_dir.as_ref()),
        }
    }

    /// Get a room from cache or load it
    pub fn get_room(&mut self, zone: &str, name: &str) -> Result<Arc<RwLock<Room>>, RoomError> {
        let key = format!("{}:{}", zone, name);

        // Check cache first
        if let Some(room) = self.rooms.get(&key) {
            return Ok(room.clone());
        }

        // Load the room
        let room = self.load_room(zone, name)?;
        let room_arc = Arc::new(RwLock::new(room));
        self.rooms.insert(key, room_arc.clone());
        Ok(room_arc)
    }

    /// Get a room from cache only (immutable, won't load new rooms)
    pub fn get_room_cached(&self, zone: &str, name: &str) -> Option<Arc<RwLock<Room>>> {
        let key = format!("{}:{}", zone, name);
        self.rooms.get(&key).cloned()
    }

    /// Load a room from JSON file
    fn load_room(&self, zone: &str, name: &str) -> Result<Room, RoomError> {
        // Build the file path: data/map/존이름/방이름.json
        let file_path = self.data_dir.join(zone).join(format!("{}.json", name));

        // Check if file exists
        if !file_path.exists() {
            return Err(RoomError::NotFound(format!("{}:{}", zone, name)));
        }

        // Read and parse the JSON file
        let content =
            std::fs::read_to_string(&file_path).map_err(|e| RoomError::IoError(e.to_string()))?;

        let json: JsonValue =
            serde_json::from_str(&content).map_err(|e| RoomError::ParseError(e.to_string()))?;

        // Extract the 맵정보 object
        let map_info = json
            .get("맵정보")
            .and_then(|v| v.as_object())
            .ok_or_else(|| RoomError::ParseError("맵정보 not found".to_string()))?;

        // Parse room data
        let raw = self.parse_raw_room_data(map_info)?;

        // Create and populate the room
        let mut room = Room::new(zone.to_string(), name.to_string());
        room.display_name = raw.name.clone();
        room.zone = raw.zone.clone();
        room.description = raw.description;
        room.properties = raw.properties;
        room.mob_ids = raw.mob_ids.clone();

        // Parse exits (zone needed for same-zone "동 35" 형식)
        room.exits = self.parse_exits(&raw.exits, &raw.zone)?;

        // Parse properties for special flags
        for prop in &room.properties {
            if prop.contains("안전지대") || prop.contains("safe") {
                room.safe_zone = true;
            }
            if prop.contains("PK불가") || prop.contains("nopk") {
                room.pk_allowed = false;
            }
        }

        // Check for level limits in properties
        for prop in &room.properties {
            if prop.contains("레벨제한") || prop.contains("levellimit") {
                // Try to extract level from property
                if let Some(num) = prop.chars().find_map(|c| c.to_digit(10)) {
                    room.level_limit = num as i64;
                }
            }
        }

        Ok(room)
    }

    /// Parse raw room data from JSON object
    fn parse_raw_room_data(
        &self,
        map_info: &serde_json::Map<String, JsonValue>,
    ) -> Result<RawRoomData, RoomError> {
        // Parse 맵속성 (properties)
        let properties = map_info
            .get("맵속성")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse 설명 (description)
        let description = map_info
            .get("설명")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse 이름 (name)
        let name = map_info
            .get("이름")
            .and_then(|v| v.as_str())
            .unwrap_or("이름 없는 방")
            .to_string();

        // Parse 존이름 (zone name)
        let zone = map_info
            .get("존이름")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Parse 출구 (exits)
        let exits = map_info
            .get("출구")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse 몹 (mob IDs in this room — spawn only on enter)
        let mob_ids = map_info
            .get("몹")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(RawRoomData {
            properties,
            description,
            name,
            zone,
            exits,
            mob_ids,
        })
    }

    /// Parse exit strings into HashMap<display_name, Exit>.
    /// - "북 35", "동 2": 방향+같은존  / "서 절강성:52": 방향+다른존
    /// - "초보수련장 하북성:3001", "출구 낙양성:42": 고유 명칭
    /// - "북" only: 방향만(출구 없음)  / "북$ 35": 숨겨진(표시 제외, 이름으로 이동 가능)
    fn parse_exits(
        &self,
        exit_strings: &[String],
        current_zone: &str,
    ) -> Result<HashMap<String, Exit>, RoomError> {
        let mut exits = HashMap::new();

        for exit_str in exit_strings {
            let parts: Vec<&str> = exit_str.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let raw_name = parts[0];
            let hidden = raw_name.ends_with('$');
            let display_name = raw_name.trim_end_matches('$').to_string();
            let direction = Direction::from_korean(display_name.as_str());

            let destination: Option<(String, String)> = if parts.len() == 1 {
                None
            } else if parts.len() >= 2 && parts[1].contains(':') {
                let zone_parts: Vec<&str> = parts[1].splitn(2, ':').collect();
                if zone_parts.len() == 2 {
                    let zone = zone_parts[0].trim().to_string();
                    let room_id = zone_parts[1].trim().to_string();
                    Some((zone, room_id))
                } else {
                    return Err(RoomError::ParseError(format!(
                        "Invalid zone format: {}",
                        parts[1]
                    )));
                }
            } else {
                Some((current_zone.to_string(), parts[1].to_string()))
            };

            let exit = Exit {
                display_name: display_name.clone(),
                direction,
                destination,
                hidden,
            };
            exits.insert(display_name, exit);
        }

        Ok(exits)
    }

    /// Preload all rooms in a zone
    pub fn preload_zone(&mut self, zone: &str) -> Result<usize, RoomError> {
        let zone_dir = self.data_dir.join(zone);

        eprintln!(
            "[RoomCache] Preloading zone '{}' from dir: {:?}",
            zone, zone_dir
        );

        if !zone_dir.exists() {
            eprintln!("[RoomCache] Zone directory does not exist: {:?}", zone_dir);
            return Err(RoomError::NotFound(zone.to_string()));
        }

        let entries =
            std::fs::read_dir(&zone_dir).map_err(|e| RoomError::IoError(e.to_string()))?;

        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| RoomError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| RoomError::ParseError("Invalid file name".to_string()))?;

                // Load the room (will be cached)
                eprintln!(
                    "[RoomCache] Loading room: {}:{} from {:?}",
                    zone, name, path
                );
                match self.get_room(zone, name) {
                    Ok(_) => {
                        count += 1;
                        eprintln!("[RoomCache] Successfully loaded room {}:{}", zone, name);
                    }
                    Err(e) => {
                        eprintln!("[RoomCache] Failed to load room {}:{}: {:?}", zone, name, e);
                    }
                }
            }
        }

        eprintln!(
            "[RoomCache] Preloaded {} rooms from zone '{}', total cached: {}",
            count,
            zone,
            self.rooms.len()
        );
        Ok(count)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.rooms.clear();
    }

    /// Get the number of cached rooms
    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }
}

impl Default for RoomCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when working with rooms
#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("Room not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Exit not found: {0}")]
    ExitNotFound(String),

    #[error("Level restriction: need at least level {0}")]
    LevelRestriction(i64),

    #[error("Level too high: maximum level {0}")]
    LevelTooHigh(i64),
}

/// Get a room from the global cache
///
/// This is a convenience function that uses a thread-local cache.
pub fn get_room(zone: &str, name: &str) -> Result<Arc<RwLock<Room>>, RoomError> {
    thread_local! {
        static CACHE: std::cell::RefCell<RoomCache> = std::cell::RefCell::new(RoomCache::new());
    }

    CACHE.with(|cache| cache.borrow_mut().get_room(zone, name))
}

/// Handle a player entering a room
///
/// Sends appropriate messages to the player and updates room state.
pub fn handle_player_enter(room: &mut Room, player_name: &str, mode: EnterMode) -> Vec<String> {
    let mut messages = Vec::new();

    // Add player to room
    room.add_player(player_name.to_string());

    // Send room description
    messages.push(room.get_description());
    messages.push(room.get_exits_display());

    // List other players in the room
    if !room.players.is_empty() {
        let others: Vec<&String> = room.players.iter().filter(|p| *p != &player_name).collect();
        if !others.is_empty() {
            let others_str: Vec<&str> = others.iter().map(|s| s.as_str()).collect();
            messages.push(format!("여기에는: {} 있습니다.", others_str.join(", ")));
        }
    }

    // Send entry message based on mode
    let entry_msg = match mode {
        EnterMode::Start => format!("{} 무림지존을 꿈꾸며 강호에 출두합니다.", player_name),
        EnterMode::Walk => format!("{} 왔습니다.", player_name),
        EnterMode::Flee => format!(
            "{} 신형을 비틀거리며 간신히 도망옵니다. '헉헉~~'",
            player_name
        ),
        EnterMode::Teleport => format!("{} 하늘에서 사뿐히 내려 앉습니다. '척~~~'", player_name),
        EnterMode::Summon => format!(
            "{} 알수 없는 기운에 휘말려 나타납니다. '고오오오~~~'",
            player_name
        ),
    };

    // Broadcast to room (excluding the entering player)
    // This would typically go through a message bus
    let _ = entry_msg;

    messages
}

/// Handle a player exiting a room
///
/// Returns the exit message to broadcast.
/// `exit_msg`: 방향이면 "북쪽", 고유명이면 "초보수련장" (Exit::exit_message_name() 사용)
pub fn handle_player_exit(
    _room: &Room,
    player_name: &str,
    exit_msg: &str,
    mode: ExitMode,
) -> String {
    let msg = match mode {
        ExitMode::Walk => format!("{} {}으로 갔습니다.", player_name, exit_msg),
        ExitMode::Flee => format!(
            "{} 신형을 비틀거리며 간신히 도망갑니다. '살리도~~'",
            player_name
        ),
        ExitMode::Teleport => format!(
            "{} 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'",
            player_name
        ),
        ExitMode::Summon => format!(
            "{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'",
            player_name
        ),
    };
    msg
}

/// Mode for entering a room
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterMode {
    /// Player just started (login)
    Start,
    /// Player walked in
    Walk,
    /// Player fled from combat
    Flee,
    /// Player teleported
    Teleport,
    /// Player was summoned
    Summon,
}

/// Mode for exiting a room
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitMode {
    /// Player walked out
    Walk,
    /// Player fled from combat
    Flee,
    /// Player teleported away
    Teleport,
    /// Player was summoned away
    Summon,
}

// --- 파이썬 viewMapData/longExitStr 형식 (ANSI) ---
const _ANSI_HEADER: &str = "\x1b[1;30m[\x1b[0;37m[[\x1b[1;37m[]\x1b[1m ";
const _ANSI_HEADER_END: &str = " \x1b[1;37m[]\x1b[0;37m]]\x1b[1;30m]\x1b[0;37m";
const _ANSI_GREEN: &str = "\x1b[32m";
const _ANSI_RESET: &str = "\x1b[37m";
const _EXIT_SEP: char = '\u{02D0}'; // ː

/// 파이썬 viewMapData: [[[[] 이름 []]]] + ANSI
pub fn format_room_header(display_name: &str) -> String {
    format!("{}{}{}", _ANSI_HEADER, display_name, _ANSI_HEADER_END)
}

/// 파이썬 objs/room.initExit longExitStr: 3줄 나침반 + 〔방향/고유명〕쪽으로 이동할 수 있습니다.
/// 방향 출구만 나침반(◁△▽▷)에 반영; 고유명(초보수련장, 출구)은 ː 이어붙인 str1에만. 숨겨진($) 출구는 표시 제외.
pub fn format_exits_long(room: &Room) -> String {
    let has = |d: Direction| {
        room.exits
            .values()
            .any(|e| e.direction == Some(d) && e.has_destination() && !e.hidden)
    };

    // 파이썬 sortExit 순서: 동,서,남,북,위,아래,남동,남서,북동,북서, 그 다음 고유명
    const ORDER: [Direction; 10] = [
        Direction::East,
        Direction::West,
        Direction::South,
        Direction::North,
        Direction::Up,
        Direction::Down,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthEast,
        Direction::NorthWest,
    ];
    let mut dirs = Vec::new();
    for d in &ORDER {
        if has(*d) {
            dirs.push(format!("{}{}{}", _ANSI_GREEN, d.korean_name(), _ANSI_RESET));
        }
    }
    // 고유 명칭(방향이 None인 출구) 추가. 표시된 것만, ː 로 이어붙일 대상.
    let mut named: Vec<String> = room
        .exits
        .values()
        .filter(|e| e.direction.is_none() && e.has_destination() && !e.hidden)
        .map(|e| format!("{}{}{}", _ANSI_GREEN, e.display_name.as_str(), _ANSI_RESET))
        .collect();
    named.sort();
    dirs.extend(named);
    if dirs.is_empty() {
        return "  ○  어느 쪽으로도 이동할 수 없습니다.".to_string();
    }
    let str1 = dirs.join(&_EXIT_SEP.to_string());

    // 1줄: [북서↖][북△][북동↗]
    let nw = if has(Direction::NorthWest) {
        format!("{}{}{}", _ANSI_GREEN, "↖", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let n = if has(Direction::North) {
        format!("{}{}{}", _ANSI_GREEN, "△", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let ne = if has(Direction::NorthEast) {
        format!("{}{}{}", _ANSI_GREEN, "↗", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let line1 = format!("{}{}{}\r\n", nw, n, ne);

    // 2줄: [서◁][○][동▷]  〔str1〕쪽으로 이동할 수 있습니다.
    let w = if has(Direction::West) {
        format!("{}{}{}", _ANSI_GREEN, "◁", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let e = if has(Direction::East) {
        format!("{}{}{}", _ANSI_GREEN, "▷", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let line2 = format!("{}○{}   〔{}〕쪽으로 이동할 수 있습니다.\r\n", w, e, str1);

    // 3줄: [남서↙][남▽][남동↘]
    let sw = if has(Direction::SouthWest) {
        format!("{}{}{}", _ANSI_GREEN, "↙", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let s = if has(Direction::South) {
        format!("{}{}{}", _ANSI_GREEN, "▽", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let se = if has(Direction::SouthEast) {
        format!("{}{}{}", _ANSI_GREEN, "↘", _ANSI_RESET)
    } else {
        "  ".to_string()
    };
    let line3 = format!("{}{}{}", sw, s, se);

    format!("{}{}{}", line1, line2, line3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_from_korean() {
        assert_eq!(Direction::from_korean("북"), Some(Direction::North));
        assert_eq!(Direction::from_korean("남"), Some(Direction::South));
        assert_eq!(Direction::from_korean("동"), Some(Direction::East));
        assert_eq!(Direction::from_korean("서"), Some(Direction::West));
        assert_eq!(Direction::from_korean("위"), Some(Direction::Up));
        assert_eq!(Direction::from_korean("아래"), Some(Direction::Down));
        assert_eq!(Direction::from_korean("북서"), Some(Direction::NorthWest));
        assert_eq!(Direction::from_korean("북동"), Some(Direction::NorthEast));
        assert_eq!(Direction::from_korean("남서"), Some(Direction::SouthWest));
        assert_eq!(Direction::from_korean("남동"), Some(Direction::SouthEast));
        assert_eq!(Direction::from_korean("invalid"), None);
    }

    #[test]
    fn test_direction_korean_name() {
        assert_eq!(Direction::North.korean_name(), "북");
        assert_eq!(Direction::South.korean_name(), "남");
        assert_eq!(Direction::East.korean_name(), "동");
        assert_eq!(Direction::West.korean_name(), "서");
        assert_eq!(Direction::Up.korean_name(), "위");
        assert_eq!(Direction::Down.korean_name(), "아래");
        assert_eq!(Direction::NorthWest.korean_name(), "북서");
        assert_eq!(Direction::NorthEast.korean_name(), "북동");
        assert_eq!(Direction::SouthWest.korean_name(), "남서");
        assert_eq!(Direction::SouthEast.korean_name(), "남동");
    }

    #[test]
    fn test_room_new() {
        let room = Room::new("test_zone".to_string(), "test_room".to_string());
        assert_eq!(room.zone, "test_zone");
        assert_eq!(room.name, "test_room");
        assert!(room.exits.is_empty());
        assert!(room.players.is_empty());
    }

    #[test]
    fn test_room_add_remove_player() {
        let mut room = Room::new("test_zone".to_string(), "test_room".to_string());

        room.add_player("player1".to_string());
        assert_eq!(room.players.len(), 1);
        assert!(room.players.contains(&"player1".to_string()));

        room.add_player("player2".to_string());
        assert_eq!(room.players.len(), 2);

        room.remove_player("player1");
        assert_eq!(room.players.len(), 1);
        assert!(!room.players.contains(&"player1".to_string()));
        assert!(room.players.contains(&"player2".to_string()));

        // Adding same player twice shouldn't duplicate
        room.add_player("player2".to_string());
        assert_eq!(room.players.len(), 1);
    }

    #[test]
    fn test_exit_none() {
        let exit = Exit {
            display_name: "북".into(),
            direction: Some(Direction::North),
            destination: None,
            hidden: false,
        };
        assert_eq!(exit.direction(), Some(Direction::North));
        assert!(!exit.has_destination());
        assert!(exit.destination("test_zone").is_none());
    }

    #[test]
    fn test_exit_local() {
        let exit = Exit {
            display_name: "남".into(),
            direction: Some(Direction::South),
            destination: Some(("test_zone".into(), "5".into())),
            hidden: false,
        };
        assert_eq!(exit.direction(), Some(Direction::South));
        assert!(exit.has_destination());
        assert_eq!(
            exit.destination("test_zone"),
            Some(("test_zone".to_string(), "5".to_string()))
        );
    }

    #[test]
    fn test_exit_remote() {
        let exit = Exit {
            display_name: "동".into(),
            direction: Some(Direction::East),
            destination: Some(("other_zone".into(), "10".into())),
            hidden: false,
        };
        assert_eq!(exit.direction(), Some(Direction::East));
        assert!(exit.has_destination());
        assert_eq!(
            exit.destination("test_zone"),
            Some(("other_zone".to_string(), "10".to_string()))
        );
    }

    #[test]
    fn test_room_cache_new() {
        let cache = RoomCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_room_cache_with_data_dir() {
        let cache = RoomCache::with_data_dir("/custom/path");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_room_can_enter() {
        let mut room = Room::new("test_zone".to_string(), "test_room".to_string());

        // No restrictions
        assert!(room.can_enter(1));
        assert!(room.can_enter(100));

        // Set level limit
        room.level_limit = 10;
        assert!(!room.can_enter(5));
        assert!(room.can_enter(10));
        assert!(room.can_enter(20));

        // Set upper limit
        room.level_limit = 10;
        room.level_upper = 50;
        assert!(!room.can_enter(5));
        assert!(room.can_enter(10));
        assert!(room.can_enter(50));
        assert!(!room.can_enter(60));
    }

    #[test]
    fn test_parse_exits() {
        let cache = RoomCache::new();
        let exit_strings = vec!["북".to_string(), "동 2".to_string(), "남 24".to_string()];
        let exits = cache.parse_exits(&exit_strings, "test_zone").unwrap();
        assert_eq!(exits.len(), 3);
        let north = exits.get("북").unwrap();
        assert!(north.destination.is_none());
        assert_eq!(
            exits.get("동").unwrap().destination,
            Some(("test_zone".into(), "2".into()))
        );
        assert_eq!(
            exits.get("남").unwrap().destination,
            Some(("test_zone".into(), "24".into()))
        );
    }

    #[test]
    fn test_handle_player_enter() {
        let mut room = Room::new("test_zone".to_string(), "test_room".to_string());
        room.display_name = "Test Room".to_string();
        room.description = vec!["A nice room.".to_string()];
        room.exits.insert(
            "북".into(),
            Exit {
                display_name: "북".into(),
                direction: Some(Direction::North),
                destination: Some(("test_zone".into(), "1".into())),
                hidden: false,
            },
        );

        let messages = handle_player_enter(&mut room, "TestPlayer", EnterMode::Walk);

        assert!(!messages.is_empty());
        assert!(messages[0].contains("Test Room"));
        assert!(messages[0].contains("A nice room"));
        assert!(messages[1].contains("출구"));
        assert!(room.players.contains(&"TestPlayer".to_string()));
    }

    #[test]
    fn test_handle_player_exit() {
        let room = Room::new("test_zone".to_string(), "test_room".to_string());
        let msg = handle_player_exit(&room, "TestPlayer", "북쪽", ExitMode::Walk);
        assert!(msg.contains("TestPlayer"));
        assert!(msg.contains("북쪽"));
    }
}
