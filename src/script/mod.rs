//! Rhai scripting engine for MUD server
//!
//! Provides hot-reloadable scripting support using Rhai.
//! Scripts are stored in cmds/ directory and automatically reloaded on change.

#![allow(clippy::type_complexity)]
#![allow(static_mut_refs)]

mod admin_combat;
mod anger;
mod box_commands;
mod cast;
pub(crate) use cast::skill_up_python;
pub(crate) mod combat_commands;
mod drop_item;
mod inventory_compat;
mod movement;
mod party;
mod return_home;
mod search_body;

pub(crate) const TEACH_SKILL_REQUEST: &str = "_teach_skill_request";
pub(crate) const REMOVE_SKILL_REQUEST: &str = "_remove_skill_request";
pub(crate) const AUTO_MOVE_REQUEST: &str = "_auto_move_request";
pub(crate) const GUILD_KICK_REQUEST: &str = "_guild_kick_request";
pub(crate) const SAVE_ALL_REQUEST: &str = "_save_all_request";
pub(crate) const SET_SKILL_REQUEST: &str = "_set_skill_request";
pub(crate) const GUILD_TRANSFER_REQUEST: &str = "_guild_transfer_request";
pub(crate) const GUILD_POSITION_REQUEST: &str = "_guild_position_request";
pub(crate) const GUILD_NICKNAME_REQUEST: &str = "_guild_nickname_request";
pub(crate) const GUILD_ACCEPT_REQUEST: &str = "_guild_accept_request";
pub(crate) const GUILD_APPLY_REQUEST: &str = "_guild_apply_request";
pub(crate) const GUILD_RESET_REQUEST: &str = "_guild_reset_request";
pub(crate) const ADMIN_SET_PLAYER_VALUE_REQUEST: &str = "_admin_set_player_value_request";
pub(crate) const SET_PLAYER_ATTR_REQUEST: &str = "_set_player_attr_request";
pub(crate) const CHANGE_PLAYER_REQUEST: &str = "_change_player_request";
pub(crate) const SUMMON_PLAYER_REQUEST: &str = "_summon_player_request";
pub(crate) const FORCE_COMMAND_REQUEST: &str = "_force_command_request";

pub(crate) fn take_summon_player_request(body: &mut Body) -> Vec<(String, String, String)> {
    body.temp_mut()
        .remove(SUMMON_PLAYER_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub(crate) fn take_force_command_request(body: &mut Body) -> Vec<(String, String)> {
    body.temp_mut()
        .remove(FORCE_COMMAND_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub(crate) fn take_guild_accept_request(body: &mut Body) -> Option<(String, String)> {
    body.temp_mut()
        .remove(GUILD_ACCEPT_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_guild_apply_request(body: &mut Body) -> Option<(String, String)> {
    body.temp_mut()
        .remove(GUILD_APPLY_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_guild_reset_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(GUILD_RESET_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_admin_set_player_value_request(
    body: &mut Body,
) -> Option<(String, String, serde_json::Value)> {
    body.temp_mut()
        .remove(ADMIN_SET_PLAYER_VALUE_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_change_player_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(CHANGE_PLAYER_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_teach_skill_request(body: &mut Body) -> Option<(String, String)> {
    body.temp_mut()
        .remove(TEACH_SKILL_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_remove_skill_request(body: &mut Body) -> Option<(String, String)> {
    body.temp_mut()
        .remove(REMOVE_SKILL_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_auto_move_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(AUTO_MOVE_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_guild_kick_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(GUILD_KICK_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_save_all_request(body: &mut Body) -> bool {
    body.temp_mut().remove(SAVE_ALL_REQUEST).is_some()
}

pub(crate) fn take_set_skill_request(body: &mut Body) -> Option<(String, String, i64)> {
    body.temp_mut()
        .remove(SET_SKILL_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_guild_transfer_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(GUILD_TRANSFER_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_guild_position_request(body: &mut Body) -> Option<(String, String)> {
    body.temp_mut()
        .remove(GUILD_POSITION_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_guild_nickname_request(body: &mut Body) -> Option<(String, String)> {
    body.temp_mut()
        .remove(GUILD_NICKNAME_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_set_player_attr_request(body: &mut Body) -> Option<(String, String, i64)> {
    body.temp_mut()
        .remove(SET_PLAYER_ATTR_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) use box_commands::{
    build_box_observer_snapshot, clear_precomputed_box_context, set_precomputed_box_context,
    take_box_deliveries, BoxDelivery,
};
pub(crate) use cast::{clear_cast_room_players, set_cast_room_players, CastRoomPlayerRef};
pub(crate) use movement::{immediate_exit_destinations, python_map_explore};
pub(crate) use party::{
    build_party_nonplayer_snapshot, build_party_person_snapshot, installed_box_party_snapshots,
    missing_party_person, set_precomputed_party_context, take_party_requests, PartyDelivery,
    PARTY_DISCONNECT_REQUEST,
};

use encoding::{EncoderTrap, Encoding};
use rand::Rng;
use rhai::{Dynamic, Engine, Scope};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

static CHAT_HISTORY: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub(crate) fn hp_status_script(current: i64, maximum: i64) -> String {
    if maximum <= 0 {
        return "(은/는) 아주 활력이 넘칩니다.".into();
    }
    const SCRIPTS: [&str; 15] = [
        "(은/는) 아주 활력이 넘칩니다.",
        "의 팔다리에 약간의 다친 흔적이 보입니다.",
        "의 가슴에서 피가 번지기 시작합니다.",
        "(은/는) 피를 약간씩 흘리고 있습니다.",
        "의 이곳 저곳에 깊은 상처를 입었습니다.",
        "(이/가) 몸을 조금 비틀거리고 있습니다.",
        "(이/가) 몸을 가누기 어려울 정도로 비틀거리고 있습니다.",
        "(이/가) 신음소리를 내며 쓰러질것 같이 휘청 거립니다.",
        "(이/가) 정신을 잃을정도로 혼미한 상태에 이르렀습니다.",
        "(이/가) 피가 분수처럼 뿜어져 나오며 숨을 헐떡 거립니다.",
        "(은/는) 숨이 멈출듯 헐떡 거리며 의식이 몽롱합니다.",
        "(이/가) 몸을 움직일수 없이 휘청거립니다.",
        "(은/는) 의식을 잃어가고 죽음의 문턱을 넘나듭니다.",
        "(은/는) 가느다란 숨만 몰아쉬고 죽음의 문과 가깝게 있습니다.",
        "에게 저승사자가 손짓하고 있습니다.",
    ];
    let index = 14 - 14 * current.clamp(0, maximum) / maximum;
    SCRIPTS[index as usize].into()
}

pub(crate) fn mp_status_script(mp: i64) -> String {
    const SCRIPTS: [&str; 14] = [
        "(은/는) [1;32m소주천[0;37;40m을 하고 있습니다.",
        "(은/는) [1;32m대주천[0;37;40m을 하고 있습니다.",
        "(은/는) [1m안광[0;37;40m이 [1;33m형형[0;37;40m하고 [1;32m태양혈[0;37;40m이 튀어나와 있습니다.",
        "(은/는) 무인의 꿈인 [1;33m생[37mㆍ[33m사[37m ㆍ[33m현[37mㆍ[33m관 [32m임독양맥[0;37;40m이 타동되었습니다.",
        "(은/는) 상단전 [33m중단전[40m[37m [1;33m하단전[0;37;40m의 기운을 모았습니다.",
        "(은/는) [1m오행의 기운[0;37;40m을 조절하는 [1;32m오기조원[0;37;40m의 경지에 도달 했습니다.",
        "(은/는) 무공이 드러나지 않는 [1;32m노화순청[0;37;40m의 경지에도달 했습니다.",
        "의 귀밑머리가 희어지고 안광을 갈무리하는 [1;32m반박귀진[0;37;40m에 도달했습니다.",
        "(은/는) [1m운기조식의 절정[0;37;40m인 [1;32m등복조극[0;37;40m의 경지에 도달했습니다.",
        "(은/는) [1m여섯호흡이 근본[0;37;40m으로 돌아가는 [1;32m육식 귀전[0;37;40m을 이루었습니다.",
        "(은/는) 늙음을 돌이켜 아이로 돌아가는 [1;32m반노환등[0;37;40m의 경지 입니다.",
        "(은/는) [1;36m음신[40m[37m과 [31m양신[0;37;40m을 만들어내는 [1;32m출신입화지경[0;37;40m을 이루었습니다.",
        "(은/는) 인간의 육신으로 [1m신선의 경지[0;37;40m에 오르는 [1;32m우화등선[0;37;40m을 이루었습니다.",
        "(은/는) 사기로 내공을 올렸습니다.",
    ];
    let index = match mp {
        0..=100 => 0,
        101..=250 => 1,
        251..=400 => 2,
        401..=600 => 3,
        601..=800 => 4,
        801..=1050 => 5,
        1051..=1300 => 6,
        1301..=1550 => 7,
        1551..=1850 => 8,
        1851..=2150 => 9,
        2151..=2550 => 10,
        2551..=3050 => 11,
        _ => 12,
    };
    SCRIPTS[index].into()
}

pub(crate) fn record_chat_history(message: &str) {
    record_chat_history_limit(message, 24);
}

fn record_chat_history_limit(message: &str, limit: usize) {
    let history = CHAT_HISTORY.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut history) = history.lock() {
        history.push(message.to_string());
        if history.len() > limit {
            let excess = history.len() - limit;
            history.drain(0..excess);
        }
    }
}

fn chat_history_snapshot() -> Vec<String> {
    CHAT_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map(|history| history.clone())
        .unwrap_or_default()
}
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::command::parser::CommandParser;
use crate::command::CommandResult;
use crate::data::SharedGlobalData;
use crate::network::Broadcaster;
use crate::object::{Object, Value};
use crate::player::{
    decode_alias_entries, encode_alias_entries, Body, MemoRecord, ALIAS_LIST_ATTR,
};
use crate::player::{get_hp_bar_string, get_item_level_display, ITEM_EQUIP_LEVELS};
use crate::scheduler::CallOutScheduler;
use crate::world::guild::{
    guild_attr_keys, guild_get, guild_has, guild_list, guild_remove, guild_save, guild_set,
};
use crate::world::rank::{rank_clear, rank_get_all, rank_get_num, rank_read, rank_write};
use crate::world::{
    format_exits_long, format_room_header, get_world_state, Direction, MobInstance, PlayerPosition,
    RawMobData, RoomObjectRef, WorldState,
};
use std::time::Duration;

fn strip_ansi_like_python(value: &str) -> String {
    let mut found_escape = false;
    let mut result = String::new();
    for character in value.chars() {
        match character {
            '\u{009b}' => continue,
            '\u{0008}' => {
                result.pop();
                continue;
            }
            '\u{001b}' => {
                found_escape = true;
                continue;
            }
            'm' if found_escape => {
                found_escape = false;
                continue;
            }
            _ if found_escape => continue,
            _ => result.push(character),
        }
    }
    result
}

fn fill_space_euc_kr(width: i64, value: &str) -> String {
    let visible = strip_ansi_like_python(value);
    let encoded_len = encoding::all::WINDOWS_949
        .encode(&visible, EncoderTrap::Replace)
        .map_or(visible.len(), |encoded| encoded.len()) as i64;
    if encoded_len >= width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat((width - encoded_len) as usize))
    }
}

fn fill_space_front_euc_kr(width: i64, value: &str) -> String {
    let visible = strip_ansi_like_python(value);
    let encoded_len = encoding::all::WINDOWS_949
        .encode(&visible, EncoderTrap::Replace)
        .map_or(visible.len(), |encoded| encoded.len()) as i64;
    if encoded_len >= width {
        value.to_string()
    } else {
        format!("{}{}", " ".repeat((width - encoded_len) as usize), value)
    }
}

pub(crate) fn get_murim_main_config_list(key: &str) -> rhai::Array {
    std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|config| config.get("메인설정").cloned())
        .and_then(|main| main.get(key).cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|values| {
            values
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .map(Dynamic::from)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn get_murim_config_int(key: &str) -> i64 {
    std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|config| config.get("메인설정").cloned())
        .and_then(|main| main.get(key).cloned())
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
}

pub(crate) fn get_murim_config_float(key: &str) -> f64 {
    std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|config| config.get("메인설정").cloned())
        .and_then(|main| main.get(key).cloned())
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
}

pub(crate) fn apply_item_magic_with_roll(
    item: &mut Object,
    original_level: i64,
    mut option_count: i64,
    force: bool,
    roll: &mut dyn FnMut(i64, i64) -> i64,
) -> bool {
    let position = item.getString("계층");
    let kind = item.getString("종류");
    if position.is_empty()
        || (kind != "무기" && kind != "방어구")
        || !item.getString("옵션").is_empty()
    {
        return false;
    }
    let Ok(source) = std::fs::read_to_string("data/config/magic_map.json") else {
        return false;
    };
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&source) else {
        return false;
    };
    let Some(maxes) = map.get(&position).and_then(serde_json::Value::as_object) else {
        return false;
    };
    let mut level = original_level;
    if force {
        level += 500;
    } else if roll(0, 4) != 0 {
        return false;
    }
    if option_count == 0 {
        option_count = roll(0, 5 * level / 10_001 + 1);
    }
    if roll(0, 10) == 10 {
        option_count += 1;
    }
    if roll(0, 20) == 20 {
        option_count += 1;
    }
    if roll(0, 50) == 50 {
        option_count += 1;
    }
    if force && option_count == 0 {
        option_count = 1;
    }
    option_count = option_count.min(4);
    if option_count == 0 {
        return false;
    }
    level = (level + level * (option_count - 1) / 4).min(10_000);
    let names = [
        "힘",
        "민첩성",
        "맷집",
        "체력",
        "내공",
        "명중",
        "필살",
        "운",
        "회피",
        "경험치",
        "마법발견",
        "공격력",
        "방어력",
    ];
    let mut options = std::collections::HashMap::new();
    let mut attempts = 0;
    let mut valuable = false;
    while options.len() < option_count as usize {
        attempts += 1;
        if attempts > 8 {
            return false;
        }
        let option_name = names[roll(0, names.len() as i64 - 1) as usize];
        if options.contains_key(option_name) {
            continue;
        }
        let max = maxes
            .get(option_name)
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        if max == 0 {
            continue;
        }
        let scaled = level * max / 10_000;
        let value = roll(scaled.div_euclid(2), (scaled as f64 * 1.5) as i64).min(max);
        if value == 0 {
            continue;
        }
        if value as f64 > max as f64 * 0.3 {
            valuable = true;
        }
        if matches!(option_name, "공격력" | "방어력") {
            item.set(option_name, item.getInt(option_name).saturating_add(value));
        }
        options.insert(option_name.to_string(), value);
    }
    item.set("레벨", original_level);
    if kind == "방어구" {
        let current = item.getInt("방어력");
        let base = original_level.div_euclid(20);
        let defense = base + roll((-base).div_euclid(10), base.div_euclid(10));
        if current < defense {
            item.set("방어력", defense);
        }
    }
    item.set_option(&options);
    if option_count > 2 || valuable {
        item.setAttr("아이템속성", "버리지못함");
        item.setAttr("아이템속성", "줄수없음");
    }
    let name = item.getString("이름");
    let plain = strip_ansi_like_python(&name);
    if name == plain {
        let color = match option_count {
            3 => "\x1b[1;37m",
            4 => "\x1b[1;33m",
            _ => "\x1b[1;34m",
        };
        item.set("이름", format!("{color}{name}\x1b[0;37m"));
    }
    true
}

fn get_murim_config_value(key: &str) -> Dynamic {
    std::fs::read_to_string("data/config/murim.json")
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|config| config.get("메인설정").cloned())
        .and_then(|main| main.get(key).cloned())
        .map(|value| crate::data::json_to_dynamic(&value))
        .unwrap_or(Dynamic::UNIT)
}

fn python_get_int(value: &str) -> i64 {
    if value.is_empty() {
        return 0;
    }
    if let Ok(value) = value.parse::<i64>() {
        return value;
    }
    let mut chars = value.chars();
    if !chars.next().is_some_and(|character| character.is_ascii_digit()) {
        return 0;
    }
    let digits: String = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

fn room_has_insurance_agent(body: &Body) -> bool {
    let Ok(world) = get_world_state().read() else {
        return false;
    };
    for mob in world.get_mobs_for_player(&body.get_name()) {
        let Some(data) = world.get_mob_data(&mob.mob_key) else {
            continue;
        };
        if data.name == "표두" || data.reaction_names.iter().any(|name| name == "표두") {
            return true;
        }
    }
    false
}

/// Python Room.findObjName(name) predicate for room-local mobs.  Commands
/// that only need to gate a branch (for example 기부/대여목록) should not
/// duplicate the room scan or accidentally inspect globally loaded mobs.
fn room_has_mob_named(body: &Body, wanted: &str) -> bool {
    let Ok(world) = get_world_state().read() else {
        return false;
    };
    for mob in world.get_mobs_for_player(&body.get_name()) {
        if !mob.alive || mob.act == 2 || mob.act == 3 {
            continue;
        }
        let Some(data) = world.get_mob_data(&mob.mob_key) else {
            continue;
        };
        if data.name == wanted || data.reaction_names.iter().any(|name| name == wanted) {
            return true;
        }
    }
    false
}

/// Script engine configuration
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Directory containing .rhai scripts
    pub script_dir: PathBuf,
    /// Enable hot-reloading
    pub hot_reload: bool,
    /// Script file extension
    pub extension: String,
    /// Data directory for JSON config files
    pub data_dir: PathBuf,
    /// Directory containing library .rhai scripts (hot-reloadable)
    pub lib_dir: PathBuf,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            script_dir: PathBuf::from("cmds"),
            hot_reload: true,
            extension: ".rhai".to_string(),
            data_dir: PathBuf::from("data/config"),
            lib_dir: PathBuf::from("lib"),
        }
    }
}

// 스크립트용: handle_game_command에서 미리 채워 둔 전 접속자 목록. get_all_online_players()가 참조.
thread_local! {
    static PRE_COMPUTED_ALL_ONLINE: RefCell<Option<rhai::Array>> = const { RefCell::new(None) };
    static PRE_COMPUTED_ONLINE_NAMES: RefCell<Option<rhai::Array>> = const { RefCell::new(None) };
    static PRE_COMPUTED_CONNECTED_NAMES: RefCell<Option<rhai::Array>> = const { RefCell::new(None) };
    static PRE_COMPUTED_TELL_PLAYERS: RefCell<Option<Vec<TellPlayerSnapshot>>> = const { RefCell::new(None) };
    static PRE_COMPUTED_ADULT_CHANNEL: RefCell<Option<rhai::Array>> = const { RefCell::new(None) };
    static PRE_COMPUTED_ADULT_CHANNEL_SELF_ID: RefCell<Option<String>> = const { RefCell::new(None) };
    static PRE_COMPUTED_ADULT_CHANNEL_SELF_MEMBER: RefCell<bool> = const { RefCell::new(false) };
    static PRE_COMPUTED_ROOM_INVENTORIES: RefCell<Option<Vec<RoomPlayerInventorySnapshot>>> = const { RefCell::new(None) };
    static PRE_COMPUTED_ROOM_MUGONG_TARGETS: RefCell<Option<Vec<RoomMugongTargetSnapshot>>> = const { RefCell::new(None) };
    /// Raw same-room player fields used by Rhai's `viewMapData` renderer.
    /// Keys are `zone:room`; values retain WorldState's room-index order.
    static PRE_COMPUTED_ROOM_VIEW_PLAYERS: RefCell<Option<HashMap<String, rhai::Array>>> = const { RefCell::new(None) };
}

/// Build data only; Rhai owns the visible `Player.getDesc()` layout.
pub(crate) fn build_room_view_player_snapshot(body: &Body) -> Dynamic {
    let mut player = rhai::Map::new();
    player.insert("name".into(), Dynamic::from(body.get_string("이름")));
    for key in ["이름", "직위", "성격", "기존성격", "입문신청자"] {
        player.insert(key.into(), Dynamic::from(body.get_string(key)));
    }
    player.insert(
        "guild_title".into(),
        Dynamic::from(body.get_string("방파별호")),
    );
    player.insert("head".into(), Dynamic::from(body.get_string("머리말")));
    player.insert("tail".into(), Dynamic::from(body.get_string("꼬리말")));
    player.insert("act".into(), Dynamic::from(body.act.to_i32() as i64));
    player.insert(
        "transparent".into(),
        Dynamic::from(body.get_int("투명상태") == 1),
    );
    player.insert(
        "defense_heads".into(),
        Dynamic::from(
            body.active_skills
                .iter()
                .filter_map(|skill| {
                    let head = crate::data::get_skill_defense_head(&skill.name);
                    (!head.is_empty()).then_some(Dynamic::from(head))
                })
                .collect::<rhai::Array>(),
        ),
    );
    Dynamic::from(player)
}

pub(crate) fn set_precomputed_room_view_players(players: HashMap<String, rhai::Array>) {
    PRE_COMPUTED_ROOM_VIEW_PLAYERS.with(|slot| *slot.borrow_mut() = Some(players));
}

pub(crate) fn clear_precomputed_room_view_players() {
    PRE_COMPUTED_ROOM_VIEW_PLAYERS.with(|slot| *slot.borrow_mut() = None);
}

fn room_view_player_snapshots(zone: &str, room: &str) -> rhai::Array {
    PRE_COMPUTED_ROOM_VIEW_PLAYERS.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|rooms| rooms.get(&format!("{zone}:{room}")))
            .cloned()
            .unwrap_or_default()
    })
}

pub(crate) const ADULT_CHANNEL_DISCONNECT_REQUEST: &str = "_adult_channel_disconnect";
pub(crate) const ADULT_CHANNEL_AUTO_JOIN_REQUEST: &str = "_adult_channel_auto_join";
const ADULT_CHANNEL_ACTION_REQUEST: &str = "_adult_channel_action";
const ADULT_CHANNEL_DELIVERY_REQUESTS: &str = "_adult_channel_deliveries";

/// Raw bytes authored by a Rhai channel command for one adult-channel
/// connection. Rust only routes them; recipient CRLF/prompt text is already
/// included by Rhai from the snapshot data.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(crate) struct AdultChannelDelivery {
    pub member_id: String,
    pub raw_text: String,
}

/// Build only the runtime/user fields used by the four adult-channel Rhai
/// commands. This intentionally does not expose the full online-player list.
pub(crate) fn build_adult_channel_member_snapshot(
    member_id: String,
    body: &Body,
    active: bool,
    interactive: i32,
) -> Dynamic {
    let config = parse_config_string(&body.get_string("설정상태"));
    let mut member = rhai::Map::new();
    member.insert("id".into(), Dynamic::from(member_id));
    member.insert("active".into(), Dynamic::from(active));
    member.insert("이름".into(), Dynamic::from(body.get_string("이름")));
    member.insert(
        "무림별호".into(),
        Dynamic::from(body.get_string("무림별호")),
    );
    member.insert("성격".into(), Dynamic::from(body.get_string("성격")));
    member.insert("소속".into(), Dynamic::from(body.get_string("소속")));
    member.insert("투명상태".into(), Dynamic::from(body.get_int("투명상태")));
    member.insert(
        "외침거부".into(),
        Dynamic::from(config.get("외침거부").map(String::as_str) == Some("1")),
    );
    member.insert(
        "show_prompt".into(),
        Dynamic::from(interactive == 1 && config.get("엘피출력").map(String::as_str) != Some("1")),
    );
    member.insert("hp".into(), Dynamic::from(body.get_hp()));
    member.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
    member.insert("mp".into(), Dynamic::from(body.get_mp()));
    member.insert("max_mp".into(), Dynamic::from(body.get_max_mp()));
    Dynamic::from(member)
}

/// Python `Player._talker`를 접속 객체 단위로 보존하기 위한 임시 키.
/// 이름만 저장하면 같은 이름으로 재접속한 새 객체에 `반전음`이 오배송된다.
pub(crate) const TELL_TALKER_TOKEN: &str = "_tell_talker_token";

/// `전음`/`반전음`에서 실제로 필요한 접속자 상태만 담은 읽기 전용
/// 스냅샷. `get_all_online_players()`의 상세 사용자 맵과 의도적으로
/// 분리한다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TellPlayerSnapshot {
    token: String,
    name: String,
    active: bool,
    transparent: bool,
    refuses_tell: bool,
    show_prompt: bool,
    hp: i64,
    max_hp: i64,
    mp: i64,
    max_mp: i64,
    is_self: bool,
}

impl TellPlayerSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        token: String,
        name: String,
        active: bool,
        transparent: bool,
        config: &str,
        interactive: i32,
        hp: i64,
        max_hp: i64,
        mp: i64,
        max_mp: i64,
        is_self: bool,
    ) -> Self {
        let config = parse_config_string(config);
        Self {
            token,
            name,
            active,
            transparent,
            refuses_tell: config.get("전음거부").map(String::as_str) == Some("1"),
            show_prompt: interactive == 1
                && config.get("엘피출력").map(String::as_str) != Some("1"),
            hp,
            max_hp,
            mp,
            max_mp,
            is_self,
        }
    }

    fn to_rhai_map(&self) -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("found".into(), Dynamic::from(true));
        map.insert("token".into(), Dynamic::from(self.token.clone()));
        map.insert("name".into(), Dynamic::from(self.name.clone()));
        map.insert("refuses_tell".into(), Dynamic::from(self.refuses_tell));
        map.insert("show_prompt".into(), Dynamic::from(self.show_prompt));
        map.insert("hp".into(), Dynamic::from(self.hp));
        map.insert("max_hp".into(), Dynamic::from(self.max_hp));
        map.insert("mp".into(), Dynamic::from(self.mp));
        map.insert("max_mp".into(), Dynamic::from(self.max_mp));
        map.insert("self".into(), Dynamic::from(self.is_self));
        map
    }
}

fn missing_tell_player() -> rhai::Map {
    let mut map = rhai::Map::new();
    map.insert("found".into(), Dynamic::from(false));
    map.insert("token".into(), Dynamic::from(String::new()));
    map.insert("name".into(), Dynamic::from(String::new()));
    map.insert("refuses_tell".into(), Dynamic::from(false));
    map.insert("show_prompt".into(), Dynamic::from(false));
    map.insert("hp".into(), Dynamic::from(0_i64));
    map.insert("max_hp".into(), Dynamic::from(0_i64));
    map.insert("mp".into(), Dynamic::from(0_i64));
    map.insert("max_mp".into(), Dynamic::from(0_i64));
    map.insert("self".into(), Dynamic::from(false));
    map
}

/// `소지품.py`가 같은 방 플레이어를 조회할 때 필요한 읽기 전용 스냅샷.
/// 네트워크의 접속자 맵을 Rhai 실행 중 다시 잠그지 않도록 명령 실행 직전에 채운다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomPlayerInventorySnapshot {
    name: String,
    reaction_names: Vec<String>,
    transparent: bool,
    items: Vec<InventoryItemSnapshot>,
    visible_inventory_count: i64,
    silver: i64,
    gold: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InventoryItemSnapshot {
    name: String,
    count: i64,
    in_use: bool,
    hidden: bool,
}

/// `무공.py`의 관리자 대상 조회에 필요한 같은 방 객체 스냅샷.
/// 출력 문자열은 포함하지 않고, Rhai가 목록을 조립할 데이터만 전달한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoomMugongTargetKind {
    Player,
    Mob,
    Item,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveMugongSnapshot {
    name: String,
    time: i64,
    level: i64,
    defense_time: i64,
    defense_time_increase: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomMugongTargetSnapshot {
    kind: RoomMugongTargetKind,
    name: String,
    reaction_names: Vec<String>,
    transparent: bool,
    act: i32,
    mob_type: i64,
    multiplicity: i64,
    skill_list_nonempty: bool,
    skill_levels: HashMap<String, i64>,
    secret_training: String,
    secret_names: Vec<String>,
    active_skills: Vec<ActiveMugongSnapshot>,
}

fn reaction_names(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == '|' || c.is_whitespace())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn attr_string_list(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    let separator = if raw.contains('|') {
        Some('|')
    } else if raw.contains(',') {
        Some(',')
    } else {
        None
    };
    match separator {
        Some(separator) => raw
            .split(separator)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        None => vec![raw.to_string()],
    }
}

fn active_mugong_snapshots(body: &Body) -> Vec<ActiveMugongSnapshot> {
    body.active_skills
        .iter()
        .filter_map(|active| {
            let skill = crate::world::get_skill(&active.name)?;
            Some(ActiveMugongSnapshot {
                name: active.name.clone(),
                time: i64::from(active.start_time),
                level: body
                    .skill_map
                    .get(&active.name)
                    .map(|training| i64::from(training.level))
                    .unwrap_or(1),
                defense_time: skill.defense_time,
                defense_time_increase: skill.defense_time_increase,
            })
        })
        .collect()
}

/// 현재 접속자의 Python `skillList`/`skillMap` 및 비전 속성 스냅샷.
pub(crate) fn build_room_mugong_player_snapshot(body: &Body) -> RoomMugongTargetSnapshot {
    let mut skill_levels: HashMap<String, i64> = body
        .skill_map
        .iter()
        .map(|(name, training)| (name.clone(), i64::from(training.level)))
        .collect();
    for name in &body.skill_list {
        // Python 무공.py: skillMap에 없지만 skillList에 있으면 1성.
        skill_levels.entry(name.clone()).or_insert(1);
    }

    RoomMugongTargetSnapshot {
        kind: RoomMugongTargetKind::Player,
        name: body.get_name(),
        reaction_names: reaction_names(&body.get_string("반응이름")),
        transparent: body.get_int("투명상태") == 1,
        act: body.act.to_i32(),
        mob_type: 0,
        multiplicity: 1,
        skill_list_nonempty: !body.skill_list.is_empty(),
        skill_levels,
        secret_training: body.get_string("비전수련"),
        secret_names: attr_string_list(&body.get_string("비전이름")),
        active_skills: active_mugong_snapshots(body),
    }
}

/// Python Mob.init에서 `종류 == "전투"`인 무공만 `skillList`에 들어간다.
fn mob_has_combat_skill(data: &RawMobData) -> bool {
    let skill_config = std::fs::read_to_string("data/config/skill.json")
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok());
    data.skills.iter().any(|(name, _, _)| {
        skill_config
            .as_ref()
            .and_then(|config| config.get(name))
            .and_then(|skill| skill.get("종류"))
            .and_then(serde_json::Value::as_str)
            == Some("전투")
    })
}

pub(crate) fn build_room_mugong_mob_snapshot(
    instance: &MobInstance,
    data: &RawMobData,
) -> RoomMugongTargetSnapshot {
    RoomMugongTargetSnapshot {
        kind: RoomMugongTargetKind::Mob,
        name: instance.name.clone(),
        reaction_names: data.reaction_names.clone(),
        transparent: false,
        act: instance.act,
        mob_type: instance.mob_type,
        multiplicity: 1,
        // Python Mob.skillList는 전투 무공 튜플 목록이고 skillMap은 비어 있다.
        skill_list_nonempty: mob_has_combat_skill(data),
        skill_levels: HashMap::new(),
        secret_training: String::new(),
        secret_names: Vec::new(),
        active_skills: Vec::new(),
    }
}

pub(crate) fn build_room_mugong_item_snapshot(item: &Object) -> RoomMugongTargetSnapshot {
    RoomMugongTargetSnapshot {
        kind: RoomMugongTargetKind::Item,
        name: item.getName(),
        reaction_names: reaction_names(&item.getString("반응이름")),
        transparent: item.getInt("투명상태") == 1,
        act: 0,
        mob_type: 0,
        multiplicity: 1,
        skill_list_nonempty: false,
        skill_levels: HashMap::new(),
        secret_training: String::new(),
        secret_names: Vec::new(),
        active_skills: Vec::new(),
    }
}

pub(crate) fn build_room_mugong_stack_item_snapshot(
    key: &str,
    count: i64,
) -> Option<RoomMugongTargetSnapshot> {
    let (name, aliases, _, _) = get_item_info(key)?;
    Some(RoomMugongTargetSnapshot {
        kind: RoomMugongTargetKind::Item,
        name,
        reaction_names: reaction_names(&aliases),
        transparent: false,
        act: 0,
        mob_type: 0,
        multiplicity: count.max(1),
        skill_list_nonempty: false,
        skill_levels: HashMap::new(),
        secret_training: String::new(),
        secret_names: Vec::new(),
        active_skills: Vec::new(),
    })
}

/// 현재 접속자의 Body에서 같은 방 조회용 소지품 스냅샷을 만든다.
pub(crate) fn build_room_player_inventory_snapshot(body: &Body) -> RoomPlayerInventorySnapshot {
    let mut items = Vec::new();
    let mut visible_inventory_count = 0i64;

    for item in &body.object.objs {
        if let Ok(item) = item.lock() {
            let in_use = item.getBool("inUse");
            let hidden = item.checkAttr("아이템속성", "출력안함");
            if !in_use && !hidden {
                visible_inventory_count += 1;
            }
            items.push(InventoryItemSnapshot {
                name: item.getName(),
                count: 1,
                in_use,
                hidden,
            });
        }
    }

    // inv_stack은 여러 Python 아이템 객체를 수량으로 압축한 Rust 내부 표현이다.
    // 키 순으로 읽어 HashMap 반복 순서가 출력에 스며들지 않게 한다.
    let mut stack_items: Vec<_> = body.object.inv_stack.iter().collect();
    stack_items.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (key, count) in stack_items {
        if *count <= 0 {
            continue;
        }
        if let Some((name, _, _, _)) = get_item_info(key) {
            visible_inventory_count += *count;
            items.push(InventoryItemSnapshot {
                name,
                count: *count,
                in_use: false,
                hidden: false,
            });
        }
    }

    RoomPlayerInventorySnapshot {
        name: body.get_name(),
        reaction_names: reaction_names(&body.get_string("반응이름")),
        transparent: body.get_int("투명상태") == 1,
        items,
        visible_inventory_count,
        silver: body.get_int("은전"),
        gold: body.get_int("금전"),
    }
}

/// 같은 방 플레이어만 담은 스냅샷을 현재 명령 실행 스레드에 설정한다.
pub(crate) fn set_precomputed_room_inventories(players: Vec<RoomPlayerInventorySnapshot>) {
    PRE_COMPUTED_ROOM_INVENTORIES.with(|cell| *cell.borrow_mut() = Some(players));
}

pub(crate) fn set_precomputed_room_mugong_targets(targets: Vec<RoomMugongTargetSnapshot>) {
    PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| *cell.borrow_mut() = Some(targets));
}

fn python_leading_int(value: &str) -> i64 {
    if let Ok(number) = value.parse::<i64>() {
        return number;
    }
    if !value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        return 0;
    }
    value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Python `Room.findObjName` 중 플레이어에 해당하는 이름/반응이름 탐색 규칙.
fn find_room_inventory_target(
    line: &str,
    players: &[RoomPlayerInventorySnapshot],
) -> Option<RoomPlayerInventorySnapshot> {
    let mut name = line.split_whitespace().next()?.to_string();
    if name.trim() == "." {
        name = "1".to_string();
    }

    // Python은 순수 숫자를 방의 N번째 몹으로만 해석한다. 따라서 플레이어 조회는 실패한다.
    if !name.is_empty() && name.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }

    let parsed_order = python_leading_int(&name);
    let order = if parsed_order == 0 { 1 } else { parsed_order };
    if parsed_order != 0 {
        name = name
            .trim_start_matches(|character: char| character.is_ascii_digit())
            .to_string();
    }

    let mut exact_count = 0i64;
    let mut prefix_count = 0i64;
    for player in players.iter().filter(|player| !player.transparent) {
        if player.name == name || player.reaction_names.iter().any(|alias| alias == &name) {
            exact_count += 1;
            if exact_count == order {
                return Some(player.clone());
            }
        } else {
            for alias in &player.reaction_names {
                if alias.starts_with(&name) {
                    prefix_count += 1;
                    if prefix_count == order {
                        return Some(player.clone());
                    }
                }
            }
        }
    }
    None
}

fn inventory_view(snapshot: &RoomPlayerInventorySnapshot, viewer_admin: i64) -> Dynamic {
    let mut result = rhai::Map::new();
    result.insert("ok".into(), Dynamic::from(true));
    result.insert(
        "empty".into(),
        Dynamic::from(snapshot.visible_inventory_count == 0),
    );
    result.insert("silver".into(), Dynamic::from(snapshot.silver));
    result.insert("gold".into(), Dynamic::from(snapshot.gold));

    let mut grouped: Vec<(String, i64)> = Vec::new();
    if snapshot.visible_inventory_count != 0 {
        for item in &snapshot.items {
            if item.in_use || (item.hidden && viewer_admin < 1000) {
                continue;
            }
            if let Some((_, count)) = grouped.iter_mut().find(|(name, _)| name == &item.name) {
                *count += item.count;
            } else {
                grouped.push((item.name.clone(), item.count));
            }
        }
    }

    let items = grouped
        .into_iter()
        .map(|(name, count)| Dynamic::from(vec![Dynamic::from(name), Dynamic::from(count)]))
        .collect::<rhai::Array>();
    result.insert("items".into(), Dynamic::from(items));
    Dynamic::from(result)
}

fn find_room_mugong_target(
    line: &str,
    targets: &[RoomMugongTargetSnapshot],
) -> Option<RoomMugongTargetSnapshot> {
    let mut name = line.split_whitespace().next()?.to_string();
    if name.trim() == "." {
        name = "1".to_string();
    }

    // Python Room.findObjName: 순수 양의 정수는 살아 있는 N번째 몹이다.
    if name.chars().all(|character| character.is_ascii_digit()) {
        let order = name.parse::<i64>().unwrap_or(0);
        if order > 0 {
            let eligible: Vec<_> = targets
                .iter()
                .filter(|target| {
                    target.kind == RoomMugongTargetKind::Mob
                        && target.mob_type != 7
                        && !matches!(target.act, 2 | 3)
                })
                .collect();
            if eligible.len() == 1 && order <= eligible[0].multiplicity {
                return Some(eligible[0].clone());
            }
            return None;
        }
    }

    let parsed_order = python_leading_int(&name);
    let order = if parsed_order == 0 { 1 } else { parsed_order };
    if parsed_order != 0 {
        name = name
            .trim_start_matches(|character: char| character.is_ascii_digit())
            .to_string();
    }

    // Rust는 Python room.objs의 player/mob/item 통합 삽입 순서를 아직
    // 보존하지 않는다. 서로 다른 객체가 동시에 일치하면 임의 우선순위를
    // 만들지 않고 미해결(None)로 남긴다. 단일 객체의 수량/복수 alias는
    // Python의 order 계산을 그대로 적용할 수 있다.
    let mut matches: Vec<(RoomMugongTargetSnapshot, i64)> = Vec::new();
    for target in targets {
        if target.transparent {
            continue;
        }
        if target.kind == RoomMugongTargetKind::Mob && name != "시체" && matches!(target.act, 2 | 3)
        {
            continue;
        }

        let corpse_match =
            name == "시체" && target.kind != RoomMugongTargetKind::Item && target.act == 2;
        let exact_match =
            target.name == name || target.reaction_names.iter().any(|alias| alias == &name);
        if corpse_match || exact_match {
            matches.push((target.clone(), target.multiplicity));
        } else {
            // Python은 반응이름 각각을 접두사 후보로 세므로 그대로 센다.
            let alias_matches = target
                .reaction_names
                .iter()
                .filter(|alias| alias.starts_with(&name))
                .count() as i64;
            if alias_matches > 0 {
                matches.push((target.clone(), alias_matches * target.multiplicity));
            }
        }
    }
    if matches.len() != 1 || order <= 0 {
        return None;
    }
    let (target, occurrences) = matches.pop().unwrap();
    (order <= occurrences).then_some(target)
}

fn mugong_view(snapshot: &RoomMugongTargetSnapshot, viewer_name: &str) -> Dynamic {
    let mut result = rhai::Map::new();
    result.insert("ok".into(), Dynamic::from(true));
    result.insert("name".into(), Dynamic::from(snapshot.name.clone()));
    result.insert(
        "self".into(),
        Dynamic::from(
            snapshot.kind == RoomMugongTargetKind::Player && snapshot.name == viewer_name,
        ),
    );
    result.insert(
        "has_skill_list".into(),
        Dynamic::from(snapshot.skill_list_nonempty),
    );

    let mut levels: Vec<_> = snapshot.skill_levels.iter().collect();
    levels.sort_by(|(left, _), (right, _)| left.cmp(right));
    let skill_entries = levels
        .into_iter()
        .map(|(name, level)| {
            let mut entry = rhai::Map::new();
            entry.insert("name".into(), Dynamic::from(name.clone()));
            entry.insert("level".into(), Dynamic::from(*level));
            Dynamic::from(entry)
        })
        .collect::<rhai::Array>();
    result.insert("skills".into(), Dynamic::from(skill_entries));
    result.insert(
        "secret_training".into(),
        Dynamic::from(snapshot.secret_training.clone()),
    );
    result.insert(
        "secret_names".into(),
        Dynamic::from(
            snapshot
                .secret_names
                .iter()
                .cloned()
                .map(Dynamic::from)
                .collect::<rhai::Array>(),
        ),
    );
    result.insert(
        "active_skills".into(),
        Dynamic::from(
            snapshot
                .active_skills
                .iter()
                .map(|active| {
                    let mut entry = rhai::Map::new();
                    entry.insert("name".into(), Dynamic::from(active.name.clone()));
                    entry.insert("time".into(), Dynamic::from(active.time));
                    entry.insert("level".into(), Dynamic::from(active.level));
                    entry.insert("defense_time".into(), Dynamic::from(active.defense_time));
                    entry.insert(
                        "defense_time_increase".into(),
                        Dynamic::from(active.defense_time_increase),
                    );
                    Dynamic::from(entry)
                })
                .collect::<rhai::Array>(),
        ),
    );
    Dynamic::from(result)
}

/// handle_game_command에서 호출. 전 접속자(이름, 무림별호, 성격, 레벨초기화, 소속) 배열 세팅.
pub fn set_precomputed_all_online(a: rhai::Array) {
    PRE_COMPUTED_ALL_ONLINE.with(|c| *c.borrow_mut() = Some(a));
}

/// 투명 상태와 무관한 실제 접속자 이름 목록. 전체 전달용이며 `누구` 표시 목록과 분리한다.
pub fn set_precomputed_online_names(a: rhai::Array) {
    PRE_COMPUTED_ONLINE_NAMES.with(|c| *c.borrow_mut() = Some(a));
}

/// Python `Client.players`/`channel.players` 호환 목록. 로그인 완료
/// 여부와 관계없이 Player 이름이 있는 현재 연결을 담는다.
pub fn set_precomputed_connected_names(a: rhai::Array) {
    PRE_COMPUTED_CONNECTED_NAMES.with(|cell| *cell.borrow_mut() = Some(a));
}

/// `전음`/`반전음` 실행 직전에 네트워크가 만든 최소 접속자 스냅샷을
/// 현재 Rhai 실행 스레드에 설치한다.
pub(crate) fn set_precomputed_tell_players(players: Vec<TellPlayerSnapshot>) {
    PRE_COMPUTED_TELL_PLAYERS.with(|cell| *cell.borrow_mut() = Some(players));
}

/// Install the ordered Python `adultCH` view for one command execution.
/// Each array entry is a hashmap of user/runtime attributes assembled by the
/// network layer; the Rhai commands own every displayed string.
pub(crate) fn set_precomputed_adult_channel(
    members: rhai::Array,
    self_id: String,
    self_is_member: bool,
) {
    PRE_COMPUTED_ADULT_CHANNEL.with(|cell| *cell.borrow_mut() = Some(members));
    PRE_COMPUTED_ADULT_CHANNEL_SELF_ID.with(|cell| *cell.borrow_mut() = Some(self_id));
    PRE_COMPUTED_ADULT_CHANNEL_SELF_MEMBER.with(|cell| *cell.borrow_mut() = self_is_member);
}

/// Drain state-only requests emitted by the adult-channel efuns. Raw text is
/// opaque here: it was authored by Rhai and is routed unchanged by network.
pub(crate) fn take_adult_channel_requests(
    body: &mut Body,
) -> (Option<String>, Vec<AdultChannelDelivery>) {
    let action = body
        .temp_mut()
        .remove(ADULT_CHANNEL_ACTION_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string));
    let deliveries = body
        .temp_mut()
        .remove(ADULT_CHANNEL_DELIVERY_REQUESTS)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    body.temp_mut().remove(ADULT_CHANNEL_DISCONNECT_REQUEST);
    body.temp_mut().remove(ADULT_CHANNEL_AUTO_JOIN_REQUEST);
    (action, deliveries)
}

pub fn get_connected_player_names() -> rhai::Array {
    PRE_COMPUTED_CONNECTED_NAMES
        .with(|cell| cell.borrow().clone())
        .unwrap_or_default()
}

/// 스크립트 get_all_online_players()에서 호출.
pub fn get_precomputed_all_online() -> rhai::Array {
    PRE_COMPUTED_ALL_ONLINE
        .with(|c| c.borrow().clone())
        .unwrap_or_default()
}

/// PreComputedOtherDescsGuard Drop에서 호출.
pub fn clear_precomputed_all_online() {
    PRE_COMPUTED_ALL_ONLINE.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_ONLINE_NAMES.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_CONNECTED_NAMES.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_TELL_PLAYERS.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_ADULT_CHANNEL.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_ADULT_CHANNEL_SELF_ID.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_ADULT_CHANNEL_SELF_MEMBER.with(|c| *c.borrow_mut() = false);
    PRE_COMPUTED_ROOM_INVENTORIES.with(|c| *c.borrow_mut() = None);
    PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|c| *c.borrow_mut() = None);
    party::clear_precomputed_party_context();
    clear_cast_room_players();
}

/// 설정상태 문자열 파싱: "키 값" (줄바꿈 또는 공백 구분). ob["설정"][키]에 대응.
fn parse_config_string(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    if s.is_empty() {
        return out;
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    if s.contains('\n') || s.contains('|') {
        for line in s.split(['\n', '|']) {
            let line = line.trim();
            if let Some(sp) = line.find(' ') {
                let (k, v) = (line[..sp].to_string(), line[sp + 1..].trim().to_string());
                if !k.is_empty() {
                    pairs.push((k, v));
                }
            }
        }
    } else {
        let toks: Vec<&str> = s.split_whitespace().collect();
        let mut i = 0;
        while i + 1 < toks.len() {
            pairs.push((toks[i].to_string(), toks[i + 1].to_string()));
            i += 2;
        }
    }
    for (k, v) in pairs {
        out.insert(k, v);
    }
    out
}

pub(crate) fn config_is_enabled(config: &str, key: &str) -> bool {
    parse_config_string(config).get(key).map(String::as_str) == Some("1")
}

/// 설정상태 맵을 문자열로 직렬화. "키 값"을 \n으로 이어붙임.
fn format_config_string(m: &std::collections::HashMap<String, String>) -> String {
    let mut v: Vec<_> = m.iter().map(|(k, val)| format!("{} {}", k, val)).collect();
    v.sort();
    v.join("\n")
}

/// 이벤트설정리스트 파싱: "키=값" 또는 "키" 한 줄씩(\n 구분). ob["이벤트"][키]에 대응.
/// world::event::do_event에서도 사용. pub(crate).
pub(crate) fn parse_event_string(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for line in s.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(eq) = line.find('=') {
            out.insert(line[..eq].to_string(), line[eq + 1..].to_string());
        } else {
            out.insert(line.to_string(), "1".to_string());
        }
    }
    out
}

pub(crate) fn format_event_string(m: &std::collections::HashMap<String, String>) -> String {
    let mut v: Vec<_> = m
        .iter()
        .map(|(k, val)| {
            if val == "1" {
                k.clone()
            } else {
                format!("{}={}", k, val)
            }
        })
        .collect();
    v.sort();
    v.join("\n")
}

/// Body의 현재 위치를 WorldState에서 우선 읽고, 저장 속성의 `zone:room` 및
/// 레거시 `zone/room` 형식을 모두 보조적으로 허용한다.
pub(crate) fn current_body_position(body: &Body) -> Option<(String, String)> {
    let name = body.get_name();
    if !name.is_empty() {
        if let Ok(world) = get_world_state().read() {
            if let Some(position) = world.get_player_position(&name) {
                return Some((position.zone.clone(), position.room.clone()));
            }
        }
    }

    for key in ["위치", "현재방"] {
        let location = body.get_string(key);
        if let Some((zone, room)) = location
            .split_once(':')
            .or_else(|| location.split_once('/'))
        {
            if !zone.is_empty() && !room.is_empty() {
                return Some((zone.to_string(), room.to_string()));
            }
        }
    }
    None
}

fn book_catalog_path(_body: &Body) -> String {
    #[cfg(test)]
    {
        let configured = _body.get_string("__시험도서목록경로");
        if !configured.is_empty() {
            return configured;
        }
    }
    "data/config/book.json".to_string()
}

fn rewrite_room_exits(
    zone: &str,
    room: &str,
    edit: impl FnOnce(&mut Vec<String>) -> String,
) -> String {
    let path = format!("data/map/{zone}/{room}.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return "missing".to_string();
    };
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return "missing".to_string();
    };
    let Some(raw) = root
        .get_mut("맵정보")
        .and_then(|value| value.get_mut("출구"))
    else {
        return "missing".to_string();
    };
    let mut exits = match raw {
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect(),
        serde_json::Value::String(value) => value
            .split("\r\n")
            .filter(|entry| !entry.is_empty())
            .map(ToString::to_string)
            .collect(),
        _ => return "missing".to_string(),
    };
    let status = edit(&mut exits);
    if status == "missing" {
        return status;
    }
    *raw = serde_json::Value::Array(
        exits
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    );
    if std::fs::write(&path, serde_json::to_string_pretty(&root).unwrap_or(text)).is_err() {
        return "missing".to_string();
    }
    status
}

fn python_coerce_attribute(existing: Option<&Value>, raw: &str) -> Result<Value, ()> {
    match existing {
        Some(Value::Int(_)) => raw.parse::<i64>().map(Value::Int).map_err(|_| ()),
        Some(Value::Float(_)) => raw.parse::<f64>().map(Value::Float).map_err(|_| ()),
        Some(Value::String(_)) => Ok(Value::String(raw.to_string())),
        None => Ok(raw.into()),
    }
}

fn python_set_admin_target(body: &mut Body, target: &str, key: &str, raw: &str) -> String {
    let Some((zone, room)) = current_body_position(body) else {
        return "missing".into();
    };
    if target == "방" {
        get_world_state()
            .write()
            .unwrap()
            .get_room_attrs_mut(&zone, &room)
            .insert(key.to_string(), raw.to_string());
        return "ok".into();
    }
    if target == body.get_name() {
        let value = match python_coerce_attribute(body.object.attr.get(key), raw) {
            Ok(value) => value,
            Err(()) => return "invalid".into(),
        };
        body.set(key, value);
        return "ok".into();
    }

    let room_objects = get_world_state()
        .read()
        .ok()
        .map(|world| world.get_room_objs(&zone, &room).to_vec())
        .unwrap_or_default();
    for object in room_objects {
        let Ok(mut object) = object.lock() else { continue };
        if object.getName() != target
            && !object
                .getString("반응이름")
                .split("\r\n")
                .any(|alias| alias == target)
        {
            continue;
        }
        let value = match python_coerce_attribute(object.attr.get(key), raw) {
            Ok(value) => value,
            Err(()) => return "invalid".into(),
        };
        object.set(key, value);
        return "ok".into();
    }

    let mob_id = get_world_state().read().ok().and_then(|world| {
        world
            .mob_cache
            .get_all_mobs_in_room(&zone, &room)
            .into_iter()
            .find_map(|mob| {
                let data = world.get_mob_data(&mob.mob_key)?;
                (mob.name == target
                    || data.name == target
                    || data.reaction_names.iter().any(|alias| alias == target))
                    .then_some(mob.instance_id)
            })
    });
    if let Some(mob_id) = mob_id {
        let mut world = get_world_state().write().unwrap();
        let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
            return "missing".into();
        };
        let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == mob_id) else {
            return "missing".into();
        };
        let existing = match key {
            "이름" => Some(Value::String(mob.name.clone())),
            "체력" => Some(Value::Int(mob.hp)),
            "최고체력" => Some(Value::Int(mob.max_hp)),
            "내공" => Some(Value::Int(mob.mp)),
            "최고내공" => Some(Value::Int(mob.max_mp)),
            "은전" => Some(Value::Int(mob.gold)),
            "레벨" => Some(Value::Int(mob.level)),
            "힘" => Some(Value::Int(mob.strength)),
            "맷집" => Some(Value::Int(mob.arm)),
            "민첩성" => Some(Value::Int(mob.agility)),
            _ => mob.runtime_attrs.get(key).cloned(),
        };
        let value = match python_coerce_attribute(existing.as_ref(), raw) {
            Ok(value) => value,
            Err(()) => return "invalid".into(),
        };
        match (key, &value) {
            ("이름", Value::String(value)) => mob.name = value.clone(),
            ("체력", Value::Int(value)) => mob.hp = *value,
            ("최고체력", Value::Int(value)) => mob.max_hp = *value,
            ("내공", Value::Int(value)) => mob.mp = *value,
            ("최고내공", Value::Int(value)) => mob.max_mp = *value,
            ("은전", Value::Int(value)) => mob.gold = *value,
            ("레벨", Value::Int(value)) => mob.level = *value,
            ("힘", Value::Int(value)) => mob.strength = *value,
            ("맷집", Value::Int(value)) => mob.arm = *value,
            ("민첩성", Value::Int(value)) => mob.agility = *value,
            _ => {
                mob.runtime_attrs.insert(key.to_string(), value);
            }
        }
        return "ok".into();
    }

    // Python falls back from env.findObjName() to the player's inventory.
    for object in &body.object.objs {
        let Ok(mut object) = object.lock() else { continue };
        if object.getName() != target
            && !object
                .getString("반응이름")
                .split("\r\n")
                .any(|alias| alias == target)
        {
            continue;
        }
        let value = match python_coerce_attribute(object.attr.get(key), raw) {
            Ok(value) => value,
            Err(()) => return "invalid".into(),
        };
        object.set(key, value);
        return "ok".into();
    }
    "missing".into()
}

fn destroy_item_result(status: &str, name: String, count: i64) -> rhai::Map {
    let mut result = rhai::Map::new();
    result.insert("status".into(), Dynamic::from(status.to_string()));
    result.insert("name".into(), Dynamic::from(name));
    result.insert("count".into(), Dynamic::from(count));
    result.insert("post".into(), Dynamic::from(String::new()));
    result
}

fn destroy_inventory_for_command(
    body: &mut Body,
    wanted: &str,
    order: i64,
    count: i64,
    break_mode: bool,
) -> rhai::Map {
    let order = order.max(1) as usize;
    let count = count.clamp(1, 100) as usize;
    if !break_mode && order == 1 {
        if let Some(key) = find_item_key_by_name(wanted) {
            if is_stackable(&key) {
                let have = *body.object.inv_stack.get(&key).unwrap_or(&0);
                let removed = (count as i64).clamp(0, have);
                if removed > 0 {
                    if let Some(value) = body.object.inv_stack.get_mut(&key) {
                        *value -= removed;
                        if *value <= 0 {
                            body.object.inv_stack.remove(&key);
                        }
                    }
                    let name = object_from_item_json(&key)
                        .and_then(|(item, _)| item.lock().ok().map(|item| item.getName()))
                        .unwrap_or_else(|| wanted.to_string());
                    let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
                    return destroy_item_result("ok", name, removed);
                }
            }
        }
    }

    let mut matched = 0usize;
    let mut selected: Vec<Arc<Mutex<Object>>> = Vec::new();
    let mut last_name = String::new();
    let mut last_ansi = String::new();
    for item in &body.object.objs {
        let Ok(object) = item.lock() else { continue };
        let aliases = reaction_names(&object.getString("반응이름"));
        if object.getName() != wanted && !aliases.iter().any(|alias| alias == wanted) {
            continue;
        }
        if object.getBool("inUse")
            || (break_mode && object.checkAttr("아이템속성", "출력안함"))
        {
            continue;
        }
        matched += 1;
        if matched < order {
            continue;
        }
        if break_mode && object.checkAttr("아이템속성", "부수지못함") {
            if selected.is_empty() {
                return destroy_item_result("unbreakable", String::new(), 0);
            }
            continue;
        }
        last_name = object.getName();
        last_ansi = object.getString("안시");
        selected.push(item.clone());
        if selected.len() >= count {
            break;
        }
    }
    if selected.is_empty() {
        return destroy_item_result("missing", String::new(), 0);
    }
    for item in &selected {
        if let Ok(object) = item.lock() {
            if object.checkAttr("아이템속성", "단일아이템") {
                let index = object.getString("인덱스");
                if !index.is_empty() {
                    let _ = crate::oneitem::oneitem_destroy(&index);
                }
            }
        }
        body.object.remove(item);
    }
    let removed = selected.len() as i64;
    let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
    let mut result = destroy_item_result("ok", last_name.clone(), removed);
    let name_a = if last_ansi.is_empty() {
        format!("\x1b[0;36m{last_name}\x1b[37m")
    } else {
        format!("{last_ansi}{last_name}\x1b[0;37m")
    };
    result.insert(
        "post".into(),
        Dynamic::from(format!("{name_a}{}", han_eul(&last_name))),
    );
    result
}

// ============================================================
// 호위 (Guard) 시스템 관련 타입 및 헬퍼 함수
// ============================================================

/// 호위 데이터 구조체
#[derive(Debug, Clone)]
struct GuardData {
    name: String,
    hp: i64,
    max_hp: i64,
    description: String,
}

/// 호위 리스트 파싱: JSON 형식 문자열에서 GuardData 벡터로
fn parse_guards_list(s: &str) -> Vec<GuardData> {
    if s.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(arr)) => {
            let mut guards = Vec::new();
            for v in arr {
                if let Some(obj) = v.as_object() {
                    let name = obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .or_else(|| obj.get("이름").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let hp = obj
                        .get("hp")
                        .and_then(|v| v.as_i64())
                        .or_else(|| obj.get("체력").and_then(|v| v.as_i64()))
                        .unwrap_or(100);
                    let max_hp = obj
                        .get("max_hp")
                        .and_then(|v| v.as_i64())
                        .or_else(|| obj.get("max_체력").and_then(|v| v.as_i64()))
                        .or_else(|| obj.get("최고체력").and_then(|v| v.as_i64()))
                        .unwrap_or(hp);
                    let description = obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .or_else(|| obj.get("설명").and_then(|v| v.as_str()))
                        .or_else(|| obj.get("설명2").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();

                    if !name.is_empty() {
                        guards.push(GuardData {
                            name,
                            hp,
                            max_hp,
                            description,
                        });
                    }
                }
            }
            guards
        }
        _ => Vec::new(),
    }
}

/// 호위 리스트를 JSON 형식 문자열로 변환
fn format_guards_list(guards: &[GuardData]) -> String {
    let arr: Vec<serde_json::Value> = guards
        .iter()
        .map(|g| {
            serde_json::json!({
                "name": g.name,
                "hp": g.hp,
                "max_hp": g.max_hp,
                "description": g.description
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_default()
}

/// 몹 이름으로 몹 데이터 조회 (get_mob_by_name 구현)
fn get_mob_by_name_impl(mob_name: &str) -> Option<serde_json::Value> {
    let full_path = format!("data/mob/{}.json", mob_name);
    std::fs::read_to_string(&full_path)
        .ok()
        .and_then(|content| {
            serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .and_then(|v| v.get("몹정보").cloned())
        })
}

/// 접속 중인 이름 목록. get_precomputed_all_online에서 이름만 추출.
pub fn get_online_names() -> rhai::Array {
    use rhai::Dynamic;
    if let Some(names) = PRE_COMPUTED_ONLINE_NAMES.with(|c| c.borrow().clone()) {
        return names;
    }
    PRE_COMPUTED_ALL_ONLINE.with(|c| {
        let a = c.borrow();
        if let Some(ref arr) = *a {
            let mut out = rhai::Array::new();
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    if let Some(n) = m
                        .get("이름")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                    {
                        if !n.is_empty() {
                            out.push(Dynamic::from(n));
                        }
                    }
                }
            }
            out
        } else {
            rhai::Array::new()
        }
    })
}

/// 해당 이름이 설정(ob["설정"]["외침거부"])에서 "1"인지. get_precomputed_all_online의 설정상태 파싱.
pub fn user_refuses_shout(name: &str) -> bool {
    use rhai::Dynamic;
    PRE_COMPUTED_ALL_ONLINE.with(|c| {
        let a = c.borrow();
        if let Some(ref arr) = *a {
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    let n: String = m
                        .get("이름")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if n == name {
                        let cfg: String = m
                            .get("설정상태")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                            .unwrap_or_default();
                        return parse_config_string(&cfg)
                            .get("외침거부")
                            .map(|v| v.as_str())
                            == Some("1");
                    }
                }
            }
        }
        false
    })
}

/// Stored script with metadata
struct StoredScript {
    /// Source code of the script
    source: String,
    /// Last modification time
    modified: std::time::SystemTime,
    /// Script name
    _name: String,
}

/// Equipment stats for applying/removing bonuses
struct EquipStats {
    attack: i32,
    defense: i32,
    strength: i32,
    dexterity: i32,
    armor: i32,
    max_hp: i32,
    max_mp: i32,
    hit: i32,
    miss: i32,
    critical: i32,
    luck: i32,
    exp: i32,
    magic_chance: i32,
}

fn equipment_stats(item: &crate::object::Object) -> EquipStats {
    let options = item.get_option().unwrap_or_default();
    let option = |name: &str| options.get(name).copied().unwrap_or(0) as i32;
    EquipStats {
        attack: item.getInt("공격력") as i32,
        defense: item.getInt("방어력") as i32,
        strength: option("힘"),
        dexterity: option("민첩성"),
        armor: option("맷집"),
        max_hp: option("체력"),
        max_mp: option("내공"),
        hit: option("명중"),
        miss: option("회피"),
        critical: option("필살"),
        luck: option("운"),
        exp: option("경험치"),
        magic_chance: option("마법발견"),
    }
}

fn apply_equipment_stats(body: &mut Body, stats: &EquipStats) {
    body.attpower += stats.attack;
    body.armor += stats.defense;
    body._str += stats.strength;
    body._dex += stats.dexterity;
    body._arm += stats.armor;
    body._maxhp += stats.max_hp;
    body._maxmp += stats.max_mp;
    body._hit += stats.hit;
    body._miss += stats.miss;
    body._critical += stats.critical;
    body._critical_chance += stats.luck;
    body._exp += stats.exp;
    body._magic_chance += stats.magic_chance;
}

/// [36m, [37m 등 ESC 없는 축약 ANSI를 \x1b[36m 형태로 확장.
/// 이미 \x1b[...]m 인 경우는 플레이스홀더로 보호 후 복원하여 이중 치환 방지.
fn expand_abbreviated_ansi(s: &str) -> String {
    let mut r = s.to_string();
    let protected: Vec<(String, String)> = vec![
        ("\x1b[36m".into(), "\u{E000}".into()),
        ("\x1b[37m".into(), "\u{E001}".into()),
        ("\x1b[33m".into(), "\u{E002}".into()),
        ("\x1b[0;37m".into(), "\u{E003}".into()),
        ("\x1b[1;32m".into(), "\u{E004}".into()),
    ];
    for (full, place) in &protected {
        r = r.replace(full, place);
    }
    r = r.replace("[;37m", "\x1b[0;37m"); // [0;37m 오타(0 누락) 보정
    r = r.replace("[36m", "\x1b[36m");
    r = r.replace("[37m", "\x1b[37m");
    r = r.replace("[33m", "\x1b[33m");
    r = r.replace("[0;37m", "\x1b[0;37m");
    r = r.replace("[1;32m", "\x1b[1;32m");
    for (full, place) in &protected {
        r = r.replace(place, full);
    }
    r
}

/// ANSI color code mapping for Rhai scripts
fn ansi_convert(msg: &str, conv: bool) -> String {
    let mut buf = msg.to_string();

    if conv {
        buf = buf.replace("{밝}", "\x1b[1m");
        buf = buf.replace("{어}", "\x1b[0m");
        buf = buf.replace("{검}", "\x1b[30m");
        buf = buf.replace("{빨}", "\x1b[31m");
        buf = buf.replace("{초}", "\x1b[32m");
        buf = buf.replace("{노}", "\x1b[33m");
        buf = buf.replace("{파}", "\x1b[34m");
        buf = buf.replace("{자}", "\x1b[35m");
        buf = buf.replace("{하}", "\x1b[36m");
        buf = buf.replace("{흰}", "\x1b[37m");
        buf = buf.replace("{배검}", "\x1b[40m");
        buf = buf.replace("{배빨}", "\x1b[41m");
        buf = buf.replace("{배초}", "\x1b[42m");
        buf = buf.replace("{배노}", "\x1b[43m");
        buf = buf.replace("{배파}", "\x1b[44m");
        buf = buf.replace("{배자}", "\x1b[45m");
        buf = buf.replace("{배하}", "\x1b[46m");
        buf = buf.replace("{배흰}", "\x1b[47m");
    } else {
        buf = buf.replace("{밝}", "");
        buf = buf.replace("{어}", "");
        buf = buf.replace("{검}", "");
        buf = buf.replace("{빨}", "");
        buf = buf.replace("{초}", "");
        buf = buf.replace("{노}", "");
        buf = buf.replace("{파}", "");
        buf = buf.replace("{자}", "");
        buf = buf.replace("{하}", "");
        buf = buf.replace("{흰}", "");
        buf = buf.replace("{배검}", "");
        buf = buf.replace("{배빨}", "");
        buf = buf.replace("{배초}", "");
        buf = buf.replace("{배노}", "");
        buf = buf.replace("{배파}", "");
        buf = buf.replace("{배자}", "");
        buf = buf.replace("{배하}", "");
        buf = buf.replace("{배흰}", "");
    }

    buf
}

/// Korean particle helper (이/가)
fn han_iga(name: &str) -> String {
    use crate::hangul::han_iga;
    han_iga(name).to_string()
}

/// Korean particle helper (을/를) - 목적어 조사
fn han_eul(name: &str) -> String {
    use crate::hangul::han_obj;
    han_obj(name).to_string()
}

/// Korean particle helper (은/는)
fn han_eun(name: &str) -> String {
    use crate::hangul::han_un;
    han_un(name).to_string()
}

/// Korean particle helper (와/과)
fn han_wa(name: &str) -> String {
    use crate::hangul::han_wa;
    han_wa(name).to_string()
}

/// Korean particle helper ((으)로)
fn han_uro(name: &str) -> String {
    crate::hangul::han_uro(name).to_string()
}

// ---------------------------------------------------------------------------
// 비밀번호 bcrypt 해시와 레거시 검증
// ---------------------------------------------------------------------------

/// 평문을 bcrypt로 해시한다. bcrypt 문자열에는 알고리즘, cost, salt가 포함된다.
pub fn password_hash(plain: &str) -> String {
    bcrypt::hash(plain, bcrypt::DEFAULT_COST).expect("bcrypt password hashing failed")
}

/// 저장된 값(해시 또는 레거시 평문)과 평문 입력이 일치하는지 검사.
/// 신규 bcrypt와 기존 SHA-512/평문 계정을 모두 읽어 점진적으로 이관한다.
pub fn password_verify(stored: &str, plain: &str) -> bool {
    if stored.starts_with("$2a$") || stored.starts_with("$2b$") || stored.starts_with("$2y$") {
        return bcrypt::verify(plain, stored).unwrap_or(false);
    }
    let is_sha512_hex = stored.len() == 128 && stored.chars().all(|c| c.is_ascii_hexdigit());
    if is_sha512_hex {
        use sha2::{Digest, Sha512};
        let mut h = Sha512::new();
        h.update(plain.as_bytes());
        format!("{:x}", h.finalize()) == stored
    } else {
        stored == plain
    }
}

pub fn password_needs_upgrade(stored: &str) -> bool {
    !(stored.starts_with("$2a$") || stored.starts_with("$2b$") || stored.starts_with("$2y$"))
}

/// 기존 평문/SHA-512 계정이 정상 로그인하면 파일의 암호만 bcrypt로 교체한다.
pub fn upgrade_user_password_hash(name: &str, plain: &str) -> std::io::Result<()> {
    let path = Path::new("data/user").join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let stored = json
        .get("사용자오브젝트")
        .and_then(|v| v.get("암호"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if !password_verify(stored, plain) || !password_needs_upgrade(stored) {
        return Ok(());
    }
    json["사용자오브젝트"]["암호"] = serde_json::Value::String(password_hash(plain));
    let serialized = serde_json::to_string_pretty(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serialized)?;
    std::fs::rename(tmp, path)
}

/// Rhai가 읽을 수 있는 텍스트 파일을 공개 데이터 디렉토리로 제한한다.
/// 절대 경로, `..` 경로 순회, 허용 디렉토리 밖을 가리키는 심볼릭 링크는 거부한다.
fn read_public_text_file(path: &str) -> String {
    let requested = Path::new(path);
    if requested.is_absolute() {
        return String::new();
    }

    let Ok(canonical) = std::fs::canonicalize(requested) else {
        return String::new();
    };
    let allowed = [Path::new("data/config"), Path::new("data/text")]
        .into_iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .any(|root| canonical.starts_with(root));
    if !allowed || !canonical.is_file() {
        return String::new();
    }
    std::fs::read_to_string(canonical).unwrap_or_default()
}

fn public_text_file_exists(path: &str) -> bool {
    let requested = Path::new(path);
    if requested.is_absolute() {
        return false;
    }
    let Ok(canonical) = std::fs::canonicalize(requested) else {
        return false;
    };
    let allowed = [Path::new("data/config"), Path::new("data/text")]
        .into_iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .any(|root| canonical.starts_with(root));
    allowed && canonical.is_file()
}

/// Register the data-only mob tracking efun. All user-facing messages stay in Rhai.
fn register_mob_tracking_efun(engine: &mut Engine) {
    engine.register_fn(
        "track_mob_in_zone",
        |zone: &str, mob_name: &str| -> rhai::Map {
            let result = crate::world::tracking::find_mob_room(zone, mob_name);
            let mut map = rhai::Map::new();
            map.insert("zone_exists".into(), Dynamic::from(result.zone_exists));
            map.insert(
                "room".into(),
                Dynamic::from(result.room.unwrap_or_default()),
            );
            map
        },
    );
}

/// data/user/{name}.json 에서 사용자오브젝트.암호 값을 읽어 반환. 로그인 검증용.
pub fn load_user_password_hash(name: &str) -> Option<String> {
    let path = format!("data/user/{}.json", name);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let uso = json.get("사용자오브젝트")?.as_object()?;
    let s = uso.get("암호")?.as_str()?;
    Some(s.to_string())
}

/// Value -> serde_json::Value (저장용)
fn value_to_serde_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Number(serde_json::Number::from(0))),
        Value::String(s) => serde_json::Value::String(s.clone()),
    }
}

/// serde_json::Value -> Value (로드용)
fn serde_json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::String(String::new()),
        serde_json::Value::Bool(b) => Value::Int(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Int(0)
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            // 배열을 파이프로 구분된 문자열로 변환 (Rust 내부 형식)
            // Python은 ["skill1", "skill2"] 또는 ["skill1 100 100", "skill2 100 100"] 형식으로 저장
            let s = arr
                .iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join("|");
            Value::String(s)
        }
        serde_json::Value::Object(_) => Value::String(serde_json::to_string(v).unwrap_or_default()),
    }
}

fn update_user_attr_int(name: &str, key: &str, value: i64) -> bool {
    let path = format!("data/user/{name}.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    let Some(attrs) = json
        .get_mut("사용자오브젝트")
        .and_then(|v| v.get_mut("attr"))
        .and_then(|v| v.as_object_mut())
    else {
        return false;
    };
    attrs.insert(key.to_string(), serde_json::Value::Number(value.into()));
    std::fs::write(
        path,
        serde_json::to_string_pretty(&json).unwrap_or_default(),
    )
    .is_ok()
}

fn queue_online_user_attr(body: &mut Body, target: &str, key: &str, value: i64) {
    let json = serde_json::json!([target, key, value]).to_string();
    body.temp_mut()
        .insert(SET_PLAYER_ATTR_REQUEST.to_string(), Value::String(json));
}

/// Body를 data/user/{이름}.json 에 저장. 소지품(objs, inv_stack) 포함.
/// Python `Player.save()`처럼 마지막저장시간을 갱신한다.
pub fn save_body_to_json(body: &mut Body, path: &str) -> bool {
    save_body_to_json_with_timestamp_mode(body, path, true)
}

/// Python `Player.save(False)` 호출용. 쪽지 상태를 저장하지만
/// 기존 `마지막저장시간`은 바꾸지 않는다.
pub(crate) fn save_body_to_json_without_timestamp(body: &mut Body, path: &str) -> bool {
    save_body_to_json_with_timestamp_mode(body, path, false)
}

fn save_body_to_json_with_timestamp_mode(
    body: &mut Body,
    path: &str,
    update_last_saved_at: bool,
) -> bool {
    if std::fs::create_dir_all(Path::new(path).parent().unwrap_or(Path::new("."))).is_err() {
        return false;
    }
    // Object만 조립한 임시 Body라면 먼저 Python loadSkillList/loadSkillUp처럼
    // 속성을 복원한다. 정상 로그인/신규 생성 Body는 이미 초기화되어 있다.
    if !body.skill_state_loaded {
        if body.skill_list.is_empty() && body.skill_map.is_empty() {
            body.load_skill_state_from_attrs();
        } else {
            body.skill_state_loaded = true;
        }
    }
    // Python `Player.save()`는 JSON 직렬화 전에 buildSkillList/buildSkillUp을
    // 호출해 런타임 무공 상태를 객체 속성에 반영한다.
    body.sync_skill_state_to_attrs();
    body.sync_active_skills_to_attrs();
    let save_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0);
    if update_last_saved_at {
        let now = save_time as i64;
        body.object
            .attr
            .insert("마지막저장시간".to_string(), Value::Int(now));
    }

    let mut uso = serde_json::Map::new();
    for (k, v) in &body.object.attr {
        // Python Player.buildAlias(): ["키 명령", ...] 배열을 그대로 저장한다.
        if k == ALIAS_LIST_ATTR {
            let entries = match v {
                Value::String(s) => decode_alias_entries(s),
                _ => Vec::new(),
            };
            let values = entries
                .into_iter()
                .map(|(key, data)| serde_json::Value::String(format!("{} {}", key, data)))
                .collect();
            uso.insert(k.clone(), serde_json::Value::Array(values));
            continue;
        }
        // Python 호환성: 파이프 구분 문자열을 배열로 변환
        if k == "무공숙련도" || k == "무공이름" || k == "비전이름" || k == "방어무공시전"
        {
            if let Value::String(s) = v {
                if !s.is_empty() {
                    // "skill1|skill2" 또는 "skill1 level exp|skill2 level exp" 형식을 배열로 변환
                    let parts: Vec<serde_json::Value> = s
                        .split('|')
                        .map(|p| serde_json::Value::String(p.trim().to_string()))
                        .filter(|p| !p.as_str().map(|s| s.is_empty()).unwrap_or(true))
                        .collect();
                    uso.insert(k.clone(), serde_json::Value::Array(parts));
                    continue;
                }
            }
        }
        uso.insert(k.clone(), value_to_serde_json(v));
    }

    // Python owns one ordered list of individual Item objects. Convert any
    // legacy Rust-only stack before serializing; unknown keys fail preflight
    // rather than disappearing from the save file.
    if inventory_compat::materialize_stacks_for_save(body).is_err() {
        return false;
    }
    let items = inventory_compat::python_inventory_records(body, save_time);

    // Python player.py reads the historical `최고체력` key while newer Rust
    // state commonly uses `최대체력`; emit both aliases so either runtime can
    // load a file saved by the other.
    if !uso.contains_key("최고체력") {
        if let Some(value) = uso.get("최대체력").cloned() {
            uso.insert("최고체력".to_string(), value);
        }
    }
    if !uso.contains_key("최대체력") {
        if let Some(value) = uso.get("최고체력").cloned() {
            uso.insert("최대체력".to_string(), value);
        }
    }
    // Python Body methods index these historical numeric fields directly;
    // never emit a missing field that would become a string default on the
    // Python side.
    for key in [
        "맷집",
        "맷집경험치",
        "명중",
        "회피",
        "필살",
        "운",
        "내공",
        "최고내공",
        "민첩성",
        "나이오름틱",
        "현재경험치",
        "힘경험치",
        "민첩성경험치",
        "특성치",
        "0 성격플킬",
        "1 성격플킬",
        "2 성격플킬",
    ] {
        uso.entry(key.to_string())
            .or_insert_with(|| serde_json::Value::Number(0.into()));
    }
    if !uso.contains_key("민첩성") {
        if let Some(value) = uso.get("민첩").cloned() {
            uso.insert("민첩성".into(), value);
        }
    }
    if !uso.contains_key("민첩") {
        if let Some(value) = uso.get("민첩성").cloned() {
            uso.insert("민첩".into(), value);
        }
    }
    if !uso.contains_key("최고내공") {
        if let Some(value) = uso.get("최대내공").cloned() {
            uso.insert("최고내공".into(), value);
        }
    }

    let mut root = serde_json::Map::new();
    root.insert("사용자오브젝트".to_string(), serde_json::Value::Object(uso));
    root.insert("아이템".to_string(), serde_json::Value::Array(items));

    for (k, v) in &body.memos {
        if let Ok(val) = serde_json::to_value(v) {
            root.insert(k.clone(), val);
        }
    }

    let j = serde_json::Value::Object(root);
    std::fs::write(path, serde_json::to_string_pretty(&j).unwrap_or_default()).is_ok()
}

/// data/user/{이름}.json 에서 Body 복원. attr, objs, inv_stack.
/// 파일 없거나 실패 시 false. 성공 시 true.
pub fn load_body_from_json(body: &mut Body, path: &str) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return false,
    };
    let root = match json.as_object() {
        Some(o) => o,
        None => return false,
    };

    let Some(uso) = root.get("사용자오브젝트").and_then(|v| v.as_object()) else {
        return false;
    };

    {
        body.object.attr.clear();
        for (k, v) in uso {
            if k == ALIAS_LIST_ATTR {
                let entries: Vec<(String, String)> = v
                    .as_array()
                    .map(|values| {
                        let raw =
                            serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string());
                        decode_alias_entries(&raw)
                    })
                    .unwrap_or_else(|| v.as_str().map(decode_alias_entries).unwrap_or_default());
                body.object
                    .attr
                    .insert(k.clone(), Value::String(encode_alias_entries(&entries)));
            } else {
                body.object.attr.insert(k.clone(), serde_json_to_value(v));
            }
        }

        // Python 호환성: 금화/은화를 은전으로 변환
        // 일부 Python JSON은 "금화", "은화" 필드를 사용하지만
        // Rust 내부와 최신 Python은 "은전" 필드를 사용
        let has_gold = uso.contains_key("금화") || uso.contains_key("은화");
        let has_money = uso.contains_key("은전");
        if has_gold && !has_money {
            let gold = body.object.getInt("금화");
            let silver = body.object.getInt("은화");
            // 금화 1개 = 은전 10000개 (Python 규칙)
            let total_money = gold * 10000 + silver;
            body.object.set("은전", total_money);
        }

        // Python 호환성: "현재방" 필드를 "위치"로도 복사
        // Python JSON은 "현재방" 필드를 사용하지만 Rust 내부에서는 "위치"를 사용
        if uso.contains_key("현재방") && !uso.contains_key("위치") {
            let current_room = body.object.getString("현재방");
            if !current_room.is_empty() {
                body.object.set("위치", current_room);
            }
        }

        if !uso.contains_key("최고체력") {
            if let Some(value) = uso.get("최대체력") {
                body.object
                    .attr
                    .insert("최고체력".into(), serde_json_to_value(value));
            }
        }
        if !uso.contains_key("최대체력") {
            if let Some(value) = uso.get("최고체력") {
                body.object
                    .attr
                    .insert("최대체력".into(), serde_json_to_value(value));
            }
        }

        // Python `Player.load()`의 loadSkillList/loadSkillUp 호출과 같은 순서로
        // 객체 속성에서 런타임 무공 목록/숙련도 맵을 복원한다.
        body.load_skill_state_from_attrs();
    }

    inventory_compat::load_python_inventory(body, root);
    // One-way compatibility for files emitted by older Rust builds. Valid
    // counts become individual objects; unknown keys remain quarantined so a
    // later save cannot silently discard them.
    inventory_compat::load_legacy_stacks(body, root);
    // Python calls Body.loadSkills() after Player.load() has restored items.
    // Rebuild persisted defense-skill modifiers after equipment for the same
    // one-time additive result.
    body.load_active_skills_from_attrs();

    body.memos.clear();
    for (k, v) in root.iter() {
        if k.starts_with("메모") {
            if let Some(obj) = v.as_object() {
                let record = MemoRecord {
                    제목: obj
                        .get("제목")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    시간: obj
                        .get("시간")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    작성자: obj
                        .get("작성자")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    내용: obj
                        .get("내용")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                };
                body.memos.insert(k.clone(), record);
            }
        }
    }

    // Python `Player.talkHistory`는 접속마다 빈 배열로 시작하는 런타임
    // 상태이며 Player.save/load 대상이 아니다. 레거시 Rust 저장 파일에
    // `대화기록`이 있어도 다시 불러오지 않는다.
    body.talk_history.clear();

    true
}

/// data/script/{path} 로드. JSON 배열이면 파싱, 아니면 줄 단위. $스크립트호출·무기강화용.
pub(crate) fn load_script_file(path: &str) -> Option<Vec<String>> {
    let p = std::path::Path::new("data/script").join(path);
    let content = std::fs::read_to_string(&p).ok()?;
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(content.trim()) {
        return Some(arr);
    }
    Some(content.lines().map(|s| s.to_string()).collect())
}

/// Create an Object from data/item/{key}.json 아이템정보.
/// Returns None if file missing or invalid; else Some((object, 아이템정보.이름 or key)).
/// world::event::$아이템주기에서 사용. pub(crate).
pub(crate) fn object_from_item_json(key: &str) -> Option<(Arc<Mutex<Object>>, String)> {
    if crate::world::item::is_runtime_deleted(key) {
        return None;
    }
    let path = format!("data/item/{}.json", key);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let info = json.get("아이템정보")?.as_object()?;
    let display_name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string();
    let mut obj = Object::new();
    obj.set("인덱스", key); // item JSON 파일명(확장자 제외). 저장/로드·스택 식별용.
    for (k, v) in info {
        match v {
            serde_json::Value::Null => {}
            serde_json::Value::Bool(b) => {
                obj.set(k, if *b { 1i64 } else { 0i64 });
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    obj.set(k, i);
                } else if let Some(f) = n.as_f64() {
                    obj.set(k, f as i64);
                }
            }
            serde_json::Value::String(s) => {
                obj.set(k, s.as_str());
            }
            serde_json::Value::Array(arr) => {
                if matches!(k.as_str(), "반응이름" | "옵션" | "아이템속성") {
                    // These list-valued item fields are written back to the
                    // Python user JSON. Preserve element boundaries (options
                    // contain spaces) in the object's temporary hashmap.
                    inventory_compat::set_item_json_field(&mut obj, k, v);
                } else {
                    let s = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    obj.set(k, s);
                }
            }
            serde_json::Value::Object(_) => {}
        }
    }
    Some((Arc::new(Mutex::new(obj)), display_name))
}

/// item JSON에서 이름, 반응이름, 판매가격(또는 값), 무게 반환. 구입 가격 계산용.
fn get_item_info(key: &str) -> Option<(String, String, i64, i64)> {
    if crate::world::item::is_runtime_deleted(key) {
        return None;
    }
    let path = format!("data/item/{}.json", key);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let info = json.get("아이템정보")?.as_object()?;
    let name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let rn = info
        .get("반응이름")
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else if let Some(arr) = v.as_array() {
                arr.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                String::new()
            }
        })
        .unwrap_or_default();
    let price = info
        .get("판매가격")
        .or_else(|| info.get("값"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let weight = info.get("무게").and_then(|v| v.as_i64()).unwrap_or(0);
    Some((name, rn, price, weight))
}

/// 소비성 아이템 정보 가져오기 (이름, 체력회복, 내공회복)
/// 종류가 "먹는것"인 경우에만 값을 반환, 아니면 (0, 0, 0)
fn get_consumable_info(key: &str) -> (String, i64, i64) {
    if crate::world::item::is_runtime_deleted(key) {
        return (String::new(), 0, 0);
    }
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (String::new(), 0, 0),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return (String::new(), 0, 0),
    };
    let info = match json.get("아이템정보").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return (String::new(), 0, 0),
    };
    let kind = info.get("종류").and_then(|v| v.as_str()).unwrap_or("");
    if kind != "먹는것" {
        return (String::new(), 0, 0);
    }
    let name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let hp = info.get("체력").and_then(|v| v.as_i64()).unwrap_or(0);
    let mp = info.get("내공").and_then(|v| v.as_i64()).unwrap_or(0);
    (name, hp, mp)
}

/// 아이템 설명1. data/item/{key}.json. 방 바닥 스택 표시용.
fn get_item_desc1(key: &str) -> String {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return String::new(),
    };
    let info = match json.get("아이템정보").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return String::new(),
    };
    info.get("설명1")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// 아이템 인덱스(스택)가 누적 가능한지. 무기/방어구·개별인스턴스 아니면 true.
fn is_stackable(key: &str) -> bool {
    let path = format!("data/item/{}.json", key);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return false,
    };
    let info = match json.get("아이템정보").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return false,
    };
    let kind = info.get("종류").and_then(|v| v.as_str()).unwrap_or("기타");
    if kind == "무기" || kind == "방어구" {
        return false;
    }
    let attrs = info.get("아이템속성");
    if let Some(serde_json::Value::Array(arr)) = attrs {
        for v in arr {
            if v.as_str() == Some("개별인스턴스") {
                return false;
            }
        }
    } else if let Some(serde_json::Value::String(s)) = attrs {
        if s.contains("개별인스턴스") {
            return false;
        }
    }
    true
}

/// 이름 또는 반응이름으로 아이템 인덱스(키) 찾기. data/item/*.json 검색.
fn find_item_key_by_name(name: &str) -> Option<String> {
    if name.is_empty() {
        return None;
    }
    let dir = std::path::Path::new("data/item");
    let read_dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return None,
    };
    for e in read_dir.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let key = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if let Some((iname, rn, _, _)) = get_item_info(&key) {
            if iname == name {
                return Some(key);
            }
            if !rn.is_empty() && rn.split_whitespace().any(|s| s == name) {
                return Some(key);
            }
        }
    }
    None
}

fn item_catalog() -> rhai::Array {
    let mut result = rhai::Array::new();
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Ok(zones) = std::fs::read_dir("data/mob") {
        for zone in zones.flatten() {
            let Ok(files) = std::fs::read_dir(zone.path()) else {
                continue;
            };
            for file in files.flatten() {
                if file.path().extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                let Ok(source) = std::fs::read_to_string(file.path()) else {
                    continue;
                };
                let Ok(root) = serde_json::from_str::<serde_json::Value>(&source) else {
                    continue;
                };
                let Some(info) = root.get("몹정보").and_then(|v| v.as_object()) else {
                    continue;
                };
                let uses = info
                    .get("사용아이템")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(|| {
                        info.get("사용아이템")
                            .and_then(|v| v.as_str())
                            .map(|v| vec![serde_json::Value::String(v.to_string())])
                            .unwrap_or_default()
                    });
                for used in uses {
                    let Some(key) = used.as_str().and_then(|v| v.split_whitespace().next()) else {
                        continue;
                    };
                    let path = std::path::Path::new("data/item").join(format!("{key}.json"));
                    if path.exists() && seen.insert(path.clone()) {
                        paths.push(path);
                    }
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir("data/item") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && seen.insert(path.clone())
            {
                paths.push(path);
            }
        }
    }
    for path in paths {
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(index) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        let Some(info) = json.get("아이템정보").and_then(|value| value.as_object()) else {
            continue;
        };
        let mut item = rhai::Map::new();
        // Python's legacy Item.Items contains this historical alias after
        // mob initialization; preserve it in administrator catalog output.
        let catalog_index = if index == "현철지륜" {
            "현지륜"
        } else {
            index
        };
        item.insert("index".into(), Dynamic::from(catalog_index.to_string()));
        item.insert(
            "name".into(),
            Dynamic::from(
                info.get("이름")
                    .and_then(|value| value.as_str())
                    .unwrap_or(index)
                    .to_string(),
            ),
        );
        item.insert(
            "type".into(),
            Dynamic::from(
                info.get("종류")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string(),
            ),
        );
        item.insert(
            "ansi".into(),
            Dynamic::from(
                info.get("안시")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string(),
            ),
        );
        item.insert(
            "user".into(),
            Dynamic::from(
                info.get("사용자")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
                    .to_string(),
            ),
        );
        result.push(Dynamic::from(item));
    }
    result
}

/// Global reference to the current object being accessed
/// This is set by the driver before calling script functions
static mut CURRENT_OBJECT: Option<Object> = None;

/// Set the current object context (called by driver)
pub fn set_current_object(obj: Object) {
    unsafe {
        CURRENT_OBJECT = Some(obj);
    }
}

/// Get the current object context
pub fn get_current_object() -> Option<Object> {
    unsafe { CURRENT_OBJECT.clone() }
}

/// Create a new Rhai engine with all API functions registered
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // ============================================================
    // UTILITY FUNCTIONS
    // ============================================================

    engine.register_fn("random", |min: i64, max: i64| -> i64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    });

    engine.register_fn("abs", |n: i64| -> i64 { n.abs() });

    // String utilities
    engine.register_fn("contains", |s: &str, pattern: &str| -> bool {
        s.contains(pattern)
    });
    engine.register_fn("starts_with", |s: &str, pattern: &str| -> bool {
        s.starts_with(pattern)
    });
    engine.register_fn("ends_with", |s: &str, pattern: &str| -> bool {
        s.ends_with(pattern)
    });
    engine.register_fn("trim", |s: &str| -> String { s.trim().to_string() });
    engine.register_fn("substring", |s: &str, start: i64, end: i64| -> String {
        let chars: Vec<char> = s.chars().collect();
        let start_idx = if start < 0 { 0 } else { start as usize };
        let end_idx = if end < 0 { chars.len() } else { end as usize };
        if start_idx >= chars.len() {
            return String::new();
        }
        let end_idx = end_idx.min(chars.len());
        chars[start_idx..end_idx].iter().collect()
    });
    engine.register_fn("length", |s: &str| -> i64 { s.chars().count() as i64 });

    // Array utilities
    engine.register_fn("len", |arr: &mut rhai::Array| -> i64 { arr.len() as i64 });
    engine.register_fn("length", |arr: &mut rhai::Array| -> i64 {
        arr.len() as i64
    });
    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });

    // ============================================================
    // ANSI COLOR CONVERSION
    // ============================================================

    engine.register_fn("ansi", |msg: &str, conv: bool| -> String {
        ansi_convert(msg, conv)
    });

    // ============================================================
    // KOREAN PARTICLE HELPERS
    // ============================================================

    engine.register_fn("han_iga", |name: &str| -> String { han_iga(name) });
    engine.register_fn("han_eul", |name: &str| -> String { han_eul(name) });
    engine.register_fn("han_eun", |name: &str| -> String { han_eun(name) });
    engine.register_fn("han_wa", |name: &str| -> String { han_wa(name) });
    engine.register_fn("han_uro", |name: &str| -> String { han_uro(name) });

    // 무림별호 전역 레지스트리. 문구/포맷은 Rhai 명령에서 처리한다.
    engine.register_fn("nickname_exists", crate::world::nickname::nickname_exists);
    engine.register_fn("nickname_owner", crate::world::nickname::nickname_owner);
    engine.register_fn("nickname_reserve", crate::world::nickname::nickname_reserve);
    engine.register_fn("nickname_release", crate::world::nickname::nickname_release);
    engine.register_fn("nickname_save", crate::world::nickname::nickname_save);
    register_mob_tracking_efun(&mut engine);

    // 이름 ANSI(노랑), 문자열 치환, 정수→문자. format_room_objs.rhai 등에서 사용.
    engine.register_fn("name_ansi", |s: &str| -> String {
        format!("\x1b[33m{}\x1b[37m", s)
    });
    engine.register_fn("replace_str", |s: &str, from: &str, to: &str| -> String {
        s.replace(from, to)
    });
    engine.register_fn("int_to_str", |i: i64| -> String { i.to_string() });

    // ============================================================
    // OUTPUT FUNCTIONS
    // ============================================================

    engine.register_fn("print", |s: &str| {
        tracing::debug!(message = s, "Rhai print");
    });
    engine.register_fn("debug", |s: &str| {
        debug!("[SCRIPT] {}", s);
    });

    // Player action functions
    // Note: These now need access to the scope's _output variable
    // For now, we'll use a simpler approach - just print and return
    engine.register_fn("send_line", |player_data: &mut rhai::Map, msg: &str| {
        // Store in player_data for now - scripts can use get_attr/set_attr
        let output = player_data.get_mut("_output");
        if let Some(arr) = output {
            if let Some(mut vec) = arr.clone().try_cast::<rhai::Array>() {
                let msg_string = msg.to_string();
                let msg_dynamic = rhai::Dynamic::from(msg_string);
                vec.push(msg_dynamic);
                player_data.insert("_output".into(), rhai::Dynamic::from(vec));
            }
        }
    });

    engine.register_fn("send_room", |player_data: &mut rhai::Map, msg: &str| {
        let output = player_data.get_mut("_output");
        if let Some(arr) = output {
            if let Some(mut vec) = arr.clone().try_cast::<rhai::Array>() {
                let msg_string = msg.to_string();
                let msg_dynamic = rhai::Dynamic::from(msg_string);
                vec.push(msg_dynamic);
                player_data.insert("_output".into(), rhai::Dynamic::from(vec));
            }
        }
    });

    // ============================================================
    // ATTRIBUTE ACCESS (Player/Object data)
    // ============================================================

    engine.register_fn(
        "get_attr",
        |player_data: &mut rhai::Map, key: &str| -> Dynamic {
            player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
        },
    );

    engine.register_fn(
        "set_attr",
        |player_data: &mut rhai::Map, key: &str, value: Dynamic| {
            player_data.insert(key.to_string().into(), value);
        },
    );

    engine.register_fn("get_int", |player_data: &mut rhai::Map, key: &str| -> i64 {
        player_data
            .get(key)
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    engine.register_fn(
        "get_string",
        |player_data: &mut rhai::Map, key: &str| -> String {
            player_data
                .get(key)
                .and_then(|v| {
                    if v.is_string() {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        },
    );

    // ============================================================
    // DIFFICULTY ZONE FUNCTIONS
    // ============================================================

    // Get difficulty level from zone name (e.g., "낙양성1" -> 1, "낙양성" -> 0)
    engine.register_fn("get_difficulty_from_zone", |zone: &str| -> i64 {
        use crate::world::difficulty_from_zone;
        difficulty_from_zone(zone) as i64
    });

    // Get base zone name (e.g., "낙양성1" -> "낙양성")
    engine.register_fn("get_base_zone_name", |zone: &str| -> String {
        use crate::world::base_zone_name;
        base_zone_name(zone).to_string()
    });

    // Get minimum level required for a difficulty zone
    engine.register_fn("get_min_level_for_difficulty", |difficulty: i64| -> i64 {
        use crate::world::DifficultyConfig;
        DifficultyConfig::min_level_for_difficulty(difficulty as u8)
    });

    // Get difficulty config for a level
    engine.register_fn("get_difficulty_config", |difficulty: i64| -> rhai::Map {
        use crate::world::DifficultyConfig;
        let config = DifficultyConfig::get(difficulty as u8);
        let mut map = rhai::Map::new();
        map.insert(
            "level_bonus".into(),
            rhai::Dynamic::from(config.level_bonus),
        );
        map.insert(
            "hp_multiplier".into(),
            rhai::Dynamic::from(config.hp_multiplier as i64),
        );
        map.insert(
            "str_multiplier".into(),
            rhai::Dynamic::from(config.str_multiplier as i64),
        );
        map.insert(
            "arm_multiplier".into(),
            rhai::Dynamic::from(config.arm_multiplier as i64),
        );
        map.insert(
            "agi_multiplier".into(),
            rhai::Dynamic::from(config.agi_multiplier as i64),
        );
        map.insert(
            "exp_multiplier".into(),
            rhai::Dynamic::from(config.exp_multiplier as i64),
        );
        map.insert(
            "gold_multiplier".into(),
            rhai::Dynamic::from(config.gold_multiplier as i64),
        );
        map
    });

    // ============================================================
    // STRING MANIPULATION HELPERS
    // ============================================================

    engine.register_fn("fill_space", |width: i64, s: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!("{}{:width$}", s, " ", width = (width - len) as usize)
        }
    });
    engine.register_fn("fill_space_euc_kr", fill_space_euc_kr);
    engine.register_fn("fill_space_front_euc_kr", fill_space_front_euc_kr);
    engine.register_fn("get_murim_config_list", get_murim_main_config_list);
    engine.register_fn("get_murim_config", get_murim_config_value);

    engine.register_fn(
        "room_player_exists",
        |ob: &mut rhai::Map, target: &str| -> bool {
            let location = ob
                .get("위치")
                .or_else(|| ob.get("현재방"))
                .map(|value| value.to_string())
                .unwrap_or_default();
            let Some((zone, room)) = location
                .split_once(':')
                .or_else(|| location.split_once('/'))
            else {
                return false;
            };
            let Ok(world) = get_world_state().try_read() else {
                return false;
            };
            world
                .get_players_in_room(zone, room)
                .iter()
                .any(|name| name == target)
        },
    );

    engine.register_fn("strip_ansi", |s: &str| -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if let Some(&'[') = chars.peek() {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    });

    engine.register_fn("to_int", |s: &str| -> i64 { s.trim().parse().unwrap_or(0) });
    engine.register_fn("python_get_int", python_get_int);

    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });

    // ============================================================
    // OBJECT QUERY FUNCTIONS (EFUNS)
    // ============================================================

    // environment(obj) - Get parent object
    engine.register_fn("environment", |obj_data: &mut rhai::Map| -> Dynamic {
        // In full implementation, would return the env object
        // For now, return the environment name
        obj_data.get("env").cloned().unwrap_or(Dynamic::UNIT)
    });

    // all_inventory(obj) - Get all child objects
    engine.register_fn("all_inventory", |obj_data: &mut rhai::Map| -> Dynamic {
        obj_data.get("objs").cloned().unwrap_or(Dynamic::UNIT)
    });

    // present(env, name) - Find object by name in environment
    // Searches through env["objs"] array for matching name/반응이름/설명1
    engine.register_fn("present", |env: &mut rhai::Map, name: &str| -> Dynamic {
        use rhai::Dynamic;

        // Get objs array from environment
        if let Some(objs_value) = env.get("objs") {
            if let Some(objs) = objs_value.clone().try_cast::<rhai::Array>() {
                for obj in &objs {
                    if let Some(obj_map) = obj.clone().try_cast::<rhai::Map>() {
                        // Check 이름
                        if let Some(name_value) = obj_map.get("이름") {
                            if let Some(obj_name) = name_value.clone().try_cast::<String>() {
                                if obj_name == name {
                                    return obj.clone();
                                }
                            }
                        }
                        // Check 반응이름 (array)
                        if let Some(reactions_value) = obj_map.get("반응이름") {
                            if let Some(reactions) =
                                reactions_value.clone().try_cast::<rhai::Array>()
                            {
                                for reaction in &reactions {
                                    if let Some(reaction_str) =
                                        reaction.clone().try_cast::<String>()
                                    {
                                        if reaction_str == name {
                                            return obj.clone();
                                        }
                                    }
                                }
                            }
                        }
                        // Check 설명1 (display name)
                        if let Some(desc_value) = obj_map.get("설명1") {
                            if let Some(desc1) = desc_value.clone().try_cast::<String>() {
                                if desc1 == name {
                                    return obj.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
        Dynamic::UNIT
    });

    // ============================================================
    // DATA LOADING FUNCTIONS (EFUNS)
    // ============================================================

    engine.register_fn("load_json", |path: &str| -> Dynamic {
        // Load JSON data from data/config/
        let full_path = format!("data/config/{}.json", path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                // Parse JSON (basic implementation)
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        // Convert to Rhai Dynamic
                        json_value_to_dynamic(value)
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });
    engine.register_fn("get_item_catalog", item_catalog);

    engine.register_fn("get_item_data", |name: &str| -> Dynamic {
        let full_path = format!("data/item/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        // Extract 아이템정보
                        if let Some(obj) = value.get("아이템정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                }
            }
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_mob_data", |name: &str| -> Dynamic {
        // Support both "zone:filename" and plain "filename" formats
        let full_path = if name.contains(':') {
            let parts: Vec<&str> = name.splitn(2, ':').collect();
            if parts.len() == 2 {
                format!("data/mob/{}/{}.json", parts[0], parts[1])
            } else {
                format!("data/mob/{}.json", name)
            }
        } else {
            format!("data/mob/{}.json", name)
        };
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("몹정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_room_data", |name: &str| -> Dynamic {
        // name format: "zone:room"
        let full_path = format!("data/map/{}.json", name.replace(":", "/"));
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("맵정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_skill_data", |name: &str| -> Dynamic {
        // Load skill.json and find the skill
        match std::fs::read_to_string("data/config/skill.json") {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(skills) = value.as_object() {
                        if let Some(skill) = skills.get(name) {
                            json_value_to_dynamic(skill.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    // ============================================================
    // SKILL UTILITY FUNCTIONS
    // ============================================================

    engine.register_fn("get_skill_defense_head", |name: &str| -> String {
        crate::world::skill::get_skill_defense_head(name)
    });

    engine.register_fn("get_skill_type", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| {
                    match s.skill_type {
                        crate::world::skill::SkillType::Combat => "전투",
                        crate::world::skill::SkillType::Defense => "방어",
                        crate::world::skill::SkillType::Internal => "내공",
                        crate::world::skill::SkillType::Other => "기타",
                    }
                    .to_string()
                })
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("get_skill_mp_cost", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.mp_cost).unwrap_or(0)
        } else {
            0
        }
    });

    engine.register_fn("get_skill_hp_cost", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.hp_cost).unwrap_or(0)
        } else {
            0
        }
    });

    engine.register_fn("get_skill_probability", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.probability).unwrap_or(100)
        } else {
            100
        }
    });

    engine.register_fn("get_skill_hit_rate", |name: &str| -> i64 {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.hit_rate as i64).unwrap_or(0)
        } else {
            0
        }
    });

    engine.register_fn("get_skill_mugong_script", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.mugong_script.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("get_skill_fail_message", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.fail_message.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("is_all_attack_skill", |name: &str| -> bool {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache.get(name).map(|s| s.is_all_attack()).unwrap_or(false)
        } else {
            false
        }
    });

    engine.register_fn("get_skill_category", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.category.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    engine.register_fn("get_skill_anti_type", |name: &str| -> String {
        if let Ok(cache) = crate::world::skill::get_skill_cache().read() {
            cache
                .get(name)
                .map(|s| s.get_anti_type().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    // Calculate normal attacks from remaining dex (after skill execution)
    // Returns array: [attack_count, remainder_dex]
    engine.register_fn("calculate_normal_attacks", |dex: i64| -> rhai::Array {
        let (count, remainder) = crate::world::skill::calculate_normal_attacks(dex);
        vec![Dynamic::from(count), Dynamic::from(remainder)]
    });

    // Note: 비전 (Secret Skill) functions are available directly via Body methods
    // and commands in vision.rs. Script efuns for 비전 removed since they require
    // a player cache system not yet implemented.

    engine
}

/// 바닥 아이템 이름별 묶음 포맷. 파이썬 viewMapData nStr. format_room_objs.rhai와 동일 로직을 Rust로 구현.
/// grouped: (name, count, desc1) 들. 공통: 봐/이동 시 방 표시.
pub fn format_room_objs_display(grouped: Vec<(String, usize, String)>) -> String {
    if grouped.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::with_capacity(grouped.len());
    for (name, count, desc1) in grouped {
        let name_a = format!("\x1b[36m{}\x1b[37m", name);
        let line = if desc1.is_empty() {
            if count == 1 {
                format!("○ {}{} 바닥에 떨어져 있다.", name_a, han_iga(&name))
            } else {
                format!("○ {} {}개가 바닥에 떨어져 있다.", name_a, count)
            }
        } else if count == 1 {
            desc1.replace("$아이템$", &name_a)
        } else {
            desc1.replace("$아이템$", &format!("{} {}개", name_a, count))
        };
        lines.push(line);
    }
    format!("\r\n{}", lines.join("\r\n"))
}

/// 바닥 아이템을 이름별로 묶어 format_room_objs_display로 포맷. room_objs + room_inv_stack 병합.
pub fn build_room_objs_grouped(
    room_objs: &[std::sync::Arc<std::sync::Mutex<Object>>],
    room_inv_stack: &std::collections::HashMap<String, i64>,
) -> String {
    let mut map: HashMap<String, (usize, String)> = HashMap::new();
    for arc in room_objs {
        if let Ok(o) = arc.lock() {
            let name = o.getName();
            let desc1 = o.getString("설명1");
            map.entry(name)
                .and_modify(|e| e.0 += 1)
                .or_insert((1, desc1));
        }
    }
    for (key, cnt) in room_inv_stack {
        if *cnt <= 0 {
            continue;
        }
        if let Some((name, _, _, _)) = get_item_info(key) {
            let desc1 = get_item_desc1(key);
            map.entry(name)
                .and_modify(|e| e.0 += *cnt as usize)
                .or_insert((*cnt as usize, desc1));
        }
    }
    let grouped: Vec<(String, usize, String)> = map
        .into_iter()
        .map(|(name, (count, desc1))| (name, count, desc1))
        .collect();
    format_room_objs_display(grouped)
}

/// Python `Box.viewShort()` lines for the installation Boxes created by
/// `Room.create()`.  `viewMapData()` shows these before mobs and floor items.
pub(super) fn installed_box_short_views(zone: &str, room: &str) -> Vec<String> {
    box_commands::installed_boxes_for_room(zone, room)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|box_object| {
            let box_value = box_object.lock().ok()?;
            let name = box_value.getName();
            // Room.create retains a Box even when its backing JSON is
            // missing. Python consequently renders its empty viewShort as
            // ` (0/0)`; do not silently discard that observable object.
            Some(format!(
                "{} ({}/{})",
                name,
                box_value.objs.len(),
                box_value.getInt("보관수량")
            ))
        })
        .collect()
}

/// 방 전체 문자열(헤더·설명·출구·몹·바닥아이템·다른유저). view_map_data efun 및 show_room_to_player_with_world와 동일 포맷.
/// 오류 시 Err((코드, zone, room)): "no_position"|"room_error"|"unknown_room". 성공 시 Ok(문자열).
/// other_player_descs: 같은 방의 다른 접속 유저 getDesc.
pub fn build_room_lines(
    player_name: &str,
    other_player_descs: &[String],
) -> Result<String, (String, String, String)> {
    let world = get_world_state().read().unwrap();
    let pos = match world.get_player_position(player_name) {
        Some(p) => p.clone(),
        None => return Err(("no_position".to_string(), String::new(), "0".to_string())),
    };
    if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room) {
        let room_ref = match room.read() {
            Ok(r) => r,
            Err(_) => return Err(("room_error".to_string(), String::new(), "0".to_string())),
        };
        let room_name_formatted = format_room_header(&room_ref.display_name);
        let exits_str = format_exits_long(&room_ref);
        // Python viewMapData traverses Room.objs including ACT_DEATH corpses;
        // get_mobs_in_room intentionally filters to living targets only.
        let mobs = world.mob_cache.get_all_mobs_in_room(&pos.zone, &pos.room);
        let mob_str = if mobs.is_empty() {
            String::new()
        } else {
            let mut mob_msgs = Vec::new();
            for mob in mobs {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    // Python viewMapData mob display logic:
                    // 몹종류 7: skip
                    if mob_data.mob_type == 7 {
                        continue;
                    }
                    // ACT_REGEN (3): skip
                    // ACT_REST (4): "이/가 흐트러진 진기를 추스리고 있습니다."
                    // ACT_STAND (0): getDesc1()
                    // ACT_FIGHT (1): 방어상태머리말 + "이/가 목숨을 건 사투를 벌이고 있습니다."
                    // ACT_DEATH (2): "의 싸늘한 시체가 있습니다."

                    if mob.act == 3 {
                        // ACT_REGEN - skip
                        continue;
                    }

                    if mob.act == 4 {
                        // ACT_REST
                        let suffix = if crate::hangul::ends_with_consonant(&mob_data.name) {
                            format!("{} 흐트러진 진기를 추스리고 있습니다.", mob_data.name)
                        } else {
                            format!("{}가 흐트러진 진기를 추스리고 있습니다.", mob_data.name)
                        };
                        mob_msgs.push(suffix);
                    } else if mob.act == 0 {
                        // ACT_STAND - show desc1
                        if !mob_data.desc1.is_empty() {
                            mob_msgs.push(mob_data.desc1.clone());
                        }
                    } else if mob.act == 1 {
                        // ACT_FIGHT
                        let mut prefix = String::new();
                        for skill_name in &mob.skills {
                            let defense_head = crate::data::get_skill_defense_head(skill_name);
                            if !defense_head.is_empty() {
                                prefix.push_str(&defense_head);
                                prefix.push(' ');
                            }
                        }
                        let suffix = if crate::hangul::ends_with_consonant(&mob_data.name) {
                            format!("{}목숨을 건 사투를 벌이고 있습니다.", mob_data.name)
                        } else {
                            format!("{}가 목숨을 건 사투를 벌이고 있습니다.", mob_data.name)
                        };
                        mob_msgs.push(format!("{}{}", prefix, suffix));
                    } else if mob.act == 2 {
                        // ACT_DEATH
                        #[allow(clippy::if_same_then_else)]
                        let suffix = if crate::hangul::ends_with_consonant(&mob_data.name) {
                            format!("{}의 싸늘한 시체가 있습니다.", mob_data.name)
                        } else {
                            format!("{}의 싸늘한 시체가 있습니다.", mob_data.name)
                        };
                        mob_msgs.push(suffix);
                    } else {
                        // Other states - show desc1
                        if !mob_data.desc1.is_empty() {
                            mob_msgs.push(mob_data.desc1.clone());
                        }
                    }
                }
            }
            if mob_msgs.is_empty() {
                String::new()
            } else {
                format!("\r\n{}", mob_msgs.join("\r\n"))
            }
        };
        let room_objs = world.get_room_objs(&pos.zone, &pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, &pos.room);
        let item_str = build_room_objs_grouped(&room_objs, &room_stack);
        let installed_boxes = installed_box_short_views(&pos.zone, &pos.room);
        let mut out = String::new();
        out.push_str("\r\n");
        out.push_str(&room_name_formatted);
        out.push_str("\r\n\r\n");
        out.push_str(&room_ref.description.join("\r\n"));
        out.push_str("\r\n");
        out.push_str(&exits_str);
        if !installed_boxes.is_empty() {
            out.push_str("\r\n☞ ");
            out.push_str(&installed_boxes.join("    "));
        }
        if !mob_str.is_empty() {
            out.push_str(&mob_str);
            out.push_str("\r\n");
        }
        if !item_str.is_empty() {
            out.push_str(&item_str);
            out.push_str("\r\n");
        }
        for s in other_player_descs {
            out.push_str(s);
            out.push_str("\r\n");
        }
        Ok(out)
    } else {
        Err((
            "unknown_room".to_string(),
            pos.zone.clone(),
            pos.room.clone(),
        ))
    }
}

/// data/item/{key}.json에서 아이템정보.계층, 아이템정보.이름 반환. 없으면 None.
fn get_item_slot_name(key: &str) -> Option<(String, String)> {
    let path = format!("data/item/{}.json", key);
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let info = json.get("아이템정보")?.as_object()?;
    let slot = info
        .get("계층")
        .and_then(|v| v.as_str())
        .unwrap_or("기타")
        .to_string();
    let name = info
        .get("이름")
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string();
    Some((slot, name))
}

/// 파이썬 objs/player.view(ob). 나/다른 플레이어 상세: 이름·성격·배우자·나이·소속·직위·장비·HP.
fn player_view(body: &Body, _myself: bool) -> Vec<String> {
    let mut lines = vec!["━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string()];
    let m = body.get_string("무림별호");
    let m = if m.is_empty() {
        "무명객".to_string()
    } else {
        m
    };
    let c = body.get_string("성격");
    let c = if c.is_empty() {
        "없음".to_string()
    } else {
        c
    };
    let s = format!("◆ 이  름 ▷ 『{}』 {}", m, body.get_name());
    let c2 = format!("◆ 성격 ▷ 『{}』", c);
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}  {}\x1b[0m\x1b[37m\x1b[40m",
        s, c2
    ));
    let ba = body.get_string("배우자");
    let ba = if ba.is_empty() {
        "미혼".to_string()
    } else {
        ba
    };
    let age = body.get_int("나이");
    let sex = body.get_string("성별");
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 배우자 ▷ 『{}』  ◆ 나이 ▷ {}살({})\x1b[0m\x1b[37m\x1b[40m",
        ba, age, sex
    ));
    let so = body.get_string("소속");
    if !so.is_empty() {
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m■ 소  속 ▷ 『{}』\x1b[0m\x1b[37m\x1b[40m",
            so
        ));
        let jw = body.get_string("직위");
        let r = body.get_string("방파별호");
        let jw_line = if r.is_empty() {
            format!("■ 직  위 ▷ 『{}』", jw)
        } else {
            format!("■ 직  위 ▷ 『{}({})』", jw, r)
        };
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m",
            jw_line
        ));
    }
    lines.push("──────────────────────────────".to_string());
    let mut item_str = String::new();
    for &lv in ITEM_EQUIP_LEVELS.iter() {
        for arc in &body.object.objs {
            if let Ok(o) = arc.lock() {
                if !o.getBool("inUse") {
                    continue;
                }
                let sl = o.getString("계층");
                if sl != lv {
                    continue;
                }
                let disp = get_item_level_display(lv);
                item_str.push_str(&format!("[{}] \x1b[36m{}\x1b[37m\r\n", disp, o.getName()));
            }
        }
    }
    if item_str.is_empty() {
        lines.push("\x1b[36m☞ 혈혈단신 맨몸으로 강호를 주유중입니다.\x1b[37m".to_string());
    } else {
        lines.push(item_str.trim_end().to_string());
    }
    lines.push("──────────────────────────────".to_string());
    lines.push(format!("★ {}", body.get_hp_string()));
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// 파이썬 objs/mob.view(ob). 살아있는 몹: 이름·설명2·사용아이템·HP·HPbar. 시체: 이름의 시체.
fn mob_view(mob: &MobInstance, data: &RawMobData) -> Vec<String> {
    let mut lines = vec!["━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string()];
    if !mob.alive {
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}의 시체\x1b[0m\x1b[37m\x1b[40m",
            data.name
        ));
        lines.push("──────────────────────────────".to_string());
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        return lines;
    }
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
        data.name
    ));
    lines.push("──────────────────────────────".to_string());
    if data.desc2.is_empty() {
        // no desc2 lines
    } else {
        for d in &data.desc2 {
            lines.push(d.clone());
        }
    }
    lines.push("──────────────────────────────".to_string());
    let mut use_lines: Vec<(String, String)> = Vec::new();
    for &lv in ITEM_EQUIP_LEVELS.iter() {
        for (key, _cnt, _prob, _scale) in &data.use_items {
            if let Some((slot, iname)) = get_item_slot_name(key) {
                if slot == lv {
                    let disp = get_item_level_display(lv);
                    use_lines.push((disp.to_string(), iname));
                    break;
                }
            }
        }
    }
    for (disp, iname) in &use_lines {
        lines.push(format!("[{}] \x1b[36m{}\x1b[37m", disp, iname));
    }
    if !use_lines.is_empty() {
        lines.push("──────────────────────────────".to_string());
    }
    let max_hp = if mob.max_hp <= 0 { 1 } else { mob.max_hp };
    let pct = (mob.hp * 100) / max_hp;
    lines.push(format!(
        "★ {} ({}%)",
        get_hp_bar_string(mob.hp, mob.max_hp),
        pct
    ));
    lines.push(format!(
        "☆ {} ({})",
        get_hp_bar_string(mob.hp, mob.max_hp),
        pct
    ));
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

fn admin_mob_view(viewer: &Body, mob: &MobInstance, data: &RawMobData) -> Vec<String> {
    let mut lines = mob_view(mob, data);
    if viewer.get_int("관리자등급") < 1000 {
        return lines;
    }
    let index = mob
        .mob_key
        .split_once(':')
        .map(|(_, index)| index)
        .unwrap_or(mob.mob_key.as_str());
    lines.insert(0, format!("Index : {index}"));
    lines.push(format!(
        "│ [레  벨] {:>15}  │ [상  태] {:>15}  │",
        mob.level, mob.act
    ));
    lines.push(format!(
        "│ [체  력] {:>15}  │ [내  공] {:>15}  │ ",
        format!("{}/{}", mob.hp, mob.max_hp),
        format!("{}/{}", mob.mp, mob.max_mp)
    ));
    lines.push(format!(
        "│ [맷  집] {:>15}  │ [민  첩] {:>15}  │",
        mob.arm, mob.agility
    ));
    let skill = mob
        .active_attack_skill
        .as_ref()
        .map(|skill| skill.name.as_str())
        .unwrap_or("----------");
    lines.push(format!(
        "│ [  힘  ] {:>15}  │ [스  킬] {:>15}  │",
        mob.strength, skill
    ));
    if mob.difficulty >= 1 {
        lines.push(format!(
            "│ [命  中] {:>15}  │ [回  避] {:>15}  │",
            data.hit, data.miss
        ));
        lines.push(format!(
            "│ [必  殺] {:>15}  │ [  運  ] {:>15}  │",
            data.critical, data.luck
        ));
    }
    if !mob.targets.is_empty() {
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        lines.push("★ 목표 대상".to_string());
        let entries = mob
            .targets
            .iter()
            .enumerate()
            .map(|(index, target)| format!(" [{:02}] {:<10}    ", index + 1, target))
            .collect::<Vec<_>>();
        lines.push(entries.chunks(3).map(|row| row.concat()).collect::<Vec<_>>().join("\r\n"));
    }
    if !data.skills.is_empty() {
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        lines.push("★ 공격 스킬 목록".to_string());
        let entries = data
            .skills
            .iter()
            .enumerate()
            .map(|(index, (name, _, _))| format!(" [{:02}] {:<10} ", index + 1, name))
            .collect::<Vec<_>>();
        lines.push(entries.chunks(3).map(|row| row.concat()).collect::<Vec<_>>().join("\r\n"));
    }
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// 아이템 상세 보기. 파이썬 objs/item.view(ob). find_target/look_at_target에서 사용.
fn item_view(obj: &Arc<Mutex<Object>>) -> Vec<String> {
    let o = obj.lock().unwrap();
    let name_a = o.getNameA();
    let mut lines = vec![
        "━━━━━━━━━━━━━━━━━━━━━".to_string(),
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
            o.getName()
        ),
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 종류 ▷ {}\x1b[0m\x1b[37m\x1b[40m",
            o.getString("종류")
        ),
        "─────────────────────".to_string(),
    ];
    let desc2 = o.getString("설명2");
    let desc = if desc2.is_empty() {
        o.getString("설명1").replace("$아이템$", &name_a)
    } else {
        desc2.replace("$아이템$", &name_a)
    };
    for line in desc.lines() {
        lines.push(line.to_string());
    }
    let opt = o.getString("옵션");
    if !opt.is_empty() {
        lines.push(opt);
    }
    lines.push("━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// [대상] 봐: 나|findObjInven|find_in_room(아이템,몹,플레이어,출구) 검색 후 타입별 표시.
/// returns (viewer_lines, Option<(target_player_name, msg_to_target)>)
fn look_at_target(
    body: &Body,
    world: &WorldState,
    viewer_name: &str,
    target_line: &str,
    other_player_descs: &HashMap<String, String>,
) -> (Vec<String>, Option<(String, String)>) {
    let not_found = (
        vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네".to_string()],
        None,
    );

    if target_line.trim() == "나" {
        return (player_view(body, true), None);
    }

    let (mut name, mut order) = CommandParser::parse_name_order(target_line);
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(o) = name.parse::<usize>() {
            if o >= 1 {
                name = String::new();
                order = o;
            }
        }
    }

    if !name.is_empty() {
        if let Some(obj) = body.object.findObjInven(&name, order) {
            return (item_view(&obj), None);
        }
    }

    let pos = match world.get_player_position(viewer_name) {
        Some(p) => p,
        None => return (vec!["위치 정보가 없습니다.".to_string()], None),
    };
    let zone = pos.zone.as_str();
    let room_s = pos.room.as_str();
    let mut c = 0usize;

    if name.is_empty() && order >= 1 {
        for mob in world.mob_cache.get_mobs_in_room(zone, room_s) {
            if !mob.alive {
                continue;
            }
            if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                if data.mob_type == 7 {
                    continue;
                }
                c += 1;
                if c == order {
                    return (admin_mob_view(body, mob, data), None);
                }
            }
        }
        return not_found;
    }

    if name == "시체" {
        for mob in world.mob_cache.get_mobs_in_room(zone, room_s) {
            if !mob.alive {
                c += 1;
                if c == order {
                    if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                        return (admin_mob_view(body, mob, data), None);
                    }
                }
            }
        }
        return not_found;
    }

    // 파이썬 room.findObjName: 이름==name, name in 반응이름, 또는 alias.find(name)==0(alias가 name으로 시작)
    let room_objs = world.get_room_objs(zone, room_s);
    let ordered_objects = world.get_room_object_order(zone, room_s);
    if !ordered_objects.is_empty() {
        let mut ordered_count = 0usize;
        for object in ordered_objects {
            match object {
                RoomObjectRef::FloorItem(pointer) => {
                    let Some(arc) = room_objs
                        .iter()
                        .find(|arc| Arc::as_ptr(arc) as usize == pointer)
                    else {
                        continue;
                    };
                    let Ok(item) = arc.lock() else { continue };
                    let aliases = item.getString("반응이름");
                    let matches = item.getName() == name
                        || aliases
                            .split_whitespace()
                            .any(|alias| alias == name || alias.starts_with(name.as_str()));
                    drop(item);
                    if matches {
                        ordered_count += 1;
                        if ordered_count == order {
                            return (item_view(arc), None);
                        }
                    }
                }
                RoomObjectRef::Mob(instance_id) => {
                    let Some(mob) = world
                        .mob_cache
                        .get_all_mobs_in_room(zone, room_s)
                        .into_iter()
                        .find(|mob| mob.instance_id == instance_id)
                    else {
                        continue;
                    };
                    let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                        continue;
                    };
                    if !mob.alive || mob.act == 3 {
                        continue;
                    }
                    let matches = data.name == name
                        || data.name.starts_with(name.as_str())
                        || data
                            .reaction_names
                            .iter()
                            .any(|alias| alias == &name || alias.starts_with(name.as_str()));
                    if matches {
                        ordered_count += 1;
                        if ordered_count == order {
                            return (admin_mob_view(body, mob, data), None);
                        }
                    }
                }
                RoomObjectRef::Player(player_name) => {
                    let Some(desc) = other_player_descs.get(&player_name) else {
                        continue;
                    };
                    if player_name == name || player_name.starts_with(name.as_str()) {
                        ordered_count += 1;
                        if ordered_count == order {
                            let msg = format!("{} 당신을 살펴봅니다.", body.han_iga());
                            return (vec![desc.clone()], Some((player_name, msg)));
                        }
                    }
                }
                RoomObjectRef::SummonedUser(id) => {
                    let Some(user) = world
                        .summoned_users()
                        .iter()
                        .find(|user| user.id == id)
                    else {
                        continue;
                    };
                    let player_name = user.body.get_name();
                    if player_name == name || player_name.starts_with(name.as_str()) {
                        ordered_count += 1;
                        if ordered_count == order {
                            return (vec![user.body.get_desc_for_look(false)], None);
                        }
                    }
                }
                RoomObjectRef::InstalledBox(ordinal) => {
                    let Some(boxes) = box_commands::installed_boxes_for_room(zone, room_s) else {
                        continue;
                    };
                    let Some(box_object) = boxes.get(ordinal) else {
                        continue;
                    };
                    let Ok(box_value) = box_object.lock() else {
                        continue;
                    };
                    let aliases = box_value.getString("반응이름");
                    let matches = box_value.getName() == name
                        || aliases
                            .split_whitespace()
                            .any(|alias| alias == name || alias.starts_with(name.as_str()));
                    drop(box_value);
                    if matches {
                        ordered_count += 1;
                        if ordered_count == order {
                            return (item_view(box_object), None);
                        }
                    }
                }
                RoomObjectRef::Box(_) => {}
            }
        }
    }

    for arc in &room_objs {
        let ok = {
            if let Ok(o) = arc.lock() {
                let n = o.getName();
                let reac = o.getString("반응이름");
                n == name
                    || reac
                        .split_whitespace()
                        .any(|s| s == name || s.starts_with(name.as_str()))
            } else {
                false
            }
        };
        if ok {
            c += 1;
            if c == order {
                return (item_view(arc), None);
            }
        }
    }

    // 파이썬: 이름==name, name in 반응이름, 또는 reaction.find(name)==0
    for mob in world.mob_cache.get_mobs_in_room(zone, room_s) {
        if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
            let ok = data.name == name
                || data.name.starts_with(name.as_str())
                || data
                    .reaction_names
                    .iter()
                    .any(|r| r.as_str() == name || r.starts_with(name.as_str()));
            if ok {
                c += 1;
                if c == order {
                    return (admin_mob_view(body, mob, data), None);
                }
            }
        }
    }

    // 파이썬: 이름 정확 or 대상 이름이 입력으로 시작(멍멍 → 멍멍이)
    // Python Room.objs is traversed in insertion-at-front order.  Do not
    // iterate the description HashMap here: its order is deliberately
    // unspecified and made same-name player lookup nondeterministic.
    let room_players = world.get_players_in_room(zone, room_s);
    for pname in room_players.iter().rev() {
        let Some(desc) = other_player_descs.get(pname) else {
            continue;
        };
        if *pname == name || pname.starts_with(name.as_str()) {
            c += 1;
            if c == order {
                let msg = format!("{} 당신을 살펴봅니다.", body.han_iga());
                return (vec![desc.clone()], Some((pname.clone(), msg)));
            }
        }
    }

    if let Some(dir) = Direction::from_korean(&name) {
        if let Some(room_arc) = world.room_cache.get_room_cached(zone, room_s) {
            if let Ok(room_guard) = room_arc.read() {
                if room_guard.get_exit(dir).is_some() {
                    c += 1;
                    if c == order {
                        return (
                            vec![format!("{}쪽으로 갈 수 있습니다.", dir.korean_name())],
                            None,
                        );
                    }
                }
            }
        }
    }

    not_found
}

/// Create a Rhai engine with output collection support
///
/// This creates an engine where `send_line` and `send_room` write to a shared output collector.
pub fn create_engine_with_output(output_collector: Arc<Mutex<Vec<String>>>) -> Engine {
    let mut engine = Engine::new();

    // ============================================================
    // UTILITY FUNCTIONS
    // ============================================================

    engine.register_fn("random", |min: i64, max: i64| -> i64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    });

    engine.register_fn("abs", |n: i64| -> i64 { n.abs() });

    // String utilities
    engine.register_fn("contains", |s: &str, pattern: &str| -> bool {
        s.contains(pattern)
    });
    engine.register_fn("starts_with", |s: &str, pattern: &str| -> bool {
        s.starts_with(pattern)
    });
    engine.register_fn("ends_with", |s: &str, pattern: &str| -> bool {
        s.ends_with(pattern)
    });
    engine.register_fn("trim", |s: &str| -> String { s.trim().to_string() });
    engine.register_fn("substring", |s: &str, start: i64, end: i64| -> String {
        let chars: Vec<char> = s.chars().collect();
        let start_idx = if start < 0 { 0 } else { start as usize };
        let end_idx = if end < 0 { chars.len() } else { end as usize };
        if start_idx >= chars.len() {
            return String::new();
        }
        let end_idx = end_idx.min(chars.len());
        chars[start_idx..end_idx].iter().collect()
    });
    engine.register_fn("length", |s: &str| -> i64 { s.chars().count() as i64 });

    // Array utilities
    engine.register_fn("len", |arr: &mut rhai::Array| -> i64 { arr.len() as i64 });
    engine.register_fn("length", |arr: &mut rhai::Array| -> i64 {
        arr.len() as i64
    });
    engine.register_fn("join", |arr: &mut rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });

    // ============================================================
    // ANSI COLOR CONVERSION
    // ============================================================

    engine.register_fn("ansi", |msg: &str, conv: bool| -> String {
        ansi_convert(msg, conv)
    });

    // ============================================================
    // KOREAN PARTICLE HELPERS
    // ============================================================

    engine.register_fn("han_iga", |name: &str| -> String { han_iga(name) });
    engine.register_fn("han_eul", |name: &str| -> String { han_eul(name) });
    engine.register_fn("han_eun", |name: &str| -> String { han_eun(name) });
    engine.register_fn("han_wa", |name: &str| -> String { han_wa(name) });
    engine.register_fn("han_uro", |name: &str| -> String { han_uro(name) });

    // 무림별호 전역 레지스트리. create_engine_with_body_and_output도 이 엔진을 기반으로 한다.
    engine.register_fn("nickname_exists", crate::world::nickname::nickname_exists);
    engine.register_fn("nickname_owner", crate::world::nickname::nickname_owner);
    engine.register_fn("nickname_reserve", crate::world::nickname::nickname_reserve);
    engine.register_fn("nickname_release", crate::world::nickname::nickname_release);
    engine.register_fn("nickname_save", crate::world::nickname::nickname_save);
    register_mob_tracking_efun(&mut engine);

    // ============================================================
    // OUTPUT FUNCTIONS (with collection)
    // ============================================================

    let oc = output_collector.clone();
    engine.register_fn(
        "send_line",
        move |_player_data: &mut rhai::Map, msg: &str| {
            match oc.lock() {
                Ok(mut output) => {
                    output.push(msg.to_string());
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Rhai output collector lock failed");
                }
            }
        },
    );

    let oc = output_collector.clone();
    engine.register_fn(
        "send_room",
        move |_player_data: &mut rhai::Map, msg: &str| {
            if let Ok(mut output) = oc.lock() {
                output.push(msg.to_string());
            }
        },
    );

    engine.register_fn("print", |s: &str| {
        tracing::debug!(message = s, "Rhai print");
    });
    engine.register_fn("debug", |s: &str| {
        debug!("[SCRIPT] {}", s);
    });

    // ============================================================
    // ATTRIBUTE ACCESS
    // ============================================================

    engine.register_fn(
        "get_attr",
        |player_data: &mut rhai::Map, key: &str| -> Dynamic {
            player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
        },
    );

    engine.register_fn(
        "set_attr",
        |player_data: &mut rhai::Map, key: &str, value: Dynamic| {
            player_data.insert(key.to_string().into(), value);
        },
    );

    engine.register_fn("get_int", |player_data: &mut rhai::Map, key: &str| -> i64 {
        player_data
            .get(key)
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    engine.register_fn(
        "get_string",
        |player_data: &mut rhai::Map, key: &str| -> String {
            player_data
                .get(key)
                .and_then(|v| {
                    if v.is_string() {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        },
    );

    // ============================================================
    // STRING MANIPULATION HELPERS
    // ============================================================

    engine.register_fn("fill_space", |width: i64, s: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!("{}{:width$}", s, " ", width = (width - len) as usize)
        }
    });
    engine.register_fn("fill_space_euc_kr", fill_space_euc_kr);
    engine.register_fn("fill_space_front_euc_kr", fill_space_front_euc_kr);
    engine.register_fn("get_murim_config_list", get_murim_main_config_list);
    engine.register_fn("get_murim_config", get_murim_config_value);

    engine.register_fn("strip_ansi", |s: &str| -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if let Some(&'[') = chars.peek() {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    });

    engine.register_fn("pad_start", |s: &str, width: i64, fill: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!(
                "{}{:width$}",
                fill.repeat((width - len) as usize),
                s,
                width = width as usize
            )
        }
    });

    // repeat function for Rhai scripts
    engine.register_fn("repeat", |s: &str, count: i64| -> String {
        s.repeat(count.max(0) as usize)
    });

    engine.register_fn("to_int", |s: &str| -> i64 { s.trim().parse().unwrap_or(0) });
    engine.register_fn("python_get_int", python_get_int);

    engine.register_fn("int_to_str", |i: i64| -> String { i.to_string() });

    engine.register_fn("split", |s: &str, sep: &str| -> rhai::Array {
        s.split(sep)
            .map(|x| rhai::Dynamic::from(x.to_string()))
            .collect()
    });

    // Parse "2검" to (order, name). Returns [order: i64, name: string] as Array.
    // Python getNameOrder: "1" 전부 숫자면 name="1" 유지(아이템 "1" 찾음). "2.검"이면 order=2, name=".검".
    engine.register_fn("parse_order_name", |s: &str| -> rhai::Array {
        let s = s.trim();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0usize;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let (order, name) = if i > 0 {
            let num_str: String = chars[..i].iter().collect();
            let n: i64 = num_str.parse().unwrap_or(1);
            let rest: String = chars[i..].iter().collect();
            // 전부 숫자("1","2")면 name=원문. 그래야 "1 버려"가 아이템 "1"을 찾고 없으면 실패.
            let name = if rest.is_empty() { s.to_string() } else { rest };
            (n.max(1), name)
        } else {
            (1i64, s.to_string())
        };
        vec![rhai::Dynamic::from(order), rhai::Dynamic::from(name)]
    });

    // parse_name_order(s): "2.검" -> [name, order]. 주다 등. CommandParser::parse_name_order.
    engine.register_fn("parse_name_order", |s: &str| -> rhai::Array {
        let (name, order) = CommandParser::parse_name_order(s);
        vec![rhai::Dynamic::from(name), rhai::Dynamic::from(order as i64)]
    });

    // ============================================================
    // COMMAND HELPER EFUNS (반복 패턴)
    // ============================================================

    engine.register_fn("is_empty", |s: &str| -> bool { s.trim().is_empty() });

    // is_unit(value) - Check if a Dynamic value is unit (empty/not found)
    engine.register_fn("is_unit", |value: rhai::Dynamic| -> bool {
        value.is_unit()
    });

    // int_to_str(value) - Convert integer to string (handles both int and string inputs)
    engine.register_fn("int_to_str", |value: rhai::Dynamic| -> String {
        if value.is_int() {
            value.as_int().unwrap_or(0).to_string()
        } else if value.is_string() {
            value.into_string().unwrap_or_default()
        } else {
            "".to_string()
        }
    });

    engine.register_fn("ob_name", |ob: &mut rhai::Map| -> String {
        ob.get("이름")
            .and_then(|v| {
                if v.is_string() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    engine.register_fn("ob_iga", |ob: &mut rhai::Map| -> String {
        let n = ob
            .get("이름")
            .and_then(|v| {
                if v.is_string() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        han_iga(&n)
    });

    engine.register_fn("line_args", |line: &str| -> rhai::Array {
        line.split_whitespace()
            .map(|x| rhai::Dynamic::from(x.to_string()))
            .collect()
    });
    engine.register_fn("first_arg", |line: &str| -> String {
        line.split_whitespace().next().unwrap_or("").to_string()
    });

    // require_arg: 기능만. line이 비었으면 false. usage/오류 메시지는 Rhai에서 send_line.
    engine.register_fn("require_arg", |_ob: &mut rhai::Map, line: &str| -> bool {
        !line.trim().is_empty()
    });

    // require_admin: 기능만. 관리자등급 < min_level 이면 false. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn(
        "require_admin",
        |ob: &mut rhai::Map, min_level: i64| -> bool {
            let adm = ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0i64);
            adm >= min_level
        },
    );

    // Text formatting functions for item actions
    engine.register_fn(
        "format_item_action_self",
        |name: &str, action: &str, count: i64| -> String {
            if count > 1 {
                format!(
                    "당신이 \x1b[36m{}\x1b[37m {}개를 {}",
                    name, count, action
                )
            } else {
                format!(
                    "당신이 \x1b[36m{}{}\x1b[37m {}",
                    name,
                    han_eul(name),
                    action
                )
            }
        },
    );
    engine.register_fn(
        "format_item_action_target",
        |name: &str, target: &str, action: &str, count: i64| -> String {
            if count > 1 {
                format!(
                    "{} {} {}개를 {} {}.",
                    name,
                    action,
                    count,
                    han_eun(target),
                    target
                )
            } else {
                format!("{} {} {} {}.", name, han_eun(target), target, action)
            }
        },
    );

    // Note: format_hp_bar, format_time, format_item_name, format_mob_name are now implemented
    // in lib/format.rhai for hot-reload capability. They are loaded as library scripts.

    // ============================================================
    // DATA LOADING (get_item_data, get_mob_data, get_room_data, get_skill_data)
    // ============================================================

    // Python `Item.Items` catalog used by administrator search commands.
    engine.register_fn("get_item_catalog", item_catalog);

    // Python `ob.cmdList` preserves the command files whose CmdObj.level is
    // exactly 1000.  Keep this discovery in an efun; Rhai owns only layout.
    engine.register_fn(
        "get_python_commands_at_level",
        |level: i64| -> rhai::Array {
            let mut result = rhai::Array::new();
            let Ok(entries) = std::fs::read_dir("cmds") else {
                return result;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|x| x.to_str()) != Some("py") {
                    continue;
                }
                let Ok(source) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let marker = format!("level = {level}");
                let marker_compact = format!("level={level}");
                if source.lines().any(|line| {
                    let code = line.trim_start();
                    !code.starts_with('#')
                        && (code.contains(&marker) || code.contains(&marker_compact))
                }) {
                    if let Some(name) = path.file_stem().and_then(|x| x.to_str()) {
                        result.push(Dynamic::from(name.to_string()));
                    }
                }
            }
            result
        },
    );

    engine.register_fn("get_item_data", |name: &str| -> Dynamic {
        let full_path = format!("data/item/{}.json", name);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("아이템정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_mob_data", |name: &str| -> Dynamic {
        // Support both "zone:filename" and plain "filename" formats
        let full_path = if name.contains(':') {
            let parts: Vec<&str> = name.splitn(2, ':').collect();
            if parts.len() == 2 {
                format!("data/mob/{}/{}.json", parts[0], parts[1])
            } else {
                format!("data/mob/{}.json", name)
            }
        } else {
            format!("data/mob/{}.json", name)
        };
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("몹정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_room_data", |name: &str| -> Dynamic {
        let full_path = format!("data/map/{}.json", name.replace(":", "/"));
        match std::fs::read_to_string(&full_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.get("맵정보") {
                        json_value_to_dynamic(obj.clone())
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("get_skill_data", |name: &str| -> Dynamic {
        match std::fs::read_to_string("data/config/skill.json") {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(skills) = value.as_object() {
                        if let Some(skill) = skills.get(name) {
                            json_value_to_dynamic(skill.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    } else {
                        Dynamic::UNIT
                    }
                }
                Err(_) => Dynamic::UNIT,
            },
            Err(_) => Dynamic::UNIT,
        }
    });

    // Python 몹찾기: live instance가 아니라 Mob.Mobs 템플릿을 등록 순서로 검색한다.
    engine.register_fn("find_mobs", |search_term: &str| -> rhai::Array {
        let world = match crate::world::get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Array::new(),
        };
        let numeric_type = python_get_int(search_term);
        let mut arr = rhai::Array::new();
        for (key, data) in world.mob_cache.ordered_mob_templates() {
            let matched = if numeric_type != 0 {
                data.mob_type == numeric_type
            } else {
                data.name.contains(search_term)
            };
            if !matched {
                continue;
            }
            let index = key.split_once(':').map(|(_, index)| index).unwrap_or(key);
            let location = data
                .attributes
                .get("위치")
                .map(python_json_repr)
                .unwrap_or_else(|| {
                    let values = data
                        .locations
                        .iter()
                        .map(|room| format!("'{}'", room))
                        .collect::<Vec<_>>();
                    format!("[{}]", values.join(", "))
                });
            let mut m = rhai::Map::new();
            m.insert("name".into(), Dynamic::from(data.name.clone()));
            m.insert("index".into(), Dynamic::from(index.to_string()));
            m.insert("location".into(), Dynamic::from(location));
            arr.push(Dynamic::from(m));
        }
        arr
    });

    // get_help(topic): data/config/help.json의 {"도움말": { "도움말": [...], ... }}에서
    // topic이 "도움말"이면 ["도움말"]["도움말"], 아니면 ["도움말"][topic] 배열을 "\r\n"으로 이어서 반환. 없으면 "".
    // Python HELP[line]은 비어 있지 않은 line을 trim하지 않는다.
    engine.register_fn("get_help", |topic: &str| -> String {
        let key = topic;
        let content = match std::fs::read_to_string("data/config/help.json") {
            Ok(c) => c,
            Err(_) => return String::new(),
        };
        let root: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        let outer = match root.get("도움말") {
            Some(o) => o,
            None => return String::new(),
        };
        let arr = match outer.get(key).and_then(|v| v.as_array()) {
            Some(a) => a,
            None => return String::new(),
        };
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("\r\n")
    });

    // get_timestamp(): Unix timestamp (초). 값값 등.
    engine.register_fn("get_timestamp", || -> i64 {
        chrono::Utc::now().timestamp()
    });

    // read_text_file(path): 공개 데이터(config/text) 안의 텍스트만 읽는다.
    engine.register_fn("read_text_file", read_public_text_file);
    engine.register_fn("text_file_exists", public_text_file_exists);

    // ============================================================
    // PLAYER DATA ACCESS
    // ============================================================

    engine.register_fn(
        "get_player_data",
        |player_data: &mut rhai::Map, key: &str| -> Dynamic {
            player_data.get(key).cloned().unwrap_or(Dynamic::UNIT)
        },
    );

    // ============================================================
    // MATH FUNCTIONS
    // ============================================================

    engine.register_fn("min", |a: i64, b: i64| -> i64 { a.min(b) });
    engine.register_fn("max", |a: i64, b: i64| -> i64 { a.max(b) });

    engine
}

/// Create a Rhai engine with output collection and item efuns (item_create, item_drop, item_get, item_destroy).
/// Used by script commands that need to modify body inventory and room floor.
/// get_other_players_desc: (exclude_name) -> 같은 방 다른 유저 getDesc 목록. 봐 시 사용, None이면 빈 목록.
/// get_other_players_map: () -> (이름→getDesc). 봐 find_target에서 사용, None이면 빈 맵.
#[allow(clippy::too_many_arguments)]
pub fn create_engine_with_body_and_output(
    body: &mut Body,
    output_collector: Arc<Mutex<Vec<String>>>,
    get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
    get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
    special_collector: Arc<Mutex<Option<CommandResult>>>,
    user_sends: Arc<Mutex<Vec<(String, String)>>>,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
    script_name: Option<&str>,
    global_data: Option<SharedGlobalData>,
) -> Engine {
    let oc = output_collector.clone();
    let mut engine = create_engine_with_output(output_collector);
    let body_ptr = body as *mut Body;
    let spec = special_collector.clone();

    // Python `cmds/업데이트.py` owns every user-visible branch and
    // message. Rust exposes only the corresponding cache/hot-reload operation.
    crate::command::commands::update::register_update_efun(
        &mut engine,
        body_ptr,
        global_data.clone(),
    );

    // Python `cmds/시전.py`의 대상 조회 및 상태 전이는 전용 데이터/로직
    // efun으로 제공한다. 사용자에게 보이는 문구와 ANSI는 `시전.rhai`에 둔다.
    cast::register_cast_efuns(&mut engine, body_ptr);
    anger::register_anger_efuns(&mut engine, body_ptr);
    admin_combat::register_admin_combat_efuns(&mut engine, body_ptr);

    // Python `넣어.py`/`꺼내.py` and `Box` ordered-child state.  Rust
    // exposes only selection/transfer/persistence data; Rhai owns all text.
    box_commands::register_box_command_efuns(&mut engine, body_ptr);

    // Python `쳐.py`/`도망.py`의 방 대상 조회와 전투·이동 상태 전이만
    // 제공한다. 모든 사용자 문구와 ANSI는 각 Rhai 명령에 둔다.
    combat_commands::register_combat_command_efuns(
        &mut engine,
        body_ptr,
        call_out_scheduler.clone(),
    );
    search_body::register_search_body_efun(&mut engine, body_ptr);

    // Python `버려.py`의 인벤토리 순회/방 수량/ONEITEM 상태 전이만
    // 제공한다. 집계 결과의 문구·ANSI·조사는 `버려.rhai`가 만든다.
    drop_item::register_drop_item_efuns(&mut engine, body_ptr);

    // Python `cmds/귀환.py`의 검사/위치 전이만 제공한다. 명령 문구와
    // 같은 방 알림은 hot-reload되는 `귀환.rhai`가 결정한다.
    return_home::register_return_home_efun(&mut engine, body_ptr);

    // Python `Player.parse_command` one-word exits and the normal
    // enterRoom/exitRoom state transition. Visible layout stays in the private
    // hot-reloaded `cmds/__movement.rhai` handler.
    movement::register_movement_efuns(&mut engine, body_ptr);

    // Python follower/Party object relationships are installed as an ordered,
    // connection-scoped snapshot by the network layer. Rhai authors all text;
    // these efuns only request state transitions and opaque deliveries.
    party::register_party_efuns(&mut engine, body_ptr);

    // Python `Player.parse_command` handles the one-word `끝`/`종료`
    // branch after checking Body.isMovable().  Rust exposes only the state
    // predicate/action; all text is supplied by the Rhai command.
    let body_ptr_can_leave = body_ptr;
    engine.register_fn("can_leave_murim", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &*body_ptr_can_leave };
        body.is_movable()
    });

    let spec_disconnect = spec.clone();
    engine.register_fn(
        "request_connection_close",
        move |_ob: &mut rhai::Map, message: &str| {
            if let Ok(mut special) = spec_disconnect.lock() {
                *special = Some(CommandResult::Disconnect(message.to_string()));
            }
        },
    );

    let spec_internal = spec.clone();
    engine.register_fn("internal_not_handled", move || {
        if let Ok(mut special) = spec_internal.lock() {
            *special = Some(CommandResult::InternalNotHandled);
        }
    });

    // `리부팅.py` has no success output.  The network layer applies the
    // loaded-room updates before acting on this stop request.
    let spec_reboot = spec.clone();
    engine.register_fn("request_reboot", move |_ob: &mut rhai::Map| {
        if let Ok(mut special) = spec_reboot.lock() {
            *special = Some(CommandResult::Reboot);
        }
    });

    engine.register_fn(
        "get_bool",
        |player_data: &mut rhai::Map, key: &str| -> bool {
            player_data
                .get(key)
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false)
        },
    );

    engine.register_fn(
        "item_create",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &mut *body_ptr };
            if let Some((arc, name)) = object_from_item_json(key) {
                body.object.append(arc);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
                name
            } else {
                String::new()
            }
        },
    );

    engine.register_fn(
        "item_drop",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            if name.is_empty() {
                return 0; // 빈 name이 "".contains("")로 전부 매칭되는 것 방지
            }
            let body = unsafe { &mut *body_ptr };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            let mut w = get_world_state().write().unwrap();
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return 0,
            };
            // 스택 아이템: inv_stack에서 빼서 room_inv_stack으로
            if let Some(ref key) = find_item_key_by_name(name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    let drop_cnt = (count as i64).min(have).max(0);
                    if drop_cnt > 0 {
                        let should_remove = {
                            let r = body.object.inv_stack.get_mut(key).unwrap();
                            *r -= drop_cnt;
                            *r <= 0
                        };
                        if should_remove {
                            body.object.inv_stack.remove(key);
                        }
                        let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                        *room_stack.entry(key.clone()).or_insert(0) += drop_cnt;
                        drop(w);
                        let path = format!("data/user/{}.json", body.get_name());
                        let _ = save_body_to_json(body, &path);
                        return drop_cnt;
                    }
                }
            }
            // 비스택: objs에서 제거해 room_objs로
            let mut n = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let ok = o.getName() == name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name));
                    if !ok || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함") {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "버리지못함") {
                        continue;
                    }
                    n += 1;
                    if n < order {
                        continue;
                    }
                    drop(o);
                    to_remove.push(obj.clone());
                    if to_remove.len() >= count {
                        break;
                    }
                }
            }
            let dropped = to_remove.len();
            if dropped == 0 {
                return 0;
            }
            {
                let room_objs = w.get_room_objs_mut(&zone, &room);
                for arc in &to_remove {
                    body.object.remove(arc);
                    room_objs.push(arc.clone());
                }
            }
            for arc in &to_remove {
                w.record_floor_item(&zone, &room, arc);
            }
            if dropped > 0 {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            dropped as i64
        },
    );

    engine.register_fn(
        "item_get",
        move |_ob: &mut rhai::Map, name: &str, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            let count = count.clamp(1, 100) as usize;
            let mut w = get_world_state().write().unwrap();
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return 0,
            };
            let mut taken = 0usize;
            // 스택: room_inv_stack에서 가져와 body.inv_stack에
            if let Some(ref key) = find_item_key_by_name(name) {
                if is_stackable(key) {
                    let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                    let have = *room_stack.get(key).unwrap_or(&0);
                    let take_cnt = (count as i64).min(have).max(0) as usize;
                    if take_cnt > 0 {
                        let should_remove = {
                            let r = room_stack.get_mut(key).unwrap();
                            *r -= take_cnt as i64;
                            *r <= 0
                        };
                        if should_remove {
                            room_stack.remove(key);
                        }
                        *body.object.inv_stack.entry(key.clone()).or_insert(0) += take_cnt as i64;
                        taken += take_cnt;
                    }
                }
            }
            // 바닥 Object에서 추가 (비스택 또는 예전 드랍)
            let room_list = w.get_room_objs_mut(&zone, &room);
            let mut i = 0;
            while i < room_list.len() && taken < count {
                let matches = {
                    let o = room_list[i].lock().unwrap();
                    o.getName() == name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name))
                };
                if matches {
                    let arc = room_list.remove(i);
                    // Python 가져.py calls ob.insert(obj): every acquired
                    // object is prepended, preserving identity.
                    body.object.objs.insert(0, arc);
                    taken += 1;
                } else {
                    i += 1;
                }
            }
            if taken > 0 {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            taken as i64
        },
    );

    let body_ptr_get_all = body_ptr;
    engine.register_fn(
        "item_get_all",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &mut *body_ptr_get_all };
            let mut world = get_world_state().write().unwrap();
            let Some(position) = world.get_player_position(&body.get_name()).cloned() else {
                return rhai::Array::new();
            };
            let max_items = get_murim_config_int("사용자아이템갯수").max(0) as usize;
            let max_weight = body.get_str().saturating_mul(10);
            let floor = world.get_room_objs_mut(&position.zone, &position.room);
            let mut groups: Vec<(String, i64)> = Vec::new();
            let mut index = 0usize;
            while index < floor.len() {
                let (name, weight) = match floor[index].lock() {
                    Ok(item) => (item.getName(), item.getInt("무게")),
                    Err(_) => {
                        index += 1;
                        continue;
                    }
                };
                // Python checks the current totals before each insertion and
                // skips overweight objects while preserving later candidates.
                if body.get_item_weight().saturating_add(weight) > max_weight {
                    index += 1;
                    continue;
                }
                if body.get_item_count() > max_items {
                    break;
                }
                let item = floor.remove(index);
                body.object.objs.insert(0, item);
                if let Some((_, count)) = groups.iter_mut().find(|(group, _)| group == &name) {
                    *count += 1;
                } else {
                    groups.push((name, 1));
                }
            }
            if groups.is_empty() {
                return rhai::Array::new();
            }
            drop(world);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            groups
                .into_iter()
                .map(|(name, count)| {
                    let mut item = rhai::Map::new();
                    item.insert("name".into(), Dynamic::from(name));
                    item.insert("count".into(), Dynamic::from(count));
                    Dynamic::from(item)
                })
                .collect()
        },
    );

    engine.register_fn(
        "item_destroy",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            // 스택: inv_stack에서 제거
            if order == 1 {
                if let Some(ref key) = find_item_key_by_name(name) {
                    if is_stackable(key) {
                        let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                        let destroy_cnt = (count as i64).clamp(0, have);
                        if destroy_cnt > 0 {
                            let should_remove = {
                                let r = body.object.inv_stack.get_mut(key).unwrap();
                                *r -= destroy_cnt;
                                *r <= 0
                            };
                            if should_remove {
                                body.object.inv_stack.remove(key);
                            }
                            let path = format!("data/user/{}.json", body.get_name());
                            let _ = save_body_to_json(body, &path);
                            return destroy_cnt;
                        }
                    }
                }
            }
            // 비스택: objs에서 제거
            let mut n = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let ok = o.getName() == name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name));
                    if !ok || o.getBool("inUse") {
                        continue;
                    }
                    n += 1;
                    if n < order {
                        continue;
                    }
                    drop(o);
                    to_remove.push(obj.clone());
                    if to_remove.len() >= count {
                        break;
                    }
                }
            }
            let len = to_remove.len();
            for arc in to_remove {
                body.object.remove(&arc);
            }
            if len > 0 {
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            len as i64
        },
    );
    let body_ptr_destroy_detail = body_ptr;
    engine.register_fn(
        "item_destroy_detail",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> rhai::Map {
            destroy_inventory_for_command(
                unsafe { &mut *body_ptr_destroy_detail },
                name,
                order,
                count,
                false,
            )
        },
    );

    // Python `분해.py`: merchant-gated decomposition of optioned weapons/armor.
    // The script owns all messages; this efun performs only the ordered state change.
    let body_ptr_decompose = body_ptr;
    engine.register_fn("decompose_all_items", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &mut *body_ptr_decompose };
        let result = |status: &str, items: rhai::Array, shards: i64| {
            let mut result = rhai::Map::new();
            result.insert("status".into(), Dynamic::from(status.to_string()));
            result.insert("items".into(), Dynamic::from(items));
            result.insert("shards".into(), Dynamic::from(shards));
            Dynamic::from(result)
        };
        let Some((zone, room)) = current_body_position(body) else {
            return result("no_merchant", rhai::Array::new(), 0);
        };
        let merchant_buys = get_world_state()
            .read()
            .ok()
            .and_then(|world| {
                let ordered = world.get_room_object_order(&zone, &room);
                let all = world.mob_cache.get_all_mobs_in_room(&zone, &room);
                let mut ids = ordered
                    .into_iter()
                    .filter_map(|object| match object {
                        crate::world::RoomObjectRef::Mob(id) => Some(id),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if ids.is_empty() {
                    ids.extend(all.iter().map(|mob| mob.instance_id));
                }
                ids.into_iter().find_map(|id| {
                    let mob = all.iter().find(|mob| mob.instance_id == id)?;
                    let data = world.get_mob_data(&mob.mob_key)?;
                    let nonempty = |key: &str| {
                        data.attributes.get(key).is_some_and(|value| match value {
                            serde_json::Value::String(value) => !value.is_empty(),
                            serde_json::Value::Array(value) => !value.is_empty(),
                            serde_json::Value::Null => false,
                            _ => true,
                        })
                    };
                    (nonempty("물건판매") || nonempty("물건구입"))
                        .then(|| nonempty("물건구입"))
                })
            })
            .unwrap_or(false);
        if !merchant_buys {
            return result("no_merchant", rhai::Array::new(), 0);
        }
        let mut shards = 0i64;
        let mut remove = Vec::new();
        let mut names = rhai::Array::new();
        for arc in &body.object.objs {
            let Ok(item) = arc.lock() else {
                continue;
            };
            if item.getBool("inUse")
                || item.checkAttr("아이템속성", "출력안함")
                || item.checkAttr("아이템속성", "팔지못함")
            {
                continue;
            }
            let kind = item.getString("종류");
            if kind != "방어구" && kind != "무기" {
                continue;
            }
            let Some(options) = item.get_option() else {
                continue;
            };
            if options.is_empty() {
                continue;
            }
            if options.len() >= 4 {
                shards += 1;
            }
            shards += 1;
            remove.push(arc.clone());
            let mut shown = rhai::Map::new();
            shown.insert("이름".into(), Dynamic::from(item.getName()));
            shown.insert("안시".into(), Dynamic::from(item.getString("안시")));
            names.push(Dynamic::from(shown));
        }
        for arc in remove {
            body.object.remove(&arc);
        }
        for _ in 0..shards {
            if let Some((arc, _)) = object_from_item_json("강철조각") {
                body.object.append(arc);
            }
        }
        if shards > 0 {
            let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
        }
        if names.is_empty() {
            result("empty", names, 0)
        } else {
            result("ok", names, shards)
        }
    });

    // item_destroy_busha: like item_destroy but skips 부수지못함. Returns -1 if first candidate has it.
    engine.register_fn(
        "item_destroy_busha",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            let mut n = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            let mut hit_unbreakable = false;
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let ok = o.getName() == name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(name));
                    if !ok || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함") {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "부수지못함") {
                        n += 1;
                        if n >= order && to_remove.is_empty() {
                            hit_unbreakable = true;
                        }
                        continue;
                    }
                    n += 1;
                    if n < order {
                        continue;
                    }
                    drop(o);
                    to_remove.push(obj.clone());
                    if to_remove.len() >= count {
                        break;
                    }
                }
            }
            if hit_unbreakable {
                return -1;
            }
            let len = to_remove.len();
            for arc in to_remove {
                body.object.remove(&arc);
            }
            len as i64
        },
    );
    let body_ptr_break_detail = body_ptr;
    engine.register_fn(
        "item_break_detail",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> rhai::Map {
            destroy_inventory_for_command(
                unsafe { &mut *body_ptr_break_detail },
                name,
                order,
                count,
                true,
            )
        },
    );

    // list_inventory(ob): body.object.objs를 순회해 [이름, 갯수] 쌍 배열 반환. inUse/출력안함(비관리자) 제외.
    let body_ptr_inv = body_ptr;
    engine.register_fn("list_inventory", move |ob: &mut rhai::Map| -> rhai::Array {
        let admin = ob
            .get("관리자등급")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0i64);
        let body = unsafe { &*body_ptr_inv };
        let mut map: HashMap<String, i64> = HashMap::new();
        for arc in &body.object.objs {
            if let Ok(o) = arc.lock() {
                if o.getBool("inUse") {
                    continue;
                }
                if o.checkAttr("아이템속성", "출력안함") && admin < 1000 {
                    continue;
                }
                let name = o.getName();
                *map.entry(name).or_insert(0) += 1;
            }
        }
        for (key, cnt) in &body.object.inv_stack {
            if let Some((name, _, _, _)) = get_item_info(key) {
                *map.entry(name).or_insert(0) += cnt;
            }
        }
        let mut arr = rhai::Array::new();
        for (k, v) in map {
            let pair = vec![rhai::Dynamic::from(k), rhai::Dynamic::from(v)];
            arr.push(rhai::Dynamic::from(pair));
        }
        arr
    });

    let body_ptr_saved_set = body_ptr;
    engine.register_fn(
        "get_saved_set_items",
        move |_ob: &mut rhai::Map, set_name: &str| -> rhai::Array {
            let body = unsafe { &*body_ptr_saved_set };
            let mut orders = HashMap::<String, i64>::new();
            let mut result = rhai::Array::new();
            for arc in &body.object.objs {
                if let Ok(item) = arc.lock() {
                    if item.getBool("inUse") {
                        continue;
                    }
                    let name = item.getName();
                    let order = orders.entry(name.clone()).or_insert(0);
                    *order += 1;
                    let kind = item.getString("종류");
                    if kind != "방어구" && kind != "무기" {
                        continue;
                    }
                    if !reaction_names(&item.getString("반응이름"))
                        .iter()
                        .any(|name| name == set_name)
                    {
                        continue;
                    }
                    let mut saved = rhai::Map::new();
                    saved.insert("name".into(), Dynamic::from(name));
                    saved.insert("order".into(), Dynamic::from(*order));
                    result.push(Dynamic::from(saved));
                }
            }
            result
        },
    );

    let body_ptr_set_option = body_ptr;
    engine.register_fn(
        "set_inventory_option",
        move |_ob: &mut rhai::Map, item_name: &str, option_name: &str, value: i64| -> String {
            let body = unsafe { &mut *body_ptr_set_option };
            for arc in &body.object.objs {
                if let Ok(mut item) = arc.lock() {
                    if strip_ansi_like_python(&item.getName()) != item_name
                        && !item.getString("반응이름").contains(item_name)
                    {
                        continue;
                    }
                    let mut options = item.get_option().unwrap_or_default();
                    options.insert(option_name.to_string(), value);
                    item.set_option(&options);
                    let current_name = item.getString("이름");
                    if !current_name.starts_with("\x1b[1;34m") {
                        item.set("이름", format!("\x1b[1;34m{}\x1b[0;37m", current_name));
                    }
                    return "ok".into();
                }
            }
            "no_item".into()
        },
    );

    let body_ptr_clear_magic = body_ptr;
    engine.register_fn(
        "clear_item_magic",
        move |_ob: &mut rhai::Map, item_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_clear_magic };
            for arc in &body.object.objs {
                if let Ok(mut item) = arc.lock() {
                    if strip_ansi_like_python(&item.getName()) != item_name
                        && !item.getString("반응이름").contains(item_name)
                    {
                        continue;
                    }
                    let option = item.getString("옵션");
                    if option.is_empty() {
                        return "no_option".into();
                    }
                    item.attr.remove("아이템속성");
                    item.attr.remove("옵션");
                    return option;
                }
            }
            "no_item".into()
        },
    );

    let body_ptr_apply_magic = body_ptr;
    engine.register_fn(
        "apply_item_magic",
        move |_ob: &mut rhai::Map, item_name: &str, level: i64| -> String {
            use rand::Rng;
            let body = unsafe { &mut *body_ptr_apply_magic };
            let map_raw = match std::fs::read_to_string("data/config/magic_map.json") {
                Ok(v) => v,
                Err(_) => return "no_effect".into(),
            };
            let map: serde_json::Value = match serde_json::from_str(&map_raw) {
                Ok(v) => v,
                Err(_) => return "no_effect".into(),
            };
            let mut rng = rand::thread_rng();
            if rng.gen_range(0..=4) != 0 {
                return "no_effect".into();
            }
            for arc in &body.object.objs {
                if let Ok(mut item) = arc.lock() {
                    if strip_ansi_like_python(&item.getName()) != item_name
                        && !item.getString("반응이름").contains(item_name)
                    {
                        continue;
                    }
                    let position = item.getString("계층");
                    let kind = item.getString("종류");
                    if position.is_empty()
                        || (kind != "무기" && kind != "방어구")
                        || !item.getString("옵션").is_empty()
                    {
                        return "no_effect".into();
                    }
                    let Some(maxes) = map.get(&position).and_then(|v| v.as_object()) else {
                        return "no_effect".into();
                    };
                    let names = [
                        "힘",
                        "민첩성",
                        "맷집",
                        "체력",
                        "내공",
                        "명중",
                        "필살",
                        "운",
                        "회피",
                        "경험치",
                        "마법발견",
                        "공격력",
                        "방어력",
                    ];
                    let mut options = std::collections::HashMap::new();
                    let mut attempts = 0;
                    while options.len() < 4 {
                        attempts += 1;
                        if attempts > 8 {
                            return "no_effect".into();
                        }
                        let option_name = names[rng.gen_range(0..names.len())];
                        if options.contains_key(option_name) {
                            continue;
                        }
                        let max = maxes.get(option_name).and_then(|v| v.as_i64()).unwrap_or(0);
                        if max <= 0 {
                            continue;
                        }
                        let scaled = level.saturating_mul(max) / 10_000;
                        if scaled <= 0 {
                            continue;
                        }
                        let low = scaled / 2;
                        let high = (scaled.saturating_mul(3) / 2).max(low);
                        let value = rng.gen_range(low..=high).min(max);
                        if value <= 0 {
                            continue;
                        }
                        options.insert(option_name.to_string(), value);
                        if option_name == "공격력" {
                            let current = item.getInt("공격력");
                            item.set("공격력", current + value);
                        }
                        if option_name == "방어력" {
                            let current = item.getInt("방어력");
                            item.set("방어력", current + value);
                        }
                    }
                    let current_defense = item.getInt("방어력");
                    let defense_base = level / 20;
                    let defense_delta = (defense_base / 10).max(0);
                    let defense_value =
                        defense_base + rng.gen_range(-defense_delta..=defense_delta);
                    if kind == "방어구" && current_defense < defense_value {
                        item.set("방어력", defense_value);
                    }
                    item.set_option(&options);
                    item.setAttr("아이템속성", "버리지못함");
                    item.setAttr("아이템속성", "줄수없음");
                    let plain = strip_ansi_like_python(&item.getString("이름"));
                    let color = if options.len() >= 4 {
                        "\x1b[1;33m"
                    } else if options.len() == 3 {
                        "\x1b[1;37m"
                    } else {
                        "\x1b[1;34m"
                    };
                    item.set("이름", format!("{}{}\x1b[0;37m", color, plain));
                    item.set("레벨", level);
                    return "ok".into();
                }
            }
            "no_item".into()
        },
    );

    // get_inventory_view(ob, line): Python `소지품.py`의 대상 선택과 출력용 데이터.
    // 관리자만 line으로 같은 방 플레이어를 고르며, 일반 사용자의 line은 Python처럼 무시한다.
    let body_ptr_inventory_view = body_ptr;
    engine.register_fn(
        "get_inventory_view",
        move |ob: &mut rhai::Map, line: &str| -> Dynamic {
            let admin = ob
                .get("관리자등급")
                .and_then(|value| value.as_int().ok())
                .unwrap_or(0);
            let body = unsafe { &*body_ptr_inventory_view };

            let target = if !line.is_empty() && admin >= 1000 {
                PRE_COMPUTED_ROOM_INVENTORIES.with(|cell| {
                    cell.borrow()
                        .as_deref()
                        .and_then(|players| find_room_inventory_target(line, players))
                })
            } else {
                Some(build_room_player_inventory_snapshot(body))
            };

            match target {
                Some(target) => inventory_view(&target, admin),
                None => {
                    let mut result = rhai::Map::new();
                    result.insert("ok".into(), Dynamic::from(false));
                    Dynamic::from(result)
                }
            }
        },
    );

    // get_mugong_view(ob, line): Python `무공.py`의 본인/관리자 같은 방 대상 선택과
    // 목록 데이터. 문구, ANSI, 열 배치는 cmds/무공.rhai에서만 만든다.
    let body_ptr_mugong_view = body_ptr;
    engine.register_fn(
        "get_mugong_view",
        move |_ob: &mut rhai::Map, line: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_mugong_view };
            let admin = body.get_int("관리자등급");

            let target = if !line.is_empty() && admin >= 1000 {
                PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                    cell.borrow()
                        .as_deref()
                        .and_then(|targets| find_room_mugong_target(line, targets))
                })
            } else {
                Some(build_room_mugong_player_snapshot(body))
            };

            match target {
                Some(target) if target.kind != RoomMugongTargetKind::Item => {
                    mugong_view(&target, &body.get_name())
                }
                _ => {
                    let mut result = rhai::Map::new();
                    result.insert("ok".into(), Dynamic::from(false));
                    Dynamic::from(result)
                }
            }
        },
    );

    // Python `무공상태.py`의 활성 방어무공 상태 데이터. 시간 막대와
    // ANSI/레이아웃은 Rhai가 계산하고 출력한다.
    let body_ptr_active_status = body_ptr;
    engine.register_fn(
        "get_active_mugong_status",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_active_status };
            body.active_skills
                .iter()
                .filter_map(|active| {
                    let skill = crate::world::get_skill(&active.name)?;
                    let level = body
                        .skill_map
                        .get(&active.name)
                        .map(|training| i64::from(training.level))
                        .unwrap_or(1);
                    let mut entry = rhai::Map::new();
                    entry.insert("name".into(), Dynamic::from(active.name.clone()));
                    entry.insert("time".into(), Dynamic::from(i64::from(active.start_time)));
                    entry.insert("level".into(), Dynamic::from(level));
                    entry.insert("defense_time".into(), Dynamic::from(skill.defense_time));
                    entry.insert(
                        "defense_time_increase".into(),
                        Dynamic::from(skill.defense_time_increase),
                    );
                    Some(Dynamic::from(entry))
                })
                .collect()
        },
    );
    let body_ptr_target_active = body_ptr;
    engine.register_fn(
        "get_target_active_mugong_status",
        move |_ob: &mut rhai::Map, line: &str| -> rhai::Array {
            let body = unsafe { &*body_ptr_target_active };
            if line.is_empty() {
                return body
                    .active_skills
                    .iter()
                    .filter_map(|active| {
                        let skill = crate::world::get_skill(&active.name)?;
                        let mut entry = rhai::Map::new();
                        entry.insert("name".into(), Dynamic::from(active.name.clone()));
                        entry.insert("time".into(), Dynamic::from(i64::from(active.start_time)));
                        entry.insert(
                            "level".into(),
                            Dynamic::from(
                                body.skill_map
                                    .get(&active.name)
                                    .map(|training| i64::from(training.level))
                                    .unwrap_or(1),
                            ),
                        );
                        entry.insert("defense_time".into(), Dynamic::from(skill.defense_time));
                        entry.insert(
                            "defense_time_increase".into(),
                            Dynamic::from(skill.defense_time_increase),
                        );
                        Some(Dynamic::from(entry))
                    })
                    .collect();
            }
            let admin = body.get_int("관리자등급");
            if admin < 1000 {
                return rhai::Array::new();
            }
            PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                let targets_guard = cell.borrow();
                let Some(targets) = targets_guard.as_ref() else {
                    return rhai::Array::new();
                };
                let Some(target) = find_room_mugong_target(line, targets) else {
                    return rhai::Array::new();
                };
                if target.kind != RoomMugongTargetKind::Player {
                    return rhai::Array::new();
                }
                target
                    .active_skills
                    .iter()
                    .map(|active| {
                        let mut entry = rhai::Map::new();
                        entry.insert("name".into(), Dynamic::from(active.name.clone()));
                        entry.insert("time".into(), Dynamic::from(active.time));
                        entry.insert("level".into(), Dynamic::from(active.level));
                        entry.insert("defense_time".into(), Dynamic::from(active.defense_time));
                        entry.insert(
                            "defense_time_increase".into(),
                            Dynamic::from(active.defense_time_increase),
                        );
                        Dynamic::from(entry)
                    })
                    .collect()
            })
        },
    );

    // get_merchant_script(ob): 현재 방의 상인(물건판매) 몹의 물건판매스크립을 "\r\n"으로 이어서 반환. 없으면 "".
    let body_ptr_merchant = body_ptr;
    engine.register_fn(
        "get_merchant_script",
        move |_ob: &mut rhai::Map| -> String {
            let body = unsafe { &*body_ptr_merchant };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return String::new(),
            };
            let mobs = w.get_mobs_for_player(name.as_str());
            for m in mobs {
                if let Some(data) = w.mob_cache.get_instance_data(m) {
                    if !data.items_for_sale.is_empty() && !data.sale_script.is_empty() {
                        return data.sale_script.join("\r\n");
                    }
                }
            }
            String::new()
        },
    );

    // get_merchant_buy_percent(ob): 현재 방의 물건구입 상인 몹의 구입 비율(1–100 등). 없으면 0.
    let body_ptr_buy = body_ptr;
    engine.register_fn(
        "get_merchant_buy_percent",
        move |_ob: &mut rhai::Map| -> i64 {
            let body = unsafe { &*body_ptr_buy };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            let mobs = w.get_mobs_for_player(name.as_str());
            for m in mobs {
                if let Some(data) = w.mob_cache.get_instance_data(m) {
                    if data.buy_percent > 0 {
                        return data.buy_percent;
                    }
                }
            }
            0
        },
    );

    let body_ptr_merchant_exists = body_ptr;
    engine.register_fn("room_has_merchant", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &*body_ptr_merchant_exists };
        let Some((zone, room)) = current_body_position(body) else {
            return false;
        };
        get_world_state()
            .read()
            .ok()
            .map(|world| {
                world.get_mobs_in_room(&zone, &room).into_iter().any(|mob| {
                    world.get_mob_data(&mob.mob_key).is_some_and(|data| {
                        !data.items_for_sale.is_empty() || data.buy_percent > 0
                    })
                })
            })
            .unwrap_or(false)
    });

    // Python 기부.py: deposit carried silver into the same-room 표두.
    let body_ptr_donate = body_ptr;
    engine.register_fn(
        "donate_to_guard",
        move |_ob: &mut rhai::Map, requested: i64| -> Dynamic {
            let body = unsafe { &mut *body_ptr_donate };
            let mut result = rhai::Map::new();
            let Some((zone, room)) = current_body_position(body) else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                return Dynamic::from(result);
            };
            let mut world = get_world_state().write().unwrap();
            let guard_id = world
                .mob_cache
                .get_mobs_in_room(&zone, &room)
                .into_iter()
                .find(|mob| {
                    mob.alive
                        && mob.act != 2
                        && mob.act != 3
                        && (mob.name == "표두"
                        || world.mob_cache.get_mob(&mob.mob_key).is_some_and(|data| {
                            data.reaction_names.iter().any(|name| name == "표두")
                        }))
                })
                .map(|mob| mob.instance_id);
            let Some(guard_id) = guard_id else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                return Dynamic::from(result);
            };
            if requested <= 0 {
                result.insert("status".into(), Dynamic::from("invalid_amount"));
                return Dynamic::from(result);
            }
            let amount = requested.min(body.get_int("은전"));
            let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                return Dynamic::from(result);
            };
            let guard = mobs.iter_mut().find(|mob| mob.instance_id == guard_id).unwrap();
            let guard_key = guard.mob_key.clone();
            guard.gold = guard.gold.saturating_add(amount);
            let total = guard.gold;
            body.set("은전", body.get_int("은전").saturating_sub(amount));
            drop(world);

            let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
            if let Some((mob_zone, mob_id)) = guard_key.split_once(':') {
                let path = std::path::Path::new("data/mob")
                    .join(mob_zone)
                    .join(format!("{mob_id}.json"));
                if let Ok(raw) = std::fs::read_to_string(&path) {
                    if let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(info) = root
                            .get_mut("몹정보")
                            .and_then(serde_json::Value::as_object_mut)
                        {
                            info.insert("은전".into(), serde_json::Value::Number(total.into()));
                            if let Ok(serialized) = serde_json::to_string_pretty(&root) {
                                let _ = std::fs::write(path, format!("{serialized}\n"));
                            }
                        }
                    }
                }
            }
            result.insert("status".into(), Dynamic::from("ok"));
            result.insert("amount".into(), Dynamic::from(amount));
            result.insert("total".into(), Dynamic::from(total));
            Dynamic::from(result)
        },
    );

    // Python 수령.py: withdraw a daily donation from the same-room 표두.
    // User-visible messages remain in Rhai; this efun owns only validation,
    // mutable mob silver and player state.
    let body_ptr_receive = body_ptr;
    engine.register_fn(
        "receive_from_guard",
        move |_ob: &mut rhai::Map, amount: i64| -> Dynamic {
            let body = unsafe { &mut *body_ptr_receive };
            let mut result = rhai::Map::new();
            let mut status = "ok";
            let mut total = body.get_int("수령액");
            let Some((zone, room)) = current_body_position(body) else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                result.insert("total".into(), Dynamic::from(total));
                return Dynamic::from(result);
            };
            let mut world = get_world_state().write().unwrap();
            let guard_id = world
                .mob_cache
                .get_mobs_in_room(&zone, &room)
                .into_iter()
                .find(|mob| {
                    mob.alive
                        && mob.act != 2
                        && mob.act != 3
                        && (mob.name == "표두"
                        || world
                            .mob_cache
                            .get_mob(&mob.mob_key)
                            .is_some_and(|data| data.reaction_names.iter().any(|name| name == "표두")))
                })
                .map(|mob| mob.instance_id);
            let Some(guard_id) = guard_id else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                result.insert("total".into(), Dynamic::from(total));
                return Dynamic::from(result);
            };
            let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                result.insert("total".into(), Dynamic::from(total));
                return Dynamic::from(result);
            };
            let guard = mobs.iter_mut().find(|mob| mob.instance_id == guard_id).unwrap();
            let guard_key = guard.mob_key.clone();
            let mut remaining_guard_gold = guard.gold;
            if amount <= 0 {
                status = "invalid_amount";
            } else if body.get_int("레벨") > 500 {
                status = "high_level";
            } else if amount > 10_000_000 {
                status = "too_greedy";
            } else if amount > guard.gold {
                status = "fund_short";
            } else if total >= 1_000_000_000 {
                status = "total_limit";
            } else if total.saturating_add(amount) >= 1_000_000_000 {
                status = "over_limit";
            } else {
                let now = chrono::Utc::now().timestamp();
                if body.get_int("마지막수령").saturating_add(86_400) > now {
                    status = "too_soon";
                } else {
                    body.set("마지막수령", now);
                    body.set("은전", body.get_int("은전").saturating_add(amount));
                    total = total.saturating_add(amount);
                    body.set("수령액", total);
                    guard.gold = guard.gold.saturating_sub(amount);
                    remaining_guard_gold = guard.gold;
                }
            }
            drop(world);
            if status == "ok" {
                let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
                if let Some((mob_zone, mob_id)) = guard_key.split_once(':') {
                    let path = std::path::Path::new("data/mob")
                        .join(mob_zone)
                        .join(format!("{mob_id}.json"));
                    if let Ok(raw) = std::fs::read_to_string(&path) {
                        if let Ok(mut root) =
                            serde_json::from_str::<serde_json::Value>(&raw)
                        {
                            if let Some(info) = root
                                .get_mut("몹정보")
                                .and_then(serde_json::Value::as_object_mut)
                            {
                                info.insert(
                                    "은전".to_string(),
                                    serde_json::Value::Number(remaining_guard_gold.into()),
                                );
                                if let Ok(serialized) = serde_json::to_string_pretty(&root) {
                                    let _ = std::fs::write(path, format!("{serialized}\n"));
                                }
                            }
                        }
                    }
                }
            }
            result.insert("status".into(), Dynamic::from(status.to_string()));
            result.insert("total".into(), Dynamic::from(total));
            Dynamic::from(result)
        },
    );

    // merchant_buy(ob, name, count): 기능만. {err: ""|"usage"|"no_merchant"|"not_for_sale"|"inv_full"|"too_heavy"|"no_money", bought, display_name, total_cost}. 오류 메시지는 Rhai에서.
    let body_ptr_mbuy = body_ptr;
    engine.register_fn(
        "merchant_buy",
        move |_ob: &mut rhai::Map, name: &str, count: i64| -> Dynamic {
            let mut m = rhai::Map::new();
            let mut err = String::new();
            let mut bought = 0i64;
            let mut display_name = String::new();
            let mut total_cost = 0i64;
            if name.is_empty() {
                m.insert("err".into(), Dynamic::from("usage".to_string()));
                m.insert("bought".into(), Dynamic::from(0i64));
                m.insert("display_name".into(), Dynamic::from(String::new()));
                m.insert("total_cost".into(), Dynamic::from(0i64));
                return Dynamic::from(m);
            }
            let body = unsafe { &mut *body_ptr_mbuy };
            let pname = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => {
                    m.insert("err".into(), Dynamic::from("no_merchant".to_string()));
                    m.insert("bought".into(), Dynamic::from(0i64));
                    m.insert("display_name".into(), Dynamic::from(String::new()));
                    m.insert("total_cost".into(), Dynamic::from(0i64));
                    return Dynamic::from(m);
                }
            };
            let mobs = w.get_mobs_for_player(pname.as_str());
            let mut item_key = String::new();
            let mut unit_price = 0i64;
            let mut weight = 0i64;
            for m in mobs {
                let data = match w.mob_cache.get_instance_data(m) {
                    Some(d) if !d.items_for_sale.is_empty() => d,
                    _ => continue,
                };
                for (key, percent) in &data.items_for_sale {
                    let Some((iname, rn, price, wg)) = get_item_info(key) else {
                        continue;
                    };
                    let ok = iname == name || (!rn.is_empty() && rn.contains(name));
                    if !ok {
                        continue;
                    }
                    let p = (*percent).max(1);
                    unit_price = price * 100 / p;
                    weight = wg;
                    display_name = iname;
                    item_key = key.clone();
                    break;
                }
                if !item_key.is_empty() {
                    break;
                }
            }
            if item_key.is_empty() {
                m.insert("err".into(), Dynamic::from("not_for_sale".to_string()));
                m.insert("bought".into(), Dynamic::from(0i64));
                m.insert("display_name".into(), Dynamic::from(display_name));
                m.insert("total_cost".into(), Dynamic::from(0i64));
                return Dynamic::from(m);
            }
            if let Some((guard_template, _)) = object_from_item_json(&item_key) {
                let is_guard = guard_template
                    .lock()
                    .is_ok_and(|item| item.getString("종류") == "호위");
                if is_guard {
                    drop(w);
                    let guard = guard_template.lock().unwrap();
                    let faction = guard.getString("구매속성");
                    let personality = body.get_string("성격");
                    if personality != "기인" && personality != "선인" {
                        if faction == "사파" && personality != faction {
                            m.insert("err".into(), Dynamic::from("guard_evil_only"));
                            return Dynamic::from(m);
                        }
                        if faction == "정파" && personality != faction {
                            m.insert("err".into(), Dynamic::from("guard_right_only"));
                            return Dynamic::from(m);
                        }
                    }
                    let conditions = attr_string_list(&guard.getString("구매조건"));
                    if conditions.is_empty() {
                        m.insert("err".into(), Dynamic::from("guard_unavailable"));
                        return Dynamic::from(m);
                    }
                    let guard_name = guard.getName();
                    let guard_level = guard.getInt("구매레벨");
                    let mut same_count = 0i64;
                    let mut max_level = 0i64;
                    for item in &body.object.objs {
                        if let Ok(item) = item.lock() {
                            if item.getString("종류") != "호위" {
                                continue;
                            }
                            if item.getName() == guard_name {
                                same_count += 1;
                            }
                            max_level = max_level.max(item.getInt("구매레벨"));
                        }
                    }
                    if guard_level < max_level {
                        m.insert("err".into(), Dynamic::from("guard_level"));
                        return Dynamic::from(m);
                    }
                    let limit = attr_string_list(&guard.getString("아이템속성"))
                        .into_iter()
                        .find_map(|attribute| {
                            let words: Vec<_> = attribute.split_whitespace().collect();
                            (words.first() == Some(&"소지한계"))
                                .then(|| words.get(1)?.parse::<i64>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if same_count >= limit {
                        m.insert("err".into(), Dynamic::from("guard_limit"));
                        return Dynamic::from(m);
                    }
                    let herb = |item: &Object| {
                        matches!(
                            item.getString("인덱스").as_str(),
                            "합성1" | "합성2" | "합성3" | "합성4" | "합성5"
                                | "합성6" | "합성7" | "합성8" | "합성9"
                        )
                    };
                    let herb_count = body
                        .object
                        .objs
                        .iter()
                        .filter(|item| item.lock().is_ok_and(|item| herb(&item)))
                        .count() as i64;
                    let named_count = |wanted: &str| {
                        body.object
                            .objs
                            .iter()
                            .filter(|item| item.lock().is_ok_and(|item| item.getName() == wanted))
                            .count() as i64
                    };
                    let mut consume_name = String::new();
                    let mut consume_count = 0i64;
                    for condition in conditions {
                        let words: Vec<_> = condition.split_whitespace().collect();
                        if words.len() == 2 {
                            let needed = words[1].parse::<i64>().unwrap_or(0);
                            let available = if words[0] == "약초" {
                                herb_count
                            } else {
                                named_count(words[0])
                            };
                            if available >= needed {
                                consume_name = words[0].to_string();
                                consume_count = needed;
                                break;
                            }
                        } else if words.len() == 3
                            && named_count(words[0]) >= 1
                            && herb_count >= words[2].parse::<i64>().unwrap_or(0)
                        {
                            consume_name = words[1].to_string();
                            consume_count = words[2].parse::<i64>().unwrap_or(0);
                            break;
                        }
                    }
                    if consume_name.is_empty() {
                        m.insert("err".into(), Dynamic::from("guard_unavailable"));
                        return Dynamic::from(m);
                    }
                    drop(guard);
                    let mut removed = 0i64;
                    let mut materials = Vec::new();
                    for item in &body.object.objs {
                        let matches = item.lock().is_ok_and(|item| {
                            if consume_name == "약초" {
                                herb(&item)
                            } else {
                                item.getName() == consume_name
                            }
                        });
                        if matches {
                            materials.push(item.clone());
                            removed += 1;
                            if removed >= consume_count {
                                break;
                            }
                        }
                    }
                    for item in materials {
                        body.object.remove(&item);
                    }
                    body.object.objs.insert(0, guard_template);
                    let path = format!("data/user/{}.json", body.get_name());
                    let _ = save_body_to_json(body, &path);
                    m.insert("err".into(), Dynamic::from(String::new()));
                    m.insert("bought".into(), Dynamic::from(1_i64));
                    m.insert("display_name".into(), Dynamic::from(guard_name));
                    m.insert("total_cost".into(), Dynamic::from(0_i64));
                    m.insert("guard".into(), Dynamic::from(true));
                    return Dynamic::from(m);
                }
            }
            let cnt = count.clamp(1, 200);
            let max_items = get_murim_config_int("사용자아이템갯수").max(0) as usize;
            for _ in 0..cnt {
                if body.get_item_count() >= max_items {
                    if bought == 0 {
                        err = "inv_full".to_string();
                    }
                    break;
                }
                if body.get_item_weight() + weight > body.get_str() * 10 {
                    if bought == 0 {
                        err = "too_heavy".to_string();
                    }
                    break;
                }
                if body.get_int("은전") < unit_price {
                    if bought == 0 {
                        err = "no_money".to_string();
                    }
                    break;
                }
                if is_stackable(&item_key) {
                    *body.object.inv_stack.entry(item_key.clone()).or_insert(0) += 1;
                    body.set("은전", body.get_int("은전") - unit_price);
                    bought += 1;
                    total_cost += unit_price;
                } else if let Some((arc, _)) = object_from_item_json(&item_key) {
                    body.object.objs.insert(0, arc);
                    body.set("은전", body.get_int("은전") - unit_price);
                    bought += 1;
                    total_cost += unit_price;
                } else {
                    break;
                }
            }
            if bought > 0 {
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            m.insert("err".into(), Dynamic::from(err));
            m.insert("bought".into(), Dynamic::from(bought));
            m.insert("display_name".into(), Dynamic::from(display_name.clone()));
            m.insert("total_cost".into(), Dynamic::from(total_cost));
            m.insert("guard".into(), Dynamic::from(false));
            Dynamic::from(m)
        },
    );

    // item_sell(ob, name, order, count, percent): 소지품을 상인에게 판매.
    // Returns [sold, total, display_name, err] where err is "" or "no_item" or "cant_sell".
    let body_ptr_sell = body_ptr;
    engine.register_fn(
        "item_sell",
        move |_ob: &mut rhai::Map,
              name: &str,
              order: i64,
              count: i64,
              percent: i64|
              -> rhai::Array {
            use rhai::Dynamic;
            let body = unsafe { &mut *body_ptr_sell };
            let order = order.max(1) as usize;
            let count = count.clamp(1, 100) as usize;
            let percent = percent.max(0);
            // 스택: order==1일 때 inv_stack에서 판매
            if order == 1 {
                if let Some(ref key) = find_item_key_by_name(name) {
                    if is_stackable(key) {
                        let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                        let sell_cnt = (count as i64).clamp(0, have);
                        if sell_cnt > 0 {
                            if let Some((iname, _, base_price, _)) = get_item_info(key) {
                                let unit = (base_price * percent) / 100;
                                let total = unit * sell_cnt;
                                let should_remove = {
                                    let r = body.object.inv_stack.get_mut(key).unwrap();
                                    *r -= sell_cnt;
                                    *r <= 0
                                };
                                if should_remove {
                                    body.object.inv_stack.remove(key);
                                }
                                body.set("은전", body.get_int("은전") + total);
                                let path = format!("data/user/{}.json", body.get_name());
                                let _ = save_body_to_json(body, &path);
                                return vec![
                                    Dynamic::from(sell_cnt),
                                    Dynamic::from(total),
                                    Dynamic::from(iname),
                                    Dynamic::from(""),
                                ];
                            }
                        }
                    }
                }
            }
            let matches = |object: &Object, wanted: &str| {
                object.getName() == wanted
                    || reaction_names(&object.getString("반응이름"))
                        .iter()
                        .any(|alias| alias == wanted)
            };
            let matching: Vec<_> = body
                .object
                .objs
                .iter()
                .filter(|item| item.lock().is_ok_and(|item| matches(&item, name)))
                .cloned()
                .collect();
            let Some(first) = matching.get(order - 1).cloned() else {
                return vec![
                    Dynamic::from(0_i64),
                    Dynamic::from(0_i64),
                    Dynamic::from(String::new()),
                    Dynamic::from("no_item"),
                ];
            };
            {
                let first = first.lock().unwrap();
                if first.getBool("inUse") || first.checkAttr("아이템속성", "출력안함") {
                    return vec![
                        Dynamic::from(0_i64),
                        Dynamic::from(0_i64),
                        Dynamic::from(String::new()),
                        Dynamic::from("no_item"),
                    ];
                }
                if first.checkAttr("아이템속성", "팔지못함") {
                    return vec![
                        Dynamic::from(0_i64),
                        Dynamic::from(0_i64),
                        Dynamic::from(String::new()),
                        Dynamic::from("cant_sell"),
                    ];
                }
            }
            let mut to_remove: Vec<Arc<Mutex<Object>>> = vec![first];
            let mut total = 0i64;
            let mut display_name = String::new();
            let mut processed = 0usize;
            while processed < to_remove.len() && processed < count {
                let current = to_remove[processed].clone();
                {
                    let o = current.lock().unwrap();
                    let mut price = (o.getInt("판매가격") * percent) / 100;
                    if let Some(options) = o.get_option() {
                        price = (price as f64 * (options.len() as f64 * 1.3)) as i64;
                    }
                    total += price;
                    if display_name.is_empty() {
                        display_name = o.getName();
                    }
                }
                processed += 1;
                if order != 1 || processed >= count {
                    break;
                }
                let next = body.object.objs.iter().find(|candidate| {
                    !to_remove.iter().any(|selected| Arc::ptr_eq(selected, candidate))
                        && candidate.lock().is_ok_and(|item| matches(&item, name))
                });
                let Some(next) = next else { break };
                if next.lock().is_ok_and(|item| item.getBool("inUse")) {
                    break;
                }
                to_remove.push(next.clone());
            }
            for arc in &to_remove {
                if let Ok(item) = arc.lock() {
                    if item.checkAttr("아이템속성", "단일아이템") {
                        let index = item.getString("인덱스");
                        if !index.is_empty() {
                            let _ = crate::oneitem::oneitem_destroy(&index);
                        }
                    }
                }
                body.object.remove(arc);
            }
            body.set("은전", body.get_int("은전") + total);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            vec![
                Dynamic::from(to_remove.len() as i64),
                Dynamic::from(total),
                Dynamic::from(display_name),
                Dynamic::from(String::new()),
            ]
        },
    );

    let body_ptr_sell_all = body_ptr;
    engine.register_fn(
        "item_sell_all",
        move |_ob: &mut rhai::Map, mode: &str, percent: i64| -> rhai::Array {
            let body = unsafe { &mut *body_ptr_sell_all };
            let percent = percent.max(0);
            let inventory = body.object.objs.clone();
            let mut sold = rhai::Array::new();
            let mut total = 0_i64;
            for arc in inventory {
                let (name, price) = {
                    let Ok(item) = arc.lock() else {
                        continue;
                    };
                    if item.getBool("inUse")
                        || item.checkAttr("아이템속성", "출력안함")
                        || item.checkAttr("아이템속성", "팔지못함")
                    {
                        continue;
                    }
                    let kind = item.getString("종류");
                    let equipment = kind == "방어구" || kind == "무기";
                    let option_count = item.get_option().map(|value| value.len()).unwrap_or(0);
                    let selected = match mode {
                        "속성0" => equipment && item.getString("옵션").is_empty(),
                        "속성1" => equipment && option_count <= 2,
                        "속성2" => equipment && option_count <= 3,
                        "속성3" => equipment && option_count <= 4,
                        "일반" => equipment && option_count == 0,
                        "장비" => equipment,
                        "모두" => true,
                        _ => false,
                    };
                    if !selected {
                        continue;
                    }
                    let mut price = (item.getInt("판매가격") * percent) / 100;
                    if let Some(options) = item.get_option() {
                        price = (price as f64 * (options.len() as f64 * 1.2)) as i64;
                    }
                    (item.getName(), price)
                };
                if let Ok(item) = arc.lock() {
                    if item.checkAttr("아이템속성", "단일아이템") {
                        let index = item.getString("인덱스");
                        if !index.is_empty() {
                            let _ = crate::oneitem::oneitem_destroy(&index);
                        }
                    }
                }
                body.object.remove(&arc);
                total = total.saturating_add(price);
                let mut event = rhai::Map::new();
                event.insert("name".into(), Dynamic::from(name));
                event.insert("price".into(), Dynamic::from(price));
                sold.push(Dynamic::from(event));
            }
            if !sold.is_empty() {
                body.set("은전", body.get_int("은전").saturating_add(total));
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            sold
        },
    );

    // view_map_data(ob): 기능만. {ok, text, err, zone, room}. ok=true면 text에 방 문자열. err="no_position"|"room_error"|"unknown_room". 출력은 Rhai에서 send_line.
    let get_other = get_other_players_desc;
    engine.register_fn("view_map_data", move |ob: &mut rhai::Map| -> Dynamic {
        let name: String = ob
            .get("이름")
            .or_else(|| ob.get("name"))
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let others = get_other.as_ref().map(|f| f(&name)).unwrap_or_default();
        let mut m = rhai::Map::new();
        match build_room_lines(&name, &others) {
            Ok(mut text) => {
                // Python displays the zone/room suffix to administrators in
                // viewMapData. build_room_lines is also used by non-admin
                // network notifications, so apply this viewer-only detail
                // at the efun boundary.
                if ob
                    .get("관리자등급")
                    .and_then(|v| v.clone().try_cast::<i64>())
                    .unwrap_or(0)
                    >= 1000
                {
                    if let Some(pos) = text.get(2..).and_then(|s| s.find("\r\n")) {
                        if let Some(position) = get_world_state()
                            .read()
                            .ok()
                            .and_then(|world| world.get_player_position(&name).cloned())
                        {
                            let (zone, room) = (position.zone, position.room);
                            let insert_at = pos + 2;
                            text.insert_str(insert_at, &format!(" ({}:{})", zone, room));
                        }
                    }
                }
                m.insert("ok".into(), Dynamic::from(true));
                m.insert("text".into(), Dynamic::from(text));
                m.insert("err".into(), Dynamic::from(String::new()));
                m.insert("zone".into(), Dynamic::from(String::new()));
                m.insert("room".into(), Dynamic::from(""));
            }
            Err((err, zone, room)) => {
                m.insert("ok".into(), Dynamic::from(false));
                m.insert("text".into(), Dynamic::from(String::new()));
                m.insert("err".into(), Dynamic::from(err));
                m.insert("zone".into(), Dynamic::from(zone));
                m.insert("room".into(), Dynamic::from(room));
            }
        }
        Dynamic::from(m)
    });

    // find_target(ob, line): [대상] 봐. {found, lines, to_target, err}. err=""|"no_position"|"not_found". 오류 메시지는 Rhai에서.
    let body_ptr_ft = body_ptr;
    let get_other_map_ft = get_other_players_map.clone();
    engine.register_fn(
        "find_target",
        move |ob: &mut rhai::Map, line: &str| -> Dynamic {
            let viewer_name: String = ob
                .get("이름")
                .or_else(|| ob.get("name"))
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let world = get_world_state().read().unwrap();
            let other = get_other_map_ft.as_ref().map(|f| f()).unwrap_or_default();
            let (lines, to_target) =
                look_at_target(unsafe { &*body_ptr_ft }, &world, &viewer_name, line, &other);
            let (err, lines_out) = if lines.len() == 1 {
                if lines[0].contains("위치 정보가 없습니다") {
                    ("no_position".to_string(), vec![])
                } else if lines[0].contains("안광으로는 그런것을 볼수 없다") {
                    ("not_found".to_string(), vec![])
                } else {
                    (String::new(), lines)
                }
            } else {
                (String::new(), lines)
            };
            let found = to_target.is_some() || (!lines_out.is_empty() && err.is_empty());
            let mut m = rhai::Map::new();
            m.insert("found".into(), Dynamic::from(found));
            m.insert("err".into(), Dynamic::from(err));
            m.insert(
                "lines".into(),
                Dynamic::from(rhai::Array::from_iter(
                    lines_out.into_iter().map(Dynamic::from),
                )),
            );
            let mut to_map = rhai::Map::new();
            if let Some((n, msg)) = to_target {
                to_map.insert("name".into(), Dynamic::from(n));
                to_map.insert("msg".into(), Dynamic::from(msg));
            } else {
                to_map.insert("name".into(), Dynamic::from(""));
                to_map.insert("msg".into(), Dynamic::from(""));
            }
            m.insert("to_target".into(), Dynamic::from(to_map));
            Dynamic::from(m)
        },
    );

    // get_all_online_players(): 전 접속자 목록. [{"이름","무림별호","성격","레벨초기화","소속","설정상태"}, ...]. 누구 스크립트용.
    engine.register_fn("get_all_online_players", get_precomputed_all_online);
    engine.register_fn("get_online_socket_entries", || -> rhai::Array {
        let mut entries: Vec<(String, String)> = get_precomputed_all_online()
            .into_iter()
            .filter_map(|value| {
                let map = value.try_cast::<rhai::Map>()?;
                let host = map.get("host")?.clone().into_string().ok()?;
                let name = map.get("이름")?.clone().into_string().ok()?;
                Some((host, name))
            })
            .collect();
        entries.sort();
        entries
            .into_iter()
            .map(|(host, name)| {
                let mut map = rhai::Map::new();
                map.insert("host".into(), Dynamic::from(host));
                map.insert("name".into(), Dynamic::from(name));
                Dynamic::from(map)
            })
            .collect()
    });
    engine.register_fn("get_online_names", get_online_names);
    engine.register_fn("get_connected_player_names", get_connected_player_names);
    engine.register_fn("user_refuses_shout", user_refuses_shout);
    engine.register_fn(
        "config_text_is_enabled",
        |config: &str, key: &str| -> bool { config_is_enabled(config, key) },
    );

    // Python `Player.adultCH` is a separate ordered global membership list,
    // not an alias for all online users. The network layer installs only
    // members of that list; Rhai owns filtering, layout, ANSI and raw CRLF.
    engine.register_fn("get_adult_channel_members", || -> rhai::Array {
        PRE_COMPUTED_ADULT_CHANNEL
            .with(|cell| cell.borrow().clone())
            .unwrap_or_default()
    });
    engine.register_fn("get_adult_channel_self_id", || -> String {
        PRE_COMPUTED_ADULT_CHANNEL_SELF_ID
            .with(|cell| cell.borrow().clone())
            .unwrap_or_default()
    });
    engine.register_fn("is_adult_channel_member", |_ob: &mut rhai::Map| -> bool {
        PRE_COMPUTED_ADULT_CHANNEL_SELF_MEMBER.with(|cell| *cell.borrow())
    });

    let body_ptr_adult_join = body_ptr;
    engine.register_fn("adult_channel_join", move |_ob: &mut rhai::Map| -> bool {
        if PRE_COMPUTED_ADULT_CHANNEL_SELF_MEMBER.with(|cell| *cell.borrow()) {
            return false;
        }
        let body = unsafe { &mut *body_ptr_adult_join };
        body.temp_mut().insert(
            ADULT_CHANNEL_ACTION_REQUEST.to_string(),
            Value::String("join".to_string()),
        );
        true
    });

    let body_ptr_adult_leave = body_ptr;
    engine.register_fn("adult_channel_leave", move |_ob: &mut rhai::Map| -> bool {
        if !PRE_COMPUTED_ADULT_CHANNEL_SELF_MEMBER.with(|cell| *cell.borrow()) {
            return false;
        }
        let body = unsafe { &mut *body_ptr_adult_leave };
        body.temp_mut().insert(
            ADULT_CHANNEL_ACTION_REQUEST.to_string(),
            Value::String("leave".to_string()),
        );
        true
    });

    let body_ptr_adult_disconnect = body_ptr;
    engine.register_fn(
        "is_adult_channel_disconnect",
        move |_ob: &mut rhai::Map| -> bool {
            let body = unsafe { &*body_ptr_adult_disconnect };
            matches!(
                body.temp().get(ADULT_CHANNEL_DISCONNECT_REQUEST),
                Some(Value::Int(1))
            )
        },
    );

    let body_ptr_adult_auto_join = body_ptr;
    engine.register_fn(
        "is_adult_channel_auto_join",
        move |_ob: &mut rhai::Map| -> bool {
            let body = unsafe { &*body_ptr_adult_auto_join };
            matches!(
                body.temp().get(ADULT_CHANNEL_AUTO_JOIN_REQUEST),
                Some(Value::Int(1))
            )
        },
    );

    let body_ptr_adult_send = body_ptr;
    engine.register_fn(
        "adult_channel_send",
        move |_ob: &mut rhai::Map, member_id: &str, raw_text: &str| {
            if member_id.is_empty() || raw_text.is_empty() {
                return;
            }
            let self_id = PRE_COMPUTED_ADULT_CHANNEL_SELF_ID
                .with(|cell| cell.borrow().clone())
                .unwrap_or_default();
            let is_known_member = PRE_COMPUTED_ADULT_CHANNEL.with(|cell| {
                cell.borrow().as_ref().is_some_and(|members| {
                    members.iter().any(|member| {
                        member
                            .clone()
                            .try_cast::<rhai::Map>()
                            .and_then(|map| map.get("id").cloned())
                            .and_then(|id| id.into_string().ok())
                            .is_some_and(|id| id == member_id)
                    })
                })
            });
            if member_id != self_id && !is_known_member {
                return;
            }

            let body = unsafe { &mut *body_ptr_adult_send };
            let current = body
                .temp()
                .get(ADULT_CHANNEL_DELIVERY_REQUESTS)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mut deliveries: Vec<AdultChannelDelivery> =
                serde_json::from_str(&current).unwrap_or_default();
            deliveries.push(AdultChannelDelivery {
                member_id: member_id.to_string(),
                raw_text: raw_text.to_string(),
            });
            if let Ok(json) = serde_json::to_string(&deliveries) {
                body.temp_mut().insert(
                    ADULT_CHANNEL_DELIVERY_REQUESTS.to_string(),
                    Value::String(json),
                );
            }
        },
    );

    // get_user_config(ob, 키), set_user_config(ob, 키, 값): 영구 저장 사용자 설정. ob["설정"][키]=값. 설정상태 파싱/저장.
    let body_ptr_cfg = body_ptr;
    engine.register_fn(
        "get_user_config",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_cfg };
            parse_config_string(&body.get_string("설정상태"))
                .get(key)
                .cloned()
                .unwrap_or_default()
        },
    );
    let body_ptr_cfg2 = body_ptr;
    engine.register_fn(
        "set_user_config",
        move |_ob: &mut rhai::Map, key: &str, value: &str| {
            let body = unsafe { &mut *body_ptr_cfg2 };
            let mut m = parse_config_string(&body.get_string("설정상태"));
            m.insert(key.to_string(), value.to_string());
            body.object.attr.insert(
                "설정상태".to_string(),
                Value::String(format_config_string(&m)),
            );
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        },
    );

    // get_user_event(ob, 키), set_user_event(ob, 키, 값), del_user_event(ob, 키): 임무 등 이벤트. ob["이벤트"][키]=값. 이벤트설정리스트.
    let body_ptr_ev = body_ptr;
    engine.register_fn(
        "get_user_event",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_ev };
            parse_event_string(&body.get_string("이벤트설정리스트"))
                .get(key)
                .cloned()
                .unwrap_or_default()
        },
    );
    let body_ptr_ev2 = body_ptr;
    engine.register_fn(
        "set_user_event",
        move |_ob: &mut rhai::Map, key: &str, value: &str| {
            let body = unsafe { &mut *body_ptr_ev2 };
            let mut m = parse_event_string(&body.get_string("이벤트설정리스트"));
            if value.is_empty() {
                m.remove(key);
            } else {
                m.insert(key.to_string(), value.to_string());
            }
            body.object.attr.insert(
                "이벤트설정리스트".to_string(),
                Value::String(format_event_string(&m)),
            );
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        },
    );
    let body_ptr_ev3 = body_ptr;
    engine.register_fn("del_user_event", move |_ob: &mut rhai::Map, key: &str| {
        let body = unsafe { &mut *body_ptr_ev3 };
        let mut m = parse_event_string(&body.get_string("이벤트설정리스트"));
        m.remove(key);
        body.object.attr.insert(
            "이벤트설정리스트".to_string(),
            Value::String(format_event_string(&m)),
        );
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);
    });

    // make_home(ob): Python Player.makeHome과 같은 사용자맵 JSON 생성.
    // 성공은 "", 실패는 Rhai가 문구로 바꿀 수 있는 오류 코드만 반환한다.
    let body_ptr_home = body_ptr;
    engine.register_fn("make_home", move |_ob: &mut rhai::Map| -> String {
        let body = unsafe { &*body_ptr_home };
        let name = body.get_name();
        match crate::world::user_home::make_user_home(&name) {
            Ok(_) => String::new(),
            Err(crate::world::user_home::UserHomeError::InvalidName) => "invalid_name".to_string(),
            Err(_) => "save_failed".to_string(),
        }
    });

    // check_mob_event(mob_key, event_key) - Check if mob has event (Python: target.checkEvent)
    engine.register_fn(
        "check_mob_event",
        |mob_key: &str, event_key: &str| -> bool {
            let cache = crate::world::mob::get_mob_cache();
            if let Ok(cache_guard) = cache.read() {
                cache_guard.check_mob_event(mob_key, event_key)
            } else {
                false
            }
        },
    );

    // set_mob_event(mob_key, event_key) - Set event on mob (Python: target.setEvent)
    engine.register_fn("set_mob_event", |mob_key: &str, event_key: &str| -> bool {
        let cache = crate::world::mob::get_mob_cache();
        if let Ok(mut cache_guard) = cache.write() {
            cache_guard.set_mob_event(mob_key, event_key)
        } else {
            false
        }
    });

    // del_mob_event(mob_key, event_key) - Delete event from mob (Python: target.delEvent)
    engine.register_fn("del_mob_event", |mob_key: &str, event_key: &str| -> bool {
        let cache = crate::world::mob::get_mob_cache();
        if let Ok(mut cache_guard) = cache.write() {
            cache_guard.del_mob_event(mob_key, event_key)
        } else {
            false
        }
    });

    // get_admin_level(ob) - Get player's admin level (관리자등급)
    let body_ptr_admin = body_ptr;
    engine.register_fn("get_admin_level", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_admin };
        crate::command::handler::helpers::get_admin_level(body)
    });

    // get_my_position(ob) -> {zone, room}. 어디 등.
    let body_ptr_pos = body_ptr;
    engine.register_fn("get_my_position", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &*body_ptr_pos };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Map::new().into(),
        };
        let pos = w.get_player_position(&name);
        let mut m = rhai::Map::new();
        if let Some(p) = pos {
            m.insert("zone".into(), Dynamic::from(p.zone.clone()));
            m.insert("room".into(), Dynamic::from(p.room.clone()));
        } else {
            m.insert("zone".into(), Dynamic::from(""));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // get_room_name(zone, room) -> 방 이름 문자열. 어디 등.
    // i64 버전
    engine.register_fn("get_room_name", |zone: &str, room: i64| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return format!("{}:{}", zone, room),
        };
        let r = w.room_cache.get_room_cached(zone, &room.to_string());
        match r {
            Some(arc) => {
                let guard = arc.read().unwrap();
                if guard.display_name.is_empty() {
                    guard.name.clone()
                } else {
                    guard.display_name.clone()
                }
            }
            None => format!("{}:{}", zone, room),
        }
    });

    // get_room_name(zone, room) -> 방 이름 문자열. 어디 등.
    // &str 버전 (room이 문자열인 경우)
    engine.register_fn("get_room_name", |zone: &str, room: &str| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return format!("{}:{}", zone, room),
        };
        let r = w.room_cache.get_room_cached(zone, room);
        match r {
            Some(arc) => {
                let guard = arc.read().unwrap();
                if guard.display_name.is_empty() {
                    guard.name.clone()
                } else {
                    guard.display_name.clone()
                }
            }
            None => format!("{}:{}", zone, room),
        }
    });

    // get_equipped(ob) -> [{slot, name, is_han, alias}, ...].
    // Python 장비.py의 ItemLevelList 순서와 외국어 이름의 첫 반응이름을 보존한다.
    let body_ptr_eq = body_ptr;
    engine.register_fn("get_equipped", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_eq };
        let mut pairs: Vec<(String, String, bool, String)> = Vec::new();
        for arc in &body.object.objs {
            if let Ok(o) = arc.lock() {
                if !o.getBool("inUse") {
                    continue;
                }
                let slot = o.getString("계층");
                if slot.is_empty() {
                    continue;
                }
                let name = o.getName();
                let is_han = crate::hangul::is_han(&strip_ansi_like_python(&name));
                let alias = reaction_names(&o.getString("반응이름"))
                    .into_iter()
                    .next()
                    .unwrap_or_default();
                pairs.push((slot, name, is_han, alias));
            }
        }
        pairs.sort_by_cached_key(|(slot, _, _, _)| {
            ITEM_EQUIP_LEVELS
                .iter()
                .position(|&level| level == slot.as_str())
                .unwrap_or(999)
        });
        let mut arr = rhai::Array::new();
        for (slot, name, is_han, alias) in pairs {
            let mut m = rhai::Map::new();
            m.insert("slot".into(), Dynamic::from(slot));
            m.insert("name".into(), Dynamic::from(name));
            m.insert("is_han".into(), Dynamic::from(is_han));
            m.insert("alias".into(), Dynamic::from(alias));
            arr.push(Dynamic::from(m));
        }
        arr
    });

    // remember_equipment_set(ob): 장착 중인 무기/방어구에 SET- 별칭을 추가한다.
    let body_ptr_res = body_ptr;
    engine.register_fn(
        "remember_equipment_set",
        move |_ob: &mut rhai::Map| -> i64 {
            let body = unsafe { &mut *body_ptr_res };
            let set_name = format!(
                "SET-{}-{}",
                body.get_name(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            );
            let mut count = 0i64;
            for arc in &body.object.objs {
                if let Ok(mut item) = arc.lock() {
                    if !item.getBool("inUse") {
                        continue;
                    }
                    let kind = item.getString("종류");
                    if kind != "방어구" && kind != "무기" {
                        continue;
                    }
                    let mut names = reaction_names(&item.getString("반응이름"));
                    names.retain(|name| !name.starts_with("SET-"));
                    names.push(set_name.clone());
                    item.set("반응이름", names.join("\r\n"));
                    count += 1;
                }
            }
            body.set("세트기억", set_name);
            count
        },
    );

    // get_armor(ob), get_att_power(ob): 장비·점수 등. Body의 방어력/공격력.
    let body_ptr_arm = body_ptr;
    engine.register_fn("get_armor", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_arm };
        body.get_armor() as i64
    });
    let body_ptr_max_hp = body_ptr;
    engine.register_fn("get_max_hp", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_max_hp };
        body.get_max_hp()
    });
    let body_ptr_persisted_max_hp = body_ptr;
    engine.register_fn("get_persisted_max_hp", move |ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_persisted_max_hp };
        let name = ob
            .get("이름")
            .and_then(|value| value.clone().into_string().ok())
            .unwrap_or_else(|| body.get_name());
        let path = format!("data/user/{}.json", name);
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|root| root.get("사용자오브젝트").cloned())
            .and_then(|user| user.get("최고체력").and_then(|value| value.as_i64()))
            .unwrap_or_else(|| body.get_int("최고체력"))
    });
    let body_ptr_max_mp = body_ptr;
    engine.register_fn("get_max_mp", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_max_mp };
        body.get_max_mp()
    });
    let body_ptr_total_exp = body_ptr;
    engine.register_fn("get_total_exp", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_total_exp };
        body.get_total_exp()
    });
    let body_ptr_arm_stat = body_ptr;
    engine.register_fn("get_arm", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_arm_stat };
        body.get_arm() as i64
    });
    let body_ptr_att = body_ptr;
    engine.register_fn("get_att_power", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_att };
        body.get_attack_power() as i64
    });

    // get_item_level_display(slot): 장비 슬롯 표기 문자열. "투구" -> "투    구" 등.
    engine.register_fn("get_item_level_display", |slot: &str| -> String {
        get_item_level_display(slot).to_string()
    });

    // set_act(ob, state): Python ACT_* 값과 동일.
    // Stand=0, Fight=1, Death=2, Regeneration=3, Rest=4, Move=5.
    let body_ptr_act = body_ptr;
    engine.register_fn(
        "set_act",
        move |_ob: &mut rhai::Map, state: rhai::Dynamic| {
            let body = unsafe { &mut *body_ptr_act };
            let n = if state.is_int() {
                state.as_int().unwrap_or(0)
            } else {
                let s = state.to_string();
                match s.trim() {
                    "서" | "stand" => 0,
                    "전투" | "fight" => 1,
                    "사망" | "death" => 2,
                    "재생" | "regen" => 3,
                    "휴식" | "rest" => 4,
                    "이동" | "move" => 5,
                    _ => 0,
                }
            };
            body.act = crate::player::ActState::from_i32(n as i32);
        },
    );

    // has_room_property(zone, room, prop): 방 맵속성에 prop 포함 여부. 쉬어(쉼금지) 등.
    engine.register_fn(
        "has_room_property",
        |zone: &str, room: i64, prop: &str| -> bool {
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return false,
            };
            if let Some(arc) = w.room_cache.get_room_cached(zone, &room.to_string()) {
                if let Ok(r) = arc.read() {
                    return r.properties.iter().any(|p| p == prop);
                }
            }
            false
        },
    );

    // has_room_property(zone, room, prop): &str 버전 (room이 문자열인 경우)
    engine.register_fn(
        "has_room_property",
        |zone: &str, room: &str, prop: &str| -> bool {
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return false,
            };
            if let Some(arc) = w.room_cache.get_room_cached(zone, room) {
                if let Ok(r) = arc.read() {
                    return r.properties.iter().any(|p| p == prop);
                }
            }
            false
        },
    );

    // get_exits_string(zone, room): 출구 나침반 문자열. 지도/맵 등.
    // i64 버전
    engine.register_fn("get_exits_string", |zone: &str, room: i64| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(zone, &room.to_string()) {
            if let Ok(r) = arc.read() {
                return format_exits_long(&r);
            }
        }
        String::new()
    });

    // get_exits_string(zone, room): 출구 나침반 문자열. 지도/맵 등.
    // &str 버전 (room이 문자열인 경우)
    engine.register_fn("get_exits_string", |zone: &str, room: &str| -> String {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(zone, room) {
            if let Ok(r) = arc.read() {
                return format_exits_long(&r);
            }
        }
        String::new()
    });

    // room_has_exit_named(zone, room, name): Python `name in ob.env.exitList`
    // 와 같은 정확한 출구명 검사. 렌더링된 나침반 문자열에 대한 부분 문자열
    // 검색은 "동"과 "북동"처럼 서로 겹치는 출구를 잘못 허용할 수 있다.
    engine.register_fn(
        "room_has_exit_named",
        |zone: &str, room: &str, name: &str| -> bool {
            let Ok(world) = get_world_state().read() else {
                return false;
            };
            world
                .room_cache
                .get_room_cached(zone, room)
                .and_then(|room| {
                    room.read().ok().map(|room| {
                        if let Some(raw) = name.strip_suffix('$') {
                            room.exits.get(raw).is_some_and(|exit| exit.hidden)
                        } else {
                            room.exits.contains_key(name)
                        }
                    })
                })
                .unwrap_or(false)
        },
    );
    engine.register_fn(
        "room_has_exit_named",
        |zone: &str, room: i64, name: &str| -> bool {
            let Ok(world) = get_world_state().read() else {
                return false;
            };
            world
                .room_cache
                .get_room_cached(zone, &room.to_string())
                .and_then(|room| {
                    room.read().ok().map(|room| {
                        if let Some(raw) = name.strip_suffix('$') {
                            room.exits.get(raw).is_some_and(|exit| exit.hidden)
                        } else {
                            room.exits.contains_key(name)
                        }
                    })
                })
                .unwrap_or(false)
        },
    );

    // parse_room_spec(s): "존:방번호" 파싱 → {zone, room}. 이동 등.
    engine.register_fn("parse_room_spec", |s: &str| -> Dynamic {
        let mut m = rhai::Map::new();
        let parts: Vec<&str> = s.trim().splitn(2, ':').collect();
        if parts.len() < 2 {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(0i64));
            return Dynamic::from(m);
        }
        let zone = parts[0].trim().to_string();
        let room = parts[1].trim().parse::<i64>().unwrap_or(0);
        m.insert("zone".into(), Dynamic::from(zone));
        m.insert("room".into(), Dynamic::from(room));
        Dynamic::from(m)
    });

    // get_position_of(player_name): 해당 플레이어의 {zone, room}. 없으면 {zone:"", room:0}. 앞(소환) 등.
    engine.register_fn("get_position_of", |name: &str| -> Dynamic {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Map::new().into(),
        };
        let mut m = rhai::Map::new();
        if let Some(p) = w.get_player_position(name) {
            m.insert("zone".into(), Dynamic::from(p.zone.clone()));
            m.insert("room".into(), Dynamic::from(p.room.clone()));
        } else {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // set_my_position(ob, zone, room): 기능만. ""=성공, "fail"|"same_place". 오류 메시지는 Rhai에서.
    let body_ptr_setpos = body_ptr;
    engine.register_fn(
        "set_my_position",
        move |_ob: &mut rhai::Map, zone: &str, room: rhai::Dynamic| -> String {
            let body = unsafe { &mut *body_ptr_setpos };
            let name = body.get_name();
            if name.is_empty() {
                return "fail".to_string();
            }
            let room_s = room.to_string();
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "fail".to_string(),
            };
            let cur = w.get_player_position(&name).cloned();
            let (cz, cr) = cur
                .as_ref()
                .map(|p| (p.zone.as_str(), p.room.as_str()))
                .unwrap_or(("", "0"));
            if cz == zone && cr == room_s {
                return "same_place".to_string();
            }
            if w.room_cache.get_room(zone, &room_s).is_err() {
                return "fail".to_string();
            }
            w.set_player_position(&name, PlayerPosition::new(zone.to_string(), room_s.clone()));
            w.spawn_mobs_for_room(zone, &room_s);
            let position = format!("{}:{}", zone, room_s);
            body.set("위치", position.as_str());
            body.set("현재방", position.as_str());
            String::new()
        },
    );

    // set_value(ob, key, val): Body에 키-값 저장. 점프(cooltime) 등. val은 정수 또는 문자열.
    let body_ptr_setv = body_ptr;
    engine.register_fn(
        "set_value",
        move |_ob: &mut rhai::Map, key: &str, val: rhai::Dynamic| {
            let body = unsafe { &mut *body_ptr_setv };
            if val.is_int() {
                body.set(key, val.as_int().unwrap_or(0));
            } else {
                body.set(key, val.to_string());
            }
        },
    );

    // 사용자 줄임말 efun. 상태/저장 변환만 Rust가 담당하고 모든 출력은
    // cmds/줄임말.rhai가 Python 문구 그대로 결정한다.
    let body_ptr_alias_count = body_ptr;
    engine.register_fn("alias_count", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_alias_count };
        decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR)).len() as i64
    });

    let body_ptr_alias_keys = body_ptr;
    engine.register_fn("alias_keys", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_alias_keys };
        decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR))
            .into_iter()
            .map(|(key, _)| Dynamic::from(key))
            .collect()
    });

    let body_ptr_alias_has = body_ptr;
    engine.register_fn("alias_has", move |_ob: &mut rhai::Map, key: &str| -> bool {
        let body = unsafe { &*body_ptr_alias_has };
        decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR))
            .iter()
            .any(|(saved_key, _)| saved_key == key)
    });

    let body_ptr_alias_get = body_ptr;
    engine.register_fn(
        "alias_get",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_alias_get };
            decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR))
                .into_iter()
                .find_map(|(saved_key, data)| (saved_key == key).then_some(data))
                .unwrap_or_default()
        },
    );
    let body_ptr_named_count = body_ptr;
    engine.register_fn(
        "inventory_named_count",
        move |_ob: &mut rhai::Map, wanted: &str| -> i64 {
            let body = unsafe { &*body_ptr_named_count };
            let mut count = body
                .object
                .objs
                .iter()
                .filter(|item| item.lock().is_ok_and(|item| item.getName() == wanted))
                .count() as i64;
            for (key, stacked) in &body.object.inv_stack {
                if get_item_info(key).is_some_and(|(name, _, _, _)| name == wanted) {
                    count += *stacked;
                }
            }
            count
        },
    );

    let body_ptr_alias_set = body_ptr;
    engine.register_fn(
        "alias_set",
        move |_ob: &mut rhai::Map, key: &str, data: &str| -> bool {
            let body = unsafe { &mut *body_ptr_alias_set };
            let mut entries = decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR));
            if entries.iter().any(|(saved_key, _)| saved_key == key) {
                return false;
            }
            entries.push((key.to_string(), data.to_string()));
            body.set(ALIAS_LIST_ATTR, encode_alias_entries(&entries));
            true
        },
    );

    let body_ptr_alias_del = body_ptr;
    engine.register_fn("alias_del", move |_ob: &mut rhai::Map, key: &str| -> bool {
        let body = unsafe { &mut *body_ptr_alias_del };
        let mut entries = decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR));
        let old_len = entries.len();
        entries.retain(|(saved_key, _)| saved_key != key);
        if entries.len() == old_len {
            return false;
        }
        body.set(ALIAS_LIST_ATTR, encode_alias_entries(&entries));
        true
    });

    // `자동경로`는 Player.autoMoveList를 소유하므로, 명령 실행 중에는
    // 네트워크 경계에서 적용할 경로 문자열만 요청한다.
    let body_ptr_auto_route = body_ptr;
    engine.register_fn(
        "request_auto_move_route",
        move |_ob: &mut rhai::Map, route: &str| {
            let body = unsafe { &mut *body_ptr_auto_route };
            body.temp_mut().insert(
                AUTO_MOVE_REQUEST.to_string(),
                Value::String(route.to_string()),
            );
        },
    );
    let body_ptr_auto_count = body_ptr;
    engine.register_fn("auto_move_count", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_auto_count };
        body.temp()
            .get("_auto_move_count")
            .and_then(|value| match value {
                Value::Int(count) => Some(*count),
                _ => None,
            })
            .unwrap_or(0)
    });

    let body_ptr_save_all = body_ptr;
    engine.register_fn("request_save_all", move |_ob: &mut rhai::Map| {
        unsafe { &mut *body_ptr_save_all }
            .temp_mut()
            .insert(SAVE_ALL_REQUEST.to_string(), Value::Int(1));
    });

    let body_ptr_space_attr = body_ptr;
    engine.register_fn(
        "admin_set_space_value",
        move |_ob: &mut rhai::Map, line: &str| -> String {
            let body = unsafe { &mut *body_ptr_space_attr };
            let input = line.trim_start();
            let Some(first_end) = input.find(char::is_whitespace) else {
                return "usage".into();
            };
            let target = &input[..first_end];
            let rest = input[first_end..].trim_start();
            let Some(second_end) = rest.find(char::is_whitespace) else {
                return "usage".into();
            };
            let key = &rest[..second_end];
            let raw = rest[second_end..].trim_start();
            if target.is_empty() || key.is_empty() || raw.is_empty() {
                return "usage".into();
            }
            // Python은 처음 `line.split()`한 words[2]만 길이를 검사한 뒤,
            // split(None, 2)로 나머지 전체를 값으로 사용한다.
            if raw
                .split_whitespace()
                .next()
                .is_some_and(|third| third.chars().count() > 50)
            {
                return "too_long".into();
            }
            python_set_admin_target(body, target, key, raw)
        },
    );

    // Python `값값`: comma-separated room target assignment with existing-type coercion.
    let body_ptr_comma_attr = body_ptr;
    engine.register_fn(
        "admin_set_comma_value",
        move |_ob: &mut rhai::Map, line: &str| -> String {
            let body = unsafe { &mut *body_ptr_comma_attr };
            let words: Vec<&str> = line.splitn(3, ',').collect();
            if line.is_empty() || words.len() < 3 {
                return "usage".into();
            }
            let (target, key, raw) = (words[0], words[1], words[2]);
            if raw.chars().count() > 20 {
                return "too_long".into();
            }
            let Some((zone, room)) = current_body_position(body) else {
                return "missing".into();
            };

            // Python Room.findObjName sees players as room objects. The executing
            // player can therefore be selected by name as well.
            if target == body.get_name() {
                let value = match python_coerce_attribute(body.object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                body.set(key, value);
                return "ok".into();
            }

            let room_objects = get_world_state()
                .read()
                .ok()
                .map(|world| world.get_room_objs(&zone, &room).to_vec())
                .unwrap_or_default();
            for object in room_objects {
                let Ok(mut object) = object.lock() else { continue };
                if object.getName() != target
                    && !object
                        .getString("반응이름")
                        .split("\r\n")
                        .any(|alias| alias == target)
                {
                    continue;
                }
                let value = match python_coerce_attribute(object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                object.set(key, value);
                return "ok".into();
            }

            let live_players = body
                .temp()
                .get("_online_room_admin")
                .and_then(Value::as_str)
                .map(str::to_string)
                .and_then(|json| serde_json::from_str::<Vec<serde_json::Value>>(&json).ok())
                .unwrap_or_default();
            if let Some(player) = live_players
                .iter()
                .find(|player| player.get("name").and_then(|value| value.as_str()) == Some(target))
            {
                let attrs = player.get("raw_attrs").and_then(|value| value.as_object());
                let existing = attrs.and_then(|attrs| attrs.get(key)).and_then(|value| {
                    if let Some(value) = value.as_i64() {
                        Some(Value::Int(value))
                    } else if let Some(value) = value.as_f64() {
                        Some(Value::Float(value))
                    } else {
                        value.as_str().map(|value| Value::String(value.to_string()))
                    }
                });
                let value = match python_coerce_attribute(existing.as_ref(), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                let json_value = match value {
                    Value::Int(value) => serde_json::Value::Number(value.into()),
                    Value::Float(value) => serde_json::Number::from_f64(value)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    Value::String(value) => serde_json::Value::String(value),
                };
                body.temp_mut().insert(
                    ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
                    Value::String(
                        serde_json::to_string(&(
                            target.to_string(),
                            key.to_string(),
                            json_value,
                        ))
                        .unwrap_or_default(),
                    ),
                );
                return "ok".into();
            }

            let mob_id = get_world_state().read().ok().and_then(|world| {
                world
                    .mob_cache
                    .get_all_mobs_in_room(&zone, &room)
                    .into_iter()
                    .find_map(|mob| {
                        let data = world.get_mob_data(&mob.mob_key)?;
                        (mob.name == target
                            || data.name == target
                            || data.reaction_names.iter().any(|alias| alias == target))
                            .then_some(mob.instance_id)
                    })
            });
            let Some(mob_id) = mob_id else {
                return "missing".into();
            };
            let mut world = get_world_state().write().unwrap();
            let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
                return "missing".into();
            };
            let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == mob_id) else {
                return "missing".into();
            };
            let existing = match key {
                "이름" => Some(Value::String(mob.name.clone())),
                "체력" => Some(Value::Int(mob.hp)),
                "최고체력" => Some(Value::Int(mob.max_hp)),
                "내공" => Some(Value::Int(mob.mp)),
                "최고내공" => Some(Value::Int(mob.max_mp)),
                "은전" => Some(Value::Int(mob.gold)),
                "레벨" => Some(Value::Int(mob.level)),
                "힘" => Some(Value::Int(mob.strength)),
                "맷집" => Some(Value::Int(mob.arm)),
                "민첩성" => Some(Value::Int(mob.agility)),
                _ => mob.runtime_attrs.get(key).cloned(),
            };
            let value = match python_coerce_attribute(existing.as_ref(), raw) {
                Ok(value) => value,
                Err(()) => return "invalid".into(),
            };
            match (key, &value) {
                ("이름", Value::String(value)) => mob.name = value.clone(),
                ("체력", Value::Int(value)) => mob.hp = *value,
                ("최고체력", Value::Int(value)) => mob.max_hp = *value,
                ("내공", Value::Int(value)) => mob.mp = *value,
                ("최고내공", Value::Int(value)) => mob.max_mp = *value,
                ("은전", Value::Int(value)) => mob.gold = *value,
                ("레벨", Value::Int(value)) => mob.level = *value,
                ("힘", Value::Int(value)) => mob.strength = *value,
                ("맷집", Value::Int(value)) => mob.arm = *value,
                ("민첩성", Value::Int(value)) => mob.agility = *value,
                _ => {
                    mob.runtime_attrs.insert(key.to_string(), value);
                }
            }
            "ok".into()
        },
    );

    // set_obj_attr(ob, target, key, val): 기능만. 대상에 속성 설정. 성공 true. 오류 메시지는 Rhai에서 send_line.
    let body_ptr_soa = body_ptr;
    let body_ptr_gra = body_ptr;
    engine.register_fn(
        "get_room_attr",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_gra };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|w| w.get_player_position(&body.get_name()).cloned())
            else {
                return String::new();
            };
            if let Some(value) = get_world_state()
                .read()
                .ok()
                .and_then(|w| {
                    w.room_attrs
                        .get(&format!("{}:{}", pos.zone, pos.room))
                        .and_then(|m| m.get(key))
                        .cloned()
                })
            {
                return value;
            }
            let path = format!("data/map/{}/{}.json", pos.zone, pos.room);
            std::fs::read_to_string(path)
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|root| root.get("맵정보").and_then(|info| info.get(key)).cloned())
                .map(|value| match value {
                    serde_json::Value::String(value) => value,
                    serde_json::Value::Array(values) => values
                        .into_iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                        .join("\r\n"),
                    serde_json::Value::Number(value) => value.to_string(),
                    serde_json::Value::Bool(value) => value.to_string(),
                    _ => String::new(),
                })
                .unwrap_or_default()
        },
    );
    engine.register_fn(
        "set_obj_attr",
        move |ob: &mut rhai::Map, target: &str, key: &str, val: rhai::Dynamic| -> bool {
            let body = unsafe { &mut *body_ptr_soa };
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let val_str = if val.is_int() {
                val.as_int().unwrap_or(0).to_string()
            } else {
                val.to_string()
            };
            let v: crate::object::Value = if val.is_int() {
                (val.as_int().unwrap_or(0)).into()
            } else {
                val_str.as_str().into()
            };
            if target == "방" {
                let pos = match get_world_state()
                    .read()
                    .ok()
                    .and_then(|w| w.get_player_position(&my_name).cloned())
                {
                    Some(p) => p,
                    None => return false,
                };
                get_world_state()
                    .write()
                    .unwrap()
                    .get_room_attrs_mut(&pos.zone, &pos.room)
                    .insert(key.to_string(), val_str.clone());
                let path = format!("data/map/{}/{}.json", pos.zone, pos.room);
                let persisted = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .and_then(|mut root| {
                        root.get_mut("맵정보")?
                            .as_object_mut()?
                            .insert(key.to_string(), serde_json::Value::String(val_str));
                        serde_json::to_string_pretty(&root).ok()
                    })
                    .is_some_and(|saved| std::fs::write(path, saved).is_ok());
                if !persisted {
                    return false;
                }
                return true;
            }
            if target == "나" || target == my_name {
                body.set(key, v);
                return true;
            }
            for arc in &body.object.objs {
                if let Ok(o) = arc.lock() {
                    if o.getName() == target
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(target))
                    {
                        drop(o);
                        if let Ok(mut obj) = arc.lock() {
                            obj.set(key, v);
                        }
                        return true;
                    }
                }
            }
            if let Some((zone, room)) = get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&my_name)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                let mut w = get_world_state().write().unwrap();
                let room_list = w.get_room_objs_mut(&zone, &room);
                for arc in room_list.iter_mut() {
                    if let Ok(o) = arc.lock() {
                        if o.getName() == target
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(target))
                        {
                            drop(o);
                            if let Ok(mut obj) = arc.lock() {
                                obj.set(key, v);
                            }
                            return true;
                        }
                    }
                }
            }
            false
        },
    );

    // Python `값삭제`: env.findObjName만 사용하고 소지품으로 fallback하지 않는다.
    // 대상 없음과 키 없음을 Rhai가 서로 다른 Python 문구로 표시할 수 있게 구분한다.
    let body_ptr_doa = body_ptr;
    engine.register_fn(
        "admin_delete_space_value",
        move |_ob: &mut rhai::Map, line: &str| -> String {
            let body = unsafe { &mut *body_ptr_doa };
            let input = line.trim_start();
            let Some(first_end) = input.find(char::is_whitespace) else {
                return "usage".into();
            };
            let target = &input[..first_end];
            let key = input[first_end..].trim_start();
            if target.is_empty() || key.is_empty() {
                return "usage".into();
            }
            let my_name = body.get_name();
            if target == "방" {
                let (zone, room) = match get_world_state().read().ok().and_then(|w| {
                    w.get_player_position(&my_name)
                        .map(|p| (p.zone.clone(), p.room.clone()))
                }) {
                    Some(x) => x,
                    None => return "missing".into(),
                };
                let mut w = get_world_state().write().unwrap();
                let attrs = w.get_room_attrs_mut(&zone, &room);
                return if attrs.remove(key).is_some() { "ok" } else { "no_key" }.into();
            }
            if let Some((zone, room)) = get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&my_name)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                let mut w = get_world_state().write().unwrap();
                let room_list = w.get_room_objs_mut(&zone, &room);
                for arc in room_list.iter_mut() {
                    if let Ok(o) = arc.lock() {
                        if o.getName() == target
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(target))
                        {
                            drop(o);
                            if let Ok(mut obj) = arc.lock() {
                                return if obj.attr.remove(key).is_some() { "ok" } else { "no_key" }.into();
                            }
                        }
                    }
                }

                let mob_id = w.mob_cache.get_all_mobs_in_room(&zone, &room).into_iter().find_map(|mob| {
                    let data = w.get_mob_data(&mob.mob_key)?;
                    (mob.name == target || data.name == target || data.reaction_names.iter().any(|alias| alias == target || alias.starts_with(target)))
                        .then_some(mob.instance_id)
                });
                if let Some(mob_id) = mob_id {
                    let Some(mobs) = w.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else { return "missing".into(); };
                    let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == mob_id) else { return "missing".into(); };
                    return if mob.runtime_attrs.remove(key).is_some() { "ok" } else { "no_key" }.into();
                }
            }
            if target == my_name {
                return if body.attr_mut().remove(key).is_some() { "ok" } else { "no_key" }.into();
            }
            "missing".into()
        },
    );

    // remove_obj_attr_value: Python checkAttr/delAttr처럼 특정 값만 제거한다.
    let body_ptr_roav = body_ptr;
    engine.register_fn(
        "remove_obj_attr_value",
        move |_ob: &mut rhai::Map, target: &str, key: &str, wanted: &str| -> String {
            let body = unsafe { &mut *body_ptr_roav };
            let name = body.get_name();
            let remove_from = |attr: &mut HashMap<String, crate::object::Value>| -> String {
                let Some(crate::object::Value::String(raw)) = attr.get(key).cloned() else {
                    return if attr.contains_key(key) {
                        "not_value"
                    } else {
                        "no_key"
                    }
                    .to_string();
                };
                let values: Vec<&str> = raw.split("\r\n").collect();
                if !values.contains(&wanted) {
                    return "not_value".to_string();
                }
                let kept: Vec<&str> = values.into_iter().filter(|v| *v != wanted).collect();
                if kept.is_empty() {
                    attr.remove(key);
                } else {
                    attr.insert(key.to_string(), kept.join("\r\n").into());
                }
                "ok".to_string()
            };
            if target == "방" {
                if let Some(pos) = get_world_state()
                    .read()
                    .ok()
                    .and_then(|w| w.get_player_position(&name).cloned())
                {
                    let mut w = get_world_state().write().unwrap();
                    let attrs = w.get_room_attrs_mut(&pos.zone, &pos.room);
                    if let Some(raw) = attrs.get(key).cloned() {
                        let values: Vec<&str> = raw.split("\r\n").collect();
                        if !values.contains(&wanted) {
                            return "not_value".into();
                        }
                        let kept: Vec<&str> = values.into_iter().filter(|v| *v != wanted).collect();
                        if kept.is_empty() {
                            attrs.remove(key);
                        } else {
                            attrs.insert(key.into(), kept.join("\r\n"));
                        }
                        return "ok".into();
                    }
                }
                return "no_key".into();
            }
            if target == "나" || target == name {
                return remove_from(&mut body.object.attr);
            }
            for arc in &body.object.objs {
                if let Ok(mut obj) = arc.lock() {
                    if obj.getName() == target || obj.getString("반응이름").contains(target) {
                        return remove_from(&mut obj.attr);
                    }
                }
            }
            if let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|w| w.get_player_position(&name).cloned())
            {
                let room_objects = get_world_state()
                    .read()
                    .ok()
                    .map(|w| w.get_room_objs(&pos.zone, &pos.room).to_vec())
                    .unwrap_or_default();
                for arc in room_objects {
                    if let Ok(mut obj) = arc.lock() {
                        if obj.getName() == target || obj.getString("반응이름").contains(target)
                        {
                            return remove_from(&mut obj.attr);
                        }
                    }
                }
            }
            "no_target".into()
        },
    );

    // Python 출구숨김은 같은 명령으로 숨김/드러냄을 토글하고 방 파일을 저장한다.
    engine.register_fn("exit_hide", move |ob: &mut rhai::Map, name: &str| -> String {
        let name_body = ob
            .get("이름")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let (zone, room) = match get_world_state().read().ok().and_then(|w| {
            w.get_player_position(&name_body)
                .map(|p| (p.zone.clone(), p.room.clone()))
        }) {
            Some(x) => x,
            None => return "missing".into(),
        };
        let status = rewrite_room_exits(&zone, &room, |exits| {
            for exit in exits {
                let Some((raw_name, destination)) = exit.split_once(char::is_whitespace) else {
                    continue;
                };
                if raw_name.trim_end_matches('$') != name {
                    continue;
                }
                let hidden = raw_name.ends_with('$');
                *exit = format!(
                    "{}{} {}",
                    name,
                    if hidden { "" } else { "$" },
                    destination.trim()
                );
                return if hidden { "shown" } else { "hidden" }.to_string();
            }
            "missing".to_string()
        });
        if status != "missing" {
            if let Ok(mut world) = get_world_state().write() {
                if let Ok(room_arc) = world.room_cache.get_room(&zone, &room) {
                    let _ = room_arc
                        .write()
                        .unwrap()
                        .set_exit_hidden(name, status == "hidden");
                }
            }
        }
        status
    });

    // exit_show(ob, name): 출구 드러냄. 성공 true.
    let _oc_es = oc.clone();
    engine.register_fn("exit_show", move |ob: &mut rhai::Map, name: &str| -> bool {
        let name_body = ob
            .get("이름")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let (zone, room) = match get_world_state().read().ok().and_then(|w| {
            w.get_player_position(&name_body)
                .map(|p| (p.zone.clone(), p.room.clone()))
        }) {
            Some(x) => x,
            None => return false,
        };
        let mut w = match get_world_state().write() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let room_arc = match w.room_cache.get_room(&zone, &room.to_string()) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let ok = room_arc.write().unwrap().set_exit_hidden(name, false);
        ok
    });

    // exit_remove(ob, name): 기능만. 출구제거. 성공 true. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn(
        "exit_remove",
        move |ob: &mut rhai::Map, name: &str| -> bool {
            let name_body = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let (zone, room) = match get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&name_body)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                Some(x) => x,
                None => return false,
            };
            let status = rewrite_room_exits(&zone, &room, |exits| {
                let before = exits.len();
                exits.retain(|exit| {
                    exit.split_whitespace()
                        .next()
                        .map(|raw| raw.trim_end_matches('$') != name)
                        .unwrap_or(true)
                });
                if exits.len() < before {
                    "removed".to_string()
                } else {
                    "missing".to_string()
                }
            });
            if status == "missing" {
                return false;
            }
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let room_arc = match w.room_cache.get_room(&zone, &room) {
                Ok(r) => r,
                Err(_) => return false,
            };
            let ok = room_arc.write().unwrap().remove_exit(name);
            ok
        },
    );

    // exit_set_wander(ob, name): 기능만. 맴돌이. 출구 목적지를 자기 방으로. 성공 true. 오류 메시지는 Rhai에서 send_line.
    engine.register_fn(
        "exit_set_wander",
        move |ob: &mut rhai::Map, name: &str| -> bool {
            let name_body = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let (zone, room) = match get_world_state().read().ok().and_then(|w| {
                w.get_player_position(&name_body)
                    .map(|p| (p.zone.clone(), p.room.clone()))
            }) {
                Some(x) => x,
                None => return false,
            };
            let destination = format!("{zone}:{room}");
            let status = rewrite_room_exits(&zone, &room, |exits| {
                let mut updated = false;
                for exit in exits {
                    let Some(raw_name) = exit.split_whitespace().next() else {
                        continue;
                    };
                    if raw_name.trim_end_matches('$') == name {
                        *exit = format!("{raw_name} {destination}");
                        updated = true;
                    }
                }
                if updated {
                    "updated".to_string()
                } else {
                    "missing".to_string()
                }
            });
            if status == "missing" {
                return false;
            }
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let room_arc = match w.room_cache.get_room(&zone, &room) {
                Ok(r) => r,
                Err(_) => return false,
            };
            let ok = room_arc
                .write()
                .unwrap()
                .set_exit_destination(name, &zone, &room);
            ok
        },
    );

    // mob_regen(ob, name): 리젠. 시체만 가능. 성공 true.
    engine.register_fn("mob_regen", move |ob: &mut rhai::Map, name: &str| -> bool {
        let name_body = ob
            .get("이름")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let (zone, room) = match get_world_state().read().ok().and_then(|w| {
            w.get_player_position(&name_body)
                .map(|p| (p.zone.clone(), p.room.clone()))
        }) {
            Some(x) => x,
            None => return false,
        };
        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .do_regen(&zone, &room, name)
    });

    // guild_get(id, key), guild_set(id, key, value), guild_attr_keys(id), guild_list(), guild_has(id), guild_remove(id), guild_save()
    // guild_list는 Vec<String> 대신 rhai::Array 반환 (len, [] 연산 호환)
    engine.register_fn("guild_get", guild_get);
    engine.register_fn("guild_set", guild_set);
    engine.register_fn("guild_attr_keys", guild_attr_keys);
    engine.register_fn("guild_list", || -> rhai::Array {
        guild_list().into_iter().map(rhai::Dynamic::from).collect()
    });
    engine.register_fn("guild_has", guild_has);
    engine.register_fn("guild_remove", guild_remove);
    engine.register_fn("guild_save", guild_save);

    // rank_write(ty, name, value, level), rank_read(ty, name), rank_get_num(ty, rank), rank_get_all(ty), rank_clear(ty). ty 빈 문자열이면 전체.
    engine.register_fn("rank_write", rank_write);
    engine.register_fn("rank_read", rank_read);
    engine.register_fn("rank_get_num", rank_get_num);
    engine.register_fn("rank_get_all", rank_get_all);
    engine.register_fn("rank_clear", rank_clear);
    engine.register_fn("live_rank_entries", move |attribute: &str| -> rhai::Array {
        let online = get_precomputed_all_online();
        let mut entries: Vec<(usize, String, i64)> = online
            .into_iter()
            .enumerate()
            .filter_map(|(index, value)| {
                let map = value.try_cast::<rhai::Map>()?;
                let name = map.get("이름")?.clone().into_string().ok()?;
                if name.is_empty() {
                    return None;
                }
                let admin = map
                    .get("관리자등급")
                    .and_then(|value| value.as_int().ok())
                    .unwrap_or(0);
                if admin != 0 {
                    return None;
                }
                let value = map
                    .get(attribute)
                    .and_then(|value| {
                        value.as_int().ok().or_else(|| value.to_string().parse::<i64>().ok())
                    })
                    .unwrap_or(0);
                (value != 0).then_some((index, name, value))
            })
            .collect();
        entries.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| left.0.cmp(&right.0))
        });
        entries
            .into_iter()
            .take(30)
            .map(|(_, name, value)| {
                let mut entry = rhai::Map::new();
                entry.insert("name".into(), Dynamic::from(name));
                entry.insert("value".into(), Dynamic::from(value));
                Dynamic::from(entry)
            })
            .collect()
    });
    let body_ptr_compare = body_ptr;
    engine.register_fn(
        "compare_combat_target",
        move |_ob: &mut rhai::Map, input: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_compare };
            let finish = |status: &str, name: String, mine: i64, other: i64| {
                let mut result = rhai::Map::new();
                result.insert("status".into(), Dynamic::from(status.to_string()));
                result.insert("name".into(), Dynamic::from(name));
                result.insert("mine".into(), Dynamic::from(mine));
                result.insert("other".into(), Dynamic::from(other));
                Dynamic::from(result)
            };
            if input == body.get_name() {
                return finish("self", String::new(), 0, 0);
            }
            let Some((zone, room)) = current_body_position(body) else {
                return finish("invalid", String::new(), 0, 0);
            };
            let random_damage = |base: i64| {
                let base = base.max(1);
                let low = base * 80 / 100;
                let high = base * 120 / 100;
                rand::thread_rng().gen_range(low..=high).max(1)
            };
            let player_base = |target_arm: i64, target_armor: i64| {
                body.get_str() * 2 + body.get_max_mp() / 5
                    + i64::from(body.get_attack_power())
                    - body.get_mastery_diff()
                    - target_arm
                    - target_armor
            };
            if get_world_state()
                .read()
                .ok()
                .is_some_and(|world| {
                    world.get_room_objs(&zone, &room).iter().any(|item| {
                        item.lock().is_ok_and(|item| {
                            item.getName() == input
                                || reaction_names(&item.getString("반응이름"))
                                    .iter()
                                    .any(|alias| alias == input)
                        })
                    })
                })
            {
                return finish("invalid", String::new(), 0, 0);
            }
            let mob_target = get_world_state().read().ok().and_then(|world| {
                world
                    .mob_cache
                    .get_all_mobs_in_room(&zone, &room)
                    .into_iter()
                    .find_map(|mob| {
                        let data = world.get_mob_data(&mob.mob_key)?;
                        (mob.name == input
                            || data.name == input
                            || data.reaction_names.iter().any(|alias| alias == input))
                            .then_some((mob.clone(), data.clone()))
                    })
            });
            if let Some((mob, data)) = mob_target {
                if data.mob_type == 7 {
                    return finish("hidden", String::new(), 0, 0);
                }
                if config_is_enabled(&body.get_string("설정상태"), "비교거부") {
                    return finish("refused", String::new(), 0, 0);
                }
                let (mob_attack, mob_armor) = data.use_items.iter().fold(
                    (0_i64, 0_i64),
                    |(attack, armor), (key, _, _, _)| {
                        let info = std::fs::read_to_string(format!("data/item/{key}.json"))
                            .ok()
                            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                            .and_then(|root| root.get("아이템정보").cloned());
                        let Some(info) = info else { return (attack, armor) };
                        (
                            attack
                                + info
                                    .get("공격력")
                                    .or_else(|| info.get("타격"))
                                    .and_then(serde_json::Value::as_i64)
                                    .unwrap_or(0),
                            armor
                                + info
                                    .get("방어력")
                                    .and_then(serde_json::Value::as_i64)
                                    .unwrap_or(0),
                        )
                    },
                );
                let my_damage = random_damage(player_base(mob.arm, mob_armor));
                let mob_damage = random_damage(
                    mob.strength * 2 + mob_attack
                        - body.get_arm()
                        - i64::from(body.get_armor()),
                );
                return finish(
                    "ok",
                    mob.name,
                    body.get_max_hp() / mob_damage,
                    mob.hp / my_damage,
                );
            }
            let target = get_precomputed_all_online().into_iter().find_map(|value| {
                let map = value.try_cast::<rhai::Map>()?;
                let name = map.get("이름")?.clone().into_string().ok()?;
                let same_room = map.get("zone")?.to_string() == zone
                    && map.get("room")?.to_string() == room;
                (same_room && (name == input || name.starts_with(input)))
                    .then_some((name, map))
            });
            let Some((target_name, target)) = target else {
                return finish("invalid", String::new(), 0, 0);
            };
            let target_config = target
                .get("설정상태")
                .map(Dynamic::to_string)
                .unwrap_or_default();
            if config_is_enabled(&body.get_string("설정상태"), "비교거부")
                || config_is_enabled(&target_config, "비교거부")
            {
                return finish("refused", String::new(), 0, 0);
            }
            let get = |key: &str| {
                target
                    .get(key)
                    .and_then(|value| value.as_int().ok())
                    .unwrap_or(0)
            };
            let my_damage = random_damage(player_base(get("맷집"), get("방어력")));
            let target_damage = random_damage(
                get("힘") * 2 + get("최고내공") / 5 + get("공격력")
                    - get("숙련도차이")
                    - body.get_arm()
                    - i64::from(body.get_armor()),
            );
            finish(
                "ok",
                target_name,
                body.get_max_hp() / target_damage,
                get("최고체력") / my_damage,
            )
        },
    );

    // password_hash(plain): 평문을 bcrypt 문자열로 해시. 암호 저장/암호변경용.
    engine.register_fn("password_hash", |plain: &str| -> String {
        password_hash(plain)
    });
    // password_verify(stored, plain): 저장된 해시(또는 레거시 평문)와 평문 일치 여부. 암호변경 검증용.
    engine.register_fn("password_verify", |stored: &str, plain: &str| -> bool {
        password_verify(stored, plain)
    });
    // verify_password(ob, plain): Body 암호와 평문 일치 여부. 해시를 스크립트에 노출하지 않고 검증.
    let body_ptr_vp = body_ptr;
    engine.register_fn(
        "verify_password",
        move |_ob: &mut rhai::Map, plain: &str| -> bool {
            let body = unsafe { &*body_ptr_vp };
            let stored = body.get_string("암호");
            password_verify(&stored, plain)
        },
    );
    // parse_two_args(s): 첫 공백 기준 [앞, 뒤]. "a b c" -> ["a","b c"]. "a" -> ["a",""].
    engine.register_fn("parse_two_args", |s: &str| -> rhai::Array {
        let parts: Vec<&str> = s.splitn(2, ' ').collect();
        vec![
            rhai::Dynamic::from(parts.first().copied().unwrap_or("").to_string()),
            rhai::Dynamic::from(parts.get(1).copied().unwrap_or("").to_string()),
        ]
    });

    // get_body_int(ob, key): Body에서 정수 읽기. Map에 없는 런타임 키(예: cooltime)용.
    let body_ptr_getbi = body_ptr;
    engine.register_fn(
        "get_body_int",
        move |_ob: &mut rhai::Map, key: &str| -> i64 {
            let body = unsafe { &*body_ptr_getbi };
            body.get_int(key)
        },
    );

    // get_body_string(ob, key): Body에서 문자열 읽기. set_value로 넣은 키(예: 위치각인, 꼬리말)용.
    let body_ptr_getbs = body_ptr;
    engine.register_fn(
        "get_body_string",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_getbs };
            body.get_string(key)
        },
    );

    // get_body_attrs_json(ob): 관리자 진단용 원본 속성 JSON. 출력 여부/형식은 Rhai가 결정한다.
    let body_ptr_attrs_json = body_ptr;
    engine.register_fn(
        "get_body_attrs_json",
        move |_ob: &mut rhai::Map| -> String {
            let body = unsafe { &*body_ptr_attrs_json };
            let attrs: serde_json::Map<String, serde_json::Value> = body
                .object
                .attr
                .iter()
                .map(|(key, value)| (key.clone(), value_to_serde_json(value)))
                .collect();
            serde_json::to_string_pretty(&serde_json::Value::Object(attrs)).unwrap_or_default()
        },
    );

    // ---- 외쳐/전음/표현/주다: special_collector에 CommandResult 설정, handler에서 Shout/Tell/EmotionToRoom/GiveToPlayer 처리 ----
    // send_to_user(name, msg): 해당 접속자에게 msg 전송. 스크립트에서 포맷·조건(외침거부 등) 정한 뒤 호출.

    let user_sends_clone = user_sends.clone();
    engine.register_fn("send_to_user", move |name: &str, msg: &str| {
        if !name.is_empty() && !msg.is_empty() {
            if let Ok(mut u) = user_sends_clone.lock() {
                u.push((name.to_string(), msg.to_string()));
            }
        }
    });

    // 암호변경 대화 시작. 모든 문구와 줄바꿈은 Rhai가 전달하고,
    // Rust는 이전 암호 → 새 암호 → 확인 상태만 보존한다.
    let spec_password = spec.clone();
    engine.register_fn(
        "begin_password_change",
        move |_ob: &mut rhai::Map,
              old_prompt: &str,
              wrong_password: &str,
              new_password_prompt: &str,
              confirm_prompt: &str,
              mismatch: &str,
              success: &str| {
            let text = crate::command::handler::PasswordChangeText {
                wrong_password: wrong_password.to_string(),
                new_password_prompt: new_password_prompt.to_string(),
                confirm_prompt: confirm_prompt.to_string(),
                mismatch: mismatch.to_string(),
                success: success.to_string(),
            };
            if let Ok(mut special) = spec_password.lock() {
                *special = Some(CommandResult::RequestInput {
                    prompt: old_prompt.to_string(),
                    state: crate::command::PendingInput::ChangePasswordOld { text },
                });
            }
        },
    );

    // 방설명 입력 시작. Rust는 입력 상태만 보존하고 시작 문구는 Rhai가 전달한다.
    let spec_room_description = spec.clone();
    let body_ptr_room_description = body_ptr;
    engine.register_fn(
        "begin_room_description",
        move |_ob: &mut rhai::Map, prompt: &str| -> bool {
            let body = unsafe { &*body_ptr_room_description };
            let Some(position) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                return false;
            };
            if let Ok(mut special) = spec_room_description.lock() {
                *special = Some(CommandResult::RequestInput {
                    prompt: prompt.to_string(),
                    state: crate::command::PendingInput::RoomDescription {
                        zone: position.zone,
                        room: position.room,
                        lines: Vec::new(),
                    },
                });
                true
            } else {
                false
            }
        },
    );

    let spec_file_edit = spec.clone();
    engine.register_fn(
        "begin_file_edit",
        move |_ob: &mut rhai::Map, relative_path: &str, prompt: &str| -> bool {
            if relative_path.is_empty() || relative_path.contains("..") {
                return false;
            }
            if let Ok(mut special) = spec_file_edit.lock() {
                *special = Some(CommandResult::RequestInput {
                    prompt: prompt.to_string(),
                    state: crate::command::PendingInput::FileEdit {
                        relative_path: relative_path.to_string(),
                        lines: Vec::new(),
                    },
                });
                true
            } else {
                false
            }
        },
    );

    // 쪽지 제목 분리. Python `line.split(None, 1)`과 동일하게
    // 성공 시 [수신자, 제목], 실패 시 빈 배열을 반환한다.
    engine.register_fn("parse_note_header", |line: &str| -> rhai::Array {
        crate::command::commands::split_recipient_subject(line)
            .map(|(recipient, subject)| vec![Dynamic::from(recipient), Dynamic::from(subject)])
            .unwrap_or_default()
    });

    // 도착한 쪽지 데이터를 반환하고 Python `ob.memo = {}`처럼
    // 메모리에서만 비운다. 이 시점에는 파일을 저장하지 않는다.
    let body_ptr_note_view = body_ptr;
    engine.register_fn(
        "take_received_notes",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &mut *body_ptr_note_view };
            let mut memos: Vec<_> = std::mem::take(&mut body.memos).into_iter().collect();
            // Python save_script(sort_keys=True) → json.load(dict) 순서와 동일.
            memos.sort_by(|(left, _), (right, _)| left.cmp(right));
            memos
                .into_iter()
                .map(|(_, memo)| memo)
                .map(|memo| {
                    let mut data = rhai::Map::new();
                    data.insert("제목".into(), Dynamic::from(memo.제목));
                    data.insert("시간".into(), Dynamic::from(memo.시간));
                    data.insert("작성자".into(), Dynamic::from(memo.작성자));
                    data.insert("내용".into(), Dynamic::from(memo.내용));
                    Dynamic::from(data)
                })
                .collect()
        },
    );

    // 쪽지 편집 시작. Rust는 수신자 로드·중복 확인·저장과
    // 입력 상태만 담당하며 모든 출력은 Rhai가 인자로 제공한다.
    let body_ptr_note_begin = body_ptr;
    let spec_note = spec.clone();
    engine.register_fn(
        "begin_note_edit",
        move |_ob: &mut rhai::Map,
              recipient_name: &str,
              subject: &str,
              initial_prompt: &str,
              target_connected: &str,
              capacity_exceeded: &str,
              complete: &str,
              continue_prompt: &str|
              -> String {
            let sender = unsafe { &*body_ptr_note_begin }.get_name();
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let recipient = match crate::command::commands::begin_note_in_dir(
                &sender,
                recipient_name,
                subject,
                &timestamp,
                Path::new("data/user"),
            ) {
                Ok(recipient) => recipient,
                Err(crate::command::commands::BeginNoteError::NotFound) => {
                    return "not_found".to_string();
                }
                Err(crate::command::commands::BeginNoteError::Duplicate) => {
                    return "duplicate".to_string();
                }
            };

            let text = crate::command::handler::NoteEditText {
                target_connected: target_connected.to_string(),
                capacity_exceeded: capacity_exceeded.to_string(),
                complete: complete.to_string(),
                continue_prompt: continue_prompt.to_string(),
            };
            if let Ok(mut special) = spec_note.lock() {
                *special = Some(CommandResult::RequestInput {
                    prompt: initial_prompt.to_string(),
                    state: crate::command::PendingInput::NoteEdit {
                        recipient,
                        body: String::new(),
                        text,
                    },
                });
            }
            String::new()
        },
    );

    // send_notice(ob, msg): Rhai가 완성한 공지 바이트를 ACTIVE 접속자에게 전달한다.
    // 문구·ANSI·정렬은 변경 가능한 표현이므로 Rust에서 생성하지 않는다.
    let spec_not = spec.clone();
    engine.register_fn(
        "send_notice",
        move |_ob: &mut rhai::Map, msg: &str| -> String {
            if let Ok(mut s) = spec_not.lock() {
                *s = Some(CommandResult::Notice(msg.to_string()));
            }
            "".to_string()
        },
    );

    // send_broadcast_to_guild(ob, msg): 기능만. [방파] 이름 : 메시지. ""=성공, "usage"=빈 msg, "no_guild"=소속 없음. 오류 메시지는 Rhai에서.
    let spec_bg = spec.clone();
    engine.register_fn(
        "send_broadcast_to_guild",
        move |ob: &mut rhai::Map, msg: &str| -> String {
            if msg.trim().is_empty() {
                return "usage".to_string();
            }
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let guild = ob
                .get("소속")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            if guild.is_empty() {
                return "no_guild".to_string();
            }
            let arr = get_precomputed_all_online();
            let mut names: Vec<String> = Vec::new();
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    let s: String = m
                        .get("소속")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if s == guild {
                        if let Some(n) = m
                            .get("이름")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        {
                            if !n.is_empty() {
                                names.push(n);
                            }
                        }
                    }
                }
            }
            let formatted = format!("\x1b[0;35m[방파]\x1b[0;37m {} : {}", my_name, msg);
            if let Ok(mut s) = spec_bg.lock() {
                *s = Some(CommandResult::BroadcastToPlayers(names, formatted));
            }
            "".to_string()
        },
    );
    let body_ptr_guild_log = body_ptr;
    engine.register_fn(
        "append_guild_chat_log",
        move |_ob: &mut rhai::Map, line: &str| -> bool {
            use std::io::Write;
            let body = unsafe { &*body_ptr_guild_log };
            let guild = body.get_string("소속");
            if guild.is_empty() || guild.contains('/') || guild.contains('\\') {
                return false;
            }
            let directory = std::path::Path::new("data/log/group");
            let _ = std::fs::create_dir_all(directory);
            let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join(&guild))
            else {
                return false;
            };
            let timestamp = chrono::Local::now().format("[%Y-%m-%d %H:%M:%S] ");
            writeln!(file, "{}{:10}: {}", timestamp, body.get_name(), line).is_ok()
        },
    );

    // Python 전음 대상 조회. ACTIVE이면서 투명하지 않은 정확한 이름만
    // 찾는다. 반환 값은 Rhai가 조건과 모든 출력 포맷을 결정하는 데 쓰며,
    // opaque token 외의 네트워크 식별자는 노출하지 않는다.
    engine.register_fn("find_tell_player", |target: &str| -> rhai::Map {
        PRE_COMPUTED_TELL_PLAYERS.with(|cell| {
            cell.borrow()
                .as_ref()
                .and_then(|players| {
                    players.iter().find(|player| {
                        player.active && !player.transparent && player.name == target
                    })
                })
                .map(TellPlayerSnapshot::to_rhai_map)
                .unwrap_or_else(missing_tell_player)
        })
    });

    // Python `ob._talker` 조회. 반전음은 ACTIVE/투명 여부를 다시 검사하지
    // 않고 동일 접속 객체가 channel.players에 남아 있는지만 확인한다.
    // 사라진 객체이면 Python처럼 현재 사용자의 `_talker`를 즉시 비운다.
    let body_ptr_reply_tell = body_ptr;
    engine.register_fn(
        "find_reply_tell_player",
        move |_ob: &mut rhai::Map| -> rhai::Map {
            let body = unsafe { &mut *body_ptr_reply_tell };
            let token = body
                .temp()
                .get(TELL_TALKER_TOKEN)
                .and_then(|value| match value {
                    Value::String(value) => Some(value.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if token.is_empty() {
                return missing_tell_player();
            }
            let found = PRE_COMPUTED_TELL_PLAYERS.with(|cell| {
                cell.borrow()
                    .as_ref()
                    .and_then(|players| players.iter().find(|player| player.token == token))
                    .cloned()
            });
            if let Some(player) = found {
                player.to_rhai_map()
            } else {
                body.temp_mut().remove(TELL_TALKER_TOKEN);
                missing_tell_player()
            }
        },
    );

    // 실제 상태 갱신·전달 요청. 발신/수신 문구와 recipient prompt까지
    // Rhai가 완성해서 넘기며 Rust는 문자열을 변경하지 않는다.
    let spec_te = spec.clone();
    engine.register_fn(
        "deliver_tell",
        move |_ob: &mut rhai::Map,
              target_token: &str,
              sender_output: &str,
              recipient_output: &str,
              history_line: &str| {
            if target_token.is_empty() {
                return;
            }
            if let Ok(mut special) = spec_te.lock() {
                *special = Some(CommandResult::Tell {
                    target_token: target_token.to_string(),
                    sender_output: sender_output.to_string(),
                    recipient_output: recipient_output.to_string(),
                    history_line: history_line.to_string(),
                });
            }
        },
    );

    // send_emotion(ob, action): 기능만. to_self/to_room 설정. ""=성공, "usage"=빈 action. 오류 메시지는 Rhai에서.
    let spec_em = spec.clone();
    let body_ptr_em = body_ptr;
    engine.register_fn(
        "send_emotion",
        move |_ob: &mut rhai::Map, action: &str| -> String {
            let body = unsafe { &*body_ptr_em };
            if action.trim().is_empty() {
                return "usage".to_string();
            }
            let name = body.get_name();
            let iga = han_iga(&name);
            let to_self = format!("당신이 {}", action);
            let to_room = format!("{}{} {}", name, iga, action);
            if let Ok(mut s) = spec_em.lock() {
                *s = Some(CommandResult::EmotionToRoom(to_self, to_room, None));
            }
            "".to_string()
        },
    );

    // request_give_silver(ob, target, amt): 기능만. ""=성공, "usage"|"no_money". 오류 메시지는 Rhai에서.
    let spec_gs = spec.clone();
    let body_ptr_gs = body_ptr;
    engine.register_fn(
        "request_give_silver",
        move |_ob: &mut rhai::Map, target: &str, amt: i64| -> String {
            let body = unsafe { &*body_ptr_gs };
            if amt < 1 {
                return "usage".to_string();
            }
            let have = body.get_int("은전");
            let give = amt.min(have.max(0));
            if give < 1 {
                return "no_money".to_string();
            }
            if let Ok(mut s) = spec_gs.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: Some(give),
                    give_gold: None,
                    give_item: None,
                    give_item_stack: None,
                    deduct_from_giver: true,
                    bypass_item_limits: false,
                });
            }
            String::new()
        },
    );

    // request_give_gold(ob, target, amt): 기능만. ""=성공, "usage"|"no_money". 오류 메시지는 Rhai에서.
    let spec_gg = spec.clone();
    let body_ptr_gg = body_ptr;
    engine.register_fn(
        "request_give_gold",
        move |_ob: &mut rhai::Map, target: &str, amt: i64| -> String {
            let body = unsafe { &*body_ptr_gg };
            if amt < 1 {
                return "usage".to_string();
            }
            let have = body.get_int("금전");
            let give = amt.min(have.max(0));
            if give < 1 {
                return "no_money".to_string();
            }
            if let Ok(mut s) = spec_gg.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: None,
                    give_gold: Some(give),
                    give_item: None,
                    give_item_stack: None,
                    deduct_from_giver: true,
                    bypass_item_limits: false,
                });
            }
            String::new()
        },
    );

    // Python 관리자 `줘줘`: 지급자의 잔액을 차감하지 않고 대상에게 화폐를 생성한다.
    let spec_grant_silver = spec.clone();
    let body_ptr_grant_silver = body_ptr;
    engine.register_fn(
        "request_grant_silver",
        move |_ob: &mut rhai::Map, target: &str, amount: i64| -> String {
            if amount < 1 {
                return "usage".into();
            }
            let body = unsafe { &*body_ptr_grant_silver };
            if let Ok(mut result) = spec_grant_silver.lock() {
                *result = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: Some(amount),
                    give_gold: None,
                    give_item: None,
                    give_item_stack: None,
                    deduct_from_giver: false,
                    bypass_item_limits: false,
                });
            }
            String::new()
        },
    );
    let spec_grant_gold = spec.clone();
    let body_ptr_grant_gold = body_ptr;
    engine.register_fn(
        "request_grant_gold",
        move |_ob: &mut rhai::Map, target: &str, amount: i64| -> String {
            if amount < 1 {
                return "usage".into();
            }
            let body = unsafe { &*body_ptr_grant_gold };
            if let Ok(mut result) = spec_grant_gold.lock() {
                *result = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: None,
                    give_gold: Some(amount),
                    give_item: None,
                    give_item_stack: None,
                    deduct_from_giver: false,
                    bypass_item_limits: false,
                });
            }
            String::new()
        },
    );

    // request_give_item(ob, target, name, order, count): 기능만. ""=성공, "no_item". 오류 메시지는 Rhai에서.
    let spec_gi = spec.clone();
    let body_ptr_gi = body_ptr;
    engine.register_fn(
        "request_give_item",
        move |_ob: &mut rhai::Map,
              target: &str,
              item_name: &str,
              order: i64,
              count: i64|
              -> String {
            let body = unsafe { &*body_ptr_gi };
            let order = order.max(1) as usize;
            let cnt = if order > 1 { 1i64 } else { count.clamp(1, 50) };
            // 스택: inv_stack에서
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    if have >= cnt {
                        if let Ok(mut s) = spec_gi.lock() {
                            *s = Some(CommandResult::GiveToPlayer {
                                target_name: target.to_string(),
                                giver_name: body.get_name(),
                                give_silver: None,
                                give_gold: None,
                                give_item: None,
                                give_item_stack: Some((key.clone(), cnt)),
                                deduct_from_giver: true,
                                bypass_item_limits: false,
                            });
                        }
                        return String::new();
                    }
                }
            }
            // 비스택: findObjInven
            let cnt_u = cnt as usize;
            if body.object.findObjInven(item_name, order).is_none() {
                return "no_item".to_string();
            }
            if let Ok(mut s) = spec_gi.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: None,
                    give_gold: None,
                    give_item: Some((item_name.to_string(), order, cnt_u)),
                    give_item_stack: None,
                    deduct_from_giver: true,
                    bypass_item_limits: false,
                });
            }
            String::new()
        },
    );
    let spec_admin_give_item = spec.clone();
    let body_ptr_admin_give_item = body_ptr;
    engine.register_fn(
        "request_admin_give_item",
        move |_ob: &mut rhai::Map,
              target: &str,
              item_name: &str,
              order: i64,
              count: i64|
              -> String {
            let body = unsafe { &*body_ptr_admin_give_item };
            let order = order.max(1) as usize;
            let count = if order > 1 { 1 } else { count.clamp(1, 100) as usize };
            if body.object.findObjInven(item_name, order).is_none() {
                return "no_item".into();
            }
            if let Ok(mut result) = spec_admin_give_item.lock() {
                *result = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: None,
                    give_gold: None,
                    give_item: Some((item_name.to_string(), order, count)),
                    give_item_stack: None,
                    deduct_from_giver: true,
                    bypass_item_limits: true,
                });
            }
            String::new()
        },
    );

    // item_equip(ob, name, order): 기능만. ""=성공, "usage"|"no_item"|"not_equippable"|"slot_used". 오류 메시지는 Rhai에서.
    let body_ptr_give_view = body_ptr;
    engine.register_fn(
        "give_item_presentation",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> rhai::Map {
            let body = unsafe { &*body_ptr_give_view };
            let mut result = rhai::Map::new();
            let mut matched = 0i64;
            for item in &body.object.objs {
                let Ok(item) = item.lock() else { continue };
                let aliases = reaction_names(&item.getString("반응이름"));
                if item.getName() != name && !aliases.iter().any(|alias| alias == name) {
                    continue;
                }
                matched += 1;
                if matched < order.max(1) {
                    continue;
                }
                let actual = item.getName();
                result.insert("found".into(), Dynamic::from(true));
                result.insert("name".into(), Dynamic::from(actual.clone()));
                result.insert("post".into(), Dynamic::from(format!("{}{}", actual, han_eul(&actual))));
                return result;
            }
            result.insert("found".into(), Dynamic::from(false));
            result.insert("name".into(), Dynamic::from(String::new()));
            result.insert("post".into(), Dynamic::from(String::new()));
            result
        },
    );

    // 아이템 착용 시 모든 속성 보너스가 플레이어에게 적용됨
    let body_ptr_equip_view = body_ptr;
    engine.register_fn(
        "item_equip_presentation",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> Dynamic {
            let body = unsafe { &*body_ptr_equip_view };
            let mut result = rhai::Map::new();
            result.insert("name".into(), Dynamic::from(String::new()));
            result.insert("script".into(), Dynamic::from(String::new()));
            result.insert("post".into(), Dynamic::from(String::new()));
            let Some(item) = body.object.findObjInven(name, order.max(1) as usize) else {
                return Dynamic::from(result);
            };
            let Ok(item) = item.lock() else {
                return Dynamic::from(result);
            };
            let item_name = item.getName();
            let name_a = if item.getString("안시").is_empty() {
                format!("\x1b[0;36m{item_name}\x1b[37m")
            } else {
                format!("{}{item_name}\x1b[0;37m", item.getString("안시"))
            };
            result.insert("name".into(), Dynamic::from(item_name.clone()));
            result.insert(
                "post".into(),
                Dynamic::from(format!("{name_a}{}", han_eul(&item_name))),
            );
            result.insert(
                "script".into(),
                Dynamic::from(
                    item.getString("사용스크립")
                        .replace("$아이템$", &item_name),
                ),
            );
            Dynamic::from(result)
        },
    );
    let body_ptr_equip = body_ptr;
    engine.register_fn(
        "item_equip",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> String {
            if name.is_empty() {
                return "usage".to_string();
            }
            let order = order.max(1) as usize;
            let body = unsafe { &mut *body_ptr_equip };
            let arc = match body.object.findObjInven(name, order) {
                Some(a) => a,
                None => return "no_item".to_string(),
            };
            // 아이템의 모든 속성 수집
            let (kind, slot, stats) = {
                let o = arc.lock().unwrap();
                let k = o.getString("종류");
                let s = o.getString("계층");
                if k != "방어구" && k != "무기" {
                    return "not_equippable".to_string();
                }
                let stats = equipment_stats(&o);
                (k, s, stats)
            };
            let slot_used = body.object.objs.iter().any(|obj| {
                if std::sync::Arc::ptr_eq(obj, &arc) {
                    return false;
                }
                obj.lock()
                    .map(|x| x.getBool("inUse") && x.getString("계층") == slot)
                    .unwrap_or(false)
            });
            if slot_used && !slot.is_empty() {
                return "slot_used".to_string();
            }
            {
                let mut o = arc.lock().unwrap();
                o.set("inUse", 1i64);
            }
            // 모든 속성 보너스 적용
            apply_equipment_stats(body, &stats);
            if kind == "무기" {
                body.weapon_item = Some(std::sync::Arc::downgrade(&arc));
            }
            String::new()
        },
    );

    // Python `모두/전부 입어`: visit inventory in insertion order, skip
    // unusable/occupied equipment silently, and return presentation data to
    // Rhai so Rust never owns the user-visible wording.
    let body_ptr_equip_all = body_ptr;
    engine.register_fn(
        "item_equip_all",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &mut *body_ptr_equip_all };
            let inventory = body.object.objs.clone();
            let mut equipped = rhai::Array::new();
            for arc in inventory {
                let (kind, slot, stats, name, script, mastery_required) = {
                    let Ok(item) = arc.lock() else {
                        continue;
                    };
                    let kind = item.getString("종류");
                    if item.getBool("inUse") || (kind != "방어구" && kind != "무기") {
                        continue;
                    }
                    let required = if item.checkAttr("아이템속성", "올숙이천무기") {
                        2000
                    } else if item.checkAttr("아이템속성", "올숙천무기") {
                        1000
                    } else {
                        0
                    };
                    (
                        kind,
                        item.getString("계층"),
                        equipment_stats(&item),
                        item.getName(),
                        item
                            .getString("사용스크립")
                            .replace("$아이템$", &item.getName()),
                        required,
                    )
                };
                if mastery_required > 0
                    && (1..=5).any(|weapon| {
                        body.get_int(&format!("{weapon} 숙련도")) < mastery_required
                    })
                {
                    continue;
                }
                let slot_used = !slot.is_empty()
                    && body.object.objs.iter().any(|other| {
                        !Arc::ptr_eq(other, &arc)
                            && other.lock().is_ok_and(|item| {
                                item.getBool("inUse") && item.getString("계층") == slot
                            })
                    });
                if slot_used {
                    continue;
                }
                if let Ok(mut item) = arc.lock() {
                    item.set("inUse", 1_i64);
                } else {
                    continue;
                }
                apply_equipment_stats(body, &stats);
                if kind == "무기" {
                    body.weapon_item = Some(Arc::downgrade(&arc));
                }
                let mut event = rhai::Map::new();
                event.insert("name".into(), Dynamic::from(name));
                event.insert("script".into(), Dynamic::from(script));
                equipped.push(Dynamic::from(event));
            }
            equipped
        },
    );

    // item_unequip(ob, name, order): 기능만. ""=성공, "usage"|"no_item". 오류 메시지는 Rhai에서.
    // 아이템 해제 시 모든 속성 보너스 제거
    let body_ptr_ue = body_ptr;
    engine.register_fn(
        "item_unequip",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> String {
            if name.is_empty() {
                return "usage".to_string();
            }
            let order = order.max(1) as usize;
            let body = unsafe { &mut *body_ptr_ue };
            let arc = match body.object.findObjInUse(name, order) {
                Some(a) => a,
                None => return "no_item".to_string(),
            };
            // 아이템의 모든 속성 수집 및 해제 처리
            let (is_weapon, stats) = {
                let mut o = arc.lock().unwrap();
                o.set("inUse", 0i64);
                let w = o.getString("종류") == "무기";
                let stats = equipment_stats(&o);
                (w, stats)
            };
            // 모든 속성 보너스 제거 (음수 방지)
            body.attpower = (body.attpower - stats.attack).max(0);
            body.armor = (body.armor - stats.defense).max(0);
            body._str = (body._str - stats.strength).max(0);
            body._dex = (body._dex - stats.dexterity).max(0);
            body._arm = (body._arm - stats.armor).max(0);
            body._maxhp = (body._maxhp - stats.max_hp).max(0);
            body._maxmp = (body._maxmp - stats.max_mp).max(0);
            body._hit = (body._hit - stats.hit).max(0);
            body._miss = (body._miss - stats.miss).max(0);
            body._critical = (body._critical - stats.critical).max(0);
            body._critical_chance = (body._critical_chance - stats.luck).max(0);
            body._exp = (body._exp - stats.exp).max(0);
            body._magic_chance = (body._magic_chance - stats.magic_chance).max(0);
            if is_weapon {
                body.weapon_item = None;
            }
            String::new()
        },
    );

    // item_unequip_all(ob): Python inventory order names for Rhai output,
    // while Body owns only the equipment state rollback.
    let body_ptr_ua = body_ptr;
    engine.register_fn("item_unequip_all", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &mut *body_ptr_ua };
        let items = body
            .object
            .objs
            .iter()
            .filter_map(|item| {
                let item = item.lock().ok()?;
                item.getBool("inUse").then(|| Dynamic::from(item.getName()))
            })
            .collect::<rhai::Array>();
        body.unwear_all();
        items
    });

    // item_use_consumable(ob, name, order): 소비성 아이템 사용.
    // 먼저 inv_stack에서 찾고(개수 관리), 없으면 objs에서 찾음.
    // {err: ""|"usage"|"bad_state"|"no_item"|"not_consumable", name}. 오류 메시지는 Rhai에서.
    let body_ptr_cons = body_ptr;
    engine.register_fn(
        "item_use_consumable",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> Dynamic {
            let mut m = rhai::Map::new();
            if name.is_empty() {
                m.insert("err".into(), Dynamic::from("usage".to_string()));
                m.insert("name".into(), Dynamic::from(String::new()));
                return Dynamic::from(m);
            }
            let body = unsafe { &mut *body_ptr_cons };
            if body.act == crate::player::ActState::Rest {
                m.insert("err".into(), Dynamic::from("bad_state".to_string()));
                m.insert("name".into(), Dynamic::from(String::new()));
                return Dynamic::from(m);
            }

            // 1단계: inv_stack에서 아이템 찾기 (개수로 관리되는 소비성 아이템)
            if let Some(key) = find_item_key_by_name(name) {
                if is_stackable(&key) {
                    let have = *body.object.inv_stack.get(&key).unwrap_or(&0);
                    if have > 0 {
                        // 아이템 정보 가져오기
                        let (item_name, hp, mp) = get_consumable_info(&key);
                        if hp == 0 && mp == 0 {
                            // 소비성 아이템이 아님
                            m.insert("err".into(), Dynamic::from("not_consumable".to_string()));
                            m.insert("name".into(), Dynamic::from(String::new()));
                            return Dynamic::from(m);
                        }

                        // HP/MP 회복 적용
                        let max_hp = body.get_max_hp();
                        let max_mp = body.get_max_mp();
                        let cur_hp = body.get_hp();
                        let cur_mp = body.get_mp();
                        let new_hp = (cur_hp + hp).min(max_hp);
                        let new_mp = (cur_mp + mp).min(max_mp);
                        body.set("체력", new_hp);
                        body.set("내공", new_mp);

                        // 개수 차감
                        if have <= 1 {
                            body.object.inv_stack.remove(&key);
                        } else {
                            *body.object.inv_stack.get_mut(&key).unwrap() -= 1;
                        }

                        // 저장
                        let path = format!("data/user/{}.json", body.get_name());
                        let _ = save_body_to_json(body, &path);

                        m.insert("err".into(), Dynamic::from(String::new()));
                        m.insert("name".into(), Dynamic::from(item_name));
                        m.insert("hp".into(), Dynamic::from(hp));
                        let (script, ansi) = object_from_item_json(&key)
                            .and_then(|(item, _)| {
                                item.lock().ok().map(|item| {
                                    (item.getString("사용스크립"), item.getString("안시"))
                                })
                            })
                            .unwrap_or_default();
                        m.insert("script".into(), Dynamic::from(script));
                        m.insert("ansi".into(), Dynamic::from(ansi));
                        m.insert("max_mp_gain".into(), Dynamic::from(0_i64));
                        m.insert("remaining".into(), Dynamic::from((have - 1).max(0)));
                        return Dynamic::from(m);
                    }
                }
            }

            // 2단계: objs에서 아이템 찾기 (기존 방식 - 개별 인스턴스)
            let order = order.max(1) as usize;
            let arc = match body.object.findObjInven(name, order) {
                Some(a) => a,
                None => {
                    m.insert("err".into(), Dynamic::from("no_item".to_string()));
                    m.insert("name".into(), Dynamic::from(String::new()));
                    return Dynamic::from(m);
                }
            };
            let (item_name, hp, mp, script, ansi, mut max_mp_gain) = {
                let o = arc.lock().unwrap();
                if o.getString("종류") != "먹는것" {
                    m.insert("err".into(), Dynamic::from("not_consumable".to_string()));
                    m.insert("name".into(), Dynamic::from(String::new()));
                    return Dynamic::from(m);
                }
                (
                    o.getName(),
                    o.getInt("체력"),
                    o.getInt("내공"),
                    o.getString("사용스크립"),
                    o.getString("안시"),
                    o.getInt("내공증진"),
                )
            };
            let max_hp = body.get_max_hp();
            let max_mp = body.get_max_mp();
            let cur_hp = body.get_hp();
            let cur_mp = body.get_mp();
            let new_hp = (cur_hp + hp).min(max_hp);
            let new_mp = (cur_mp + mp).min(max_mp);
            body.set("체력", new_hp);
            body.set("내공", new_mp);
            if max_mp_gain != 0 {
                let continuous = arc
                    .lock()
                    .is_ok_and(|item| item.checkAttr("아이템속성", "내공계속증진"));
                if !continuous {
                    if body.object.checkAttr("내공증진아이템리스트", &item_name) {
                        max_mp_gain = 0;
                    } else {
                        body.object.setAttr("내공증진아이템리스트", &item_name);
                        body.set("최고내공", body.get_int("최고내공") + max_mp_gain);
                    }
                } else {
                    body.set("최고내공", body.get_int("최고내공") + max_mp_gain);
                }
            }
            body.object
                .objs
                .retain(|x| !std::sync::Arc::ptr_eq(x, &arc));
            let remaining = body
                .object
                .objs
                .iter()
                .filter(|item| item.lock().is_ok_and(|item| item.getName() == item_name))
                .count() as i64;
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            m.insert("err".into(), Dynamic::from(String::new()));
            m.insert("name".into(), Dynamic::from(item_name));
            m.insert("hp".into(), Dynamic::from(hp));
            m.insert("script".into(), Dynamic::from(script));
            m.insert("ansi".into(), Dynamic::from(ansi));
            m.insert("max_mp_gain".into(), Dynamic::from(max_mp_gain));
            m.insert("remaining".into(), Dynamic::from(remaining));
            Dynamic::from(m)
        },
    );

    // body_save(ob): 캐릭터 저장. data/user/{이름}.json 에 저장.
    engine.register_fn("body_save", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr };
        if let Some((zone, room)) = current_body_position(body) {
            let location = format!("{}:{}", zone, room);
            body.set("위치", location.as_str());
            body.set("현재방", location.as_str());
        }
        let path = format!("data/user/{}.json", body.get_name());
        save_body_to_json(body, &path)
    });

    // add_stack_item(ob, item_key, count) - 스택 아이템을 inv_stack에 추가
    // 성공 시 true 반환, 실패 시 false
    let body_ptr_stack = body_ptr;
    engine.register_fn(
        "add_stack_item",
        move |_ob: &mut rhai::Map, item_key: &str, count: i64| -> bool {
            if item_key.is_empty() || count <= 0 {
                return false;
            }
            let body = unsafe { &mut *body_ptr_stack };

            // 스택 가능한 아이템인지 확인
            if !is_stackable(item_key) {
                return false;
            }

            // inv_stack에 추가
            *body
                .object
                .inv_stack
                .entry(item_key.to_string())
                .or_insert(0) += count;

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            save_body_to_json(body, &path)
        },
    );

    // get_stack_count(ob, item_key) - inv_stack에서 아이템 개수 조회
    let body_ptr_gs = body_ptr;
    engine.register_fn(
        "get_stack_count",
        move |_ob: &mut rhai::Map, item_key: &str| -> i64 {
            let body = unsafe { &*body_ptr_gs };
            *body.object.inv_stack.get(item_key).unwrap_or(&0)
        },
    );

    // remove_stack_item(ob, item_key, count) - inv_stack에서 아이템 제거
    // 성공 시 true, 실패(부족) 시 false
    let body_ptr_rs = body_ptr;
    engine.register_fn(
        "remove_stack_item",
        move |_ob: &mut rhai::Map, item_key: &str, count: i64| -> bool {
            if item_key.is_empty() || count <= 0 {
                return false;
            }
            let body = unsafe { &mut *body_ptr_rs };

            let have = *body.object.inv_stack.get(item_key).unwrap_or(&0);
            if have < count {
                return false;
            }

            if have == count {
                body.object.inv_stack.remove(item_key);
            } else {
                *body.object.inv_stack.get_mut(item_key).unwrap() -= count;
            }

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            save_body_to_json(body, &path)
        },
    );

    // ONEITEM (단일아이템/기연) 시스템. Python ONEITEM과 동일.
    engine.register_fn("oneitem_get_name", crate::oneitem::oneitem_get_name);
    engine.register_fn("oneitem_get", crate::oneitem::oneitem_get);
    engine.register_fn("oneitem_have", crate::oneitem::oneitem_have);
    engine.register_fn("oneitem_drop", crate::oneitem::oneitem_drop);
    engine.register_fn("oneitem_drop2", crate::oneitem::oneitem_drop2);
    engine.register_fn("oneitem_keep", crate::oneitem::oneitem_keep);
    engine.register_fn("oneitem_destroy", crate::oneitem::oneitem_destroy);
    engine.register_fn(
        "cleanup_offline_oneitem",
        move |_ob: &mut rhai::Map, owner: &str, index: &str| -> String {
            let mut player = Body::new();
            let path = format!("data/user/{}.json", owner);
            if !load_body_from_json(&mut player, &path) {
                return "존재하지않는 사용자입니다.".into();
            }
            let last = player.get_int("마지막저장시간");
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if last > 0 && now.saturating_sub(last) < 259_200 {
                return "아직 3일이 경과하지 않았습니다.".into();
            }
            let before = player.object.objs.len();
            if let Some(position) = player.object.objs.iter().position(|arc| {
                arc.lock()
                    .map(|obj| obj.getString("인덱스") == index)
                    .unwrap_or(false)
            }) {
                player.object.objs.remove(position);
            }
            if player.object.objs.len() == before {
                return "not_found".into();
            }
            if !save_body_to_json_without_timestamp(&mut player, &path) {
                return "저장할 수 없습니다.".into();
            }
            "ok".into()
        },
    );
    engine.register_fn("oneitem_check_name", crate::oneitem::oneitem_check_name);
    engine.register_fn("oneitem_check_index", crate::oneitem::oneitem_check_index);
    engine.register_fn("oneitem_list", crate::oneitem::oneitem_list);
    engine.register_fn("oneitem_clear", crate::oneitem::oneitem_clear);
    engine.register_fn("oneitem_attr_keys", crate::oneitem::oneitem_attr_keys);
    engine.register_fn(
        "oneitem_get_index_by_name",
        crate::oneitem::oneitem_get_index_by_name,
    );
    engine.register_fn(
        "oneitem_list_index_entries",
        crate::oneitem::oneitem_list_index_entries,
    );

    // call_out / call_later / remove_call_out — 점프 2초 후 착지 등. script_name이 있을 때만 등록(지연 시 스크립트 함수 실행).
    if let (Some(sched), Some(sn)) = (call_out_scheduler, script_name) {
        let s = sched.clone();
        let script_owned = sn.to_string();
        engine.register_fn(
            "call_out",
            move |target: &str, function: &str, delay: i64| {
                let d = Duration::from_secs(delay.max(0) as u64);
                s.call_out(target, function, d, vec![], Some(script_owned.clone()));
            },
        );
        let s2 = sched.clone();
        let script_owned2 = sn.to_string();
        engine.register_fn(
            "call_later",
            move |target: &str, function: &str, delay: i64| {
                let d = Duration::from_secs(delay.max(0) as u64);
                s2.call_out(target, function, d, vec![], Some(script_owned2.clone()));
            },
        );
        let s3 = sched.clone();
        engine.register_fn(
            "remove_call_out",
            move |target: &str, function: &str| -> bool {
                s3.remove_call_out_by_name(target, function)
            },
        );
    }

    // ============================================================
    // TALK HISTORY FUNCTIONS (대화 기록)
    // ============================================================

    // get_talk_history(ob) -> 배열
    // NPC와의 대화 기록을 가져옵니다.
    engine.register_fn(
        "get_talk_history",
        move |_obj: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &mut *body_ptr };
            let arr: rhai::Array = body
                .talk_history
                .iter()
                .map(|s| rhai::Dynamic::from(s.as_str()))
                .collect();
            arr
        },
    );

    // add_talk_history(ob, key) - 대화 기록 추가
    engine.register_fn(
        "add_talk_history",
        move |_obj: &mut rhai::Map, key: &str| {
            let body = unsafe { &mut *body_ptr };
            if !body.talk_history.contains(&key.to_string()) {
                body.talk_history.push(key.to_string());
            }
        },
    );

    // clear_talk_history(ob) - 대화 기록 초기화
    engine.register_fn("clear_talk_history", move |_obj: &mut rhai::Map| {
        let body = unsafe { &mut *body_ptr };
        body.talk_history.clear();
    });

    engine.register_fn(
        "get_chat_history",
        move |_obj: &mut rhai::Map| -> rhai::Array {
            chat_history_snapshot()
                .into_iter()
                .map(rhai::Dynamic::from)
                .collect()
        },
    );
    engine.register_fn(
        "add_chat_history",
        move |_obj: &mut rhai::Map, message: &str| {
            if !message.is_empty() {
                record_chat_history(message);
            }
        },
    );
    engine.register_fn(
        "add_chat_history_limit",
        move |_obj: &mut rhai::Map, message: &str, limit: i64| {
            if !message.is_empty() {
                record_chat_history_limit(message, limit.clamp(1, 100) as usize);
            }
        },
    );
    engine.register_fn("local_time_hm", || -> String {
        chrono::Local::now().format("[%H:%M] ").to_string()
    });
    let body_ptr_self_desc = body_ptr;
    engine.register_fn("get_self_desc", move |_obj: &mut rhai::Map| -> String {
        unsafe { &*body_ptr_self_desc }.get_desc_for_look(true)
    });
    let body_ptr_status = body_ptr;
    engine.register_fn(
        "get_hp_status_script",
        move |_obj: &mut rhai::Map| -> String {
            let body = unsafe { &*body_ptr_status };
            // Python lib.script.get_hp_script() uses the persisted
            // 최고체력 attribute, not the derived armor-adjusted maximum.
            hp_status_script(body.get_hp(), body.get_int("최고체력"))
        },
    );
    let body_ptr_mp_status = body_ptr;
    engine.register_fn(
        "get_mp_status_script",
        move |_obj: &mut rhai::Map| -> String {
            let body = unsafe { &*body_ptr_mp_status };
            mp_status_script(body.get_mp())
        },
    );
    let body_ptr_change = body_ptr;
    engine.register_fn(
        "request_change_player",
        move |_obj: &mut rhai::Map, target: &str| {
            if !target.trim().is_empty() {
                unsafe { &mut *body_ptr_change }.temp_mut().insert(
                    CHANGE_PLAYER_REQUEST.to_string(),
                    Value::String(target.trim().to_string()),
                );
            }
        },
    );

    // ============================================================
    // 몹/오브젝트 관련 efun (스크립트용)
    // ============================================================

    // find_mob_in_room(ob, mob_name) - 현재 방에서 몹 찾기
    // 몹이 있으면 몹 데이터를 반환, 없으면 UNIT
    let body_ptr_mob = body_ptr;
    engine.register_fn(
        "find_mob_in_room",
        move |ob: &mut rhai::Map, mob_name: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_mob };

            // 플레이어 이름과 위치 가져오기
            let player_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();

            if player_name.is_empty() {
                return Dynamic::UNIT;
            }

            let Some((zone, room)) = current_body_position(body) else {
                return Dynamic::UNIT;
            };

            // WorldState에서 현재 방의 몹 검색
            if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);

                // mob_name으로 검색 (이름 또는 반응 이름 일치)
                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    // Python Room.findObjName: 정확한 이름/반응이름 또는
                    // 반응이름의 접두어만 허용한다. 설명1은 대상 키가 아니다.
                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let matched = mob_data.name == mob_name
                            || mob_data
                                .reaction_names
                                .iter()
                                .any(|reaction| reaction == mob_name || reaction.starts_with(mob_name));
                        if matched {
                            let mut mob_info = rhai::Map::new();
                            mob_info.insert("이름".into(), Dynamic::from(mob_data.name.clone()));
                            mob_info.insert("표시".into(), Dynamic::from(display_name.clone()));
                            mob_info.insert("hp".into(), Dynamic::from(mob.hp));
                            mob_info.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                            mob_info.insert("level".into(), Dynamic::from(mob_data.level));
                            mob_info.insert("zone".into(), Dynamic::from(mob.zone.clone()));
                            mob_info.insert("room".into(), Dynamic::from(mob.room.clone()));
                            mob_info.insert("mob_key".into(), Dynamic::from(mob.mob_key.clone()));
                            return Dynamic::from(mob_info);
                        }
                    }
                }
            }

            Dynamic::UNIT
        },
    );

    let body_ptr_room_mob = body_ptr;
    engine.register_fn(
        "room_has_mob",
        move |_ob: &mut rhai::Map, mob_name: &str| -> bool {
            room_has_mob_named(unsafe { &*body_ptr_room_mob }, mob_name)
        },
    );

    // get_mob_by_name(ob, mob_name) - 데이터베이스에서 몹 정보 조회
    // 몹 데이터베이스(Mobs)에서 몹 정보를 가져옴
    let body_ptr_get_mob = body_ptr;
    engine.register_fn(
        "get_mob_by_name",
        move |_ob: &mut rhai::Map, mob_name: &str| -> Dynamic {
            let _body = unsafe { &*body_ptr_get_mob };
            // 기존 get_mob_data 함수와 동일하게 동작
            let full_path = format!("data/mob/{}.json", mob_name);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        if let Some(obj) = value.get("몹정보") {
                            json_value_to_dynamic(obj.clone())
                        } else {
                            Dynamic::UNIT
                        }
                    }
                    Err(_) => Dynamic::UNIT,
                },
                Err(_) => Dynamic::UNIT,
            }
        },
    );

    // kill_mob(ob, mob_name) - 몹 처치
    let body_ptr_kill = body_ptr;
    engine.register_fn(
        "kill_mob",
        move |ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &*body_ptr_kill };

            // 플레이어 이름과 위치 가져오기
            let player_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();

            if player_name.is_empty() {
                return false;
            }

            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };

            // WorldState에서 현재 방의 몹 검색 후 처치
            // 먼저 읽기 lock으로 몹 찾기
            let mob_key_to_kill = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();
                let mut found_key = None;

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    // 몹 데이터로 표시 이름 확인
                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let display_name_lower = display_name.to_lowercase();

                        // 정확히 일치하거나 포함
                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_key = Some(mob.mob_key.clone());
                            break;
                        }

                        // 반응 이름들도 확인
                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_key = Some(mob.mob_key.clone());
                                break;
                            }
                        }
                    }
                }
                found_key
            } else {
                None
            };

            // 찾은 몹 처치 (쓰기 lock)
            if let Some(mob_key) = mob_key_to_kill {
                if let Ok(mut world) = get_world_state().write() {
                    world.kill_mob(&zone, &room, &mob_key);
                    return true;
                }
            }

            false
        },
    );

    // create_mob(ob, mob_name, zone, room) - 새 몹 생성
    let body_ptr_create = body_ptr;
    engine.register_fn(
        "create_mob",
        move |_ob: &mut rhai::Map, mob_name: &str, zone: &str, room: &str| -> String {
            let _body = unsafe { &*body_ptr_create };

            // 몹 데이터 로드 - WorldState를 통해 로드
            let mob_data = if let Ok(mut world) = get_world_state().write() {
                match world.mob_cache.load_mob(zone, mob_name) {
                    Ok(data) => data,
                    Err(_) => {
                        // zone 폴더에 없으면 시도
                        match world.mob_cache.load_mob(zone, mob_name) {
                            Ok(data) => data,
                            Err(_) => return format!("몹 데이터를 찾을 수 없습니다: {}", mob_name),
                        }
                    }
                }
            } else {
                return "월드 상태 접근 실패".to_string();
            };

            // 몹 생성
            if let Ok(mut world) = get_world_state().write() {
                // Use with_difficulty constructor for proper stat initialization
                let mob_instance = MobInstance::with_difficulty(
                    format!("{}:{}", zone, mob_name),
                    zone.to_string(),
                    room.to_string(),
                    &mob_data,
                    0, // difficulty 0 for spawned mobs
                );

                world.mob_cache.add_mob_instance(mob_instance);
                String::new() // 성공 시 빈 문자열 반환
            } else {
                "월드 상태 접근 실패".to_string()
            }
        },
    );

    // mob_say(mob_name, message) - 몹이 말하기
    let body_ptr_say = body_ptr;
    engine.register_fn(
        "mob_say",
        move |_ob: &mut rhai::Map, mob_name: &str, message: &str| -> bool {
            let body = unsafe { &*body_ptr_say };

            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };

            // WorldState에서 몹 찾기 (display_name을 소유하여 반환)
            let found_display_name = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();

                let mut found_name = None;
                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name_lower = mob_data.desc1.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_name = Some(mob_data.desc1.clone());
                            break;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_name = Some(mob_data.desc1.clone());
                                break;
                            }
                        }
                    }
                }
                found_name
            } else {
                None
            };

            if let Some(display_name) = found_display_name {
                // 메시지 전송 - 브로드캐스터에 메시지 보내기
                // 현재는 로그로 출력 (실제로는 broadcaster를 통해 방에 있는 모든 플레이어에게 전송)
                println!("[MOB_SAY] {}: {}", display_name, message);
                true
            } else {
                false
            }
        },
    );

    // mob_follow(mob_name, target_name) - 몹이 대상 따라가기
    let body_ptr_follow = body_ptr;
    engine.register_fn(
        "mob_follow",
        move |_ob: &mut rhai::Map, mob_name: &str, target_name: &str| -> bool {
            let body = unsafe { &*body_ptr_follow };

            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };

            // 먼저 읽기 lock으로 몹 찾기
            let mob_key_to_follow = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();
                let mut found_key = None;
                let mut found_name = None;

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let display_name_lower = display_name.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_key = Some(mob.mob_key.clone());
                            found_name = Some(display_name.clone());
                            break;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_key = Some(mob.mob_key.clone());
                                found_name = Some(display_name.clone());
                                break;
                            }
                        }
                    }
                }
                (found_key, found_name)
            } else {
                (None, None)
            };

            // 찾은 몹의 타겟 설정
            if let (Some(mob_key), Some(display_name)) = mob_key_to_follow {
                if let Ok(mut world) = get_world_state().write() {
                    if let Some(mob_instance) =
                        world.mob_cache.get_mob_instance_mut(&zone, &room, &mob_key)
                    {
                        if !mob_instance.targets.contains(&target_name.to_string()) {
                            mob_instance.targets.push(target_name.to_string());
                        }
                    }
                    println!(
                        "[MOB_FOLLOW] {} now following {}",
                        display_name, target_name
                    );
                    return true;
                }
            }

            false
        },
    );

    // get_mob_hp(ob, mob_name) - 몹의 현재 HP 조회
    let body_ptr_get_hp = body_ptr;
    engine.register_fn(
        "get_mob_hp",
        move |_ob: &mut rhai::Map, mob_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_hp };

            let Some((zone, room)) = current_body_position(body) else {
                return 0;
            };

            // WorldState에서 몹 찾기
            if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name = &mob_data.desc1;
                        let display_name_lower = display_name.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            return mob.hp;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                return mob.hp;
                            }
                        }
                    }
                }
            }

            0
        },
    );

    // set_mob_hp(ob, mob_name, hp) - 몹의 HP 설정
    let body_ptr_set_hp = body_ptr;
    engine.register_fn(
        "set_mob_hp",
        move |_ob: &mut rhai::Map, mob_name: &str, hp: i64| -> bool {
            let body = unsafe { &*body_ptr_set_hp };

            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };

            // 먼저 읽기 lock으로 몹 찾기
            let mob_key_to_set = if let Ok(world) = get_world_state().read() {
                let mobs = world.get_mobs_in_room(&zone, &room);
                let mob_name_lower = mob_name.to_lowercase();
                let mut found_key = None;

                for mob in mobs {
                    if !mob.alive {
                        continue;
                    }

                    if let Some(mob_data) = world.get_mob_data(&mob.mob_key) {
                        let display_name_lower = mob_data.desc1.to_lowercase();

                        if display_name_lower == mob_name_lower
                            || display_name_lower.contains(&mob_name_lower)
                        {
                            found_key = Some(mob.mob_key.clone());
                            break;
                        }

                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase() == mob_name_lower {
                                found_key = Some(mob.mob_key.clone());
                                break;
                            }
                        }
                    }
                }
                found_key
            } else {
                None
            };

            // 찾은 몹의 HP 설정
            if let Some(mob_key) = mob_key_to_set {
                if let Ok(mut world) = get_world_state().write() {
                    if let Some(mob_instance) =
                        world.mob_cache.get_mob_instance_mut(&zone, &room, &mob_key)
                    {
                        mob_instance.hp = hp.max(0).min(mob_instance.max_hp);
                        if mob_instance.hp <= 0 {
                            world.kill_mob(&zone, &room, &mob_key);
                        }
                        return true;
                    }
                }
            }

            false
        },
    );

    // delete_mob_definition(index): Mob.Mobs에서만 제거하고 원본 JSON은 보존한다.
    engine.register_fn("spawn_mob_definition", move |index: &str| -> rhai::Map {
        let mut result = rhai::Map::new();
        result.insert("ok".into(), Dynamic::from(false));
        result.insert("이름".into(), Dynamic::from(""));
        let Ok(mut world) = get_world_state().write() else {
            return result;
        };
        let Some(data) = world.mob_cache.get_mob(index).cloned() else {
            return result;
        };
        let mut placed = 0_i64;
        for location in &data.locations {
            let room = location.to_string();
            if world.room_cache.get_room(&data.zone, &room).is_err() {
                continue;
            }
            world.mob_cache.add_mob_instance(crate::world::MobInstance::new(
                index.to_string(),
                data.zone.clone(),
                room,
                &data,
            ));
            placed += 1;
        }
        // Python getMob/clone/place succeeds even when every configured room is invalid.
        let _ = placed;
        result.insert("ok".into(), Dynamic::from(true));
        result.insert("이름".into(), Dynamic::from(data.name));
        result
    });

    engine.register_fn("delete_mob_definition", move |index: &str| -> bool {
        if !index.contains(':') {
            return false;
        }
        get_world_state()
            .write()
            .ok()
            .map(|mut world| world.mob_cache.remove_mob_definition(index))
            .unwrap_or(false)
    });

    engine.register_fn("delete_item_definition", move |index: &str| -> bool {
        if index.is_empty() || index.contains('/') || index.contains('\\') {
            return false;
        }
        get_world_state()
            .write()
            .ok()
            .map(|mut world| world.item_cache.remove_item(index))
            .unwrap_or(false)
    });

    engine.register_fn("delete_room_definition", move |index: &str| -> bool {
        let Some((zone, room)) = index.split_once(':') else {
            return false;
        };
        if zone.is_empty() || room.is_empty() {
            return false;
        }
        get_world_state()
            .write()
            .ok()
            .map(|mut world| world.room_cache.remove_room(zone, room))
            .unwrap_or(false)
    });

    engine.register_fn(
        "get_mob_item_holders",
        move |item_key: &str| -> rhai::Array {
            get_world_state()
                .read()
                .ok()
                .map(|world| {
                    world
                        .mob_cache
                        .item_holders(item_key)
                        .into_iter()
                        .map(|(name, index)| {
                            let mut map = rhai::Map::new();
                            map.insert("name".into(), Dynamic::from(name));
                            map.insert("index".into(), Dynamic::from(index));
                            Dynamic::from(map)
                        })
                        .collect()
                })
                .unwrap_or_default()
        },
    );

    // ============================================================
    // Room/Zone 관련 efun
    // ============================================================

    // get_room(ob, zone:room_id) - 특정 zone:room의 방 데이터 조회
    let body_ptr_get_room = body_ptr;
    engine.register_fn(
        "get_room",
        move |_ob: &mut rhai::Map, zone: &str, room_id: &str| -> Dynamic {
            let _body = unsafe { &*body_ptr_get_room };
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return Dynamic::UNIT,
            };
            let _room_key = format!("{}:{}", zone, room_id);
            if let Some(arc) = w.room_cache.get_room_cached(zone, room_id) {
                if let Ok(room_ref) = arc.read() {
                    let mut m = rhai::Map::new();
                    m.insert("zone".into(), Dynamic::from(room_ref.zone.clone()));
                    m.insert("room".into(), Dynamic::from(room_ref.name.clone()));
                    m.insert("name".into(), Dynamic::from(room_ref.display_name.clone()));
                    m.insert(
                        "desc".into(),
                        Dynamic::from(room_ref.description.join("\n")),
                    );
                    // 출구 배열: [{direction, display_name, destination_zone, destination_room}, ...]
                    let mut exits_arr = rhai::Array::new();
                    for (display_name, exit) in &room_ref.exits {
                        let mut exit_map = rhai::Map::new();
                        exit_map.insert("display_name".into(), Dynamic::from(display_name.clone()));
                        if let Some(dir) = &exit.direction {
                            exit_map.insert("direction".into(), Dynamic::from(dir.korean_name()));
                        } else {
                            exit_map.insert("direction".into(), Dynamic::from(""));
                        }
                        if let Some((dest_zone, dest_room)) = &exit.destination {
                            exit_map.insert(
                                "destination_zone".into(),
                                Dynamic::from(dest_zone.clone()),
                            );
                            exit_map.insert(
                                "destination_room".into(),
                                Dynamic::from(dest_room.clone()),
                            );
                        }
                        exit_map.insert("hidden".into(), Dynamic::from(exit.hidden));
                        exits_arr.push(Dynamic::from(exit_map));
                    }
                    m.insert("exits".into(), Dynamic::from(exits_arr));
                    // 맵속성 배열
                    let mut props_arr = rhai::Array::new();
                    for prop in &room_ref.properties {
                        props_arr.push(Dynamic::from(prop.clone()));
                    }
                    m.insert("properties".into(), Dynamic::from(props_arr));
                    return Dynamic::from(m);
                }
            }
            Dynamic::UNIT
        },
    );

    // get_current_room(ob) - 현재 플레이어의 방 데이터 조회
    let body_ptr_cur_room = body_ptr;
    engine.register_fn("get_current_room", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &*body_ptr_cur_room };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return Dynamic::UNIT,
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return Dynamic::UNIT,
        };
        if let Some(arc) = w.room_cache.get_room_cached(&pos.zone, &pos.room) {
            if let Ok(room_ref) = arc.read() {
                let mut m = rhai::Map::new();
                m.insert("zone".into(), Dynamic::from(room_ref.zone.clone()));
                m.insert("room".into(), Dynamic::from(room_ref.name.clone()));
                m.insert("name".into(), Dynamic::from(room_ref.display_name.clone()));
                m.insert(
                    "desc".into(),
                    Dynamic::from(room_ref.description.join("\n")),
                );
                // 출구 배열
                let mut exits_arr = rhai::Array::new();
                for (display_name, exit) in &room_ref.exits {
                    let mut exit_map = rhai::Map::new();
                    exit_map.insert("display_name".into(), Dynamic::from(display_name.clone()));
                    if let Some(dir) = &exit.direction {
                        exit_map.insert("direction".into(), Dynamic::from(dir.korean_name()));
                    } else {
                        exit_map.insert("direction".into(), Dynamic::from(""));
                    }
                    if let Some((dest_zone, dest_room)) = &exit.destination {
                        exit_map
                            .insert("destination_zone".into(), Dynamic::from(dest_zone.clone()));
                        exit_map
                            .insert("destination_room".into(), Dynamic::from(dest_room.clone()));
                    }
                    exit_map.insert("hidden".into(), Dynamic::from(exit.hidden));
                    exits_arr.push(Dynamic::from(exit_map));
                }
                m.insert("exits".into(), Dynamic::from(exits_arr));
                return Dynamic::from(m);
            }
        }
        Dynamic::UNIT
    });

    // find_obj_in_room(ob, obj_name) - 현재 방에서 아이템으로 이름 찾기
    let body_ptr_find_obj = body_ptr;
    engine.register_fn(
        "find_obj_in_room",
        move |_ob: &mut rhai::Map, obj_name: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_find_obj };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return Dynamic::UNIT,
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return Dynamic::UNIT,
            };
            // 바닥 아이템 검색
            let room_objs = w.get_room_objs(&pos.zone, &pos.room);
            for arc in room_objs {
                if let Ok(o) = arc.lock() {
                    let item_name = o.getName();
                    // 정확히 일치하거나 접두사 일치
                    if item_name == obj_name || item_name.starts_with(obj_name) {
                        let mut m = rhai::Map::new();
                        m.insert("name".into(), Dynamic::from(item_name));
                        m.insert("name_a".into(), Dynamic::from(o.getNameA()));
                        m.insert("desc1".into(), Dynamic::from(o.getString("설명1")));
                        m.insert("count".into(), Dynamic::from(1i64));
                        return Dynamic::from(m);
                    }
                }
            }
            // 쌓을 수 있는 아이템 검색
            let room_stack = w.get_room_objs_stack(&pos.zone, &pos.room);
            for (key, count) in room_stack {
                if count > 0 {
                    if let Some((item_name, _, _, _)) = get_item_info(&key) {
                        let obj_name_str = obj_name.to_string();
                        if item_name == obj_name_str || item_name.starts_with(&obj_name_str) {
                            let mut m = rhai::Map::new();
                            m.insert("name".into(), Dynamic::from(item_name.clone()));
                            m.insert("desc1".into(), Dynamic::from(get_item_desc1(&key)));
                            m.insert("count".into(), Dynamic::from(count));
                            m.insert("key".into(), Dynamic::from(key));
                            return Dynamic::from(m);
                        }
                    }
                }
            }
            Dynamic::UNIT
        },
    );

    // get_room_exits(ob) - 현재 방의 출구 방향 배열
    let body_ptr_exits = body_ptr;
    engine.register_fn(
        "get_room_exits",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_exits };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let mut exits = rhai::Array::new();
            if let Some(arc) = w.room_cache.get_room_cached(&pos.zone, &pos.room) {
                if let Ok(room_ref) = arc.read() {
                    // 방향이 있는 출구만 (방향 이동용)
                    for exit in room_ref.exits.values() {
                        if let Some(dir) = &exit.direction {
                            exits.push(Dynamic::from(dir.korean_name()));
                        }
                    }
                }
            }
            exits
        },
    );

    // get_room_players(ob) - 현재 방의 플레이어 목록 (실제 구현)
    let body_ptr_players = body_ptr;
    engine.register_fn(
        "get_room_players",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_players };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let players = w.get_players_in_room(&pos.zone, &pos.room);
            let mut arr = rhai::Array::new();
            for player_name in players {
                arr.push(Dynamic::from(player_name));
            }
            arr
        },
    );
    let body_ptr_give_player = body_ptr;
    engine.register_fn(
        "find_give_player",
        move |_ob: &mut rhai::Map, input: &str| -> String {
            let body = unsafe { &*body_ptr_give_player };
            let Some((zone, room)) = current_body_position(body) else {
                return String::new();
            };
            let world = get_world_state().read().unwrap();
            world
                .get_players_in_room(&zone, &room)
                .iter()
                .rev()
                .find(|name| name.as_str() == input || name.starts_with(input))
                .cloned()
                .unwrap_or_default()
        },
    );

    // get_room_mobs(ob) - 현재 방의 몹 목록 (실제 구현)
    let body_ptr_room_mobs = body_ptr;
    engine.register_fn("get_room_mobs", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_room_mobs };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Array::new(),
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return rhai::Array::new(),
        };
        let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
        let mut arr = rhai::Array::new();
        for mob in mobs {
            if let Some(mob_data) = w.mob_cache.get_mob(&mob.mob_key) {
                let mut m = rhai::Map::new();
                m.insert("name".into(), Dynamic::from(mob_data.name.clone()));
                m.insert("desc1".into(), Dynamic::from(mob_data.desc1.clone()));
                m.insert(
                    "reaction_names".into(),
                    Dynamic::from(
                        mob_data
                            .reaction_names
                            .iter()
                            .cloned()
                            .map(Dynamic::from)
                            .collect::<rhai::Array>(),
                    ),
                );
                m.insert("alive".into(), Dynamic::from(mob.alive));
                m.insert("hp".into(), Dynamic::from(mob.hp));
                m.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                m.insert("mob_key".into(), Dynamic::from(mob.mob_key.clone()));
                let inventory = mob
                    .inventory
                    .iter()
                    .filter_map(|item| item.lock().ok().map(|obj| Dynamic::from(obj.getName())))
                    .collect::<rhai::Array>();
                m.insert("inventory".into(), Dynamic::from(inventory));
                let mut events = rhai::Map::new();
                for (key, script) in &mob_data.events {
                    let text = match script {
                        crate::world::mob::EventScript::Legacy(lines) => lines.join("\r\n"),
                        crate::world::mob::EventScript::Rhai(name) => name.clone(),
                    };
                    events.insert(key.clone().into(), Dynamic::from(text));
                }
                m.insert("events".into(), Dynamic::from(events));
                arr.push(Dynamic::from(m));
            }
        }
        arr
    });

    // Python `조회.py`/`입금.py`: 표국무사 존재 확인과 보험료 상태 전이만
    // 제공한다. 금액 계산과 사용자에게 보이는 문구는 Rhai 명령이 담당한다.
    let body_ptr_insurance = body_ptr;
    engine.register_fn(
        "get_insurance_view",
        move |_ob: &mut rhai::Map| -> Dynamic {
            let body = unsafe { &*body_ptr_insurance };
            let mut result = rhai::Map::new();
            let has_agent = room_has_insurance_agent(body);
            let level = body.get_int("레벨");
            let unit = get_murim_config_int("보험료단가");
            let premium = body.get_int("보험료");
            let divisor = level.saturating_mul(unit);
            result.insert("has_agent".into(), Dynamic::from(has_agent));
            result.insert("premium".into(), Dynamic::from(premium));
            result.insert(
                "count".into(),
                Dynamic::from(if divisor > 0 { premium / divisor } else { 0 }),
            );
            result.insert("threshold".into(), Dynamic::from(divisor));
            result.insert(
                "trip_cost".into(),
                Dynamic::from(divisor * get_murim_config_int("보험출장률") / 100),
            );
            Dynamic::from(result)
        },
    );

    let body_ptr_deposit = body_ptr;
    engine.register_fn(
        "deposit_insurance",
        move |_ob: &mut rhai::Map, amount: i64| -> Dynamic {
            let body = unsafe { &mut *body_ptr_deposit };
            let mut result = rhai::Map::new();
            let has_agent = room_has_insurance_agent(body);
            result.insert("has_agent".into(), Dynamic::from(has_agent));
            if !has_agent {
                result.insert("status".into(), Dynamic::from("no_agent"));
                return Dynamic::from(result);
            }
            if amount <= 0 {
                result.insert("status".into(), Dynamic::from("invalid_amount"));
                return Dynamic::from(result);
            }
            let paid = amount.min(body.get_int("은전").max(0));
            body.set("은전", body.get_int("은전") - paid);
            body.set("보험료", body.get_int("보험료") + paid);
            result.insert("status".into(), Dynamic::from("ok"));
            result.insert("paid".into(), Dynamic::from(paid));
            result.insert("premium".into(), Dynamic::from(body.get_int("보험료")));
            let divisor = body
                .get_int("레벨")
                .saturating_mul(get_murim_config_int("보험료단가"));
            result.insert(
                "count".into(),
                Dynamic::from(if divisor > 0 {
                    body.get_int("보험료") / divisor
                } else {
                    0
                }),
            );
            Dynamic::from(result)
        },
    );

    // get_room_mobs_admin(ob) - 관리자용 몹 상세 정보 (infoMob 대응)
    // 레벨, 체력, 내공, 힘, 민첩, 맷집, 타겟 등 상세 정보 반환
    let body_ptr_room_mobs_admin = body_ptr;
    engine.register_fn(
        "get_room_mobs_admin",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_room_mobs_admin };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(name.as_str()) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut arr = rhai::Array::new();
            for mob in mobs {
                if let Some(mob_data) = w.mob_cache.get_mob(&mob.mob_key) {
                    let attr_int = |key: &str| {
                        mob_data
                            .attributes
                            .get(key)
                            .and_then(|value| value.as_i64())
                            .unwrap_or(0)
                    };
                    let attr_string = |key: &str| {
                        mob_data
                            .attributes
                            .get(key)
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string()
                    };
                    let (attack, armor, weight) = mob_data.use_items.iter().fold(
                        (0_i64, 0_i64, 0_i64),
                        |(attack, armor, weight), (key, count, _, _)| {
                            let Some((item, _)) = object_from_item_json(key) else {
                                return (attack, armor, weight);
                            };
                            let Ok(item) = item.lock() else {
                                return (attack, armor, weight);
                            };
                            (
                                attack + item.getInt("공격력") * count,
                                armor + item.getInt("방어력") * count,
                                weight + item.getInt("무게") * count,
                            )
                        },
                    );
                    let mut m = rhai::Map::new();
                    m.insert("name".into(), Dynamic::from(mob_data.name.clone()));
                    m.insert("index".into(), Dynamic::from(mob.mob_key.clone()));
                    m.insert("instance_id".into(), Dynamic::from(mob.instance_id as i64));
                    m.insert("level".into(), Dynamic::from(mob.level));
                    m.insert("age".into(), Dynamic::from(attr_int("나이")));
                    m.insert("hp".into(), Dynamic::from(mob.hp));
                    m.insert("max_hp".into(), Dynamic::from(mob.max_hp));
                    m.insert("mp".into(), Dynamic::from(mob.mp));
                    m.insert("max_mp".into(), Dynamic::from(mob.max_mp));
                    m.insert("attack".into(), Dynamic::from(attack));
                    m.insert(
                        "strength".into(),
                        Dynamic::from((mob.strength + mob.str_modifier).max(0)),
                    );
                    m.insert("armor".into(), Dynamic::from(armor));
                    m.insert(
                        "arm".into(),
                        Dynamic::from((mob.arm + mob.arm_modifier).max(0)),
                    );
                    m.insert(
                        "dex".into(),
                        Dynamic::from((mob.agility + mob.dex_modifier).max(0)),
                    );
                    m.insert("weight".into(), Dynamic::from(weight));
                    m.insert("current_exp".into(), Dynamic::from(attr_int("현재경험치")));
                    let total_exp = ((((mob.level * mob.level) / 3) + 30) * (mob.level + 4))
                        .clamp(1, 999_999_999);
                    m.insert("total_exp".into(), Dynamic::from(total_exp));
                    m.insert("hit".into(), Dynamic::from(mob_data.hit));
                    m.insert("miss".into(), Dynamic::from(mob_data.miss));
                    m.insert("critical".into(), Dynamic::from(mob_data.critical));
                    m.insert("luck".into(), Dynamic::from(mob_data.luck));
                    m.insert("silver".into(), Dynamic::from(mob_data.gold));
                    for key in ["성격", "성별", "소속", "직위", "배우자"] {
                        m.insert(key.into(), Dynamic::from(attr_string(key)));
                    }
                    m.insert("feature".into(), Dynamic::from(attr_int("특성치")));
                    m.insert("insurance".into(), Dynamic::from(0_i64));
                    m.insert("mp_script".into(), Dynamic::from(mp_status_script(mob.mp)));
                    m.insert("alive".into(), Dynamic::from(mob.alive));
                    // 타겟 목록
                    let mut targets_arr = rhai::Array::new();
                    for target_name in &mob.targets {
                        targets_arr.push(Dynamic::from(target_name.clone()));
                    }
                    m.insert("targets".into(), Dynamic::from(targets_arr));
                    // 상태 (alive/dead)
                    let state = if mob.alive { "활동" } else { "사망" };
                    m.insert("state".into(), Dynamic::from(state));
                    arr.push(Dynamic::from(m));
                }
            }
            arr
        },
    );

    // get_room_players_admin(ob) - 관리자용 플레이어 상세 정보 (infoPlayer 대응)
    // 레벨, 체력, 내공, 힘, 민첩, 맷집, 타겟 등 상세 정보 반환
    let body_ptr_room_players_admin = body_ptr;
    engine.register_fn(
        "get_room_players_admin",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_room_players_admin };
            let viewer_name = body.get_name();

            // 같은 방의 다른 플레이어 목록
            let mut arr = rhai::Array::new();

            if let Some(crate::object::Value::String(raw)) = body.temp().get("_online_room_admin") {
                if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
                    for value in values {
                        if value.get("name").and_then(|v| v.as_str()) == Some(viewer_name.as_str())
                        {
                            continue;
                        }
                        let mut m = rhai::Map::new();
                        if let Some(object) = value.as_object() {
                            for (key, value) in object {
                                if value.is_array() || value.is_object() {
                                    m.insert(
                                        key.clone().into(),
                                        json_value_to_dynamic(value.clone()),
                                    );
                                } else if let Some(number) = value.as_i64() {
                                    m.insert(key.clone().into(), Dynamic::from(number));
                                } else if let Some(text) = value.as_str() {
                                    m.insert(key.clone().into(), Dynamic::from(text.to_string()));
                                }
                            }
                        }
                        arr.push(Dynamic::from(m));
                    }
                    return arr;
                }
            }

            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return arr,
            };
            let pos = match w.get_player_position(viewer_name.as_str()) {
                Some(p) => p,
                None => return arr,
            };
            let players = w.get_players_in_room(&pos.zone, &pos.room);
            for player_name in players {
                if player_name != viewer_name {
                    let mut m = rhai::Map::new();
                    m.insert("name".into(), Dynamic::from(player_name.clone()));
                    let path = format!("data/user/{}.json", player_name);
                    if let Ok(raw) = std::fs::read_to_string(path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                            if let Some(attrs) = json
                                .get("사용자오브젝트")
                                .and_then(|v| v.get("attr"))
                                .and_then(|v| v.as_object())
                            {
                                let int_attr = |key: &str| {
                                    attrs.get(key).and_then(|v| v.as_i64()).unwrap_or_else(|| {
                                        attrs
                                            .get(key)
                                            .and_then(|v| v.as_str())
                                            .and_then(|s| s.parse().ok())
                                            .unwrap_or(0)
                                    })
                                };
                                m.insert("level".into(), Dynamic::from(int_attr("레벨")));
                                m.insert("hp".into(), Dynamic::from(int_attr("체력")));
                                m.insert("max_hp".into(), Dynamic::from(int_attr("최고체력")));
                                m.insert("inner_power".into(), Dynamic::from(int_attr("내공")));
                                m.insert("strength".into(), Dynamic::from(int_attr("힘")));
                                m.insert("agility".into(), Dynamic::from(int_attr("민첩성")));
                                m.insert("money".into(), Dynamic::from(int_attr("은전")));
                                m.insert(
                                    "guild".into(),
                                    Dynamic::from(
                                        attrs
                                            .get("소속")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    ),
                                );
                                m.insert(
                                    "nickname".into(),
                                    Dynamic::from(
                                        attrs
                                            .get("방파별호")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    ),
                                );
                            }
                        }
                    }
                    arr.push(Dynamic::from(m));
                }
            }
            arr
        },
    );

    // look_room(ob) - 현재 방 설명 (look 명령용)
    let body_ptr_look = body_ptr;
    engine.register_fn("look_room", move |_ob: &mut rhai::Map| -> String {
        let body = unsafe { &*body_ptr_look };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return "방 정보를 가져올 수 없습니다.".to_string(),
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return "위치 정보가 없습니다.".to_string(),
        };
        if let Some(arc) = w.room_cache.get_room_cached(&pos.zone, &pos.room) {
            if let Ok(room_ref) = arc.read() {
                let room_name_formatted = format_room_header(&room_ref.display_name);
                let exits_str = format_exits_long(&room_ref);
                let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
                let mob_str = if mobs.is_empty() {
                    String::new()
                } else {
                    let mut mob_msgs = Vec::new();
                    for mob in mobs {
                        if let Some(mob_data) = w.mob_cache.get_mob(&mob.mob_key) {
                            if !mob_data.desc1.is_empty() {
                                mob_msgs.push(mob_data.desc1.clone());
                            }
                        }
                    }
                    if mob_msgs.is_empty() {
                        String::new()
                    } else {
                        format!("\r\n{}", mob_msgs.join("\r\n"))
                    }
                };
                let room_objs = w.get_room_objs(&pos.zone, &pos.room);
                let room_stack = w.get_room_objs_stack(&pos.zone, &pos.room);
                let item_str = build_room_objs_grouped(&room_objs, &room_stack);
                let mut out = String::new();
                out.push_str("\r\n");
                out.push_str(&room_name_formatted);
                out.push_str("\r\n\r\n");
                out.push_str(&room_ref.description.join("\r\n"));
                out.push_str("\r\n");
                out.push_str(&exits_str);
                if !mob_str.is_empty() {
                    out.push_str(&mob_str);
                    out.push_str("\r\n");
                }
                if !item_str.is_empty() {
                    out.push_str(&item_str);
                    out.push_str("\r\n");
                }
                return out;
            }
        }
        "방 정보를 가져올 수 없습니다.".to_string()
    });

    // move_player(ob, direction) - 플레이어 이동
    let body_ptr_move = body_ptr;
    engine.register_fn(
        "move_player",
        move |_ob: &mut rhai::Map, direction: &str| -> String {
            let body = unsafe { &mut *body_ptr_move };
            let name = body.get_name();
            if name.is_empty() {
                return "플레이어 정보가 없습니다.".to_string();
            }
            // 방향 문자열을 Direction으로 변환
            let dir = match crate::world::Direction::from_korean(direction) {
                Some(d) => d,
                None => return format!("{}쪽은 없습니다.", direction),
            };
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "이동할 수 없습니다.".to_string(),
            };
            match w.move_player(&name, dir) {
                Ok((zone, room)) => {
                    let location = format!("{}:{}", zone, room);
                    body.set("위치", location.as_str());
                    body.set("현재방", location.as_str());
                    String::new()
                }
                Err(e) => e,
            }
        },
    );

    // ============================================================
    // 플레이어 간 상호작용 efun (관리자용)
    // ============================================================

    // get_player_by_name(name) - 이름으로 플레이어 데이터 조회
    // 다른 플레이어의 데이터를 조회할 때 사용 (관리자 기능)
    // 현재는 제한적 구현 - 자기 자신만 가능
    let body_ptr_get = body_ptr;
    engine.register_fn("get_player_by_name", move |name: &str| -> Dynamic {
        let body = unsafe { &*body_ptr_get };
        if body.get_name() == name {
            // 자기 자신의 데이터 반환 (확장)
            let mut m = rhai::Map::new();
            m.insert("이름".into(), Dynamic::from(body.get_name()));
            m.insert("레벨".into(), Dynamic::from(body.get_int("레벨")));
            m.insert("hp".into(), Dynamic::from(body.get_hp()));
            m.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
            m.insert("은전".into(), Dynamic::from(body.get_int("은전")));
            m.insert("금전".into(), Dynamic::from(body.get_int("금전")));
            m.insert(
                "무림별호".into(),
                Dynamic::from(body.get_string("무림별호")),
            );
            m.insert("소속".into(), Dynamic::from(body.get_string("소속")));

            // 스킬 목록
            let skills: rhai::Array = body
                .skill_list
                .iter()
                .map(|s: &String| Dynamic::from(s.clone()))
                .collect();
            m.insert("스킬".into(), Dynamic::from(skills));

            // 인벤토리 (비스택 아이템)
            let mut inv_items: rhai::Array = rhai::Array::new();
            for arc in &body.object.objs {
                if let Ok(o) = arc.lock() {
                    let mut item = rhai::Map::new();
                    item.insert("이름".into(), Dynamic::from(o.getName()));
                    item.insert("인덱스".into(), Dynamic::from(o.getString("인덱스")));
                    inv_items.push(Dynamic::from(item));
                }
            }
            m.insert("인벤토리".into(), Dynamic::from(inv_items));

            // 스택 아이템
            let mut stack_items = rhai::Map::new();
            for (key, count) in &body.object.inv_stack {
                stack_items.insert(key.clone().into(), Dynamic::from(*count));
            }
            m.insert("스택아이템".into(), Dynamic::from(stack_items));

            Dynamic::from(m)
        } else {
            // 다른 플레이어는 현재 조회 불가
            Dynamic::UNIT
        }
    });

    // give_silver_to_player(from_ob, to_name, amount) - 은전 전송
    let body_ptr_give = body_ptr;
    let spec_give = spec.clone();
    engine.register_fn(
        "give_silver_to_player",
        move |_from_ob: &mut rhai::Map, to_name: &str, amount: i64| -> String {
            let body = unsafe { &*body_ptr_give };
            if body.get_name() == to_name {
                return "self".to_string();
            }
            if amount < 1 {
                return "usage".into();
            }
            let give = amount.min(body.get_int("은전").max(0));
            if give < 1 {
                return "no_money".into();
            }
            if let Ok(mut result) = spec_give.lock() {
                *result = Some(CommandResult::GiveToPlayer {
                    target_name: to_name.to_string(),
                    giver_name: body.get_name(),
                    give_silver: Some(give),
                    give_gold: None,
                    give_item: None,
                    give_item_stack: None,
                    deduct_from_giver: true,
                    bypass_item_limits: false,
                });
            }
            String::new()
        },
    );

    // teach_skill_to_player(teacher_ob, student_name, skill_name) - 무공 전수
    let body_ptr_teach = body_ptr;
    engine.register_fn(
        "teach_skill_to_player",
        move |_teacher_ob: &mut rhai::Map, student_name: &str, skill_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_teach };
            if body.get_name() == student_name {
                return "self".to_string();
            }
            let target_exists = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                cell.borrow().as_ref().is_some_and(|targets| {
                    targets.iter().any(|target| {
                        target.kind == RoomMugongTargetKind::Player
                            && !target.transparent
                            && target.name == student_name
                    })
                })
            });
            if !target_exists {
                return "not_found".to_string();
            }
            let Ok(json) = serde_json::to_string(&(student_name, skill_name)) else {
                return "not_found".to_string();
            };
            body.temp_mut()
                .insert(TEACH_SKILL_REQUEST.to_string(), Value::String(json));
            "ok".to_string()
        },
    );

    // check_player_skill(player_name, skill_name) - 플레이어 스킬 보유 확인
    let body_ptr_check = body_ptr;
    engine.register_fn(
        "check_player_skill",
        move |player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &*body_ptr_check };
            if body.get_name() == player_name {
                return body.skill_list.contains(&skill_name.to_string());
            }
            PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                cell.borrow().as_ref().is_some_and(|targets| {
                    targets.iter().any(|target| {
                        target.kind == RoomMugongTargetKind::Player
                            && !target.transparent
                            && target.name == player_name
                            && target.skill_levels.contains_key(skill_name)
                    })
                })
            })
        },
    );

    // ============================================================
    // 플레이어 상호작용 관련 추가 efun
    // ============================================================

    // find_player_online(name) - 플레이어 접속 중인지 확인
    // 접속 중이면 true 반환
    engine.register_fn("find_player_online", move |name: &str| -> bool {
        if let Ok(w) = get_world_state().try_read() {
            w.get_player_position(name).is_some()
        } else {
            false
        }
    });

    // Python admin skill commands require the target to be in the viewer's
    // room, not merely connected globally.
    let body_ptr_room_player_exists = body_ptr;
    engine.register_fn(
        "room_player_exists",
        move |_ob: &mut rhai::Map, target: &str| -> bool {
            let body = unsafe { &*body_ptr_room_player_exists };
            let name = body.get_name();
            let Ok(w) = get_world_state().try_read() else {
                return false;
            };
            let Some(pos) = w.get_player_position(&name) else {
                return false;
            };
            w.get_players_in_room(&pos.zone, &pos.room)
                .iter()
                .any(|candidate| candidate == target)
        },
    );

    // send_to_player(player_name, message) - 특정 플레이어에게 메시지 전송
    // 성공 시 true 반환
    let user_sends_clone = user_sends.clone();
    engine.register_fn(
        "send_to_player",
        move |player_name: &str, message: &str| -> bool {
            if player_name.is_empty() || message.is_empty() {
                return false;
            }
            // 플레이어가 접속 중인지 확인
            let online = if let Ok(w) = get_world_state().try_read() {
                w.get_player_position(player_name).is_some()
            } else {
                false
            };
            if !online {
                return false;
            }
            // user_sends에 메시지 추가
            if let Ok(mut sends) = user_sends_clone.lock() {
                sends.push((player_name.to_string(), message.to_string()));
                true
            } else {
                false
            }
        },
    );

    // give_money_to_player(from_ob, to_name, amount) - 돈 전송
    // 성공 시 "", 실패 시 에러 코드 반환
    let spec_money = spec.clone();
    let body_ptr_money = body_ptr;
    engine.register_fn(
        "give_money_to_player",
        move |_from_ob: &mut rhai::Map, to_name: &str, amount: i64| -> String {
            let body = unsafe { &*body_ptr_money };

            // 파라미터 검증
            if amount < 1 {
                return "usage".to_string(); // 잘못된 금액
            }

            let my_name = body.get_name();

            // 자기 자신에게는 줄 수 없음
            if my_name == to_name {
                return "self".to_string();
            }

            // 상대방이 접속 중인지 확인
            let target_online = if let Ok(w) = get_world_state().try_read() {
                w.get_player_position(to_name).is_some()
            } else {
                false
            };
            if !target_online {
                return "not_online".to_string();
            }

            // 보내는 사람의 돈 확인 (은전)
            let have = body.get_int("은전");
            if have < amount {
                return "no_money".to_string();
            }

            // CommandResult에 GiveToPlayer 설정 (실제 전송은 핸들러에서)
            if let Ok(mut s) = spec_money.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: to_name.to_string(),
                    giver_name: my_name,
                    give_silver: Some(amount),
                    give_gold: None,
                    give_item: None,
                    give_item_stack: None,
                    deduct_from_giver: true,
                    bypass_item_limits: false,
                });
            }

            String::new() // 성공
        },
    );

    // give_item_to_player(from_ob, to_name, item_name) - 아이템 전송
    // 성공 시 "", 실패 시 에러 코드 반환
    let spec_item = spec.clone();
    let body_ptr_item = body_ptr;
    engine.register_fn(
        "give_item_to_player",
        move |_from_ob: &mut rhai::Map, to_name: &str, item_name: &str| -> String {
            let body = unsafe { &*body_ptr_item };

            // 파라미터 검증
            if item_name.is_empty() {
                return "usage".to_string();
            }

            let my_name = body.get_name();

            // 자기 자신에게는 줄 수 없음
            if my_name == to_name {
                return "self".to_string();
            }

            // 상대방이 접속 중인지 확인
            let target_online = if let Ok(w) = get_world_state().try_read() {
                w.get_player_position(to_name).is_some()
            } else {
                false
            };
            if !target_online {
                return "not_online".to_string();
            }

            // 아이템이 있는지 확인 (스택 아이템 우선)
            let mut found_item = false;
            let mut give_stack: Option<(String, i64)> = None;
            let mut give_non_stack: Option<(String, usize, usize)> = None;

            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    if have > 0 {
                        found_item = true;
                        give_stack = Some((key.clone(), 1)); // 기본 1개
                    }
                }
            }

            // 비스택 아이템 확인
            if !found_item {
                if let Some(_arc) = body.object.findObjInven(item_name, 1) {
                    found_item = true;
                    give_non_stack = Some((item_name.to_string(), 1, 1));
                }
            }

            if !found_item {
                return "no_item".to_string();
            }

            // CommandResult에 GiveToPlayer 설정
            if let Ok(mut s) = spec_item.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: to_name.to_string(),
                    giver_name: my_name,
                    give_silver: None,
                    give_gold: None,
                    give_item: give_non_stack,
                    give_item_stack: give_stack,
                    deduct_from_giver: true,
                    bypass_item_limits: false,
                });
            }

            String::new() // 성공
        },
    );

    // add_skill_to_player(ob, player_name, skill_name) - 스킬 추가
    // 성공 시 true 반환
    let body_ptr_add_skill = body_ptr;
    engine.register_fn(
        "add_skill_to_player",
        move |_ob: &mut rhai::Map, player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_add_skill };
            if skill_name.is_empty() {
                return false;
            }
            // 다른 플레이어는 같은 방 스냅샷을 확인한 뒤 네트워크 경계에서
            // 실제 Body를 변경한다.
            if body.get_name() != player_name {
                let target_exists = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                    cell.borrow().as_ref().is_some_and(|targets| {
                        targets.iter().any(|target| {
                            target.kind == RoomMugongTargetKind::Player
                                && !target.transparent
                                && target.name == player_name
                        })
                    })
                });
                if !target_exists {
                    return false;
                }
                let Ok(json) = serde_json::to_string(&(player_name, skill_name)) else {
                    return false;
                };
                body.temp_mut()
                    .insert(TEACH_SKILL_REQUEST.to_string(), Value::String(json));
                return true;
            }
            // 이미 있는지 확인
            if body.skill_list.contains(&skill_name.to_string()) {
                return true; // 이미 있음
            }

            // 스킬 추가
            body.skill_list.push(skill_name.to_string());
            body.skill_map.insert(
                skill_name.to_string(),
                crate::player::SkillTraining::new(1, 0),
            );
            true
        },
    );

    // ============================================================
    // SKILL/ABILITY 관련 efun
    // ============================================================

    // Helper function to parse MP cost from skill 속성
    fn parse_mp_cost(skill_data: &serde_json::Value) -> i64 {
        if let Some(attrs) = skill_data.get("속성") {
            let attr_str: String = if attrs.is_string() {
                attrs.as_str().unwrap_or("").to_string()
            } else if attrs.is_array() {
                attrs
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            // "내공소모 240" -> 240
            for part in attr_str.split_whitespace() {
                if part == "내공소모" {
                    if let Ok(val) = attr_str
                        .split("내공소모")
                        .nth(1)
                        .unwrap_or("")
                        .split_whitespace()
                        .next()
                        .unwrap_or("0")
                        .parse::<i64>()
                    {
                        return val;
                    }
                }
            }
        }
        0
    }

    // Helper function to parse skill bonuses from 속성
    // Returns (hp_bonus, mp_bonus, str_bonus, dex_bonus, arm_bonus, skill_bonus)
    fn parse_skill_bonuses(skill_data: &serde_json::Value) -> (i64, i64, i64, i64, i64, i64) {
        let mut hp_bonus = 0i64;
        let mut mp_bonus = 0i64;
        let mut str_bonus = 0i64;
        let mut dex_bonus = 0i64;
        let mut arm_bonus = 0i64;
        let mut skill_bonus = 0i64;

        if let Some(attrs) = skill_data.get("속성") {
            let attr_str: String = if attrs.is_string() {
                attrs.as_str().unwrap_or("").to_string()
            } else if attrs.is_array() {
                attrs
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Parse each bonus type
            let parts: Vec<&str> = attr_str.split_whitespace().collect();
            let mut i = 0;
            while i < parts.len() {
                match parts[i] {
                    "체력증가" | "체력회복" => {
                        if i + 1 < parts.len() {
                            hp_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "내공증가" | "내공회복" => {
                        if i + 1 < parts.len() {
                            mp_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "힘증가" => {
                        if i + 1 < parts.len() {
                            str_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "민첩증가" => {
                        if i + 1 < parts.len() {
                            dex_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "맷집증가" => {
                        if i + 1 < parts.len() {
                            arm_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    "위력" | "보너스" => {
                        if i + 1 < parts.len() {
                            skill_bonus = parts[i + 1].parse().unwrap_or(0);
                            i += 2;
                            continue;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }

        (
            hp_bonus,
            mp_bonus,
            str_bonus,
            dex_bonus,
            arm_bonus,
            skill_bonus,
        )
    }

    // Helper function to get skill description from 속성
    fn get_skill_description(skill_data: &serde_json::Value) -> String {
        let mut desc_parts = Vec::new();

        if let Some(kind) = skill_data.get("종류") {
            if let Some(s) = kind.as_str() {
                desc_parts.push(format!("종류: {}", s));
            }
        }

        if let Some(attrs) = skill_data.get("속성") {
            let attr_str = if attrs.is_string() {
                attrs.as_str().unwrap_or("").to_string()
            } else if attrs.is_array() {
                attrs
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default()
            } else {
                "".to_string()
            };
            if !attr_str.is_empty() {
                desc_parts.push(format!("속성: {}", attr_str));
            }
        }

        if let Some(prob) = skill_data.get("확률") {
            if let Some(n) = prob.as_i64() {
                desc_parts.push(format!("확률: {}", n));
            }
        }

        desc_parts.join(" | ")
    }

    // use_skill(ob, skill_name, target) - 무공 스킬 사용
    // Returns "" on success, error string on failure
    let body_ptr_use_skill = body_ptr;
    engine.register_fn(
        "use_skill",
        move |_ob: &mut rhai::Map, skill_name: &str, _target: &str| -> String {
            let body = unsafe { &mut *body_ptr_use_skill };

            // Check if player has the skill
            if !body.skill_list.contains(&skill_name.to_string()) {
                return format!("배우지 않은 무공입니다: {}", skill_name);
            }

            // Check cooldown
            let cooldown_remaining = body.get_skill_cooldown_remaining(skill_name);
            if cooldown_remaining > 0 {
                return format!("쿨다운 중입니다. {}초 남음.", cooldown_remaining);
            }

            // Load skill data
            let skill_path = "data/config/skill.json";
            let skill_data = match std::fs::read_to_string(skill_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => value,
                    Err(_) => return "스킬 데이터를 읽을 수 없습니다.".to_string(),
                },
                Err(_) => return "스킬 데이터를 찾을 수 없습니다.".to_string(),
            };

            let skill_info = match skill_data.get(skill_name) {
                Some(s) => s,
                None => return format!("스킬을 찾을 수 없습니다: {}", skill_name),
            };

            // Check MP cost
            let mp_cost = parse_mp_cost(skill_info);
            if mp_cost > 0 {
                let current_mp = body.get_mp();
                if current_mp < mp_cost {
                    return format!("내공이 부족합니다. 필요: {}, 현재: {}", mp_cost, current_mp);
                }
                // Deduct MP
                let new_mp = current_mp - mp_cost;
                body.set("내공", new_mp);
            }

            // Set skill cast time (mark as used)
            body.set_skill_cast_time(skill_name);

            // Get skill level
            let skill_level = body
                .skill_map
                .get(skill_name)
                .map(|t| t.level as i32)
                .unwrap_or(1);

            // Parse skill bonuses from 속성
            let (hp_bonus, mp_bonus, str_bonus, dex_bonus, arm_bonus, skill_bonus) =
                parse_skill_bonuses(skill_info);

            // Apply skill effects to player (healing, stat boosts, etc.)
            let effects = crate::combat::apply_skill_effects(
                body, skill_name, hp_bonus, mp_bonus, str_bonus, dex_bonus, arm_bonus,
            );

            // Log effects
            for effect in &effects {
                if !effect.message.is_empty() {
                    println!("[SKILL] {}", effect.message);
                }
            }

            // Calculate skill damage if there's a target
            if !_target.is_empty() {
                let damage_result = crate::combat::calculate_skill_damage(
                    body,
                    skill_name,
                    skill_level,
                    skill_bonus,
                    _target,
                );

                // Log damage
                if damage_result.hit {
                    println!(
                        "[SCRIPT] use_skill: {} used by {} on {} for {} damage",
                        skill_name,
                        body.get_name(),
                        _target,
                        damage_result.final_damage
                    );
                } else {
                    println!(
                        "[SCRIPT] use_skill: {} used by {} on {} (missed)",
                        skill_name,
                        body.get_name(),
                        _target
                    );
                }
            } else {
                println!(
                    "[SCRIPT] use_skill: {} used by {} (self-buff)",
                    skill_name,
                    body.get_name()
                );
            }

            // Success - return empty string
            "".to_string()
        },
    );

    // learn_skill(ob, skill_name) - 새 스킬 학습
    // Returns "" on success, error string on failure
    let body_ptr_learn = body_ptr;
    engine.register_fn(
        "learn_skill",
        move |_ob: &mut rhai::Map, skill_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_learn };

            // Check if already learned
            if body.skill_list.contains(&skill_name.to_string()) {
                return format!("이미 배운 무공입니다: {}", skill_name);
            }

            // Validate skill exists
            let skill_path = "data/config/skill.json";
            let skill_data = match std::fs::read_to_string(skill_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => value,
                    Err(_) => return "스킬 데이터를 읽을 수 없습니다.".to_string(),
                },
                Err(_) => return "스킬 데이터를 찾을 수 없습니다.".to_string(),
            };

            if skill_data.get(skill_name).is_none() {
                return format!("존재하지 않는 무공입니다: {}", skill_name);
            }

            // Add to skill_list
            body.skill_list.push(skill_name.to_string());

            // Initialize skill_map with level 1, exp 0
            body.skill_map.insert(
                skill_name.to_string(),
                crate::player::SkillTraining::new(1, 0),
            );

            println!(
                "[SCRIPT] learn_skill: {} learned by {}",
                skill_name,
                body.get_name()
            );

            "".to_string()
        },
    );

    // forget_skill(ob, skill_name) - 스킬 잊기
    // Returns true on success
    let body_ptr_forget = body_ptr;
    engine.register_fn(
        "forget_skill",
        move |_ob: &mut rhai::Map, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_forget };

            // Check if has the skill
            if !body.skill_list.contains(&skill_name.to_string()) {
                return false;
            }

            // Remove from skill_list
            body.skill_list.retain(|s| s != skill_name);

            // Remove from skill_map
            body.skill_map.remove(skill_name);

            println!(
                "[SCRIPT] forget_skill: {} forgotten by {}",
                skill_name,
                body.get_name()
            );

            true
        },
    );

    // get_skill_list(ob) - 배운 무공 목록 가져오기
    // Returns Array of skill names
    let body_ptr_get_skills = body_ptr;
    engine.register_fn(
        "get_skill_list",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_get_skills };

            let mut result = rhai::Array::new();
            for skill_name in &body.skill_list {
                result.push(Dynamic::from(skill_name.clone()));
            }
            result
        },
    );

    // get_skill_level(ob, skill_name) - 무공 수련 레벨 가져오기
    // Returns level as i64, 0 if not trained
    let body_ptr_get_level = body_ptr;
    engine.register_fn(
        "get_skill_level",
        move |_ob: &mut rhai::Map, skill_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_level };

            if let Some(training) = body.skill_map.get(skill_name) {
                training.level as i64
            } else {
                0
            }
        },
    );

    // train_skill(ob, skill_name, exp) - 무공 수련 (경험치 추가)
    // Returns new level after training
    let body_ptr_train = body_ptr;
    engine.register_fn(
        "train_skill",
        move |_ob: &mut rhai::Map, skill_name: &str, exp_add: i64| -> i64 {
            let body = unsafe { &mut *body_ptr_train };

            // Get current training or initialize new
            let current = body
                .skill_map
                .get(skill_name)
                .copied()
                .unwrap_or_else(|| crate::player::SkillTraining::new(1, 0));

            let mut new_exp = current.exp as i64 + exp_add;
            let mut new_level = current.level;

            // Simple level up logic: every 100 exp = 1 level, max 12
            while new_exp >= 100 && new_level < 12 {
                new_exp -= 100;
                new_level += 1;
            }

            // Update skill_map
            body.skill_map.insert(
                skill_name.to_string(),
                crate::player::SkillTraining::new(new_level, new_exp as u32),
            );

            println!(
                "[SCRIPT] train_skill: {} trained by {}, exp+{}, now level {}",
                skill_name,
                body.get_name(),
                exp_add,
                new_level
            );

            new_level as i64
        },
    );

    let body_ptr_set_skill = body_ptr;
    engine.register_fn(
        "set_skill_training",
        move |_ob: &mut rhai::Map, target: &str, skill: &str, level: i64| -> String {
            let body = unsafe { &mut *body_ptr_set_skill };
            if target == body.get_name() {
                body.skill_map.insert(
                    skill.to_string(),
                    crate::player::SkillTraining::new(level, 199_999),
                );
                return "ok".into();
            }
            let exists = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                cell.borrow()
                    .as_ref()
                    .is_some_and(|targets| targets.iter().any(|candidate| candidate.name == target))
            });
            if !exists {
                return "not_found".into();
            }
            let Ok(json) = serde_json::to_string(&(target, skill, level)) else {
                return "not_found".into();
            };
            body.temp_mut()
                .insert(SET_SKILL_REQUEST.to_string(), Value::String(json));
            "ok".into()
        },
    );

    // get_skill_desc(skill_name) - 무공 설명 가져오기
    // Returns description string from MUGONG data
    engine.register_fn("get_skill_desc", move |skill_name: &str| -> String {
        let skill_path = "data/config/skill.json";
        match std::fs::read_to_string(skill_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(skill_info) = value.get(skill_name) {
                        get_skill_description(skill_info)
                    } else {
                        "".to_string()
                    }
                }
                Err(_) => "".to_string(),
            },
            Err(_) => "".to_string(),
        }
    });

    // cast_spell(ob, spell_name, target) - 주문 시전
    // Similar to use_skill but for spells (could use spell.json in future)
    let body_ptr_cast = body_ptr;
    engine.register_fn(
        "cast_spell",
        move |_ob: &mut rhai::Map, spell_name: &str, _target: &str| -> String {
            let body = unsafe { &mut *body_ptr_cast };

            // Check if player has the spell
            if !body.skill_list.contains(&spell_name.to_string()) {
                return format!("배우지 않은 주문입니다: {}", spell_name);
            }

            // For now, spells use the same skill.json data
            let skill_path = "data/config/skill.json";
            let skill_data = match std::fs::read_to_string(skill_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => value,
                    Err(_) => return "주문 데이터를 읽을 수 없습니다.".to_string(),
                },
                Err(_) => return "주문 데이터를 찾을 수 없습니다.".to_string(),
            };

            let spell_info = match skill_data.get(spell_name) {
                Some(s) => s,
                None => return format!("주문을 찾을 수 없습니다: {}", spell_name),
            };

            // Check MP cost
            let mp_cost = parse_mp_cost(spell_info);
            if mp_cost > 0 {
                let current_mp = body.get_mp();
                if current_mp < mp_cost {
                    return format!("내공이 부족합니다. 필요: {}, 현재: {}", mp_cost, current_mp);
                }
                body.set("내공", current_mp - mp_cost);
            }

            // Spell-specific effects are authored by the hot-reloaded Rhai
            // command; this legacy helper only performs the shared MP cost.
            println!(
                "[SCRIPT] cast_spell: {} cast by {}",
                spell_name,
                body.get_name()
            );

            "".to_string()
        },
    );

    // has_skill(ob, skill_name) - 스킬 보유 여부 확인
    // Returns true if player has the skill
    let body_ptr_has_skill2 = body_ptr;
    engine.register_fn(
        "has_skill",
        move |_ob: &mut rhai::Map, skill_name: &str| -> bool {
            let body = unsafe { &*body_ptr_has_skill2 };
            body.skill_list.contains(&skill_name.to_string())
        },
    );

    // Python 비전.py는 일반 무공 목록이 아니라 비전이름 배열의 정확한
    // 원소 포함 여부를 검사한다.
    let body_ptr_has_vision = body_ptr;
    engine.register_fn(
        "has_vision",
        move |_ob: &mut rhai::Map, vision_name: &str| -> bool {
            let body = unsafe { &*body_ptr_has_vision };
            body.has_secret_skill(vision_name)
        },
    );

    let body_ptr_set_vision = body_ptr;
    engine.register_fn(
        "set_vision",
        move |_ob: &mut rhai::Map, vision_name: &str| {
            let body = unsafe { &mut *body_ptr_set_vision };
            body.set_vision_setting(vision_name);
        },
    );

    // remove_skill_from_player(ob, player_name, skill_name) - 스킬 제거
    // 성공 시 true 반환
    let body_ptr_remove_skill = body_ptr;
    engine.register_fn(
        "remove_skill_from_player",
        move |_ob: &mut rhai::Map, player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_remove_skill };
            if skill_name.is_empty() {
                return false;
            }
            // 현재는 자기 자신의 스킬만 제거 가능
            if body.get_name() != player_name {
                let target_has_skill = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                    cell.borrow().as_ref().is_some_and(|targets| {
                        targets.iter().any(|target| {
                            target.kind == RoomMugongTargetKind::Player
                                && !target.transparent
                                && target.name == player_name
                                && target.skill_levels.contains_key(skill_name)
                        })
                    })
                });
                if !target_has_skill {
                    return false;
                }
                let Ok(json) = serde_json::to_string(&(player_name, skill_name)) else {
                    return false;
                };
                body.temp_mut()
                    .insert(REMOVE_SKILL_REQUEST.to_string(), Value::String(json));
                return true;
            }
            // 스킬 제거
            let original_len = body.skill_list.len();
            body.skill_list.retain(|s| s != skill_name);
            let removed = body.skill_list.len() < original_len;
            if removed {
                // 저장
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            removed
        },
    );

    // player_has_skill(ob, player_name, skill_name) - 플레이어 스킬 보유 확인
    // 스킬이 있으면 true 반환
    let body_ptr_has_skill = body_ptr;
    engine.register_fn(
        "player_has_skill",
        move |_ob: &mut rhai::Map, player_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &*body_ptr_has_skill };
            if skill_name.is_empty() {
                return false;
            }
            // 현재는 자기 자신의 스킬만 확인 가능
            if body.get_name() == player_name {
                body.skill_list.contains(&skill_name.to_string())
            } else {
                false
            }
        },
    );

    // ============================================================
    // 오브젝트/아이템 조작 관련 efun
    // ============================================================

    // find_obj_in_inventory(ob, obj_name) - 플레이어 인벤토리에서 오브젝트 찾기
    // 오브젝트를 찾으면 오브젝트 데이터를 반환, 없으면 UNIT 반환
    let body_ptr_fii = body_ptr;
    engine.register_fn(
        "find_obj_in_inventory",
        move |_ob: &mut rhai::Map, obj_name: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_fii };
            for obj_arc in &body.object.objs {
                if let Ok(obj) = obj_arc.lock() {
                    let matches = obj.getName() == obj_name
                        || (!obj.getString("반응이름").is_empty()
                            && obj.getString("반응이름").contains(obj_name));
                    if matches {
                        // 오브젝트 데이터를 Map으로 변환하여 반환
                        let mut obj_data = rhai::Map::new();
                        obj_data.insert("이름".into(), Dynamic::from(obj.getName()));
                        obj_data.insert("표시".into(), Dynamic::from(obj.getNameA())); // getNameA를 표시로 사용
                        obj_data.insert("종류".into(), Dynamic::from(obj.getString("종류")));
                        drop(obj);
                        return Dynamic::from(obj_data);
                    }
                }
            }
            Dynamic::UNIT
        },
    );

    // JSON catalogue operations. These efuns own only persistence/state,
    // while Rhai owns text.
    let body_ptr_book_entries = body_ptr;
    engine.register_fn("book_entries", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_book_entries };
        let catalog_path = book_catalog_path(body);
        let Ok(entries) = crate::book::load(&catalog_path) else {
            return rhai::Array::new();
        };
        entries
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                let mut m = rhai::Map::new();
                m.insert("번호".into(), Dynamic::from((i + 1) as i64));
                m.insert(
                    "이름".into(),
                    Dynamic::from(crate::book::dict_get_string(&e, "이름")),
                );
                m.insert(
                    "등록자".into(),
                    Dynamic::from(crate::book::dict_get_string(&e, "등록자")),
                );
                m.insert(
                    "고유번호".into(),
                    Dynamic::from(crate::book::dict_get_string(&e, "고유번호")),
                );
                m.insert(
                    "대여".into(),
                    Dynamic::from(crate::book::dict_get_string(&e, "대여")),
                );
                m.insert(
                    "대여가능".into(),
                    Dynamic::from(crate::book::dict_get_bool(&e, "대여가능")),
                );
                Dynamic::from(m)
            })
            .collect()
    });

    let body_ptr_book = body_ptr;
    engine.register_fn(
        "book_borrow",
        move |_ob: &mut rhai::Map, number: i64| -> String {
            let body = unsafe { &mut *body_ptr_book };
            let catalog_path = book_catalog_path(body);
            if number < 1 {
                return "unavailable".into();
            }
            let Ok(mut entries) = crate::book::load(&catalog_path) else {
                return "unavailable".into();
            };
            let idx = number as usize - 1;
            let Some(entry) = entries.get_mut(idx) else {
                return "unavailable".into();
            };
            if !crate::book::dict_get_bool(entry, "대여가능") {
                return "borrowed".into();
            }
            let key = crate::book::dict_get_string(entry, "인덱스");
            let Some((item, _)) = object_from_item_json(&key) else {
                return "unavailable".into();
            };
            if let Ok(mut obj) = item.lock() {
                if let Some(attributes) = crate::book::dict_get(entry, "attr") {
                    inventory_compat::replace_item_attributes_from_json(&mut obj, attributes);
                }
                obj.set("고유번호", crate::book::dict_get_string(entry, "고유번호"));
            }
            if crate::book::mark_borrowed(
                &catalog_path,
                number as usize,
                &body.get_name(),
            )
            .is_err()
            {
                return "persist_failed".into();
            }
            body.object.objs.push(item);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            "ok".into()
        },
    );

    let body_ptr_guard_qi = body_ptr;
    engine.register_fn("inject_guard_qi", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &mut *body_ptr_guard_qi };
        let mut healed = rhai::Array::new();
        let mut spent = 0i64;
        let strength = body.get_int("힘");
        let guard_count = body
            .object
            .objs
            .iter()
            .filter(|arc| arc.lock().is_ok_and(|obj| obj.getString("종류") == "호위"))
            .count();
        for arc in &body.object.objs {
            let Ok(mut guard) = arc.lock() else { continue };
            if guard.getString("종류") != "호위" {
                continue;
            }
            let max_hp = object_from_item_json(&guard.getString("인덱스"))
                .and_then(|(template, _)| template.lock().ok().map(|item| item.getInt("체력")))
                .unwrap_or_else(|| guard.getInt("최고체력").max(guard.getInt("체력")));
            let hp = guard.getInt("체력");
            if hp >= max_hp {
                continue;
            }
            let cost = strength * guard.getInt("내공감소") / 100;
            if body.get_int("내공") - spent - cost < 0 {
                break;
            }
            let gain = max_hp * guard.getInt("체력증가") / 100;
            guard.set("체력", (hp + gain).min(max_hp));
            spent += cost;
            let mut m = rhai::Map::new();
            m.insert("이름".into(), Dynamic::from(guard.getName()));
            m.insert("회복".into(), Dynamic::from(gain));
            healed.push(Dynamic::from(m));
        }
        if !healed.is_empty() {
            body.set("내공", body.get_int("내공") - spent);
        }
        let mut result = rhai::Map::new();
        let status = if guard_count == 0 {
            "no_guards"
        } else if healed.is_empty() {
            let needs_heal = body.object.objs.iter().any(|arc| {
                arc.lock().is_ok_and(|guard| {
                    guard.getString("종류") == "호위"
                        && guard.getInt("체력")
                            < object_from_item_json(&guard.getString("인덱스"))
                                .and_then(|(template, _)| {
                                    template.lock().ok().map(|item| item.getInt("체력"))
                                })
                                .unwrap_or_else(|| {
                                    guard.getInt("최고체력").max(guard.getInt("체력"))
                                })
                })
            });
            if needs_heal {
                "mp_shortage"
            } else {
                "full"
            }
        } else {
            "ok"
        };
        result.insert("status".into(), Dynamic::from(status));
        result.insert("healed".into(), Dynamic::from(healed));
        result.insert("spent".into(), Dynamic::from(spent));
        Dynamic::from(result)
    });

    let body_ptr_map = body_ptr;
    engine.register_fn(
        "map_explore_directions",
        move |_ob: &mut rhai::Map, excluded: &str| -> rhai::Array {
            let body = unsafe { &*body_ptr_map };
            python_map_explore(body, excluded)
        },
    );
    let body_ptr_book_register = body_ptr;
    engine.register_fn(
        "book_register",
        move |_ob: &mut rhai::Map, item_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_book_register };
            let catalog_path = book_catalog_path(body);
            let Some((pos, key, name, attributes)) =
                body.object.objs.iter().enumerate().find_map(|(i, arc)| {
                    let obj = arc.lock().ok()?;
                    (obj.getName() == item_name || obj.getString("반응이름").contains(item_name))
                        .then(|| {
                            let attributes = inventory_compat::item_attributes_to_json(&obj)
                                .as_object()
                                .cloned()
                                .unwrap_or_default();
                            (i, obj.getString("인덱스"), obj.getName(), attributes)
                        })
                })
            else {
                return "no_item".into();
            };
            if key.is_empty() {
                return "cannot_register".into();
            }
            let Some((kind, cannot_give, item_id)) = body.object.objs[pos].lock().ok().map(|o| {
                (
                    o.getString("종류"),
                    o.checkAttr("아이템속성", "줄수없음"),
                    o.getString("고유번호"),
                )
            }) else {
                return "cannot_register".into();
            };
            if kind != "무기" || cannot_give || !item_id.is_empty() {
                return "cannot_register".into();
            }
            if crate::book::register_item(
                &catalog_path,
                &key,
                &name,
                &body.get_name(),
                attributes,
            )
            .is_err()
            {
                return "cannot_register".into();
            }
            body.object.objs.remove(pos);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            "ok".into()
        },
    );

    let body_ptr_book_return = body_ptr;
    engine.register_fn(
        "book_return",
        move |_ob: &mut rhai::Map, item_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_book_return };
            let catalog_path = book_catalog_path(body);
            let Some((pos, item_id)) = body.object.objs.iter().enumerate().find_map(|(i, arc)| {
                let obj = arc.lock().ok()?;
                (obj.getName() == item_name || obj.getString("반응이름").contains(item_name))
                    .then(|| (i, obj.getString("고유번호")))
            }) else {
                return "no_item".into();
            };
            if item_id.is_empty() {
                return "not_returnable".into();
            }
            let Ok(entries) = crate::book::load(&catalog_path) else {
                return "catalog_unavailable".into();
            };
            if entries.is_empty() {
                return "catalog_unavailable".into();
            }
            if !entries
                .iter()
                .any(|entry| crate::book::dict_get_string(entry, "고유번호") == item_id)
            {
                return "cannot_return".into();
            }
            if crate::book::mark_returned(&catalog_path, &item_id).is_err() {
                return "cannot_return".into();
            }
            body.object.objs.remove(pos);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            "ok".into()
        },
    );

    let body_ptr_book_cancel = body_ptr;
    engine.register_fn(
        "book_cancel",
        move |_ob: &mut rhai::Map, number: i64| -> String {
            let body = unsafe { &mut *body_ptr_book_cancel };
            let catalog_path = book_catalog_path(body);
            if number < 1 {
                return "unavailable".into();
            }
            let Ok(entries) = crate::book::load(&catalog_path) else {
                return "unavailable".into();
            };
            let Some(candidate) = entries.get(number as usize - 1) else {
                return "unavailable".into();
            };
            if crate::book::dict_get_string(candidate, "등록자") != body.get_name() {
                return "not_owner".into();
            }
            if !crate::book::dict_get_bool(candidate, "대여가능") {
                return "borrowed".into();
            }
            let key = crate::book::dict_get_string(candidate, "인덱스");
            if object_from_item_json(&key).is_none() {
                return "unavailable".into();
            }
            let Ok(entry) = crate::book::remove_entry(
                &catalog_path,
                number as usize,
                &body.get_name(),
                0,
                true,
            ) else {
                return "unavailable".into();
            };
            let key = crate::book::dict_get_string(&entry, "인덱스");
            let Some((item, _)) = object_from_item_json(&key) else {
                return "unavailable".into();
            };
            if let Ok(mut obj) = item.lock() {
                if let Some(attributes) = crate::book::dict_get(&entry, "attr") {
                    inventory_compat::replace_item_attributes_from_json(&mut obj, attributes);
                }
                obj.set("고유번호", "");
            }
            body.object.objs.push(item);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            "ok".into()
        },
    );
    let body_ptr_book_delete = body_ptr;
    engine.register_fn(
        "book_delete",
        move |_ob: &mut rhai::Map, number: i64| -> String {
            let body = unsafe { &mut *body_ptr_book_delete };
            let catalog_path = book_catalog_path(body);
            if number < 1 {
                return "unavailable".into();
            }
            let Ok(entries) = crate::book::load(&catalog_path) else {
                return "unavailable".into();
            };
            let Some(candidate) = entries.get(number as usize - 1) else {
                return "unavailable".into();
            };
            if body.get_int("관리자등급") < 1000
                && crate::book::dict_get_string(candidate, "등록자") != body.get_name()
            {
                return "not_owner".into();
            }
            if crate::book::remove_entry(
                &catalog_path,
                number as usize,
                &body.get_name(),
                body.get_int("관리자등급"),
                false,
            )
            .is_ok()
            {
                "ok".into()
            } else {
                "unavailable".into()
            }
        },
    );
    let body_ptr_save_object = body_ptr;
    engine.register_fn(
        "save_object_python",
        move |_ob: &mut rhai::Map, line: &str| -> rhai::Map {
            let body = unsafe { &mut *body_ptr_save_object };
            let result = |status: &str, path: String| {
                let mut map = rhai::Map::new();
                map.insert("status".into(), Dynamic::from(status.to_string()));
                map.insert("path".into(), Dynamic::from(path));
                map
            };
            if line.trim().is_empty() {
                let Some((zone, room)) = current_body_position(body) else {
                    return result("missing", String::new());
                };
                let path = format!("data/map/{zone}/{room}.json");
                let Ok(text) = std::fs::read_to_string(&path) else {
                    return result("open_failed", String::new());
                };
                let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
                    return result("open_failed", String::new());
                };
                if root.get("맵정보").and_then(|v| v.as_object()).is_none() {
                    return result("open_failed", String::new());
                }
                if std::fs::write(&path, serde_json::to_string_pretty(&root).unwrap_or(text))
                    .is_err()
                {
                    return result("open_failed", String::new());
                }
                return result("ok", path);
            }

            let Some((zone, room)) = current_body_position(body) else {
                return result("missing", String::new());
            };
            let mob_target = get_world_state().read().ok().and_then(|world| {
                world
                    .mob_cache
                    .get_all_mobs_in_room(&zone, &room)
                    .into_iter()
                    .find_map(|mob| {
                        let data = world.get_mob_data(&mob.mob_key)?;
                        (mob.name == line
                            || data.name == line
                            || data.desc1 == line
                            || data.reaction_names.iter().any(|alias| alias == line))
                            .then(|| (mob.clone(), data.clone()))
                    })
            });
            if let Some((mob, data)) = mob_target {
                let Some((mob_zone, filename)) = mob.mob_key.split_once(':') else {
                    return result("cannot_save", String::new());
                };
                let path = format!("data/mob/{mob_zone}/{filename}.json");
                let Ok(text) = std::fs::read_to_string(&path) else {
                    return result("open_failed", String::new());
                };
                let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&text) else {
                    return result("open_failed", String::new());
                };
                let Some(info) = root.get_mut("몹정보").and_then(|value| value.as_object_mut())
                else {
                    return result("open_failed", String::new());
                };
                for (key, value) in &data.attributes {
                    info.insert(key.clone(), value.clone());
                }
                info.insert("이름".into(), serde_json::Value::String(mob.name));
                for (key, value) in [
                    ("체력", mob.hp),
                    ("최고체력", mob.max_hp),
                    ("내공", mob.mp),
                    ("최고내공", mob.max_mp),
                    ("은전", mob.gold),
                    ("레벨", mob.level),
                    ("힘", mob.strength),
                    ("맷집", mob.arm),
                    ("민첩성", mob.agility),
                ] {
                    info.insert(key.into(), serde_json::Value::Number(value.into()));
                }
                for (key, value) in mob.runtime_attrs {
                    let value = match value {
                        Value::Int(value) => serde_json::Value::Number(value.into()),
                        Value::Float(value) => serde_json::Number::from_f64(value)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null),
                        Value::String(value) => serde_json::Value::String(value),
                    };
                    info.insert(key, value);
                }
                if std::fs::write(&path, serde_json::to_string_pretty(&root).unwrap_or(text))
                    .is_err()
                {
                    return result("open_failed", String::new());
                }
                return result("ok", path);
            }

            let matches = |object: &Object| {
                object.getName() == line
                    || reaction_names(&object.getString("반응이름"))
                        .iter()
                        .any(|alias| alias == line)
            };
            let room_items = get_world_state()
                .read()
                .ok()
                .map(|world| world.get_room_objs(&zone, &room).to_vec())
                .unwrap_or_default();
            let item = room_items
                .into_iter()
                .find(|item| item.lock().is_ok_and(|object| matches(&object)))
                .or_else(|| {
                    body.object
                        .objs
                        .iter()
                        .find(|item| item.lock().is_ok_and(|object| matches(&object)))
                        .cloned()
                });
            let Some(item) = item else {
                return result("missing", String::new());
            };
            let Ok(object) = item.lock() else {
                return result("cannot_save", String::new());
            };
            let key = object.getString("인덱스");
            if key.is_empty() || key.contains('/') || key.contains('\\') {
                return result("cannot_save", String::new());
            }
            let path = format!("data/item/{key}.json");
            let out = serde_json::json!({
                "아이템정보": inventory_compat::item_attributes_to_json(&object)
            });
            if std::fs::write(&path, serde_json::to_string_pretty(&out).unwrap_or_default()).is_err()
            {
                return result("open_failed", String::new());
            }
            result("ok", path)
        },
    );
    let body_ptr_install = body_ptr;
    engine.register_fn(
        "install_item",
        move |_ob: &mut rhai::Map, item_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_install };
            let Some((zone, room)) = current_body_position(body) else {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            };
            let Some((pos, item)) = body.object.objs.iter().enumerate().find_map(|(i, arc)| {
                let obj = arc.lock().ok()?;
                (obj.getName() == item_name || obj.getString("반응이름").contains(item_name))
                    .then(|| (i, arc.clone()))
            }) else {
                return "☞ 그런 아이템이 소지품에 없어요.".into();
            };
            let Ok(source) = item.lock() else {
                return "☞ 설치할 수 있는 것이 아닙니다. ^^".into();
            };
            if source.getString("종류") != "설치아이템" {
                return "☞ 설치할 수 있는 것이 아닙니다. ^^".into();
            }
            let name = source.getName();
            let path = format!("data/map/{zone}/{room}.json");
            let Ok(text) = std::fs::read_to_string(&path) else {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            };
            let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&text) else {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            };
            let Some(info) = root.get_mut("맵정보").and_then(|v| v.as_object_mut()) else {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            };
            let owner = info
                .get("주인")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let guild_owner = info
                .get("방파주인")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !owner.is_empty() && owner != body.get_name() {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            }
            if owner.is_empty() && guild_owner != body.get_string("소속") {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            }
            let installed = info
                .entry("설치리스트")
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
            let Some(list) = installed.as_array_mut() else {
                return "☞ 이미 설치가 되어 있습니다. ^^".into();
            };
            if list.iter().any(|v| v.as_str() == Some(&name)) {
                return "☞ 이미 설치가 되어 있습니다. ^^".into();
            }
            list.push(serde_json::Value::String(name.clone()));
            let owner_name = if owner.is_empty() {
                body.get_string("소속")
            } else {
                body.get_name()
            };
            #[allow(dropping_references)]
            drop(info);
            if std::fs::write(&path, serde_json::to_string_pretty(&root).unwrap_or(text)).is_err() {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            }
            let mut boxed = Object::new();
            for (k, v) in &source.attr {
                boxed.attr.insert(k.clone(), v.clone());
            }
            boxed.set("주인", owner_name.clone());
            drop(source);
            if !box_commands::prepare_installed_box(&mut boxed, &owner_name, &name) {
                return "☞ 이곳에 설치할 허가권이 없습니다.".into();
            }
            box_commands::register_installed_box(
                &zone,
                &room,
                std::sync::Arc::new(std::sync::Mutex::new(boxed)),
            );
            body.object.objs.remove(pos);
            let save_path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &save_path);
            String::new()
        },
    );

    // drop_item(ob, item_name, count) - 아이템을 바닥에 버리기
    // 성공 시 빈 문자열 "", 실패 시 오류 메시지 반환
    let body_ptr_di = body_ptr;
    engine.register_fn(
        "drop_item",
        move |_ob: &mut rhai::Map, item_name: &str, count: i64| -> String {
            if item_name.is_empty() {
                return "아이템 이름을 입력해주세요.".to_string();
            }
            let body = unsafe { &mut *body_ptr_di };
            let count = count.clamp(1, 100) as usize;
            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return "플레이어 위치를 찾을 수 없습니다.".to_string(),
            };

            // 스택 아이템 처리
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    let drop_cnt = (count as i64).min(have).max(0);
                    if drop_cnt <= 0 {
                        return format!("{}을(를) 가지고 있지 않습니다.", item_name);
                    }
                    let should_remove = {
                        let r = body.object.inv_stack.get_mut(key).unwrap();
                        *r -= drop_cnt;
                        *r <= 0
                    };
                    if should_remove {
                        body.object.inv_stack.remove(key);
                    }
                    let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                    *room_stack.entry(key.clone()).or_insert(0) += drop_cnt;
                    drop(w);
                    let path = format!("data/user/{}.json", body.get_name());
                    let _ = save_body_to_json(body, &path);
                    return String::new();
                }
            }

            // 비스택 아이템 처리
            let mut dropped = 0usize;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let ok = o.getName() == item_name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(item_name));
                    if !ok || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함") {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "버리지못함") {
                        drop(o);
                        continue;
                    }
                    drop(o);
                    to_remove.push(obj.clone());
                    dropped += 1;
                    if dropped >= count {
                        break;
                    }
                }
            }

            if dropped == 0 {
                return format!("{}을(를) 가지고 있지 않습니다.", item_name);
            }

            {
                let room_objs = w.get_room_objs_mut(&zone, &room);
                for arc in &to_remove {
                    body.object.remove(arc);
                    room_objs.push(arc.clone());
                }
            }
            for arc in &to_remove {
                w.record_floor_item(&zone, &room, arc);
            }
            drop(w);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            String::new()
        },
    );

    // pick_up_item(ob, item_name, count) - 바닥에서 아이템 줍기
    // 성공 시 빈 문자열 "", 실패 시 오류 메시지 반환
    // 관리자(등급>=1000)는 무게/수량 제한 없음
    const MAX_ITEMS_PICKUP: usize = 50;
    let body_ptr_pui = body_ptr;
    engine.register_fn(
        "pick_up_item",
        move |_ob: &mut rhai::Map, item_name: &str, count: i64| -> String {
            if item_name.is_empty() {
                return "아이템 이름을 입력해주세요.".to_string();
            }
            let body = unsafe { &mut *body_ptr_pui };
            let admin_level = body.get_int("관리자등급");
            let is_admin = admin_level >= 1000;
            let count = count.clamp(1, 100) as usize;
            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };
            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return "플레이어 위치를 찾을 수 없습니다.".to_string(),
            };

            let mut taken = 0usize;

            // 스택 아이템 처리
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                    let have = *room_stack.get(key).unwrap_or(&0);
                    let take_cnt = (count as i64).min(have).max(0) as usize;
                    if take_cnt > 0 {
                        // 관리자가 아니면 무게/수량 체크
                        if !is_admin {
                            // get_item_info returns (name, rn, price, weight)
                            let item_weight = get_item_info(key).map(|(_, _, _, w)| w).unwrap_or(0);
                            let total_weight = item_weight * take_cnt as i64;
                            if body.get_item_weight() + total_weight > body.get_str() * 10 {
                                return "무거워서 더 이상 들 수 없습니다.".to_string();
                            }
                            if body.get_item_count() + take_cnt > MAX_ITEMS_PICKUP {
                                return "소지품이 가득 찼습니다.".to_string();
                            }
                        }
                        let should_remove = {
                            let r = room_stack.get_mut(key).unwrap();
                            *r -= take_cnt as i64;
                            *r <= 0
                        };
                        if should_remove {
                            room_stack.remove(key);
                        }
                        *body.object.inv_stack.entry(key.clone()).or_insert(0) += take_cnt as i64;
                        taken += take_cnt;
                    }
                }
            }

            // 바닥 Object에서 가져오기 (비스택 또는 예전 드랍)
            let room_list = w.get_room_objs_mut(&zone, &room);
            let mut i = 0;
            while i < room_list.len() && taken < count {
                let (matches, item_weight) = {
                    let o = room_list[i].lock().unwrap();
                    let m = o.getName() == item_name
                        || (!o.getString("반응이름").is_empty()
                            && o.getString("반응이름").contains(item_name));
                    (m, o.getInt("무게"))
                };
                if matches {
                    // 관리자가 아니면 무게/수량 체크
                    if !is_admin {
                        if body.get_item_weight() + item_weight > body.get_str() * 10 {
                            if taken == 0 {
                                return "무거워서 더 이상 들 수 없습니다.".to_string();
                            }
                            break;
                        }
                        if body.get_item_count() + 1 > MAX_ITEMS_PICKUP {
                            if taken == 0 {
                                return "소지품이 가득 찼습니다.".to_string();
                            }
                            break;
                        }
                    }
                    let arc = room_list.remove(i);
                    body.object.objs.insert(0, arc);
                    taken += 1;
                } else {
                    i += 1;
                }
            }

            if taken == 0 {
                return format!("여기에는 {}이(가) 없습니다.", item_name);
            }

            drop(w);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            String::new()
        },
    );

    // move_item_to_room(ob, item_name, room) - 특정 방으로 아이템 이동
    // room 형식: "zone:room_id" (예: "낙양성:1")
    // 성공 시 true 반환
    let body_ptr_mitr = body_ptr;
    engine.register_fn(
        "move_item_to_room",
        move |_ob: &mut rhai::Map, item_name: &str, room: &str| -> bool {
            if item_name.is_empty() || room.is_empty() {
                return false;
            }
            let body = unsafe { &mut *body_ptr_mitr };

            // room 파싱: "zone:room_id"
            let parts: Vec<&str> = room.split(':').collect();
            if parts.len() != 2 {
                return false;
            }
            let target_zone = parts[0];
            let target_room = parts[1];

            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return false,
            };

            let mut moved = false;

            // 스택 아이템 처리 (전체 수량 이동)
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    if let Some(&have) = body.object.inv_stack.get(key) {
                        if have > 0 {
                            body.object.inv_stack.remove(key);
                            let target_stack = w.get_room_objs_stack_mut(target_zone, target_room);
                            *target_stack.entry(key.clone()).or_insert(0) += have;
                            moved = true;
                        }
                    }
                }
            }

            // 비스택 아이템 처리 (첫 번째 매칭 아이템만 이동)
            if !moved {
                let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
                for obj in &body.object.objs {
                    if let Ok(o) = obj.lock() {
                        let ok = o.getName() == item_name
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(item_name));
                        if ok {
                            drop(o);
                            to_remove.push(obj.clone());
                            break; // 첫 번째 아이템만 이동
                        }
                    }
                }

                if !to_remove.is_empty() {
                    {
                        let target_room_objs = w.get_room_objs_mut(target_zone, target_room);
                        for arc in &to_remove {
                            body.object.remove(arc);
                            target_room_objs.push(arc.clone());
                        }
                    }
                    for arc in &to_remove {
                        w.record_floor_item(target_zone, target_room, arc);
                    }
                    moved = true;
                }
            }

            if moved {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }

            moved
        },
    );

    // get_obj_attr(ob, obj_name, attr) - 오브젝트 속성 가져오기
    // 속성 값 반환, 없으면 UNIT 반환
    let body_ptr_goa = body_ptr;
    engine.register_fn(
        "get_obj_attr",
        move |_ob: &mut rhai::Map, obj_name: &str, attr: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_goa };

            // 인벤토리에서 검색
            for obj_arc in &body.object.objs {
                if let Ok(obj) = obj_arc.lock() {
                    let matches = obj.getName() == obj_name
                        || (!obj.getString("반응이름").is_empty()
                            && obj.getString("반응이름").contains(obj_name));
                    if matches {
                        let value = obj.get(attr);
                        drop(obj);
                        // Value 타입을 Dynamic으로 변환
                        match value {
                            crate::object::Value::Int(n) => return Dynamic::from_int(n),
                            crate::object::Value::String(s) => return Dynamic::from(s),
                            crate::object::Value::Float(f) => return Dynamic::from(f),
                        }
                    }
                }
            }

            // 현재 방의 바닥에서 검색
            if let Ok(w) = get_world_state().read() {
                if let Some(pos) = w.get_player_position(body.get_name().as_str()) {
                    let room_objs = w.get_room_objs(&pos.zone, &pos.room);
                    for obj_arc in room_objs {
                        if let Ok(obj) = obj_arc.lock() {
                            let matches = obj.getName() == obj_name
                                || (!obj.getString("반응이름").is_empty()
                                    && obj.getString("반응이름").contains(obj_name));
                            if matches {
                                let value = obj.get(attr);
                                drop(obj);
                                match value {
                                    crate::object::Value::Int(n) => return Dynamic::from_int(n),
                                    crate::object::Value::String(s) => return Dynamic::from(s),
                                    crate::object::Value::Float(f) => return Dynamic::from(f),
                                }
                            }
                        }
                    }
                }
            }

            Dynamic::UNIT
        },
    );

    let body_ptr_room_player_attr = body_ptr;
    engine.register_fn(
        "get_room_player_attr",
        move |_ob: &mut rhai::Map, target: &str, key: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_room_player_attr };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|w| w.get_player_position(&body.get_name()).cloned())
            else {
                return Dynamic::UNIT;
            };
            if get_world_state()
                .read()
                .ok()
                .and_then(|w| w.get_player_position(target).cloned())
                .is_none_or(|p| p.zone != pos.zone || p.room != pos.room)
            {
                return Dynamic::UNIT;
            }
            room_view_player_snapshots(&pos.zone, &pos.room)
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .find(|player| {
                    player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .is_some_and(|name| name == target)
                })
                .and_then(|player| player.get(key).cloned())
                .unwrap_or(Dynamic::UNIT)
        },
    );
    let body_ptr_set_room_player_attr = body_ptr;
    engine.register_fn(
        "set_room_player_attr",
        move |_ob: &mut rhai::Map, target: &str, key: &str, value: &str| -> bool {
            let body = unsafe { &*body_ptr_set_room_player_attr };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|w| w.get_player_position(&body.get_name()).cloned())
            else {
                return false;
            };
            if get_world_state()
                .read()
                .ok()
                .and_then(|w| w.get_player_position(target).cloned())
                .is_none_or(|p| p.zone != pos.zone || p.room != pos.room)
            {
                return false;
            }
            let path = format!("data/user/{}.json", target);
            let Ok(raw) = std::fs::read_to_string(&path) else {
                return false;
            };
            let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                return false;
            };
            let Some(attrs) = json
                .get_mut("사용자오브젝트")
                .and_then(|v| v.get_mut("attr"))
                .and_then(|v| v.as_object_mut())
            else {
                return false;
            };
            attrs.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
            std::fs::write(
                path,
                serde_json::to_string_pretty(&json).unwrap_or_default(),
            )
            .is_ok()
        },
    );

    // get_obj_attrs(ob, target): 관리자 속성 조회용 데이터. 출력 형식은 Rhai가 담당한다.
    let body_ptr_goas = body_ptr;
    engine.register_fn(
        "get_obj_attrs",
        move |_ob: &mut rhai::Map, target: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_goas };
            let name = body.get_name();
            let mut attrs: Vec<(String, String)> = Vec::new();
            let mut found = false;
            let json_text = |value: &serde_json::Value| -> String {
                match value {
                    serde_json::Value::String(value) => value.clone(),
                    serde_json::Value::Array(values) => values
                        .iter()
                        .map(|value| {
                            value
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| value.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join("\r\n"),
                    serde_json::Value::Null => String::new(),
                    value => value.to_string(),
                }
            };
            if target.is_empty() || target == "방" {
                if let Ok(w) = get_world_state().read() {
                    if let Some(pos) = w.get_player_position(&name) {
                        let path = format!("data/map/{}/{}/", pos.zone, pos.room);
                        let path = path.trim_end_matches('/').to_string() + ".json";
                        if let Ok(raw) = std::fs::read_to_string(path) {
                            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&raw) {
                                if let Some(info) = root.get("맵정보").and_then(|v| v.as_object()) {
                                    found = true;
                                    attrs.extend(info.iter().map(|(key, value)| {
                                        (key.clone(), json_text(value))
                                    }));
                                }
                            }
                        }
                        let room_key = format!("{}:{}", pos.zone, pos.room);
                        if let Some(room) = w.room_attrs.get(&room_key) {
                            found = true;
                            for (key, value) in room {
                                if let Some(existing) = attrs.iter_mut().find(|(name, _)| name == key)
                                {
                                    existing.1 = value.clone();
                                } else {
                                    attrs.push((key.clone(), value.clone()));
                                }
                            }
                        }
                    }
                }
            } else if target == "나" || target == name {
                found = true;
                attrs.extend(body.object.attr.iter().map(|(k, v)| {
                    let value = match v {
                        crate::object::Value::Int(n) => n.to_string(),
                        crate::object::Value::Float(n) => n.to_string(),
                        crate::object::Value::String(s) => s.clone(),
                    };
                    (k.clone(), value)
                }));
            } else {
                let position = current_body_position(body);
                if let Some((zone, room)) = position.as_ref() {
                    let floor = get_world_state()
                        .read()
                        .ok()
                        .map(|world| world.get_room_objs(zone, room).to_vec())
                        .unwrap_or_default();
                    for arc in floor {
                        let Ok(obj) = arc.lock() else { continue };
                        if obj.getName() == target
                            || reaction_names(&obj.getString("반응이름"))
                                .iter()
                                .any(|alias| alias == target)
                        {
                            found = true;
                            attrs.extend(obj.attr.iter().map(|(key, value)| {
                                let value = match value {
                                    Value::Int(value) => value.to_string(),
                                    Value::Float(value) => value.to_string(),
                                    Value::String(value) => value.clone(),
                                };
                                (key.clone(), value)
                            }));
                            break;
                        }
                    }
                    if !found {
                        if let Ok(world) = get_world_state().read() {
                            if let Some((mob, data)) = world
                                .mob_cache
                                .get_all_mobs_in_room(zone, room)
                                .into_iter()
                                .find_map(|mob| {
                                    let data = world.get_mob_data(&mob.mob_key)?;
                                    (mob.name == target
                                        || data.name == target
                                        || data.reaction_names.iter().any(|alias| alias == target))
                                        .then_some((mob, data))
                                })
                            {
                                found = true;
                                attrs.extend(data.attributes.iter().map(|(key, value)| {
                                    (key.clone(), json_text(value))
                                }));
                                for (key, value) in [
                                    ("이름", mob.name.clone()),
                                    ("체력", mob.hp.to_string()),
                                    ("최고체력", mob.max_hp.to_string()),
                                    ("내공", mob.mp.to_string()),
                                    ("은전", mob.gold.to_string()),
                                ] {
                                    if let Some(existing) =
                                        attrs.iter_mut().find(|(name, _)| name == key)
                                    {
                                        existing.1 = value;
                                    } else {
                                        attrs.push((key.to_string(), value));
                                    }
                                }
                            }
                        }
                    }
                    if !found {
                        let player_name = get_world_state().read().ok().and_then(|world| {
                            world
                                .get_players_in_room(zone, room)
                                .iter()
                                .rev()
                                .find(|player| {
                                    player.as_str() == target || player.starts_with(target)
                                })
                                .cloned()
                        });
                        if let Some(player_name) = player_name {
                            let path = format!("data/user/{player_name}.json");
                            if let Ok(raw) = std::fs::read_to_string(path) {
                                if let Ok(root) = serde_json::from_str::<serde_json::Value>(&raw) {
                                    if let Some(values) = root
                                        .get("사용자오브젝트")
                                        .and_then(|value| value.get("attr"))
                                        .and_then(|value| value.as_object())
                                    {
                                        found = true;
                                        attrs.extend(values.iter().map(|(key, value)| {
                                            (key.clone(), json_text(value))
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
                // Python falls back to ob.findObjName after the room lookup.
                if !found {
                for arc in &body.object.objs {
                    if let Ok(obj) = arc.lock() {
                        if obj.getName() == target
                            || reaction_names(&obj.getString("반응이름"))
                                .iter()
                                .any(|alias| alias == target)
                        {
                            found = true;
                            attrs.extend(obj.attr.iter().map(|(k, v)| {
                                let value = match v {
                                    crate::object::Value::Int(n) => n.to_string(),
                                    crate::object::Value::Float(n) => n.to_string(),
                                    crate::object::Value::String(s) => s.clone(),
                                };
                                (k.clone(), value)
                            }));
                            break;
                        }
                    }
                }
                }
            }
            attrs.sort_by(|a, b| a.0.cmp(&b.0));
            let attrs = attrs
                .into_iter()
                .map(|(key, value)| {
                    let mut item = rhai::Map::new();
                    item.insert("key".into(), Dynamic::from(key));
                    item.insert("value".into(), Dynamic::from(value));
                    Dynamic::from(item)
                })
                .collect::<rhai::Array>();
            let mut result = rhai::Map::new();
            result.insert("found".into(), Dynamic::from(found));
            result.insert("attrs".into(), Dynamic::from(attrs));
            Dynamic::from(result)
        },
    );

    // find_object_rooms(ob, name): 관리자 찾아라용. 로드된 Room.Zones 순서를 유지한다.
    engine.register_fn(
        "find_object_rooms",
        move |_ob: &mut rhai::Map, wanted: &str| -> rhai::Array {
            let mut result = rhai::Array::new();
            if wanted.is_empty() {
                return result;
            }
            let Ok(world) = get_world_state().read() else {
                return result;
            };
            for (zone, room_id) in world.room_cache.loaded_rooms_in_python_zone_order() {
                let Some(room_arc) = world.room_cache.get_room_cached(&zone, &room_id) else {
                    continue;
                };
                let Ok(room) = room_arc.read() else { continue };
                let room_objects = world.get_room_objs(&zone, &room_id).to_vec();
                let mut found = room.items.iter().any(|name| name == wanted)
                    || room.npcs.iter().any(|name| name == wanted);
                if !found {
                    for arc in room_objects {
                        if let Ok(obj) = arc.lock() {
                            if obj.getName() == wanted || obj.getString("반응이름").contains(wanted)
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if found {
                    result.push(Dynamic::from(room.display_name.clone()));
                }
            }
            result
        },
    );

    // destroy_item(ob, item_name, count) - 아이템 완전히 파괴
    // 파괴된 아이템 수 반환
    let body_ptr_dest = body_ptr;
    engine.register_fn(
        "destroy_item",
        move |_ob: &mut rhai::Map, item_name: &str, count: i64| -> i64 {
            if item_name.is_empty() {
                return 0;
            }
            let body = unsafe { &mut *body_ptr_dest };
            let count = count.clamp(1, 100) as usize;

            let mut destroyed = 0i64;

            // 스택 아이템 파괴
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    let destroy_cnt = (count as i64).min(have).max(0);
                    if destroy_cnt > 0 {
                        let should_remove = {
                            let r = body.object.inv_stack.get_mut(key).unwrap();
                            *r -= destroy_cnt;
                            *r <= 0
                        };
                        if should_remove {
                            body.object.inv_stack.remove(key);
                        }
                        destroyed += destroy_cnt;
                    }
                }
            }

            // 비스택 아이템 파괴
            if destroyed < count as i64 {
                let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
                for obj in &body.object.objs {
                    if let Ok(o) = obj.lock() {
                        let ok = o.getName() == item_name
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(item_name));
                        if !ok || o.getBool("inUse") {
                            continue;
                        }
                        drop(o);
                        to_remove.push(obj.clone());
                        destroyed += 1;
                        if destroyed >= count as i64 {
                            break;
                        }
                    }
                }

                for arc in to_remove {
                    body.object.remove(&arc);
                }
            }

            if destroyed > 0 {
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }

            destroyed
        },
    );

    // give_item_to_mob(ob, mob_name, item_name) - 몹에게 아이템 주기
    // 성공 시 true 반환
    let body_ptr_gitm = body_ptr;
    engine.register_fn(
        "give_item_to_mob",
        move |_ob: &mut rhai::Map, mob_name: &str, item_name: &str| -> bool {
            if item_name.is_empty() {
                return false;
            }
            let body = unsafe { &mut *body_ptr_gitm };

            let mut w = match get_world_state().write() {
                Ok(w) => w,
                Err(_) => return false,
            };

            let (zone, room) = match w.get_player_position(body.get_name().as_str()) {
                Some(p) => (p.zone.clone(), p.room.clone()),
                None => return false,
            };

            // 방에 있는 몹 찾기
            let mob_key = w
                .mob_cache
                .get_mobs_in_room(&zone, &room)
                .iter()
                .find_map(|m| {
                    w.get_mob_data(&m.mob_key).and_then(|data| {
                        (m.name == mob_name
                            || data.name == mob_name
                            || data.desc1 == mob_name
                            || data.reaction_names.iter().any(|alias| alias == mob_name))
                        .then(|| m.mob_key.clone())
                    })
                });

            let Some(mob_key) = mob_key else {
                return false;
            };

            let mut given = false;

            // 스택 아이템 처리 (1개만 주기)
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                    if have > 0 {
                        let should_remove = {
                            let r = body.object.inv_stack.get_mut(key).unwrap();
                            *r -= 1;
                            *r <= 0
                        };
                        if should_remove {
                            body.object.inv_stack.remove(key);
                        }
                        if let Some((item, _)) = object_from_item_json(key) {
                            if let Some(mob) = w
                                .mob_cache
                                .get_all_mobs_in_room_mut(&zone, &room)
                                .and_then(|mobs| mobs.iter_mut().find(|m| m.mob_key == mob_key))
                            {
                                mob.inventory.push(item);
                            }
                        }
                        given = true;
                    }
                }
            }

            // 비스택 아이템 처리
            if !given {
                let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
                for obj in &body.object.objs {
                    if let Ok(o) = obj.lock() {
                        let ok = o.getName() == item_name
                            || (!o.getString("반응이름").is_empty()
                                && o.getString("반응이름").contains(item_name));
                        if ok && !o.getBool("inUse") {
                            drop(o);
                            to_remove.push(obj.clone());
                            break;
                        }
                    }
                }
                for arc in to_remove {
                    body.object.remove(&arc);
                    if let Some(mob) = w
                        .mob_cache
                        .get_all_mobs_in_room_mut(&zone, &room)
                        .and_then(|mobs| mobs.iter_mut().find(|m| m.mob_key == mob_key))
                    {
                        mob.inventory.push(arc);
                    }
                    given = true;
                }
            }

            if given {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }

            given
        },
    );

    // ============================================================
    // Admin command efun (관리자 명령)
    // ============================================================

    // summon_player(admin_ob, target_name) - 대상 플레이어를 관리자의 현재 위치로 소환
    // Returns "" on success, error string on failure
    // Python 사용자몹소환과 동일하게 관리자 1000 이상
    let body_ptr_summon = body_ptr;
    engine.register_fn(
        "summon_player",
        move |admin_ob: &mut rhai::Map, target_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 1000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &mut *body_ptr_summon };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            // 관리자의 현재 위치 확인
            let admin_pos = match w.get_player_position(&admin_name).cloned() {
                Some(p) => p,
                None => return "관리자의 위치를 찾을 수 없습니다.".to_string(),
            };

            // 대상 플레이어의 현재 위치 확인
            let target_pos = match w.get_player_position(target_name) {
                Some(p) => p.clone(),
                None => return "대상을 찾을 수 없습니다.".to_string(),
            };

            // 이미 같은 위치에 있는지 확인
            if target_pos.zone == admin_pos.zone && target_pos.room == admin_pos.room {
                return "이미 같은 위치에 있습니다.".to_string();
            }

            let mut requests: Vec<(String, String, String)> = body
                .temp()
                .get(SUMMON_PLAYER_REQUEST)
                .and_then(Value::as_str)
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            requests.push((
                target_name.to_string(),
                admin_pos.zone,
                admin_pos.room,
            ));
            if let Ok(request) = serde_json::to_string(&requests) {
                body.temp_mut().insert(
                    SUMMON_PLAYER_REQUEST.to_string(),
                    Value::String(request),
                );
                String::new()
            } else {
                "공간이동에 실패하였습니다.".to_string()
            }
        },
    );

    // goto_player(admin_ob, target_name) - 관리자가 대상 플레이어의 위치로 이동
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let body_ptr_goto = body_ptr;
    engine.register_fn(
        "goto_player",
        move |admin_ob: &mut rhai::Map, target_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 1000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_goto };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 자기 자신에게 이동 불가
            if target_name == admin_name {
                return "자기 자신에게 이동할 수 없습니다.".to_string();
            }

            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            // 대상 플레이어의 현재 위치 확인
            let target_pos = match w.get_player_position(target_name).cloned() {
                Some(p) => p,
                None => return "대상을 찾을 수 없습니다.".to_string(),
            };

            // 관리자의 현재 위치 확인
            let admin_pos = match w.get_player_position(&admin_name) {
                Some(p) => p.clone(),
                None => return "관리자의 위치를 찾을 수 없습니다.".to_string(),
            };

            // 이미 같은 위치에 있는지 확인
            if admin_pos.zone == target_pos.zone && admin_pos.room == target_pos.room {
                return "이미 같은 위치에 있습니다.".to_string();
            }

            // 관리자를 대상의 위치로 이동
            w.set_player_position(&admin_name, target_pos.clone());
            w.spawn_mobs_for_room(&target_pos.zone, &target_pos.room);

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // kick_player(admin_ob, target_name) - 플레이어 강제 로그아웃
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let spec_kick = spec.clone();
    let body_ptr_kick = body_ptr;
    engine.register_fn(
        "kick_player",
        move |admin_ob: &mut rhai::Map, target_name: &str| -> String {
            use crate::command::handler::CommandResult;

            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_kick };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 자기 자신 킥 불가
            if target_name == admin_name {
                return "자기 자신을 킥할 수 없습니다.".to_string();
            }

            // 대상이 접속 중인지 확인
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            if w.get_player_position(target_name).is_none() {
                return "대상을 찾을 수 없습니다.".to_string();
            }

            // CommandResult::Kick 설정 (핸들러에서 실제 처리)
            if let Ok(mut s) = spec_kick.lock() {
                *s = Some(CommandResult::Kick {
                    target_name: target_name.to_string(),
                    reason: "관리자에 의해 강제 로그아웃되었습니다.".to_string(),
                });
            }

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // disconnect_player_for_cleanup: Python 정리(level=1000)의 전용 종료 요청.
    let spec_cleanup = spec.clone();
    engine.register_fn(
        "disconnect_player_for_cleanup",
        move |_ob: &mut rhai::Map, target_name: &str| -> bool {
            if target_name.trim().is_empty() {
                return false;
            }
            if let Ok(mut s) = spec_cleanup.lock() {
                *s = Some(CommandResult::Kick {
                    target_name: target_name.to_string(),
                    reason: "정리 명령".to_string(),
                });
                true
            } else {
                false
            }
        },
    );

    // ban_player(admin_ob, target_name, duration) - 플레이어 접속 차단
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let spec_ban = spec.clone();
    let body_ptr_ban = body_ptr;
    engine.register_fn(
        "ban_player",
        move |admin_ob: &mut rhai::Map, target_name: &str, duration: i64| -> String {
            use crate::command::handler::CommandResult;

            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_ban };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 기간 체크
            if duration <= 0 {
                return "차단 기간은 0보다 커야 합니다.".to_string();
            }

            // 자기 자신 밴 불가
            if target_name == admin_name {
                return "자기 자신을 밴할 수 없습니다.".to_string();
            }

            // CommandResult::Ban 설정 (핸들러에서 실제 처리)
            if let Ok(mut s) = spec_ban.lock() {
                *s = Some(CommandResult::Ban {
                    target_name: target_name.to_string(),
                    duration,
                    reason: format!("관리자에 의해 {}초간 접속이 차단되었습니다.", duration),
                });
            }

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // set_player_level(admin_ob, target_name, level) - 플레이어 레벨 설정
    // Returns true on success
    // Admin level 2000 required
    let body_ptr_set_lvl = body_ptr;
    engine.register_fn(
        "set_player_level",
        move |admin_ob: &mut rhai::Map, target_name: &str, level: i64| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return false;
            }

            // 레벨 범위 체크
            if !(1..=1000).contains(&level) {
                return false;
            }

            let ok = update_user_attr_int(target_name, "레벨", level);
            if ok {
                queue_online_user_attr(
                    unsafe { &mut *body_ptr_set_lvl },
                    target_name,
                    "레벨",
                    level,
                );
            }
            ok
        },
    );

    // set_player_money(admin_ob, target_name, amount) - 플레이어 돈 설정
    // Returns true on success
    // Admin level 2000 required
    let body_ptr_set_money = body_ptr;
    engine.register_fn(
        "set_player_money",
        move |admin_ob: &mut rhai::Map, target_name: &str, amount: i64| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return false;
            }

            // 금액 범위 체크
            if !(0..=1_000_000_000).contains(&amount) {
                return false;
            }

            let ok = update_user_attr_int(target_name, "은전", amount);
            if ok {
                queue_online_user_attr(
                    unsafe { &mut *body_ptr_set_money },
                    target_name,
                    "은전",
                    amount,
                );
            }
            ok
        },
    );

    // set_player_hp(admin_ob, target_name, hp) - 플레이어 HP 설정
    // Returns true on success
    // Admin level 2000 required
    let body_ptr_set_hp_player = body_ptr;
    engine.register_fn(
        "set_player_hp",
        move |admin_ob: &mut rhai::Map, target_name: &str, hp: i64| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return false;
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return false;
            }

            // HP 범위 체크
            if !(0..=1_000_000).contains(&hp) {
                return false;
            }

            let ok = update_user_attr_int(target_name, "체력", hp);
            if ok {
                queue_online_user_attr(
                    unsafe { &mut *body_ptr_set_hp_player },
                    target_name,
                    "체력",
                    hp,
                );
            }
            ok
        },
    );

    // Python 사용자몹소환: persisted Player를 socket-less ACTIVE Player로
    // 복원해 channel.players와 현재 Room.objs에 넣는 상태 동작.
    let body_ptr_create_mob = body_ptr;
    engine.register_fn(
        "create_user_mob",
        move |admin_ob: &mut rhai::Map, mob_name: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 1000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            let body = unsafe { &*body_ptr_create_mob };
            let admin_name = body.get_name();

            if mob_name.trim().is_empty() {
                return "존재하지않는 사용자입니다.".to_string();
            }

            // 관리자 현재 위치 확인
            let (zone, room) = {
                let w = match get_world_state().read() {
                    Ok(g) => g,
                    Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
                };
                let pos = match w.get_player_position(&admin_name) {
                    Some(p) => p.clone(),
                    None => return "위치 정보를 찾을 수 없습니다.".to_string(),
                };
                (pos.zone, pos.room)
            };

            let mut summoned = Body::new();
            if !load_body_from_json(
                &mut summoned,
                &format!("data/user/{}.json", mob_name),
            ) {
                return "존재하지않는 사용자입니다.".to_string();
            }
            summoned.act = crate::player::ActState::Stand;
            let loaded_name = summoned.get_name();
            get_world_state().write().unwrap().add_summoned_user(
                summoned,
                crate::world::PlayerPosition::new(zone, room),
            );
            loaded_name
        },
    );

    let body_ptr_remove_room_user = body_ptr;
    engine.register_fn(
        "remove_room_user_mob",
        move |_admin_ob: &mut rhai::Map, query: &str| -> bool {
            let body = unsafe { &*body_ptr_remove_room_user };
            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };
            get_world_state()
                .write()
                .map(|mut world| world.remove_summoned_user_in_room(&zone, &room, query))
                .unwrap_or(false)
        },
    );

    // remove_user_mob(admin_ob, mob_name) - 사용자 정의 몹 제거
    // Returns true on success
    // Admin level 2000 required
    let body_ptr_remove_mob = body_ptr;
    engine.register_fn(
        "remove_user_mob",
        move |admin_ob: &mut rhai::Map, mob_name: &str| -> bool {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 1000 {
                return false;
            }

            let body = unsafe { &*body_ptr_remove_mob };
            let admin_name = body.get_name();

            // 빈 몹 이름 체크
            if mob_name.trim().is_empty() {
                return false;
            }

            let _ = admin_name;
            get_world_state()
                .write()
                .map(|mut world| world.remove_summoned_user(mob_name))
                .unwrap_or(false)
        },
    );

    let body_ptr_remove_room_mob = body_ptr;
    engine.register_fn(
        "remove_room_mob",
        move |_ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &*body_ptr_remove_room_mob };
            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };
            let Ok(mut world) = get_world_state().write() else {
                return false;
            };
            let metadata = world
                .mob_cache
                .get_all_mobs_in_room(&zone, &room)
                .into_iter()
                .filter_map(|mob| {
                    world
                        .mob_cache
                        .get_mob(&mob.mob_key)
                        .cloned()
                        .map(|data| (mob.mob_key.clone(), data))
                })
                .collect::<HashMap<_, _>>();
            let selected = {
                let mobs = world
                    .mob_cache
                    .get_all_mobs_in_room(&zone, &room)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                admin_combat::python_room_mob_index(&mobs, &metadata, mob_name)
            };
            let Some(index) = selected else { return false };
            let Some(mobs) = world
                .mob_cache
                .get_all_mobs_in_room_mut(&zone, &room)
            else {
                return false;
            };
            if index >= mobs.len() {
                return false;
            }
            mobs.remove(index);
            true
        },
    );

    // warp_player(admin_ob, target_name, zone, room) - 플레이어를 특정 위치로 이동
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    let _body_ptr_warp = body_ptr;
    engine.register_fn(
        "warp_player",
        move |admin_ob: &mut rhai::Map, target_name: &str, zone: &str, room: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 빈 zone/room 체크
            if zone.trim().is_empty() || room.trim().is_empty() {
                return "위치를 입력해주세요 (zone:room).".to_string();
            }

            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return "월드 상태를 가져올 수 없습니다.".to_string(),
            };

            // 대상 플레이어의 현재 위치 확인
            let current_pos = w.get_player_position(target_name);
            if current_pos.is_none() {
                return "대상을 찾을 수 없습니다.".to_string();
            }

            let room_s = room.to_string();

            // 방 존재 확인
            if w.room_cache.get_room(zone, &room_s).is_err() {
                return "해당 위치를 찾을 수 없습니다.".to_string();
            }

            // 플레이어 이동
            w.set_player_position(
                target_name,
                PlayerPosition::new(zone.to_string(), room_s.clone()),
            );
            w.spawn_mobs_for_room(zone, &room_s);

            String::new() // 성공 시 빈 문자열 반환
        },
    );

    // admin_force_command(admin_ob, target_name, command) - 대상 플레이어에게 명령 강제 실행
    // Returns "" on success, error string on failure
    // Admin level 2000 required
    // Note: This adds the command to user_sends which will be processed as if the player typed it
    let body_ptr_force = body_ptr;
    engine.register_fn(
        "admin_force_command",
        move |admin_ob: &mut rhai::Map, target_name: &str, command: &str| -> String {
            // 관리자 권한 확인
            let admin_level = admin_ob
                .get("관리자등급")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            if admin_level < 2000 {
                return "관리자 권한이 없습니다.".to_string();
            }

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "대상 이름을 입력해주세요.".to_string();
            }

            // 빈 명령어 체크
            if command.trim().is_empty() {
                return "명령어를 입력해주세요.".to_string();
            }

            // 플레이어가 접속 중인지 확인
            let online = if let Ok(w) = get_world_state().try_read() {
                w.get_player_position(target_name).is_some()
            } else {
                return "월드 상태를 확인할 수 없습니다.".to_string();
            };

            if !online {
                return "대상 플레이어가 접속 중이 아닙니다.".to_string();
            }

            let body = unsafe { &mut *body_ptr_force };
            let mut requests: Vec<(String, String)> = body
                .temp()
                .get(FORCE_COMMAND_REQUEST)
                .and_then(Value::as_str)
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            requests.push((target_name.to_string(), command.to_string()));
            let Ok(request) = serde_json::to_string(&requests) else {
                return "명령어 큐에 추가할 수 없습니다.".to_string();
            };
            body.temp_mut().insert(
                FORCE_COMMAND_REQUEST.to_string(),
                Value::String(request),
            );
            String::new()
        },
    );

    let user_sends_delayed = user_sends.clone();
    engine.register_fn(
        "queue_self_command",
        move |ob: &mut rhai::Map, command: &str| {
            let name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            if !name.is_empty() && !command.trim().is_empty() {
                if let Ok(mut sends) = user_sends_delayed.lock() {
                    sends.push((name, command.to_string()));
                }
            }
        },
    );

    // ============================================================
    // HELPER/UTILITY FUNCTIONS (Display formatting)
    // ============================================================
    // Note: Text formatting functions (format_bar, format_money, format_number,
    // get_item_display, get_mob_display, time_to_string) are now implemented
    // in lib/format.rhai for hot-reload capability.
    //
    // However, format_item_name and format_mob_name are frequently used and
    // kept in Rust for performance.

    // format_item_name - Item name with color (frequently used, kept in Rust)
    engine.register_fn("format_item_name", |display_name: &str| -> String {
        format!("\x1b[1;37m{}\x1b[0;37m", display_name)
    });

    // format_mob_name - Mob name with color (frequently used, kept in Rust)
    engine.register_fn("format_mob_name", |display_name: &str| -> String {
        format!("\x1b[1;33m{}\x1b[0;37m", display_name)
    });

    // ============================================================
    // 호위 (Guard/Protection) 시스템 efun
    // ============================================================

    // add_guard(ob, mob_name) - 몹을 호위로 추가
    // Returns "" on success, error string on failure
    let body_ptr_add_guard = body_ptr;
    engine.register_fn(
        "add_guard",
        move |_ob: &mut rhai::Map, mob_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_add_guard };

            if mob_name.trim().is_empty() {
                return "몹 이름을 입력해주세요.".to_string();
            }

            // 몹 데이터 확인
            let mob_data = match get_mob_by_name_impl(mob_name) {
                Some(data) => data,
                None => return format!("몹 '{}'을(를) 찾을 수 없습니다.", mob_name),
            };

            let guard_name = mob_data
                .get("이름")
                .and_then(|v| v.as_str())
                .unwrap_or(mob_name);

            let max_hp = mob_data.get("체력").and_then(|v| v.as_i64()).unwrap_or(100);

            let desc = mob_data.get("설명2").and_then(|v| v.as_str()).unwrap_or("");

            // 현재 호위 목록 가져오기
            let mut guards = parse_guards_list(&body.get_string("호위_리스트"));

            // 이미 있는 호위인지 확인
            if guards.iter().any(|g| g.name == guard_name) {
                return format!("{}은(는) 이미 호위로 있습니다.", guard_name);
            }

            // 호위 추가
            guards.push(crate::script::GuardData {
                name: guard_name.to_string(),
                hp: max_hp,
                max_hp,
                description: desc.to_string(),
            });

            // 호위 목록 저장
            body.set("호위_리스트", format_guards_list(&guards));

            String::new()
        },
    );

    // remove_guard(ob, mob_name) - 호위 제거
    // Returns true on success
    let body_ptr_remove_guard = body_ptr;
    engine.register_fn(
        "remove_guard",
        move |_ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_remove_guard };

            if mob_name.trim().is_empty() {
                return false;
            }

            let mut guards = parse_guards_list(&body.get_string("호위_리스트"));
            let original_len = guards.len();

            guards.retain(|g| g.name != mob_name);

            if guards.len() < original_len {
                body.set("호위_리스트", format_guards_list(&guards));
                true
            } else {
                false
            }
        },
    );

    // get_guards(ob) - 호위 목록 가져오기
    // Returns Array of guard data (이름, 체력, max_체력, 설명)
    let body_ptr_get_guards = body_ptr;
    engine.register_fn("get_guards", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_get_guards };
        let guards = parse_guards_list(&body.get_string("호위_리스트"));

        let mut result = rhai::Array::new();
        for guard in guards {
            let mut guard_map = rhai::Map::new();
            guard_map.insert("이름".into(), Dynamic::from(guard.name.clone()));
            guard_map.insert("체力".into(), Dynamic::from(guard.hp));
            guard_map.insert("max_체력".into(), Dynamic::from(guard.max_hp));
            guard_map.insert("설명".into(), Dynamic::from(guard.description));
            result.push(Dynamic::from(guard_map));
        }
        result
    });

    // Python `호위.py` traverses the player's ordered inventory objects.
    let body_ptr_inventory_guards = body_ptr;
    engine.register_fn(
        "get_inventory_guards",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_inventory_guards };
            body.object
                .objs
                .iter()
                .filter_map(|arc| {
                    let guard = arc.lock().ok()?;
                    if guard.getString("종류") != "호위" {
                        return None;
                    }
                    let max_hp = object_from_item_json(&guard.getString("인덱스"))
                        .and_then(|(template, _)| {
                            template.lock().ok().map(|item| item.getInt("체력"))
                        })
                        .unwrap_or_else(|| guard.getInt("최고체력").max(guard.getInt("체력")));
                    let mut map = rhai::Map::new();
                    map.insert("name".into(), Dynamic::from(guard.getName()));
                    map.insert("hp".into(), Dynamic::from(guard.getInt("체력")));
                    map.insert("max_hp".into(), Dynamic::from(max_hp));
                    map.insert(
                        "description".into(),
                        Dynamic::from(guard.getString("설명2")),
                    );
                    Some(Dynamic::from(map))
                })
                .collect()
        },
    );

    // get_guard_count(ob) - 호위 수 가져오기
    // Returns count as i64
    let body_ptr_guard_count = body_ptr;
    engine.register_fn("get_guard_count", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_guard_count };
        let guards = parse_guards_list(&body.get_string("호위_리스트"));
        guards.len() as i64
    });

    // get_anger(ob) - 분노 (anger) 점수 가져오기
    let body_ptr_get_anger = body_ptr;
    engine.register_fn("get_anger", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &*body_ptr_get_anger };
        body.get_int("분노")
    });

    // set_anger(ob, value) - 분노 점수 설정
    // Returns true on success
    let body_ptr_set_anger = body_ptr;
    engine.register_fn(
        "set_anger",
        move |_ob: &mut rhai::Map, value: i64| -> bool {
            let body = unsafe { &mut *body_ptr_set_anger };
            let clamped = value.clamp(0, 10000); // 분노 값 범위 제한
            body.set("분노", clamped);
            true
        },
    );

    // guard_fight(ob) - 호위가 싸우게 하기
    // Returns true if any guard attacked
    let body_ptr_guard_fight = body_ptr;
    engine.register_fn("guard_fight", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &*body_ptr_guard_fight };
        let guards = parse_guards_list(&body.get_string("호위_리스트"));

        if guards.is_empty() {
            return false;
        }

        // Combat scheduling is owned by the Rhai combat command; this efun
        // reports whether the Python guard list contains an eligible guard.
        println!(
            "[SCRIPT] guard_fight: {} guards attacking for {}",
            guards.len(),
            body.get_name()
        );
        true
    });

    // find_guard_in_room(ob, mob_name) - 방의 몹이 플레이어의 호위인지 확인
    // Returns true if mob is player's guard
    let body_ptr_find_guard = body_ptr;
    engine.register_fn(
        "find_guard_in_room",
        move |_ob: &mut rhai::Map, mob_name: &str| -> bool {
            let body = unsafe { &*body_ptr_find_guard };
            let guards = parse_guards_list(&body.get_string("호위_리스트"));

            guards.iter().any(|g| g.name == mob_name)
        },
    );

    // ============================================================
    // SHOP/MERCHANT SYSTEM EFUNS
    // ============================================================

    // get_shop_mobs(ob) - 현재 방의 상인(상점) 몹 목록 반환
    // Returns: Array of mob names that are merchants (have items_for_sale or buy_percent > 0)
    let body_ptr_shop_mobs = body_ptr;
    engine.register_fn("get_shop_mobs", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &*body_ptr_shop_mobs };
        let name = body.get_name();
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Array::new(),
        };
        let pos = match w.get_player_position(&name) {
            Some(p) => p,
            None => return rhai::Array::new(),
        };
        let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
        let mut arr = rhai::Array::new();
        for mob in mobs {
            if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                // 상인 확인: 물건판매 있거나 물건구입 비율이 0보다 큰 경우
                if !data.items_for_sale.is_empty() || data.buy_percent > 0 {
                    arr.push(Dynamic::from(data.name.clone()));
                }
            }
        }
        arr
    });

    // get_shop_items(ob, mob_name) - 특정 상인이 판매하는 아이템 목록 반환
    // Returns: Array of {name, price, count} maps
    let body_ptr_shop_items = body_ptr;
    engine.register_fn(
        "get_shop_items",
        move |_ob: &mut rhai::Map, mob_name: &str| -> rhai::Array {
            let body = unsafe { &*body_ptr_shop_items };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return rhai::Array::new(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut arr = rhai::Array::new();
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    // 몹 이름 매칭 (정확히 일치하거나 포함)
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    // items_for_sale 목록 반환
                    for (item_key, percent) in &data.items_for_sale {
                        if let Some((iname, _, base_price, _)) = get_item_info(item_key) {
                            let p = (*percent).max(1);
                            let price = base_price * 100 / p;
                            let mut item_map = rhai::Map::new();
                            item_map.insert("name".into(), Dynamic::from(iname.clone()));
                            item_map.insert("price".into(), Dynamic::from(price));
                            item_map.insert("count".into(), Dynamic::from(1i64)); // 기본값: 1 (무제한인 경우)
                            arr.push(Dynamic::from(item_map));
                        }
                    }
                    break;
                }
            }
            arr
        },
    );

    // buy_from_shop(ob, mob_name, item_name, count) - 상인에게 아이템 구매
    // Returns: "" on success, error code on failure
    // Error codes: "no_merchant", "not_for_sale", "no_money", "inv_full", "too_heavy"
    let body_ptr_buy_shop = body_ptr;
    engine.register_fn(
        "buy_from_shop",
        move |_ob: &mut rhai::Map, mob_name: &str, item_name: &str, count: i64| -> String {
            let body = unsafe { &mut *body_ptr_buy_shop };
            let pname = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "no_merchant".to_string(),
            };
            let pos = match w.get_player_position(&pname) {
                Some(p) => p,
                None => return "no_merchant".to_string(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut item_key = String::new();
            let mut unit_price = 0i64;
            let mut weight = 0i64;
            let mut _display_name = String::new();

            // 상인 찾기 및 아이템 확인
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    if data.items_for_sale.is_empty() {
                        return "no_merchant".to_string();
                    }
                    for (key, percent) in &data.items_for_sale {
                        let Some((iname, rn, price, wg)) = get_item_info(key) else {
                            continue;
                        };
                        let ok = iname == item_name || (!rn.is_empty() && rn.contains(item_name));
                        if !ok {
                            continue;
                        }
                        let p = (*percent).max(1);
                        unit_price = price * 100 / p;
                        weight = wg;
                        _display_name = iname;
                        item_key = key.clone();
                        break;
                    }
                    break;
                }
            }

            if item_key.is_empty() {
                return "not_for_sale".to_string();
            }

            let cnt = count.clamp(1, 50);
            const MAX_ITEMS: usize = 50;
            let is_admin = body.get_int("관리자등급") >= 1000;

            // 돈 확인
            let total_cost = unit_price * cnt;
            if body.get_int("은전") < total_cost {
                return "no_money".to_string();
            }

            // 인벤토리 공간 및 무게 확인 (관리자 제외)
            if !is_admin {
                if body.get_item_count() + cnt as usize > MAX_ITEMS {
                    return "inv_full".to_string();
                }
                if body.get_item_weight() + (weight * cnt) > body.get_str() * 10 {
                    return "too_heavy".to_string();
                }
            }

            // 아이템 추가 및 돈 차감
            for _ in 0..cnt {
                if is_stackable(&item_key) {
                    *body.object.inv_stack.entry(item_key.clone()).or_insert(0) += 1;
                } else if let Some((arc, _)) = object_from_item_json(&item_key) {
                    body.object.objs.insert(0, arc);
                } else {
                    return "not_for_sale".to_string();
                }
            }
            body.set("은전", body.get_int("은전") - total_cost);

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            String::new() // 성공
        },
    );

    // sell_to_shop(ob, mob_name, item_name, count) - 상인에게 아이템 판매
    // Returns: "" on success, error code on failure
    // Error codes: "no_merchant", "no_item", "cant_sell"
    let body_ptr_sell_shop = body_ptr;
    engine.register_fn(
        "sell_to_shop",
        move |_ob: &mut rhai::Map, mob_name: &str, item_name: &str, count: i64| -> String {
            let body = unsafe { &mut *body_ptr_sell_shop };
            let pname = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "no_merchant".to_string(),
            };
            let pos = match w.get_player_position(&pname) {
                Some(p) => p,
                None => return "no_merchant".to_string(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut buy_percent = 0i64;

            // 상인 찾기 및 구입 비율 확인
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    if data.buy_percent <= 0 {
                        return "no_merchant".to_string();
                    }
                    buy_percent = data.buy_percent;
                    break;
                }
            }

            if buy_percent <= 0 {
                return "no_merchant".to_string();
            }

            let count = count.clamp(1, 100) as usize;
            let _sold = 0usize;
            let mut total = 0i64;

            // 스택 아이템 먼저 확인
            if let Some(ref key) = find_item_key_by_name(item_name) {
                if is_stackable(key) {
                    if let Some((iname, _, base_price, _)) = get_item_info(key) {
                        if iname == item_name {
                            let have = *body.object.inv_stack.get(key).unwrap_or(&0);
                            let sell_cnt = (count as i64).clamp(0, have);
                            if sell_cnt > 0 {
                                let unit = (base_price * buy_percent) / 100;
                                total = unit * sell_cnt;
                                let should_remove = {
                                    let r = body.object.inv_stack.get_mut(key).unwrap();
                                    *r -= sell_cnt;
                                    *r <= 0
                                };
                                if should_remove {
                                    body.object.inv_stack.remove(key);
                                }
                                body.set("은전", body.get_int("은전") + total);
                                let path = format!("data/user/{}.json", body.get_name());
                                let _ = save_body_to_json(body, &path);
                                return String::new(); // 성공
                            }
                        }
                    }
                }
            }

            // 개별 아이템 확인
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let nm = o.getName();
                    let rn = o.getString("반응이름");
                    let match_ = nm == item_name || (!rn.is_empty() && rn.contains(item_name));
                    if !match_ || o.getBool("inUse") || o.checkAttr("아이템속성", "출력안함")
                    {
                        continue;
                    }
                    if o.checkAttr("아이템속성", "팔지못함") {
                        return "cant_sell".to_string();
                    }
                    let price = (o.getInt("판매가격") * buy_percent) / 100;
                    total += price;
                    to_remove.push(obj.clone());
                    if to_remove.len() >= count {
                        break;
                    }
                }
            }

            if to_remove.is_empty() {
                return "no_item".to_string();
            }

            for arc in &to_remove {
                body.object.remove(arc);
            }
            body.set("은전", body.get_int("은전") + total);

            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            String::new() // 성공
        },
    );

    // get_shop_buy_price(mob_name) - 상인의 구입 비율 반환 (1-100)
    // get_merchant_buy_percent와 동일하지만 mob_name을 인자로 받음
    let body_ptr_get_buy_price = body_ptr;
    engine.register_fn(
        "get_shop_buy_price",
        move |_ob: &mut rhai::Map, mob_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_buy_price };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return 0,
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if (data.name == mob_name || data.name.contains(mob_name))
                        && data.buy_percent > 0
                    {
                        return data.buy_percent;
                    }
                }
            }
            0
        },
    );

    // get_shop_sell_price(mob_name) - 상인의 판매 비율 반환 (1-100)
    // items_for_sale에 있는 percent 값 반환 (첫 번째 아이템의 비율)
    let body_ptr_get_sell_price = body_ptr;
    engine.register_fn(
        "get_shop_sell_price",
        move |_ob: &mut rhai::Map, mob_name: &str| -> i64 {
            let body = unsafe { &*body_ptr_get_sell_price };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return 0,
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if (data.name == mob_name || data.name.contains(mob_name))
                        && !data.items_for_sale.is_empty()
                    {
                        // 첫 번째 아이템의 판매 비율 반환
                        return data.items_for_sale[0].1.max(1);
                    }
                }
            }
            0
        },
    );

    // list_shop_inventory(ob, mob_name) - 상점 재고 목록 문자열 반환
    // Returns: 포맷된 재고 목록 문자열
    let body_ptr_list_shop = body_ptr;
    engine.register_fn(
        "list_shop_inventory",
        move |_ob: &mut rhai::Map, mob_name: &str| -> String {
            let body = unsafe { &*body_ptr_list_shop };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return "상점 정보를 가져올 수 없습니다.".to_string(),
            };
            let pos = match w.get_player_position(&name) {
                Some(p) => p,
                None => return "상점 정보를 가져올 수 없습니다.".to_string(),
            };
            let mobs = w.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
            let mut result = String::new();

            for mob in mobs {
                if let Some(data) = w.mob_cache.get_mob(&mob.mob_key) {
                    if data.name != mob_name && !data.name.contains(mob_name) {
                        continue;
                    }
                    if data.items_for_sale.is_empty() {
                        result = format!("{}: 판매하는 물건이 없습니다.", data.name);
                        break;
                    }
                    result = format!("=== {} 상점 목록 ===\r\n", data.name);
                    for (item_key, percent) in &data.items_for_sale {
                        if let Some((iname, _, base_price, _)) = get_item_info(item_key) {
                            let p = (*percent).max(1);
                            let price = base_price * 100 / p;
                            result.push_str(&format!("  {} - {}은전\r\n", iname, price));
                        }
                    }
                    break;
                }
            }

            if result.is_empty() {
                "상인을 찾을 수 없습니다.".to_string()
            } else {
                result
            }
        },
    );

    // ============================================================
    // 방파(Guild) 시스템 efun
    // ============================================================

    // Helper function: 방파에 소속된 모든 멤버 이름을 data/user/*.json에서 검색
    fn get_guild_members_from_files(guild_name: &str) -> Vec<String> {
        let mut members = Vec::new();
        if let Ok(entries) = std::fs::read_dir("data/user") {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object())
                        {
                            if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                                if let Some(guild) = attr.get("소속").and_then(|v| v.as_str()) {
                                    if guild == guild_name {
                                        if let Some(name) = uso.get("이름").and_then(|v| v.as_str())
                                        {
                                            members.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        members
    }

    // Helper function: 방파 멤버의 직위를 가져옴 (data/user/*.json에서)
    fn get_guild_member_position(member_name: &str) -> String {
        let path = format!("data/user/{}.json", member_name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object()) {
                    if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                        if let Some(pos) = attr.get("직위").and_then(|v| v.as_str()) {
                            return pos.to_string();
                        }
                    }
                }
            }
        }
        String::new()
    }

    // Helper function: 방파 멤버의 직위를 설정 (data/user/*.json에 직접 저장)
    fn set_guild_member_position(member_name: &str, position: &str) -> bool {
        let path = format!("data/user/{}.json", member_name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(uso) = json
                    .get_mut("사용자오브젝트")
                    .and_then(|v| v.as_object_mut())
                {
                    if let Some(attr) = uso.get_mut("attr").and_then(|v| v.as_object_mut()) {
                        attr.insert(
                            "직위".to_string(),
                            serde_json::Value::String(position.to_string()),
                        );
                        if let Ok(new_content) = serde_json::to_string_pretty(&json) {
                            return std::fs::write(&path, new_content).is_ok();
                        }
                    }
                }
            }
        }
        false
    }

    // guild_create(ob, guild_name) - 방파 생성
    // Returns "" on success, error string on failure
    // Admin level 1000 required
    let body_ptr_gc = body_ptr;
    engine.register_fn(
        "guild_create",
        move |_ob: &mut rhai::Map, guild_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_gc };

            // 빈 방파 이름 체크
            if guild_name.trim().is_empty() {
                return "방파 이름을 입력해주세요.".to_string();
            }

            // 중복 방파 이름 체크
            if crate::world::guild::guild_has(guild_name) {
                return "이미 존재하는 방파 이름입니다.".to_string();
            }

            // 현재 플레이어가 이미 다른 방파에 소속되어 있는지 확인
            let current_guild = body.get_string("소속");
            if !current_guild.is_empty() {
                return format!("이미 {}에 소속되어 있습니다.", current_guild);
            }

            let Some((zone, room)) = current_body_position(body) else {
                return "현재 위치를 찾을 수 없습니다.".to_string();
            };
            let home = format!("{zone}:{room}");
            if !crate::world::guild::guild_create(guild_name, &body.get_name(), &home) {
                return "이미 존재하는 방파 이름입니다.".to_string();
            }
            crate::world::guild::guild_claim_rooms(guild_name, &home);

            // 플레이어의 소속을 새 방파로 설정
            body.set("소속", guild_name.to_string());
            body.set("직위", "방주".to_string());

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            String::new() // 성공
        },
    );

    // guild_add_member(ob, member): 방주가 같은 방의 신청자를 제자로 입문시킨다.
    let body_ptr_gam = body_ptr;
    engine.register_fn(
        "guild_add_member",
        move |_ob: &mut rhai::Map, member: &str| -> bool {
            let body = unsafe { &*body_ptr_gam };
            let guild = body.get_string("소속");
            if guild.is_empty() || member.is_empty() {
                return false;
            }
            crate::world::guild::guild_add_member(&guild, "제자", member)
        },
    );

    let body_ptr_accept = body_ptr;
    engine.register_fn(
        "guild_accept_member",
        move |_ob: &mut rhai::Map, member: &str| -> String {
            let body = unsafe { &mut *body_ptr_accept };
            let applicants = body.get_string("입문신청자");
            let mut names: Vec<String> = applicants
                .split(['\r', '\n', ','])
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
            let Some(index) = names.iter().position(|name| name == member) else {
                return "not_requested".into();
            };
            let guild = body.get_string("소속");
            if guild.is_empty() {
                return "failed".into();
            }
            names.remove(index);
            body.set("입문신청자", names.join("\r\n"));
            if !crate::world::guild::guild_add_member(&guild, "방파인", member) {
                return "failed".into();
            }
            body.temp_mut().insert(
                GUILD_ACCEPT_REQUEST.to_string(),
                Value::String(
                    serde_json::to_string(&(member.to_string(), guild))
                        .unwrap_or_default(),
                ),
            );
            "ok".into()
        },
    );

    let body_ptr_apply = body_ptr;
    engine.register_fn(
        "request_guild_application",
        move |_ob: &mut rhai::Map, target: &str| {
            let body = unsafe { &mut *body_ptr_apply };
            let applicant = body.get_name();
            let request = serde_json::to_string(&(target.to_string(), applicant))
                .unwrap_or_default();
            body.temp_mut().insert(
                GUILD_APPLY_REQUEST.to_string(),
                Value::String(request),
            );
        },
    );

    let body_ptr_reset = body_ptr;
    engine.register_fn("guild_reset", move |guild_name: &str| -> bool {
        if !crate::world::guild::guild_reset(guild_name) {
            return false;
        }
        unsafe { &mut *body_ptr_reset }.temp_mut().insert(
            GUILD_RESET_REQUEST.to_string(),
            Value::String(guild_name.to_string()),
        );
        true
    });

    // guild_join(ob, guild_name) - 방파 가입
    // Returns "" on success, error string on failure
    let body_ptr_gj = body_ptr;
    engine.register_fn(
        "guild_join",
        move |_ob: &mut rhai::Map, guild_name: &str| -> String {
            let body = unsafe { &mut *body_ptr_gj };

            // 빈 방파 이름 체크
            if guild_name.trim().is_empty() {
                return "방파 이름을 입력해주세요.".to_string();
            }

            // 방파 존재 확인
            if !crate::world::guild::guild_has(guild_name) {
                return "존재하지 않는 방파입니다.".to_string();
            }

            // 이미 다른 방파에 소속되어 있는지 확인
            let current_guild = body.get_string("소속");
            if !current_guild.is_empty() {
                return format!(
                    "이미 {}에 소속되어 있습니다. 탈퇴 후 가입해주세요.",
                    current_guild
                );
            }

            // 소속 설정
            body.set("소속", guild_name.to_string());
            body.set("직위", "제자".to_string()); // 기본 직위: 제자

            // 저장
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);

            println!(
                "[SCRIPT] guild_join: {} joined {}",
                body.get_name(),
                guild_name
            );

            String::new() // 성공
        },
    );

    // guild_leave(ob) - 방파 탈퇴
    // Returns true on success
    let body_ptr_gl = body_ptr;
    engine.register_fn("guild_leave", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr_gl };

        let current_guild = body.get_string("소속");
        if current_guild.is_empty() {
            return false; // 소속된 방파가 없음
        }

        let my_name = body.get_name();

        // 방주인지 확인 (방주는 탈퇴 불가, 해체만 가능)
        let leader = crate::world::guild::guild_get(&current_guild, "방주");
        if leader == my_name {
            return false; // 방주는 탈퇴 불가
        }

        // 소속 및 직위 제거
        body.set("소속", "".to_string());
        body.set("직위", "".to_string());

        // 저장
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);

        println!("[SCRIPT] guild_leave: {} left {}", my_name, current_guild);

        true
    });

    // guild_set_nickname(ob, member, nickname): 방주의 같은 방파원 별호 설정.
    let body_ptr_gsn = body_ptr;
    engine.register_fn(
        "guild_set_nickname",
        move |_ob: &mut rhai::Map, member: &str, nickname: &str| -> String {
            let body = unsafe { &mut *body_ptr_gsn };
            let guild = body.get_string("소속");
            if guild.is_empty() {
                return "no_guild".into();
            }
            if member == body.get_name() {
                body.set("방파별호", nickname.to_string());
                return "ok".into();
            }
            let online = get_precomputed_all_online()
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .find(|player| {
                    player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .is_some_and(|name| name == member)
                });
            if let Some(target) = online {
                let target_guild = target
                    .get("소속")
                    .and_then(|value| value.clone().into_string().ok())
                    .unwrap_or_default();
                if target_guild != guild {
                    return "wrong_guild".into();
                }
                body.temp_mut().insert(
                    GUILD_NICKNAME_REQUEST.to_string(),
                    Value::String(
                        serde_json::to_string(&(member.to_string(), nickname.to_string()))
                            .unwrap_or_default(),
                    ),
                );
                return "ok".into();
            }
            let path = format!("data/user/{}.json", member);
            let Ok(content) = std::fs::read_to_string(&path) else {
                return "not_found".into();
            };
            let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) else {
                return "not_found".into();
            };
            let Some(user_object) = json
                .get_mut("사용자오브젝트")
                .and_then(|value| value.as_object_mut())
            else {
                return "not_found".into();
            };
            let attrs = if user_object.get("attr").is_some_and(|value| value.is_object()) {
                user_object
                    .get_mut("attr")
                    .and_then(|value| value.as_object_mut())
                    .unwrap()
            } else {
                user_object
            };
            if attrs.get("소속").and_then(|v| v.as_str()).unwrap_or("") != guild {
                return "wrong_guild".into();
            }
            attrs.insert(
                "방파별호".into(),
                serde_json::Value::String(nickname.to_string()),
            );
            if let Ok(saved) = serde_json::to_string_pretty(&json) {
                if std::fs::write(path, saved).is_ok() {
                    if let Ok(request) = serde_json::to_string(&(
                        member.to_string(),
                        nickname.to_string(),
                    )) {
                        body.temp_mut().insert(
                            GUILD_NICKNAME_REQUEST.to_string(),
                            Value::String(request),
                        );
                    }
                    return "ok".into();
                }
            }
            "save_failed".into()
        },
    );

    // guild_kick_member(ob, member): 방주가 같은 방파원을 파문시킨다.
    let body_ptr_gkick = body_ptr;
    engine.register_fn(
        "guild_kick_member",
        move |_ob: &mut rhai::Map, member: &str| -> String {
            let body = unsafe { &*body_ptr_gkick };
            let guild = body.get_string("소속");
            if guild.is_empty() {
                return "no_guild".into();
            }
            let online = get_precomputed_all_online()
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .find(|player| {
                    player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .is_some_and(|name| name == member)
                });
            let online_guild = online.as_ref().and_then(|player| {
                player
                    .get("소속")
                    .and_then(|value| value.clone().into_string().ok())
            });
            let online_position = online.as_ref().and_then(|player| {
                player
                    .get("직위")
                    .and_then(|value| value.clone().into_string().ok())
            });
            if let (Some(target_guild), Some(position)) = (online_guild, online_position) {
                if target_guild != guild {
                    return "wrong_guild".into();
                }
                if !crate::world::guild::guild_kick_member(&guild, &position, member) {
                    return "not_found".into();
                }
                let path = format!("data/user/{}.json", member);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(user_object) = json
                            .get_mut("사용자오브젝트")
                            .and_then(|value| value.as_object_mut())
                        {
                            let attrs = if user_object
                                .get("attr")
                                .is_some_and(|value| value.is_object())
                            {
                                user_object
                                    .get_mut("attr")
                                    .and_then(|value| value.as_object_mut())
                                    .unwrap()
                            } else {
                                user_object
                            };
                            attrs.insert(
                                "소속".into(),
                                serde_json::Value::String(String::new()),
                            );
                            attrs.insert(
                                "직위".into(),
                                serde_json::Value::String(String::new()),
                            );
                            attrs.remove("방파별호");
                            if let Ok(saved) = serde_json::to_string_pretty(&json) {
                                let _ = std::fs::write(path, saved);
                            }
                        }
                    }
                }
                unsafe { &mut *body_ptr_gkick }.temp_mut().insert(
                    GUILD_KICK_REQUEST.to_string(),
                    Value::String(member.to_string()),
                );
                return "ok".into();
            }
            let path = format!("data/user/{}.json", member);
            let Ok(content) = std::fs::read_to_string(&path) else {
                return "not_found".into();
            };
            let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) else {
                return "not_found".into();
            };
            let Some(user_object) = json
                .get_mut("사용자오브젝트")
                .and_then(|value| value.as_object_mut())
            else {
                return "not_found".into();
            };
            let attrs = if user_object.get("attr").is_some_and(|value| value.is_object()) {
                user_object
                    .get_mut("attr")
                    .and_then(|value| value.as_object_mut())
                    .unwrap()
            } else {
                user_object
            };
            if attrs.get("소속").and_then(|v| v.as_str()).unwrap_or("") != guild {
                return "wrong_guild".into();
            }
            let position = attrs
                .get("직위")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !crate::world::guild::guild_kick_member(&guild, &position, member) {
                return "not_found".into();
            }
            attrs.insert("소속".into(), serde_json::Value::String(String::new()));
            attrs.insert("직위".into(), serde_json::Value::String(String::new()));
            attrs.remove("방파별호");
            if let Ok(saved) = serde_json::to_string_pretty(&json) {
                if std::fs::write(path, saved).is_ok() {
                    unsafe { &mut *body_ptr_gkick }.temp_mut().insert(
                        GUILD_KICK_REQUEST.to_string(),
                        Value::String(member.to_string()),
                    );
                    return "ok".into();
                }
            }
            "save_failed".into()
        },
    );

    let body_ptr_transfer = body_ptr;
    engine.register_fn(
        "transfer_guild_leader",
        move |_ob: &mut rhai::Map, target: &str| -> String {
            let body = unsafe { &mut *body_ptr_transfer };
            let guild = body.get_string("소속");
            if guild.is_empty() {
                return "no_guild".into();
            }
            if target == body.get_name() {
                return "self".into();
            }
            let online = get_precomputed_all_online()
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .find(|player| {
                    player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .is_some_and(|name| name == target)
                });
            if let Some(target_player) = online {
                let target_guild = target_player
                    .get("소속")
                    .and_then(|value| value.clone().into_string().ok())
                    .unwrap_or_default();
                if target_guild != guild {
                    return "wrong_guild".into();
                }
                let position = target_player
                    .get("직위")
                    .and_then(|value| value.clone().into_string().ok())
                    .unwrap_or_default();
                if position != "부방주" {
                    return "not_deputy".into();
                }
                let level = target_player
                    .get("레벨")
                    .and_then(|value| value.as_int().ok())
                    .unwrap_or(0);
                let required = get_murim_config_int("부방주양도레벨");
                if required > level {
                    return "low_level".into();
                }
                if !crate::world::guild::guild_transfer_leader(
                    &guild,
                    &body.get_name(),
                    target,
                ) {
                    return "not_found".into();
                }
                body.set("직위", "부방주".to_string());
                body.temp_mut().insert(
                    GUILD_TRANSFER_REQUEST.to_string(),
                    Value::String(target.to_string()),
                );
                return "ok".into();
            }
            let path = format!("data/user/{}.json", target);
            let Ok(raw) = std::fs::read_to_string(&path) else {
                return "not_found".into();
            };
            let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                return "not_found".into();
            };
            let Some(user_object) = json
                .get_mut("사용자오브젝트")
                .and_then(|value| value.as_object_mut())
            else {
                return "not_found".into();
            };
            let attrs = if user_object.get("attr").is_some_and(|value| value.is_object()) {
                user_object
                    .get_mut("attr")
                    .and_then(|value| value.as_object_mut())
                    .unwrap()
            } else {
                user_object
            };
            if attrs.get("소속").and_then(|v| v.as_str()).unwrap_or("") != guild {
                return "wrong_guild".into();
            }
            if attrs.get("직위").and_then(|v| v.as_str()).unwrap_or("") != "부방주" {
                return "not_deputy".into();
            }
            let required = get_murim_config_int("부방주양도레벨");
            if required > 0 && attrs.get("레벨").and_then(|v| v.as_i64()).unwrap_or(0) < required
            {
                return "low_level".into();
            }
            if !crate::world::guild::guild_transfer_leader(
                &guild,
                &body.get_name(),
                target,
            ) {
                return "not_found".into();
            }
            attrs.insert("직위".into(), serde_json::Value::String("방주".into()));
            if let Ok(saved) = serde_json::to_string_pretty(&json) {
                let _ = std::fs::write(path, saved);
            }
            body.set("직위", "부방주".to_string());
            body.temp_mut().insert(
                GUILD_TRANSFER_REQUEST.to_string(),
                Value::String(target.to_string()),
            );
            "ok".into()
        },
    );

    // guild_get_members(ob) - 방파 멤버 목록 가져오기
    // Returns Array of member names
    let body_ptr_ggm = body_ptr;
    engine.register_fn(
        "guild_get_members",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_ggm };

            let guild_name = body.get_string("소속");
            if guild_name.is_empty() {
                return rhai::Array::new();
            }

            let members = get_guild_members_from_files(&guild_name);
            let mut arr = rhai::Array::new();
            for member in members {
                arr.push(Dynamic::from(member));
            }
            arr
        },
    );

    // guild_get_leader(ob, guild_name) - 방파 방주 이름 가져오기
    // Returns leader name
    let _body_ptr_gglead = body_ptr;
    engine.register_fn(
        "guild_get_leader",
        move |_ob: &mut rhai::Map, guild_name: &str| -> String {
            if guild_name.is_empty() {
                return String::new();
            }

            // Guild 모듈에서 방주 정보 조회
            crate::world::guild::guild_get(guild_name, "방주")
        },
    );

    // Python 직위임명 상태 변경. 대상 조회/분기 문구/출력은 Rhai가 담당한다.
    // old_position은 같은 방 온라인 snapshot에서 얻어 전달한다.
    let body_ptr_gpr = body_ptr;
    engine.register_fn(
        "guild_promote",
        move |_ob: &mut rhai::Map,
              member_name: &str,
              old_position: &str,
              position: &str|
              -> String {
            let body = unsafe { &mut *body_ptr_gpr };
            let my_guild = body.get_string("소속");
            let limit = match position {
                "방주" => Some(get_murim_config_int("방파 방주 인원").max(0) as usize),
                "부방주" => Some(get_murim_config_int("방파 부방주 인원").max(0) as usize),
                "장로" => Some(get_murim_config_int("방파 장로 인원").max(0) as usize),
                "방파인" => None,
                _ => return "invalid".to_string(),
            };
            let status = crate::world::guild::guild_reassign_position(
                &my_guild,
                member_name,
                old_position,
                position,
                limit,
            );
            if status == "ok" {
                if let Ok(request) = serde_json::to_string(&(
                    member_name.to_string(),
                    position.to_string(),
                )) {
                    body.temp_mut().insert(
                        GUILD_POSITION_REQUEST.to_string(),
                        Value::String(request),
                    );
                }
            }
            status.to_string()
        },
    );

    // guild_demote(ob, member_name) - 방파 멤버 강등
    // Returns "" on success, error string on failure
    let body_ptr_gdm = body_ptr;
    engine.register_fn(
        "guild_demote",
        move |_ob: &mut rhai::Map, member_name: &str| -> String {
            let body = unsafe { &*body_ptr_gdm };

            let my_name = body.get_name();
            let my_guild = body.get_string("소속");
            let my_position = body.get_string("직위");

            // 빈 인자 체크
            if member_name.trim().is_empty() {
                return "사용법: guild_demote(이름)".to_string();
            }

            // 방파 소속 확인
            if my_guild.is_empty() {
                return "방파에 소속되어 있지 않습니다.".to_string();
            }

            // 방주만 강등 가능
            if my_position != "방주" {
                return "방주만 멤버를 강등할 수 있습니다.".to_string();
            }

            // 대상 멤버의 현재 소속 확인
            let member_guild = {
                let path = format!("data/user/{}.json", member_name);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(uso) = json.get("사용자오브젝트").and_then(|v| v.as_object())
                        {
                            if let Some(attr) = uso.get("attr").and_then(|v| v.as_object()) {
                                attr.get("소속")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            };

            if member_guild != my_guild {
                return "해당 플레이어가 같은 방파에 소속되어 있지 않습니다.".to_string();
            }

            // 현재 직위 확인
            let current_position = get_guild_member_position(member_name);
            if current_position == "방주" {
                return "방주를 강등할 수 없습니다.".to_string();
            }

            // 한 단계 강등 (부방주->장로, 장로->제자, 제자->제자)
            let new_position = match current_position.as_str() {
                "부방주" => "장로",
                "장로" => "제자",
                _ => "제자",
            };

            // 직위 설정
            if set_guild_member_position(member_name, new_position) {
                println!(
                    "[SCRIPT] guild_demote: {} demoted to {} by {}",
                    member_name, new_position, my_name
                );
                String::new()
            } else {
                "강등에 실패했습니다.".to_string()
            }
        },
    );

    // guild_chat(ob, message) - 방파 채팅
    // Already exists as send_broadcast_to_guild, but add alias
    let spec_gchat = spec.clone();
    let _body_ptr_gchat = body_ptr;
    engine.register_fn(
        "guild_chat",
        move |ob: &mut rhai::Map, msg: &str| -> String {
            if msg.trim().is_empty() {
                return "usage".to_string();
            }
            let my_name = ob
                .get("이름")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            let guild = ob
                .get("소속")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();
            if guild.is_empty() {
                return "no_guild".to_string();
            }
            let arr = get_precomputed_all_online();
            let mut names: Vec<String> = Vec::new();
            for d in arr {
                if let Some(m) = d.clone().try_cast::<rhai::Map>() {
                    let s: String = m
                        .get("소속")
                        .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        .unwrap_or_default();
                    if s == guild {
                        if let Some(n) = m
                            .get("이름")
                            .and_then(|v: &Dynamic| v.clone().into_string().ok())
                        {
                            if !n.is_empty() {
                                names.push(n);
                            }
                        }
                    }
                }
            }
            let formatted = format!("\x1b[0;35m[방파]\x1b[0;37m {} : {}", my_name, msg);
            if let Ok(mut s) = spec_gchat.lock() {
                *s = Some(CommandResult::BroadcastToPlayers(names, formatted));
            }
            "".to_string()
        },
    );

    // guild_get_info(ob) - 방파 정보 가져오기
    // Returns Map with guild data
    let body_ptr_ggi = body_ptr;
    engine.register_fn("guild_get_info", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &*body_ptr_ggi };

        let guild_name = body.get_string("소속");
        if guild_name.is_empty() {
            return Dynamic::UNIT;
        }

        let mut info = rhai::Map::new();
        info.insert("이름".into(), Dynamic::from(guild_name.clone()));

        // Guild 모듈에서 정보 가져오기
        let mut leader = crate::world::guild::guild_get(&guild_name, "방주이름");
        if leader.is_empty() {
            leader = crate::world::guild::guild_role_members(&guild_name, "방주")
                .into_iter()
                .next()
                .unwrap_or_default();
        }
        info.insert("방주".into(), Dynamic::from(leader));
        for (out_key, role) in [("부방주", "부방주"), ("장로", "장로"), ("방파인", "방파인")] {
            let members: rhai::Array = crate::world::guild::guild_role_members(&guild_name, role)
                .into_iter()
                .map(Dynamic::from)
                .collect();
            info.insert(out_key.into(), Dynamic::from(members));
        }
        info.insert(
            "멤버수".into(),
            Dynamic::from(
                crate::world::guild::guild_get(&guild_name, "방파원수")
                    .parse::<i64>()
                    .unwrap_or(0),
            ),
        );

        Dynamic::from(info)
    });

    // guild_disband(ob) - 방파 해체
    // Returns true on success
    // Leader only
    let body_ptr_gdis = body_ptr;
    engine.register_fn("guild_disband", move |_ob: &mut rhai::Map| -> bool {
        let body = unsafe { &mut *body_ptr_gdis };

        let my_name = body.get_name();
        let my_guild = body.get_string("소속");
        let my_position = body.get_string("직위");

        // 방파 소속 확인
        if my_guild.is_empty() {
            return false;
        }

        // 방주만 해체 가능
        if my_position != "방주" {
            return false;
        }

        // 모든 멤버의 소속 및 직위 제거
        let members = get_guild_members_from_files(&my_guild);
        for member_name in &members {
            set_guild_member_position(member_name, "");
            // 직접 파일을 수정하여 소속 제거
            let path = format!("data/user/{}.json", member_name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(uso) = json
                        .get_mut("사용자오브젝트")
                        .and_then(|v| v.as_object_mut())
                    {
                        if let Some(attr) = uso.get_mut("attr").and_then(|v| v.as_object_mut()) {
                            attr.insert(
                                "소속".to_string(),
                                serde_json::Value::String("".to_string()),
                            );
                            attr.insert(
                                "직위".to_string(),
                                serde_json::Value::String("".to_string()),
                            );
                            let _ = std::fs::write(
                                &path,
                                serde_json::to_string_pretty(&json).unwrap_or_default(),
                            );
                        }
                    }
                }
            }
        }

        // 방주 본인의 소속도 제거
        body.set("소속", "".to_string());
        body.set("직위", "".to_string());

        // 방파 데이터 제거
        let _ = crate::world::guild::guild_remove(&my_guild);

        // 저장
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);

        println!(
            "[SCRIPT] guild_disband: {} disbanded by {}",
            my_guild, my_name
        );

        true
    });

    // ============================================================
    // GLOBAL DATA ACCESS FUNCTIONS (if global_data provided)
    // ============================================================
    if let Some(gd) = global_data {
        // MAIN_CONFIG는 Python처럼 메모리 캐시를 사용하고 `업데이트` 시
        // global data reload를 통해 갱신한다. 기본 엔진의 파일 읽기 버전은
        // global data가 없는 독립 테스트/도구 실행의 fallback이다.
        let gd_clone = gd.clone();
        engine.register_fn("get_murim_config_list", move |key: &str| -> rhai::Array {
            if let Ok(data) = gd_clone.try_read() {
                if let Some(values) = data
                    .get("murim")
                    .and_then(|config| config.get("메인설정"))
                    .and_then(|main| main.get(key))
                    .and_then(serde_json::Value::as_array)
                {
                    return values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .map(Dynamic::from)
                        .collect();
                }
            }
            rhai::Array::new()
        });

        // get_global(file) - 전체 파일 데이터 가져오기
        let gd_clone = gd.clone();
        engine.register_fn("get_global", move |file: &str| -> Dynamic {
            if let Ok(data) = gd_clone.try_read() {
                if let Some(json) = data.get(file) {
                    return crate::data::json_to_dynamic(json);
                }
            }
            Dynamic::UNIT
        });

        // get_global_key(file, key) - 파일에서 특정 키의 데이터 가져오기
        let gd_clone = gd.clone();
        engine.register_fn("get_global_key", move |file: &str, key: &str| -> Dynamic {
            if let Ok(data) = gd_clone.try_read() {
                if let Some(json) = data.get_path(file, key) {
                    return crate::data::json_to_dynamic(json);
                }
            }
            Dynamic::UNIT
        });

        // get_global_keys(file) - 파일의 모든 키 목록
        let gd_clone = gd.clone();
        engine.register_fn("get_global_keys", move |file: &str| -> rhai::Array {
            if let Ok(data) = gd_clone.try_read() {
                let keys: rhai::Array = data.keys(file).into_iter().map(Dynamic::from).collect();
                keys
            } else {
                rhai::Array::new()
            }
        });

        // list_globals() - 모든 파일 이름 목록
        let gd_clone = gd.clone();
        engine.register_fn("list_globals", move || -> rhai::Array {
            if let Ok(data) = gd_clone.try_read() {
                let names: rhai::Array = data.file_names().into_iter().map(Dynamic::from).collect();
                names
            } else {
                rhai::Array::new()
            }
        });

        // has_global(file) - 파일 존재 확인
        let gd_clone = gd.clone();
        engine.register_fn("has_global", move |file: &str| -> bool {
            if let Ok(data) = gd_clone.try_read() {
                data.contains(file)
            } else {
                false
            }
        });

        // has_global_key(file, key) - 파일의 키 존재 확인
        let gd_clone = gd.clone();
        engine.register_fn("has_global_key", move |file: &str, key: &str| -> bool {
            if let Ok(data) = gd_clone.try_read() {
                data.contains_key(file, key)
            } else {
                false
            }
        });
    }

    engine
}

/// Create a new Rhai engine with global data access
///
/// 글로벌 데이터 캐시에 접근할 수 있는 efuns을 등록합니다.
pub fn create_engine_with_global_data(global_data: SharedGlobalData) -> Engine {
    let mut engine = create_engine();

    // 글로벌 데이터를 clone하여 캡처
    let _gd = global_data.clone();

    // ============================================================
    // GLOBAL DATA ACCESS FUNCTIONS
    // ============================================================

    // get_global(file) - 전체 파일 데이터 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_global", move |file: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get(file) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_global_key(file, key) - 파일에서 특정 키의 데이터 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_global_key", move |file: &str, key: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_path(file, key) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_skill(name) - 스킬 데이터 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_skill", move |name: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_skill(name) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_murim_config(key) - 무림 설정 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_murim_config", move |key: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_murim_config(key) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // get_map_path(zone) - 맵 경로 가져오기
    let gd_clone = global_data.clone();
    engine.register_fn("get_map_path", move |zone: &str| -> Dynamic {
        if let Ok(data) = gd_clone.try_read() {
            if let Some(json) = data.get_map_path(zone) {
                return crate::data::json_to_dynamic(json);
            }
        }
        Dynamic::UNIT
    });

    // has_global(file) - 파일 존재 확인
    let gd_clone = global_data.clone();
    engine.register_fn("has_global", move |file: &str| -> bool {
        if let Ok(data) = gd_clone.try_read() {
            data.contains(file)
        } else {
            false
        }
    });

    // has_global_key(file, key) - 파일의 키 존재 확인
    let gd_clone = global_data.clone();
    engine.register_fn("has_global_key", move |file: &str, key: &str| -> bool {
        if let Ok(data) = gd_clone.try_read() {
            data.contains_key(file, key)
        } else {
            false
        }
    });

    // get_global_keys(file) - 파일의 모든 키 목록
    let gd_clone = global_data.clone();
    engine.register_fn("get_global_keys", move |file: &str| -> rhai::Array {
        if let Ok(data) = gd_clone.try_read() {
            let keys: rhai::Array = data.keys(file).into_iter().map(Dynamic::from).collect();
            keys
        } else {
            rhai::Array::new()
        }
    });

    // list_globals() - 모든 파일 이름 목록
    let gd_clone = global_data.clone();
    engine.register_fn("list_globals", move || -> rhai::Array {
        if let Ok(data) = gd_clone.try_read() {
            let names: rhai::Array = data.file_names().into_iter().map(Dynamic::from).collect();
            names
        } else {
            rhai::Array::new()
        }
    });

    // reload_global(file) - 특정 파일 다시 로드
    let gd_clone = global_data.clone();
    engine.register_fn("reload_global", move |file: &str| -> bool {
        if let Ok(mut data) = gd_clone.try_write() {
            data.reload(file).unwrap_or(false)
        } else {
            false
        }
    });

    // reload_all_globals() - 모든 파일 다시 로드
    let gd_clone = global_data.clone();
    engine.register_fn("reload_all_globals", move || -> i64 {
        if let Ok(mut data) = gd_clone.try_write() {
            data.reload_all().unwrap_or(0) as i64
        } else {
            0
        }
    });

    engine
}

/// Convert serde_json::Value to Rhai Dynamic
/// 내부적으로 data 모듈의 json_to_dynamic를 사용합니다.
fn json_value_to_dynamic(value: serde_json::Value) -> Dynamic {
    crate::data::json_to_dynamic(&value)
}

fn python_json_repr(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(value) => {
            if *value { "True" } else { "False" }.to_string()
        }
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'")),
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(python_json_repr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_json::Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("'{}': {}", key, python_json_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Script storage - stores raw script source code
pub struct ScriptStorage {
    scripts: HashMap<String, StoredScript>,
    /// Library scripts loaded from lib/ directory (hot-reloadable)
    libraries: HashMap<String, String>,
    config: ScriptConfig,
    /// 글로벌 데이터 캐시 참조
    global_data: Option<SharedGlobalData>,
}

unsafe impl Send for ScriptStorage {}
unsafe impl Sync for ScriptStorage {}

impl ScriptStorage {
    pub fn new(config: ScriptConfig) -> Self {
        let mut storage = Self {
            scripts: HashMap::new(),
            libraries: HashMap::new(),
            config,
            global_data: None,
        };
        storage.load_all_libraries().ok();
        storage.load_all_scripts().ok();
        storage
    }

    /// 글로벌 데이터 캐시와 함께 생성합니다.
    pub fn with_global_data(config: ScriptConfig, global_data: SharedGlobalData) -> Self {
        let mut storage = Self {
            scripts: HashMap::new(),
            libraries: HashMap::new(),
            config,
            global_data: Some(global_data),
        };
        storage.load_all_libraries().ok();
        storage.load_all_scripts().ok();
        storage
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(ScriptConfig::default())
    }

    /// 글로벌 데이터 캐시를 설정합니다.
    pub fn set_global_data(&mut self, global_data: SharedGlobalData) {
        self.global_data = Some(global_data);
    }

    /// 글로벌 데이터 캐시를 가져옵니다.
    pub fn get_global_data(&self) -> Option<SharedGlobalData> {
        self.global_data.clone()
    }

    pub fn load_all_scripts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = self.config.script_dir.clone();
        if !dir.exists() {
            info!("Creating script directory: {:?}", dir);
            std::fs::create_dir_all(&dir)?;
            return Ok(());
        }

        let entries = std::fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rhai") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                self.load_script(&name, &path)?;
            }
        }

        info!("Loaded {} scripts from {:?}", self.scripts.len(), dir);
        Ok(())
    }

    /// Python `init_commands()` compiles each source before replacing that
    /// command. Keep already-loaded sources on a syntax failure while allowing
    /// earlier valid files in directory order to have been refreshed.
    #[allow(dead_code)]
    pub(crate) fn load_all_scripts_checked(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = self.config.script_dir.clone();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
            return Ok(());
        }

        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rhai") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("unknown")
                .to_string();
            let source = std::fs::read_to_string(&path)?;
            Engine::new()
                .compile(&source)
                .map_err(|error| format!("{}: {}", path.display(), error))?;
            let modified = std::fs::metadata(&path)?.modified()?;
            self.scripts.insert(
                name.clone(),
                StoredScript {
                    source,
                    modified,
                    _name: name,
                },
            );
        }
        Ok(())
    }

    /// Load all library scripts from lib/ directory (recursively) for hot-reloadable functions
    pub fn load_all_libraries(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = self.config.lib_dir.clone();
        if !dir.exists() {
            info!("Library directory does not exist: {:?}", dir);
            return Ok(());
        }

        self.load_libraries_recursive(&dir)?;

        info!(
            "Loaded {} library scripts from {:?}",
            self.libraries.len(),
            dir
        );
        Ok(())
    }

    /// Recursively load .rhai files from a directory
    fn load_libraries_recursive(&mut self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip lib/std/ and lib/doumi/ directories
                // lib/std/ files define object templates with duplicate function names
                // lib/doumi/ files are DOUMI character creation scripts, not libraries
                if let Some(file_name) = path.file_name() {
                    if file_name == "std" || file_name == "doumi" {
                        continue;
                    }
                }
                // Recursively load from subdirectories
                self.load_libraries_recursive(&path)?;
            } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rhai") {
                // Create a unique name based on relative path from lib_dir
                let rel_path = path
                    .strip_prefix(&self.config.lib_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/");

                // Skip std/ and doumi/ directory files
                if rel_path.starts_with("std/") || rel_path.starts_with("doumi/") {
                    continue;
                }

                // Remove .rhai extension from the relative path to get a unique library name
                let name = rel_path
                    .strip_suffix(".rhai")
                    .unwrap_or(&rel_path)
                    .to_string();

                let source = std::fs::read_to_string(&path)?;
                debug!("Loaded library: {} from {:?}", name, path);
                self.libraries.insert(name, source);
            }
        }
        Ok(())
    }

    /// Reload all library scripts from lib/ directory
    pub fn reload_libraries(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        self.libraries.clear();
        self.load_all_libraries()?;
        Ok(self.libraries.len())
    }

    /// Get combined library source code to prepend to scripts
    pub fn get_library_source(&self) -> String {
        let mut combined = String::new();
        for (name, source) in &self.libraries {
            combined.push_str("// Library: ");
            combined.push_str(name);
            combined.push('\n');
            combined.push_str(source);
            combined.push('\n');
        }
        combined
    }

    pub fn load_script(
        &mut self,
        name: &str,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let source = std::fs::read_to_string(path)?;
        self.scripts.insert(
            name.to_string(),
            StoredScript {
                source,
                modified,
                _name: name.to_string(),
            },
        );
        debug!("Loaded script: {} from {:?}", name, path);
        Ok(())
    }

    pub fn reload_script(&mut self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let script_path = self.config.script_dir.join(format!("{}.rhai", name));
        if !script_path.exists() {
            return Ok(false);
        }

        let metadata = std::fs::metadata(&script_path)?;
        let modified = metadata.modified()?;

        if let Some(script) = self.scripts.get(name) {
            if modified <= script.modified {
                return Ok(false);
            }
        }

        let source = std::fs::read_to_string(&script_path)?;
        self.scripts.insert(
            name.to_string(),
            StoredScript {
                source,
                modified,
                _name: name.to_string(),
            },
        );

        info!("Reloaded script: {}", name);
        Ok(true)
    }

    pub fn reload_all(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut reloaded = 0;
        let names: Vec<String> = self.scripts.keys().cloned().collect();
        for name in names {
            if self.reload_script(&name)? {
                reloaded += 1;
            }
        }
        Ok(reloaded)
    }

    /// get_other_players_desc: 봐 시 같은 방 다른 유저 getDesc. None이면 빈 목록.
    /// get_other_players_map: 봐 find_target에서 (이름→getDesc). None이면 빈 맵.
    /// call_out_scheduler: Some이면 call_out/call_later 사용 가능(지연 시 스크립트 함수 실행).
    /// Returns (outputs, special). special=Some(CommandResult)이면 Shout/Tell/EmotionToRoom/GiveToPlayer 등.
    pub fn execute(
        &self,
        name: &str,
        player: &mut Body,
        line: &str,
        get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
        get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
        call_out_scheduler: Option<Arc<CallOutScheduler>>,
    ) -> Result<(Vec<String>, Option<CommandResult>), Box<dyn std::error::Error>> {
        tracing::debug!(script = name, "Executing Rhai script");
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;

        let output_collector = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output_collector.clone();
        let special_collector = Arc::new(Mutex::new(None));
        let user_sends = Arc::new(Mutex::new(Vec::new()));

        let engine = create_engine_with_body_and_output(
            player,
            output_clone,
            get_other_players_desc,
            get_other_players_map,
            special_collector.clone(),
            user_sends.clone(),
            call_out_scheduler,
            Some(name),
            self.global_data.clone(),
        );
        let mut scope = Scope::new();

        let player_data = build_ob_from_body(player);
        scope.push("player", player_data.clone());
        scope.push("me", player_data.clone());
        scope.push("ob", player_data.clone());
        scope.push("this", player_data); // For std library functions that use 'this'
        scope.push("cmdline", rhai::Dynamic::from(line.to_string()));

        // DOUMI system global variables for script suspension/resumption
        scope.push("_doumi_resume_op", "" as &str);
        scope.push("_doumi_resume_input", "" as &str);

        // Prepend library source for hot-reloadable functions
        let library_source = self.get_library_source();
        let script_with_main = format!("{}\n{}\nmain(ob, cmdline)", library_source, script.source);
        tracing::debug!(
            script = name,
            source_length = script_with_main.len(),
            "Running Rhai script"
        );
        let result = engine.run_with_scope(&mut scope, &script_with_main);
        tracing::debug!(script = name, success = result.is_ok(), "Rhai script finished");
        result.map_err(|e| format!("스크립트 실행 오류: {}", e))?;

        let outputs = output_collector.lock().unwrap().clone();
        tracing::debug!(script = name, outputs = outputs.len(), "Collected Rhai output");
        let expanded: Vec<String> = outputs
            .into_iter()
            .map(|s| expand_abbreviated_ansi(&s))
            .collect();
        let mut special = special_collector.lock().unwrap().take();
        let to_send = user_sends.lock().unwrap().drain(..).collect::<Vec<_>>();
        if special.is_none() && !to_send.is_empty() {
            special = if expanded.is_empty() {
                Some(CommandResult::SendToUsers(to_send))
            } else {
                Some(CommandResult::OutputAndSendToUsers(
                    expanded.join("\r\n"),
                    to_send,
                ))
            };
        }
        Ok((expanded, special))
    }

    pub fn execute_with_scope(
        &self,
        name: &str,
        scope: &mut Scope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;
        let engine = create_engine();
        engine.run_with_scope(scope, &script.source)?;
        Ok(())
    }

    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }

    pub fn script_names(&self) -> Vec<String> {
        self.scripts.keys().cloned().collect()
    }

    /// Get script source by name. For call_out script_runner to run a function from the script.
    pub fn get_script_source(&self, name: &str) -> Option<String> {
        self.scripts.get(name).map(|s| s.source.clone())
    }

    /// Call a named function from a loaded Rhai script. This is the driver
    /// boundary used by Master/heartbeat; the script still owns the policy.
    pub fn call_script_function(
        &self,
        name: &str,
        function: &str,
        args: Vec<Dynamic>,
    ) -> Result<Dynamic, String> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("script not found: {name}"))?;
        let source = script.source.clone();
        let engine = create_engine();
        let ast = engine
            .compile(&source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let mut scope = Scope::new();
        let result = match args.as_slice() {
            [] => engine.call_fn::<Dynamic>(&mut scope, &ast, function, ()),
            [arg] => engine.call_fn::<Dynamic>(&mut scope, &ast, function, (arg.clone(),)),
            [first, second] => engine.call_fn::<Dynamic>(
                &mut scope,
                &ast,
                function,
                (first.clone(), second.clone()),
            ),
            [first, second, third] => engine.call_fn::<Dynamic>(
                &mut scope,
                &ast,
                function,
                (first.clone(), second.clone(), third.clone()),
            ),
            _ => return Err("Rhai apply supports at most three arguments".to_string()),
        };
        result.map_err(|error| format!("call {name}.{function}: {error}"))
    }

    pub fn call_script_string(
        &self,
        name: &str,
        function: &str,
        arg: &str,
    ) -> Result<String, String> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("script not found: {name}"))?;
        let source = script.source.clone();
        let engine = create_engine();
        let ast = engine
            .compile(&source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let mut scope = Scope::new();
        engine
            .call_fn::<String>(&mut scope, &ast, function, (arg.to_string(),))
            .map_err(|error| format!("call {name}.{function}: {error}"))
    }

    pub fn call_script_string0(&self, name: &str, function: &str) -> Result<String, String> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("script not found: {name}"))?;
        let source = script.source.clone();
        let engine = create_engine();
        let ast = engine
            .compile(&source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let mut scope = Scope::new();
        engine
            .call_fn::<String>(&mut scope, &ast, function, ())
            .map_err(|error| format!("call {name}.{function}: {error}"))
    }

    pub fn call_script_string2(
        &self,
        name: &str,
        function: &str,
        first: &str,
        second: &str,
    ) -> Result<String, String> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("script not found: {name}"))?;
        let source = script.source.clone();
        let engine = create_engine();
        let ast = engine
            .compile(&source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let mut scope = Scope::new();
        engine
            .call_fn::<String>(
                &mut scope,
                &ast,
                function,
                (first.to_string(), second.to_string()),
            )
            .map_err(|error| format!("call {name}.{function}: {error}"))
    }

    pub fn call_script_bool2(
        &self,
        name: &str,
        function: &str,
        first: &str,
        second: &str,
    ) -> Result<bool, String> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("script not found: {name}"))?;
        let source = script.source.clone();
        let engine = create_engine();
        let ast = engine
            .compile(&source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let mut scope = Scope::new();
        engine
            .call_fn::<bool>(
                &mut scope,
                &ast,
                function,
                (first.to_string(), second.to_string()),
            )
            .map_err(|error| format!("call {name}.{function}: {error}"))
    }

    pub fn call_script_unit(
        &self,
        name: &str,
        function: &str,
        args: Vec<Dynamic>,
    ) -> Result<(), String> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("script not found: {name}"))?;
        let source = script.source.clone();
        let engine = create_engine();
        let ast = engine
            .compile(&source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let mut scope = Scope::new();
        let result = match args.as_slice() {
            [] => engine.call_fn::<()>(&mut scope, &ast, function, ()),
            [arg] => engine.call_fn::<()>(&mut scope, &ast, function, (arg.clone(),)),
            [first, second] => {
                engine.call_fn::<()>(&mut scope, &ast, function, (first.clone(), second.clone()))
            }
            _ => return Err("Rhai apply supports at most two arguments".to_string()),
        };
        result.map_err(|error| format!("call {name}.{function}: {error}"))
    }
}

/// Body로부터 Rhai ob(Map) 생성. execute 및 call_out 콜백에서 사용.
fn build_ob_from_body(body: &Body) -> rhai::Map {
    let mut m = rhai::Map::new();

    // Object.attr가 Rhai 객체 속성의 기준이다. 고정 화이트리스트만 복사하면
    // 새/레거시 속성이 스크립트에서 보이지 않으므로 모든 속성을 먼저 노출한다.
    for (key, value) in &body.object.attr {
        let dynamic = match value {
            Value::Int(value) => Dynamic::from(*value),
            Value::Float(value) => Dynamic::from(*value),
            Value::String(value) => Dynamic::from(value.clone()),
        };
        m.insert(key.clone().into(), dynamic);
    }

    // 아래 항목은 Python 호환 별칭 또는 런타임 계산값이다.
    m.insert("name".into(), body.get_name().into());
    m.insert("hp".into(), body.get_hp().into());
    m.insert("max_hp".into(), body.get_max_hp().into());
    m.insert("mp".into(), body.get_mp().into());
    m.insert("max_mp".into(), body.get_max_mp().into());
    m.insert("level".into(), body.get_int("레벨").into());
    m.insert("레벨".into(), body.get_int("레벨").into());
    m.insert("나이".into(), body.get_int("나이").into());
    m.insert("맷집".into(), body.get_int("맷집").into());
    m.insert("현재경험치".into(), body.get_int("현재경험치").into());
    m.insert("money".into(), body.get_int("은전").into());
    m.insert("은전".into(), body.get_int("은전").into());
    m.insert("금전".into(), body.get_int("금전").into());
    m.insert("str".into(), body.get_str().into());
    m.insert("dex".into(), body.get_dex().into());
    m.insert("이름".into(), body.get_name().into());
    m.insert("관리자등급".into(), body.get_int("관리자등급").into());
    m.insert("act".into(), (body.act.to_i32() as i64).into());
    m.insert("성격".into(), body.get_string("성격").into());
    m.insert("소속".into(), body.get_string("소속").into());
    m.insert("설정상태".into(), body.get_string("설정상태").into());
    m.insert(
        "운기조식".into(),
        (body.act == crate::player::ActState::Rest).into(),
    );
    m.insert("env".into(), "".into());
    m.insert("objs".into(), rhai::Dynamic::from(rhai::Array::new()));
    // 숙련도.rhai: 검/도/창/기타/맨손
    m.insert("1 숙련도".into(), body.get_int("1 숙련도").into());
    m.insert("2 숙련도".into(), body.get_int("2 숙련도").into());
    m.insert("3 숙련도".into(), body.get_int("3 숙련도").into());
    m.insert("4 숙련도".into(), body.get_int("4 숙련도").into());
    m.insert("5 숙련도".into(), body.get_int("5 숙련도").into());

    // Korean attribute keys that scripts access via get_int()
    // These are required by 능력치.rhai and other scripts
    m.insert("체력".into(), body.get_hp().into());
    m.insert("최고체력".into(), body.get_int("최고체력").into());
    m.insert("내공".into(), body.get_mp().into());
    m.insert("최고내공".into(), body.get_max_mp().into());
    m.insert("힘".into(), body.get_int("힘").into());
    m.insert("민첩성".into(), body.get_int("민첩성").into());
    m.insert("명중".into(), body.get_int("명중").into());
    m.insert("회피".into(), body.get_int("회피").into());
    m.insert("필살".into(), body.get_int("필살").into());
    m.insert("운".into(), body.get_int("운").into());
    m.insert("배우자".into(), body.get_string("배우자").into());
    m.insert("직위".into(), body.get_string("직위").into());
    m.insert("성별".into(), body.get_string("성별").into());
    m.insert("목표경험치".into(), body.get_int("목표경험치").into());
    m.insert("분노".into(), body.get_int("분노").into());
    m.insert("소지품무게".into(), body.get_int("소지품무게").into());
    m.insert("특성치".into(), body.get_int("특성치").into());
    m
}

/// call_out 만료 시 Rhai 스크립트 함수를 실행하는 runner 생성.
/// (target, script, function, args) -> Result. process_due에서 호출.
pub fn create_call_out_script_runner(
    script_storage: Arc<tokio::sync::RwLock<ScriptStorage>>,
    broadcaster: Arc<Broadcaster>,
) -> Arc<dyn Fn(&str, Option<&str>, &str, Vec<serde_json::Value>) -> Result<(), String> + Send + Sync>
{
    Arc::new(
        move |target: &str, script: Option<&str>, function: &str, _args: Vec<serde_json::Value>| {
            let script = script.ok_or_else(|| "call_out: script name required".to_string())?;
            // process_due는 tokio 워커에서 호출되므로 blocking_read 전에 block_in_place로 블로킹 허용
            let (source, global_data) = tokio::task::block_in_place(|| {
                let storage = script_storage.blocking_read();
                (
                    storage.get_script_source(script),
                    storage.global_data.clone(),
                )
            });
            let source = source.ok_or_else(|| format!("script not found: {}", script))?;

            // 클로저 안에서는 clients 락이 잡혀 있으므로 send_to_by_player_name(→clients.lock()) 호출 금지.
            // 메시지만 수집하고, 락 해제 후 밖에서 전송.
            let to_send = broadcaster
                .with_player_body_by_name(target, |body| {
                    let output_collector = Arc::new(Mutex::new(Vec::new()));
                    let special_collector = Arc::new(Mutex::new(None));
                    let user_sends = Arc::new(Mutex::new(Vec::new()));
                    let engine = create_engine_with_body_and_output(
                        body,
                        output_collector.clone(),
                        None,
                        None,
                        special_collector,
                        user_sends,
                        None,
                        None,
                        global_data.clone(),
                    );
                    let ast = engine
                        .compile(&source)
                        .map_err(|e| format!("compile: {}", e))?;
                    let mut scope = Scope::new();
                    let ob = Dynamic::from(build_ob_from_body(body));
                    let _ = engine
                        .call_fn::<Dynamic>(&mut scope, &ast, function, (ob,))
                        .map_err(|e| format!("call_fn {}: {}", function, e))?;

                    let outputs = output_collector.lock().unwrap().clone();
                    let messages: Vec<String> = outputs
                        .iter()
                        .map(|line| {
                            let expanded = expand_abbreviated_ansi(line);
                            format!("{}\r\n", expanded)
                        })
                        .collect();
                    Ok::<_, String>(messages)
                })
                .ok_or_else(|| format!("player not found: {}", target))?;

            let messages = to_send?;
            for msg in messages {
                let _ = broadcaster.send_to_by_player_name(target, &msg);
            }
            Ok(())
        },
    )
}

pub struct SharedScriptStorage {
    inner: Arc<RwLock<ScriptStorage>>,
}

impl SharedScriptStorage {
    pub fn new(config: ScriptConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ScriptStorage::new(config))),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(ScriptConfig::default())
    }

    pub async fn execute(
        &self,
        name: &str,
        player: &mut Body,
        line: &str,
        get_other_players_desc: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>>,
        get_other_players_map: Option<Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
        call_out_scheduler: Option<Arc<CallOutScheduler>>,
    ) -> Result<(Vec<String>, Option<CommandResult>), Box<dyn std::error::Error>> {
        let storage = self.inner.read().await;
        storage.execute(
            name,
            player,
            line,
            get_other_players_desc,
            get_other_players_map,
            call_out_scheduler,
        )
    }

    pub async fn reload_all(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut storage = self.inner.write().await;
        storage.reload_all()
    }

    pub async fn has_script(&self, name: &str) -> bool {
        let storage = self.inner.read().await;
        storage.has_script(name)
    }

    pub async fn script_names(&self) -> Vec<String> {
        let storage = self.inner.read().await;
        storage.script_names()
    }
}

pub type ScriptEngine = ScriptStorage;
pub type SharedScriptEngine = SharedScriptStorage;

#[cfg(test)]
mod tests {
    use super::*;

    static ONEITEM_COMMAND_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_script_config_default() {
        let config = ScriptConfig::default();
        assert_eq!(config.script_dir, PathBuf::from("cmds"));
        assert!(config.hot_reload);
        assert_eq!(config.extension, ".rhai");
    }

    #[test]
    fn test_script_storage_new() {
        let storage = ScriptStorage::default();
        assert!(storage.config.script_dir.ends_with("cmds"));
    }

    #[test]
    fn test_has_script() {
        let storage = ScriptStorage::default();
        assert!(!storage.has_script("nonexistent"));
    }

    #[test]
    fn guard_qi_command_distinguishes_python_no_guard_branch() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        let (output, special) = storage
            .execute("내공주입", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 호위를 거느리지 않고 있습니다."]);
        assert!(special.is_none());
    }

    #[test]
    fn guard_qi_command_heals_template_hp_and_reports_total_spend() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("힘", 100_i64);
        body.set("내공", 500_i64);
        let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
        {
            let mut guard = guard.lock().unwrap();
            guard.set("체력", 1000_i64);
        }
        body.object.objs.push(guard);

        let (output, _) = storage
            .execute("내공주입", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("내공"), 490);
        assert_eq!(body.object.objs[0].lock().unwrap().getInt("체력"), 1224);
        assert!(output[0].contains("사강시에게 내가진기를 주입하여 체력을 회복 시킵니다."));
        assert!(output[0].contains("+224"));
        assert_eq!(
            output[1],
            "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-10\x1b[0;37m)"
        );
    }

    #[test]
    fn guard_qi_command_matches_python_full_shortage_and_partial_inventory_order() {
        let storage = ScriptStorage::default();
        let make_guard = |hp: i64| {
            let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
            guard.lock().unwrap().set("체력", hp);
            guard
        };

        let mut full = Body::new();
        full.set("힘", 100_i64);
        full.set("내공", 100_i64);
        full.object.objs.push(make_guard(2240));
        let full_result = storage
            .execute("내공주입", &mut full, "무시", None, None, None)
            .unwrap();
        assert_eq!(full_result.0, vec!["☞ 회복할 호위가 없습니다."]);
        assert_eq!(full.get_int("내공"), 100);

        let mut shortage = Body::new();
        shortage.set("힘", 100_i64);
        shortage.set("내공", 9_i64);
        shortage.object.objs.push(make_guard(1000));
        let shortage_result = storage
            .execute("내공주입", &mut shortage, "", None, None, None)
            .unwrap();
        assert_eq!(
            shortage_result.0,
            vec!["☞ 내가진기를 주입할 내공이 부족합니다."]
        );
        assert_eq!(shortage.get_int("내공"), 9);
        assert_eq!(shortage.object.objs[0].lock().unwrap().getInt("체력"), 1000);

        let mut partial = Body::new();
        partial.set("힘", 100_i64);
        partial.set("내공", 15_i64);
        partial.object.objs.push(make_guard(1000));
        partial.object.objs.push(make_guard(1200));
        let partial_result = storage
            .execute("내공주입", &mut partial, "입력은 사용하지 않음", None, None, None)
            .unwrap();
        assert_eq!(partial.get_int("내공"), 5);
        assert_eq!(partial.object.objs[0].lock().unwrap().getInt("체력"), 1224);
        assert_eq!(partial.object.objs[1].lock().unwrap().getInt("체력"), 1200);
        assert_eq!(partial_result.0.len(), 2);
        assert_eq!(
            partial_result.0[0],
            "당신이 사강시에게 내가진기를 주입하여 체력을 회복 시킵니다. (\x1b[1;36m+224\x1b[0;37m)\r\n"
        );
        assert_eq!(
            partial_result.0[1],
            "당신이 소모된 진기를 다스립니다. (\x1b[1;32m-10\x1b[0;37m)"
        );

        let mut negative = Body::new();
        negative.set("힘", 100_i64);
        negative.set("내공", 20_i64);
        let odd_guard = make_guard(1000);
        odd_guard.lock().unwrap().set("내공감소", -10_i64);
        negative.object.objs.push(odd_guard);
        let odd = storage
            .execute("내공주입", &mut negative, "", None, None, None)
            .unwrap();
        assert_eq!(negative.get_int("내공"), 30);
        assert_eq!(
            odd.0[1],
            "당신이 소모된 진기를 다스립니다. (\x1b[1;32m--10\x1b[0;37m)"
        );
    }

    #[test]
    fn guard_view_reads_the_same_inventory_objects_as_guard_combat() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("분노", 37_i64);
        let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
        guard.lock().unwrap().set("체력", 700_i64);
        body.object.objs.push(guard);

        let (output, _) = storage
            .execute("호위", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output.len(), 1);
        assert!(output[0].contains("☞ 당신의 호위 : 사강시, 호위수 : 1, 분노 : 37"));
        assert!(output[0].contains("사강시\x1b[0;37m ː"));
        assert!(output[0].contains("(50)"));
    }

    #[test]
    fn teleport_rejects_a_non_dragon_first_guard_like_python() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("위치각인", "낙양성:42");
        let (guard, _) = object_from_item_json("사강시").expect("guard fixture");
        body.object.objs.push(guard);
        let (output, _) = storage
            .execute("이형환위", &mut body, "비학천룡", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 비학천룡이 없습니다."]);
        let (output, _) = storage
            .execute("위치각인", &mut body, "비학천룡", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 비학천룡이 없습니다."]);
    }

    #[test]
    fn eating_uses_item_script_and_clamps_vitals_before_removal() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "복용검사");
        body.set("체력", 100_i64);
        body.set("최고체력", 500_i64);
        let (food, _) = object_from_item_json("1037").expect("food fixture");
        body.object.objs.push(food);
        let (output, _) = storage
            .execute("먹어", &mut body, "탕수육", None, None, None)
            .unwrap();
        assert_eq!(body.get_hp(), 500);
        assert!(body.object.objs.is_empty());
        assert_eq!(
            output,
            vec!["당신이 \x1b[0m\x1b[36m\x1b[40m탕수육\x1b[0m\x1b[37m\x1b[40m을 맛있게 먹습니다."]
        );
    }

    #[test]
    fn eating_poison_preserves_python_negative_vitals_without_lower_clamp() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "독음식검사");
        body.set("체력", 5_i64);
        body.set("내공", 3_i64);
        body.set("최고체력", 100_i64);
        body.set("최고내공", 100_i64);
        let mut poison = Object::new();
        poison.set("이름", "독버섯");
        poison.set("반응이름", "독버섯");
        poison.set("종류", "먹는것");
        poison.set("체력", -10_i64);
        poison.set("내공", -8_i64);
        poison.set("사용스크립", "$아이템$을 먹고 고통스러워합니다.");
        body.object.objs.push(Arc::new(Mutex::new(poison)));

        let eaten = storage
            .execute("먹어", &mut body, "독버섯", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("체력"), -5);
        assert_eq!(body.get_int("내공"), -5);
        assert!(body.object.objs.is_empty());
        assert_eq!(
            eaten.0,
            vec!["당신이 \x1b[0;36m독버섯\x1b[37m을 먹고 고통스러워합니다."]
        );
    }

    #[test]
    fn eating_requeues_any_healing_food_when_python_continuous_aliases_are_enabled() {
        use crate::command::handler::CommandResult;

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "연속복용검사");
        body.set("체력", 100_i64);
        body.set("최고체력", 10_000_i64);
        body.set(
            ALIAS_LIST_ATTR,
            encode_alias_entries(&[
                ("체력약".into(), "다른약".into()),
                ("체력".into(), "9000이하".into()),
                ("연속복용".into(), "켜기".into()),
            ]),
        );
        let (food, _) = object_from_item_json("1037").expect("food fixture");
        body.object.objs.push(food);
        let (output, special) = storage
            .execute("먹어", &mut body, "탕수육", None, None, None)
            .unwrap();
        assert_eq!(body.get_hp(), 7030);
        assert_eq!(output.len(), 1);
        assert!(matches!(
            special,
            Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
                if own == &output[0]
                    && sends == &vec![("연속복용검사".to_string(), "탕수육 먹어".to_string())]
        ));
    }

    #[test]
    fn admin_skill_rank_preserves_unbounded_python_integer() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "성값검사");
        body.set("관리자등급", 2000_i64);
        let (output, _) = storage
            .execute("성올려", &mut body, "성값검사 태극권 999", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 값이 설정되었습니다."]);
        assert_eq!(
            body.skill_map["태극권"],
            crate::player::SkillTraining::new(999, 199_999)
        );
        let _ = storage
            .execute("성올려", &mut body, "성값검사 태극권 -7", None, None, None)
            .unwrap();
        assert_eq!(body.skill_map["태극권"].level, -7);
    }

    #[test]
    fn rank_command_uses_live_players_stable_ties_and_python_columns() {
        let online = [
            ("Alpha", 10_i64, 0_i64),
            ("Bravo", 20_i64, 0_i64),
            ("Charlie", 20_i64, 0_i64),
            ("Operator", 999_i64, 1000_i64),
            ("Zero", 0_i64, 0_i64),
        ]
        .into_iter()
        .map(|(name, strength, admin)| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name));
            map.insert("힘".into(), Dynamic::from(strength));
            map.insert("관리자등급".into(), Dynamic::from(admin));
            Dynamic::from(map)
        })
        .collect();
        set_precomputed_all_online(online);
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "순위조회자");
        body.set("은전", 200_000_i64);
        let normal = storage
            .execute("순위", &mut body, "힘", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("은전"), 100_000);
        assert_eq!(normal.0, vec!["[01] Bravo      [02] Charlie    [03] Alpha      "]);

        body.set("관리자등급", 1000_i64);
        let admin = storage
            .execute("순위", &mut body, "힘", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("은전"), 0);
        assert_eq!(
            admin.0,
            vec!["     Bravo 20                Charlie 20                  Alpha 10             \r\n"]
        );
        clear_precomputed_all_online();
        let _ = std::fs::remove_file("data/user/순위조회자.json");
    }

    #[test]
    fn who_command_preserves_python_order_filters_and_exact_line_count() {
        let make = |name: &str,
                    nick: &str,
                    tendency: &str,
                    reset: &str,
                    guild: &str,
                    transparent: i64| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name.to_string()));
            map.insert("무림별호".into(), Dynamic::from(nick.to_string()));
            map.insert("성격".into(), Dynamic::from(tendency.to_string()));
            map.insert("레벨초기화".into(), Dynamic::from(reset.to_string()));
            map.insert("소속".into(), Dynamic::from(guild.to_string()));
            map.insert("투명상태".into(), Dynamic::from(transparent));
            Dynamic::from(map)
        };
        set_precomputed_all_online(vec![
            make("첫째", "", "", "", "청룡", 0),
            make("숨은자", "은자", "선인", "", "청룡", 1),
            make("둘째", "검성", "정파", "", "청룡", 0),
            make("셋째", "혈마", "사파", "1", "백호", 0),
        ]);
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "누구회귀");
        body.set("소속", "청룡");

        let all = storage.execute("누구", &mut body, "", None, None, None).unwrap();
        assert_eq!(all.0.len(), 6, "Python emits no invented final blank line");
        assert!(all.0[3].find("첫째").unwrap() < all.0[3].find("둘째").unwrap());
        assert!(all.0[3].find("둘째").unwrap() < all.0[3].find("셋째").unwrap());
        assert!(!all.0[3].contains("숨은자"));
        assert!(all.0[3].contains("[\x1b[0;37m무명객\x1b[0;37m]"));
        assert!(all.0[3].contains("[\x1b[1;32m검성\x1b[0;37m]"));
        assert!(all.0[3].contains("<\x1b[0;31m혈마\x1b[0;37m>"));
        assert_eq!(all.0[5], " ★ 총 3명의 무림인이 활동하고 있습니다.");

        let guild = storage
            .execute("누구", &mut body, "방파", None, None, None)
            .unwrap();
        assert_eq!(guild.0.len(), 6);
        assert!(guild.0[3].contains("첫째"));
        assert!(guild.0[3].contains("둘째"));
        assert!(!guild.0[3].contains("셋째"));
        assert_eq!(
            guild.0[5],
            " ★ 총 2명의 \x1b[1m【\x1b[36m청룡\x1b[37m】\x1b[0;37m파 무림인이 활동하고 있습니다."
        );
        clear_precomputed_all_online();
    }

    #[test]
    fn expression_command_matches_python_usage_room_guard_and_delivery_text() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player = format!("표현회귀-{suffix}");
        let zone = format!("표현회귀존-{suffix}");
        let room_dir = std::path::Path::new("data/map").join(&zone);
        let room_path = room_dir.join("1.json");
        std::fs::create_dir_all(&room_dir).unwrap();
        std::fs::write(
            &room_path,
            r#"{"맵정보":{"맵속성":["모든통신금지"],"이름":"표현시험방","존이름":"표현시험존","설명":[],"출구":[]}}"#,
        )
        .unwrap();
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room(&zone, "1").unwrap();
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player.as_str());

        let usage = storage
            .execute("표현", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [내용] 표현(')"]);
        let forbidden = storage
            .execute("표현", &mut body, "고개를 끄덕입니다.", None, None, None)
            .unwrap();
        assert_eq!(forbidden.0, vec!["☞ 이지역에서는 어떠한 통신도 불가능합니다."]);
        assert!(forbidden.1.is_none());

        get_world_state()
            .read()
            .unwrap()
            .room_cache
            .get_room_cached(&zone, "1")
            .unwrap()
            .write()
            .unwrap()
            .properties
            .clear();
        let delivered = storage
            .execute("표현", &mut body, "고개를 끄덕입니다.", None, None, None)
            .unwrap();
        assert!(matches!(
            delivered.1,
            Some(CommandResult::EmotionToRoom(ref own, ref room, None))
                if own == "당신이 고개를 끄덕입니다."
                    && room == &format!("{}{} 고개를 끄덕입니다.", player, han_iga(&player))
        ));

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player);
        let _ = std::fs::remove_dir_all(&room_dir);
    }

    #[test]
    fn notice_command_owns_python_usage_border_ansi_and_unbounded_width() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "공지회귀");

        let usage = storage
            .execute("공지말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 운영자 명령: [내용] 공지말"]);

        let notice = storage
            .execute("공지말", &mut body, "서버 점검", None, None, None)
            .unwrap();
        let expected = format!(
            "{}\r\n\x1b[7m☞ 공지 : {:<68}\x1b[0m\r\n{}",
            "─".repeat(39),
            "서버 점검",
            "─".repeat(39)
        );
        assert_eq!(notice.0, Vec::<String>::new());
        assert!(matches!(notice.1, Some(CommandResult::Notice(ref text)) if text == &expected));

        let long = "긴".repeat(201);
        let result = storage
            .execute("공지말", &mut body, &long, None, None, None)
            .unwrap();
        assert!(matches!(result.1, Some(CommandResult::Notice(ref text)) if text.contains(&long)));
    }

    #[test]
    fn notice_board_preserves_python_cat_whitespace_ansi_and_crlf_conversion() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "공지사항회귀");
        let source = std::fs::read_to_string("data/text/notice.txt").unwrap();
        let result = storage
            .execute("공지사항", &mut body, "무시되는 인자", None, None, None)
            .unwrap();
        assert_eq!(result.0, vec![source.replace('\n', "\r\n")]);
        assert!(result.0[0].starts_with("\x1b[H\x1b[2J┌"));
        assert!(result.0[0].contains("                              공  지  사  항"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fishing_delayed_callbacks_clear_python_cooltime_and_send_exact_lines() {
        use crate::network::{Broadcaster, Client};
        use crate::player::{Player, STATE_ACTIVE};
        use tokio::sync::mpsc;

        let storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::default()));
        let broadcaster = Arc::new(Broadcaster::new());
        let scheduler = Arc::new(CallOutScheduler::default_resolution(broadcaster.clone()));
        let mut command_body = Body::new();
        command_body.set("이름", "낚시예약회귀");
        let command = storage
            .read()
            .await
            .execute("낚시", &mut command_body, "무시", None, None, Some(scheduler.clone()))
            .unwrap();
        assert_eq!(
            command.0,
            vec!["낚시바늘에 미끼를 끼우고 낚시대를 드리웁니다."]
        );
        assert_eq!(scheduler.pending_count(), 2);

        let addr = "127.0.0.1:18054".parse().unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut client = Client::new(addr, tx);
        client.complete_login();
        let mut player = Player::new();
        player.state = STATE_ACTIVE;
        player.body.set("이름", "낚시회귀");
        player.body.set("cooltime", 9_i64);
        client.player = Some(player);
        broadcaster.add_client(client);

        let runner = create_call_out_script_runner(storage, broadcaster.clone());
        runner("낚시회귀", Some("낚시"), "fishing_2", vec![]).unwrap();
        assert_eq!(
            rx.try_recv().unwrap(),
            "낚시줄에 엄청난것이 걸린것 같다...\r\n"
        );
        assert_eq!(
            broadcaster
                .with_player_body_by_name("낚시회귀", |body| body.get_int("cooltime")),
            Some(0)
        );

        broadcaster.with_player_body_by_name("낚시회귀", |body| {
            body.set("cooltime", 7_i64);
        });
        runner("낚시회귀", Some("낚시"), "fishing_3", vec![]).unwrap();
        assert_eq!(rx.try_recv().unwrap(), "젠장! 낚시줄이 끊어졌다.\r\n");
        assert_eq!(
            broadcaster
                .with_player_body_by_name("낚시회귀", |body| body.get_int("cooltime")),
            Some(0)
        );
    }

    #[test]
    fn guild_position_command_moves_python_role_lists_and_emits_group_layout() {
        use crate::command::handler::CommandResult;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let actor = format!("직위방주-{suffix}");
        let target = format!("직위대상-{suffix}");
        let guild = format!("직위시험방파-{suffix}");
        let zone = format!("직위시험존-{suffix}");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        crate::world::guild::guild_set(&guild, "이름", &guild);
        crate::world::guild::guild_set(&guild, "방주명칭", "시험방주");
        crate::world::guild::guild_set(&guild, "방주리스트", &actor);
        crate::world::guild::guild_set(&guild, "방파인리스트", &target);
        get_world_state().write().unwrap().set_player_position(
            &actor,
            PlayerPosition::new(zone.clone(), "1".into()),
        );
        let online = [
            (&actor, "방주", "", 900_i64, 18_i64),
            (&target, "방파인", "", 700_i64, 12_i64),
        ]
        .into_iter()
        .map(|(name, position, config, hp, mp)| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name.clone()));
            map.insert("소속".into(), Dynamic::from(guild.clone()));
            map.insert("직위".into(), Dynamic::from(position));
            map.insert("설정상태".into(), Dynamic::from(config));
            map.insert("zone".into(), Dynamic::from(zone.clone()));
            map.insert("room".into(), Dynamic::from("1"));
            map.insert("현재체력".into(), Dynamic::from(hp));
            map.insert("최고체력".into(), Dynamic::from(hp));
            map.insert("현재내공".into(), Dynamic::from(mp));
            map.insert("최고내공".into(), Dynamic::from(mp));
            Dynamic::from(map)
        })
        .collect();
        set_precomputed_all_online(online);
        let mut body = Body::new();
        body.set("이름", actor.as_str());
        body.set("소속", guild.as_str());
        body.set("직위", "방주");
        let storage = ScriptStorage::default();

        let usage = storage
            .execute("직위임명", &mut body, "대상 제자", None, None, None)
            .unwrap();
        assert_eq!(
            usage.0,
            vec!["☞ 사용법 : [대상] [방주|부방주|장로|방파인] 직위임명"]
        );
        let same = storage
            .execute(
                "직위임명",
                &mut body,
                &format!("{target} 방파인"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(same.0, vec!["☞ 같은 직위입니다."]);
        crate::world::guild::guild_set(&guild, "장로리스트", "장로1\r\n장로2\r\n장로3\r\n장로4");
        let full = storage
            .execute(
                "직위임명",
                &mut body,
                &format!("{target} 장로"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(full.0, vec!["☞ 같은 직위의 인원이 너무 많습니다."]);
        assert_eq!(take_guild_position_request(&mut body), None);
        crate::world::guild::guild_set(&guild, "장로리스트", "");
        let changed = storage
            .execute(
                "직위임명",
                &mut body,
                &format!("{target} 장로"),
                None,
                None,
                None,
            )
            .unwrap();
        let actor_text = format!("\x1b[1m{actor}\x1b[0;37m{}", han_iga(&actor));
        let target_text = format!("\x1b[1m{target}\x1b[0;37m{}", han_eul(&target));
        let expected = format!(
            "\x1b[1m《\x1b[36m시험방주\x1b[37mː\x1b[36m{actor}\x1b[37m》\x1b[0;37m {actor_text} {target_text} \x1b[1m장로\x1b[0m로 직위를 임명합니다."
        );
        assert_eq!(changed.0, vec![expected.clone()]);
        let sends = match changed.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected result: {other:?}"),
        };
        assert_eq!(
            sends,
            vec![(
                target.clone(),
                format!("{expected}\r\n\x1b[0;37;40m[ 700/700, 12/12 ] ")
            )]
        );
        assert_eq!(
            take_guild_position_request(&mut body),
            Some((target.clone(), "장로".to_string()))
        );
        assert!(crate::world::guild::guild_kick_member(
            &guild, "장로", &target
        ));

        clear_precomputed_all_online();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&actor);
        crate::world::guild::guild_remove(&guild);
        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    }

    #[test]
    fn guild_chat_aliases_match_python_guard_order_layout_and_prompt() {
        use crate::command::handler::CommandResult;

        let suffix = std::process::id();
        let sender = "길동".to_string();
        let recipient = format!("방파말수신-{suffix}");
        let rejecting = format!("방파말거부-{suffix}");
        let transparent = format!("방파말투명-{suffix}");
        let guild = format!("방파말시험-{suffix}");
        let _ = std::fs::remove_file(format!("data/log/group/{guild}"));
        let mut body = Body::new();
        body.set("이름", sender.as_str());
        let storage = ScriptStorage::default();

        let no_guild = storage
            .execute("방파말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(no_guild.0, vec!["☞ 당신은 소속이 없습니다."]);
        body.set("소속", guild.as_str());
        body.set("직위", "장로");
        let usage = storage
            .execute("똥파말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법 : [내용] 방파말(])"]);

        body.set("설정상태", "방파말거부 1");
        let refused = storage
            .execute("방파말", &mut body, "기록되면 안됨", None, None, None)
            .unwrap();
        assert_eq!(refused.0, vec!["☞ 방파말 거부중 이에요. *^^*"]);
        assert!(!std::path::Path::new(&format!("data/log/group/{guild}")).exists());
        body.set("설정상태", "");

        let online = [
            (&sender, "", 0_i64, 900_i64, 18_i64),
            (&recipient, "", 0_i64, 700_i64, 12_i64),
            (&rejecting, "방파말거부 1", 0_i64, 600_i64, 11_i64),
            (&transparent, "", 1_i64, 500_i64, 10_i64),
        ]
            .into_iter()
            .map(|(name, config, hidden, hp, mp)| {
                let mut player = rhai::Map::new();
                player.insert("이름".into(), Dynamic::from(name.clone()));
                player.insert("소속".into(), Dynamic::from(guild.clone()));
                player.insert("설정상태".into(), Dynamic::from(config));
                player.insert("투명상태".into(), Dynamic::from(hidden));
                player.insert("현재체력".into(), Dynamic::from(hp));
                player.insert("최고체력".into(), Dynamic::from(hp));
                player.insert("현재내공".into(), Dynamic::from(mp));
                player.insert("최고내공".into(), Dynamic::from(mp));
                Dynamic::from(player)
            })
            .collect();
        set_precomputed_all_online(online);
        let sent = storage
            .execute("방파말", &mut body, "모두 안녕", None, None, None)
            .unwrap();
        let line = format!(
            "\x1b[1m《\x1b[36m장로\x1b[37mː\x1b[36m{sender}\x1b[37m》\x1b[0;37m 모두 안녕"
        );
        assert_eq!(sent.0, vec![line.clone()]);
        let sends = match sent.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected guild chat result: {other:?}"),
        };
        assert_eq!(
            sends,
            vec![
                (
                    recipient.clone(),
                    format!("{line}\r\n\x1b[0;37;40m[ 700/700, 12/12 ] ")
                ),
                (
                    transparent.clone(),
                    format!("{line}\r\n\x1b[0;37;40m[ 500/500, 10/10 ] ")
                )
            ]
        );
        let alias = storage
            .execute("똥파말", &mut body, "별칭 안녕", None, None, None)
            .unwrap();
        assert_eq!(alias.0.len(), 1);
        let log = std::fs::read_to_string(format!("data/log/group/{guild}")).unwrap();
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(regex::Regex::new(
            r"^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\] 길동        : 모두 안녕$"
        )
        .unwrap()
        .is_match(lines[0]), "{}", lines[0]);
        assert!(lines[1].ends_with("길동        : 별칭 안녕"));

        clear_precomputed_all_online();
        let _ = std::fs::remove_file(format!("data/log/group/{guild}"));
    }

    #[test]
    fn guild_list_matches_python_columns_count_leader_map_and_single_sendline() {
        let snapshot = std::fs::read("data/config/guild.json").unwrap_or_default();
        let suffix = std::process::id();
        let id = format!("리스트방파-{suffix}");
        guild_set(&id, "이름", "청풍회");
        guild_set(&id, "방주이름", "홍길동");
        guild_set(&id, "방파원수", "7");
        guild_set(&id, "방파맵", "방파맵:청풍회");
        let storage = ScriptStorage::default();
        let mut normal = Body::new();
        let shown = storage
            .execute("방파리스트", &mut normal, "", None, None, None)
            .unwrap();
        assert_eq!(shown.0.len(), 1, "Python builds one buffer and calls sendLine once");
        let expected_normal = format!("{:<12} : {:<30}   {:>3} 명", "[청풍회]", "홍길동", 7);
        assert!(shown.0[0].contains(&format!("{expected_normal}\r\n")));
        assert!(!shown.0[0].contains("방파맵:청풍회\r\n"));

        let mut admin = Body::new();
        admin.set("관리자등급", 1000_i64);
        let admin_shown = storage
            .execute("방파리스트", &mut admin, "", None, None, None)
            .unwrap();
        let expected_admin = format!("{expected_normal} 방파맵:청풍회\r\n");
        assert!(admin_shown.0[0].contains(&expected_admin));
        assert!(admin_shown.0[0].starts_with("━━━━━"));
        assert!(admin_shown.0[0].ends_with("━━━━━"));

        guild_remove(&id);
        let _ = std::fs::write("data/config/guild.json", snapshot);
    }

    #[test]
    fn guild_status_uses_legacy_roles_stored_count_three_columns_and_active_filter() {
        let snapshot = std::fs::read("data/config/guild.json").unwrap_or_default();
        let suffix = std::process::id();
        let guild = format!("상태방파-{suffix}");
        guild_set(&guild, "방주이름", "방주홍");
        guild_set(&guild, "부방주리스트", "부일\r\n부이");
        guild_set(&guild, "장로리스트", "장로하나");
        guild_set(&guild, "방파인리스트", "방파원갑\r\n방파원을");
        guild_set(&guild, "방파원수", "6");
        let online = [
            ("방주홍", guild.as_str(), 0_i64),
            ("부일", guild.as_str(), 0_i64),
            ("숨은방파원", guild.as_str(), 1_i64),
            ("타방파원", "다른방파", 0_i64),
        ]
        .into_iter()
        .map(|(name, affiliation, transparent)| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name));
            map.insert("소속".into(), Dynamic::from(affiliation.to_string()));
            map.insert("투명상태".into(), Dynamic::from(transparent));
            Dynamic::from(map)
        })
        .collect();
        set_precomputed_all_online(online);
        let mut body = Body::new();
        body.set("소속", guild.as_str());
        let shown = ScriptStorage::default()
            .execute("방파상태", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(shown.0.len(), 1);
        let text = &shown.0[0];
        assert!(text.contains("방주홍        "));
        assert!(text.contains("부일         "));
        assert!(text.contains("부이         \r\n"), "leader + two deputies complete Python's first row");
        assert!(text.contains("장로하나       "));
        assert!(text.contains("방파원갑       "));
        assert!(text.contains("방파원을       \r\n"));
        assert!(text.contains("방파총인원 : 6       ☞ 현재 2명이 활동중 입니다."));

        clear_precomputed_all_online();
        guild_remove(&guild);
        let _ = std::fs::write("data/config/guild.json", snapshot);
    }

    #[test]
    fn guild_acceptance_consumes_leader_applicant_and_adds_python_member_role() {
        use crate::world::{get_world_state, PlayerPosition};
        let snapshot = std::fs::read("data/config/guild.json").unwrap_or_default();
        let suffix = std::process::id();
        let leader = format!("입문방주-{suffix}");
        let target = format!("입문대상-{suffix}");
        let guild = format!("입문방파-{suffix}");
        let zone = format!("입문존-{suffix}");
        guild_set(&guild, "이름", &guild);
        guild_set(&guild, "방주이름", &leader);
        guild_set(&guild, "방파원수", "1");
        {
            let mut world = get_world_state().write().unwrap();
            for name in [&leader, &target] {
                world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
            }
        }
        let mut body = Body::new();
        body.set("이름", leader.as_str());
        body.set("직위", "방주");
        body.set("소속", guild.as_str());
        body.set("입문신청자", format!("다른신청자\r\n{target}"));
        let result = ScriptStorage::default()
            .execute("방파입문", &mut body, &target, None, None, None)
            .unwrap();
        assert_eq!(
            result.0,
            vec![format!(
                "당신이 \x1b[1m{target}\x1b[0;37m{} 방파에 입문시켰음을 선포합니다.",
                han_eul(&target)
            )]
        );
        assert!(matches!(
            result.1,
            Some(CommandResult::OutputAndSendToUsers(_, ref sends))
                if sends == &vec![(
                    target.clone(),
                    format!("\r\n\x1b[1m{leader}\x1b[0;37m{} 당신을 방파에 입문시켰음을 선포합니다.\r\n", han_iga(&leader))
                )]
        ));
        assert_eq!(body.get_string("입문신청자"), "다른신청자");
        assert_eq!(
            crate::world::guild::guild_role_members(&guild, "방파인"),
            vec![target.clone()]
        );
        assert_eq!(guild_get(&guild, "방파원수"), "2");
        assert_eq!(
            take_guild_accept_request(&mut body),
            Some((target.clone(), guild.clone()))
        );

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&leader);
        world.remove_player_position(&target);
        drop(world);
        guild_remove(&guild);
        let _ = std::fs::write("data/config/guild.json", snapshot);
    }

    #[test]
    fn guild_application_matches_python_personality_duplicate_and_output_rules() {
        use crate::command::handler::CommandResult;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let applicant = format!("신청자-{suffix}");
        let leader = format!("신청방주-{suffix}");
        let zone = format!("신청존-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            for name in [&applicant, &leader] {
                world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
            }
        }

        let snapshot = |applicants: &str| {
            let mut target = rhai::Map::new();
            target.insert("이름".into(), Dynamic::from(leader.clone()));
            target.insert("직위".into(), Dynamic::from("방주"));
            target.insert("성격".into(), Dynamic::from("기인"));
            target.insert("기존성격".into(), Dynamic::from("정파"));
            target.insert("입문신청자".into(), Dynamic::from(applicants.to_string()));
            HashMap::from([(format!("{zone}:1"), vec![Dynamic::from(target)])])
        };

        let mut body = Body::new();
        body.set("이름", applicant.as_str());
        body.set("성격", "정파");
        set_precomputed_room_view_players(snapshot("다른신청자"));
        let result = ScriptStorage::default()
            .execute("입문신청", &mut body, &leader, None, None, None)
            .unwrap();
        assert_eq!(
            result.0,
            vec![format!(
                "당신이 \x1b[1m{leader}\x1b[0;37m의 방파에 입문을 신청합니다."
            )]
        );
        assert!(matches!(
            result.1,
            Some(CommandResult::OutputAndSendToUsers(_, ref sends))
                if sends == &vec![(
                    leader.clone(),
                    format!("\x1b[1m{applicant}\x1b[0;37m{} 당신의 방파에 입문을 신청합니다.", han_iga(&applicant))
                )]
        ));
        assert_eq!(
            take_guild_apply_request(&mut body),
            Some((leader.clone(), applicant.clone()))
        );

        set_precomputed_room_view_players(snapshot(&format!("다른신청자\r\n{applicant}")));
        let duplicate = ScriptStorage::default()
            .execute("입문신청", &mut body, &leader, None, None, None)
            .unwrap();
        assert_eq!(duplicate.0, vec!["☞ 이미 입문 신청을 하였습니다."]);
        assert_eq!(take_guild_apply_request(&mut body), None);

        body.set("성격", "사파");
        set_precomputed_room_view_players(snapshot(""));
        let rejected = ScriptStorage::default()
            .execute("입문신청", &mut body, &leader, None, None, None)
            .unwrap();
        assert_eq!(rejected.0, vec!["☞ 방파에 입문 신청을 할 수 없습니다."]);
        assert_eq!(take_guild_apply_request(&mut body), None);

        clear_precomputed_room_view_players();
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&applicant);
        world.remove_player_position(&leader);
    }

    #[test]
    fn guild_room_name_and_description_use_room_owner_and_persist_python_fields() {
        use crate::command::handler::{CommandResult, PendingInput};
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player = format!("방파방주-{suffix}");
        let guild = format!("방파방-{suffix}");
        let zone = format!("방파방존-{suffix}");
        let room = "1";
        let dir = format!("data/map/{zone}");
        let path = format!("{dir}/{room}.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            &path,
            r#"{"맵정보":{"이름":"옛이름","설명":["옛설명"],"방파주인":"다른방파"}}"#,
        )
        .unwrap();
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), room.to_string()),
            );
            world
                .get_room_attrs_mut(&zone, room)
                .insert("방파주인".into(), "다른방파".into());
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("직위", "방주");
        body.set("소속", guild.as_str());
        let scripts = ScriptStorage::default();

        let denied = scripts
            .execute("방파방이름", &mut body, "새이름", None, None, None)
            .unwrap();
        assert_eq!(
            denied.0,
            vec!["☞ 무림인은 아무곳에나 이름을 새기지 않는다네."]
        );

        get_world_state()
            .write()
            .unwrap()
            .get_room_attrs_mut(&zone, room)
            .insert("방파주인".into(), guild.clone());
        let renamed = scripts
            .execute("방파방이름", &mut body, "새이름", None, None, None)
            .unwrap();
        assert_eq!(renamed.0, vec!["이름이 변경 되었습니다."]);
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["맵정보"]["이름"], "새이름");

        let description = scripts
            .execute("방파방설명", &mut body, "작성", None, None, None)
            .unwrap();
        assert!(matches!(
            description.1,
            Some(CommandResult::RequestInput {
                ref prompt,
                state: PendingInput::RoomDescription { ref zone, ref room, ref lines }
            }) if prompt == "방 설명 작성을 마치시려면 '.' 를 입력하세요.\r\n:"
                && zone == &format!("방파방존-{suffix}") && room == "1" && lines.is_empty()
        ));

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.room_attrs.remove(&format!("{zone}:{room}"));
        drop(world);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn guild_nickname_updates_online_target_and_excludes_leader_from_group_echo() {
        use crate::command::handler::CommandResult;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let leader = format!("별호방주-{suffix}");
        let target = format!("별호대상-{suffix}");
        let guild = format!("별호방파-{suffix}");
        let zone = format!("별호존-{suffix}");
        let target_path = format!("data/user/{target}.json");
        let mut saved = Body::new();
        saved.set("이름", target.as_str());
        saved.set("소속", "오래된별호소속");
        assert!(save_body_to_json(&mut saved, &target_path));
        get_world_state().write().unwrap().set_player_position(
            &leader,
            PlayerPosition::new(zone.clone(), "1".into()),
        );
        let online = [(&leader, "방주", 900_i64), (&target, "방파인", 700_i64)]
            .into_iter()
            .map(|(name, position, hp)| {
                let mut player = rhai::Map::new();
                player.insert("이름".into(), Dynamic::from(name.clone()));
                player.insert("소속".into(), Dynamic::from(guild.clone()));
                player.insert("직위".into(), Dynamic::from(position));
                player.insert("zone".into(), Dynamic::from(zone.clone()));
                player.insert("room".into(), Dynamic::from("1"));
                player.insert("설정상태".into(), Dynamic::from(""));
                player.insert("현재체력".into(), Dynamic::from(hp));
                player.insert("최고체력".into(), Dynamic::from(hp));
                player.insert("현재내공".into(), Dynamic::from(10_i64));
                player.insert("최고내공".into(), Dynamic::from(10_i64));
                Dynamic::from(player)
            })
            .collect();
        set_precomputed_all_online(online);
        let mut body = Body::new();
        body.set("이름", leader.as_str());
        body.set("소속", guild.as_str());
        body.set("직위", "방주");
        let result = ScriptStorage::default()
            .execute(
                "방파별호",
                &mut body,
                &format!("{target} 푸른별"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(result.0.len(), 1, "leader receives only the direct declaration");
        assert!(
            result.0[0].contains("『\x1b[1;32m푸른별\x1b[0;37m』"),
            "{:?}",
            result.0
        );
        let sends = match result.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected result: {other:?}"),
        };
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, target);
        assert!(!sends[0].1.starts_with("\r\n"));
        assert!(sends[0].1.contains(&format!("\x1b[1m{leader}\x1b[0;37m")));
        assert_eq!(
            take_guild_nickname_request(&mut body),
            Some((target.clone(), "푸른별".to_string()))
        );

        let self_result = ScriptStorage::default()
            .execute(
                "방파별호",
                &mut body,
                &format!("{leader} 청룡"),
                None,
                None,
                None,
            )
            .unwrap();
        assert!(self_result.0[0].contains("\x1b[1m자신\x1b[0;37m의 방파별호"));
        assert_eq!(body.get_string("방파별호"), "청룡");
        assert_eq!(take_guild_nickname_request(&mut body), None);
        let self_sends = match self_result.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected self nickname result: {other:?}"),
        };
        assert_eq!(self_sends.len(), 1);
        assert_eq!(self_sends[0].0, target);
        assert!(self_sends[0].1.contains("\x1b[1m자신\x1b[0;37m의 방파별호"));

        clear_precomputed_all_online();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&leader);
        let _ = std::fs::remove_file(target_path);
    }

    #[test]
    fn guild_expulsion_updates_current_json_roster_and_python_deliveries() {
        use crate::command::handler::CommandResult;

        let suffix = std::process::id();
        let leader = format!("파문방주-{suffix}");
        let target = format!("파문대상-{suffix}");
        let guild = format!("파문방파-{suffix}");
        let target_path = format!("data/user/{target}.json");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        crate::world::guild::guild_set(&guild, "이름", &guild);
        crate::world::guild::guild_set(&guild, "방주리스트", &leader);
        crate::world::guild::guild_set(&guild, "방파인리스트", &target);
        crate::world::guild::guild_set(&guild, "방파원수", "2");
        let mut saved = Body::new();
        saved.set("이름", target.as_str());
        // Python uses the connected Player object.  A stale save must not
        // override the live target's current guild/position.
        saved.set("소속", "오래된소속");
        saved.set("직위", "방파인");
        saved.set("방파별호", "옛별호");
        assert!(save_body_to_json(&mut saved, &target_path));
        let online = [(&leader, 900_i64), (&target, 700_i64)]
            .into_iter()
            .map(|(name, hp)| {
                let mut player = rhai::Map::new();
                player.insert("이름".into(), Dynamic::from(name.clone()));
                player.insert("소속".into(), Dynamic::from(guild.clone()));
                player.insert(
                    "직위".into(),
                    Dynamic::from(if name == &leader { "방주" } else { "방파인" }),
                );
                player.insert("설정상태".into(), Dynamic::from(""));
                player.insert("현재체력".into(), Dynamic::from(hp));
                player.insert("최고체력".into(), Dynamic::from(hp));
                player.insert("현재내공".into(), Dynamic::from(10_i64));
                player.insert("최고내공".into(), Dynamic::from(10_i64));
                Dynamic::from(player)
            })
            .collect();
        set_precomputed_all_online(online);
        let mut body = Body::new();
        body.set("이름", leader.as_str());
        body.set("소속", guild.as_str());
        body.set("직위", "방주");
        let result = ScriptStorage::default()
            .execute("방파파문", &mut body, &target, None, None, None)
            .unwrap();
        assert_eq!(result.0.len(), 1);
        assert!(result.0[0].contains("방파에서 파문시킴을 선포합니다."));
        let sends = match result.1.unwrap() {
            CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected expulsion result: {other:?}"),
        };
        assert_eq!(sends.len(), 1, "expelled target only receives its private notice");
        assert_eq!(sends[0].0, target);
        assert_eq!(
            sends[0].1,
            "당신은 파문되었습니다.\r\n\x1b[0;37;40m[ 700/700, 10/10 ] "
        );
        assert_eq!(take_guild_kick_request(&mut body), Some(target.clone()));
        let loaded: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&target_path).unwrap()).unwrap();
        assert_eq!(loaded["사용자오브젝트"]["소속"], "");
        assert_eq!(loaded["사용자오브젝트"]["직위"], "");
        assert!(loaded["사용자오브젝트"].get("방파별호").is_none());

        clear_precomputed_all_online();
        crate::world::guild::guild_remove(&guild);
        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
        let _ = std::fs::remove_file(target_path);
    }

    #[test]
    fn guild_reset_clears_flat_saved_members_and_queues_live_cleanup() {
        let suffix = std::process::id();
        let admin = format!("초기화관리자-{suffix}");
        let member = format!("초기화방파원-{suffix}");
        let guild = format!("초기화방파-{suffix}");
        let member_path = format!("data/user/{member}.json");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        crate::world::guild::guild_set(&guild, "이름", &guild);
        crate::world::guild::guild_set(&guild, "방파원수", "1");
        let mut saved = Body::new();
        saved.set("이름", member.as_str());
        saved.set("소속", guild.as_str());
        saved.set("직위", "방파인");
        assert!(save_body_to_json(&mut saved, &member_path));

        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let result = ScriptStorage::default()
            .execute("방파초기화", &mut body, &guild, None, None, None)
            .unwrap();
        assert_eq!(result.0, vec!["* 방파가 초기화되었습니다."]);
        assert!(!crate::world::guild::guild_has(&guild));
        assert_eq!(take_guild_reset_request(&mut body), Some(guild.clone()));
        let loaded: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&member_path).unwrap()).unwrap();
        assert_eq!(loaded["사용자오브젝트"]["소속"], "");
        assert_eq!(loaded["사용자오브젝트"]["직위"], "");

        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
        let _ = std::fs::remove_file(member_path);
    }

    #[test]
    fn specific_guild_reset_removes_only_requested_guild_despite_python_clear_all_bug() {
        let suffix = std::process::id();
        let selected = format!("특정초기화-{suffix}");
        let preserved = format!("보존방파-{suffix}");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        crate::world::guild::guild_set(&selected, "이름", &selected);
        crate::world::guild::guild_set(&preserved, "이름", &preserved);
        let mut body = Body::new();
        body.set("관리자등급", 2000_i64);

        let result = ScriptStorage::default()
            .execute(
                "특정방파초기화",
                &mut body,
                &selected,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(result.0, vec!["* 방파가 초기화되었습니다."]);
        assert!(!crate::world::guild::guild_has(&selected));
        assert!(crate::world::guild::guild_has(&preserved));
        assert_eq!(take_guild_reset_request(&mut body), Some(selected.clone()));

        crate::world::guild::guild_remove(&preserved);
        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    }

    #[test]
    fn guild_leader_transfer_requires_same_room_deputy_and_moves_python_roster() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let leader = format!("양도방주-{suffix}");
        let target = format!("양도부방주-{suffix}");
        let guild = format!("양도방파-{suffix}");
        let zone = format!("양도존-{suffix}");
        let target_path = format!("data/user/{target}.json");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        crate::world::guild::guild_set(&guild, "이름", &guild);
        crate::world::guild::guild_set(&guild, "방주이름", &leader);
        crate::world::guild::guild_set(&guild, "부방주리스트", &target);
        let mut saved = Body::new();
        saved.set("이름", target.as_str());
        saved.set("소속", "오래된양도소속");
        saved.set("직위", "방파인");
        saved.set("레벨", 1_i64);
        assert!(save_body_to_json(&mut saved, &target_path));
        get_world_state().write().unwrap().set_player_position(
            &leader,
            PlayerPosition::new(zone.clone(), "1".into()),
        );
        let online = |target_level: i64| [(&leader, "방주", 900_i64), (&target, "부방주", 700_i64)]
            .into_iter()
            .map(|(name, position, hp)| {
                let mut player = rhai::Map::new();
                player.insert("이름".into(), Dynamic::from(name.clone()));
                player.insert("소속".into(), Dynamic::from(guild.clone()));
                player.insert("직위".into(), Dynamic::from(position));
                player.insert(
                    "레벨".into(),
                    Dynamic::from(if name == &target { target_level } else { 999_999_i64 }),
                );
                player.insert("zone".into(), Dynamic::from(zone.clone()));
                player.insert("room".into(), Dynamic::from("1"));
                player.insert("설정상태".into(), Dynamic::from(""));
                player.insert("현재체력".into(), Dynamic::from(hp));
                player.insert("최고체력".into(), Dynamic::from(hp));
                player.insert("현재내공".into(), Dynamic::from(10_i64));
                player.insert("최고내공".into(), Dynamic::from(10_i64));
                Dynamic::from(player)
            })
            .collect();
        set_precomputed_all_online(online(499));
        let mut body = Body::new();
        body.set("이름", leader.as_str());
        body.set("소속", guild.as_str());
        body.set("직위", "방주");
        let low = ScriptStorage::default()
            .execute("방주권한양도", &mut body, &target, None, None, None)
            .unwrap();
        assert_eq!(low.0, vec!["☞ 방주가 되기에는 역량이 부족합니다."]);
        assert_eq!(body.get_string("직위"), "방주");
        assert_eq!(take_guild_transfer_request(&mut body), None);

        set_precomputed_all_online(online(999_999));
        let result = ScriptStorage::default()
            .execute("방주권한양도", &mut body, &target, None, None, None)
            .unwrap();
        assert_eq!(body.get_string("직위"), "부방주");
        assert_eq!(
            take_guild_transfer_request(&mut body),
            Some(target.clone())
        );
        assert_eq!(crate::world::guild::guild_get(&guild, "방주이름"), target);
        assert!(result.0[0].contains("방주로 권한이양을 선포합니다."));
        let sends = match result.1.unwrap() {
            crate::command::handler::CommandResult::OutputAndSendToUsers(_, sends) => sends,
            other => panic!("unexpected transfer result: {other:?}"),
        };
        assert_eq!(sends.len(), 2, "target receives obj.lpPrompt then sendGroup prompt");
        assert_eq!(sends[0].0, target);
        assert_eq!(
            sends[0].1,
            "\x1b[0;37;40m[ 700/700, 10/10 ] "
        );
        assert_eq!(sends[1].0, target);
        assert!(!sends[1].1.starts_with("\r\n"));
        assert!(sends[1].1.contains("방주로 권한이양을 선포합니다."));

        clear_precomputed_all_online();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&leader);
        crate::world::guild::guild_remove(&guild);
        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
        let _ = std::fs::remove_file(target_path);
    }

    #[test]
    fn guild_title_setting_persists_and_announces_with_new_sender_title() {
        let suffix = std::process::id();
        let leader = format!("명칭방주-{suffix}");
        let guild = format!("명칭방파-{suffix}");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        crate::world::guild::guild_set(&guild, "이름", &guild);
        crate::world::guild::guild_set(&guild, "방주명칭", "옛방주");
        let mut player = rhai::Map::new();
        player.insert("이름".into(), Dynamic::from(leader.clone()));
        player.insert("소속".into(), Dynamic::from(guild.clone()));
        player.insert("설정상태".into(), Dynamic::from(""));
        player.insert("현재체력".into(), Dynamic::from(900_i64));
        player.insert("최고체력".into(), Dynamic::from(900_i64));
        player.insert("현재내공".into(), Dynamic::from(10_i64));
        player.insert("최고내공".into(), Dynamic::from(10_i64));
        set_precomputed_all_online(vec![Dynamic::from(player)]);
        let mut body = Body::new();
        body.set("이름", leader.as_str());
        body.set("소속", guild.as_str());
        body.set("직위", "방주");
        let usage = ScriptStorage::default()
            .execute("명칭설정", &mut body, "제자 이름", None, None, None)
            .unwrap();
        assert_eq!(
            usage.0,
            vec!["☞ 사용법 : [방주|부방주|장로|방파인] [이름] 명칭설정"]
        );
        let result = ScriptStorage::default()
            .execute(
                "명칭설정",
                &mut body,
                "방주 대종사 무시되는말",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(crate::world::guild::guild_get(&guild, "방주명칭"), "대종사");
        assert_eq!(result.0.len(), 1);
        assert!(result.0[0].contains("《\x1b[36m대종사\x1b[37mː"));
        assert!(result.0[0].contains(&format!("\x1b[1m{leader}\x1b[0;37m")));
        assert!(result.0[0].contains("방주의 명칭을 \x1b[1m대종사\x1b[0;37m"));

        clear_precomputed_all_online();
        crate::world::guild::guild_remove(&guild);
        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
    }

    #[test]
    fn guild_signboard_matches_python_guard_order_schema_rooms_cost_item_and_notice() {
        use crate::command::handler::CommandResult;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player = format!("현판방주-{suffix}");
        let guild = format!("현판{suffix}");
        let zone = format!("현판존-{suffix}");
        let dir = format!("data/map/{zone}");
        let home_path = format!("{dir}/1.json");
        let entrance_path = format!("{dir}/2.json");
        let guild_file_before = std::fs::read("data/config/guild.json").unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            &home_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "맵정보": {"이름":"방파터", "방파자리":["가능"], "방파입구":["2"]}
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            &entrance_path,
            serde_json::to_string_pretty(&serde_json::json!({"맵정보":{"이름":"입구"}}))
                .unwrap(),
        )
        .unwrap();
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
        }
        let scripts = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("레벨", 399_i64);
        body.set("은전", 20_000_000_i64);

        get_world_state()
            .write()
            .unwrap()
            .get_room_attrs_mut(&zone, "1")
            .insert("방파주인".into(), "이미있는방파".into());
        let room_first = scripts
            .execute("현판걸어", &mut body, &guild, None, None, None)
            .unwrap();
        assert_eq!(room_first.0, vec!["☞ 이곳엔 현판을 걸 수 없습니다."]);
        get_world_state()
            .write()
            .unwrap()
            .get_room_attrs_mut(&zone, "1")
            .insert("방파주인".into(), "".into());

        let low = scripts
            .execute("현판걸어", &mut body, &guild, None, None, None)
            .unwrap();
        assert_eq!(low.0, vec!["☞ 당신은 방파를 세울 수 없습니다."]);
        body.set("레벨", 400_i64);
        body.set("방파금지", "금지");
        let banned = scripts
            .execute("현판걸어", &mut body, &guild, None, None, None)
            .unwrap();
        assert_eq!(banned.0, vec!["☞ 당신은 방파를 세울 수 없습니다."]);
        body.set("방파금지", "");
        body.set("은전", 9_999_999_i64);
        let poor = scripts
            .execute("현판걸어", &mut body, &guild, None, None, None)
            .unwrap();
        assert_eq!(
            poor.0,
            vec!["☞ 방파를 세우는데는 은전 10,000,000개 이상이 필요합니다."]
        );

        body.set("은전", 20_000_000_i64);
        body.set("성격", "정파");
        body.set("무림별호", "청협");
        let created = scripts
            .execute("현판걸어", &mut body, &guild, None, None, None)
            .unwrap();
        assert_eq!(
            created.0,
            vec!["당신이 현판을 세우는데 은전 10000000개를 사용합니다."]
        );
        assert_eq!(body.get_string("소속"), guild);
        assert_eq!(body.get_string("직위"), "방주");
        assert_eq!(body.get_int("은전"), 10_000_000);
        assert!(body.object.objs.iter().any(|item| {
            item.lock()
                .ok()
                .is_some_and(|item| item.getName() == "보관함")
        }));
        assert_eq!(crate::world::guild::guild_get(&guild, "방주이름"), player);
        assert_eq!(crate::world::guild::guild_get(&guild, "방파원수"), "1");
        assert_eq!(
            crate::world::guild::guild_get(&guild, "방파맵"),
            format!("{zone}:1")
        );
        for key in ["방주명칭", "부방주명칭", "장로명칭", "방파인명칭"] {
            assert!(!crate::world::guild::guild_get(&guild, key).is_empty());
        }
        for path in [&home_path, &entrance_path] {
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert_eq!(json["맵정보"]["방파주인"], guild);
        }
        let notice = match created.1.unwrap() {
            CommandResult::Notice(text) => text,
            other => panic!("unexpected guild creation result: {other:?}"),
        };
        assert!(notice.starts_with("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n"));
        assert!(notice.contains(&format!(
            "[\x1b[1;32m청협\x1b[0;37m] \x1b[1;36m{player}\x1b[37m{} 방파 『{guild}』{} 창설했습니다.\x1b[0m",
            han_iga(&player),
            han_eul(&guild)
        )));
        assert!(notice.ends_with("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));

        crate::world::guild::guild_remove(&guild);
        std::fs::write("data/config/guild.json", guild_file_before).unwrap();
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.room_attrs.remove(&format!("{zone}:1"));
        world.room_attrs.remove(&format!("{zone}:2"));
        drop(world);
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(format!("data/user/{player}.json"));
    }

    #[test]
    fn mob_find_command_searches_ordered_templates_by_name_or_nonzero_type() {
        use crate::world::{get_world_state, RawMobData};

        let suffix = std::process::id();
        let zone = format!("몹찾기시험존-{suffix}");
        let first_key = format!("{zone}:첫몹");
        let second_key = format!("{zone}:둘째몹");
        let mut first = RawMobData::new();
        first.name = "검색산적대장".into();
        first.mob_type = 6;
        first.attributes.insert(
            "위치".into(),
            serde_json::json!(["11", "20-23"]),
        );
        let mut second = RawMobData::new();
        second.name = "검색산적졸개".into();
        second.mob_type = 6;
        second
            .attributes
            .insert("위치".into(), serde_json::json!(["12"]));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(first_key.clone(), first);
            world.mob_cache.insert_mob_data(second_key.clone(), second);
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("관리자등급", 1000_i64);

        let by_name = storage
            .execute("몹찾기", &mut body, "산적대", None, None, None)
            .unwrap();
        assert_eq!(
            by_name.0,
            vec!["\x1b[33m검색산적대장\x1b[37m(첫몹) : ['11', '20-23']"]
        );
        let by_type = storage
            .execute("몹찾기", &mut body, "6종", None, None, None)
            .unwrap();
        assert_eq!(
            by_type.0,
            vec![
                "\x1b[33m검색산적대장\x1b[37m(첫몹) : ['11', '20-23']",
                "\x1b[33m검색산적졸개\x1b[37m(둘째몹) : ['12']",
            ]
        );
        let missing = storage
            .execute("몹찾기", &mut body, "절대없는검색어", None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["☞ 찾으시는 몹이 없습니다."]);

        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&first_key);
        world.mob_cache.remove_mob(&second_key);
    }

    #[test]
    fn item_find_command_uses_python_template_name_ansi_and_failure_text() {
        let mut body = Body::new();
        body.set("관리자등급", 1000_i64);
        let storage = ScriptStorage::default();
        let found = storage
            .execute("아이템찾기", &mut body, "간장검", None, None, None)
            .unwrap();
        assert!(
            found
                .0
                .iter()
                .any(|line| line == "\x1b[0;36m간장검\x1b[37m : 77"),
            "{:?}",
            found.0
        );
        let missing = storage
            .execute(
                "아이템찾기",
                &mut body,
                "절대없는아이템검색어",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(missing.0, vec!["☞ 찾으시는 아이템이 없습니다."]);

        let armor = storage
            .execute("방어구찾기", &mut body, "무시됨", None, None, None)
            .unwrap();
        assert!(
            armor
                .0
                .iter()
                .any(|line| line == "\x1b[0;36m비단머리띠\x1b[37m : 423"),
            "{:?}",
            armor.0
        );
    }

    #[test]
    fn admin_front_variants_keep_summon_and_silent_direct_move_distinct() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let admin = format!("앞관리자-{suffix}");
        let target = format!("앞대상-{suffix}");
        let zone = format!("앞시험존-{suffix}");
        let directory = std::path::Path::new("data/map").join(&zone);
        std::fs::create_dir_all(&directory).unwrap();
        for (room, name) in [("1", "출발방"), ("2", "도착방")] {
            std::fs::write(
                directory.join(format!("{room}.json")),
                serde_json::json!({"맵정보":{"이름":name,"존이름":zone,"설명":[],"출구":[]}})
                    .to_string(),
            )
            .unwrap();
        }
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room(&zone, "1").unwrap();
            world.room_cache.get_room(&zone, "2").unwrap();
            world.set_player_position(
                &admin,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
            world.set_player_position(
                &target,
                PlayerPosition::new(zone.clone(), "2".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();
        let direct = storage
            .execute("앞앞", &mut body, &target, None, None, None)
            .unwrap();
        assert!(direct.0.is_empty(), "Python 앞앞 directly inserts with no room output");
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .get_player_position(&admin)
                .unwrap()
                .room,
            "2"
        );

        get_world_state().write().unwrap().set_player_position(
            &admin,
            PlayerPosition::new(zone.clone(), "1".into()),
        );
        let summoned = storage
            .execute("앞", &mut body, &target, None, None, None)
            .unwrap();
        assert!(summoned.0[0].contains("알수 없는 기운에 휘말려 사라집니다."));
        assert!(summoned.0.iter().any(|line| line.contains("도착방")));

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&admin);
        world.remove_player_position(&target);
        drop(world);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn summon_command_queues_target_side_move_without_invented_admin_success_text() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let admin = format!("소환관리자-{suffix}");
        let target = format!("소환대상-{suffix}");
        let zone = format!("소환요청존-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &admin,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
            world.set_player_position(
                &target,
                PlayerPosition::new(zone.clone(), "2".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let result = ScriptStorage::default()
            .execute("소환", &mut body, &target, None, None, None)
            .unwrap();
        assert!(result.0.is_empty());
        assert_eq!(
            take_summon_player_request(&mut body),
            vec![(target.clone(), zone.clone(), "1".to_string())]
        );
        let world = get_world_state().read().unwrap();
        assert_eq!(
            world.get_player_position(&target).unwrap().room,
            "2",
            "admin-side script must not move the target before target-side Rhai runs"
        );
        drop(world);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&admin);
        world.remove_player_position(&target);
    }

    #[test]
    fn summon_all_reports_each_same_room_player_and_queues_each_remote_player() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let admin = format!("모두소환관리자-{suffix}");
        let same = format!("모두소환동실-{suffix}");
        let remote = format!("모두소환원격-{suffix}");
        let zone = format!("모두소환존-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            for name in [&admin, &same] {
                world.set_player_position(
                    name,
                    PlayerPosition::new(zone.clone(), "1".into()),
                );
            }
            world.set_player_position(
                &remote,
                PlayerPosition::new(zone.clone(), "2".into()),
            );
        }
        let online = [&admin, &same, &remote]
            .into_iter()
            .map(|name| {
                let mut player = rhai::Map::new();
                player.insert("이름".into(), Dynamic::from(name.clone()));
                Dynamic::from(player)
            })
            .collect();
        set_precomputed_all_online(online);
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let result = ScriptStorage::default()
            .execute("모두소환", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            result.0,
            vec!["☞ 같은 곳이에요. ^^", "☞ 같은 곳이에요. ^^"]
        );
        assert_eq!(
            take_summon_player_request(&mut body),
            vec![(remote.clone(), zone.clone(), "1".to_string())]
        );

        clear_precomputed_all_online();
        let mut world = get_world_state().write().unwrap();
        for name in [&admin, &same, &remote] {
            world.remove_player_position(name);
        }
    }

    #[test]
    fn force_command_queues_real_target_reentry_and_has_no_admin_success_text() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let admin = format!("명령관리자-{suffix}");
        let target = format!("명령대상-{suffix}");
        let zone = format!("명령시험존-{suffix}");
        let mob_key = format!("{zone}:명령몹");
        let mut mob_data = RawMobData::new();
        mob_data.name = "명령몹".into();
        mob_data.zone = zone.clone();
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(mob_key.clone(), mob_data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                "1",
                &mob_data,
            ));
            world.set_player_position(
                &admin,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
            world.set_player_position(
                &target,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let result = ScriptStorage::default()
            .execute("명령", &mut body, &format!("{target} 점수"), None, None, None)
            .unwrap();
        assert!(result.0.is_empty());
        assert_eq!(
            take_force_command_request(&mut body),
            vec![(target.clone(), "점수".to_string())]
        );
        let mob_target = ScriptStorage::default()
            .execute("명령", &mut body, "명령몹 점수", None, None, None)
            .unwrap();
        assert_eq!(
            mob_target.0,
            vec!["☞ 그런 대상이 없어요. *^_^*"],
            "Python rejects non-player room objects with the generic target message"
        );
        assert!(take_force_command_request(&mut body).is_empty());
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&admin);
        world.remove_player_position(&target);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn admin_command_list_matches_python_level_filter_and_three_column_layout() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "명령어리스트검사");
        body.set("관리자등급", 999_i64);
        let denied = storage
            .execute("명령어리스트", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

        body.set("관리자등급", 1000_i64);
        let listed = storage
            .execute("명령어리스트", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(listed.0.len(), 1);
        let output = &listed.0[0];
        let mut expected_names = Vec::new();
        for entry in std::fs::read_dir("cmds").unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("py") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            if source.lines().any(|line| {
                let code = line.trim_start();
                !code.starts_with('#')
                    && (code.contains("level = 1000") || code.contains("level=1000"))
            }) {
                expected_names.push(path.file_stem().unwrap().to_string_lossy().to_string());
            }
        }
        let mut expected = String::new();
        for (index, name) in expected_names.iter().enumerate() {
            expected.push_str(&format!("{:>20}", name));
            if (index + 1) % 3 == 0 {
                expected.push_str("\r\n");
            }
        }
        assert_eq!(output, &expected);
    }

    #[test]
    fn mob_delete_removes_python_template_but_keeps_existing_room_clone() {
        use crate::world::{get_world_state, MobInstance, RawMobData};

        let suffix = std::process::id();
        let zone = format!("몹삭제존-{suffix}");
        let key = format!("{zone}:삭제시험몹");
        let mut data = RawMobData::new();
        data.name = "삭제시험몹".into();
        data.zone = zone.clone();
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                key.clone(),
                zone.clone(),
                "1",
                &data,
            ));
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("관리자등급", 2000_i64);
        let usage = storage
            .execute("몹삭제", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["사용법: [몹 인덱스] 몹삭제"]);
        let deleted = storage
            .execute("몹삭제", &mut body, &key, None, None, None)
            .unwrap();
        assert_eq!(deleted.0, vec!["몹이 삭제되었습니다."]);
        {
            let world = get_world_state().read().unwrap();
            assert!(world.mob_cache.get_mob(&key).is_none());
            assert_eq!(
                world.mob_cache.get_all_mobs_in_room(&zone, "1").len(),
                1,
                "Python room clone survives deletion from Mob.Mobs"
            );
        }
        let missing = storage
            .execute("몹삭제", &mut body, &key, None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["존재하지않는 몹입니다."]);

        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&key);
    }

    #[test]
    fn mob_editor_uses_python_whitespace_split_and_usage_before_permission() {
        use crate::command::handler::{CommandResult, PendingInput};

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("관리자등급", 0_i64);
        let whitespace = storage
            .execute("몹제작", &mut body, "   ", None, None, None)
            .unwrap();
        assert_eq!(whitespace.0, vec!["☞ 사용법: [존이름] [몹이름] 몹제작"]);

        let denied = storage
            .execute("몹제작", &mut body, "시험존 시험몹", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^"]);

        body.set("관리자등급", 1000_i64);
        let editor = storage
            .execute(
                "몹제작",
                &mut body,
                "  시험존\t시험몹   뒤의값은무시 ",
                None,
                None,
                None,
            )
            .unwrap();
        assert!(matches!(
            editor.1,
            Some(CommandResult::RequestInput {
                ref prompt,
                state: PendingInput::FileEdit { ref relative_path, ref lines }
            }) if prompt == "작성을 마치시려면 '.' 를 입력하세요.\r\n:"
                && relative_path == "mob/시험존/시험몹.json"
                && lines.is_empty()
        ));
    }

    #[test]
    fn room_editor_and_delete_match_python_whitespace_path_and_memory_only_removal() {
        use crate::command::handler::{CommandResult, PendingInput};
        use crate::world::get_world_state;

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("관리자등급", 0_i64);
        let usage = storage
            .execute("방제작", &mut body, "\t  ", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [존이름] [방이름] 방제작"]);
        let denied = storage
            .execute("방제작", &mut body, "시험존 시험방", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^"]);

        body.set("관리자등급", 1000_i64);
        let editor = storage
            .execute(
                "방제작",
                &mut body,
                "  시험존\t시험방   추가단어 ",
                None,
                None,
                None,
            )
            .unwrap();
        assert!(matches!(
            editor.1,
            Some(CommandResult::RequestInput {
                ref prompt,
                state: PendingInput::FileEdit { ref relative_path, ref lines }
            }) if prompt == "작성을 마치시려면 '.' 를 입력하세요.\r\n:"
                && relative_path == "map/시험존/시험방.json"
                && lines.is_empty()
        ));

        let suffix = std::process::id();
        let zone = format!("방제거존-{suffix}가");
        let dir = std::path::Path::new("data/map").join(&zone);
        let path = dir.join("1.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            &path,
            format!(r#"{{"맵정보":{{"이름":"삭제방","존이름":"{zone}","설명":[],"출구":[]}}}}"#),
        )
        .unwrap();
        get_world_state()
            .write()
            .unwrap()
            .room_cache
            .get_room(&zone, "1")
            .unwrap();
        body.set("관리자등급", 2000_i64);
        let deleted = storage
            .execute(
                "방제거",
                &mut body,
                &format!("{zone}:1"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(deleted.0, vec!["방이 제거되었습니다."]);
        assert!(path.exists(), "Python deletes only Room.Zones entry");
        assert!(get_world_state()
            .read()
            .unwrap()
            .room_cache
            .get_room_cached(&zone, "1")
            .is_none());
        let repeated = storage
            .execute(
                "방제거",
                &mut body,
                &format!("{zone}:1"),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(repeated.0, vec!["존재하지않는 방입니다."]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn item_editor_and_delete_match_python_runtime_registry_semantics() {
        use crate::command::handler::{CommandResult, PendingInput};

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("관리자등급", 0_i64);
        let usage = storage
            .execute("아이템제작", &mut body, "   ", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [파일명] 아이템제작"]);
        let denied = storage
            .execute("아이템제작", &mut body, "시험파일", None, None, None)
            .unwrap();
        assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^"]);
        body.set("관리자등급", 1000_i64);
        let editor = storage
            .execute(
                "아이템제작",
                &mut body,
                "  시험파일\t뒤의값은무시 ",
                None,
                None,
                None,
            )
            .unwrap();
        assert!(matches!(
            editor.1,
            Some(CommandResult::RequestInput {
                ref prompt,
                state: PendingInput::FileEdit { ref relative_path, ref lines }
            }) if prompt == "작성을 마치시려면 '.' 를 입력하세요.\r\n:"
                && relative_path == "item/시험파일.json" && lines.is_empty()
        ));

        let key = format!("아이템삭제회귀-{}", std::process::id());
        let path = std::path::Path::new("data/item").join(format!("{key}.json"));
        std::fs::write(
            &path,
            serde_json::json!({"아이템정보":{"이름":"삭제시험품","종류":"기타","반응이름":["삭제시험품"]}})
                .to_string(),
        )
        .unwrap();
        get_world_state()
            .write()
            .unwrap()
            .item_cache
            .load_item(&key)
            .unwrap();
        let existing = object_from_item_json(&key).unwrap().0;
        body.object.objs.push(existing.clone());
        body.set("관리자등급", 2000_i64);
        let deleted = storage
            .execute("아이템삭제", &mut body, &key, None, None, None)
            .unwrap();
        assert_eq!(deleted.0, vec!["아이템이 삭제되었습니다."]);
        assert!(path.exists());
        assert_eq!(existing.lock().unwrap().getName(), "삭제시험품");
        let cannot_recreate = storage
            .execute("생성", &mut body, &key, None, None, None)
            .unwrap();
        assert_eq!(cannot_recreate.0, vec!["* 생성 실패!!!"]);
        assert_eq!(body.object.objs.len(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn finish_all_queues_every_other_active_player_in_python_connection_order() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let admin = format!("모두끝관리자-{suffix}");
        let first = format!("모두끝첫째-{suffix}");
        let second = format!("모두끝둘째-{suffix}");
        let zone = format!("모두끝존-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            for name in [&admin, &first, &second] {
                world.set_player_position(
                    name,
                    PlayerPosition::new(zone.clone(), "1".into()),
                );
            }
        }
        let online = [&first, &admin, &second]
            .into_iter()
            .map(|name| {
                let mut map = rhai::Map::new();
                map.insert("이름".into(), Dynamic::from(name.to_string()));
                Dynamic::from(map)
            })
            .collect();
        set_precomputed_all_online(online);
        let mut body = Body::new();
        body.set("이름", admin.as_str());
        body.set("관리자등급", 2000_i64);
        let result = ScriptStorage::default()
            .execute("모두끝", &mut body, "무시", None, None, None)
            .unwrap();
        assert!(result.0.is_empty());
        assert_eq!(
            take_force_command_request(&mut body),
            vec![(first.clone(), "끝".into()), (second.clone(), "끝".into())]
        );
        clear_precomputed_all_online();
        let mut world = get_world_state().write().unwrap();
        for name in [&admin, &first, &second] {
            world.remove_player_position(name);
        }
    }

    #[test]
    fn oneitem_cleanup_variants_preserve_python_storage_delete_difference() {
        let _oneitem_guard = ONEITEM_COMMAND_TEST_LOCK.lock().unwrap();
        let attr_before = std::fs::read("data/config/oneitem.json").unwrap();
        assert!(crate::oneitem::oneitem_have(
            "77",
            "기연소유자 보관 추가"
        ));
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("관리자등급", 2000_i64);

        set_precomputed_connected_names(vec![Dynamic::from("기연소유자")]);
        let online = storage
            .execute("기연정리", &mut body, "간장검", None, None, None)
            .unwrap();
        assert_eq!(online.0, vec!["사용자가 접속중입니다.!"]);
        assert_eq!(crate::oneitem::oneitem_get("77"), "기연소유자 보관 추가");

        set_precomputed_connected_names(Vec::new());
        let retained = storage
            .execute("기연정리", &mut body, "간장검", None, None, None)
            .unwrap();
        assert_eq!(retained.0, vec!["기연소유자 보관"]);
        assert_eq!(crate::oneitem::oneitem_get("77"), "기연소유자 보관 추가");

        let removed = storage
            .execute("기연정리리", &mut body, "간장검", None, None, None)
            .unwrap();
        assert_eq!(removed.0, vec!["기연소유자 보관"]);
        assert_eq!(crate::oneitem::oneitem_get("77"), "");

        clear_precomputed_all_online();
        std::fs::write("data/config/oneitem.json", attr_before).unwrap();
    }

    #[test]
    fn oneitem_admin_commands_cover_offline_age_inventory_delete_and_empty_list() {
        let _oneitem_guard = ONEITEM_COMMAND_TEST_LOCK.lock().unwrap();
        use crate::object::Object;

        let attr_before = std::fs::read("data/config/oneitem.json").unwrap();
        let index_before = std::fs::read("data/config/oneitem_index.json").unwrap();
        assert!(crate::oneitem::oneitem_clear());
        let suffix = std::process::id();
        let owner = format!("기연오프라인-{suffix}");
        let owner_path = format!("data/user/{owner}.json");
        let mut item = Object::new();
        item.set("이름", "간장검");
        item.set("인덱스", "77");
        let item = Arc::new(Mutex::new(item));
        let mut saved = Body::new();
        saved.set("이름", owner.as_str());
        let old_timestamp = chrono::Utc::now().timestamp() - 259_201;
        saved.set("마지막저장시간", old_timestamp);
        saved.object.append(item);
        assert!(save_body_to_json_without_timestamp(&mut saved, &owner_path));

        let mut admin = Body::new();
        admin.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();
        set_precomputed_connected_names(Vec::new());

        assert!(crate::oneitem::oneitem_have("77", &owner));
        let cleaned = storage
            .execute("기연정리", &mut admin, "간장검", None, None, None)
            .unwrap();
        assert_eq!(
            cleaned.0,
            vec![format!("{owner}의 간장검{} 정리하였습니다.", han_eul("간장검"))]
        );
        assert_eq!(crate::oneitem::oneitem_get("77"), "");
        let mut loaded = Body::new();
        assert!(load_body_from_json(&mut loaded, &owner_path));
        assert!(loaded
            .object
            .objs
            .iter()
            .all(|item| item.lock().unwrap().getString("인덱스") != "77"));
        assert_eq!(loaded.get_int("마지막저장시간"), old_timestamp);

        saved.object.objs.clear();
        let mut recent_item = Object::new();
        recent_item.set("이름", "간장검");
        recent_item.set("인덱스", "77");
        saved.object.append(Arc::new(Mutex::new(recent_item)));
        saved.set("마지막저장시간", chrono::Utc::now().timestamp());
        assert!(save_body_to_json_without_timestamp(&mut saved, &owner_path));
        assert!(crate::oneitem::oneitem_have("77", &owner));
        let recent = storage
            .execute("기연정리", &mut admin, "간장검", None, None, None)
            .unwrap();
        assert_eq!(recent.0, vec!["아직 3일이 경과하지 않았습니다."]);
        assert_eq!(crate::oneitem::oneitem_get("77"), owner);

        assert!(crate::oneitem::oneitem_destroy("77"));
        let absent_delete = storage
            .execute("기연삭제", &mut admin, "간장검", None, None, None)
            .unwrap();
        assert_eq!(absent_delete.0, vec!["☞ 그런 아이템은 없습니다.!"]);

        assert!(crate::oneitem::oneitem_have("77", "이름  상태   추가"));
        let spaced_owner = storage
            .execute("기연정리", &mut admin, "간장검", None, None, None)
            .unwrap();
        assert_eq!(spaced_owner.0, vec!["이름 상태"]);
        assert_eq!(crate::oneitem::oneitem_get("77"), "이름  상태   추가");
        assert!(crate::oneitem::oneitem_destroy("77"));
        assert!(crate::oneitem::oneitem_have("77", "하나 둘 셋 넷"));
        let malformed = storage
            .execute("기연정리", &mut admin, "간장검", None, None, None)
            .unwrap();
        assert_eq!(malformed.0, vec!["아무도 소지하고 있지 않습니다.!"]);

        let initialized = storage
            .execute("기연초기화", &mut admin, "무시", None, None, None)
            .unwrap();
        assert_eq!(initialized.0, vec!["* 기연아이템 목록이 초기화되었습니다."]);
        let empty = storage
            .execute("기연", &mut admin, "", None, None, None)
            .unwrap();
        assert_eq!(empty.0, vec![""]);

        set_precomputed_connected_names(Vec::new());
        let _ = std::fs::remove_file(owner_path);
        std::fs::write("data/config/oneitem.json", attr_before).unwrap();
        std::fs::write("data/config/oneitem_index.json", index_before).unwrap();
        assert!(crate::oneitem::oneitem_reload());
    }

    #[test]
    fn head_and_tail_commands_match_python_character_limit_state_and_room_snapshot() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "꼬리말시험");

        let usage = storage
            .execute("꼬리말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [내용] 꼬리말"]);
        assert_eq!(body.get_string("꼬리말"), "");

        let twenty = "가".repeat(20);
        let accepted = storage
            .execute("꼬리말", &mut body, &twenty, None, None, None)
            .unwrap();
        assert_eq!(accepted.0, vec!["☞ 꼬리말을 설정 하였습니다."]);
        assert_eq!(body.get_string("꼬리말"), twenty);
        let snapshot = build_room_view_player_snapshot(&body)
            .try_cast::<rhai::Map>()
            .unwrap();
        assert_eq!(snapshot["tail"].clone().into_string().unwrap(), twenty);

        let rejected = storage
            .execute("꼬리말", &mut body, &"나".repeat(21), None, None, None)
            .unwrap();
        assert_eq!(rejected.0, vec!["☞ 너무 깁니다."]);
        assert_eq!(body.get_string("꼬리말"), twenty);

        let ansi = "\x1b[31m붉음\x1b[0m";
        let colored = storage
            .execute("꼬리말", &mut body, ansi, None, None, None)
            .unwrap();
        assert_eq!(colored.0, vec!["☞ 꼬리말을 설정 하였습니다."]);
        assert_eq!(body.get_string("꼬리말"), ansi);

        let removed = storage
            .execute("꼬리말제거", &mut body, "무시되는 입력", None, None, None)
            .unwrap();
        assert_eq!(removed.0, vec!["☞ 꼬리말을 제거 하였습니다."]);
        assert_eq!(body.get_string("꼬리말"), "");

        let head = storage
            .execute("머리말", &mut body, &"앞".repeat(10), None, None, None)
            .unwrap();
        assert_eq!(head.0, vec!["☞ 머리말을 설정 하였습니다."]);
        assert_eq!(body.get_string("머리말"), "앞".repeat(10));
        let head_too_long = storage
            .execute("머리말", &mut body, &"머".repeat(21), None, None, None)
            .unwrap();
        assert_eq!(head_too_long.0, vec!["☞ 너무 깁니다."]);
        assert_eq!(body.get_string("머리말"), "앞".repeat(10));
        let snapshot = build_room_view_player_snapshot(&body)
            .try_cast::<rhai::Map>()
            .unwrap();
        assert_eq!(
            snapshot["head"].clone().into_string().unwrap(),
            "앞".repeat(10)
        );
        let head_removed = storage
            .execute("머리말제거", &mut body, "무시", None, None, None)
            .unwrap();
        assert_eq!(head_removed.0, vec!["☞ 머리말을 제거 하였습니다."]);
        assert_eq!(body.get_string("머리말"), "");
    }

    #[test]
    fn lower_trait_command_distinguishes_missing_from_explicit_zero_like_python() {
        let suffix = std::process::id();
        let name = format!("특성내려-{suffix}");
        let path = format!("data/user/{name}.json");
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", name.as_str());
        body.set("힘", 5_i64);

        let usage = storage
            .execute("내려", &mut body, "잘못", None, None, None)
            .unwrap();
        assert_eq!(
            usage.0,
            vec!["☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려"]
        );
        let ordinary_missing = storage
            .execute("내려", &mut body, "힘", None, None, None)
            .unwrap();
        assert_eq!(ordinary_missing.0, vec!["☞ [힘] 더이상 내릴 수 없습니다."]);
        assert_eq!(body.get_int("힘"), 5);

        body.set("힘특성치", 2_i64);
        let strength = storage
            .execute("내려", &mut body, "힘", None, None, None)
            .unwrap();
        assert_eq!(strength.0, vec!["☞ [힘] 특성치를 내렸습니다."]);
        assert_eq!(body.get_int("힘특성치"), 1);
        assert_eq!(body.get_int("힘"), 4);
        assert_eq!(body.get_int("특성치"), 1);
        assert!(body.get_int("마지막저장시간") > 0);
        assert!(std::path::Path::new(&path).exists());

        body.set("명중", 5_i64);
        body.set("명중특성치", 0_i64);
        let explicit_zero = storage
            .execute("내려", &mut body, "명중", None, None, None)
            .unwrap();
        assert_eq!(explicit_zero.0, vec!["☞ [명중] 더이상 내릴 수 없습니다."]);
        assert_eq!(body.get_int("명중"), 5);
        body.object.attr.remove("명중특성치");
        let legacy_fallback = storage
            .execute("내려", &mut body, "명중", None, None, None)
            .unwrap();
        assert_eq!(legacy_fallback.0, vec!["☞ [명중] 특성치를 내렸습니다."]);
        assert_eq!(body.get_int("명중특성치"), 4);
        assert_eq!(body.get_int("명중"), 4);
        assert_eq!(body.get_int("특성치"), 2);

        body.set("내공특성치", 1_i64);
        body.set("최고내공", 50_i64);
        let mp = storage
            .execute("내려", &mut body, "내공", None, None, None)
            .unwrap();
        assert_eq!(mp.0, vec!["☞ [내공] 특성치를 내렸습니다."]);
        assert_eq!(body.get_int("최고내공"), 40);
        body.set("체력특성치", 1_i64);
        body.set("최고체력", 500_i64);
        let hp = storage
            .execute("내려", &mut body, "체력", None, None, None)
            .unwrap();
        assert_eq!(hp.0, vec!["☞ [체력] 특성치를 내렸습니다."]);
        assert_eq!(body.get_int("최고체력"), 400);
        assert_eq!(body.get_int("특성치"), 4);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn user_mob_commands_create_and_remove_socketless_player_objects() {
        use crate::world::{get_world_state, PlayerPosition, RoomObjectRef};

        let suffix = std::process::id();
        let admin_name = format!("사용자몹관리자-{suffix}");
        let summoned_name = format!("사용자몹대상-{suffix}");
        let loaded_name = format!("사용자몹실제이름-{suffix}");
        let zone = format!("사용자몹시험존-{suffix}");
        let user_path = format!("data/user/{summoned_name}.json");
        let mut saved = Body::new();
        saved.set("이름", loaded_name.as_str());
        saved.set("설명1", "소환된 사용자가 서 있습니다.");
        assert!(save_body_to_json(&mut saved, &user_path));
        get_world_state().write().unwrap().set_player_position(
            &admin_name,
            PlayerPosition::new(zone.clone(), "1".into()),
        );
        let mut admin = Body::new();
        admin.set("이름", admin_name.as_str());
        admin.set("관리자등급", 1000_i64);
        let storage = ScriptStorage::default();

        let usage = storage
            .execute("사용자몹소환", &mut admin, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [대상] 사용자몹소환"]);
        let summoned = storage
            .execute(
                "사용자몹소환",
                &mut admin,
                &summoned_name,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            summoned.0,
            vec![format!(
                "\x1b[1m{loaded_name}\x1b[0;37m{} 소환하였습니다.",
                han_eul(&loaded_name)
            )]
        );
        {
            let world = get_world_state().read().unwrap();
            assert_eq!(world.summoned_users().len(), 1);
            assert_eq!(world.summoned_users()[0].body.get_name(), loaded_name);
            assert!(matches!(
                world.get_room_object_order(&zone, "1").first(),
                Some(RoomObjectRef::SummonedUser(_))
            ));
        }
        let absent = storage
            .execute("사용자몹제거1", &mut admin, "없는사용자", None, None, None)
            .unwrap();
        assert!(absent.0.is_empty(), "Python silently returns when no name matches");
        let removed_in_room = storage
            .execute(
                "사용자몹제거",
                &mut admin,
                &loaded_name,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(removed_in_room.0, vec!["사용자몹이 제거되었습니다."]);
        assert!(get_world_state().read().unwrap().summoned_users().is_empty());

        storage
            .execute(
                "사용자몹소환",
                &mut admin,
                &summoned_name,
                None,
                None,
                None,
            )
            .unwrap();
        get_world_state().write().unwrap().set_player_position(
            &admin_name,
            PlayerPosition::new(zone.clone(), "2".into()),
        );
        let wrong_room = storage
            .execute("사용자몹제거", &mut admin, &loaded_name, None, None, None)
            .unwrap();
        assert_eq!(wrong_room.0, vec!["그런 몹이 없어요!"]);
        let removed = storage
            .execute(
                "사용자몹제거1",
                &mut admin,
                &loaded_name,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(removed.0, vec!["사용자몹이 제거되었습니다."]);
        assert!(get_world_state().read().unwrap().summoned_users().is_empty());

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&admin_name);
        let _ = std::fs::remove_file(user_path);
    }

    #[test]
    fn look_command_uses_plain_python_failure_and_silences_missing_environment() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let name = format!("봐실패회귀-{suffix}");
        let mut body = Body::new();
        body.set("이름", name.as_str());
        let storage = ScriptStorage::default();
        let no_environment = storage
            .execute("봐", &mut body, "없는것", None, None, None)
            .unwrap();
        assert!(no_environment.0.is_empty());

        get_world_state().write().unwrap().set_player_position(
            &name,
            PlayerPosition::new(format!("봐실패존-{suffix}"), "1".into()),
        );
        let missing = storage
            .execute("봐", &mut body, "없는것", None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&name);
    }

    #[test]
    fn look_command_renders_python_detailed_box_layout() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player = format!("봐상자회귀-{suffix}");
        let zone = format!("봐상자존-{suffix}");
        let mut box_object = Object::new();
        box_object.set("이름", "무기고");
        box_object.set("주인", player.as_str());
        box_object.set("보관수량", 3_i64);
        box_object.set("보관최대수량", 5_i64);
        box_object.set("보관증가은전", 100_i64);
        box_object.set("은전", 40_i64);
        let mut sword = Object::new();
        sword.set("이름", "청룡검");
        sword.set("옵션", "힘 +3");
        let mut unique = Object::new();
        unique.set("이름", "간장검");
        unique.set("아이템속성", "단일아이템");
        assert!(box_commands::prepare_installed_box(
            &mut box_object,
            &player,
            "무기고"
        ));
        box_object.objs.push(Arc::new(Mutex::new(sword)));
        box_object.objs.push(Arc::new(Mutex::new(unique)));
        box_commands::register_installed_box(
            &zone,
            "1",
            Arc::new(Mutex::new(box_object)),
        );
        get_world_state().write().unwrap().set_player_position(
            &player,
            PlayerPosition::new(zone.clone(), "1".into()),
        );
        let mut body = Body::new();
        body.set("이름", player.as_str());
        let output = ScriptStorage::default()
            .execute("봐", &mut body, "무기고", None, None, None)
            .unwrap()
            .0;
        assert_eq!(output[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        assert!(output[1].starts_with(&format!(
            "\x1b[1m\x1b[44m\x1b[37m◁ {player}의 무기고 ▷"
        )));
        assert!(output[3].contains("[   1] 청룡검 힘 +3"));
        assert!(output[3].contains("[   2] \x1b[1;36m간장검\x1b[0;37m"));
        assert_eq!(output[4], "───────────────────────────────────────");
        assert!(output[5].contains("◆ 수량 (2/3)  ◆ 최대수량 (5)  ◆ 확장에 필요한 은전 (40/100)"));
        assert_eq!(output[6], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player);
        let _ = std::fs::remove_file(format!("data/box/{player}_무기고.json"));
    }

    #[test]
    fn admin_look_at_mob_adds_python_index_and_runtime_table() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player = format!("봐관리자-{suffix}");
        let zone = format!("봐관리자존-{suffix}");
        let key = format!("{zone}:시험몹파일");
        let mut data = RawMobData::new();
        data.name = "관리자조회몹".into();
        data.zone = zone.clone();
        data.level = 25;
        data.max_hp = 500;
        data.inner_power = 80;
        data.strength = 31;
        data.arm = 12;
        data.agility = 17;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(key.clone(), data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                key.clone(),
                zone.clone(),
                "1",
                &data,
            ));
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 1000_i64);
        let output = ScriptStorage::default()
            .execute("봐", &mut body, "관리자조회몹", None, None, None)
            .unwrap()
            .0;
        assert_eq!(output[0], "Index : 시험몹파일");
        assert!(output.iter().any(|line| line.contains("│ [레  벨]              25")));
        assert!(output.iter().any(|line| line.contains("500/500")));
        assert!(output.iter().any(|line| line.contains("│ [맷  집]              12")));
        assert_eq!(output.last().unwrap(), "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&key);
    }

    #[test]
    fn compare_command_targets_mob_and_restores_python_interface_and_table() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player = format!("비교회귀-{suffix}");
        let zone = format!("비교회귀존-{suffix}");
        let mob_key = format!("{zone}:상대");
        let mut mob_data = RawMobData::new();
        mob_data.name = "비교허수아비".into();
        mob_data.zone = zone.clone();
        mob_data.max_hp = 1000;
        mob_data.strength = 20;
        mob_data.arm = 5;
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), mob_data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                "1",
                &mob_data,
            ));
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("힘", 100_i64);
        body.set("최고내공", 500_i64);
        body.set("최고체력", 2000_i64);
        body.attpower = 100;
        let storage = ScriptStorage::default();

        let usage = storage
            .execute("비교", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [대상] 비교"]);
        let missing = storage
            .execute("비교", &mut body, "없는대상", None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["자신의 상태를 통탄해 합니다. @_@"]);
        let compared = storage
            .execute("비교", &mut body, "비교허수아비", None, None, None)
            .unwrap();
        assert_eq!(compared.0.len(), 7);
        assert_eq!(compared.0[0], "━━━━━━━━━━━━━━━");
        assert_eq!(
            compared.0[1],
            "▶ \x1b[1m비교허수아비\x1b[0;37m와의 상대비교"
        );
        assert!(compared.0[3].starts_with("☞ 당신의 승률 오차ː"));
        assert!(compared.0[4].starts_with("☞ 상대의 승률 오차ː"));

        body.set("설정상태", "비교거부 1");
        let refused = storage
            .execute("비교", &mut body, "비교허수아비", None, None, None)
            .unwrap();
        assert_eq!(
            refused.0,
            vec!["☞ 진정한 승부란 비무를 통해서 알 수 있는 것 이지"]
        );

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn map_command_checks_python_usage_before_missing_position() {
        let mut body = Body::new();
        body.set("이름", "맵순서검사");
        body.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();
        let usage = storage
            .execute("맵", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [제외할방향] 맵"]);
        let invisible = storage
            .execute("맵", &mut body, "동", None, None, None)
            .unwrap();
        assert_eq!(invisible.0, vec!["\r\n* 아무것도 보이지 않습니다.\r\n"]);
    }

    #[test]
    fn map_command_uses_python_first_word_and_raw_hidden_exit_membership() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player = format!("맵탐색검사-{suffix}");
        // A trailing ASCII digit denotes Python difficulty-zone suffixing, so
        // keep the synthetic base-zone name non-numeric.
        let zone = format!("맵탐색존-{suffix}가");
        let dir = std::path::Path::new("data/map").join(&zone);
        std::fs::create_dir_all(&dir).unwrap();
        let write_room = |number: &str, exits: Vec<&str>| {
            std::fs::write(
                dir.join(format!("{number}.json")),
                serde_json::to_string_pretty(&serde_json::json!({
                    "맵정보": {
                        "이름": format!("지도방{number}"),
                        "존이름": zone,
                        "설명": ["지도 탐색 시험"],
                        "출구": exits,
                        "몹": []
                    }
                }))
                .unwrap(),
            )
            .unwrap();
        };
        write_room("1", vec!["동 2", "서 3", "비밀$ 4"]);
        write_room("2", vec!["서 1"]);
        write_room("3", vec!["동 1"]);
        write_room("4", vec!["서 1"]);

        {
            let mut world = get_world_state().write().unwrap();
            for room in ["1", "2", "3", "4"] {
                world.room_cache.get_room(&zone, room).unwrap();
            }
            let loaded = world.room_cache.get_room_cached(&zone, "1").unwrap();
            assert!(loaded.read().unwrap().exits.contains_key("동"));
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 2000_i64);

        let direct = python_map_explore(&body, "동")
            .into_iter()
            .map(|value| value.into_string().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(direct, vec!["서"]);

        let single_word = storage
            .execute("맵", &mut body, "동", None, None, None)
            .unwrap();
        assert_eq!(single_word.0, vec!["서;"]);

        let extra_words = storage
            .execute("맵", &mut body, "  동   뒤의말은무시  ", None, None, None)
            .unwrap();
        assert_eq!(extra_words.0, vec!["서;"]);

        let hidden_raw_name = storage
            .execute("맵", &mut body, "비밀$", None, None, None)
            .unwrap();
        assert_ne!(hidden_raw_name.0, vec!["☞ 그 방향으로는 갈수가 없어요!."]);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        drop(world);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ansi_command_is_python_hp_bar_not_a_persisted_toggle() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "안시막대검사");
        body.set("체력", 45_i64);
        body.set("최고체력", 100_i64);
        body.set("안시", 77_i64);
        let shown = storage
            .execute("안시", &mut body, "꺼기", None, None, None)
            .unwrap();
        assert_eq!(shown.0, vec!["\x1b[32m━━━━\x1b[37m━━━━━━"]);
        assert_eq!(body.get_int("안시"), 77, "Python ignores the command argument");
    }

    #[test]
    fn test_ansi_convert() {
        let result = ansi_convert("{밝}hello{어}", true);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("\x1b[0m"));

        let result = ansi_convert("{밝}hello{어}", false);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_han_iga() {
        assert_eq!(han_iga("사과"), "가");
        assert_eq!(han_iga("검"), "이");
    }

    #[test]
    fn test_build_ob_exposes_all_object_attributes() {
        let mut body = Body::new();
        body.set("이름", "속성검사");
        body.set("사용자정의문자", "값");
        body.set("사용자정의정수", 77i64);
        body.object.set("사용자정의실수", 1.5f64);

        let map = build_ob_from_body(&body);
        assert_eq!(map["사용자정의문자"].clone().into_string().unwrap(), "값");
        assert_eq!(map["사용자정의정수"].as_int().unwrap(), 77);
        assert_eq!(map["사용자정의실수"].as_float().unwrap(), 1.5);
    }

    #[test]
    fn user_alias_rhai_matches_python_messages_and_state_rules() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("줄임말"));
        let mut body = Body::new();
        body.set("이름", "줄임말검사");

        let (output, _) = storage
            .execute("줄임말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 줄임말이 설정되어 있지 않아요. ^^"]);

        let (output, _) = storage
            .execute("줄임말", &mut body, "길 동;서", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 줄임말을 설정하였어요. ^^"]);
        assert_eq!(
            decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR)),
            vec![("길".to_string(), "동;서".to_string())]
        );

        let (output, _) = storage
            .execute("줄임말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            output,
            vec![
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                "\x1b[47m\x1b[30m◁ 줄임말 ▷                                                                  \x1b[40m\x1b[37m",
                "───────────────────────────────────────",
                "[길] 동;서",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            ]
        );

        let (output, _) = storage
            .execute("줄임말", &mut body, "길 북", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 이미 설정되어 있는 줄임말입니다."]);

        for invalid in ["자기 자기", "중첩 길"] {
            let (output, _) = storage
                .execute("줄임말", &mut body, invalid, None, None, None)
                .unwrap();
            assert_eq!(output, vec!["☞ 중첩된 줄임말은 사용할 수 없어요. ^^"]);
        }

        let (output, _) = storage
            .execute("줄임말", &mut body, "길", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 줄임말을 제거하였어요. ^^"]);
        let (output, _) = storage
            .execute("줄임말", &mut body, "길", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 줄임말이 설정되어 있지 않아요. ^^"]);
    }

    #[test]
    fn user_alias_rhai_enforces_python_hundred_entry_limit() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "줄임말제한검사");
        let entries: Vec<(String, String)> = (0..100)
            .map(|index| (format!("키{}", index), "북".to_string()))
            .collect();
        body.set(ALIAS_LIST_ATTR, encode_alias_entries(&entries));

        let (output, _) = storage
            .execute("줄임말", &mut body, "초과 남", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 줄임말이 너무 많아요. ^^"]);
        assert_eq!(
            decode_alias_entries(&body.get_string(ALIAS_LIST_ATTR)).len(),
            100
        );
    }

    #[test]
    fn user_alias_json_round_trip_uses_python_array_without_touching_user_data() {
        let path = std::env::temp_dir().join(format!(
            "muc_alias_round_trip_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let entries = vec![
            ("대상".to_string(), "* 쳐;봐".to_string()),
            ("파이프".to_string(), "값|그대로 전음".to_string()),
        ];
        let mut body = Body::new();
        body.set("이름", "임시줄임말검사");
        body.set(ALIAS_LIST_ATTR, encode_alias_entries(&entries));
        assert!(save_body_to_json(&mut body, path.to_str().unwrap()));

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            json["사용자오브젝트"][ALIAS_LIST_ATTR],
            serde_json::json!(["대상 * 쳐;봐", "파이프 값|그대로 전음"])
        );

        let mut loaded = Body::new();
        assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
        assert_eq!(
            decode_alias_entries(&loaded.get_string(ALIAS_LIST_ATTR)),
            entries
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_body_emits_python_numeric_alias_defaults() {
        let path =
            std::env::temp_dir().join(format!("muc_numeric_alias_{}.json", std::process::id()));
        let mut body = Body::new();
        body.set("이름", "숫자별칭검사");
        body.set("최대체력", 450);
        assert!(save_body_to_json(&mut body, path.to_str().unwrap()));
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let user = &json["사용자오브젝트"];
        assert_eq!(user["최고체력"], serde_json::json!(450));
        assert_eq!(user["맷집"], serde_json::json!(0));
        assert_eq!(user["내공"], serde_json::json!(0));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tell_history_is_runtime_only_like_python_player_state() {
        let path = std::env::temp_dir().join(format!(
            "muc_tell_history_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut body = Body::new();
        body.set("이름", "임시전음기록검사");
        body.talk_history.push("현재 접속 기록".to_string());
        assert!(save_body_to_json(&mut body, path.to_str().unwrap()));

        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(json.get("대화기록").is_none());

        // 과거 Rust 파일의 발명된 필드도 새 접속에는 복원하지 않는다.
        json.as_object_mut()
            .unwrap()
            .insert("대화기록".to_string(), serde_json::json!(["오래된 기록"]));
        std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();
        let mut loaded = Body::new();
        loaded.talk_history.push("초기값".to_string());
        assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
        assert!(loaded.talk_history.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mugong_json_round_trip_uses_python_arrays_and_rebuilds_runtime_state() {
        let path = std::env::temp_dir().join(format!(
            "muc_mugong_round_trip_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut body = Body::new();
        body.set("이름", "임시무공검사");
        body.skill_list = vec!["지르기".to_string(), "강룡십팔장".to_string()];
        body.skill_map.insert(
            "지르기".to_string(),
            crate::player::SkillTraining::new(2, 7),
        );
        body.skill_map.insert(
            "강룡십팔장".to_string(),
            crate::player::SkillTraining::new(9, 42),
        );
        body.set("비전이름", "비전검법|비전도법");
        assert!(save_body_to_json(&mut body, path.to_str().unwrap()));

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            json["사용자오브젝트"]["무공이름"],
            serde_json::json!(["지르기", "강룡십팔장"])
        );
        assert_eq!(
            json["사용자오브젝트"]["무공숙련도"],
            serde_json::json!(["지르기 2 7", "강룡십팔장 9 42"])
        );
        assert_eq!(
            json["사용자오브젝트"]["비전이름"],
            serde_json::json!(["비전검법", "비전도법"])
        );

        let mut loaded = Body::new();
        assert!(load_body_from_json(&mut loaded, path.to_str().unwrap()));
        assert_eq!(loaded.skill_list, vec!["지르기", "강룡십팔장"]);
        assert_eq!(
            loaded.skill_map.get("강룡십팔장"),
            Some(&crate::player::SkillTraining::new(9, 42))
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_current_body_position_accepts_colon_and_legacy_slash() {
        let mut body = Body::new();
        body.set("이름", "위치형식검사전용");
        body.set("위치", "낙양성:42");
        assert_eq!(
            current_body_position(&body),
            Some(("낙양성".to_string(), "42".to_string()))
        );

        body.set("위치", "");
        body.set("현재방", "하북성/3001");
        assert_eq!(
            current_body_position(&body),
            Some(("하북성".to_string(), "3001".to_string()))
        );
    }

    #[test]
    fn test_track_command_matches_python_messages_and_first_room() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("추적"));
        let mut body = Body::new();
        body.set("이름", "추적검사");

        let (output, _) = storage
            .execute("추적", &mut body, "청강석 낙양성", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

        body.set("관리자등급", 1000i64);
        for line in ["", "청강석"] {
            let (output, _) = storage
                .execute("추적", &mut body, line, None, None, None)
                .unwrap();
            assert_eq!(output, vec!["몹이름 존이름 추적"]);
        }

        let (output, _) = storage
            .execute(
                "추적",
                &mut body,
                "청강석 __존재하지_않는_존__",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(output, vec!["그런 존은 없어요!"]);

        let (output, _) = storage
            .execute(
                "추적",
                &mut body,
                "__존재하지_않는_몹__ 낙양성",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(output, vec!["못찾았음"]);

        // Python loadAllMob 순서에서 청강석의 첫 배치 방은 낙양성:4004이다.
        let (output, _) = storage
            .execute("추적", &mut body, "청강석 낙양성", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["4004"]);
    }

    #[test]
    fn test_read_text_file_is_confined_to_public_data() {
        assert!(read_public_text_file("/etc/passwd").is_empty());
        assert!(read_public_text_file("data/config/../user/밍밍.json").is_empty());
        if Path::new("data/text/notice.txt").exists() {
            assert!(!read_public_text_file("data/text/notice.txt").is_empty());
        }
    }

    #[test]
    fn test_rest_command_uses_python_act_rest_value() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("쉬어"));
        let mut body = Body::new();
        body.set("이름", "휴식검사");

        let result = storage.execute("쉬어", &mut body, "", None, None, None);
        assert!(result.is_ok(), "쉬어 실행 실패: {:?}", result.err());
        assert_eq!(body.act, crate::player::ActState::Rest);
        assert_eq!(body.act.to_i32(), 4);
    }

    #[test]
    fn test_rest_notifies_only_players_in_the_same_room() {
        let self_name = "휴식방알림본인";
        let same_room_name = "휴식방알림동일방";
        let other_room_name = "휴식방알림다른방";
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                self_name,
                PlayerPosition::new("휴식방알림존".to_string(), "1".to_string()),
            );
            world.set_player_position(
                same_room_name,
                PlayerPosition::new("휴식방알림존".to_string(), "1".to_string()),
            );
            world.set_player_position(
                other_room_name,
                PlayerPosition::new("휴식방알림존".to_string(), "2".to_string()),
            );
        }

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", self_name);
        let (output, special) = storage
            .execute("쉬어", &mut body, "", None, None, None)
            .unwrap();

        assert_eq!(
            output,
            vec!["당신이 자세를 편안히 하며 운기조식에 들어갑니다."]
        );
        assert!(matches!(
            special,
            Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
                if own == "당신이 자세를 편안히 하며 운기조식에 들어갑니다."
                    && sends == &vec![(
                        same_room_name.to_string(),
                        format!(
                            "\r\n\x1b[1m{}\x1b[0;37m{} 자세를 편안히 하며 운기조식에 들어갑니다.\r\n",
                            self_name,
                            han_iga(self_name)
                        )
                    )]
        ));

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(self_name);
        world.remove_player_position(same_room_name);
        world.remove_player_position(other_room_name);
    }

    #[test]
    fn equipment_script_matches_python_layout_and_item_order() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "장비검사");
        body.armor = 120;
        body.attpower = 44;

        let weapon = Arc::new(Mutex::new(Object::new()));
        {
            let mut item = weapon.lock().unwrap();
            item.set("이름", "철검");
            item.set("계층", "무기");
            item.set("반응이름", "검 철검");
            item.set("inUse", 1i64);
        }
        body.object.objs.push(weapon);

        let helmet = Arc::new(Mutex::new(Object::new()));
        {
            let mut item = helmet.lock().unwrap();
            item.set("이름", "Excalibur");
            item.set("계층", "투구");
            item.set("반응이름", "엑스칼리버 보검");
            item.set("inUse", 1i64);
        }
        body.object.objs.push(helmet);

        let (output, special) = storage
            .execute("장비", &mut body, "", None, None, None)
            .unwrap();
        assert!(special.is_none());
        let header = fill_space_euc_kr(54, "▷ 당신은 초라한 방어구를 착용하고 있습니다.");
        let expected = format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n\
             \x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m\r\n\
             ───────────────────────────\r\n\
             [투    구] \x1b[36mExcalibur(엑스칼리버)\x1b[37m\r\n\
             [무    기] \x1b[36m철검\x1b[37m\r\n\
             ───────────────────────────\r\n\
             【방어력】▷ 120    【공격력】▷ 44\r\n\
             ━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            header
        );
        assert_eq!(output, vec![expected]);
    }

    #[test]
    fn test_recover_command_updates_canonical_hp_mp_attributes() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("회복"));
        let mut body = Body::new();
        body.set("이름", "회복검사");
        body.set("관리자등급", 1000i64);
        body.set("최고체력", 321i64);
        body.set("최고내공", 123i64);
        body.set("체력", 1i64);
        body.set("내공", 2i64);

        let result = storage.execute("회복", &mut body, "", None, None, None);
        assert!(result.is_ok(), "회복 실행 실패: {:?}", result.err());
        assert_eq!(body.get_hp(), 321);
        assert_eq!(body.get_mp(), 123);
    }

    #[test]
    fn test_auto_skill_commands_match_python_state_changes() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("자동무공"));
        assert!(storage.has_script("자동무공삭제"));
        let mut body = Body::new();
        body.set("이름", "자동무공검사");
        body.skill_list.push("강룡십팔장".to_string());

        let (output, _) = storage
            .execute("자동무공", &mut body, "강룡", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 자동무공을 지정하였습니다."]);
        assert_eq!(body.get_string("자동무공"), "강룡십팔장");

        let (output, _) = storage
            .execute("자동무공삭제", &mut body, "무시되는 인자", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 자동무공을 삭제하였습니다."]);
        assert_eq!(body.get_string("자동무공"), "");

        let (output, _) = storage
            .execute("자동무공삭제", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 자동무공 : 없음"]);
    }

    #[test]
    fn test_nickname_command_usage_path_executes() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("무림별호"));
        let mut body = Body::new();
        body.set("이름", "별호검사");

        let (output, special) = storage
            .execute("무림별호", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 사용법: [별호이름] 무림별호"]);
        assert!(special.is_none());
    }

    #[test]
    fn test_nickname_command_rejects_legacy_duplicate() {
        if !Path::new("data/config/nickname.json").exists() {
            return;
        }
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "별호중복검사");
        body.set("무림별호", "");
        body.set("이벤트설정리스트", "무림별호설정");

        let (output, special) = storage
            .execute("무림별호", &mut body, "감정노동자", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 다른 무림인이 사용중인 별호입니다. ^^"]);
        assert!(special.is_none());
        assert_eq!(body.get_string("무림별호"), "");
    }

    #[test]
    fn test_json_debug_command_requires_level_2000() {
        let storage = ScriptStorage::default();
        assert!(storage.has_script("제이슨"));
        let mut body = Body::new();
        body.set("이름", "일반사용자");
        body.set("관리자등급", 0i64);

        let (output, _) = storage
            .execute("제이슨", &mut body, "../user/밍밍", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    }

    #[test]
    fn test_script_preserves_self_output_with_targeted_sends() {
        let root = std::env::temp_dir().join(format!(
            "muc_script_combined_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("combined.rhai"),
            r#"fn main(ob, line) {
                send_line(ob, "self");
                send_to_user("other", "target");
            }"#,
        )
        .unwrap();

        let config = ScriptConfig {
            script_dir: root.clone(),
            ..ScriptConfig::default()
        };
        let storage = ScriptStorage::new(config);
        let mut body = Body::new();
        body.set("이름", "self");
        let (output, special) = storage
            .execute("combined", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["self"]);
        assert!(matches!(
            special,
            Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
                if own == "self" && sends == &vec![("other".to_string(), "target".to_string())]
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    fn inventory_test_item(name: &str, in_use: bool, hidden: bool) -> Arc<Mutex<Object>> {
        let item = Arc::new(Mutex::new(Object::new()));
        {
            let mut item = item.lock().unwrap();
            item.set("이름", name);
            item.set("inUse", i64::from(in_use));
            if hidden {
                item.set("아이템속성", "출력안함");
            }
        }
        item
    }

    #[test]
    fn test_inventory_admin_views_same_room_target_like_python() {
        let storage = ScriptStorage::default();
        let mut viewer = Body::new();
        viewer.set("이름", "관리자");
        viewer.set("관리자등급", 1000i64);

        let mut target = Body::new();
        target.set("이름", "대상");
        target.set("은전", 7i64);
        target.set("금전", 9i64);
        target
            .object
            .objs
            .push(inventory_test_item("약초", false, false));
        target
            .object
            .objs
            .push(inventory_test_item("비밀패", false, true));
        target
            .object
            .objs
            .push(inventory_test_item("철검", true, false));
        target
            .object
            .objs
            .push(inventory_test_item("약초", false, false));
        set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);

        let (output, special) = storage
            .execute("소지품", &mut viewer, "대상", None, None, None)
            .unwrap();
        clear_precomputed_all_online();

        assert!(special.is_none());
        assert_eq!(
            output,
            vec![
                "━━━━━━━━━━━━━━━━━".to_string(),
                "\x1b[0m\x1b[44m\x1b[1m\x1b[37m  ◁     소     지     품     ▷  \x1b[0m\x1b[37m\x1b[40m".to_string(),
                "─────────────────".to_string(),
                "\x1b[36m약초 \x1b[36m2개\x1b[37m".to_string(),
                "\x1b[36m비밀패\x1b[37m".to_string(),
                "─────────────────".to_string(),
                format!(
                    "\x1b[0m\x1b[47m\x1b[30m▶ 은전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m",
                    7
                ),
                format!(
                    "\x1b[0m\x1b[43m\x1b[30m▶ 금전 : {:>20} 개 \x1b[0m\x1b[37m\x1b[40m",
                    9
                ),
                "─────────────────\x1b[0;37m".to_string(),
            ]
        );
    }

    #[test]
    fn compact_inventory_uses_admin_target_but_python_viewer_gold() {
        let storage = ScriptStorage::default();
        let mut viewer = Body::new();
        viewer.set("이름", "소소관리자");
        viewer.set("관리자등급", 1000_i64);
        viewer.set("금전", 33_i64);
        let mut target = Body::new();
        target.set("이름", "소소대상");
        target.set("은전", 7_i64);
        target.set("금전", 99_i64);
        target
            .object
            .objs
            .push(inventory_test_item("약초", false, false));
        target
            .object
            .objs
            .push(inventory_test_item("약초", false, false));
        set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);

        let output = storage
            .execute("소소", &mut viewer, "소소대상", None, None, None)
            .unwrap()
            .0;
        clear_precomputed_all_online();
        assert!(output.contains(&"\x1b[36m약초 \x1b[36m2개\x1b[37m".to_string()));
        assert!(output.iter().any(|line| line.contains("은전 :                    7 개")));
        assert!(output.iter().any(|line| line.contains("금전 :                   33 개")));
        assert!(!output.iter().any(|line| line.contains("금전 :                   99 개")));
        assert_eq!(output.last().unwrap(), "─────────────────");
    }

    #[test]
    fn socket_command_sorts_host_name_pairs_and_right_aligns_python_host_column() {
        let entries = [
            ("10.0.0.2", "나"),
            ("10.0.0.1", "다"),
            ("10.0.0.1", "가"),
        ]
        .into_iter()
        .map(|(host, name)| {
            let mut map = rhai::Map::new();
            map.insert("host".into(), Dynamic::from(host.to_string()));
            map.insert("이름".into(), Dynamic::from(name.to_string()));
            Dynamic::from(map)
        })
        .collect();
        set_precomputed_all_online(entries);
        let mut body = Body::new();
        body.set("관리자등급", 1000_i64);
        let output = ScriptStorage::default()
            .execute("소켓", &mut body, "", None, None, None)
            .unwrap();
        clear_precomputed_all_online();
        assert_eq!(
            output.0,
            vec!["\r\n        10.0.0.1 : 가, 다\r\n        10.0.0.2 : 나"]
        );
    }

    #[test]
    fn tweet_uses_python_usage_and_recipient_time_ansi_preferences() {
        use crate::command::handler::CommandResult;
        use crate::world::{get_world_state, PlayerPosition};

        let sender = "트윗발신자";
        let timed = "트윗시간수신자";
        let blocked = "트윗거부자";
        let online = [
            (sender, ""),
            (timed, "잡담시간보기 1\n사용자안시거부 1"),
            (blocked, "외침거부 1"),
        ]
        .into_iter()
        .map(|(name, config)| {
            let mut map = rhai::Map::new();
            map.insert("이름".into(), Dynamic::from(name));
            map.insert("설정상태".into(), Dynamic::from(config));
            Dynamic::from(map)
        })
        .collect();
        set_precomputed_all_online(online);
        get_world_state().write().unwrap().set_player_position(
            sender,
            PlayerPosition::new("트윗시험존".into(), "1".into()),
        );
        let mut body = Body::new();
        body.set("이름", sender);
        body.set("act", 1_i64);
        let storage = ScriptStorage::default();

        let usage = storage
            .execute("트윗", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [내용] 외침(,)"]);

        let sent = storage
            .execute("트윗", &mut body, "{빨}안녕", None, None, None)
            .unwrap();
        assert!(sent.0.is_empty());
        let sends = match sent.1 {
            Some(CommandResult::SendToUsers(sends)) => sends,
            other => panic!("unexpected tweet result: {other:?}"),
        };
        assert_eq!(sends.len(), 2);
        let self_wire = sends.iter().find(|(name, _)| name == sender).unwrap().1.clone();
        assert!(!self_wire.starts_with("\r\n"));
        assert!(self_wire.contains("\x1b[31m안녕"));
        let timed_wire = sends.iter().find(|(name, _)| name == timed).unwrap().1.clone();
        assert!(timed_wire.starts_with("\r\n["));
        assert!(!timed_wire.contains("\x1b[31m"));
        assert!(timed_wire.contains("안녕"));
        assert!(!sends.iter().any(|(name, _)| name == blocked));
        assert!(chat_history_snapshot()
            .last()
            .is_some_and(|line| line.contains("\x1b[31m안녕\x1b[0;37m")));

        body.set("성격", "선인");
        body.set("관리자등급", 2000_i64);
        let shouted = storage
            .execute("외쳐", &mut body, "{빨}호령", None, None, None)
            .unwrap();
        let shout_sends = match shouted.1 {
            Some(CommandResult::SendToUsers(sends)) => sends,
            other => panic!("unexpected shout result: {other:?}"),
        };
        assert!(shout_sends
            .iter()
            .find(|(name, _)| name == sender)
            .is_some_and(|(_, wire)| wire.contains("\x1b[0;35m사자후\x1b[0;37m")
                && wire.contains("\x1b[31m호령")));
        assert!(shout_sends
            .iter()
            .find(|(name, _)| name == timed)
            .is_some_and(|(_, wire)| wire.starts_with("\r\n[") && !wire.contains("\x1b[31m")));

        let shout2 = storage
            .execute("외쳐2", &mut body, "두번째", None, None, None)
            .unwrap();
        let shout2_sends = match shout2.1 {
            Some(CommandResult::SendToUsers(sends)) => sends,
            other => panic!("unexpected shout2 result: {other:?}"),
        };
        assert!(shout2_sends
            .iter()
            .all(|(_, wire)| wire.ends_with(" \x1b[1;32m밍밍이지렁~\x1b[0;37m")));

        clear_precomputed_all_online();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(sender);
    }

    #[test]
    fn test_inventory_keeps_python_hidden_only_and_target_failure_behavior() {
        let storage = ScriptStorage::default();
        let mut viewer = Body::new();
        viewer.set("이름", "관리자");
        viewer.set("관리자등급", 1000i64);

        let mut target = Body::new();
        target.set("이름", "대상");
        target
            .object
            .objs
            .push(inventory_test_item("비밀패", false, true));
        set_precomputed_room_inventories(vec![build_room_player_inventory_snapshot(&target)]);
        let (output, _) = storage
            .execute("소지품", &mut viewer, "대상", None, None, None)
            .unwrap();
        assert!(output.contains(&"\x1b[36m☞ 아무것도 없습니다.\x1b[37m".to_string()));
        assert!(!output.iter().any(|line| line.contains("비밀패")));

        let (output, _) = storage
            .execute("소지품", &mut viewer, "없는사람", None, None, None)
            .unwrap();
        clear_precomputed_all_online();
        assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
    }

    #[test]
    fn test_mugong_self_output_matches_python_categories_width_and_visions() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "검객");
        body.skill_list = vec![
            "강룡십팔장".to_string(),
            "지르기".to_string(),
            "철포삼".to_string(),
        ];
        body.skill_map.insert(
            "강룡십팔장".to_string(),
            crate::player::SkillTraining::new(9, 42),
        );
        body.skill_map.insert(
            "지르기".to_string(),
            crate::player::SkillTraining::new(2, 5),
        );
        body.set("비전수련", "강룡십팔장비전 17");
        body.set("비전이름", "비전검법|비전도법|비전창법");

        let (output, special) = storage
            .execute("무공", &mut body, "", None, None, None)
            .unwrap();

        assert!(special.is_none());
        assert_eq!(output.len(), 8);
        assert_eq!(output[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        assert_eq!(
            output[1],
            "\x1b[0m\x1b[47m\x1b[30m◁ 당신의 무공 ▷                                                             \x1b[0m\x1b[40m\x1b[37m"
        );
        assert_eq!(output[2], "───────────────────────────────────────");
        assert_eq!(
            output[3],
            concat!(
                "\x1b[1m\x1b[40m\x1b[32m▷ 초급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                " ◇ 지르기(2성)          \r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 중급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 상급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 고급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 특급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                " ◇ 강룡십팔장(9성)      \r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 절정무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 회복무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 방어무공\x1b[0m\x1b[40m\x1b[37m\r\n",
                " ◇ 철포삼(1성)          \r\n",
                "\x1b[1m\x1b[40m\x1b[32m▷ 기타무공\x1b[0m\x1b[40m\x1b[37m"
            )
        );
        assert_eq!(output[4], "───────────────────────────────────────");
        assert_eq!(
            output[5],
            "\x1b[1m\x1b[40m\x1b[32m▷ 비전\x1b[0m\x1b[40m\x1b[37m"
        );
        assert_eq!(
            output[6],
            concat!(
                "\x1b[1m\x1b[33m강룡십팔장비전 17\x1b[0m\x1b[40m\x1b[37m(수련중)\r\n",
                " ◇ 비전검법              ◇ 비전도법              ◇ 비전창법             "
            )
        );
        assert_eq!(output[7], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    #[test]
    fn test_mugong_skill_cells_use_python_three_column_wrap() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "검객");
        body.skill_list = ["지르기", "비각", "원앙퇴", "쌍비각"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let (output, _) = storage
            .execute("무공", &mut body, "", None, None, None)
            .unwrap();

        assert!(output[3].starts_with(concat!(
            "\x1b[1m\x1b[40m\x1b[32m▷ 초급무공\x1b[0m\x1b[40m\x1b[37m\r\n",
            " ◇ 지르기(1성)           ◇ 비각(1성)             ◇ 원앙퇴(1성)          \r\n",
            " ◇ 쌍비각(1성)          "
        )));
    }

    #[test]
    fn test_mugong_admin_uses_same_room_snapshot_and_regular_line_is_ignored() {
        let storage = ScriptStorage::default();
        let mut viewer = Body::new();
        viewer.set("이름", "관리자");
        viewer.set("관리자등급", 1000i64);

        let mut target = Body::new();
        target.set("이름", "대상");
        target.set("반응이름", "검객 대상자");
        target.skill_list.push("지르기".to_string());
        set_precomputed_room_mugong_targets(vec![build_room_mugong_player_snapshot(&target)]);

        let (output, _) = storage
            .execute("무공", &mut viewer, "대상", None, None, None)
            .unwrap();
        assert!(output[1].contains("◁ 대상의 무공 ▷"));

        viewer.set("관리자등급", 999i64);
        let (output, _) = storage
            .execute("무공", &mut viewer, "대상", None, None, None)
            .unwrap();
        clear_precomputed_all_online();
        assert!(output[1].contains("◁ 당신의 무공 ▷"));
    }

    #[test]
    fn test_mugong_admin_can_view_python_mob_target_shape() {
        let storage = ScriptStorage::default();
        let mut viewer = Body::new();
        viewer.set("이름", "관리자");
        viewer.set("관리자등급", 1000i64);

        let mut data = RawMobData::new();
        data.name = "수련인".to_string();
        data.reaction_names = vec!["수련".to_string()];
        data.skills = vec![("지르기".to_string(), 100, 30)];
        let instance = MobInstance::new("시험:수련인".to_string(), "시험".to_string(), "1", &data);
        set_precomputed_room_mugong_targets(vec![build_room_mugong_mob_snapshot(&instance, &data)]);

        let (output, _) = storage
            .execute("무공", &mut viewer, "수련", None, None, None)
            .unwrap();
        clear_precomputed_all_online();

        assert!(output[1].contains("◁ 수련인의 무공 ▷"));
        // Python Mob.skillList는 무공 튜플 목록이고 skillMap은 비어 있으므로
        // 카테고리 머리말은 출력되지만 플레이어식 `지르기(1성)` 셀은 없다.
        assert!(output[3].contains("▷ 초급무공"));
        assert!(!output[3].contains("지르기(1성)"));

        let mut second_data = RawMobData::new();
        second_data.name = "수련인둘".to_string();
        let second = MobInstance::new(
            "시험:수련인둘".to_string(),
            "시험".to_string(),
            "1",
            &second_data,
        );
        set_precomputed_room_mugong_targets(vec![
            build_room_mugong_mob_snapshot(&instance, &data),
            build_room_mugong_mob_snapshot(&second, &second_data),
        ]);
        let (output, _) = storage
            .execute("무공", &mut viewer, "1", None, None, None)
            .unwrap();
        clear_precomputed_all_online();
        assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
    }

    #[test]
    fn test_mugong_admin_rejects_item_and_does_not_guess_unified_order_collision() {
        let storage = ScriptStorage::default();
        let mut viewer = Body::new();
        viewer.set("이름", "관리자");
        viewer.set("관리자등급", 1000i64);

        let mut item = Object::new();
        item.set("이름", "옥패");
        set_precomputed_room_mugong_targets(vec![build_room_mugong_item_snapshot(&item)]);
        let (output, _) = storage
            .execute("무공", &mut viewer, "옥패", None, None, None)
            .unwrap();
        assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);

        let mut player = Body::new();
        player.set("이름", "옥패");
        set_precomputed_room_mugong_targets(vec![
            build_room_mugong_player_snapshot(&player),
            build_room_mugong_item_snapshot(&item),
        ]);
        let (output, _) = storage
            .execute("무공", &mut viewer, "옥패", None, None, None)
            .unwrap();
        clear_precomputed_all_online();
        assert_eq!(output, vec!["☞ 당신의 안광으로는 그런것을 볼수 없다네"]);
    }

    #[test]
    fn test_fill_space_euc_kr_matches_python_fill_space() {
        assert_eq!(fill_space_euc_kr(20, "지르기(2성)"), "지르기(2성)         ");
        assert_eq!(
            fill_space_euc_kr(20, "\x1b[31m비전검법\x1b[0m"),
            "\x1b[31m비전검법\x1b[0m            "
        );
    }

    #[tokio::test]
    async fn test_shared_storage() {
        let shared = SharedScriptStorage::new(ScriptConfig::default());
        let storage = shared.inner.read().await;
        assert!(storage.config.script_dir.ends_with("cmds"));
    }

    #[test]
    fn test_item_commands_create_drop_get_destroy() {
        use crate::player::Body;
        use crate::world::{get_world_state, PlayerPosition};

        let mut body = Body::new();
        body.set("이름", "item_test_player");
        body.set("관리자등급", 2000i64);
        body.set("힘", 1000_i64);

        // 플레이어 위치를 낙양성:1로 설정 (버리기/가져오기에 필요)
        {
            let mut w = get_world_state().write().unwrap();
            w.set_player_position(
                "item_test_player",
                PlayerPosition::new("낙양성".to_string(), "1".to_string()),
            );
        }

        let storage = ScriptStorage::default();
        if !storage.has_script("생성") {
            return; // cmds/생성.rhai가 없으면 스킵
        }

        // data/item/289.json 필요 (cargo test 시 cwd=프로젝트 루트)
        if !std::path::Path::new("data/item/289.json").exists() {
            return; // 데이터 없으면 스킵
        }

        // 1) 생성 289 (data/item/289.json = 철퇴)
        let res = storage.execute("생성", &mut body, "289", None, None, None);
        assert!(res.is_ok(), "생성 실패: {:?}", res.err());
        let (out, _) = res.as_ref().unwrap();
        assert_eq!(
            body.object.objs.len(),
            1,
            "생성 후 인벤 1개 (outputs: {:?})",
            out
        );
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "철퇴");

        // 2) 버리기 철퇴
        let res = storage.execute("버려", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "버리기 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "버린 후 인벤 비어있음");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", "1");
            assert_eq!(ro.len(), 1, "방 바닥에 1개");
            assert_eq!(ro[0].lock().unwrap().getName(), "철퇴");
        }

        // 3) 가져오기 철퇴
        let res = storage.execute("가져", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "가져오기 실패: {:?}", res.err());
        assert!(
            res.as_ref().unwrap().0.join("\r\n").contains("철퇴를\x1b[37m 집어서"),
            "가져 조사는 Python han_obj처럼 목적격이어야 함: {:?}",
            res.as_ref().unwrap().0
        );
        assert_eq!(body.object.objs.len(), 1, "가져온 후 인벤 1개");
        {
            let mut w = get_world_state().write().unwrap();
            let ro = w.get_room_objs_mut("낙양성", "1");
            assert_eq!(ro.len(), 0, "가져온 후 방 바닥 비어있음");
        }

        // 4) 소각 철퇴
        let res = storage.execute("소각", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "소각 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "소각 후 인벤 비어있음");

        // 5) 생성 → 부셔
        let _ = storage.execute("생성", &mut body, "289", None, None, None);
        assert_eq!(body.object.objs.len(), 1);
        let res = storage.execute("부셔", &mut body, "철퇴", None, None, None);
        assert!(res.is_ok(), "부셔 실패: {:?}", res.err());
        assert_eq!(body.object.objs.len(), 0, "부신 후 인벤 비어있음");

        // 6) 모두 가져 / 모두 입어
        let _ = storage.execute("생성", &mut body, "289", None, None, None);
        let _ = storage.execute("생성", &mut body, "289", None, None, None);
        let _ = storage.execute("버려", &mut body, "모두", None, None, None);
        assert!(body.object.objs.is_empty());
        let picked = storage
            .execute("가져", &mut body, "모두", None, None, None)
            .unwrap();
        assert_eq!(body.object.objs.len(), 2);
        assert!(picked.0.join("\r\n").contains("철퇴\x1b[37m 2개를 집어서"));

        let equipped = storage
            .execute("입어", &mut body, "모두", None, None, None)
            .unwrap();
        assert_eq!(
            body.object
                .objs
                .iter()
                .filter(|item| item.lock().is_ok_and(|item| item.getBool("inUse")))
                .count(),
            1,
            "같은 계층 장비는 Python checkArmed에 따라 하나만 착용"
        );
        assert!(
            !equipped.0.is_empty(),
            "모두 입어는 착용한 각 장비의 Python 사용 문구를 출력해야 함"
        );
        let unequipped = storage
            .execute("벗어", &mut body, "모두", None, None, None)
            .unwrap();
        let unequip_output = unequipped.0.join("\r\n");
        assert!(unequip_output.contains("당신이 \x1b[36m철퇴를\x1b[37m 착용해제 합니다."));
        assert!(!unequip_output.contains("착용한 장비 1개를 해제했습니다."));
        assert!(body
            .object
            .objs
            .iter()
            .all(|item| item.lock().is_ok_and(|item| !item.getBool("inUse"))));

        let _ = storage.execute("입어", &mut body, "철퇴", None, None, None);
        let remembered = storage
            .execute("세트기억", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(remembered.0.join("\r\n"), "☞ 기억 되었습니다.");
        let _ = storage.execute("벗어", &mut body, "모두", None, None, None);
        let set_equipped = storage
            .execute("세트착용", &mut body, "", None, None, None)
            .unwrap();
        assert!(!set_equipped.0.is_empty());
        assert!(body
            .object
            .objs
            .iter()
            .any(|item| item.lock().is_ok_and(|item| item.getBool("inUse"))));
    }

    #[test]
    fn set_wear_equips_the_tagged_same_name_instance_by_python_inventory_order() {
        let mut body = Body::new();
        body.set("이름", "세트순번검사");
        body.set("세트기억", "SET-선택");
        let mut ordinary = Object::new();
        ordinary.set("이름", "쌍검");
        ordinary.set("반응이름", "쌍검");
        ordinary.set("종류", "무기");
        ordinary.set("계층", "무기");
        let ordinary = Arc::new(Mutex::new(ordinary));
        let mut tagged = Object::new();
        tagged.set("이름", "쌍검");
        tagged.set("반응이름", "쌍검\r\nSET-선택");
        tagged.set("종류", "무기");
        tagged.set("계층", "무기");
        tagged.set("안시", "\x1b[35m");
        let tagged = Arc::new(Mutex::new(tagged));
        body.object.objs.push(ordinary.clone());
        body.object.objs.push(tagged.clone());

        let worn = ScriptStorage::default()
            .execute("세트착용", &mut body, "", None, None, None)
            .unwrap();
        assert!(!ordinary.lock().unwrap().getBool("inUse"));
        assert!(tagged.lock().unwrap().getBool("inUse"));
        assert_eq!(
            worn.0,
            vec!["당신이 \x1b[36m\x1b[35m쌍검\x1b[0;37m을\x1b[37m 착용합니다."]
        );
    }

    #[test]
    fn burn_and_break_use_actual_item_text_notify_room_and_persist_removal() {
        let _oneitem_guard = ONEITEM_COMMAND_TEST_LOCK.lock().unwrap();
        use crate::command::handler::CommandResult;
        use crate::object::Object;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let self_name = format!("파괴회귀-{suffix}");
        let observer = format!("파괴관찰-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &self_name,
                PlayerPosition::new("파괴회귀존".into(), suffix.to_string()),
            );
            world.set_player_position(
                &observer,
                PlayerPosition::new("파괴회귀존".into(), suffix.to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", self_name.as_str());
        let mut fruit = Object::new();
        fruit.set("이름", "설삼과");
        fruit.set("반응이름", "설삼과\r\n과일");
        body.object.append(Arc::new(Mutex::new(fruit)));
        let storage = ScriptStorage::default();

        let (output, special) = storage
            .execute("소각", &mut body, "과일", None, None, None)
            .unwrap();
        assert_eq!(
            output,
            vec!["당신이 \x1b[36m\x1b[0;36m설삼과\x1b[37m를\x1b[37m 소각해버립니다."]
        );
        assert!(body.object.objs.is_empty());
        assert!(matches!(
            special,
            Some(CommandResult::OutputAndSendToUsers(ref own, ref sends))
                if own == "당신이 \x1b[36m\x1b[0;36m설삼과\x1b[37m를\x1b[37m 소각해버립니다."
                    && sends == &vec![(
                        observer.clone(),
                        format!("\r\n\x1b[1m{self_name}\x1b[0;37m{} \x1b[36m\x1b[0;36m설삼과\x1b[37m를\x1b[37m 소각해버립니다.\r\n", han_iga(&self_name))
                    )]
        ));

        let mut unbreakable = Object::new();
        unbreakable.set("이름", "금강석");
        unbreakable.set("반응이름", "돌");
        unbreakable.set("아이템속성", "부수지못함");
        body.object.append(Arc::new(Mutex::new(unbreakable)));
        let blocked = storage
            .execute("부셔", &mut body, "돌", None, None, None)
            .unwrap();
        assert_eq!(blocked.0, vec!["☞ 부셔지지 않네요. ^^"]);
        assert_eq!(body.object.objs.len(), 1);

        body.object.objs.clear();
        for _ in 0..2 {
            let mut pottery = Object::new();
            pottery.set("이름", "도자기");
            pottery.set("반응이름", "그릇");
            body.object.append(Arc::new(Mutex::new(pottery)));
        }
        let broken = storage
            .execute("부셔", &mut body, "그릇 2", None, None, None)
            .unwrap();
        assert_eq!(
            broken.0,
            vec!["당신이 \x1b[36m도자기\x1b[37m 2개를 부셔버립니다."]
        );
        assert!(body.object.objs.is_empty());

        let mut colored = Object::new();
        colored.set("이름", "옥");
        colored.set("반응이름", "옥");
        colored.set("안시", "\x1b[35m");
        body.object.append(Arc::new(Mutex::new(colored)));
        let single = storage
            .execute("부셔", &mut body, "옥 1개", None, None, None)
            .unwrap();
        assert_eq!(
            single.0,
            vec!["당신이 \x1b[36m\x1b[35m옥\x1b[0;37m을\x1b[37m 부셔버립니다."]
        );
        assert!(body.object.objs.is_empty());

        let unique_index = format!("파괴단일-{suffix}");
        let mut unique = Object::new();
        unique.set("이름", "단일옥패");
        unique.set("반응이름", "옥패");
        unique.set("인덱스", unique_index.as_str());
        unique.set("아이템속성", "단일아이템");
        body.object.append(Arc::new(Mutex::new(unique)));
        assert!(crate::oneitem::oneitem_have(&unique_index, &self_name));
        let _ = storage
            .execute("소각", &mut body, "옥패", None, None, None)
            .unwrap();
        assert_eq!(crate::oneitem::oneitem_get(&unique_index), "");

        let _ = std::fs::remove_file(format!("data/user/{self_name}.json"));
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&self_name);
        world.remove_player_position(&observer);
    }

    #[test]
    fn decompose_uses_first_python_merchant_and_preserves_item_ansi_and_shard_bug() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData, RoomObjectRef};

        let suffix = std::process::id();
        let player = format!("분해회귀-{suffix}");
        let zone = format!("분해회귀존-{suffix}");
        let seller_key = format!("{zone}:판매만");
        let buyer_key = format!("{zone}:매입상인");
        let mut seller_data = RawMobData::new();
        seller_data
            .attributes
            .insert("물건판매".into(), serde_json::json!(["물품"]));
        let seller = MobInstance::new(seller_key.clone(), zone.clone(), "1", &seller_data);
        let seller_id = seller.instance_id;
        let mut buyer_data = RawMobData::new();
        buyer_data
            .attributes
            .insert("물건구입".into(), serde_json::json!("고물상 40"));
        let buyer = MobInstance::new(buyer_key.clone(), zone.clone(), "1", &buyer_data);
        let buyer_id = buyer.instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(seller_key.clone(), seller_data);
            world.mob_cache.insert_mob_data(buyer_key.clone(), buyer_data);
            world.mob_cache.add_mob_instance(seller);
            world.mob_cache.add_mob_instance(buyer);
            world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(buyer_id));
            world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(seller_id));
            world.set_player_position(&player, PlayerPosition::new(zone.clone(), "1".into()));
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        let mut weapon = Object::new();
        weapon.set("이름", "자빛검");
        weapon.set("종류", "무기");
        weapon.set("안시", "\x1b[35m");
        weapon.set_option(&HashMap::from([
            ("힘".into(), 1), ("맷집".into(), 1),
            ("민첩성".into(), 1), ("운".into(), 1),
        ]));
        body.object.objs.push(Arc::new(Mutex::new(weapon)));
        let storage = ScriptStorage::default();
        let blocked = storage.execute("분해", &mut body, "모두", None, None, None).unwrap();
        assert_eq!(blocked.0, vec!["☞ 상인이 없어요. ^_^"]);
        assert_eq!(body.object.objs.len(), 1);

        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.remove_instance(&zone, "1", &seller_key);
            world.record_test_room_object(&zone, "1", RoomObjectRef::Mob(buyer_id));
        }
        let decomposed = storage.execute("분해", &mut body, "모두", None, None, None).unwrap();
        assert_eq!(decomposed.0, vec!["당신이 \x1b[35m자빛검\x1b[0;37m 1개를 분해합니다."]);
        assert_eq!(body.object.objs.iter().filter(|item| {
            item.lock().is_ok_and(|item| item.getName() == "강철조각")
        }).count(), 2);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player);
        world.mob_cache.remove_mob(&seller_key);
        world.mob_cache.remove_mob(&buyer_key);
        let _ = std::fs::remove_file(format!("data/user/{player}.json"));
    }

    #[test]
    fn give_commands_preserve_python_lookup_self_and_admin_grant_requests() {
        use crate::command::handler::CommandResult;
        use crate::object::Object;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let giver = format!("전달자-{suffix}");
        let target = format!("수령자-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &giver,
                PlayerPosition::new("전달회귀존".into(), suffix.to_string()),
            );
            world.set_player_position(
                &target,
                PlayerPosition::new("전달회귀존".into(), suffix.to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", giver.as_str());
        body.set("관리자등급", 2000_i64);
        body.set("은전", 10_i64);
        let mut item = Object::new();
        item.set("이름", "청옥패");
        item.set("반응이름", "옥패");
        item.set("아이템속성", "줄수없음");
        body.object.append(Arc::new(Mutex::new(item)));
        let storage = ScriptStorage::default();

        let missing_item_first = storage
            .execute("줘", &mut body, "없는대상 없는물건", None, None, None)
            .unwrap();
        assert_eq!(missing_item_first.0, vec!["☞ 그런 아이템이 소지품에 없어요."]);

        let self_play = storage
            .execute("줘", &mut body, &format!("{giver} 옥패"), None, None, None)
            .unwrap();
        assert_eq!(
            self_play.0,
            vec!["당신이 \x1b[36m청옥패를\x1b[37m 가지고 장난합니다. '@_@'"]
        );
        assert!(self_play.1.is_none());

        let normal_money = storage
            .execute("줘", &mut body, &format!("{target} 은전 7"), None, None, None)
            .unwrap();
        assert!(matches!(
            normal_money.1,
            Some(CommandResult::GiveToPlayer {
                give_silver: Some(7),
                deduct_from_giver: true,
                bypass_item_limits: false,
                ..
            })
        ));

        let admin_money = storage
            .execute("줘줘", &mut body, &format!("{target} 은전 25"), None, None, None)
            .unwrap();
        assert!(matches!(
            admin_money.1,
            Some(CommandResult::GiveToPlayer {
                give_silver: Some(25),
                deduct_from_giver: false,
                bypass_item_limits: false,
                ..
            })
        ));
        assert_eq!(body.get_int("은전"), 10, "요청 단계에서 관리자 은전을 차감하지 않음");

        let admin_item = storage
            .execute("줘줘", &mut body, &format!("{target} 옥패 100"), None, None, None)
            .unwrap();
        assert!(matches!(
            admin_item.1,
            Some(CommandResult::GiveToPlayer {
                give_item: Some((ref name, 1, 100)),
                bypass_item_limits: true,
                ..
            }) if name == "청옥패"
        ));

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&giver);
        world.remove_player_position(&target);
    }

    #[test]
    fn shop_commands_match_python_valid_quantity_and_item_name_format() {
        use crate::world::{get_world_state, PlayerPosition};

        let player_name = format!("상점회귀-{}", std::process::id());
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("힘", 100_i64);
        body.set("은전", 100_i64);
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("낙양성", "6").unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new("낙양성".to_string(), "6".to_string()),
            );
            world.spawn_mobs_for_room("낙양성", "6");
        }

        let storage = ScriptStorage::default();
        let bought = storage
            .execute("구입", &mut body, "수박모자 0", None, None, None)
            .unwrap();
        assert_eq!(body.object.objs.len(), 1);
        assert_eq!(body.get_int("은전"), 91);
        assert_eq!(
            bought.0.join("\r\n"),
            "당신이 \x1b[36m수박모자\x1b[0;37m 1개를 은전 9개에 구입합니다."
        );
        assert!(!bought.0.join("\r\n").contains("수박모자를 1개를"));

        body.set(
            ALIAS_LIST_ATTR,
            encode_alias_entries(&[
                ("체력약".into(), "수박모자".into()),
                ("체력약개수".into(), "3".into()),
            ]),
        );
        let auto_bought = storage
            .execute("구입", &mut body, "체력약", None, None, None)
            .unwrap();
        assert_eq!(body.object.objs.len(), 3);
        assert_eq!(body.get_int("은전"), 73);
        assert_eq!(
            auto_bought.0.join("\r\n"),
            "당신이 \x1b[36m수박모자\x1b[0;37m 2개를 은전 18개에 구입합니다."
        );
        let enough = storage
            .execute("구입", &mut body, "체력약", None, None, None)
            .unwrap();
        assert_eq!(enough.0, vec!["☞ 구매할 물품이 충분합니다. ^_^"]);

        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("낙양성", "43").unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new("낙양성".to_string(), "43".to_string()),
            );
            world.spawn_mobs_for_room("낙양성", "43");
        }
        let sold = storage
            .execute("판매", &mut body, "모두", None, None, None)
            .unwrap();
        assert!(body.object.objs.is_empty());
        assert_eq!(
            sold.0,
            vec!["당신이 \x1b[0;36m수박모자\x1b[37m 1개를 은전 3개에 판매합니다."; 3]
        );

        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player_name);
    }

    #[test]
    fn guard_purchase_consumes_python_requirement_without_charging_silver() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player_name = format!("호위구매-{suffix}");
        let zone = format!("호위구매존-{suffix}");
        let mob_key = format!("{zone}:상인");
        let mut merchant = RawMobData::new();
        merchant.name = "호위상인".into();
        merchant.zone = zone.clone();
        merchant.items_for_sale = vec![("명견".into(), 100), ("사강시".into(), 100)];
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), merchant.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                "1",
                &merchant,
            ));
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".into()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("성격", "정사");
        body.set("은전", 1234_i64);
        let (herb, _) = object_from_item_json("합성1").expect("herb fixture");
        body.object.append(herb);
        let storage = ScriptStorage::default();
        let bought = storage
            .execute("구입", &mut body, "명견", None, None, None)
            .unwrap();
        assert_eq!(bought.0, vec!["당신이 \x1b[36m명견을\x1b[37m 구입합니다."]);
        assert_eq!(body.get_int("은전"), 1234);
        assert_eq!(body.object.objs.len(), 1, "약초 1개를 소모하고 호위 1개를 추가");
        assert_eq!(body.object.objs[0].lock().unwrap().getString("종류"), "호위");
        assert_eq!(body.object.objs[0].lock().unwrap().getInt("체력"), 1000);

        body.set("성격", "정파");
        let faction = storage
            .execute("구입", &mut body, "사강시", None, None, None)
            .unwrap();
        assert_eq!(faction.0, vec!["☞ 해당 호위는 사파원만 사용 가능합니다."]);

        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn receive_command_runs_python_guard_funds_daily_limit_and_success_state() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let player_name = format!("수령회귀-{}", std::process::id());
        let zone = format!("수령회귀존-{}", std::process::id());
        let room = "1";
        let mob_key = format!("{zone}:표두");
        let mut guard_data = RawMobData::new();
        guard_data.name = "표두".to_string();
        guard_data.zone = zone.clone();
        guard_data.gold = 50_000;
        guard_data.reaction_names = vec!["표두".to_string(), "무사".to_string()];
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), guard_data.clone());
            let mut dead_guard = MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                room,
                &guard_data,
            );
            dead_guard.kill();
            world.mob_cache.add_mob_instance(dead_guard);
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), room.to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("레벨", 100_i64);
        body.set("은전", 10_i64);
        body.set("수령액", 0_i64);
        body.set("마지막수령", 0_i64);
        let storage = ScriptStorage::default();

        let dead_missing = storage
            .execute("수령", &mut body, "0", None, None, None)
            .unwrap();
        assert_eq!(dead_missing.0, vec!["☞ 이곳에 표국무사가 없네요."]);
        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                room,
                &guard_data,
            ));

        let invalid = storage
            .execute("수령", &mut body, "0", None, None, None)
            .unwrap();
        assert_eq!(invalid.0, vec!["☞ 은전 1개 이상 입력 하셔야 해요."]);
        body.set("레벨", 501_i64);
        let high_level = storage
            .execute("수령", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(high_level.0, vec!["☞ 충분한 능력이 있어 보이는데요???"]);
        body.set("레벨", 100_i64);
        let greedy = storage
            .execute("수령", &mut body, "10000001", None, None, None)
            .unwrap();
        assert_eq!(greedy.0, vec!["☞ 너무 욕심이 크군요???"]);
        let short = storage
            .execute("수령", &mut body, "50001", None, None, None)
            .unwrap();
        assert_eq!(short.0, vec!["☞ 기부금이 모잘라요^^;"]);
        body.set("수령액", 1_000_000_000_i64);
        let total_limit = storage
            .execute("수령", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(total_limit.0, vec!["☞ 더이상 수령은 곤란해요^^;"]);
        body.set("수령액", 999_999_999_i64);
        let over_limit = storage
            .execute("수령", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(over_limit.0, vec!["☞ 한도 초과에요!!!"]);
        body.set("수령액", 0_i64);
        body.set("마지막수령", chrono::Utc::now().timestamp());
        let too_soon = storage
            .execute("수령", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(too_soon.0, vec!["☞ 또 오셨어요???"]);
        assert_eq!(body.get_int("은전"), 10);
        assert_eq!(body.get_int("수령액"), 0);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, room)
                .into_iter()
                .find(|mob| mob.alive)
                .unwrap()
                .gold,
            50_000
        );
        body.set("마지막수령", 0_i64);

        let success = storage
            .execute("수령", &mut body, "1000개", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("은전"), 1010);
        assert_eq!(body.get_int("수령액"), 1000);
        assert!(body.get_int("마지막수령") > 0);
        assert!(success.0.join("\r\n").contains("은전 1000개를 표국무사에게 수령합니다."));
        let guard_gold = get_world_state()
            .read()
            .unwrap()
            .mob_cache
            .get_all_mobs_in_room(&zone, room)
            .into_iter()
            .find(|mob| mob.alive)
            .unwrap()
            .gold;
        assert_eq!(guard_gold, 49_000);

        let repeated = storage
            .execute("수령", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(repeated.0.join("\r\n"), "☞ 또 오셨어요???");

        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player_name);
        get_world_state()
            .write()
            .unwrap()
            .mob_cache
            .remove_mob(&mob_key);
    }

    #[test]
    fn donation_command_requires_guard_then_clamps_to_carried_silver_like_python() {
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let player_name = format!("기부회귀-{}", std::process::id());
        let zone = format!("기부회귀존-{}", std::process::id());
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("은전", 50_i64);
        {
            get_world_state().write().unwrap().set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
        }
        let storage = ScriptStorage::default();
        let no_guard_before_amount = storage
            .execute("기부", &mut body, "0", None, None, None)
            .unwrap();
        assert_eq!(no_guard_before_amount.0, vec!["☞ 이곳에 표국무사가 없네요."]);

        let mob_key = format!("{zone}:표두");
        let mut guard_data = RawMobData::new();
        guard_data.name = "표두".into();
        guard_data.zone = zone.clone();
        guard_data.gold = 100;
        guard_data.reaction_names = vec!["표두".into(), "표국무사".into()];
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), guard_data.clone());
            let mut dead_guard = MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                "1",
                &guard_data,
            );
            dead_guard.kill();
            world.mob_cache.add_mob_instance(dead_guard);
        }
        let dead_is_missing = storage
            .execute("기부", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(dead_is_missing.0, vec!["☞ 이곳에 표국무사가 없네요."]);
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key.clone(),
                zone.clone(),
                "1",
                &guard_data,
            ));
        }
        let invalid = storage
            .execute("기부", &mut body, "0", None, None, None)
            .unwrap();
        assert_eq!(invalid.0, vec!["☞ 은전 1개 이상 입금 하셔야 해요."]);

        let numeric_prefix = storage
            .execute("기부", &mut body, "10개", None, None, None)
            .unwrap();
        assert_eq!(
            numeric_prefix.0,
            vec!["당신이 은전 10개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m110\x1b[0;37m개 입니다."]
        );
        assert_eq!(body.get_int("은전"), 40);

        let donated = storage
            .execute("기부", &mut body, "100", None, None, None)
            .unwrap();
        assert_eq!(body.get_int("은전"), 0);
        assert_eq!(
            donated.0,
            vec!["당신이 은전 40개를 표국무사에게 기탁합니다.\r\n현재까지 모여진 기부금 총액은 은전 \x1b[1m150\x1b[0;37m개 입니다."]
        );
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, "1")
                .into_iter()
                .find(|mob| mob.alive)
                .unwrap()
                .gold,
            150
        );

        let zero_clamped = storage
            .execute("기부", &mut body, "1", None, None, None)
            .unwrap();
        assert!(zero_clamped.0[0].starts_with("당신이 은전 0개를"));

        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn install_command_creates_reloadable_box_and_python_success_text() {
        use crate::object::Object;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player_name = format!("설치회귀-{suffix}");
        let zone = format!("설치회귀존-{suffix}");
        let room_dir = std::path::Path::new("data/map").join(&zone);
        let room_path = room_dir.join("1.json");
        std::fs::create_dir_all(&room_dir).unwrap();
        std::fs::write(
            &room_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "맵정보": {
                    "이름": "설치 시험방", "존이름": zone,
                    "주인": player_name, "설치리스트": [],
                    "설명": [], "출구": []
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        let mut item = Object::new();
        item.set("이름", "시험보관함");
        item.set("반응이름", "시험보관함\r\n보관함");
        item.set("종류", "설치아이템");
        item.set("보관수량", 10_i64);
        item.set("보관최대수량", 20_i64);
        item.set("보관증가은전", 100_i64);
        body.object.append(Arc::new(Mutex::new(item)));
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room(&zone, "1").unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
        }

        let storage = ScriptStorage::default();
        let installed = storage
            .execute("설치", &mut body, "시험보관함", None, None, None)
            .unwrap();
        assert_eq!(
            installed.0.join("\r\n"),
            "당신이 \x1b[36m시험보관함을\x1b[37m 설치합니다."
        );
        assert!(body.object.objs.is_empty());
        let box_path =
            std::path::Path::new("data/box").join(format!("{player_name}_시험보관함.json"));
        assert!(box_path.exists(), "설치 상자는 loader가 읽는 단일 .json 경로에 저장");
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&box_path).unwrap()).unwrap();
        assert_eq!(saved["상자정보"]["이름"], "시험보관함");

        let _ = std::fs::remove_file(box_path);
        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        let _ = std::fs::remove_dir_all(room_dir);
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player_name);
    }

    #[test]
    fn borrow_and_return_commands_match_python_branch_messages_and_state() {
        use crate::object::Object;
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player_name = format!("대여회귀-{suffix}");
        let zone = format!("대여회귀존-{suffix}");
        let catalog_path = std::env::temp_dir().join(format!("muc-book-command-{suffix}.json"));
        crate::book::save(
            &catalog_path,
            &[serde_json::json!({
                "이름": "철퇴",
                "고유번호": "command-book-id",
                "등록자": "등록자",
                "대여가능": true,
                "대여": "",
                "인덱스": "289",
                "attr": {
                    "이름": "철퇴",
                    "반응이름": "철퇴",
                    "계층": "무기",
                    "종류": "무기"
                }
            })],
        )
        .unwrap();

        let mob_key = format!("{zone}:진영");
        let mut mob_data = RawMobData::new();
        mob_data.name = "진영".to_string();
        mob_data.zone = zone.clone();
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), mob_data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key,
                zone.clone(),
                "1",
                &mob_data,
            ));
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
        }

        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("__시험도서목록경로", catalog_path.to_string_lossy().as_ref());

        let usage = storage.execute("대여", &mut body, "", None, None, None).unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [물품번호] 대여"]);
        let invalid = storage.execute("대여", &mut body, "0", None, None, None).unwrap();
        assert_eq!(invalid.0, vec!["☞ 대여 가능한 물품이 없습니다."]);

        // Python getInt accepts a decimal prefix even when text follows it.
        let borrowed = storage
            .execute("대여", &mut body, "1번", None, None, None)
            .unwrap();
        assert_eq!(borrowed.0, vec!["☞ 대여가 완료 되었습니다."]);
        assert_eq!(body.object.objs.len(), 1);
        assert_eq!(
            body.object.objs[0].lock().unwrap().getString("고유번호"),
            "command-book-id"
        );
        let entry = crate::book::load(&catalog_path).unwrap().remove(0);
        assert!(!crate::book::dict_get_bool(&entry, "대여가능"));
        assert_eq!(crate::book::dict_get_string(&entry, "대여"), player_name);

        let returned = storage
            .execute("반납", &mut body, "철퇴", None, None, None)
            .unwrap();
        assert_eq!(returned.0, vec!["☞ 반납이 완료 되었습니다."]);
        assert!(body.object.objs.is_empty());
        let entry = crate::book::load(&catalog_path).unwrap().remove(0);
        assert!(crate::book::dict_get_bool(&entry, "대여가능"));
        assert_eq!(crate::book::dict_get_string(&entry, "대여"), "");

        let mut ordinary = Object::new();
        ordinary.set("이름", "평범한물품");
        body.object.append(Arc::new(Mutex::new(ordinary)));
        let not_returnable = storage
            .execute("반납", &mut body, "평범한물품", None, None, None)
            .unwrap();
        assert_eq!(not_returnable.0, vec!["☞ 반납 가능한 물품이 아닙니다."]);

        body.object.objs.clear();
        let mut weapon = Object::new();
        weapon.set("이름", "등록시험철퇴");
        weapon.set("반응이름", "등록시험철퇴\r\n시험철퇴");
        weapon.set("인덱스", "289");
        weapon.set("종류", "무기");
        weapon.set("계층", "무기");
        body.object.append(Arc::new(Mutex::new(weapon)));
        let registered = storage
            .execute("등록", &mut body, "등록시험철퇴", None, None, None)
            .unwrap();
        assert_eq!(registered.0, vec!["☞ 등록 되었습니다."]);
        assert!(body.object.objs.is_empty());

        let list = storage
            .execute("대여목록", &mut body, "등록시험철퇴", None, None, None)
            .unwrap();
        assert_eq!(list.0.len(), 1);
        assert!(list.0[0].starts_with("2\t등록시험철퇴\t\t("));
        assert!(list.0[0].ends_with(")\t대여가능"));

        let whitespace_does_not_match = storage
            .execute(
                "대여목록",
                &mut body,
                " 등록시험철퇴 ",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            whitespace_does_not_match.0,
            vec!["☞ 대여가능한 품목이 없어요."]
        );

        let canceled = storage
            .execute("등록취소", &mut body, "2", None, None, None)
            .unwrap();
        assert_eq!(canceled.0, vec!["☞ 등록 취소 되었습니다."]);
        assert_eq!(body.object.objs.len(), 1);
        assert_eq!(body.object.objs[0].lock().unwrap().getName(), "등록시험철퇴");
        assert_eq!(body.object.objs[0].lock().unwrap().getString("고유번호"), "");

        let mut entries = crate::book::load(&catalog_path).unwrap();
        entries[0]["등록자"] = serde_json::Value::String("다른등록자".into());
        crate::book::save(&catalog_path, &entries).unwrap();
        let not_owner = storage
            .execute("등록취소", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(not_owner.0, vec!["☞ 자신이 등록한 물품이 아닙니다."]);

        body.set("관리자등급", 1000_i64);
        let deleted = storage
            .execute("등록삭제", &mut body, "1", None, None, None)
            .unwrap();
        assert_eq!(deleted.0, vec!["☞ 등록 삭제 되었습니다."]);
        assert!(crate::book::load(&catalog_path).unwrap().is_empty());

        let _ = std::fs::remove_file(catalog_path);
        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player_name);
    }

    #[test]
    fn say_includes_speaker_in_python_send_room_output_and_converts_ansi() {
        let storage = ScriptStorage::default();
        let mut body = Body::new();
        body.set("이름", "철수");

        let empty = storage
            .execute("말", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(empty.0, vec!["\r\nSay What???"]);

        let spoken = storage
            .execute("말", &mut body, "{빨}안녕", None, None, None)
            .unwrap();
        assert_eq!(
            spoken.0,
            vec![
                "당신이 말합니다 : '\x1b[31m안녕\x1b[0;40;37m'",
                "\x1b[33m철수\x1b[37m가 말합니다 : '\x1b[31m안녕\x1b[0;40;37m'"
            ]
        );
    }

    #[test]
    fn settings_command_lists_python_cfg_and_toggles_with_python_text() {
        let player_name = format!("설정회귀-{}", std::process::id());
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("설정상태", "자동습득 1\n전음거부 0");
        let storage = ScriptStorage::default();

        let listed = storage
            .execute("설정", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(listed.0[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        assert_eq!(
            listed.0[1],
            "\x1b[47m\x1b[30m◁               설      정      상      태               ▷\x1b[40m\x1b[37m"
        );
        assert!(listed.0.join("\r\n").contains("자동습득         [\x1b[1m설  정\x1b[0;37m]"));
        assert!(listed.0.join("\r\n").contains("전음거부         [비설정]"));
        assert!(listed.0.join("\r\n").contains("자동채널입장     [비설정]"));
        assert_eq!(listed.0.last().unwrap(), "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let enabled = storage
            .execute("설정", &mut body, "전음거부", None, None, None)
            .unwrap();
        assert_eq!(
            enabled.0,
            vec!["☞ 전음거부를 \x1b[1m[설정]\x1b[0;37m 하였습니다."]
        );
        assert!(config_is_enabled(&body.get_string("설정상태"), "전음거부"));

        let disabled = storage
            .execute("설정", &mut body, "전음거부", None, None, None)
            .unwrap();
        assert_eq!(
            disabled.0,
            vec!["☞ 전음거부를 \x1b[1m[비설정]\x1b[0;37m 하였습니다."]
        );
        assert!(!config_is_enabled(&body.get_string("설정상태"), "전음거부"));

        let invalid = storage
            .execute("설정", &mut body, "없는설정", None, None, None)
            .unwrap();
        assert_eq!(invalid.0, vec!["☞ 그런 설정은 없어요. ^^"]);
        let _ = std::fs::remove_file(format!("data/user/{player_name}.json"));
    }

    #[test]
    fn exit_admin_commands_toggle_and_persist_like_python() {
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player_name = format!("출구회귀-{suffix}");
        let zone = format!("출구회귀존-{suffix}");
        let room_dir = std::path::Path::new("data/map").join(&zone);
        let room_path = room_dir.join("1.json");
        std::fs::create_dir_all(&room_dir).unwrap();
        std::fs::write(
            &room_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "맵정보": {
                    "이름": "출구 시험방", "존이름": zone,
                    "설명": ["시험방"], "출구": ["동 2", "비밀$ 3", "비밀 4"], "몹": []
                }
            }))
            .unwrap(),
        )
        .unwrap();
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room(&zone, "1").unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
        }
        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("관리자등급", 1000_i64);
        let storage = ScriptStorage::default();

        let hidden = storage
            .execute("출구숨김", &mut body, "동", None, None, None)
            .unwrap();
        assert_eq!(hidden.0, vec!["☞ 출구가 숨겨졌습니다."]);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
        assert!(json["맵정보"]["출구"].as_array().unwrap().iter().any(|v| v == "동$ 2"));

        let shown = storage
            .execute("출구숨김", &mut body, "동", None, None, None)
            .unwrap();
        assert_eq!(shown.0, vec!["☞ 출구가 드러났습니다."]);

        let wandered = storage
            .execute("맴돌이", &mut body, "비밀", None, None, None)
            .unwrap();
        assert_eq!(wandered.0, vec!["☞ 출구가 맴돌이 되었습니다."]);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
        assert!(json["맵정보"]["출구"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == &format!("비밀$ {zone}:1")));
        assert!(json["맵정보"]["출구"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == &format!("비밀 {zone}:1")));

        let spaced_wander = storage
            .execute("맴돌이", &mut body, " 비밀 ", None, None, None)
            .unwrap();
        assert_eq!(spaced_wander.0, vec!["☞ 그런 출구가 없습니다."]);

        let usage = storage
            .execute("출구제거", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(usage.0, vec!["☞ 사용법: [출구] 출구숨김"]);
        let removed = storage
            .execute("출구제거", &mut body, "동", None, None, None)
            .unwrap();
        assert_eq!(removed.0, vec!["☞ 출구가 제거되었습니다."]);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
        assert!(!json["맵정보"]["출구"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str().is_some_and(|v| v.starts_with("동 "))));

        let _ = std::fs::remove_dir_all(room_dir);
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position(&player_name);
    }

    #[test]
    fn comma_value_command_executes_python_assignment_branches() {
        use crate::object::Object;
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player_name = format!("값값회귀-{suffix}");
        let zone = format!("값값회귀존-{suffix}");
        let mut floor_item = Object::new();
        floor_item.set("이름", "시험석");
        floor_item.set("반응이름", "시험석\r\n돌");
        floor_item.set("무게", 10_i64);
        floor_item.set("설명", "기존");
        let floor_item = Arc::new(Mutex::new(floor_item));

        let mob_key = format!("{zone}:시험몹");
        let mut mob_data = RawMobData::new();
        mob_data.name = "시험몹".to_string();
        mob_data.zone = zone.clone();
        mob_data.max_hp = 100;
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
            world.get_room_objs_mut(&zone, "1").push(floor_item.clone());
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), mob_data.clone());
            world.mob_cache.add_mob_instance(MobInstance::new(
                mob_key,
                zone.clone(),
                "1",
                &mob_data,
            ));
        }

        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();

        let usage = storage.execute("값값", &mut body, "", None, None, None).unwrap();
        assert_eq!(usage.0.len(), 2);
        assert!(usage.0[0].parse::<i64>().is_ok());
        assert_eq!(usage.0[1], "☞ 사용법: [대상],[키],[값] 값설정");

        let numeric = storage
            .execute("값값", &mut body, "시험석,무게,25", None, None, None)
            .unwrap();
        assert_eq!(numeric.0[1], "☞ 값이 설정되었습니다.");
        assert_eq!(floor_item.lock().unwrap().getInt("무게"), 25);

        let string_with_comma = storage
            .execute(
                "값값",
                &mut body,
                "시험석,설명,쉼표,포함",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(string_with_comma.0[1], "☞ 값이 설정되었습니다.");
        assert_eq!(floor_item.lock().unwrap().getString("설명"), "쉼표,포함");

        let invalid = storage
            .execute("값값", &mut body, "시험석,무게,아님", None, None, None)
            .unwrap();
        assert_eq!(invalid.0[1], "☞ 잘못된 값입니다.");
        assert_eq!(floor_item.lock().unwrap().getInt("무게"), 25);

        let mob = storage
            .execute("값값", &mut body, "시험몹,체력,77", None, None, None)
            .unwrap();
        assert_eq!(mob.0[1], "☞ 값이 설정되었습니다.");
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .mob_cache
                .get_all_mobs_in_room(&zone, "1")[0]
                .hp,
            77
        );

        let missing = storage
            .execute("값값", &mut body, "없는것,키,값", None, None, None)
            .unwrap();
        assert_eq!(missing.0[1], "☞ 그런 대상이 없어요!");

        body.temp_mut().insert(
            "_online_room_admin".into(),
            Value::String(
                serde_json::json!([{
                    "name": "다른무림인",
                    "raw_attrs": {"레벨": 10, "설명": "기존설명"}
                }])
                .to_string(),
            ),
        );
        let other_player = storage
            .execute("값값", &mut body, "다른무림인,레벨,33", None, None, None)
            .unwrap();
        assert_eq!(other_player.0[1], "☞ 값이 설정되었습니다.");
        assert_eq!(
            take_admin_set_player_value_request(&mut body),
            Some((
                "다른무림인".to_string(),
                "레벨".to_string(),
                serde_json::json!(33)
            ))
        );
        let invalid_other_player = storage
            .execute(
                "값값",
                &mut body,
                "다른무림인,레벨,숫자아님",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(invalid_other_player.0[1], "☞ 잘못된 값입니다.");
        assert_eq!(take_admin_set_player_value_request(&mut body), None);

        let spaced = storage
            .execute(
                "값설정",
                &mut body,
                "시험석 설명 공백이 들어간 설명",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(spaced.0, vec!["☞ 값이 설정되었습니다."]);
        assert_eq!(
            floor_item.lock().unwrap().getString("설명"),
            "공백이 들어간 설명"
        );

        let long_tail = format!("시험석 설명 짧음 {}", "뒤".repeat(60));
        let accepted_long_tail = storage
            .execute("값설정", &mut body, &long_tail, None, None, None)
            .unwrap();
        assert_eq!(accepted_long_tail.0, vec!["☞ 값이 설정되었습니다."]);
        assert_eq!(
            floor_item.lock().unwrap().getString("설명"),
            format!("짧음 {}", "뒤".repeat(60))
        );
        let too_long_third = format!("시험석 설명 {} 뒤", "긴".repeat(51));
        let rejected_third = storage
            .execute("값설정", &mut body, &too_long_third, None, None, None)
            .unwrap();
        assert_eq!(rejected_third.0, vec!["☞ 너무 길어요!"]);

        let mut inventory_item = Object::new();
        inventory_item.set("이름", "소지시험품");
        inventory_item.set("반응이름", "소지시험품");
        inventory_item.set("무게", 3_i64);
        let inventory_item = Arc::new(Mutex::new(inventory_item));
        body.object.append(inventory_item.clone());
        let inventory = storage
            .execute("값설정", &mut body, "소지시험품 무게 9", None, None, None)
            .unwrap();
        assert_eq!(inventory.0, vec!["☞ 값이 설정되었습니다."]);
        assert_eq!(inventory_item.lock().unwrap().getInt("무게"), 9);

        let room_attr = storage
            .execute("값설정", &mut body, "방 시험속성 여러 단어 값", None, None, None)
            .unwrap();
        assert_eq!(room_attr.0, vec!["☞ 값이 설정되었습니다."]);
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .room_attrs
                .get(&format!("{zone}:1"))
                .and_then(|attrs| attrs.get("시험속성"))
                .map(String::as_str),
            Some("여러 단어 값")
        );

        let invalid_space = storage
            .execute("값설정", &mut body, "소지시험품 무게 숫자아님", None, None, None)
            .unwrap();
        assert_eq!(invalid_space.0, vec!["☞ 잘못된 값입니다."]);
        assert_eq!(inventory_item.lock().unwrap().getInt("무게"), 9);

        let delete_usage = storage
            .execute("값삭제", &mut body, "시험석", None, None, None)
            .unwrap();
        assert_eq!(delete_usage.0, vec!["☞ 사용법: [대상] [키] 값삭제"]);

        let delete_floor = storage
            .execute("값삭제", &mut body, "시험석 설명", None, None, None)
            .unwrap();
        assert_eq!(delete_floor.0, vec!["☞ 값이 삭제되었습니다."]);
        assert!(!floor_item.lock().unwrap().attr.contains_key("설명"));
        let delete_floor_again = storage
            .execute("값삭제", &mut body, "시험석 설명", None, None, None)
            .unwrap();
        assert_eq!(delete_floor_again.0, vec!["☞ 해당 키가 없습니다."]);

        // Python 값삭제는 env.findObjName만 사용하므로 소지품을 찾지 않는다.
        let inventory_not_environment = storage
            .execute("값삭제", &mut body, "소지시험품 무게", None, None, None)
            .unwrap();
        assert_eq!(inventory_not_environment.0, vec!["☞ 그런 대상이 없어요!"]);
        assert_eq!(inventory_item.lock().unwrap().getInt("무게"), 9);

        get_world_state()
            .write()
            .unwrap()
            .get_room_attrs_mut(&zone, "1")
            .insert("시험 속성".into(), "값".into());
        let delete_room_spaced_key = storage
            .execute("값삭제", &mut body, "방 시험 속성", None, None, None)
            .unwrap();
        assert_eq!(delete_room_spaced_key.0, vec!["☞ 값이 삭제되었습니다."]);
        assert!(!get_world_state()
            .read()
            .unwrap()
            .room_attrs
            .get(&format!("{zone}:1"))
            .is_some_and(|attrs| attrs.contains_key("시험 속성")));

        let delete_missing = storage
            .execute("값삭제", &mut body, "없는대상 없는키", None, None, None)
            .unwrap();
        assert_eq!(delete_missing.0, vec!["☞ 그런 대상이 없어요!"]);

        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").clear();
        world.remove_player_position(&player_name);
    }

    #[test]
    fn who_gives_item_preserves_python_mob_registration_order() {
        use crate::world::{get_world_state, RawMobData};

        let suffix = std::process::id();
        let item_key = format!("누가주나시험-{suffix}");
        let first_key = format!("순서시험:{suffix}-첫째");
        let second_key = format!("순서시험:{suffix}-둘째");
        let merchant_key = format!("순서시험:{suffix}-상인");
        let duplicate_key = format!("순서시험:{suffix}-중복");
        let mut first = RawMobData::new();
        first.name = "첫째몹".into();
        first.drop_items.push((item_key.clone(), 1, 100, 1));
        let mut second = RawMobData::new();
        second.name = "둘째몹".into();
        second.use_items.push((item_key.clone(), 1, 1, 1));
        let mut merchant = RawMobData::new();
        merchant.name = "판매만하는몹".into();
        merchant.items_for_sale.push((item_key.clone(), 100));
        let mut duplicate = RawMobData::new();
        duplicate.name = "중복몹".into();
        duplicate.drop_items.push((item_key.clone(), 1, 100, 1));
        duplicate.use_items.push((item_key.clone(), 1, 100, 1));
        {
            let mut world = get_world_state().write().unwrap();
            world.mob_cache.insert_mob_data(first_key.clone(), first);
            world.mob_cache.insert_mob_data(second_key.clone(), second);
            world
                .mob_cache
                .insert_mob_data(merchant_key.clone(), merchant);
            world
                .mob_cache
                .insert_mob_data(duplicate_key.clone(), duplicate);
        }

        let mut body = Body::new();
        body.set("이름", "누가주나회귀");
        body.set("관리자등급", 1000_i64);
        let output = ScriptStorage::default()
            .execute("누가주나", &mut body, &item_key, None, None, None)
            .unwrap();
        assert_eq!(
            output.0,
            vec![
                format!("첫째몹 : {first_key}"),
                format!("둘째몹 : {second_key}"),
                format!("중복몹 : {duplicate_key}"),
                format!("중복몹 : {duplicate_key}")
            ]
        );
        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&first_key);
        world.mob_cache.remove_mob(&second_key);
        world.mob_cache.remove_mob(&merchant_key);
        world.mob_cache.remove_mob(&duplicate_key);
    }

    #[test]
    fn save_object_command_handles_room_item_mob_and_room_as_valid_python_targets() {
        use crate::object::Object;
        use crate::world::{get_world_state, MobInstance, PlayerPosition, RawMobData};

        let suffix = std::process::id();
        let player_name = format!("저장회귀-{suffix}");
        let zone = format!("저장회귀존-{suffix}");
        let item_key = format!("저장회귀아이템-{suffix}");
        let mob_file = format!("저장회귀몹-{suffix}");
        let mob_key = format!("{zone}:{mob_file}");
        let room_dir = std::path::Path::new("data/map").join(&zone);
        let room_path = room_dir.join("1.json");
        let mob_dir = std::path::Path::new("data/mob").join(&zone);
        let mob_path = mob_dir.join(format!("{mob_file}.json"));
        let item_path = std::path::Path::new("data/item").join(format!("{item_key}.json"));
        std::fs::create_dir_all(&room_dir).unwrap();
        std::fs::create_dir_all(&mob_dir).unwrap();
        std::fs::write(
            &room_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "맵정보": {"이름": "저장 시험방", "존이름": zone, "설명": [], "출구": []}
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            &mob_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "몹정보": {"이름": "저장시험몹", "체력": 100, "최고체력": 100}
            }))
            .unwrap(),
        )
        .unwrap();

        let mut floor_item = Object::new();
        floor_item.set("이름", "저장시험석");
        floor_item.set("반응이름", "저장시험석\r\n시험석");
        floor_item.set("인덱스", item_key.as_str());
        floor_item.set("종류", "일반");
        floor_item.set("설명1", "변경된 설명");
        let floor_item = Arc::new(Mutex::new(floor_item));

        let mut mob_data = RawMobData::new();
        mob_data.name = "저장시험몹".into();
        mob_data.zone = zone.clone();
        mob_data.max_hp = 100;
        mob_data
            .attributes
            .insert("이름".into(), serde_json::Value::String("저장시험몹".into()));
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player_name,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
            world.get_room_objs_mut(&zone, "1").push(floor_item);
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), mob_data.clone());
            let mut instance = MobInstance::new(mob_key.clone(), zone.clone(), "1", &mob_data);
            instance.hp = 73;
            instance.runtime_attrs.insert("시험값".into(), Value::Int(9));
            world.mob_cache.add_mob_instance(instance);
        }

        let mut body = Body::new();
        body.set("이름", player_name.as_str());
        body.set("관리자등급", 2000_i64);
        let storage = ScriptStorage::default();

        let room_saved = storage
            .execute("오브젝트저장", &mut body, "", None, None, None)
            .unwrap();
        assert_eq!(
            room_saved.0,
            vec![format!("* data/map/{zone}/1.json 저장되었습니다.")]
        );
        let item_saved = storage
            .execute("오브젝트저장", &mut body, "시험석", None, None, None)
            .unwrap();
        assert_eq!(
            item_saved.0,
            vec![format!("* data/item/{item_key}.json 저장되었습니다.")]
        );
        let item_json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&item_path).unwrap()).unwrap();
        assert_eq!(item_json["아이템정보"]["설명1"], "변경된 설명");

        let mob_saved = storage
            .execute("오브젝트저장", &mut body, "저장시험몹", None, None, None)
            .unwrap();
        assert_eq!(
            mob_saved.0,
            vec![format!("* data/mob/{zone}/{mob_file}.json 저장되었습니다.")]
        );
        let mob_json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mob_path).unwrap()).unwrap();
        assert_eq!(mob_json["몹정보"]["체력"], 73);
        assert_eq!(mob_json["몹정보"]["시험값"], 9);

        let missing = storage
            .execute("오브젝트저장", &mut body, "없는대상", None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);

        let _ = std::fs::remove_file(item_path);
        let _ = std::fs::remove_dir_all(room_dir);
        let _ = std::fs::remove_dir_all(mob_dir);
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").clear();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn attributes_command_reads_room_json_then_room_object_and_inventory_fallback() {
        use crate::object::Object;
        use crate::world::{get_world_state, PlayerPosition};

        let suffix = std::process::id();
        let player = format!("속성회귀-{suffix}");
        let zone = format!("속성회귀존-{suffix}");
        let room_dir = std::path::Path::new("data/map").join(&zone);
        std::fs::create_dir_all(&room_dir).unwrap();
        std::fs::write(
            room_dir.join("1.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "맵정보": {"이름": "속성 시험방", "설명": ["첫줄", "둘째줄"], "출구": []}
            }))
            .unwrap(),
        )
        .unwrap();
        let mut floor = Object::new();
        floor.set("이름", "바닥옥패");
        floor.set("반응이름", "옥패");
        floor.set("시험수치", 17_i64);
        let floor = Arc::new(Mutex::new(floor));
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &player,
                PlayerPosition::new(zone.clone(), "1".to_string()),
            );
            world.get_room_objs_mut(&zone, "1").push(floor);
        }
        let mut body = Body::new();
        body.set("이름", player.as_str());
        body.set("관리자등급", 1000_i64);
        let mut inventory = Object::new();
        inventory.set("이름", "소지옥패");
        inventory.set("반응이름", "소지패");
        inventory.set("문자값", "보관값");
        body.object.append(Arc::new(Mutex::new(inventory)));
        let storage = ScriptStorage::default();

        let room = storage
            .execute("속성", &mut body, "", None, None, None)
            .unwrap()
            .0
            .join("");
        assert!(room.contains("#설명\r\n첫줄\r\n둘째줄\r\n\r\n"));
        assert!(room.contains("#이름\r\n속성 시험방\r\n\r\n"));

        let floor = storage
            .execute("속성", &mut body, "옥패", None, None, None)
            .unwrap()
            .0
            .join("");
        assert!(floor.contains("#시험수치\r\n17\r\n\r\n"));
        let inventory = storage
            .execute("속성", &mut body, "소지패", None, None, None)
            .unwrap()
            .0
            .join("");
        assert!(inventory.contains("#문자값\r\n보관값\r\n\r\n"));

        let missing = storage
            .execute("속성", &mut body, "없는대상", None, None, None)
            .unwrap();
        assert_eq!(missing.0, vec!["☞ 그런 대상이 없어요!"]);

        let _ = std::fs::remove_dir_all(room_dir);
        let mut world = get_world_state().write().unwrap();
        world.get_room_objs_mut(&zone, "1").clear();
        world.remove_player_position(&player);
    }

    fn adult_channel_test_body(name: &str, nickname: &str, config: &str) -> Body {
        let mut body = Body::new();
        body.set("이름", name);
        body.set("무림별호", nickname);
        body.set("성격", "정파");
        body.set("소속", "");
        body.set("투명상태", 0_i64);
        body.set("설정상태", config);
        body
    }

    #[test]
    fn adult_channel_scripts_use_ordered_membership_for_join_leave_chat_and_list() {
        let storage = ScriptStorage::default();
        let self_id = "127.0.0.1:31901";
        let other_id = "127.0.0.1:31902";
        let mut actor = adult_channel_test_body("입장인", "푸른별", "외침거부 0");
        let other = adult_channel_test_body("기존인", "", "외침거부 0");
        let other_map = build_adult_channel_member_snapshot(other_id.to_string(), &other, true, 1);

        set_precomputed_adult_channel(vec![other_map.clone()], self_id.to_string(), false);
        let (outputs, special) = storage
            .execute("채널입장", &mut actor, "", None, None, None)
            .unwrap();
        assert!(outputs.is_empty());
        assert!(special.is_none());
        let (action, deliveries) = take_adult_channel_requests(&mut actor);
        assert_eq!(action.as_deref(), Some("join"));
        assert_eq!(deliveries.len(), 2);
        assert_eq!(deliveries[0].member_id, self_id);
        assert_eq!(deliveries[0].raw_text, "☞ 채널에 입장합니다.\r\n\r\n");
        assert_eq!(deliveries[1].member_id, other_id);
        assert!(deliveries[1].raw_text.starts_with("\r\n\x1b[1;31m①⑨"));
        assert!(deliveries[1]
            .raw_text
            .ends_with("\r\n\x1b[0;37;40m[ 0/0, 0/0 ] "));
        clear_precomputed_all_online();

        actor
            .temp_mut()
            .insert(ADULT_CHANNEL_AUTO_JOIN_REQUEST.to_string(), Value::Int(1));
        set_precomputed_adult_channel(vec![other_map.clone()], self_id.to_string(), false);
        storage
            .execute("채널입장", &mut actor, "", None, None, None)
            .unwrap();
        let (action, deliveries) = take_adult_channel_requests(&mut actor);
        assert_eq!(action.as_deref(), Some("join"));
        assert_eq!(deliveries[0].member_id, other_id);
        assert_eq!(deliveries[1].member_id, self_id);
        clear_precomputed_all_online();

        set_precomputed_adult_channel(vec![], self_id.to_string(), true);
        let (outputs, _) = storage
            .execute("채널입장", &mut actor, "", None, None, None)
            .unwrap();
        assert_eq!(outputs, vec!["☞ 이미 입장하셨습니다.\r\n"]);
        assert_eq!(take_adult_channel_requests(&mut actor), (None, vec![]));
        clear_precomputed_all_online();

        set_precomputed_adult_channel(vec![], self_id.to_string(), false);
        let (outputs, _) = storage
            .execute("채널퇴장", &mut actor, "", None, None, None)
            .unwrap();
        assert_eq!(outputs, vec!["☞ 먼저 채널에 입장하세요.\r\n"]);
        assert_eq!(take_adult_channel_requests(&mut actor), (None, vec![]));
        clear_precomputed_all_online();

        let self_map = build_adult_channel_member_snapshot(self_id.to_string(), &actor, true, 1);
        set_precomputed_adult_channel(
            vec![other_map.clone(), self_map.clone()],
            self_id.to_string(),
            true,
        );
        let (outputs, _) = storage
            .execute("채널퇴장", &mut actor, "", None, None, None)
            .unwrap();
        assert!(outputs.is_empty());
        let (action, deliveries) = take_adult_channel_requests(&mut actor);
        assert_eq!(action.as_deref(), Some("leave"));
        assert_eq!(deliveries.len(), 2);
        assert_eq!(deliveries[0].member_id, other_id);
        assert_eq!(deliveries[1].member_id, self_id);
        assert_eq!(deliveries[1].raw_text, "☞ 채널에서 퇴장합니다.\r\n\r\n");
        clear_precomputed_all_online();

        let inactive = adult_channel_test_body("잠든인", "잠든별", "외침거부 0");
        let refusing = adult_channel_test_body("거부인", "거부별", "외침거부 1");
        let inactive_map =
            build_adult_channel_member_snapshot("127.0.0.1:31903".into(), &inactive, false, 1);
        let refusing_map =
            build_adult_channel_member_snapshot("127.0.0.1:31904".into(), &refusing, true, 1);
        set_precomputed_adult_channel(
            vec![
                self_map.clone(),
                inactive_map,
                refusing_map,
                other_map.clone(),
            ],
            self_id.to_string(),
            true,
        );
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                "입장인",
                PlayerPosition::new("성인채널검증존".into(), "1".into()),
            );
        }
        storage
            .execute("채널잡담", &mut actor, "안녕하세요", None, None, None)
            .unwrap();
        let (action, deliveries) = take_adult_channel_requests(&mut actor);
        assert!(action.is_none());
        assert_eq!(deliveries.len(), 2);
        assert_eq!(deliveries[0].member_id, self_id);
        assert_eq!(deliveries[1].member_id, other_id);
        assert!(deliveries[0].raw_text.ends_with("안녕하세요\r\n\r\n"));
        assert_eq!(deliveries[1].raw_text.matches("안녕하세요").count(), 1);
        clear_precomputed_all_online();
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position("입장인");

        set_precomputed_adult_channel(vec![other_map, self_map], self_id.to_string(), true);
        let (outputs, _) = storage
            .execute("채널누구", &mut actor, "", None, None, None)
            .unwrap();
        assert_eq!(outputs.len(), 6);
        assert_eq!(outputs[0], "┌─────────────────────────────────────┐");
        assert!(outputs[3].contains("무명객"));
        assert!(outputs[3].contains("푸른별"));
        assert_eq!(outputs[5], " ★ 총 2명의 무림인이 활동하고 있습니다.\r\n");
        clear_precomputed_all_online();
    }

    #[test]
    fn adult_channel_disconnect_uses_leave_script_without_self_confirmation() {
        let storage = ScriptStorage::default();
        let self_id = "127.0.0.1:31911";
        let other_id = "127.0.0.1:31912";
        let mut actor = adult_channel_test_body("퇴장인", "", "");
        let other = adult_channel_test_body("남은인", "", "");
        let self_map = build_adult_channel_member_snapshot(self_id.to_string(), &actor, true, 1);
        let other_map = build_adult_channel_member_snapshot(other_id.to_string(), &other, true, 1);
        set_precomputed_adult_channel(vec![self_map, other_map], self_id.to_string(), true);
        actor
            .temp_mut()
            .insert(ADULT_CHANNEL_DISCONNECT_REQUEST.to_string(), Value::Int(1));

        storage
            .execute("채널퇴장", &mut actor, "", None, None, None)
            .unwrap();
        let (action, deliveries) = take_adult_channel_requests(&mut actor);
        assert_eq!(action.as_deref(), Some("leave"));
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].member_id, other_id);
        assert!(deliveries[0].raw_text.contains("퇴장하셨습니다."));
        clear_precomputed_all_online();
    }
}
