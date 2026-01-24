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
    NorthWest,  // 북서 ↖
    NorthEast,  // 북동 ↗
    SouthWest,  // 남서 ↙
    SouthEast,  // 남동 ↘
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

/// Room exit information
#[derive(Debug, Clone)]
pub enum Exit {
    /// No exit in this direction (just a direction name)
    None(Direction),
    /// Exit to a room in the same zone
    Local { direction: Direction, room_index: i64 },
    /// Exit to a room in another zone
    Remote { direction: Direction, zone: String, room_index: i64 },
}

impl Exit {
    /// Get the direction of this exit
    pub fn direction(&self) -> Direction {
        match self {
            Exit::None(dir) => *dir,
            Exit::Local { direction, .. } => *direction,
            Exit::Remote { direction, .. } => *direction,
        }
    }

    /// Check if this exit leads somewhere
    pub fn has_destination(&self) -> bool {
        !matches!(self, Exit::None(_))
    }

    /// Get the destination as a tuple of (zone, room_index)
    /// Returns None if exit doesn't lead anywhere
    pub fn destination(&self, current_zone: &str) -> Option<(String, i64)> {
        match self {
            Exit::None(_) => None,
            Exit::Local { room_index, .. } => Some((current_zone.to_string(), *room_index)),
            Exit::Remote { zone, room_index, .. } => Some((zone.clone(), *room_index)),
        }
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
    /// Parsed exits
    pub exits: HashMap<Direction, Exit>,
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
        let mut exits: Vec<&str> = self.exits
            .values()
            .filter(|e| e.has_destination())
            .map(|e| e.direction().description())
            .collect();
        exits.sort();
        format!("출구: {}", exits.join(", "))
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

    /// Get the exit in a specific direction
    pub fn get_exit(&self, direction: Direction) -> Option<&Exit> {
        self.exits.get(&direction)
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
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| RoomError::IoError(e.to_string()))?;

        let json: JsonValue = serde_json::from_str(&content)
            .map_err(|e| RoomError::ParseError(e.to_string()))?;

        // Extract the 맵정보 object
        let map_info = json.get("맵정보")
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

        // Parse exits
        room.exits = self.parse_exits(&raw.exits)?;

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
    fn parse_raw_room_data(&self, map_info: &serde_json::Map<String, JsonValue>) -> Result<RawRoomData, RoomError> {
        // Parse 맵속성 (properties)
        let properties = map_info.get("맵속성")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse 설명 (description)
        let description = map_info.get("설명")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse 이름 (name)
        let name = map_info.get("이름")
            .and_then(|v| v.as_str())
            .unwrap_or("이름 없는 방")
            .to_string();

        // Parse 존이름 (zone name)
        let zone = map_info.get("존이름")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Parse 출구 (exits)
        let exits = map_info.get("출구")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse 몹 (mob IDs in this room — spawn only on enter)
        let mob_ids = map_info.get("몹")
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

    /// Parse exit strings into Exit enums
    fn parse_exits(&self, exit_strings: &[String]) -> Result<HashMap<Direction, Exit>, RoomError> {
        let mut exits = HashMap::new();

        for exit_str in exit_strings {
            let parts: Vec<&str> = exit_str.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            // Parse direction
            let direction = Direction::from_korean(parts[0])
                .ok_or_else(|| RoomError::ParseError(format!("Invalid direction: {}", parts[0])))?;

            // Create exit based on format
            // "존:방" 형식(다른 존 연결)을 먼저 검사. parts.len()==2일 때 "남 6"과 "서 섬서성:52" 구분.
            let exit = if parts.len() == 1 {
                // 방향만, 출구 없음
                Exit::None(direction)
            } else if parts.len() >= 2 && parts[1].contains(':') {
                // "서 섬서성:52", "동 낙양성:1072" — 다른 존으로 연결
                let zone_parts: Vec<&str> = parts[1].splitn(2, ':').collect();
                if zone_parts.len() == 2 {
                    let zone = zone_parts[0].trim().to_string();
                    let room_index = zone_parts[1].trim().parse::<i64>()
                        .map_err(|_| RoomError::ParseError(format!("Invalid room index: {}", parts[1])))?;
                    Exit::Remote { direction, zone, room_index }
                } else {
                    return Err(RoomError::ParseError(format!("Invalid zone format: {}", parts[1])));
                }
            } else if parts.len() == 2 {
                // "남 6", "동 1071" — 같은 존 내 방
                let room_index = parts[1].parse::<i64>()
                    .map_err(|_| RoomError::ParseError(format!("Invalid room index: {}", parts[1])))?;
                Exit::Local { direction, room_index }
            } else {
                // 기타: 첫 번째 토큰을 방 번호로
                let room_index = parts[1].parse::<i64>()
                    .map_err(|_| RoomError::ParseError(format!("Invalid exit format: {}", exit_str)))?;
                Exit::Local { direction, room_index }
            };

            exits.insert(direction, exit);
        }

        Ok(exits)
    }

    /// Preload all rooms in a zone
    pub fn preload_zone(&mut self, zone: &str) -> Result<usize, RoomError> {
        let zone_dir = self.data_dir.join(zone);

        eprintln!("[RoomCache] Preloading zone '{}' from dir: {:?}", zone, zone_dir);

        if !zone_dir.exists() {
            eprintln!("[RoomCache] Zone directory does not exist: {:?}", zone_dir);
            return Err(RoomError::NotFound(zone.to_string()));
        }

        let entries = std::fs::read_dir(&zone_dir)
            .map_err(|e| RoomError::IoError(e.to_string()))?;

        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| RoomError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| RoomError::ParseError("Invalid file name".to_string()))?;

                // Load the room (will be cached)
                eprintln!("[RoomCache] Loading room: {}:{} from {:?}", zone, name, path);
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

        eprintln!("[RoomCache] Preloaded {} rooms from zone '{}', total cached: {}",
            count, zone, self.rooms.len());
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

    CACHE.with(|cache| {
        cache.borrow_mut().get_room(zone, name)
    })
}

/// Handle a player entering a room
///
/// Sends appropriate messages to the player and updates room state.
pub fn handle_player_enter(
    room: &mut Room,
    player_name: &str,
    mode: EnterMode,
) -> Vec<String> {
    let mut messages = Vec::new();

    // Add player to room
    room.add_player(player_name.to_string());

    // Send room description
    messages.push(room.get_description());
    messages.push(room.get_exits_display());

    // List other players in the room
    if !room.players.is_empty() {
        let others: Vec<&String> = room.players.iter()
            .filter(|p| *p != &player_name)
            .collect();
        if !others.is_empty() {
            let others_str: Vec<&str> = others.iter().map(|s| s.as_str()).collect();
            messages.push(format!("여기에는: {} 있습니다.", others_str.join(", ")));
        }
    }

    // Send entry message based on mode
    let entry_msg = match mode {
        EnterMode::Start => format!("{} 무림지존을 꿈꾸며 강호에 출두합니다.", player_name),
        EnterMode::Walk => format!("{} 왔습니다.", player_name),
        EnterMode::Flee => format!("{} 신형을 비틀거리며 간신히 도망옵니다. '헉헉~~'", player_name),
        EnterMode::Teleport => format!("{} 하늘에서 사뿐히 내려 앉습니다. '척~~~'", player_name),
        EnterMode::Summon => format!("{} 알수 없는 기운에 휘말려 나타납니다. '고오오오~~~'", player_name),
    };

    // Broadcast to room (excluding the entering player)
    // This would typically go through a message bus
    let _ = entry_msg;

    messages
}

/// Handle a player exiting a room
///
/// Returns the exit message to broadcast.
pub fn handle_player_exit(
    _room: &Room,
    player_name: &str,
    direction: Direction,
    mode: ExitMode,
) -> String {
    // Remove player from room
    // Note: We can't modify room here due to borrow, caller should do that

    let msg = match mode {
        ExitMode::Walk => format!("{} {}쪽으로 갔습니다.", player_name, direction.korean_name()),
        ExitMode::Flee => format!("{} 신형을 비틀거리며 간신히 도망갑니다. '살리도~~'", player_name),
        ExitMode::Teleport => format!("{} 경공술을 펼치며 하늘로 치솟아 오릅니다. '무영지신!!!'", player_name),
        ExitMode::Summon => format!("{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'", player_name),
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

/// 파이썬 objs/room.initExit longExitStr: 3줄 나침반 + 〔방향〕쪽으로 이동할 수 있습니다.
/// 1줄: 북서↖ 북△ 북동↗ (Rust는 북만), 2줄: 서◁ ○ 동▷ + 문구, 3줄: 남서↙ 남▽ 남동↘ (Rust는 남만).
/// ◁△▽▷는 해당 출구가 있을 때만 표시, 없으면 공백. 방향은 동서남북위아래 순, ː로 이어붙임.
pub fn format_exits_long(room: &Room) -> String {
    let has = |d: Direction| room.exits.get(&d).map_or(false, |e| e.has_destination());

    // 파이썬 sortExit 순서: 동,서,남,북,위,아래,남동,남서,북동,북서
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
        let exit = Exit::None(Direction::North);
        assert_eq!(exit.direction(), Direction::North);
        assert!(!exit.has_destination());
        assert!(exit.destination("test_zone").is_none());
    }

    #[test]
    fn test_exit_local() {
        let exit = Exit::Local { direction: Direction::South, room_index: 5 };
        assert_eq!(exit.direction(), Direction::South);
        assert!(exit.has_destination());
        assert_eq!(exit.destination("test_zone"), Some(("test_zone".to_string(), 5)));
    }

    #[test]
    fn test_exit_remote() {
        let exit = Exit::Remote {
            direction: Direction::East,
            zone: "other_zone".to_string(),
            room_index: 10
        };
        assert_eq!(exit.direction(), Direction::East);
        assert!(exit.has_destination());
        assert_eq!(exit.destination("test_zone"), Some(("other_zone".to_string(), 10)));
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
        assert_eq!(cache.data_dir, PathBuf::from("/custom/path"));
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

        // Test exit formats: "남", "동 6", "서 절강성:7"
        let exit_strings = vec![
            "북".to_string(),
            "동 2".to_string(),
            "남 24".to_string(),
        ];

        let exits = cache.parse_exits(&exit_strings).unwrap();

        assert_eq!(exits.len(), 3);
        assert!(matches!(exits.get(&Direction::North), Some(Exit::None(_))));
        assert!(matches!(exits.get(&Direction::East), Some(Exit::Local { room_index: 2, .. })));
        assert!(matches!(exits.get(&Direction::South), Some(Exit::Local { room_index: 24, .. })));
    }

    #[test]
    fn test_handle_player_enter() {
        let mut room = Room::new("test_zone".to_string(), "test_room".to_string());
        room.display_name = "Test Room".to_string();
        room.description = vec!["A nice room.".to_string()];
        room.exits.insert(Direction::North, Exit::Local { direction: Direction::North, room_index: 1 });

        let messages = handle_player_enter(&mut room, "TestPlayer", EnterMode::Walk);

        assert!(!messages.is_empty());
        assert!(messages[0].contains("Test Room"));
        assert!(messages[0].contains("A nice room"));
        assert!(messages[1].contains("출구"));
        assert!(room.players.contains(&"TestPlayer".to_string()));
    }

    #[test]
    fn test_handle_player_exit() {
        let mut room = Room::new("test_zone".to_string(), "test_room".to_string());
        room.add_player("TestPlayer".to_string());

        let msg = handle_player_exit(&room, "TestPlayer", Direction::North, ExitMode::Walk);

        assert!(msg.contains("TestPlayer"));
        assert!(msg.contains("북"));
    }
}
