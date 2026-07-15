//! Player module for MUD engine
//!
//! This module provides the player and body structures for managing
//! game entities with stats, combat, skills, and network connectivity.

pub mod body;
#[allow(clippy::module_inception)]
pub mod player;
pub mod soul;

pub use body::{ActState, ActiveSkill, Body, MemoRecord, SendLine, SkillLevel, SkillTraining};
pub use player::{
    decode_alias_entries, encode_alias_entries, Channel, Player, ALIAS_LIST_ATTR, CFG_OPTIONS,
    STATE_ACTIVE, STATE_DOUMI, STATE_INACTIVE, STATE_NOTICE,
};
pub use soul::{Soul, SoulBodyKind, SoulBodyMember, MAX_SOUL_BODIES};

/// Configuration constants for game mechanics
pub struct GameConfig;

impl GameConfig {
    /// Maximum level difference for hunting
    pub const MAX_HUNT_LEVEL_DIFF: i64 = 100;
    /// Maximum experience
    pub const MAX_EXP: i64 = 999999999;
    /// Experience bonus multiplier
    pub const EXP_BONUS_MULTIPLIER: f64 = 1.0;
}

/// Item equipment slot levels
pub static ITEM_EQUIP_LEVELS: &[&str] = &[
    "투구",
    "왕관",
    "머리",
    "귀걸이",
    "목걸이",
    "어깨",
    "상의",
    "하의",
    "장신구",
    "갑옷",
    "허리",
    "팔찌",
    "장갑",
    "반지",
    "슬호",
    "신발",
    "무기",
    "기타",
];

/// Item level display mapping
pub fn get_item_level_display(slot: &str) -> &str {
    match slot {
        "투구" => "투    구",
        "왕관" => "   관   ",
        "머리" => "머    리",
        "귀걸이" => "귀 걸 이",
        "목걸이" => "목 걸 이",
        "어깨" => "어    깨",
        "상의" => "상    의",
        "하의" => "하    의",
        "장신구" => "장 신 구",
        "갑옷" => "갑    옷",
        "허리" => "허    리",
        "팔찌" => "팔    찌",
        "장갑" => "장    갑",
        "반지" => "반    지",
        "슬호" => "슬    호",
        "신발" => "신    발",
        "무기" => "무    기",
        "기타" => "기    타",
        _ => slot,
    }
}

/// HP status bar strings for different health levels
pub fn get_hp_bar_string(current: i64, max: i64) -> &'static str {
    if max <= 0 {
        return HP_BARS[0];
    }
    let ratio = ((current * 100) / max).clamp(0, 100) as usize;
    let index = (ratio * 10 / 100).min(10);
    HP_BARS[index]
}

static HP_BARS: &[&str] = &[
    "\x1b[37m━━━━━━━━━━\x1b[37m",         // 0%
    "\x1b[31m━\x1b[37m━━━━━━━━━\x1b[37m", // 1-10%
    "\x1b[31m━━\x1b[37m━━━━━━━━\x1b[37m", // 11-20%
    "\x1b[31m━━━\x1b[37m━━━━━━━\x1b[37m", // 21-30%
    "\x1b[33m━━━━\x1b[37m━━━━━━\x1b[37m", // 31-40%
    "\x1b[33m━━━━━\x1b[37m━━━━━\x1b[37m", // 41-50%
    "\x1b[33m━━━━━━\x1b[37m━━━━\x1b[37m", // 51-60%
    "\x1b[32m━━━━━━━\x1b[37m━━━\x1b[37m", // 61-70%
    "\x1b[32m━━━━━━━━\x1b[37m━━\x1b[37m", // 71-80%
    "\x1b[32m━━━━━━━━━\x1b[37m━\x1b[37m", // 81-90%
    "\x1b[32m━━━━━━━━━━\x1b[37m",         // 91-100%
];

/// Skill level names
pub static SKILL_LEVEL_NAMES: &[&str] = &["초급", "중급", "상급", "고급", "특급", "절정", "초절정"];

/// Skill level type names
pub static SKILL_LEVEL_TYPES: &[&str] = &[
    "초급",
    "중급",
    "상급",
    "고급",
    "특급",
    "절정",
    "초절정",
    "회복",
    "방어",
    "기타",
];

/// Get skill level from name
pub fn get_skill_level(name: &str) -> Option<u8> {
    SKILL_LEVEL_NAMES
        .iter()
        .position(|&s| s == name)
        .map(|i| i as u8 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_equip_levels() {
        assert_eq!(ITEM_EQUIP_LEVELS.len(), 18);
        assert_eq!(ITEM_EQUIP_LEVELS[0], "투구");
        assert_eq!(ITEM_EQUIP_LEVELS[16], "무기");
    }

    #[test]
    fn test_get_item_level_display() {
        assert_eq!(get_item_level_display("투구"), "투    구");
        assert_eq!(get_item_level_display("무기"), "무    기");
    }

    #[test]
    fn test_hp_bar_string() {
        assert_eq!(get_hp_bar_string(0, 100), HP_BARS[0]);
        assert_eq!(get_hp_bar_string(100, 100), HP_BARS[10]);
        assert_eq!(get_hp_bar_string(50, 100), HP_BARS[5]);
    }

    #[test]
    fn test_skill_level_names() {
        assert_eq!(SKILL_LEVEL_NAMES.len(), 7);
        assert_eq!(SKILL_LEVEL_NAMES[0], "초급");
        assert_eq!(SKILL_LEVEL_NAMES[6], "초절정");
    }

    #[test]
    fn test_get_skill_level() {
        assert_eq!(get_skill_level("초급"), Some(1));
        assert_eq!(get_skill_level("중급"), Some(2));
        assert_eq!(get_skill_level("초절정"), Some(7));
        assert_eq!(get_skill_level("없음"), None);
    }
}
