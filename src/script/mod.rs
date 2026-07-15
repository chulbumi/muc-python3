//! Rhai scripting engine for MUD server
//!
//! Provides hot-reloadable scripting support using Rhai.
//! Scripts are stored in cmds/ directory and automatically reloaded on change.

#![allow(clippy::type_complexity)]
#![allow(static_mut_refs)]

mod admin_combat;
pub(crate) use admin_combat::clear_summon_combat;
mod anger;
mod box_commands;
mod cast;
mod chat_history;
pub(crate) use cast::skill_up_python;
pub(crate) mod combat_commands;
mod drop_item;
mod fixture;
pub(crate) use fixture::{try_fixture_event, visible_fixture_short_lines};
pub(crate) mod inventory_compat;
pub(crate) use inventory_compat::mark_item_field_as_json_array;
mod item_event;
pub(crate) use item_event::try_item_event;
mod movement;
mod party;
mod requests;
mod return_home;
mod search_body;
pub(crate) use requests::*;

#[cfg(test)]
pub(crate) use box_commands::register_installed_box;
pub(crate) use box_commands::{
    build_box_observer_snapshot, clear_precomputed_box_context, set_precomputed_box_context,
    take_box_deliveries, BoxDelivery,
};
pub(crate) use cast::{clear_cast_room_players, set_cast_room_players, CastRoomPlayerRef};
pub(crate) use movement::{immediate_exit_destinations, python_map_explore};
pub(crate) fn python_item_field_contains(item: &Object, field: &str, wanted: &str) -> bool {
    inventory_compat::python_item_field_contains(item, field, wanted)
}
pub(crate) use party::{
    build_party_nonplayer_snapshot, build_party_person_snapshot,
    installed_box_party_snapshot_by_pointer, installed_box_party_snapshots, missing_party_person,
    set_precomputed_party_context, take_party_requests, PartyDelivery, PARTY_DISCONNECT_REQUEST,
};
#[cfg(test)]
pub(crate) use party::{
    clear_precomputed_party_context, find_follow_player_for_test,
    precomputed_party_context_for_test,
};

use encoding::{EncoderTrap, Encoding};
use rand::Rng;
use rhai::{Dynamic, Engine, Scope, AST};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(test)]
static ONEITEM_COMMAND_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

#[cfg(test)]
pub(crate) use chat_history::CHAT_HISTORY;
pub(crate) use chat_history::{
    chat_history_snapshot, record_chat_history, record_chat_history_limit,
};
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
    format_exits_long, format_room_header, get_world_state, Fixture, MobInstance, PlayerPosition,
    RawMobData, RoomObjectRef, WorldState,
};
use std::time::{Duration, Instant};

/// Python `Player.enterRoom(..., "소환", "소환")` destination guards.
///
/// This is intentionally shared by the Rhai efun and the network-side
/// `$위치이동` completion path.  The latter already owns the event's wire
/// presentation, but must not bypass the state checks merely because the
/// directive was collected before the movement is applied.
fn check_summon_destination_impl(
    body: &Body,
    zone: &str,
    room: &str,
    same_place_is_noop: bool,
) -> String {
    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return "fail".into(),
    };
    let room_arc = match world.room_cache.get_room(zone, room) {
        Ok(room) => room,
        Err(_) => return "fail".into(),
    };
    if same_place_is_noop
        && world
            .get_player_position(&body.get_name())
            .is_some_and(|position| position.zone == zone && position.room == room)
    {
        return "same_place".into();
    }
    let room_guard = match room_arc.read() {
        Ok(room) => room,
        Err(_) => return "fail".into(),
    };
    let attrs = world
        .room_attrs
        .get(&format!("{zone}:{room}"))
        .cloned()
        .unwrap_or_default();
    let raw_info = std::fs::read_to_string(format!("data/map/{zone}/{room}.json"))
        .ok()
        .and_then(|source| serde_json::from_str::<serde_json::Value>(&source).ok())
        .and_then(|root| {
            root.get("맵정보")
                .and_then(|value| value.as_object())
                .cloned()
        })
        .unwrap_or_default();
    let raw_int = |key: &str, fallback: i64| {
        raw_info.get(key).map_or(fallback, |value| match value {
            serde_json::Value::Number(number) => number.as_i64().unwrap_or(fallback),
            serde_json::Value::String(text) => text.trim().parse().unwrap_or(fallback),
            _ => fallback,
        })
    };
    let int_attr = |key: &str, fallback: i64| {
        attrs
            .get(key)
            .and_then(|value| value.trim().parse::<i64>().ok())
            .unwrap_or(fallback)
    };
    let level = body.get_int("레벨");
    let level_upper = int_attr("레벨상한", raw_int("레벨상한", room_guard.level_upper));
    let level_lower = int_attr("레벨제한", raw_int("레벨제한", room_guard.level_limit));
    let strength_upper = int_attr("힘상한제한", raw_int("힘상한제한", 0));
    let dexterity_upper = int_attr("민첩상한제한", raw_int("민첩상한제한", 0));
    if (level_upper > 0 && level_upper < level)
        || level_lower > level
        || (strength_upper > 0 && strength_upper < body.get_int("힘"))
        || (dexterity_upper > 0 && dexterity_upper < body.get_dex())
    {
        return "pressure".into();
    }
    let property_limit = room_guard
        .properties
        .iter()
        .find_map(|property| {
            let mut words = property.split_whitespace();
            (words.next() == Some("인원제한"))
                .then(|| words.next()?.parse::<i64>().ok())
                .flatten()
        })
        .unwrap_or(0);
    drop(room_guard);
    if property_limit > 0 && world.get_players_in_room(zone, room).len() as i64 >= property_limit {
        return "room_full".into();
    }
    let properties = room_arc
        .read()
        .map(|room| room.properties.clone())
        .unwrap_or_default();
    let personality = body.get_string("성격");
    if properties.iter().any(|property| property == "사파출입금지") && personality == "사파"
    {
        return "evil_forbidden".into();
    }
    if properties.iter().any(|property| property == "정파출입금지") && personality == "정파"
    {
        return "good_forbidden".into();
    }
    let guild_owner = attrs.get("방파주인").cloned().unwrap_or_else(|| {
        raw_info
            .get("방파주인")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    });
    if !guild_owner.is_empty() && guild_owner != body.get_string("소속") {
        return "guild_forbidden".into();
    }
    String::new()
}

/// Destination guards used by administrative `이동`, whose Python command
/// reports a same-room request before calling `Player.enterRoom`.
pub(crate) fn check_summon_destination(body: &Body, zone: &str, room: &str) -> String {
    check_summon_destination_impl(body, zone, room, true)
}

/// Destination guards for legacy event `$위치이동`.  Unlike `이동`, Python
/// `doEvent()` always calls `enterRoom`, including when the destination is
/// the current room, so it must not short-circuit as `same_place`.
pub(crate) fn check_event_summon_destination(body: &Body, zone: &str, room: &str) -> String {
    check_summon_destination_impl(body, zone, room, false)
}

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

fn euc_kr_len(value: &str) -> i64 {
    let visible = strip_ansi_like_python(value);
    encoding::all::WINDOWS_949
        .encode(&visible, EncoderTrap::Replace)
        .map_or(visible.len(), |encoded| encoded.len()) as i64
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

fn help_text_from_root(root: &serde_json::Value, topic: &str) -> String {
    root.get("도움말")
        .and_then(|outer| outer.get(topic))
        .and_then(serde_json::Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join("\r\n")
        })
        .unwrap_or_default()
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
    let mut options: Vec<(String, i64)> = Vec::new();
    let mut attempts = 0;
    let mut valuable = false;
    while options.len() < option_count as usize {
        attempts += 1;
        if attempts > 8 {
            return false;
        }
        let option_name = names[roll(0, names.len() as i64 - 1) as usize];
        if options.iter().any(|(name, _)| name == option_name) {
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
        options.push((option_name.to_string(), value));
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
    item.set(
        "옵션",
        options
            .iter()
            .map(|(name, value)| format!("{name} {value}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    // Python Item.setOption() writes a list of option lines.  Retain that
    // JSON shape even though runtime Object attributes use newline strings.
    inventory_compat::mark_item_field_as_json_array(item, "옵션");
    if option_count > 2 || valuable {
        item.setAttr("아이템속성", "버리지못함");
        item.setAttr("아이템속성", "줄수없음");
        inventory_compat::mark_item_field_as_json_array(item, "아이템속성");
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

fn parse_int_prefix(value: &str) -> i64 {
    let value = value.trim();
    if value.is_empty() {
        return 0;
    }
    if let Ok(value) = value.parse::<i64>() {
        return value;
    }
    // Python getInt first calls int(s). Python integer literals accept a
    // single underscore between decimal digits (and an optional sign), while
    // Rust's FromStr does not. Only after that full parse fails does getInt
    // fall back to the leading decimal prefix.
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    let python_underscored_integer = !unsigned.is_empty()
        && unsigned
            .split('_')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));
    if python_underscored_integer && unsigned.contains('_') {
        if let Ok(parsed) = value.replace('_', "").parse::<i64>() {
            return parsed;
        }
    }
    let mut chars = value.chars();
    if !chars
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        return 0;
    }
    let digits: String = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

fn strict_int_result(value: &str) -> rhai::Map {
    let mut result = rhai::Map::new();
    let raw = value.trim();
    let unsigned = raw.strip_prefix(['+', '-']).unwrap_or(raw);
    let valid_underscores = !unsigned.is_empty()
        && !unsigned.starts_with('_')
        && !unsigned.ends_with('_')
        && !unsigned.contains("__")
        && unsigned
            .chars()
            .all(|character| character.is_ascii_digit() || character == '_');
    let normalized = valid_underscores.then(|| raw.replace('_', ""));
    match normalized
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
    {
        Some(value) => {
            result.insert("valid".into(), Dynamic::from(true));
            result.insert("value".into(), Dynamic::from(value));
        }
        None => {
            result.insert("valid".into(), Dynamic::from(false));
            result.insert("value".into(), Dynamic::from(0_i64));
        }
    }
    result
}

fn room_has_insurance_agent(body: &Body) -> bool {
    room_has_object_named(body, "표두")
}

/// Whether Python `Room.findObjName(name)` would find any visible room object.
///
/// The book commands misleadingly name the result `mob`, but never call
/// `is_mob`: a matching player, item, or box also satisfies their location
/// guard. Preserve that observable behavior instead of narrowing it to mobs.
fn room_has_object_named(body: &Body, wanted: &str) -> bool {
    let Ok(world) = get_world_state().read() else {
        return false;
    };
    let Some(position) = world.get_player_position(&body.get_name()) else {
        return false;
    };
    let aliases_match = |name: &str, aliases: &str| {
        name == wanted
            || reaction_names(aliases)
                .iter()
                .any(|alias| alias == wanted || alias.starts_with(wanted))
    };

    if world
        .get_players_in_room(&position.zone, &position.room)
        .iter()
        .any(|name| name == wanted)
    {
        return true;
    }
    if world.summoned_users().iter().any(|user| {
        user.position.zone == position.zone
            && user.position.room == position.room
            && user.body.get_int("투명상태") != 1
            && aliases_match(&user.body.get_name(), &user.body.get_string("반응이름"))
    }) {
        return true;
    }
    for item in world.get_room_objs(&position.zone, &position.room) {
        let Ok(item) = item.lock() else { continue };
        if item.getInt("투명상태") != 1
            && aliases_match(&item.getName(), &item.getString("반응이름"))
        {
            return true;
        }
    }
    if let Some(boxes) = box_commands::installed_boxes_for_room(&position.zone, &position.room) {
        for item in boxes {
            let Ok(item) = item.lock() else { continue };
            if item.getInt("투명상태") != 1
                && aliases_match(&item.getName(), &item.getString("반응이름"))
            {
                return true;
            }
        }
    }
    for mob in world
        .mob_cache
        .get_all_mobs_in_room(&position.zone, &position.room)
    {
        if !mob.alive || mob.act == 2 || mob.act == 3 {
            continue;
        }
        let Some(data) = world.get_mob_data(&mob.mob_key) else {
            continue;
        };
        let transparent = mob
            .runtime_attrs
            .get("투명상태")
            .is_some_and(|value| matches!(value, Value::Int(1)))
            || data
                .attributes
                .get("투명상태")
                .and_then(serde_json::Value::as_i64)
                == Some(1);
        if transparent {
            continue;
        }
        if data.name == wanted
            || data
                .reaction_names
                .iter()
                .any(|name| name == wanted || name.starts_with(wanted))
        {
            return true;
        }
    }
    false
}

#[derive(Clone)]
enum RoomFundTarget {
    Mob(u64, String),
    Object(Arc<Mutex<Object>>),
    Player(String),
    Summoned(u64),
}

/// First mutable object selected by Python Room.findObjName for the donation
/// commands. Player-backed targets require the live network Body and are not
/// representable inside this synchronous efun, while mobs and room objects are.
fn find_room_fund_target(body: &Body, wanted: &str) -> Option<RoomFundTarget> {
    if let Some(selected) = select_python_room_object(body, wanted) {
        let world = get_world_state().read().ok()?;
        let position = world.get_player_position(&body.get_name())?;
        let floor = world.get_room_objs(&position.zone, &position.room);
        let installed = box_commands::installed_boxes_for_room(&position.zone, &position.room)
            .unwrap_or_default();
        return match selected {
            RoomObjectRef::Player(name) => Some(RoomFundTarget::Player(name)),
            RoomObjectRef::SummonedUser(id) => Some(RoomFundTarget::Summoned(id)),
            RoomObjectRef::Mob(id) => world
                .mob_cache
                .get_all_mobs_in_room(&position.zone, &position.room)
                .iter()
                .find(|mob| mob.instance_id == id)
                .map(|mob| RoomFundTarget::Mob(id, mob.mob_key.clone())),
            RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => floor
                .iter()
                .chain(installed.iter())
                .find(|object| Arc::as_ptr(object) as usize == pointer)
                .cloned()
                .map(RoomFundTarget::Object),
            RoomObjectRef::InstalledBox(ordinal) => {
                installed.get(ordinal).cloned().map(RoomFundTarget::Object)
            }
            RoomObjectRef::Fixture(_) => None,
        };
    }
    let world = get_world_state().read().ok()?;
    let position = world.get_player_position(&body.get_name())?;
    let floor = world.get_room_objs(&position.zone, &position.room);
    let mobs = world
        .mob_cache
        .get_all_mobs_in_room(&position.zone, &position.room);
    let installed =
        box_commands::installed_boxes_for_room(&position.zone, &position.room).unwrap_or_default();
    let object_matches = |object: &Object| {
        object.getInt("투명상태") != 1
            && (object.getName() == wanted
                || reaction_names(&object.getString("반응이름"))
                    .iter()
                    .any(|alias| alias == wanted || alias.starts_with(wanted)))
    };
    let mob_matches = |id: u64| {
        let mob = mobs.iter().find(|mob| mob.instance_id == id)?;
        if !mob.alive || mob.act == 2 || mob.act == 3 {
            return None;
        }
        let data = world.mob_cache.get_mob(&mob.mob_key)?;
        let transparent = mob
            .runtime_attrs
            .get("투명상태")
            .is_some_and(|value| matches!(value, Value::Int(1)))
            || data
                .attributes
                .get("투명상태")
                .and_then(serde_json::Value::as_i64)
                == Some(1);
        (!transparent
            && (data.name == wanted
                || data
                    .reaction_names
                    .iter()
                    .any(|alias| alias == wanted || alias.starts_with(wanted))))
        .then(|| RoomFundTarget::Mob(id, mob.mob_key.clone()))
    };

    for entry in world.get_room_object_order(&position.zone, &position.room) {
        match entry {
            crate::world::RoomObjectRef::Mob(id) => {
                if let Some(target) = mob_matches(id) {
                    return Some(target);
                }
            }
            crate::world::RoomObjectRef::FloorItem(pointer)
            | crate::world::RoomObjectRef::Box(pointer) => {
                let Some(object) = floor
                    .iter()
                    .find(|object| Arc::as_ptr(object) as usize == pointer)
                    .cloned()
                else {
                    continue;
                };
                let matches = object.lock().is_ok_and(|object| object_matches(&object));
                if matches {
                    return Some(RoomFundTarget::Object(object));
                }
            }
            crate::world::RoomObjectRef::InstalledBox(ordinal) => {
                let Some(object) = installed.get(ordinal).cloned() else {
                    continue;
                };
                let matches = object.lock().is_ok_and(|object| object_matches(&object));
                if matches {
                    return Some(RoomFundTarget::Object(object));
                }
            }
            crate::world::RoomObjectRef::Player(_)
            | crate::world::RoomObjectRef::SummonedUser(_)
            | crate::world::RoomObjectRef::Fixture(_) => {}
        }
    }
    // Directly constructed legacy/test rooms may predate the unified order.
    // Preserve their Python list behavior with the cache's insertion order.
    for mob in &mobs {
        if let Some(target) = mob_matches(mob.instance_id) {
            return Some(target);
        }
    }
    for object in floor {
        let matches = object.lock().is_ok_and(|object| object_matches(&object));
        if matches {
            return Some(RoomFundTarget::Object(object));
        }
    }
    None
}

fn persist_room_object_gold(object: &Object, gold: i64) {
    let index = object.getString("인덱스");
    if index.is_empty() {
        return;
    }
    let path = Path::new("data/item").join(format!("{index}.json"));
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(info) = root
        .get_mut("아이템정보")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    info.insert("은전".into(), serde_json::Value::Number(gold.into()));
    if let Ok(serialized) = serde_json::to_string_pretty(&root) {
        let _ = std::fs::write(path, format!("{serialized}\n"));
    }
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

/// Internal tag for a Rhai-authored payload that must bypass the normal
/// Python `sendLine` CRLF wrapper. The tag is removed before network output.
pub(crate) const RAW_USER_MESSAGE_PREFIX: &str = "\0MUC_RAW_USER\0";

/// Build data only; Rhai owns the visible `Player.getDesc()` layout.
#[cfg(test)]
pub(crate) fn build_room_view_player_snapshot(body: &Body) -> Dynamic {
    build_room_view_player_snapshot_with_interactive(body, 1)
}

pub(crate) fn build_room_view_player_snapshot_with_interactive(
    body: &Body,
    interactive: i32,
) -> Dynamic {
    let mut player = rhai::Map::new();
    player.insert("name".into(), Dynamic::from(body.get_string("이름")));
    for key in [
        "이름",
        "직위",
        "성격",
        "기존성격",
        "입문신청자",
        "반응이름",
        "설정상태",
        "이벤트설정리스트",
    ] {
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
    player.insert("interactive".into(), Dynamic::from(interactive as i64));
    player.insert(
        "show_prompt".into(),
        Dynamic::from(
            interactive == 1 && !config_is_enabled(&body.get_string("설정상태"), "엘피출력"),
        ),
    );
    player.insert("hp".into(), Dynamic::from(body.get_hp()));
    player.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
    player.insert("mp".into(), Dynamic::from(body.get_mp()));
    player.insert("max_mp".into(), Dynamic::from(body.get_max_mp()));
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

thread_local! {
    static PRE_COMPUTED_ROOM_ADMIN_BODIES: RefCell<Option<RoomAdminSnapshot>> =
        const { RefCell::new(None) };
}

struct RoomAdminSnapshot {
    bodies: Vec<(String, Body)>,
    values: Option<Vec<serde_json::Value>>,
}

pub(crate) fn set_precomputed_room_admin_bodies(players: Vec<(String, Body)>) {
    PRE_COMPUTED_ROOM_ADMIN_BODIES.with(|slot| {
        *slot.borrow_mut() = Some(RoomAdminSnapshot {
            bodies: players,
            values: None,
        });
    });
}

pub(crate) fn clear_precomputed_room_admin_bodies() {
    PRE_COMPUTED_ROOM_ADMIN_BODIES.with(|slot| *slot.borrow_mut() = None);
}

fn build_room_admin_player_value(name: &str, body: &Body) -> serde_json::Value {
    let guards: Vec<serde_json::Value> = body
        .object
        .objs
        .iter()
        .filter_map(|arc| {
            let obj = arc.lock().ok()?;
            (obj.getString("종류") == "호위").then(|| {
                let max_hp = object_from_item_json(&obj.getString("인덱스"))
                    .and_then(|(template, _)| template.lock().ok().map(|item| item.getInt("체력")))
                    .unwrap_or_else(|| obj.getInt("최고체력").max(obj.getInt("체력")));
                serde_json::json!({
                    "name": obj.getName(), "hp": obj.getInt("체력"),
                    "max_hp": max_hp, "description": obj.getString("설명2")
                })
            })
        })
        .collect();
    serde_json::json!({
        "name": name,
        "level": body.get_int("레벨"),
        "age": body.get_int("나이"),
        "hp": body.get_hp(),
        "max_hp": body.get_max_hp(),
        "mp": body.get_mp(),
        "max_mp": body.get_max_mp(),
        "attack": body.get_attack_power(),
        "strength": body.get_str(),
        "armor": body.get_armor(),
        "arm": body.get_arm(),
        "dex": body.get_dex(),
        "weight": body.get_item_weight(),
        "current_exp": body.get_int("현재경험치"),
        "total_exp": body.get_total_exp(),
        "hit": body.get_hit(), "miss": body.get_miss(),
        "critical": body.get_critical(), "luck": body.get_critical_chance(),
        "silver": body.get_int("은전"),
        "성격": body.get_string("성격"), "성별": body.get_string("성별"),
        "소속": body.get_string("소속"), "직위": body.get_string("직위"),
        "배우자": body.get_string("배우자"),
        "feature": body.get_int("특성치"),
        "insurance_premium": body.get_int("보험료"),
        "hp_script": hp_status_script(body.get_hp(), body.get_int("최고체력")),
        "mp_script": mp_status_script(body.get_mp()),
        "nickname": body.get_string("방파별호"),
        "anger": body.get_int("분노"),
        "targets": body.targets.iter().filter_map(|target| {
            target.upgrade().and_then(|target| target.lock().ok().map(|object| object.getName()))
        }).collect::<Vec<_>>(),
        "guards": guards,
        "raw_attrs": body.object.attr.iter().map(|(key, value)| {
            let value = match value {
                Value::Int(value) => serde_json::Value::Number((*value).into()),
                Value::Float(value) => serde_json::Number::from_f64(*value)
                    .map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null),
                Value::String(value) => serde_json::Value::String(value.clone()),
            };
            (key.clone(), value)
        }).collect::<serde_json::Map<String, serde_json::Value>>()
    })
}

fn room_admin_player_values(body: &Body) -> Option<Vec<serde_json::Value>> {
    if let Some(values) = body
        .temp()
        .get("_online_room_admin")
        .and_then(Value::as_str)
        .and_then(|json| serde_json::from_str::<Vec<serde_json::Value>>(json).ok())
    {
        return Some(values);
    }
    PRE_COMPUTED_ROOM_ADMIN_BODIES.with(|slot| {
        let mut slot = slot.borrow_mut();
        let snapshot = slot.as_mut()?;
        if snapshot.values.is_none() {
            snapshot.values = Some(
                snapshot
                    .bodies
                    .iter()
                    .map(|(name, player)| build_room_admin_player_value(name, player))
                    .collect(),
            );
        }
        snapshot.values.clone()
    })
}

pub(super) fn room_view_player_snapshots(zone: &str, room: &str) -> rhai::Array {
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

fn select_python_room_object(body: &Body, raw: &str) -> Option<RoomObjectRef> {
    let mut query = raw.split_whitespace().next()?;
    if query.trim() == "." {
        query = "1";
    }
    let world = get_world_state().read().ok()?;
    let position = world.get_player_position(&body.get_name())?;
    let mobs = world
        .mob_cache
        .get_all_mobs_in_room(&position.zone, &position.room);
    if query.chars().all(|ch| ch.is_ascii_digit()) {
        let order = query.parse::<usize>().ok()?;
        if order == 0 {
            return None;
        }
        return mobs
            .iter()
            .filter(|mob| {
                !matches!(mob.act, 2 | 3)
                    && world
                        .get_mob_data(&mob.mob_key)
                        .is_some_and(|data| data.mob_type != 7)
            })
            .nth(order - 1)
            .map(|mob| RoomObjectRef::Mob(mob.instance_id));
    }
    let digits = query.chars().take_while(|ch| ch.is_ascii_digit()).count();
    let order = if digits == 0 {
        1
    } else {
        query[..digits].parse::<usize>().ok()?
    };
    query = &query[digits..];
    if order == 0 || query.is_empty() {
        return None;
    }
    let floor = world.get_room_objs(&position.zone, &position.room);
    let installed =
        box_commands::installed_boxes_for_room(&position.zone, &position.room).unwrap_or_default();
    let players = room_view_player_snapshots(&position.zone, &position.room)
        .into_iter()
        .filter_map(|entry| entry.try_cast::<rhai::Map>())
        .collect::<Vec<_>>();
    let classify = |name: &str, aliases: &[String]| {
        let exact = name == query || aliases.iter().any(|alias| alias == query);
        let prefixes = if exact {
            0
        } else {
            aliases
                .iter()
                .filter(|alias| alias.starts_with(query))
                .count()
        };
        (exact, prefixes)
    };
    let mut exact_count = 0usize;
    let mut prefix_count = 0usize;
    world
        .get_room_object_order(&position.zone, &position.room)
        .into_iter()
        .find_map(|entry| {
            let counts = match &entry {
                RoomObjectRef::Player(name) => {
                    if name == &body.get_name() {
                        if body.get_int("투명상태") == 1 {
                            return None;
                        }
                        classify(name, &reaction_names(&body.get_string("반응이름")))
                    } else {
                        let snapshot = players.iter().find(|player| {
                            player
                                .get("이름")
                                .and_then(|value| value.clone().into_string().ok())
                                .as_deref()
                                == Some(name)
                        });
                        let online = get_precomputed_all_online()
                            .into_iter()
                            .filter_map(|entry| entry.try_cast::<rhai::Map>())
                            .find(|player| {
                                player
                                    .get("이름")
                                    .and_then(|value| value.clone().into_string().ok())
                                    .as_deref()
                                    == Some(name)
                            });
                        let source = snapshot.or(online.as_ref());
                        let transparent = source
                            .and_then(|player| player.get("transparent"))
                            .and_then(|value| value.as_bool().ok())
                            .or_else(|| {
                                source
                                    .and_then(|player| player.get("투명상태"))
                                    .and_then(|value| value.as_int().ok())
                                    .map(|value| value == 1)
                            })
                            .unwrap_or(false);
                        if transparent {
                            return None;
                        }
                        let aliases = source
                            .and_then(|player| player.get("반응이름"))
                            .and_then(|value| value.clone().into_string().ok())
                            .unwrap_or_default();
                        classify(name, &reaction_names(&aliases))
                    }
                }
                RoomObjectRef::SummonedUser(id) => {
                    let user = world.summoned_users().iter().find(|user| user.id == *id)?;
                    if user.body.get_int("투명상태") == 1 {
                        return None;
                    }
                    classify(
                        &user.body.get_name(),
                        &reaction_names(&user.body.get_string("반응이름")),
                    )
                }
                RoomObjectRef::Mob(id) => {
                    let mob = mobs.iter().find(|mob| mob.instance_id == *id)?;
                    let data = world.get_mob_data(&mob.mob_key)?;
                    if query != "시체" && matches!(mob.act, 2 | 3) {
                        return None;
                    }
                    let corpse = query == "시체" && mob.act == 2;
                    if corpse {
                        (true, 0)
                    } else {
                        classify(&data.name, &data.reaction_names)
                    }
                }
                RoomObjectRef::FloorItem(pointer) => {
                    let object = floor
                        .iter()
                        .find(|object| Arc::as_ptr(object) as usize == *pointer)?;
                    let object = object.lock().ok()?;
                    if object.getInt("투명상태") == 1 {
                        return None;
                    }
                    classify(
                        &object.getName(),
                        &reaction_names(&object.getString("반응이름")),
                    )
                }
                RoomObjectRef::Box(pointer) => {
                    let object = floor
                        .iter()
                        .chain(installed.iter())
                        .find(|object| Arc::as_ptr(object) as usize == *pointer)?;
                    let object = object.lock().ok()?;
                    if object.getInt("투명상태") == 1 {
                        return None;
                    }
                    classify(
                        &object.getName(),
                        &reaction_names(&object.getString("반응이름")),
                    )
                }
                RoomObjectRef::InstalledBox(ordinal) => {
                    let object = installed.get(*ordinal)?.lock().ok()?;
                    if object.getInt("투명상태") == 1 {
                        return None;
                    }
                    classify(
                        &object.getName(),
                        &reaction_names(&object.getString("반응이름")),
                    )
                }
                RoomObjectRef::Fixture(id) => world.get_fixture(*id)?.match_counts(query),
            };
            let (exact, prefixes) = counts;
            if exact {
                exact_count += 1;
                (exact_count == order).then_some(entry)
            } else {
                let previous = prefix_count;
                prefix_count += prefixes;
                (previous < order && order <= prefix_count).then_some(entry)
            }
        })
}

/// Python Item.han_obj(): 아이템 고유 ANSI(없으면 기본 청록색)와
/// 실제 이름을 표시하고, 비한글 이름은 첫 반응이름으로 조사를 정한다.
fn item_han_obj(item: &Object) -> String {
    let name = item.getName();
    let ansi = item.getString("안시");
    let name_a = if ansi.is_empty() {
        format!("\x1b[0;36m{name}\x1b[37m")
    } else {
        format!("{ansi}{name}\x1b[0;37m")
    };
    let particle_source = if crate::hangul::is_han(&name) {
        name.clone()
    } else {
        reaction_names(&item.getString("반응이름"))
            .into_iter()
            .next()
            .unwrap_or(name)
    };
    format!("{name_a}{}", han_eul(&particle_source))
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
    let now = chrono::Utc::now().timestamp();
    let active_skills = instance
        .skill_effects
        .iter()
        .filter_map(|effect| {
            let skill = crate::world::get_skill(&effect.name)?;
            Some(ActiveMugongSnapshot {
                name: effect.name.clone(),
                time: effect.expires_at.saturating_sub(now).max(0),
                level: 1,
                defense_time: skill.defense_time,
                defense_time_increase: skill.defense_time_increase,
            })
        })
        .collect();
    let mut learned_levels = instance
        .learned_skills
        .iter()
        .cloned()
        .map(|name| (name, 1_i64))
        .collect::<HashMap<_, _>>();
    for (name, training) in &instance.skill_map {
        learned_levels.insert(name.clone(), i64::from(training.level));
    }
    RoomMugongTargetSnapshot {
        kind: RoomMugongTargetKind::Mob,
        name: instance.name.clone(),
        reaction_names: data.reaction_names.clone(),
        transparent: false,
        act: instance.act,
        mob_type: instance.mob_type,
        multiplicity: 1,
        // Python Mob.skillList는 전투 무공 튜플 목록이고 skillMap은 비어 있다.
        skill_list_nonempty: mob_has_combat_skill(data) || !instance.learned_skills.is_empty(),
        // Configured mob entries are tuples in Python and do not match a
        // plain skill-name membership test. Administrator-taught strings do.
        skill_levels: learned_levels,
        secret_training: String::new(),
        secret_names: Vec::new(),
        active_skills,
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
        let mut remaining = name.parse::<i64>().unwrap_or(0);
        if remaining > 0 {
            for target in targets.iter().filter(|target| {
                target.kind == RoomMugongTargetKind::Mob
                    && target.mob_type != 7
                    && !matches!(target.act, 2 | 3)
            }) {
                if remaining <= target.multiplicity {
                    return Some(target.clone());
                }
                remaining -= target.multiplicity;
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

    // Python Room.findObjName keeps exact/corpse matches (`c`) and reaction
    // prefix matches (`d`) in separate counters.  A prior exact match must not
    // consume the ordinal requested from the prefix counter (and vice versa).
    let mut exact_count = 0_i64;
    let mut prefix_count = 0_i64;
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
            exact_count = exact_count.saturating_add(target.multiplicity);
            if exact_count >= order && exact_count.saturating_sub(target.multiplicity) < order {
                return Some(target.clone());
            }
        } else {
            // Python은 반응이름 각각을 접두사 후보로 세므로 그대로 센다.
            let alias_matches = target
                .reaction_names
                .iter()
                .filter(|alias| alias.starts_with(&name))
                .count() as i64
                * target.multiplicity;
            let previous = prefix_count;
            prefix_count = prefix_count.saturating_add(alias_matches);
            if previous < order && order <= prefix_count {
                return Some(target.clone());
            }
        }
    }
    None
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

/// 이벤트설정리스트 파싱: "키=값" 또는 "키" 항목. Python JSON 배열은
/// 로드 시 내부적으로 `|`로 이어지며, 이전 Rust 저장은 줄바꿈을 사용했다.
/// 두 표현 모두 `Player.setEvent/checkEvent/delEvent`의 같은 목록이다.
/// world::event::do_event에서도 사용. pub(crate).
pub(crate) fn parse_event_string(s: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for line in s.split(['\n', '|']) {
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

fn event_entries(raw: &str) -> Vec<String> {
    raw.split(['\n', '|'])
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Python `Object.setAttr('이벤트설정리스트', entry)` preserves insertion
/// order and only suppresses an identical full entry.  Keep that list shape
/// rather than round-tripping through a HashMap (which sorted event flags).
pub(crate) fn event_list_set(raw: &str, key: &str, value: &str) -> String {
    let mut entries = event_entries(raw);
    if value.is_empty() {
        if let Some(position) = entries.iter().position(|entry| entry == key) {
            entries.remove(position);
        }
    } else {
        let entry = if value == "1" {
            key.to_string()
        } else {
            format!("{key}={value}")
        };
        if !entries.iter().any(|current| current == &entry) {
            entries.push(entry);
        }
    }
    entries.join("\n")
}

/// Python `Object.delAttr()` removes one exact list element, not every
/// matching prefix or every duplicate.
pub(crate) fn event_list_remove(raw: &str, key: &str) -> String {
    event_list_set(raw, key, "")
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
    *raw = serde_json::Value::Array(exits.into_iter().map(serde_json::Value::String).collect());
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
        // Python 값설정/값값은 새 키에 대해 `int(raw)`만 시도한다.
        // 따라서 `1.5`는 Float가 아니라 원문 문자열이며, int 변환에
        // 실패했을 때 후행 공백도 버리지 않는다.
        None => Ok(raw
            .parse::<i64>()
            .map(Value::Int)
            .unwrap_or_else(|_| Value::String(raw.to_string()))),
    }
}

fn request_online_player_admin_value(
    body: &mut Body,
    target: &str,
    key: &str,
    raw: &str,
) -> Option<String> {
    let players = room_admin_player_values(body)?;
    let player = players
        .iter()
        .find(|player| player.get("name").and_then(|value| value.as_str()) == Some(target))?;
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
        Err(()) => return Some("invalid".into()),
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
            serde_json::to_string(&(target.to_string(), key.to_string(), json_value))
                .unwrap_or_default(),
        ),
    );
    Some("ok".into())
}

fn online_player_raw_attr(
    body: &Body,
    target: &str,
    key: &str,
) -> Option<Option<serde_json::Value>> {
    let players = room_admin_player_values(body)?;
    let player = players
        .iter()
        .find(|player| player.get("name").and_then(|value| value.as_str()) == Some(target))?;
    Some(
        player
            .get("raw_attrs")
            .and_then(|value| value.as_object())
            .and_then(|attrs| attrs.get(key))
            .cloned(),
    )
}

fn queue_admin_player_json_value(
    body: &mut Body,
    target: &str,
    key: &str,
    value: serde_json::Value,
) {
    body.temp_mut().insert(
        ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
        Value::String(
            serde_json::to_string(&(target.to_string(), key.to_string(), value))
                .unwrap_or_default(),
        ),
    );
}

fn set_mob_admin_value(zone: &str, room: &str, id: u64, key: &str, raw: &str) -> String {
    let mut world = get_world_state().write().unwrap();
    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(zone, room) else {
        return "missing".into();
    };
    let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == id) else {
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
    if let Some(selected) = select_python_room_object(body, target) {
        match selected {
            RoomObjectRef::Player(name) => {
                if name == body.get_name() {
                    let value = match python_coerce_attribute(body.object.attr.get(key), raw) {
                        Ok(value) => value,
                        Err(()) => return "invalid".into(),
                    };
                    body.set(key, value);
                    return "ok".into();
                }
                return request_online_player_admin_value(body, &name, key, raw)
                    .unwrap_or_else(|| "missing".into());
            }
            RoomObjectRef::SummonedUser(id) => {
                let mut world = get_world_state().write().unwrap();
                let Some(user) = world.summoned_user_mut(id) else {
                    return "missing".into();
                };
                let value = match python_coerce_attribute(user.body.object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                user.body.set(key, value);
                return "ok".into();
            }
            RoomObjectRef::Mob(id) => return set_mob_admin_value(&zone, &room, id, key, raw),
            RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                let floor = get_world_state()
                    .read()
                    .ok()
                    .map(|world| world.get_room_objs(&zone, &room))
                    .unwrap_or_default();
                let installed =
                    box_commands::installed_boxes_for_room(&zone, &room).unwrap_or_default();
                let object = floor
                    .into_iter()
                    .chain(installed)
                    .find(|object| Arc::as_ptr(object) as usize == pointer);
                let Some(object) = object else {
                    return "missing".into();
                };
                let Ok(mut object) = object.lock() else {
                    return "missing".into();
                };
                let value = match python_coerce_attribute(object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                object.set(key, value);
                return "ok".into();
            }
            RoomObjectRef::InstalledBox(ordinal) => {
                let object = box_commands::installed_boxes_for_room(&zone, &room)
                    .and_then(|boxes| boxes.get(ordinal).cloned());
                let Some(object) = object else {
                    return "missing".into();
                };
                let Ok(mut object) = object.lock() else {
                    return "missing".into();
                };
                let value = match python_coerce_attribute(object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                object.set(key, value);
                return "ok".into();
            }
            RoomObjectRef::Fixture(id) => {
                let mut world = get_world_state().write().unwrap();
                let Some(fixture) = world.get_fixture_mut(id) else {
                    return "missing".into();
                };
                fixture.set_attribute(key, serde_json::Value::String(raw.to_string()));
                return "ok".into();
            }
        }
    }
    if target == body.get_name()
        && (current_body_position(body).is_none()
            || select_python_room_object(body, target)
                == Some(RoomObjectRef::Player(body.get_name())))
    {
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
        let Ok(mut object) = object.lock() else {
            continue;
        };
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

    // Room.findObjName()에는 같은 방의 Player도 포함된다. 실제 Body는
    // 네트워크 클라이언트가 소유하므로 변경 요청을 경계 밖에서 적용한다.
    if let Some(status) = request_online_player_admin_value(body, target, key, raw) {
        return status;
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
    let inventory_object = body.object.objs.iter().find_map(|object| {
        let item = object.lock().ok()?;
        (item.getName() == target
            || item
                .getString("반응이름")
                .split("\r\n")
                .any(|alias| alias == target))
        .then(|| object.clone())
    });
    if let Some(object_arc) = inventory_object {
        let Ok(mut object) = object_arc.lock() else {
            return "missing".into();
        };
        let value = match python_coerce_attribute(object.attr.get(key), raw) {
            Ok(value) => value,
            Err(()) => return "invalid".into(),
        };
        object.set(key, value);
        drop(object);
        restore_pristine_inventory_object(body, &object_arc);
        return "ok".into();
    }
    if let Some(stack_key) = inventory_compat::find_counted_item_key(&body.object.inv_stack, target)
    {
        if body.object.inv_stack.get(&stack_key).copied().unwrap_or(0) > 0 {
            let Some(object_arc) =
                inventory_compat::materialize_one(&mut body.object, &stack_key, true)
            else {
                return "missing".into();
            };
            let Ok(mut object) = object_arc.lock() else {
                return "missing".into();
            };
            let value = match python_coerce_attribute(object.attr.get(key), raw) {
                Ok(value) => value,
                Err(()) => {
                    drop(object);
                    restore_pristine_inventory_object(body, &object_arc);
                    return "invalid".into();
                }
            };
            object.set(key, value);
            drop(object);
            restore_pristine_inventory_object(body, &object_arc);
            return "ok".into();
        }
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
    let mut matched = 0usize;
    let mut selected: Vec<Arc<Mutex<Object>>> = Vec::new();
    let mut last_name = String::new();
    let mut last_post = String::new();
    for item in &body.object.objs {
        let Ok(object) = item.lock() else { continue };
        let aliases = reaction_names(&object.getString("반응이름"));
        if object.getName() != wanted && !aliases.iter().any(|alias| alias == wanted) {
            continue;
        }
        if object.getBool("inUse") || (break_mode && object.checkAttr("아이템속성", "출력안함"))
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
        last_post = item_han_obj(&object);
        selected.push(item.clone());
        if selected.len() >= count {
            break;
        }
    }

    // Runtime counts follow the individual objects. A numbered occurrence
    // beyond the objects therefore starts inside the counted groups; a bulk
    // command beginning in the object list may continue into those groups.
    let mut stack_removals = Vec::<(String, i64)>::new();
    let mut remaining_count = count.saturating_sub(selected.len()) as i64;
    let mut stack_order = if order > matched {
        order.saturating_sub(matched) as i64
    } else {
        1
    };
    if remaining_count > 0 {
        for key in inventory_compat::counted_item_keys(&body.object.inv_stack, wanted) {
            let have = body.object.inv_stack.get(&key).copied().unwrap_or(0).max(0);
            if have == 0 {
                continue;
            }
            let Some((template, _)) = object_from_item_json(&key) else {
                continue;
            };
            let Ok(template) = template.lock() else {
                continue;
            };
            if break_mode && template.checkAttr("아이템속성", "출력안함") {
                continue;
            }
            if stack_order > have {
                stack_order -= have;
                continue;
            }
            if break_mode && template.checkAttr("아이템속성", "부수지못함") {
                if selected.is_empty() && stack_removals.is_empty() {
                    return destroy_item_result("unbreakable", String::new(), 0);
                }
                stack_order = 1;
                continue;
            }
            let available = have.saturating_sub(stack_order - 1);
            let take = remaining_count.min(available);
            if take > 0 {
                last_name = template.getName();
                last_post = item_han_obj(&template);
                stack_removals.push((key, take));
                remaining_count -= take;
            }
            stack_order = 1;
            if remaining_count == 0 {
                break;
            }
        }
    }

    if selected.is_empty() && stack_removals.is_empty() {
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
    let mut removed = selected.len() as i64;
    for (key, count) in stack_removals {
        if inventory_compat::remove_pristine_count(&mut body.object, &key, count) {
            removed += count;
        }
    }
    let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
    let mut result = destroy_item_result("ok", last_name.clone(), removed);
    result.insert("post".into(), Dynamic::from(last_post));
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
    /// Changes only after this script or a shared library is recompiled.
    revision: u64,
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

const BCRYPT_SHA256_PREFIX: &str = "$murim$bcrypt-sha256$";
const BCRYPT_COST: u32 = 10;

fn password_sha256(plain: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(plain.as_bytes());
    format!("{:x}", hash.finalize())
}

/// 평문을 bcrypt로 해시한다. bcrypt 문자열에는 알고리즘, cost, salt가 포함된다.
pub fn password_hash(plain: &str) -> String {
    bcrypt::hash(plain, BCRYPT_COST).expect("bcrypt password hashing failed")
}

/// 저장된 값(해시 또는 레거시 평문)과 평문 입력이 일치하는지 검사.
/// 신규 bcrypt와 기존 SHA-512/평문 계정을 모두 읽어 점진적으로 이관한다.
pub fn password_verify(stored: &str, plain: &str) -> bool {
    if let Some(hash) = stored.strip_prefix(BCRYPT_SHA256_PREFIX) {
        return bcrypt::verify(password_sha256(plain), hash).unwrap_or(false);
    }
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
    if stored.starts_with(BCRYPT_SHA256_PREFIX) {
        return true;
    }
    if stored.starts_with("$2a$") || stored.starts_with("$2b$") || stored.starts_with("$2y$") {
        return stored
            .split('$')
            .nth(2)
            .and_then(|cost| cost.parse::<u32>().ok())
            != Some(BCRYPT_COST);
    }
    true
}

fn upgrade_verified_user_password_hash(
    name: &str,
    plain: &str,
    verified_stored: &str,
) -> std::io::Result<()> {
    let path = Path::new("data/user").join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let stored = json
        .get("사용자오브젝트")
        .and_then(|v| v.get("암호"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    // Do not overwrite a password that changed after the successful check.
    if stored != verified_stored || !password_needs_upgrade(stored) {
        return Ok(());
    }
    json["사용자오브젝트"]["암호"] = serde_json::Value::String(password_hash(plain));
    let serialized = serde_json::to_string_pretty(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serialized)?;
    std::fs::rename(tmp, path)
}

/// 기존 평문/SHA-512/SHA-256 전처리 bcrypt 계정이 정상 로그인하면
/// 파일의 암호만 일반 bcrypt 형식으로 교체한다.
pub fn upgrade_user_password_hash(name: &str, plain: &str) -> std::io::Result<()> {
    let path = Path::new("data/user").join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let stored = json
        .get("사용자오브젝트")
        .and_then(|v| v.get("암호"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if !password_verify(stored, plain) || !password_needs_upgrade(stored) {
        return Ok(());
    }
    upgrade_verified_user_password_hash(name, plain, stored)
}

/// Perform the expensive login check off the async runtime, upgrading the
/// stored work factor without repeating a successful bcrypt verification.
pub fn verify_and_upgrade_user_password(name: &str, plain: &str) -> bool {
    let Some(stored) = load_user_password_hash(name) else {
        return false;
    };
    if !password_verify(&stored, plain) {
        return false;
    }
    if password_needs_upgrade(&stored) {
        if let Err(error) = upgrade_verified_user_password_hash(name, plain, &stored) {
            log::error!("failed to upgrade password hash for {}: {}", name, error);
        }
    }
    true
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

fn python_json_ensure_ascii(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        let code = character as u32;
        if code <= 0x7f {
            output.push(character);
        } else if code <= 0xffff {
            output.push_str(&format!("\\u{code:04x}"));
        } else {
            let adjusted = code - 0x1_0000;
            let high = 0xd800 + (adjusted >> 10);
            let low = 0xdc00 + (adjusted & 0x3ff);
            output.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
        }
    }
    output
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

const BODY_JSON_ARRAY_MARKER_PREFIX: &str = "_body_json_array_attr:";

fn body_json_array_marker(key: &str) -> String {
    format!("{BODY_JSON_ARRAY_MARKER_PREFIX}{key}")
}

/// `Player.setAttr()`가 만든 Python 배열 속성임을 보존한다. 이벤트 플래그는
/// Python에서 항상 이 경로로 추가/삭제되므로, Rust가 다음 저장에서 문자열로
/// 내보내면 Python `checkAttr()`의 문자열 부분검색으로 의미가 달라진다.
pub(crate) fn mark_body_attr_as_json_array(body: &mut Body, key: &str) {
    body.object
        .temp
        .insert(body_json_array_marker(key), Value::Int(1));
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
        // Preserve the JSON list shape of arbitrary Python user attributes.
        // Object values are scalar internally, so load records this marker
        // separately. All currently observed user arrays contain strings.
        if body.object.temp.contains_key(&body_json_array_marker(k)) {
            let raw = match v {
                Value::String(value) => value.as_str(),
                _ => "",
            };
            let values = raw
                .split(if k == "이벤트설정리스트" {
                    ['|', '\n']
                } else {
                    ['|', '\0']
                })
                .filter(|entry| !entry.is_empty())
                .map(|entry| serde_json::Value::String(entry.to_string()))
                .collect();
            uso.insert(k.clone(), serde_json::Value::Array(values));
            continue;
        }
        // Python 호환성: 파이프 구분 문자열을 배열로 변환
        if k == "무공숙련도"
            || k == "무공이름"
            || k == "무공이름수련리스트"
            || k == "설정상태"
            || k == "비전이름"
            || k == "방어무공시전"
            || k == "입문신청자"
        {
            if let Value::String(s) = v {
                // Python buildSkillList/buildSkillUp always replace these
                // three fields with lists, including the empty case.  A
                // non-empty string for 무공이름수련리스트 makes Python's
                // loadSkillList iterate characters and fail to parse it.
                let always_array =
                    matches!(k.as_str(), "무공숙련도" | "무공이름" | "무공이름수련리스트");
                if always_array || !s.is_empty() || k == "입문신청자" {
                    // "skill1|skill2" 또는 "skill1 level exp|skill2 level exp" 형식을 배열로 변환
                    let parts: Vec<serde_json::Value> = s
                        .split(['|', '\r', '\n', ','])
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

    // Pristine template-equal items remain counted at runtime and are saved by
    // index and quantity. Stateful items stay as individual objects and save
    // only the attributes that differ from their template.
    if inventory_compat::materialize_stacks_for_save(body).is_err() {
        return false;
    }
    let items = inventory_compat::compact_inventory_records(body);

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

/// data/user/{이름}.json 에서 Body 복원. 원본과 같은 압축 아이템은 런타임에서도
/// 인덱스별 수량으로 유지하고, 상태가 다른 아이템만 개별 객체로 복원한다.
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
        body.object
            .temp
            .retain(|key, _| !key.starts_with(BODY_JSON_ARRAY_MARKER_PREFIX));
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
                if v.is_array() {
                    body.object
                        .temp
                        .insert(body_json_array_marker(k), Value::Int(1));
                }
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
    let path = PathBuf::from(format!("data/item/{key}.json"));
    object_from_item_path(&path, key)
}

#[derive(Clone)]
struct CachedItemObjectTemplate {
    modified: std::time::SystemTime,
    file_len: u64,
    object: Object,
    display_name: String,
}

static ITEM_OBJECT_TEMPLATE_CACHE: std::sync::LazyLock<
    std::sync::RwLock<HashMap<PathBuf, CachedItemObjectTemplate>>,
> = std::sync::LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

fn object_from_item_path(path: &Path, key: &str) -> Option<(Arc<Mutex<Object>>, String)> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let file_len = metadata.len();
    if let Ok(cache) = ITEM_OBJECT_TEMPLATE_CACHE.read() {
        if let Some(cached) = cache.get(path) {
            if cached.modified == modified && cached.file_len == file_len {
                return Some((
                    Arc::new(Mutex::new(cached.object.deepclone())),
                    cached.display_name.clone(),
                ));
            }
        }
    }

    let (object, display_name) = load_item_object_from_path(path, key)?;
    if let Ok(mut cache) = ITEM_OBJECT_TEMPLATE_CACHE.write() {
        cache.insert(
            path.to_path_buf(),
            CachedItemObjectTemplate {
                modified,
                file_len,
                object: object.deepclone(),
                display_name: display_name.clone(),
            },
        );
    }
    Some((Arc::new(Mutex::new(object)), display_name))
}

fn load_item_object_from_path(path: &Path, key: &str) -> Option<(Object, String)> {
    let content = std::fs::read_to_string(path).ok()?;
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
                    // Python 먹어.py joins list-valued 사용스크립 entries
                    // with CRLF at command time. Keep line boundaries here;
                    // the Rhai presentation layer performs the CRLF mapping.
                    let separator = if matches!(k.as_str(), "사용스크립" | "설명2") {
                        "\n"
                    } else {
                        " "
                    };
                    let s = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(separator);
                    obj.set(k, s);
                }
            }
            serde_json::Value::Object(_) => {}
        }
    }
    Some((obj, display_name))
}

/// Upgrade Rust's legacy compact floor representation into the individual
/// Item objects used by Python Room.objs. Historical compression has already
/// lost cross-key interleaving, so keys are materialized deterministically;
/// all new runtime drops preserve their exact prepend order directly.
pub(crate) fn materialize_legacy_room_stacks_for_player(player_name: &str) {
    let Some(position) = get_world_state()
        .read()
        .ok()
        .and_then(|world| world.get_player_position(player_name).cloned())
    else {
        return;
    };
    let stacks = get_world_state()
        .read()
        .ok()
        .map(|world| world.get_room_objs_stack(&position.zone, &position.room))
        .unwrap_or_default();
    let mut entries = stacks
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    if entries.is_empty() {
        return;
    }

    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return,
    };
    for (key, count) in entries {
        let mut restored = Vec::new();
        for _ in 0..count {
            let Some((item, _)) = object_from_item_json(&key) else {
                break;
            };
            world
                .get_room_objs_mut(&position.zone, &position.room)
                .insert(0, item.clone());
            restored.push(item);
        }
        if restored.is_empty() {
            continue;
        }
        for item in &restored {
            world.record_floor_item(&position.zone, &position.room, item);
        }
        let stack = world.get_room_objs_stack_mut(&position.zone, &position.room);
        let remaining = count - restored.len() as i64;
        if remaining > 0 {
            stack.insert(key, remaining);
        } else {
            stack.remove(&key);
        }
    }
}

fn materialize_legacy_room_stacks(body: &Body) {
    materialize_legacy_room_stacks_for_player(&body.get_name());
}

/// item JSON에서 이름, 반응이름, 판매가격(또는 값), 무게 반환. 구입 가격 계산용.
fn get_item_info(key: &str) -> Option<(String, String, i64, i64)> {
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

/// 아이템 인덱스가 동일 원본 수량으로 누적 가능한지.
pub(crate) fn is_stackable(key: &str) -> bool {
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
    // 호위는 생성 직후부터 각자의 체력과 전투 상태를 가지므로 같은
    // 템플릿이라도 독립 인스턴스다.
    if info.get("종류").and_then(|value| value.as_str()) == Some("호위") {
        return false;
    }
    let attrs = info.get("아이템속성");
    if let Some(serde_json::Value::Array(arr)) = attrs {
        for v in arr {
            if matches!(v.as_str(), Some("개별인스턴스" | "단일아이템")) {
                return false;
            }
        }
    } else if let Some(serde_json::Value::String(s)) = attrs {
        if s.contains("개별인스턴스") || s.contains("단일아이템") {
            return false;
        }
    }
    true
}

/// Return an inventory object for an operation that mutates one item. Pristine
/// counted items are split only when no existing individual object matches.
fn inventory_object_for_mutation(
    body: &mut Body,
    name: &str,
    order: usize,
) -> Option<Arc<Mutex<Object>>> {
    if let Some(item) = body.object.findObjInven(name, order.max(1)) {
        return Some(item);
    }
    let individual = body
        .object
        .objs
        .iter()
        .filter(|item| {
            item.lock().is_ok_and(|item| {
                !item.getBool("inUse")
                    && (item.getName() == name || item.getString("반응이름").contains(name))
            })
        })
        .count();
    let remaining = order.max(1).saturating_sub(individual) as i64;
    let (key, _) = inventory_compat::counted_item_at(&body.object.inv_stack, name, remaining)?;
    inventory_compat::materialize_one(&mut body.object, &key, true)
}

fn restore_pristine_inventory_object(body: &mut Body, item: &Arc<Mutex<Object>>) {
    if inventory_compat::absorb_pristine_object(&mut body.object, item) {
        body.object.remove(item);
    }
}

fn inventory_exact_name_count(body: &Body, name: &str) -> i64 {
    let individual = body
        .object
        .objs
        .iter()
        .filter(|item| item.lock().is_ok_and(|item| item.getName() == name))
        .count() as i64;
    let counted = body
        .object
        .inv_stack
        .iter()
        .filter_map(|(key, count)| {
            object_from_item_json(key).and_then(|(item, _)| {
                item.lock()
                    .ok()
                    .and_then(|item| (item.getName() == name).then_some(*count))
            })
        })
        .sum::<i64>();
    individual.saturating_add(counted)
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
        // Item.Items is keyed by the filename stem.  Mob preloading changes
        // insertion order, but it does not rename the template index.
        item.insert("index".into(), Dynamic::from(index.to_string()));
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
        let user_enabled = match info.get("사용자") {
            None => false,
            Some(serde_json::Value::String(value)) => !value.is_empty(),
            Some(serde_json::Value::Number(value)) => value.as_f64() != Some(0.0),
            Some(serde_json::Value::Bool(value)) => *value,
            // Python: None/list/dict are unequal to both integer 0 and "".
            Some(
                serde_json::Value::Null
                | serde_json::Value::Array(_)
                | serde_json::Value::Object(_),
            ) => true,
        };
        item.insert("user_enabled".into(), Dynamic::from(user_enabled));
        result.push(Dynamic::from(item));
    }
    result
}

fn resolve_skill_definition_name(query: &str) -> String {
    let Ok(source) = std::fs::read_to_string("data/config/skill.json") else {
        return String::new();
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&source) else {
        return String::new();
    };
    let Some(skills) = root.as_object() else {
        return String::new();
    };
    if skills.contains_key(query) {
        return query.to_string();
    }
    skills
        .keys()
        .find(|name| name.starts_with(query))
        .cloned()
        .unwrap_or_default()
}

fn skill_definition_names() -> rhai::Array {
    std::fs::read_to_string("data/config/skill.json")
        .ok()
        .and_then(|source| serde_json::from_str::<serde_json::Value>(&source).ok())
        .and_then(|root| root.as_object().cloned())
        .map(|skills| {
            skills
                .into_iter()
                .map(|(name, _)| Dynamic::from(name))
                .collect()
        })
        .unwrap_or_default()
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
    engine.register_fn("han_un", |name: &str| -> String { han_eun(name) });
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
        "python_value_is_empty",
        |player_data: &mut rhai::Map, key: &str| -> bool {
            player_data
                .get(key)
                .is_none_or(|value| value.is_string() && value.to_string().is_empty())
        },
    );
    engine.register_fn("python_chars", |value: &str| -> rhai::Array {
        value
            .chars()
            .map(|character| Dynamic::from(character.to_string()))
            .collect()
    });
    engine.register_fn("python_split_once", |value: &str| -> rhai::Array {
        let input = value.trim_start();
        let Some(boundary) = input.find(char::is_whitespace) else {
            return if input.is_empty() {
                Vec::new()
            } else {
                vec![Dynamic::from(input.to_string())]
            };
        };
        let first = &input[..boundary];
        let rest = input[boundary..].trim_start();
        if rest.is_empty() {
            vec![Dynamic::from(first.to_string())]
        } else {
            vec![
                Dynamic::from(first.to_string()),
                Dynamic::from(rest.to_string()),
            ]
        }
    });
    engine.register_fn("map_has_key", |map: rhai::Map, key: &str| -> bool {
        map.contains_key(key)
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
    engine.register_fn("euc_kr_len", euc_kr_len);
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

    engine.register_fn("strip_ansi", strip_ansi_like_python);

    engine.register_fn("to_int", |s: &str| -> i64 { s.trim().parse().unwrap_or(0) });
    engine.register_fn("parse_int_prefix", parse_int_prefix);
    engine.register_fn("strict_int", strict_int_result);

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
    engine.register_fn("resolve_skill_definition", resolve_skill_definition_name);
    engine.register_fn("skill_definition_names", skill_definition_names);

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
        let fixture_lines = visible_fixture_short_lines(&world, &pos.zone, &pos.room);
        // Python viewMapData traverses Room.objs including ACT_DEATH corpses;
        // get_mobs_in_room intentionally filters to living targets only.
        let mut mobs = world.mob_cache.get_all_mobs_in_room(&pos.zone, &pos.room);
        let unified_order = world.get_room_object_order(&pos.zone, &pos.room);
        if !unified_order.is_empty() {
            mobs.sort_by_key(|mob| {
                unified_order
                    .iter()
                    .position(|object| *object == RoomObjectRef::Mob(mob.instance_id))
                    .unwrap_or(usize::MAX)
            });
        }
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
        if !fixture_lines.is_empty() {
            out.push_str(&fixture_lines.join("\r\n"));
            out.push_str("\r\n");
        }
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
    let s = fill_space_euc_kr(41, &format!("◆ 이  름 ▷ 『{}』 {}", m, body.get_name()));
    let c2 = fill_space_euc_kr(19, &format!("◆ 성격 ▷ 『{}』", c));
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}{}\x1b[0m\x1b[37m\x1b[40m",
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
    let spouse = fill_space_euc_kr(41, &format!("◆ 배우자 ▷ 『{}』", ba));
    let age_sex = fill_space_euc_kr(19, &format!("◆ 나이 ▷ {}살({})", age, sex));
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}{}\x1b[0m\x1b[37m\x1b[40m",
        spouse, age_sex
    ));
    let so = body.get_string("소속");
    if !so.is_empty() {
        let guild_line = format!("■ 소  속 ▷ 『{}』", so);
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{:<60}\x1b[0m\x1b[37m\x1b[40m",
            guild_line
        ));
        let jw = body.get_string("직위");
        let r = body.get_string("방파별호");
        let configured_title = guild_get(&so, &format!("{}명칭", jw));
        let title = if configured_title.is_empty() {
            jw
        } else {
            configured_title
        };
        let jw_line = if r.is_empty() {
            format!("■ 직  위 ▷ 『{}』", title)
        } else {
            format!("■ 직  위 ▷ 『{}({})』", title, r)
        };
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{:<60}\x1b[0m\x1b[37m\x1b[40m",
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

fn mob_hp_status_text(mob: &MobInstance, data: &RawMobData) -> String {
    let Ok(source) = std::fs::read_to_string("data/config/script.json") else {
        return String::new();
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&source) else {
        return String::new();
    };
    let key = format!("{}스크립", data.hp_display_type);
    let Some(value) = root.get("메인설정").and_then(|main| main.get(&key)) else {
        return String::new();
    };
    let scripts = match value {
        serde_json::Value::String(value) => vec![value.as_str()],
        serde_json::Value::Array(values) => {
            values.iter().filter_map(|value| value.as_str()).collect()
        }
        _ => Vec::new(),
    };
    if scripts.is_empty() || mob.max_hp <= 0 {
        return String::new();
    }
    let last = scripts.len() - 1;
    let index = (last as i64 - (last as i64 * mob.hp) / mob.max_hp).clamp(0, last as i64) as usize;
    crate::hangul::post_position_all(&format!("{}{}", mob.name, scripts[index]))
}

/// 파이썬 objs/mob.view(ob). 살아있는 몹: 이름·설명2·사용아이템·HP·HPbar. 시체: 이름의 시체.
fn mob_view(mob: &MobInstance, data: &RawMobData) -> Vec<String> {
    let mut lines = vec!["━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string()];
    if !mob.alive {
        lines.push(format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {:<49}\x1b[0m\x1b[37m\x1b[40m",
            format!("{}의 시체", mob.name)
        ));
        lines.push("──────────────────────────────".to_string());
        if mob.inventory.is_empty() {
            lines.push("\x1b[36m☞ 아무것도 없습니다.\x1b[37m".to_string());
        } else {
            lines.push(
                mob.inventory
                    .iter()
                    .filter_map(|item| item.lock().ok().map(|item| item.getName()))
                    .map(|name| format!("\x1b[36m{name}\x1b[37m"))
                    .collect::<Vec<_>>()
                    .join("\r\n"),
            );
        }
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        return lines;
    }
    lines.push(format!(
        "\x1b[0m\x1b[44m\x1b[1m\x1b[37m◆ 이름 ▷ {:<47}\x1b[0m\x1b[37m\x1b[40m",
        mob.name
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
                }
            }
        }
    }
    for (disp, iname) in &use_lines {
        lines.push(format!("[{}] \x1b[36m{}\x1b[37m", disp, iname));
    }
    if !data.use_items.is_empty() {
        lines.push("──────────────────────────────".to_string());
    }
    let max_hp = if mob.max_hp <= 0 { 1 } else { mob.max_hp };
    let pct = (mob.hp * 100) / max_hp;
    lines.push(format!("★ {}", mob_hp_status_text(mob, data)));
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
        lines.push(
            entries
                .chunks(3)
                .map(|row| row.concat())
                .collect::<Vec<_>>()
                .join("\r\n"),
        );
    }
    let skill_root = std::fs::read_to_string("data/config/skill.json")
        .ok()
        .and_then(|source| serde_json::from_str::<serde_json::Value>(&source).ok());
    let skill_kind = |name: &str| {
        skill_root
            .as_ref()
            .and_then(|root| root.get(name))
            .and_then(|skill| skill.get("종류"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
    };
    let attack_skills = data
        .skills
        .iter()
        .filter(|(name, _, _)| skill_kind(name) == "전투")
        .collect::<Vec<_>>();
    let defense_skills = data
        .skills
        .iter()
        .filter(|(name, _, _)| {
            skill_root
                .as_ref()
                .and_then(|root| root.get(name))
                .is_some()
                && skill_kind(name) != "전투"
        })
        .collect::<Vec<_>>();
    if !attack_skills.is_empty() {
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        lines.push("★ 공격 스킬 목록".to_string());
        let entries = attack_skills
            .iter()
            .enumerate()
            .map(|(index, (name, _, _))| format!(" [{:02}] {:<10} ", index + 1, name))
            .collect::<Vec<_>>();
        lines.push(
            entries
                .chunks(3)
                .map(|row| row.concat())
                .collect::<Vec<_>>()
                .join("\r\n"),
        );
    }
    if !defense_skills.is_empty() {
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        lines.push("★ 기타 스킬 목록".to_string());
        let entries = defense_skills
            .iter()
            .enumerate()
            .map(|(index, (name, _, _))| format!(" [{:02}] {:<10} ", index + 1, name))
            .collect::<Vec<_>>();
        lines.push(
            entries
                .chunks(3)
                .map(|row| row.concat())
                .collect::<Vec<_>>()
                .join("\r\n"),
        );
    }
    if !mob.skill_effects.is_empty() {
        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        lines.push("★ 무공집결상태".to_string());
        let now = chrono::Utc::now().timestamp();
        for effect in &mob.skill_effects {
            let state = skill_root
                .as_ref()
                .and_then(|root| root.get(&effect.name))
                .and_then(|skill| skill.get("방어상태출력"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let remaining = effect.expires_at - now;
            let bar_index = if effect.expires_at <= 0 {
                0
            } else {
                ((now * 10) / effect.expires_at).clamp(0, 10)
            };
            let timer = format!("{:>5}ː{}", remaining, get_hp_bar_string(bar_index, 10));
            lines.push(format!(
                "\x1b[1m\x1b[40m\x1b[36m·\x1b[0m\x1b[40m\x1b[37m{:<14}│{:<12}│ {}",
                effect.name, state, timer
            ));
        }
    }
    lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

/// 아이템 상세 보기. 파이썬 objs/item.view(ob). find_target/look_at_target에서 사용.
fn item_view(obj: &Arc<Mutex<Object>>) -> Vec<String> {
    let o = obj.lock().unwrap();
    let name_line = fill_space_euc_kr(42, &format!("◆ 이름 ▷ {}", o.getName()));
    let type_line = fill_space_euc_kr(42, &format!("◆ 종류 ▷ {}", o.getString("종류")));
    let mut lines = vec![
        "━━━━━━━━━━━━━━━━━━━━━".to_string(),
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m",
            name_line
        ),
        format!(
            "\x1b[0m\x1b[44m\x1b[1m\x1b[37m{}\x1b[0m\x1b[37m\x1b[40m",
            type_line
        ),
        "─────────────────────".to_string(),
    ];
    let desc = o.getString("설명2");
    if desc.is_empty() {
        lines.push(String::new());
    } else {
        for line in desc.lines() {
            if line.starts_with("방어력 - ") {
                lines.push(format!("방어력 - {}", o.getInt("방어력")));
            } else {
                lines.push(line.to_string());
            }
        }
    }
    let opt = o.get_option_str();
    if !opt.is_empty() {
        lines.push(opt);
    }
    lines.push("━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines
}

fn python_room_name_match(object_name: &str, reactions: &str, query: &str) -> u8 {
    if object_name == query || reactions.split_whitespace().any(|alias| alias == query) {
        1
    } else if reactions
        .split_whitespace()
        .any(|alias| alias.starts_with(query))
    {
        2
    } else {
        0
    }
}

fn python_room_match_selected(
    kind: u8,
    exact: &mut usize,
    prefix: &mut usize,
    order: usize,
) -> bool {
    match kind {
        1 => {
            *exact += 1;
            *exact == order
        }
        2 => {
            *prefix += 1;
            *prefix == order
        }
        _ => false,
    }
}

/// [대상] 봐: 나|findObjInven|find_in_room(아이템,몹,플레이어,출구) 검색 후 타입별 표시.
/// returns (viewer_lines, Option<(target_player_name, msg_to_target)>)
fn look_at_target(
    body: &Body,
    world: &WorldState,
    viewer_name: &str,
    target_line: &str,
    other_player_descs: &HashMap<String, String>,
    selected_fixture: &mut Option<Fixture>,
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
        let individual = body
            .object
            .objs
            .iter()
            .filter(|item| {
                item.lock().is_ok_and(|item| {
                    !item.getBool("inUse")
                        && (item.getName() == name || item.getString("반응이름").contains(&name))
                })
            })
            .count();
        let remaining = order.saturating_sub(individual) as i64;
        if let Some((key, _)) =
            inventory_compat::counted_item_at(&body.object.inv_stack, &name, remaining)
        {
            if let Some((item, _)) = object_from_item_json(&key) {
                return (item_view(&item), None);
            }
        }
    }

    let pos = match world.get_player_position(viewer_name) {
        Some(p) => p,
        None => return (vec!["위치 정보가 없습니다.".to_string()], None),
    };
    let zone = pos.zone.as_str();
    let room_s = pos.room.as_str();
    let mut c = 0usize;
    let player_snapshots = room_view_player_snapshots(zone, room_s)
        .into_iter()
        .filter_map(|value| value.try_cast::<rhai::Map>())
        .collect::<Vec<_>>();
    let player_match_kind = |player_name: &str| {
        let Some(player) = player_snapshots.iter().find(|player| {
            player
                .get("name")
                .and_then(|value| value.clone().into_string().ok())
                .as_deref()
                == Some(player_name)
        }) else {
            return if player_name == name { 1 } else { 0 };
        };
        if player
            .get("transparent")
            .and_then(|value| value.as_bool().ok())
            == Some(true)
        {
            return 0;
        }
        let reactions = player
            .get("반응이름")
            .and_then(|value| value.clone().into_string().ok())
            .unwrap_or_default();
        python_room_name_match(player_name, &reactions, &name)
    };
    let player_matches = |player_name: &str| player_match_kind(player_name) != 0;

    if name.is_empty() && order >= 1 {
        // Python Room.findObjName("1") walks the unified room.objs order and
        // counts only living, non-type-7 mobs.  `쳐` uses this same order;
        // `봐` must not fall back to MobCache's internal vector order.
        let mobs = world.mob_cache.get_all_mobs_in_room(zone, room_s);
        let mut ordered = world.get_room_object_order(zone, room_s);
        if ordered.is_empty() {
            ordered.extend(mobs.iter().map(|mob| RoomObjectRef::Mob(mob.instance_id)));
        }
        for object in ordered {
            let RoomObjectRef::Mob(instance_id) = object else {
                continue;
            };
            let Some(mob) = mobs.iter().find(|mob| mob.instance_id == instance_id) else {
                continue;
            };
            if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                if !mob.alive || mob.act == 2 || mob.act == 3 || data.mob_type == 7 {
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
        let mut ordered_exact = 0usize;
        let mut ordered_prefix = 0usize;
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
                    let match_kind = python_room_name_match(&item.getName(), &aliases, &name);
                    drop(item);
                    if python_room_match_selected(
                        match_kind,
                        &mut ordered_exact,
                        &mut ordered_prefix,
                        order,
                    ) {
                        return (item_view(arc), None);
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
                    let reactions = data.reaction_names.join(" ");
                    let match_kind = python_room_name_match(&data.name, &reactions, &name);
                    if python_room_match_selected(
                        match_kind,
                        &mut ordered_exact,
                        &mut ordered_prefix,
                        order,
                    ) {
                        return (admin_mob_view(body, mob, data), None);
                    }
                }
                RoomObjectRef::Player(player_name) => {
                    let Some(desc) = other_player_descs.get(&player_name) else {
                        continue;
                    };
                    if python_room_match_selected(
                        player_match_kind(&player_name),
                        &mut ordered_exact,
                        &mut ordered_prefix,
                        order,
                    ) {
                        let msg = format!("{} 당신을 살펴봅니다.", body.han_iga());
                        return (vec![desc.clone()], Some((player_name, msg)));
                    }
                }
                RoomObjectRef::SummonedUser(id) => {
                    let Some(user) = world.summoned_users().iter().find(|user| user.id == id)
                    else {
                        continue;
                    };
                    let player_name = user.body.get_name();
                    let reactions = user.body.get_string("반응이름");
                    let match_kind = if user.body.get_int("투명상태") != 1 {
                        python_room_name_match(&player_name, &reactions, &name)
                    } else {
                        0
                    };
                    if python_room_match_selected(
                        match_kind,
                        &mut ordered_exact,
                        &mut ordered_prefix,
                        order,
                    ) {
                        return (vec![user.body.get_desc_for_look(false)], None);
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
                    let match_kind = python_room_name_match(&box_value.getName(), &aliases, &name);
                    drop(box_value);
                    if python_room_match_selected(
                        match_kind,
                        &mut ordered_exact,
                        &mut ordered_prefix,
                        order,
                    ) {
                        return (item_view(box_object), None);
                    }
                }
                RoomObjectRef::Box(_) => {}
                RoomObjectRef::Fixture(id) => {
                    let match_kind = world
                        .get_fixture(id)
                        .map(|fixture| {
                            let (exact, prefixes) = fixture.match_counts(&name);
                            if exact {
                                1
                            } else if prefixes > 0 {
                                2
                            } else {
                                0
                            }
                        })
                        .unwrap_or(0);
                    if python_room_match_selected(
                        match_kind,
                        &mut ordered_exact,
                        &mut ordered_prefix,
                        order,
                    ) {
                        *selected_fixture = world.get_fixture(id).cloned();
                        return (Vec::new(), None);
                    }
                }
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
        if player_matches(pname) {
            c += 1;
            if c == order {
                let msg = format!("{} 당신을 살펴봅니다.", body.han_iga());
                return (vec![desc.clone()], Some((pname.clone(), msg)));
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
    engine.register_fn("han_un", |name: &str| -> String { han_eun(name) });
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
        move |_player_data: &mut rhai::Map, msg: &str| match oc.lock() {
            Ok(mut output) => {
                output.push(msg.to_string());
            }
            Err(e) => {
                tracing::error!(error = ?e, "Rhai output collector lock failed");
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
        "python_value_is_empty",
        |player_data: &mut rhai::Map, key: &str| -> bool {
            player_data
                .get(key)
                .is_none_or(|value| value.is_string() && value.to_string().is_empty())
        },
    );
    engine.register_fn("python_chars", |value: &str| -> rhai::Array {
        value
            .chars()
            .map(|character| Dynamic::from(character.to_string()))
            .collect()
    });
    engine.register_fn("python_split_once", |value: &str| -> rhai::Array {
        let input = value.trim_start();
        let Some(boundary) = input.find(char::is_whitespace) else {
            return if input.is_empty() {
                Vec::new()
            } else {
                vec![Dynamic::from(input.to_string())]
            };
        };
        let first = &input[..boundary];
        let rest = input[boundary..].trim_start();
        if rest.is_empty() {
            vec![Dynamic::from(first.to_string())]
        } else {
            vec![
                Dynamic::from(first.to_string()),
                Dynamic::from(rest.to_string()),
            ]
        }
    });
    engine.register_fn("map_has_key", |map: rhai::Map, key: &str| -> bool {
        map.contains_key(key)
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
    engine.register_fn("euc_kr_len", euc_kr_len);
    engine.register_fn("get_murim_config_list", get_murim_main_config_list);
    engine.register_fn("get_murim_config", get_murim_config_value);

    engine.register_fn("strip_ansi", strip_ansi_like_python);
    engine.register_fn("strict_int", strict_int_result);

    engine.register_fn("pad_start", |s: &str, width: i64, fill: &str| -> String {
        let len = s.chars().count() as i64;
        if len >= width {
            s.to_string()
        } else {
            format!("{}{}", fill.repeat((width - len) as usize), s)
        }
    });

    // repeat function for Rhai scripts
    engine.register_fn("repeat", |s: &str, count: i64| -> String {
        s.repeat(count.max(0) as usize)
    });
    engine.register_fn("python_floor_div", |left: i64, right: i64| -> i64 {
        let quotient = left / right;
        let remainder = left % right;
        if remainder != 0 && (left < 0) != (right < 0) {
            quotient - 1
        } else {
            quotient
        }
    });

    engine.register_fn("to_int", |s: &str| -> i64 { s.trim().parse().unwrap_or(0) });
    engine.register_fn("parse_int_prefix", parse_int_prefix);

    engine.register_fn("int_to_str", |i: i64| -> String { i.to_string() });

    engine.register_fn("split", |s: &str, sep: &str| -> rhai::Array {
        s.split(sep)
            .map(|x| rhai::Dynamic::from(x.to_string()))
            .collect()
    });

    // Parse "2검" to (order, name). Returns [order: i64, name: string] as Array.
    // Python getNameOrder: "1" 전부 숫자면 name="1" 유지(아이템 "1" 찾음). "2.검"이면 order=2, name=".검".
    engine.register_fn("parse_order_name", |s: &str| -> rhai::Array {
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

    // ============================================================
    // DATA LOADING (get_item_data, get_mob_data, get_room_data, get_skill_data)
    // ============================================================

    // Python `Item.Items` catalog used by administrator search commands.
    engine.register_fn("get_item_catalog", item_catalog);
    engine.register_fn("resolve_skill_definition", resolve_skill_definition_name);
    engine.register_fn("skill_definition_names", skill_definition_names);

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
        let numeric_type = parse_int_prefix(search_term);
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
        help_text_from_root(&root, key)
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
    // Commands execute against Python's individual Room.objs model. Upgrade
    // any legacy compact floor entries before installing lookup efuns.
    materialize_legacy_room_stacks(body);
    let oc = output_collector.clone();
    let mut engine = create_engine_with_output(output_collector);
    #[cfg(test)]
    engine.register_fn("__test_sleep_ms", |milliseconds: i64| {
        std::thread::sleep(Duration::from_millis(milliseconds.max(0) as u64));
    });
    let body_ptr = body as *mut Body;
    let spec = special_collector.clone();

    // Python `cmds/업데이트.py` owns every user-visible branch and
    // message. Rust exposes only the corresponding cache/hot-reload operation.
    crate::command::commands::update::register_update_efun(
        &mut engine,
        body_ptr,
        global_data.clone(),
    );

    // Python's global HELP object is loaded at startup and changes only when
    // `도움말 업데이트` calls HELP.load().  The real server owns GlobalData,
    // so use that snapshot instead of rereading help.json for every command.
    // Bare test/tool engines without GlobalData retain the file-backed efun
    // registered by create_engine_with_output.
    if let Some(help_data) = global_data.clone() {
        engine.register_fn("get_help", move |topic: &str| -> String {
            help_data
                .read()
                .ok()
                .and_then(|data| data.get_clone("help"))
                .map(|root| help_text_from_root(&root, topic))
                .unwrap_or_default()
        });
    }

    // `skill.json` is loaded with the other global configuration files and is
    // refreshed by `무공 업데이트`.  Replace the base engine's file-backed
    // fallback so ordinary command execution reads that snapshot instead of
    // reparsing the complete file for every `get_skill_data()` call.
    if let Some(skill_data) = global_data.clone() {
        register_cached_skill_data_efun(&mut engine, skill_data);
    }

    // `murim.json` is part of the same hot-reloadable GlobalData snapshot.
    // Keep ordinary commands off the synchronous file-read fallback installed
    // by the base engine.
    if let Some(murim_data) = global_data.clone() {
        register_cached_murim_config_efun(&mut engine, murim_data);
    }

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

    // Room-bound interactive objects expose only identity, placement, and
    // attribute state. Rhai owns their interaction rules and visible output.
    fixture::register_fixture_efuns(&mut engine, body_ptr);

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
                if !inventory_compat::store_acquired_object(&mut body.object, arc, true) {
                    return String::new();
                }
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
            if let Some(ref key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, name)
            {
                if is_stackable(key) {
                    // A compact entry without its authoritative item JSON
                    // cannot be reconstructed as Python Item objects. Keep it
                    // in the inventory instead of silently moving an
                    // unselectable floor count.
                    if object_from_item_json(key).is_none() {
                        return 0;
                    }
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
                        let mut dropped_items = Vec::with_capacity(drop_cnt as usize);
                        for _ in 0..drop_cnt {
                            if let Some((item, _)) = object_from_item_json(key) {
                                w.get_room_objs_mut(&zone, &room).insert(0, item.clone());
                                dropped_items.push(item);
                            }
                        }
                        let compact_fallback = drop_cnt - dropped_items.len() as i64;
                        if compact_fallback > 0 {
                            *w.get_room_objs_stack_mut(&zone, &room)
                                .entry(key.clone())
                                .or_insert(0) += compact_fallback;
                        }
                        for item in &dropped_items {
                            w.record_floor_item(&zone, &room, item);
                        }
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
                    room_objs.insert(0, arc.clone());
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
            {
                let room_stack = w.get_room_objs_stack_mut(&zone, &room);
                if let Some(ref key) = inventory_compat::find_counted_item_key(room_stack, name) {
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
            let mut removed_floor_items = Vec::new();
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
                    if !inventory_compat::store_acquired_object(&mut body.object, arc.clone(), true)
                    {
                        room_list.insert(i, arc);
                        i += 1;
                        continue;
                    }
                    removed_floor_items.push(arc);
                    taken += 1;
                } else {
                    i += 1;
                }
            }
            for item in &removed_floor_items {
                w.remove_floor_item_record(&zone, &room, item);
            }
            if taken > 0 {
                drop(w);
                let path = format!("data/user/{}.json", body.get_name());
                let _ = save_body_to_json(body, &path);
            }
            taken as i64
        },
    );

    let body_ptr_get_detail = body_ptr;
    engine.register_fn(
        "item_get_detail",
        move |_ob: &mut rhai::Map, name: &str, count: i64| -> rhai::Map {
            let body = unsafe { &mut *body_ptr_get_detail };
            let mut result = rhai::Map::new();
            result.insert("status".into(), Dynamic::from("missing"));
            result.insert("groups".into(), Dynamic::from(rhai::Array::new()));
            if name.is_empty() {
                return result;
            }
            let Some(position) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                return result;
            };
            let max_items = get_murim_config_int("사용자아이템갯수").max(0) as usize;
            let max_weight = body.get_str().saturating_mul(10);
            let wanted = count.clamp(0, 100) as usize;
            let mut groups: Vec<(String, String, i64)> = Vec::new();
            let mut removed = Vec::new();
            let mut blocked = "missing";
            let mut world = get_world_state().write().unwrap();
            {
                let floor = world.get_room_objs_mut(&position.zone, &position.room);
                let mut index = 0usize;
                while index < floor.len() && removed.len() < wanted {
                    let (matches, item_name, post, weight, oneitem_index) = {
                        let Ok(item) = floor[index].lock() else {
                            index += 1;
                            continue;
                        };
                        let matches = item.getName() == name
                            || reaction_names(&item.getString("반응이름"))
                                .iter()
                                .any(|alias| alias == name);
                        let item_name = item.getName();
                        let ansi = item.getString("안시");
                        let shown = if ansi.is_empty() {
                            format!("\x1b[0;36m{item_name}\x1b[37m")
                        } else {
                            format!("{ansi}{item_name}\x1b[0;37m")
                        };
                        (
                            matches,
                            item_name.clone(),
                            format!("{shown}{}", han_eul(&item_name)),
                            item.getInt("무게"),
                            item.getString("인덱스"),
                        )
                    };
                    if !matches {
                        index += 1;
                        continue;
                    }
                    if floor[index].lock().ok().is_some_and(|item| {
                        !inventory_compat::can_accept_object(&body.object, &item)
                    }) {
                        index += 1;
                        continue;
                    }
                    if body.get_item_weight().saturating_add(weight) > max_weight {
                        blocked = "too_heavy";
                        break;
                    }
                    if body.get_item_count() > max_items {
                        blocked = "inv_full";
                        break;
                    }
                    let item = floor.remove(index);
                    let accepted = inventory_compat::store_acquired_object(
                        &mut body.object,
                        item.clone(),
                        true,
                    );
                    debug_assert!(accepted);
                    if !oneitem_index.is_empty()
                        && item
                            .lock()
                            .is_ok_and(|item| item.checkAttr("아이템속성", "단일아이템"))
                    {
                        let _ = crate::oneitem::oneitem_have(&oneitem_index, &body.get_name());
                    }
                    if let Some(group) = groups.iter_mut().find(|group| group.0 == item_name) {
                        group.2 += 1;
                    } else {
                        groups.push((item_name, post, 1));
                    }
                    removed.push(item);
                }
            }
            for item in &removed {
                world.remove_floor_item_record(&position.zone, &position.room, item);
            }
            if removed.is_empty() {
                result.insert("status".into(), Dynamic::from(blocked));
                return result;
            }
            drop(world);
            let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
            let groups = groups
                .into_iter()
                .map(|(name, post, count)| {
                    let mut group = rhai::Map::new();
                    group.insert("name".into(), Dynamic::from(name));
                    group.insert("post".into(), Dynamic::from(post));
                    group.insert("count".into(), Dynamic::from(count));
                    Dynamic::from(group)
                })
                .collect::<rhai::Array>();
            result.insert("status".into(), Dynamic::from("ok"));
            result.insert("groups".into(), Dynamic::from(groups));
            result
        },
    );

    let body_ptr_get_all = body_ptr;
    engine.register_fn("item_get_all", move |_ob: &mut rhai::Map| -> rhai::Array {
        let body = unsafe { &mut *body_ptr_get_all };
        let mut world = get_world_state().write().unwrap();
        let Some(position) = world.get_player_position(&body.get_name()).cloned() else {
            return rhai::Array::new();
        };
        let max_items = get_murim_config_int("사용자아이템갯수").max(0) as usize;
        let max_weight = body.get_str().saturating_mul(10);
        let floor = world.get_room_objs_mut(&position.zone, &position.room);
        let mut groups: Vec<(String, String, i64)> = Vec::new();
        let mut removed = Vec::new();
        let mut index = 0usize;
        while index < floor.len() {
            let (name, post, weight) = match floor[index].lock() {
                Ok(item) => {
                    let name = item.getName();
                    let ansi = item.getString("안시");
                    let shown = if ansi.is_empty() {
                        format!("\x1b[0;36m{name}\x1b[37m")
                    } else {
                        format!("{ansi}{name}\x1b[0;37m")
                    };
                    (
                        name.clone(),
                        format!("{shown}{}", han_eul(&name)),
                        item.getInt("무게"),
                    )
                }
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
            if floor[index]
                .lock()
                .ok()
                .is_some_and(|item| !inventory_compat::can_accept_object(&body.object, &item))
            {
                index += 1;
                continue;
            }
            let item = floor.remove(index);
            if let Ok(item_value) = item.lock() {
                if item_value.checkAttr("아이템속성", "단일아이템") {
                    let unique_index = item_value.getString("인덱스");
                    if !unique_index.is_empty() {
                        let _ = crate::oneitem::oneitem_have(&unique_index, &body.get_name());
                    }
                }
            }
            let accepted =
                inventory_compat::store_acquired_object(&mut body.object, item.clone(), true);
            debug_assert!(accepted);
            removed.push(item);
            if let Some(group) = groups.iter_mut().find(|group| group.0 == name) {
                group.2 += 1;
            } else {
                groups.push((name, post, 1));
            }
        }
        if groups.is_empty() {
            return rhai::Array::new();
        }
        for item in &removed {
            world.remove_floor_item_record(&position.zone, &position.room, item);
        }
        drop(world);
        let path = format!("data/user/{}.json", body.get_name());
        let _ = save_body_to_json(body, &path);
        groups
            .into_iter()
            .map(|(name, post, count)| {
                let mut item = rhai::Map::new();
                item.insert("name".into(), Dynamic::from(name));
                item.insert("post".into(), Dynamic::from(post));
                item.insert("count".into(), Dynamic::from(count));
                Dynamic::from(item)
            })
            .collect()
    });

    engine.register_fn(
        "item_destroy",
        move |_ob: &mut rhai::Map, name: &str, order: i64, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr };
            destroy_inventory_for_command(body, name, order, count, false)
                .get("count")
                .and_then(|value| value.as_int().ok())
                .unwrap_or(0)
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
    engine.register_fn(
        "decompose_all_items",
        move |_ob: &mut rhai::Map| -> Dynamic {
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
                        (nonempty("물건판매") || nonempty("물건구입")).then(|| nonempty("물건구입"))
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
                    || item.getString("인덱스").contains("올숙무기")
                {
                    continue;
                }
                let kind = item.getString("종류");
                if kind != "방어구" && kind != "무기" {
                    continue;
                }
                if item.getString("옵션").is_empty() {
                    continue;
                }
                // Python getOption() can return an empty dict for a nonempty
                // but malformed option string; that item is still decomposed.
                let option_count = item.get_option().map(|options| options.len()).unwrap_or(0);
                if option_count >= 4 {
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
                if let Ok(item) = arc.lock() {
                    if item.checkAttr("아이템속성", "단일아이템") {
                        let index = item.getString("인덱스");
                        if !index.is_empty() {
                            let _ = crate::oneitem::oneitem_destroy(&index);
                        }
                    }
                }
                body.object.remove(&arc);
            }
            if shards > 0 {
                if is_stackable("강철조각") {
                    *body
                        .object
                        .inv_stack
                        .entry("강철조각".to_string())
                        .or_insert(0) += shards;
                } else {
                    for _ in 0..shards {
                        if let Some((arc, _)) = object_from_item_json("강철조각") {
                            body.object.append(arc);
                        }
                    }
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
        },
    );

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
    let body_ptr_inventory_indices = body_ptr;
    engine.register_fn(
        "list_inventory_indices",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_inventory_indices };
            let mut counts = indexmap::IndexMap::<String, i64>::new();
            for arc in &body.object.objs {
                let Ok(item) = arc.lock() else { continue };
                if item.getBool("inUse") {
                    continue;
                }
                *counts.entry(item.getString("인덱스")).or_insert(0) += 1;
            }
            for (key, count) in &body.object.inv_stack {
                *counts.entry(key.clone()).or_insert(0) += *count;
            }
            counts
                .into_iter()
                .map(|(key, count)| Dynamic::from(vec![Dynamic::from(key), Dynamic::from(count)]))
                .collect()
        },
    );
    let body_ptr_destroy_index = body_ptr;
    engine.register_fn(
        "destroy_inventory_index",
        move |_ob: &mut rhai::Map, key: &str, count: i64| -> i64 {
            let body = unsafe { &mut *body_ptr_destroy_index };
            let mut remaining = count.max(0);
            let mut removed = 0_i64;
            let candidates = body.object.objs.clone();
            for arc in candidates {
                if remaining == 0 {
                    break;
                }
                let matches = arc
                    .lock()
                    .is_ok_and(|item| !item.getBool("inUse") && item.getString("인덱스") == key);
                if matches {
                    body.object.remove(&arc);
                    remaining -= 1;
                    removed += 1;
                }
            }
            if remaining > 0 {
                let have = body.object.inv_stack.get(key).copied().unwrap_or(0);
                let take = remaining.min(have);
                if take > 0 {
                    if have == take {
                        body.object.inv_stack.remove(key);
                    } else {
                        body.object.inv_stack.insert(key.to_string(), have - take);
                    }
                    removed += take;
                }
            }
            removed
        },
    );

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
            let mut stack_keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
            stack_keys.sort();
            for key in stack_keys {
                let count = body.object.inv_stack.get(&key).copied().unwrap_or(0);
                let Some((item, _)) = object_from_item_json(&key) else {
                    continue;
                };
                let Ok(item) = item.lock() else { continue };
                let kind = item.getString("종류");
                if (kind != "방어구" && kind != "무기")
                    || !reaction_names(&item.getString("반응이름"))
                        .iter()
                        .any(|name| name == set_name)
                {
                    continue;
                }
                let name = item.getName();
                for _ in 0..count {
                    let order = orders.entry(name.clone()).or_insert(0);
                    *order += 1;
                    let mut saved = rhai::Map::new();
                    saved.insert("name".into(), Dynamic::from(name.clone()));
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
        move |_ob: &mut rhai::Map, item_selector: &str, option_name: &str, value: i64| -> String {
            let body = unsafe { &mut *body_ptr_set_option };
            let (name, order) = crate::command::CommandParser::parse_name_order(item_selector);
            let Some(arc) = inventory_object_for_mutation(body, &name, order) else {
                return "no_item".into();
            };
            if let Ok(mut item) = arc.lock() {
                let mut options: Vec<(String, i64)> = item
                    .getString("옵션")
                    .lines()
                    .filter_map(|line| {
                        let words: Vec<&str> = line.split_whitespace().collect();
                        (words.len() == 2)
                            .then(|| words[1].parse::<i64>().ok().map(|v| (words[0].into(), v)))
                            .flatten()
                    })
                    .collect();
                if let Some((_, current)) = options.iter_mut().find(|(key, _)| key == option_name) {
                    *current = value;
                } else {
                    options.push((option_name.to_string(), value));
                }
                item.set(
                    "옵션",
                    options
                        .iter()
                        .map(|(key, value)| format!("{key} {value}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                let current_name = item.getString("이름");
                item.set("이름", format!("\x1b[1;34m{current_name}\x1b[0;37m"));
                return "ok".into();
            }
            "no_item".into()
        },
    );

    let body_ptr_clear_magic = body_ptr;
    engine.register_fn(
        "clear_item_magic",
        move |_ob: &mut rhai::Map, item_selector: &str| -> String {
            let body = unsafe { &mut *body_ptr_clear_magic };
            let (name, order) = crate::command::CommandParser::parse_name_order(item_selector);
            let Some(arc) = inventory_object_for_mutation(body, &name, order) else {
                return "no_item".into();
            };
            let Ok(mut item) = arc.lock() else {
                return "no_item".into();
            };
            if !item.attr.contains_key("옵션") {
                drop(item);
                if inventory_compat::absorb_pristine_object(&mut body.object, &arc) {
                    body.object.remove(&arc);
                }
                return "no_option".into();
            }
            let option = item.getString("옵션");
            item.attr.remove("아이템속성");
            item.attr.remove("옵션");
            if option.is_empty() {
                "cleared_empty".into()
            } else {
                option
            }
        },
    );

    let body_ptr_apply_magic = body_ptr;
    engine.register_fn(
        "apply_item_magic",
        move |_ob: &mut rhai::Map, line: &str, level: i64| -> String {
            use rand::Rng;
            let body = unsafe { &mut *body_ptr_apply_magic };
            let Some(first) = line.split_whitespace().next() else {
                return "no_item".into();
            };
            let (name, order) = crate::command::CommandParser::parse_name_order(first);
            let Some(arc) = inventory_object_for_mutation(body, &name, order) else {
                return "no_item".into();
            };
            let Ok(mut item) = arc.lock() else {
                return "no_item".into();
            };
            let mut rng = rand::thread_rng();
            let _ = apply_item_magic_with_roll(&mut item, level, 6, false, &mut |low, high| {
                rng.gen_range(low..=high)
            });
            // opts/종류/확률과 무관하게 Python command가 applyMagic 뒤에
            // 현재 이름 전체를 다시 파란 ANSI로 감싼다.
            let current_name = item.getString("이름");
            item.set("이름", format!("\x1b[1;34m{current_name}\x1b[0;37m"));
            "ok".into()
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
            let body = unsafe { &mut *body_ptr_inventory_view };

            let target = if !line.is_empty() && admin >= 1000 {
                match select_python_room_object(body, line) {
                    Some(RoomObjectRef::SummonedUser(id)) => get_world_state()
                        .read()
                        .ok()
                        .and_then(|world| {
                            world
                                .summoned_users()
                                .iter()
                                .find(|user| user.id == id)
                                .map(|user| build_room_player_inventory_snapshot(&user.body))
                        }),
                    Some(RoomObjectRef::Player(name)) if name == body.get_name() => {
                        Some(build_room_player_inventory_snapshot(body))
                    }
                    Some(RoomObjectRef::Player(name)) => {
                        PRE_COMPUTED_ROOM_INVENTORIES.with(|cell| {
                            cell.borrow().as_deref().and_then(|players| {
                                find_room_inventory_target(&name, players)
                            })
                        })
                    }
                    None if current_body_position(body).is_none() => {
                        PRE_COMPUTED_ROOM_INVENTORIES.with(|cell| {
                            cell.borrow().as_deref().and_then(|players| {
                                find_room_inventory_target(line, players)
                            })
                        })
                    }
                    _ => None,
                }
            } else {
                if matches!(body.object.attr.get("금전"), Some(Value::String(value)) if value.is_empty()) {
                    body.set("금전", 0_i64);
                }
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
            let admin = body.get_int("관리자등급");
            if line.is_empty() || admin < 1000 {
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
            PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                let targets_guard = cell.borrow();
                let Some(targets) = targets_guard.as_ref() else {
                    return rhai::Array::new();
                };
                let Some(target) = find_room_mugong_target(line, targets) else {
                    return rhai::Array::new();
                };
                if target.kind == RoomMugongTargetKind::Item {
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

    // get_merchant_script(ob): Python 물건판매스크립의 원소 경계를
    // 보존한 배열. 품목표.py는 배열 원소마다 sendLine을 호출한다.
    let body_ptr_merchant = body_ptr;
    engine.register_fn(
        "get_merchant_script",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_merchant };
            let name = body.get_name();
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return rhai::Array::new(),
            };
            let Some(position) = w.get_player_position(&name) else {
                return rhai::Array::new();
            };
            let mobs = w
                .mob_cache
                .get_all_mobs_in_room(&position.zone, &position.room);
            let ordered = w.get_room_object_order(&position.zone, &position.room);
            let mut ids = ordered
                .into_iter()
                .filter_map(|object| match object {
                    crate::world::RoomObjectRef::Mob(id) => Some(id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if ids.is_empty() {
                ids.extend(mobs.iter().map(|mob| mob.instance_id));
            }
            for id in ids {
                let Some(mob) = mobs.iter().find(|mob| mob.instance_id == id) else {
                    continue;
                };
                let Some(data) = w.mob_cache.get_instance_data(mob) else {
                    continue;
                };
                if !data.items_for_sale.is_empty() || data.buy_percent > 0 {
                    return data
                        .sale_script
                        .iter()
                        .cloned()
                        .map(Dynamic::from)
                        .collect();
                }
            }
            rhai::Array::new()
        },
    );

    // get_merchant_buy_percent(ob): 현재 방의 물건구입 상인 몹의 구입 비율(1–100 등). 없으면 0.
    let body_ptr_buy = body_ptr;
    engine.register_fn(
        "get_merchant_buy_percent",
        move |_ob: &mut rhai::Map| -> i64 {
            let body = unsafe { &*body_ptr_buy };
            let Some((zone, room)) = current_body_position(body) else {
                return 0;
            };
            let w = match get_world_state().read() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            let mobs = w.mob_cache.get_all_mobs_in_room(&zone, &room);
            let mut ids = w
                .get_room_object_order(&zone, &room)
                .into_iter()
                .filter_map(|object| match object {
                    crate::world::RoomObjectRef::Mob(id) => Some(id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if ids.is_empty() {
                ids.extend(mobs.iter().map(|mob| mob.instance_id));
            }
            for id in ids {
                let Some(mob) = mobs.iter().find(|mob| mob.instance_id == id) else {
                    continue;
                };
                let Some(data) = w.mob_cache.get_instance_data(mob) else {
                    continue;
                };
                // Python Room.findMerchant returns the first mob with either
                // 물건판매 or 물건구입. 판매.py then rejects that same mob if
                // its 물건구입 is empty; it does not scan for a later buyer.
                if !data.items_for_sale.is_empty() || data.buy_percent > 0 {
                    return data.buy_percent;
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
                // Python Room.findMerchant iterates room.objs without checking
                // ACT_DEATH/ACT_REGEN or visibility.
                world
                    .mob_cache
                    .get_all_mobs_in_room(&zone, &room)
                    .into_iter()
                    .any(|mob| {
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
            let Some(target) = find_room_fund_target(body, "표두") else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                return Dynamic::from(result);
            };
            if requested <= 0 {
                result.insert("status".into(), Dynamic::from("invalid_amount"));
                return Dynamic::from(result);
            }
            let amount = requested.min(body.get_int("은전"));
            let donates_to_self = matches!(
                &target,
                RoomFundTarget::Player(name) if name == &body.get_name()
            );
            let (total, guard_key) = match target {
                RoomFundTarget::Player(name) if name == body.get_name() => {
                    // Python subtracts and then adds on the same object.
                    (body.get_int("은전"), None)
                }
                RoomFundTarget::Player(name) => {
                    let Some(total) = cast::with_room_player_body_mut(&name, |target| {
                        let total = target.get_int("은전").saturating_add(amount);
                        target.set("은전", total);
                        let _ = save_body_to_json(
                            target,
                            &format!("data/user/{}.json", target.get_name()),
                        );
                        total
                    }) else {
                        result.insert("status".into(), Dynamic::from("no_guard"));
                        return Dynamic::from(result);
                    };
                    (total, None)
                }
                RoomFundTarget::Summoned(id) => {
                    let mut world = get_world_state().write().unwrap();
                    let Some(target) = world.summoned_user_mut(id) else {
                        result.insert("status".into(), Dynamic::from("no_guard"));
                        return Dynamic::from(result);
                    };
                    let total = target.body.get_int("은전").saturating_add(amount);
                    target.body.set("은전", total);
                    (total, None)
                }
                RoomFundTarget::Object(object) => {
                    let Ok(mut object) = object.lock() else {
                        result.insert("status".into(), Dynamic::from("no_guard"));
                        return Dynamic::from(result);
                    };
                    let total = object.getInt("은전").saturating_add(amount);
                    object.set("은전", total);
                    persist_room_object_gold(&object, total);
                    (total, None)
                }
                RoomFundTarget::Mob(guard_id, guard_key) => {
                    let mut world = get_world_state().write().unwrap();
                    let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
                        result.insert("status".into(), Dynamic::from("no_guard"));
                        return Dynamic::from(result);
                    };
                    let Some(guard) = mobs.iter_mut().find(|mob| mob.instance_id == guard_id)
                    else {
                        result.insert("status".into(), Dynamic::from("no_guard"));
                        return Dynamic::from(result);
                    };
                    guard.gold = guard.gold.saturating_add(amount);
                    (guard.gold, Some(guard_key))
                }
            };
            if !donates_to_self {
                body.set("은전", body.get_int("은전").saturating_sub(amount));
            }

            let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
            if let Some((mob_zone, mob_id)) =
                guard_key.as_deref().and_then(|key| key.split_once(':'))
            {
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
            let Some(target) = find_room_fund_target(body, "표두") else {
                result.insert("status".into(), Dynamic::from("no_guard"));
                result.insert("total".into(), Dynamic::from(total));
                return Dynamic::from(result);
            };
            let guard_gold = match &target {
                RoomFundTarget::Player(name) if name == &body.get_name() => body.get_int("은전"),
                RoomFundTarget::Player(name) => {
                    cast::with_room_player_body_mut(name, |target| target.get_int("은전"))
                        .unwrap_or(0)
                }
                RoomFundTarget::Summoned(id) => get_world_state()
                    .read()
                    .ok()
                    .and_then(|world| {
                        world
                            .summoned_users()
                            .iter()
                            .find(|user| user.id == *id)
                            .map(|user| user.body.get_int("은전"))
                    })
                    .unwrap_or(0),
                RoomFundTarget::Object(object) => object
                    .lock()
                    .map(|object| object.getInt("은전"))
                    .unwrap_or(0),
                RoomFundTarget::Mob(guard_id, _) => get_world_state()
                    .read()
                    .ok()
                    .and_then(|world| {
                        world
                            .mob_cache
                            .get_all_mobs_in_room(&zone, &room)
                            .into_iter()
                            .find(|mob| mob.instance_id == *guard_id)
                            .map(|mob| mob.gold)
                    })
                    .unwrap_or(0),
            };
            let mut remaining_guard_gold = guard_gold;
            if amount <= 0 {
                status = "invalid_amount";
            } else if body.get_int("레벨") > 500 {
                status = "high_level";
            } else if amount > 10_000_000 {
                status = "too_greedy";
            } else if amount > guard_gold {
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
                    remaining_guard_gold = guard_gold.saturating_sub(amount);
                    match &target {
                        RoomFundTarget::Player(name) if name == &body.get_name() => {
                            // `ob` and `mob` are the same Python object here:
                            // `ob['은전'] += m` followed by `mob['은전'] -= m`
                            // leaves the original silver unchanged.
                            body.set("은전", guard_gold);
                        }
                        RoomFundTarget::Player(name) => {
                            let _ = cast::with_room_player_body_mut(name, |target| {
                                target.set("은전", remaining_guard_gold);
                                let _ = save_body_to_json(
                                    target,
                                    &format!("data/user/{}.json", target.get_name()),
                                );
                            });
                        }
                        RoomFundTarget::Summoned(id) => {
                            if let Ok(mut world) = get_world_state().write() {
                                if let Some(target) = world.summoned_user_mut(*id) {
                                    target.body.set("은전", remaining_guard_gold);
                                }
                            }
                        }
                        RoomFundTarget::Object(object) => {
                            if let Ok(mut object) = object.lock() {
                                object.set("은전", remaining_guard_gold);
                                persist_room_object_gold(&object, remaining_guard_gold);
                            }
                        }
                        RoomFundTarget::Mob(guard_id, _) => {
                            if let Ok(mut world) = get_world_state().write() {
                                if let Some(mobs) =
                                    world.mob_cache.get_all_mobs_in_room_mut(&zone, &room)
                                {
                                    if let Some(guard) =
                                        mobs.iter_mut().find(|mob| mob.instance_id == *guard_id)
                                    {
                                        guard.gold = remaining_guard_gold;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if status == "ok" {
                let _ = save_body_to_json(body, &format!("data/user/{}.json", body.get_name()));
                if let RoomFundTarget::Mob(_, guard_key) = &target {
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
            let Some((zone, room)) = current_body_position(body) else {
                m.insert("err".into(), Dynamic::from("no_merchant".to_string()));
                m.insert("bought".into(), Dynamic::from(0_i64));
                m.insert("display_name".into(), Dynamic::from(String::new()));
                m.insert("total_cost".into(), Dynamic::from(0_i64));
                return Dynamic::from(m);
            };
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
            let mobs = w.mob_cache.get_all_mobs_in_room(&zone, &room);
            let mut ids = w
                .get_room_object_order(&zone, &room)
                .into_iter()
                .filter_map(|object| match object {
                    crate::world::RoomObjectRef::Mob(id) => Some(id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if ids.is_empty() {
                ids.extend(mobs.iter().map(|mob| mob.instance_id));
            }
            let merchant = ids.into_iter().find_map(|id| {
                let mob = mobs.iter().find(|mob| mob.instance_id == id)?;
                let data = w.mob_cache.get_instance_data(mob)?;
                (!data.items_for_sale.is_empty() || data.buy_percent > 0).then_some(data)
            });
            let mut item_key = String::new();
            let mut unit_price = 0i64;
            let mut weight = 0i64;
            if let Some(data) = merchant {
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
            }
            if item_key.is_empty() {
                m.insert("err".into(), Dynamic::from("not_for_sale".to_string()));
                m.insert("bought".into(), Dynamic::from(0i64));
                m.insert("display_name".into(), Dynamic::from(display_name));
                m.insert("total_cost".into(), Dynamic::from(0i64));
                return Dynamic::from(m);
            }
            let display_post = object_from_item_json(&item_key)
                .and_then(|(item, _)| {
                    item.lock().ok().map(|item| {
                        let ansi = item.getString("안시");
                        if ansi.is_empty() {
                            format!("\x1b[0;36m{}\x1b[37m", item.getName())
                        } else {
                            format!("{}{}\x1b[0;37m", ansi, item.getName())
                        }
                    })
                })
                .unwrap_or_else(|| format!("\x1b[0;36m{display_name}\x1b[37m"));
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
                            "합성1"
                                | "합성2"
                                | "합성3"
                                | "합성4"
                                | "합성5"
                                | "합성6"
                                | "합성7"
                                | "합성8"
                                | "합성9"
                        )
                    };
                    let herb_count = body
                        .object
                        .objs
                        .iter()
                        .filter(|item| item.lock().is_ok_and(|item| herb(&item)))
                        .count() as i64
                        + body
                            .object
                            .inv_stack
                            .iter()
                            .filter_map(|(key, count)| {
                                object_from_item_json(key).and_then(|(item, _)| {
                                    item.lock()
                                        .ok()
                                        .and_then(|item| herb(&item).then_some(*count))
                                })
                            })
                            .sum::<i64>();
                    let named_count = |wanted: &str| {
                        body.object
                            .objs
                            .iter()
                            .filter(|item| item.lock().is_ok_and(|item| item.getName() == wanted))
                            .count() as i64
                            + body
                                .object
                                .inv_stack
                                .iter()
                                .filter_map(|(key, count)| {
                                    object_from_item_json(key).and_then(|(item, _)| {
                                        item.lock().ok().and_then(|item| {
                                            (item.getName() == wanted).then_some(*count)
                                        })
                                    })
                                })
                                .sum::<i64>()
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
                    let mut remaining = consume_count.saturating_sub(removed);
                    if remaining > 0 {
                        let mut stack_keys =
                            body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
                        stack_keys.sort();
                        for key in stack_keys {
                            if remaining == 0 {
                                break;
                            }
                            let matches = object_from_item_json(&key).is_some_and(|(item, _)| {
                                item.lock().is_ok_and(|item| {
                                    if consume_name == "약초" {
                                        herb(&item)
                                    } else {
                                        item.getName() == consume_name
                                    }
                                })
                            });
                            if !matches {
                                continue;
                            }
                            let have = body.object.inv_stack.get(&key).copied().unwrap_or(0);
                            let take = remaining.min(have);
                            let _ = inventory_compat::remove_pristine_count(
                                &mut body.object,
                                &key,
                                take,
                            );
                            remaining -= take;
                        }
                    }
                    body.object.objs.insert(0, guard_template);
                    let path = format!("data/user/{}.json", body.get_name());
                    let _ = save_body_to_json(body, &path);
                    m.insert("err".into(), Dynamic::from(String::new()));
                    m.insert("bought".into(), Dynamic::from(1_i64));
                    m.insert("display_name".into(), Dynamic::from(guard_name));
                    m.insert("post".into(), Dynamic::from(display_post));
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
            m.insert("post".into(), Dynamic::from(display_post));
            m.insert("total_cost".into(), Dynamic::from(total_cost));
            m.insert("guard".into(), Dynamic::from(false));
            Dynamic::from(m)
        },
    );

    // item_sell(ob, name, order, count, percent): 소지품을 상인에게 판매.
    // Returns [sold, total, display_name, err, post_name] where post_name is Item.getNameA().
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
            let error = |kind: &str| {
                vec![
                    Dynamic::from(0_i64),
                    Dynamic::from(0_i64),
                    Dynamic::from(String::new()),
                    Dynamic::from(kind.to_string()),
                    Dynamic::from(String::new()),
                ]
            };
            let matches = |object: &Object, wanted: &str| {
                object.getName() == wanted || object.getString("반응이름").contains(wanted)
            };
            let matching: Vec<_> = body
                .object
                .objs
                .iter()
                .filter(|item| {
                    item.lock()
                        .is_ok_and(|item| !item.getBool("inUse") && matches(&item, name))
                })
                .cloned()
                .collect();
            let requested = if order == 1 { count } else { 1 };
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();
            let mut stack_removals = Vec::<(String, i64)>::new();
            let mut total = 0i64;
            let mut display_name = String::new();
            let mut post_name = String::new();
            let mut remaining = requested;

            if order <= matching.len() {
                for current in matching.iter().skip(order - 1).take(remaining) {
                    let o = current.lock().unwrap();
                    if to_remove.is_empty() {
                        if o.checkAttr("아이템속성", "출력안함") {
                            return error("no_item");
                        }
                        if o.checkAttr("아이템속성", "팔지못함") {
                            return error("cant_sell");
                        }
                    }
                    let mut price = (o.getInt("판매가격") * percent) / 100;
                    if let Some(options) = o.get_option() {
                        price = (price as f64 * (options.len() as f64 * 1.3)) as i64;
                    }
                    total += price;
                    if display_name.is_empty() {
                        display_name = o.getName();
                        let ansi = o.getString("안시");
                        post_name = if ansi.is_empty() {
                            format!("\x1b[0;36m{}\x1b[37m", display_name)
                        } else {
                            format!("{}{}\x1b[0;37m", ansi, display_name)
                        };
                    }
                    drop(o);
                    to_remove.push(current.clone());
                    remaining -= 1;
                }
            }

            let mut stack_order = if order > matching.len() {
                order.saturating_sub(matching.len()) as i64
            } else {
                1
            };
            if remaining > 0 {
                for key in inventory_compat::counted_item_keys(&body.object.inv_stack, name) {
                    let have = body.object.inv_stack.get(&key).copied().unwrap_or(0).max(0);
                    if stack_order > have {
                        stack_order -= have;
                        continue;
                    }
                    let Some((template, _)) = object_from_item_json(&key) else {
                        continue;
                    };
                    let template = template.lock().unwrap();
                    if to_remove.is_empty() && stack_removals.is_empty() {
                        if template.checkAttr("아이템속성", "출력안함") {
                            return error("no_item");
                        }
                        if template.checkAttr("아이템속성", "팔지못함") {
                            return error("cant_sell");
                        }
                    }
                    let available = have.saturating_sub(stack_order - 1);
                    let take = (remaining as i64).min(available);
                    if take > 0 {
                        let mut price = (template.getInt("판매가격") * percent) / 100;
                        if let Some(options) = template.get_option() {
                            price = (price as f64 * (options.len() as f64 * 1.3)) as i64;
                        }
                        total += price.saturating_mul(take);
                        if display_name.is_empty() {
                            display_name = template.getName();
                            let ansi = template.getString("안시");
                            post_name = if ansi.is_empty() {
                                format!("\x1b[0;36m{}\x1b[37m", display_name)
                            } else {
                                format!("{}{}\x1b[0;37m", ansi, display_name)
                            };
                        }
                        stack_removals.push((key, take));
                        remaining -= take as usize;
                    }
                    stack_order = 1;
                    if remaining == 0 {
                        break;
                    }
                }
            }
            if to_remove.is_empty() && stack_removals.is_empty() {
                return error("no_item");
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
            let mut sold = to_remove.len() as i64;
            for (key, count) in stack_removals {
                if inventory_compat::remove_pristine_count(&mut body.object, &key, count) {
                    sold += count;
                }
            }
            body.set("은전", body.get_int("은전") + total);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            vec![
                Dynamic::from(sold),
                Dynamic::from(total),
                Dynamic::from(display_name),
                Dynamic::from(String::new()),
                Dynamic::from(post_name),
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
                let (name, post, price) = {
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
                    let name = item.getName();
                    let ansi = item.getString("안시");
                    let post = if ansi.is_empty() {
                        format!("\x1b[0;36m{name}\x1b[37m")
                    } else {
                        format!("{ansi}{name}\x1b[0;37m")
                    };
                    (name, post, price)
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
                event.insert("post".into(), Dynamic::from(post));
                event.insert("price".into(), Dynamic::from(price));
                sold.push(Dynamic::from(event));
            }
            let mut stack_keys = body.object.inv_stack.keys().cloned().collect::<Vec<_>>();
            stack_keys.sort();
            for key in stack_keys {
                let quantity = body.object.inv_stack.get(&key).copied().unwrap_or(0);
                if quantity <= 0 {
                    continue;
                }
                let Some((template, _)) = object_from_item_json(&key) else {
                    continue;
                };
                let Some((name, post, price, selected)) = template.lock().ok().map(|item| {
                    let kind = item.getString("종류");
                    let equipment = kind == "방어구" || kind == "무기";
                    let option_count = item.get_option().map(|value| value.len()).unwrap_or(0);
                    let selected = !item.checkAttr("아이템속성", "출력안함")
                        && !item.checkAttr("아이템속성", "팔지못함")
                        && match mode {
                            "속성0" => equipment && item.getString("옵션").is_empty(),
                            "속성1" => equipment && option_count <= 2,
                            "속성2" => equipment && option_count <= 3,
                            "속성3" => equipment && option_count <= 4,
                            "일반" => equipment && option_count == 0,
                            "장비" => equipment,
                            "모두" => true,
                            _ => false,
                        };
                    let mut price = (item.getInt("판매가격") * percent) / 100;
                    if let Some(options) = item.get_option() {
                        price = (price as f64 * (options.len() as f64 * 1.2)) as i64;
                    }
                    let name = item.getName();
                    let ansi = item.getString("안시");
                    let post = if ansi.is_empty() {
                        format!("\x1b[0;36m{name}\x1b[37m")
                    } else {
                        format!("{ansi}{name}\x1b[0;37m")
                    };
                    (name, post, price, selected)
                }) else {
                    continue;
                };
                if !selected {
                    continue;
                }
                body.object.inv_stack.remove(&key);
                total = total.saturating_add(price.saturating_mul(quantity));
                for _ in 0..quantity {
                    let mut event = rhai::Map::new();
                    event.insert("name".into(), Dynamic::from(name.clone()));
                    event.insert("post".into(), Dynamic::from(post.clone()));
                    event.insert("price".into(), Dynamic::from(price));
                    sold.push(Dynamic::from(event));
                }
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
            let mut selected_fixture = None;
            let (lines, to_target) = look_at_target(
                unsafe { &*body_ptr_ft },
                &world,
                &viewer_name,
                line,
                &other,
                &mut selected_fixture,
            );
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
            let is_player_target = to_target.is_some();
            let found = is_player_target
                || selected_fixture.is_some()
                || (!lines_out.is_empty() && err.is_empty());
            let mut m = rhai::Map::new();
            m.insert("found".into(), Dynamic::from(found));
            m.insert("player".into(), Dynamic::from(is_player_target));
            m.insert(
                "fixture_target".into(),
                Dynamic::from(selected_fixture.is_some()),
            );
            m.insert(
                "fixture".into(),
                selected_fixture
                    .as_ref()
                    .map(fixture::fixture_to_dynamic)
                    .unwrap_or_else(|| Dynamic::from(rhai::Map::new())),
            );
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

    // get_user_config/set_user_config: runtime user settings. Python does not
    // save here; the regular player-save cycle persists the changed array.
    let body_ptr_cfg = body_ptr;
    engine.register_fn(
        "get_user_config",
        move |_ob: &mut rhai::Map, key: &str| -> String {
            let body = unsafe { &*body_ptr_cfg };
            let raw = body.get_string("설정상태");
            let entries = if raw.contains('\n') || raw.contains('|') {
                raw.split(['\n', '|'])
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            } else {
                raw.split_whitespace()
                    .collect::<Vec<_>>()
                    .chunks(2)
                    .filter(|pair| pair.len() == 2)
                    .map(|pair| format!("{} {}", pair[0], pair[1]))
                    .collect::<Vec<_>>()
            };
            // Python `_checkConfig` stops at the first array entry whose
            // text starts with the requested CFG name.
            for entry in entries {
                if entry.starts_with(key) {
                    let words = entry.split_whitespace().collect::<Vec<_>>();
                    return if words.get(1) == Some(&"1") {
                        "1".to_string()
                    } else {
                        String::new()
                    };
                }
            }
            String::new()
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
        },
    );
    let body_ptr_toggle_cfg = body_ptr;
    engine.register_fn(
        "toggle_user_config_python",
        move |_ob: &mut rhai::Map, key: &str| -> bool {
            let body = unsafe { &mut *body_ptr_toggle_cfg };
            let raw = body.get_string("설정상태");
            let entries = if raw.contains('\n') || raw.contains('|') {
                raw.split(['\n', '|'])
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            } else {
                raw.split_whitespace()
                    .collect::<Vec<_>>()
                    .chunks(2)
                    .filter(|pair| pair.len() == 2)
                    .map(|pair| format!("{} {}", pair[0], pair[1]))
                    .collect::<Vec<_>>()
            };
            let mut found = false;
            let mut enabled = false;
            let mut updated = Vec::new();
            for entry in entries {
                if entry.starts_with(key) {
                    found = true;
                    let words = entry.split_whitespace().collect::<Vec<_>>();
                    if words.len() > 1 {
                        enabled = words[1] != "1";
                        updated.push(format!("{} {}", words[0], if enabled { "1" } else { "0" }));
                    }
                } else {
                    updated.push(entry);
                }
            }
            if !found {
                enabled = true;
                updated.push(format!("{key} 1"));
            }
            body.object
                .attr
                .insert("설정상태".to_string(), Value::String(updated.join("\n")));
            enabled
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
            let updated = event_list_set(&body.get_string("이벤트설정리스트"), key, value);
            mark_body_attr_as_json_array(body, "이벤트설정리스트");
            body.object
                .attr
                .insert("이벤트설정리스트".to_string(), Value::String(updated));
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
        },
    );
    let body_ptr_ev3 = body_ptr;
    engine.register_fn("del_user_event", move |_ob: &mut rhai::Map, key: &str| {
        let body = unsafe { &mut *body_ptr_ev3 };
        let updated = event_list_remove(&body.get_string("이벤트설정리스트"), key);
        mark_body_attr_as_json_array(body, "이벤트설정리스트");
        body.object
            .attr
            .insert("이벤트설정리스트".to_string(), Value::String(updated));
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
            if let Ok(world) = get_world_state().read() {
                world.mob_cache.check_mob_event(mob_key, event_key)
            } else {
                false
            }
        },
    );

    // set_mob_event(mob_key, event_key) - Set event on mob (Python: target.setEvent)
    engine.register_fn("set_mob_event", |mob_key: &str, event_key: &str| -> bool {
        if let Ok(mut world) = get_world_state().write() {
            world.mob_cache.set_mob_event(mob_key, event_key)
        } else {
            false
        }
    });

    // del_mob_event(mob_key, event_key) - Delete event from mob (Python: target.delEvent)
    engine.register_fn("del_mob_event", |mob_key: &str, event_key: &str| -> bool {
        if let Ok(mut world) = get_world_state().write() {
            world.mob_cache.del_mob_event(mob_key, event_key)
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
                if !ITEM_EQUIP_LEVELS.contains(&slot.as_str()) {
                    continue;
                }
                let name = o.getName();
                let is_han = crate::hangul::is_han(&strip_ansi_like_python(&name));
                // Python 장비.py는 반응이름이 list일 때만 첫 항목을 쓰고,
                // scalar 문자열이면 공백을 포함한 원문 전체를 표시한다.
                // Rust 객체의 Python 배열 호환 표현은 CRLF 구분 문자열이다.
                let reaction = o.getString("반응이름");
                let alias = reaction
                    .split_once("\r\n")
                    .map(|(first, _)| first.to_string())
                    .unwrap_or(reaction);
                pairs.push((slot, name, is_han, alias));
            }
        }
        pairs.sort_by_cached_key(|(slot, _, _, _)| {
            ITEM_EQUIP_LEVELS
                .iter()
                .position(|&level| level == slot.as_str())
                .expect("equipped slots are filtered against ITEM_EQUIP_LEVELS")
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
            // Python: savedSet = 'SET-' + str(uuid.uuid4())
            let set_name = format!("SET-{}", uuid::Uuid::new_v4());
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
                    let raw_names = item.getString("반응이름");
                    // JSON arrays are represented internally with `|`, while
                    // arrays produced at runtime by this command use CRLF.
                    // A plain Python string is one alias even when it contains
                    // spaces; `reaction_names()` would incorrectly split it.
                    let mut names = if raw_names.contains('|') {
                        raw_names
                            .split('|')
                            .filter(|name| !name.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    } else if raw_names.contains("\r\n") {
                        raw_names
                            .split("\r\n")
                            .filter(|name| !name.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    } else if raw_names.is_empty() {
                        Vec::new()
                    } else {
                        vec![raw_names]
                    };
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
    let body_ptr_str_stat = body_ptr;
    engine.register_fn("get_current_str", move |_ob: &mut rhai::Map| -> i64 {
        unsafe { &*body_ptr_str_stat }.get_str()
    });
    let body_ptr_dex_stat = body_ptr;
    engine.register_fn("get_current_dex", move |_ob: &mut rhai::Map| -> i64 {
        unsafe { &*body_ptr_dex_stat }.get_dex()
    });
    let body_ptr_hit_stat = body_ptr;
    engine.register_fn("get_current_hit", move |_ob: &mut rhai::Map| -> i64 {
        unsafe { &*body_ptr_hit_stat }.get_hit()
    });
    let body_ptr_miss_stat = body_ptr;
    engine.register_fn("get_current_miss", move |_ob: &mut rhai::Map| -> i64 {
        unsafe { &*body_ptr_miss_stat }.get_miss()
    });
    let body_ptr_critical_stat = body_ptr;
    engine.register_fn("get_current_critical", move |_ob: &mut rhai::Map| -> i64 {
        unsafe { &*body_ptr_critical_stat }.get_critical()
    });
    let body_ptr_luck_stat = body_ptr;
    engine.register_fn("get_current_luck", move |_ob: &mut rhai::Map| -> i64 {
        unsafe { &*body_ptr_luck_stat }.get_critical_chance()
    });
    let body_ptr_score_gold = body_ptr;
    engine.register_fn("get_score_gold", move |_ob: &mut rhai::Map| -> i64 {
        let body = unsafe { &mut *body_ptr_score_gold };
        if matches!(body.object.attr.get("금전"), Some(Value::String(value)) if value.is_empty())
        {
            // Python 점수.py: if ob['금전'] == '': ob['금전'] = 0
            body.set("금전", 0_i64);
        }
        body.get_int("금전")
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
    // Python getRoom(line)는 사전의 원문 "zone:room" 키를 그대로 조회한다.
    engine.register_fn("parse_room_spec_exact", |s: &str| -> Dynamic {
        let mut m = rhai::Map::new();
        let mut parts = s.splitn(2, ':');
        let zone = parts.next().unwrap_or("");
        let room = parts.next();
        if zone.is_empty() || room.is_none() {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(String::new()));
        } else {
            m.insert("zone".into(), Dynamic::from(zone.to_string()));
            m.insert("room".into(), Dynamic::from(room.unwrap().to_string()));
        }
        Dynamic::from(m)
    });

    // get_position_of(player_name): 해당 플레이어의 {zone, room}. 없으면 {zone:"", room:0}. 앞(소환) 등.
    engine.register_fn("get_position_of", |name: &str| -> Dynamic {
        let w = match get_world_state().read() {
            Ok(g) => g,
            Err(_) => return rhai::Map::new().into(),
        };
        let mut m = rhai::Map::new();
        let position = w.get_player_position(name).cloned().or_else(|| {
            w.summoned_users()
                .iter()
                .find(|user| user.body.get_name() == name)
                .map(|user| user.position.clone())
        });
        if let Some(p) = position {
            m.insert("zone".into(), Dynamic::from(p.zone.clone()));
            m.insert("room".into(), Dynamic::from(p.room.clone()));
        } else {
            m.insert("zone".into(), Dynamic::from(String::new()));
            m.insert("room".into(), Dynamic::from(0i64));
        }
        Dynamic::from(m)
    });

    // Python Player.enterRoom(..., "소환", "소환") still enforces all
    // destination-room restrictions.  Return state only; Rhai owns text.
    let body_ptr_summon_check = body_ptr;
    engine.register_fn(
        "check_summon_destination",
        move |_ob: &mut rhai::Map, zone: &str, room: &str| -> String {
            let body = unsafe { &*body_ptr_summon_check };
            check_summon_destination(body, zone, room)
        },
    );

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

    // set_value(ob, key, val): 실제 Rhai 호출 타입인 정수와 문자열을
    // 명시적으로 분리해 문자열(특히 공백 전용 값)을 원문 그대로 보존한다.
    let body_ptr_setv_int = body_ptr;
    engine.register_fn(
        "set_value",
        move |_ob: &mut rhai::Map, key: &str, val: i64| {
            unsafe { &mut *body_ptr_setv_int }.set(key, val);
        },
    );
    let body_ptr_setv_string = body_ptr;
    engine.register_fn(
        "set_value",
        move |_ob: &mut rhai::Map, key: &str, val: &str| {
            // Value::from(String)은 숫자 변환을 위해 trim하므로 Python의
            // 원문 문자열 속성 저장에는 사용할 수 없다.
            unsafe { &mut *body_ptr_setv_string }
                .object
                .attr
                .insert(key.to_string(), Value::String(val.to_string()));
        },
    );
    engine.register_fn(
        "set_text_affix",
        move |ob: &mut rhai::Map, key: &str, val: &str| {
            ob.insert(key.into(), Dynamic::from(val.to_string()));
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

            // Resolve the first Python Room.objs match before touching any
            // type-specific collection.  Without this, the old Rust code
            // always preferred floor items over players and mobs.
            #[derive(Clone)]
            enum OrderedValueTarget {
                Player(String),
                Summoned(u64),
                Mob(u64),
                Object(Arc<Mutex<Object>>),
                Fixture(u64),
            }
            let ordered_target = get_world_state().read().ok().and_then(|world| {
                let floor = world.get_room_objs(&zone, &room);
                let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
                let installed =
                    box_commands::installed_boxes_for_room(&zone, &room).unwrap_or_default();
                if target.chars().all(|character| character.is_ascii_digit()) {
                    let order = target.parse::<usize>().unwrap_or(0);
                    if order == 0 {
                        return None;
                    }
                    return mobs
                        .iter()
                        .filter(|mob| {
                            !matches!(mob.act, 2 | 3)
                                && world
                                    .get_mob_data(&mob.mob_key)
                                    .is_some_and(|data| data.mob_type != 7)
                        })
                        .nth(order - 1)
                        .map(|mob| OrderedValueTarget::Mob(mob.instance_id));
                }
                let players = room_admin_player_values(body).unwrap_or_default();
                let digit_count = target.chars().take_while(|ch| ch.is_ascii_digit()).count();
                let order = if digit_count == 0 {
                    1
                } else {
                    target[..digit_count].parse::<usize>().unwrap_or(0)
                };
                let query = &target[digit_count..];
                if order == 0 || query.is_empty() {
                    return None;
                }
                let classify = |name: &str, aliases: &[String]| {
                    let exact = name == query || aliases.iter().any(|alias| alias == query);
                    let prefixes = if exact {
                        0
                    } else {
                        aliases.iter().filter(|alias| alias.starts_with(query)).count()
                    };
                    (exact, prefixes)
                };
                let mut exact_count = 0usize;
                let mut prefix_count = 0usize;
                world
                    .get_room_object_order(&zone, &room)
                    .into_iter()
                    .find_map(|entry| {
                      let candidate = match entry {
                        RoomObjectRef::Player(name) => {
                            let reactions = if name == body.get_name() {
                                body.get_string("반응이름")
                            } else {
                                players
                                    .iter()
                                    .find(|player| player.get("name").and_then(|v| v.as_str()) == Some(name.as_str()))
                                    .and_then(|player| player.get("raw_attrs"))
                                    .and_then(|attrs| attrs.get("반응이름"))
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("")
                                    .to_string()
                            };
                            let (exact, prefixes) = classify(&name, &reaction_names(&reactions));
                            (exact || prefixes > 0)
                                .then_some((OrderedValueTarget::Player(name), exact, prefixes))
                        }
                        RoomObjectRef::Mob(id) => mobs.iter().find(|mob| mob.instance_id == id).and_then(|mob| {
                            let data = world.get_mob_data(&mob.mob_key)?;
                            let transparent = mob.runtime_attrs.get("투명상태").is_some_and(|v| matches!(v, Value::Int(1)))
                                || data.attributes.get("투명상태").and_then(serde_json::Value::as_i64) == Some(1);
                            if transparent || matches!(mob.act, 2 | 3) { return None; }
                            let (exact, prefixes) = classify(&data.name, &data.reaction_names);
                            (exact || prefixes > 0)
                                .then_some((OrderedValueTarget::Mob(id), exact, prefixes))
                        }),
                        RoomObjectRef::FloorItem(pointer) => floor
                            .iter()
                            .find(|object| Arc::as_ptr(object) as usize == pointer)
                            .cloned()
                            .and_then(|selected| {
                                let object = selected.lock().ok()?;
                                let aliases = reaction_names(&object.getString("반응이름"));
                                let (exact, prefixes) = classify(&object.getName(), &aliases);
                                (object.getInt("투명상태") != 1 && (exact || prefixes > 0))
                                    .then_some((OrderedValueTarget::Object(selected.clone()), exact, prefixes))
                            }),
                        RoomObjectRef::Box(pointer) => floor
                            .iter()
                            .chain(installed.iter())
                            .find(|object| Arc::as_ptr(object) as usize == pointer)
                            .cloned()
                            .and_then(|selected| {
                                let object = selected.lock().ok()?;
                                let aliases = reaction_names(&object.getString("반응이름"));
                                let (exact, prefixes) = classify(&object.getName(), &aliases);
                                (object.getInt("투명상태") != 1 && (exact || prefixes > 0))
                                    .then_some((OrderedValueTarget::Object(selected.clone()), exact, prefixes))
                            }),
                        RoomObjectRef::InstalledBox(ordinal) => installed
                            .get(ordinal)
                            .cloned()
                            .and_then(|selected| {
                                let object = selected.lock().ok()?;
                                let aliases = reaction_names(&object.getString("반응이름"));
                                let (exact, prefixes) = classify(&object.getName(), &aliases);
                                (object.getInt("투명상태") != 1 && (exact || prefixes > 0))
                                .then_some((OrderedValueTarget::Object(selected.clone()), exact, prefixes))
                            }),
                        RoomObjectRef::SummonedUser(id) => world
                            .summoned_users()
                            .iter()
                            .find(|user| user.id == id)
                            .and_then(|user| {
                                if user.body.get_int("투명상태") == 1 { return None; }
                                let aliases = reaction_names(&user.body.get_string("반응이름"));
                                let (exact, prefixes) = classify(&user.body.get_name(), &aliases);
                                (exact || prefixes > 0)
                                    .then_some((OrderedValueTarget::Summoned(id), exact, prefixes))
                            }),
                        RoomObjectRef::Fixture(id) => world.get_fixture(id).and_then(|fixture| {
                            let (exact, prefixes) = fixture.match_counts(query);
                            (exact || prefixes > 0)
                                .then_some((OrderedValueTarget::Fixture(id), exact, prefixes))
                        }),
                      };
                      let (selected, exact, prefixes) = candidate?;
                      if exact {
                          exact_count += 1;
                          (exact_count == order).then_some(selected)
                      } else {
                          let previous = prefix_count;
                          prefix_count += prefixes;
                          (previous < order && order <= prefix_count).then_some(selected)
                      }
                    })
            });

            // Python Room.findObjName sees players as room objects. The executing
            // player can therefore be selected by name as well.
            if matches!(ordered_target.as_ref(), Some(OrderedValueTarget::Player(name)) if name == &body.get_name())
                || (ordered_target.is_none() && target == body.get_name())
            {
                let value = match python_coerce_attribute(body.object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                body.set(key, value);
                return "ok".into();
            }

            if let Some(OrderedValueTarget::Summoned(id)) = ordered_target.as_ref() {
                let mut world = get_world_state().write().unwrap();
                let Some(user) = world.summoned_user_mut(*id) else {
                    return "missing".into();
                };
                let value = match python_coerce_attribute(user.body.object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                user.body.set(key, value);
                return "ok".into();
            }

            if let Some(OrderedValueTarget::Fixture(id)) = ordered_target.as_ref() {
                let mut world = get_world_state().write().unwrap();
                let Some(fixture) = world.get_fixture_mut(*id) else {
                    return "missing".into();
                };
                let value = match fixture.attribute(key) {
                    Some(serde_json::Value::Number(number)) if number.is_i64() => raw
                        .parse::<i64>()
                        .map(|value| serde_json::Value::Number(value.into()))
                        .map_err(|_| ()),
                    Some(serde_json::Value::Number(_)) => raw
                        .parse::<f64>()
                        .ok()
                        .and_then(serde_json::Number::from_f64)
                        .map(serde_json::Value::Number)
                        .ok_or(()),
                    Some(serde_json::Value::Bool(_)) => match raw {
                        "true" | "True" | "1" => Ok(serde_json::Value::Bool(true)),
                        "false" | "False" | "0" => Ok(serde_json::Value::Bool(false)),
                        _ => Err(()),
                    },
                    _ => Ok(serde_json::Value::String(raw.to_string())),
                };
                let Ok(value) = value else {
                    return "invalid".into();
                };
                fixture.set_attribute(key, value);
                return "ok".into();
            }

            let room_objects = get_world_state()
                .read()
                .ok()
                .map(|world| world.get_room_objs(&zone, &room).to_vec())
                .unwrap_or_default();
            if let Some(OrderedValueTarget::Object(selected)) = ordered_target.as_ref() {
                let Ok(mut object) = selected.lock() else {
                    return "missing".into();
                };
                let value = match python_coerce_attribute(object.attr.get(key), raw) {
                    Ok(value) => value,
                    Err(()) => return "invalid".into(),
                };
                object.set(key, value);
                return "ok".into();
            }
            for object in room_objects {
                if ordered_target.is_some() {
                    continue;
                }
                let Ok(mut object) = object.lock() else {
                    continue;
                };
                let aliases = reaction_names(&object.getString("반응이름"));
                if object.getName() != target
                    && !aliases
                        .iter()
                        .any(|alias| alias == target || alias.starts_with(target))
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

            let live_players = room_admin_player_values(body).unwrap_or_default();
            if let Some(player) = live_players
                .iter()
                .find(|player| {
                    let name = player.get("name").and_then(|value| value.as_str());
                    if let Some(OrderedValueTarget::Player(selected)) = ordered_target.as_ref() {
                        name == Some(selected.as_str())
                    } else if ordered_target.is_none() {
                        name == Some(target)
                    } else {
                        false
                    }
                })
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
                        serde_json::to_string(&(target.to_string(), key.to_string(), json_value))
                            .unwrap_or_default(),
                    ),
                );
                return "ok".into();
            }

            let mob_id = if let Some(OrderedValueTarget::Mob(id)) = ordered_target.as_ref() {
                Some(*id)
            } else if ordered_target.is_some() {
                None
            } else { get_world_state().read().ok().and_then(|world| {
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
            })};
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
            if let Some(value) = get_world_state().read().ok().and_then(|w| {
                w.room_attrs
                    .get(&format!("{}:{}", pos.zone, pos.room))
                    .and_then(|m| m.get(key))
                    .cloned()
            }) {
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
                if key == "이름" {
                    if let Some(cached) = get_world_state()
                        .read()
                        .ok()
                        .and_then(|world| world.room_cache.get_room_cached(&pos.zone, &pos.room))
                    {
                        if let Ok(mut cached) = cached.write() {
                            cached.name = val_str.clone();
                        }
                    }
                }
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
            let inventory_object = body.object.objs.iter().find_map(|arc| {
                let object = arc.lock().ok()?;
                (object.getName() == target
                    || (!object.getString("반응이름").is_empty()
                        && object.getString("반응이름").contains(target)))
                .then(|| arc.clone())
            });
            if let Some(object_arc) = inventory_object {
                let changed = if let Ok(mut object) = object_arc.lock() {
                    object.set(key, v.clone());
                    true
                } else {
                    false
                };
                restore_pristine_inventory_object(body, &object_arc);
                return changed;
            }
            if let Some(stack_key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, target)
            {
                let Some(object_arc) =
                    inventory_compat::materialize_one(&mut body.object, &stack_key, true)
                else {
                    return false;
                };
                let changed = if let Ok(mut object) = object_arc.lock() {
                    object.set(key, v.clone());
                    true
                } else {
                    false
                };
                restore_pristine_inventory_object(body, &object_arc);
                if changed {
                    return true;
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
                return if attrs.remove(key).is_some() {
                    "ok"
                } else {
                    "no_key"
                }
                .into();
            }
            let Some((zone, room)) = current_body_position(body) else {
                return "missing".into();
            };
            let Some(selected) = select_python_room_object(body, target) else {
                return "missing".into();
            };
            let removed = match selected {
                RoomObjectRef::Player(name) if name == my_name => {
                    body.attr_mut().remove(key).is_some()
                }
                RoomObjectRef::Player(name) => {
                    if !matches!(online_player_raw_attr(body, &name, key), Some(Some(_))) {
                        return "no_key".into();
                    }
                    body.temp_mut().insert(
                        ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
                        Value::String(
                            serde_json::to_string(&(name, key, serde_json::Value::Null))
                                .unwrap_or_default(),
                        ),
                    );
                    true
                }
                RoomObjectRef::SummonedUser(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        world
                            .summoned_user_mut(id)
                            .map(|user| user.body.attr_mut().remove(key).is_some())
                    })
                    .unwrap_or(false),
                RoomObjectRef::Mob(id) => {
                    let mut world = get_world_state().write().unwrap();
                    world
                        .mob_cache
                        .get_all_mobs_in_room_mut(&zone, &room)
                        .and_then(|mobs| mobs.iter_mut().find(|mob| mob.instance_id == id))
                        .is_some_and(|mob| mob.runtime_attrs.remove(key).is_some())
                }
                RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                    let floor = get_world_state()
                        .read()
                        .ok()
                        .map(|world| world.get_room_objs(&zone, &room))
                        .unwrap_or_default();
                    let installed =
                        box_commands::installed_boxes_for_room(&zone, &room).unwrap_or_default();
                    floor
                        .into_iter()
                        .chain(installed)
                        .find(|object| Arc::as_ptr(object) as usize == pointer)
                        .and_then(|object| {
                            object
                                .lock()
                                .ok()
                                .map(|mut object| object.attr.remove(key).is_some())
                        })
                        .unwrap_or(false)
                }
                RoomObjectRef::InstalledBox(ordinal) => {
                    box_commands::installed_boxes_for_room(&zone, &room)
                        .and_then(|boxes| boxes.get(ordinal).cloned())
                        .and_then(|object| {
                            object
                                .lock()
                                .ok()
                                .map(|mut object| object.attr.remove(key).is_some())
                        })
                        .unwrap_or(false)
                }
                RoomObjectRef::Fixture(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        world
                            .get_fixture_mut(id)
                            .map(|fixture| fixture.attributes.remove(key).is_some())
                    })
                    .unwrap_or(false),
            };
            if removed {
                "ok".into()
            } else {
                "no_key".into()
            }
        },
    );

    // Python 속성추가: Room.findObjName으로 선택한 객체의 새 속성을 만들거나
    // 기존 문자열 뒤에 CRLF와 값을 덧붙인다.
    let body_ptr_aoav = body_ptr;
    engine.register_fn(
        "append_obj_attr_value",
        move |_ob: &mut rhai::Map, target: &str, key: &str, wanted: &str| -> String {
            let body = unsafe { &mut *body_ptr_aoav };
            let append_to = |attr: &mut HashMap<String, Value>| -> String {
                match attr.get(key).cloned() {
                    None => {
                        let value = wanted
                            .parse::<i64>()
                            .map(Value::Int)
                            .unwrap_or_else(|_| Value::String(wanted.to_string()));
                        attr.insert(key.to_string(), value);
                        "ok".into()
                    }
                    Some(Value::String(current)) => {
                        attr.insert(
                            key.to_string(),
                            Value::String(format!("{current}\r\n{wanted}")),
                        );
                        "ok".into()
                    }
                    Some(_) => "failed".into(),
                }
            };
            let Some((zone, room)) = current_body_position(body) else {
                return "no_target".into();
            };
            let Some(selected) = select_python_room_object(body, target) else {
                return "no_target".into();
            };
            match selected {
                RoomObjectRef::Player(player) if player == body.get_name() => {
                    append_to(body.attr_mut())
                }
                RoomObjectRef::Player(player) => {
                    let Some(existing) = online_player_raw_attr(body, &player, key) else {
                        return "no_target".into();
                    };
                    let value = match existing {
                        None => wanted
                            .parse::<i64>()
                            .map(|value| serde_json::Value::Number(value.into()))
                            .unwrap_or_else(|_| serde_json::Value::String(wanted.to_string())),
                        Some(serde_json::Value::String(current)) => {
                            serde_json::Value::String(format!("{current}\r\n{wanted}"))
                        }
                        Some(_) => return "failed".into(),
                    };
                    queue_admin_player_json_value(body, &player, key, value);
                    "ok".into()
                }
                RoomObjectRef::SummonedUser(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        world
                            .summoned_user_mut(id)
                            .map(|user| append_to(user.body.attr_mut()))
                    })
                    .unwrap_or_else(|| "no_target".into()),
                RoomObjectRef::Mob(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        world
                            .mob_cache
                            .get_all_mobs_in_room_mut(&zone, &room)
                            .and_then(|mobs| mobs.iter_mut().find(|mob| mob.instance_id == id))
                            .map(|mob| append_to(&mut mob.runtime_attrs))
                    })
                    .unwrap_or_else(|| "no_target".into()),
                RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                    let floor = get_world_state()
                        .read()
                        .ok()
                        .map(|world| world.get_room_objs(&zone, &room))
                        .unwrap_or_default();
                    let installed =
                        box_commands::installed_boxes_for_room(&zone, &room).unwrap_or_default();
                    floor
                        .into_iter()
                        .chain(installed)
                        .find(|object| Arc::as_ptr(object) as usize == pointer)
                        .and_then(|object| {
                            object
                                .lock()
                                .ok()
                                .map(|mut object| append_to(&mut object.attr))
                        })
                        .unwrap_or_else(|| "no_target".into())
                }
                RoomObjectRef::InstalledBox(ordinal) => {
                    box_commands::installed_boxes_for_room(&zone, &room)
                        .and_then(|boxes| boxes.get(ordinal).cloned())
                        .and_then(|object| {
                            object
                                .lock()
                                .ok()
                                .map(|mut object| append_to(&mut object.attr))
                        })
                        .unwrap_or_else(|| "no_target".into())
                }
                RoomObjectRef::Fixture(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        let fixture = world.get_fixture_mut(id)?;
                        let result = match fixture.attribute(key).cloned() {
                            None => {
                                let value = wanted
                                    .parse::<i64>()
                                    .map(|value| serde_json::Value::Number(value.into()))
                                    .unwrap_or_else(|_| {
                                        serde_json::Value::String(wanted.to_string())
                                    });
                                fixture.set_attribute(key, value);
                                "ok"
                            }
                            Some(serde_json::Value::String(current)) => {
                                fixture.set_attribute(
                                    key,
                                    serde_json::Value::String(format!("{current}\r\n{wanted}")),
                                );
                                "ok"
                            }
                            Some(_) => "failed",
                        };
                        Some(result.to_string())
                    })
                    .unwrap_or_else(|| "no_target".into()),
            }
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
            // Python 속성제거는 ob.env.findObjName()만 호출한다. 방/자기 자신을
            // 특별 취급하거나 소지품으로 fallback하지 않는다.
            let Some((zone, room)) = current_body_position(body) else {
                return "no_target".into();
            };
            let Some(selected) = select_python_room_object(body, target) else {
                return "no_target".into();
            };
            match selected {
                RoomObjectRef::Player(player) if player == name => remove_from(body.attr_mut()),
                RoomObjectRef::Player(player) => {
                    let Some(existing) = online_player_raw_attr(body, &player, key) else {
                        return "no_target".into();
                    };
                    let raw = match existing {
                        Some(serde_json::Value::String(raw)) => raw,
                        Some(_) => return "not_value".into(),
                        None => return "no_key".into(),
                    };
                    let values = raw.split("\r\n").collect::<Vec<_>>();
                    if !values.contains(&wanted) {
                        return "not_value".into();
                    }
                    let kept = values
                        .into_iter()
                        .filter(|value| *value != wanted)
                        .collect::<Vec<_>>();
                    let value = if kept.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(kept.join("\r\n"))
                    };
                    queue_admin_player_json_value(body, &player, key, value);
                    "ok".into()
                }
                RoomObjectRef::SummonedUser(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        world
                            .summoned_user_mut(id)
                            .map(|user| remove_from(user.body.attr_mut()))
                    })
                    .unwrap_or_else(|| "no_target".into()),
                RoomObjectRef::Mob(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        world
                            .mob_cache
                            .get_all_mobs_in_room_mut(&zone, &room)
                            .and_then(|mobs| mobs.iter_mut().find(|mob| mob.instance_id == id))
                            .map(|mob| remove_from(&mut mob.runtime_attrs))
                    })
                    .unwrap_or_else(|| "no_target".into()),
                RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                    let floor = get_world_state()
                        .read()
                        .ok()
                        .map(|world| world.get_room_objs(&zone, &room))
                        .unwrap_or_default();
                    let installed =
                        box_commands::installed_boxes_for_room(&zone, &room).unwrap_or_default();
                    floor
                        .into_iter()
                        .chain(installed)
                        .find(|object| Arc::as_ptr(object) as usize == pointer)
                        .and_then(|object| {
                            object
                                .lock()
                                .ok()
                                .map(|mut object| remove_from(&mut object.attr))
                        })
                        .unwrap_or_else(|| "no_target".into())
                }
                RoomObjectRef::InstalledBox(ordinal) => {
                    box_commands::installed_boxes_for_room(&zone, &room)
                        .and_then(|boxes| boxes.get(ordinal).cloned())
                        .and_then(|object| {
                            object
                                .lock()
                                .ok()
                                .map(|mut object| remove_from(&mut object.attr))
                        })
                        .unwrap_or_else(|| "no_target".into())
                }
                RoomObjectRef::Fixture(id) => get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| {
                        let fixture = world.get_fixture_mut(id)?;
                        let Some(serde_json::Value::String(raw)) = fixture.attribute(key).cloned()
                        else {
                            return Some(if fixture.attribute(key).is_some() {
                                "not_value".to_string()
                            } else {
                                "no_key".to_string()
                            });
                        };
                        let values = raw.split("\r\n").collect::<Vec<_>>();
                        if !values.contains(&wanted) {
                            return Some("not_value".to_string());
                        }
                        let kept = values
                            .into_iter()
                            .filter(|value| *value != wanted)
                            .collect::<Vec<_>>();
                        if kept.is_empty() {
                            fixture.attributes.remove(key);
                        } else {
                            fixture
                                .set_attribute(key, serde_json::Value::String(kept.join("\r\n")));
                        }
                        Some("ok".to_string())
                    })
                    .unwrap_or_else(|| "no_target".into()),
            }
        },
    );

    // Python 출구숨김은 같은 명령으로 숨김/드러냄을 토글하고 방 파일을 저장한다.
    engine.register_fn(
        "exit_hide",
        move |ob: &mut rhai::Map, name: &str| -> String {
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
                let mut status = "missing";
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
                        destination.trim_start()
                    );
                    status = if hidden { "shown" } else { "hidden" };
                }
                status.to_string()
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
        },
    );

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
            let mut w = match get_world_state().write() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let room_arc = match w.room_cache.get_room(&zone, &room) {
                Ok(r) => r,
                Err(_) => return false,
            };
            // Python changes only the live Room attribute and calls init();
            // it does not save the map JSON. The parsed Rust Exit is the live
            // equivalent, so never rewrite the source file here.
            let updated = room_arc
                .write()
                .unwrap()
                .set_exit_destination(name, &zone, &room);
            updated
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
    engine.register_fn("guild_has_attr", |id: &str, key: &str| -> bool {
        guild_attr_keys(id).iter().any(|saved| saved == key)
    });
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
                        value
                            .as_int()
                            .ok()
                            .or_else(|| value.to_string().parse::<i64>().ok())
                    })
                    .unwrap_or(0);
                (value != 0).then_some((index, name, value))
            })
            .collect();
        entries.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0)));
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
            let raw_query = if input.trim() == "." { "1" } else { input };
            // Room.findObjName splits a multi-word argument and keeps only
            // the first token before applying numeric/name lookup.
            let query = raw_query.split_whitespace().next().unwrap_or("");
            if query == body.get_name() {
                return finish("self", String::new(), 0, 0);
            }
            let numeric_order = query.parse::<usize>().ok().filter(|order| *order > 0);
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
                body.get_str() * 2 + body.get_max_mp() / 5 + i64::from(body.get_attack_power())
                    - body.get_mastery_diff()
                    - target_arm
                    - target_armor
            };
            enum CompareTarget {
                Invalid,
                Mob(crate::world::MobInstance, crate::world::RawMobData),
                Player(String, rhai::Map),
            }
            let online_players = get_precomputed_all_online()
                .into_iter()
                .filter_map(|value| value.try_cast::<rhai::Map>())
                .filter_map(|map| {
                    let name = map.get("이름")?.clone().into_string().ok()?;
                    let same_room = map.get("zone")?.to_string() == zone
                        && map.get("room")?.to_string() == room;
                    same_room.then_some((name, map))
                })
                .collect::<Vec<_>>();
            let player_matches = |name: &str, map: &rhai::Map| {
                let reactions = map
                    .get("반응이름")
                    .map(Dynamic::to_string)
                    .unwrap_or_default();
                name == query
                    || reactions.contains(query)
                    || reaction_names(&reactions)
                        .iter()
                        .any(|alias| alias.starts_with(query))
            };
            let selected = get_world_state().read().ok().and_then(|world| {
                let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
                if let Some(selected) = select_python_room_object(body, input) {
                    let resolved = match selected {
                        RoomObjectRef::Mob(id) => mobs
                            .iter()
                            .find(|mob| mob.instance_id == id)
                            .and_then(|mob| {
                                world
                                    .get_mob_data(&mob.mob_key)
                                    .cloned()
                                    .map(|data| CompareTarget::Mob((*mob).clone(), data))
                            }),
                        RoomObjectRef::Player(name) => online_players
                            .iter()
                            .find(|(candidate, _)| candidate == &name)
                            .map(|(name, map)| CompareTarget::Player(name.clone(), map.clone())),
                        RoomObjectRef::FloorItem(_)
                        | RoomObjectRef::Box(_)
                        | RoomObjectRef::InstalledBox(_)
                        | RoomObjectRef::SummonedUser(_)
                        | RoomObjectRef::Fixture(_) => Some(CompareTarget::Invalid),
                    };
                    if resolved.is_some() {
                        return resolved;
                    }
                }
                if let Some(order) = numeric_order {
                    let ordered = world.get_room_object_order(&zone, &room);
                    let ordered_ids = ordered
                        .iter()
                        .filter_map(|object| match object {
                            RoomObjectRef::Mob(id) => Some(*id),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    let found = ordered
                        .iter()
                        .filter_map(|object| match object {
                            RoomObjectRef::Mob(id) => mobs
                                .iter()
                                .find(|mob| mob.instance_id == *id)
                                .map(|mob| (*mob).clone()),
                            _ => None,
                        })
                        .chain(
                            mobs.iter()
                                .filter(|mob| !ordered_ids.contains(&mob.instance_id))
                                .map(|mob| (*mob).clone()),
                        )
                        .filter(|mob| {
                            mob.alive
                                && world
                                    .get_mob_data(&mob.mob_key)
                                    .is_some_and(|data| data.mob_type != 7)
                        })
                        .nth(order - 1);
                    return found.and_then(|mob| {
                        world
                            .get_mob_data(&mob.mob_key)
                            .cloned()
                            .map(|data| CompareTarget::Mob(mob, data))
                    });
                }
                let floor = world.get_room_objs(&zone, &room);
                let ordered = world.get_room_object_order(&zone, &room);
                let selected = ordered.into_iter().find_map(|object| match object {
                    RoomObjectRef::FloorItem(pointer) => floor
                        .iter()
                        .find(|item| Arc::as_ptr(item) as usize == pointer)
                        .and_then(|item| item.lock().ok())
                        .and_then(|item| {
                            let aliases = item.getString("반응이름");
                            (item.getName() == query
                                || aliases.contains(query)
                                || reaction_names(&aliases)
                                    .iter()
                                    .any(|alias| alias.starts_with(query)))
                            .then_some(CompareTarget::Invalid)
                        }),
                    RoomObjectRef::Mob(id) => mobs
                        .iter()
                        .find(|mob| mob.instance_id == id)
                        .and_then(|mob| {
                            let data = world.get_mob_data(&mob.mob_key)?;
                            if data.attributes.get("투명상태").and_then(|v| v.as_i64()) == Some(1)
                                || (!mob.alive && query != "시체")
                            {
                                return None;
                            }
                            (mob.name == query
                                || data.name == query
                                || data
                                    .reaction_names
                                    .iter()
                                    .any(|alias| alias == query || alias.starts_with(query)))
                            .then(|| CompareTarget::Mob((*mob).clone(), data.clone()))
                        }),
                    RoomObjectRef::Player(name) => online_players
                        .iter()
                        .find(|(candidate, _)| candidate == &name)
                        .and_then(|(name, map)| {
                            player_matches(name, map)
                                .then(|| CompareTarget::Player(name.clone(), map.clone()))
                        }),
                    _ => None,
                });
                selected.or_else(|| {
                    mobs.iter().find_map(|mob| {
                        let data = world.get_mob_data(&mob.mob_key)?;
                        if data.attributes.get("투명상태").and_then(|v| v.as_i64()) == Some(1)
                            || (!mob.alive && query != "시체")
                        {
                            return None;
                        }
                        (mob.name == query
                            || data.name == query
                            || data
                                .reaction_names
                                .iter()
                                .any(|alias| alias == query || alias.starts_with(query)))
                        .then(|| CompareTarget::Mob((*mob).clone(), data.clone()))
                    })
                })
            });
            let Some(selected) = selected else {
                return finish("invalid", String::new(), 0, 0);
            };
            if matches!(selected, CompareTarget::Invalid) {
                return finish("invalid", String::new(), 0, 0);
            }
            if let CompareTarget::Mob(mob, data) = selected {
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
                        let Some(info) = info else {
                            return (attack, armor);
                        };
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
                    mob.strength * 2 + mob_attack - body.get_arm() - i64::from(body.get_armor()),
                );
                return finish(
                    "ok",
                    mob.name,
                    body.get_max_hp() / mob_damage,
                    // Python 비교.py reads obj['체력']; Mob keeps that
                    // configured maximum separate from the mutable obj.hp.
                    // An already injured opponent therefore has the same
                    // comparison estimate as a freshly spawned one.
                    data.max_hp / my_damage,
                );
            }
            let CompareTarget::Player(target_name, target) = selected else {
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
            let value = serde_json::Value::Object(attrs);
            let mut bytes = Vec::new();
            let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
            let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
            if serde::Serialize::serialize(&value, &mut serializer).is_err() {
                return String::new();
            }
            python_json_ensure_ascii(&String::from_utf8(bytes).unwrap_or_default())
        },
    );
    engine.register_fn("console_print_twice", |text: &str| -> bool {
        println!("{text}");
        println!("{text}");
        true
    });

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
    let raw_user_sends = user_sends.clone();
    engine.register_fn("send_raw_to_user", move |name: &str, msg: &str| {
        if !name.is_empty() && !msg.is_empty() {
            if let Ok(mut sends) = raw_user_sends.lock() {
                sends.push((
                    name.to_string(),
                    format!("{}{}", RAW_USER_MESSAGE_PREFIX, msg),
                ));
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
            if action.is_empty() {
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
            if amount == 0 {
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
    let body_ptr_grant_self_silver = body_ptr;
    engine.register_fn(
        "grant_self_silver",
        move |_ob: &mut rhai::Map, amount: i64| -> i64 {
            let body = unsafe { &mut *body_ptr_grant_self_silver };
            let total = body.get_int("은전").saturating_add(amount);
            body.set("은전", total);
            total
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
            let cnt_u = cnt as usize;
            let individual = body
                .object
                .objs
                .iter()
                .filter(|item| {
                    item.lock().is_ok_and(|item| {
                        !item.getBool("inUse")
                            && !inventory_compat::python_item_field_contains(
                                &item,
                                "아이템속성",
                                "출력안함",
                            )
                            && (item.getName() == item_name
                                || inventory_compat::python_item_field_contains(
                                    &item,
                                    "반응이름",
                                    item_name,
                                ))
                    })
                })
                .count();
            let object_count = if order <= individual {
                cnt_u.min(individual.saturating_sub(order - 1))
            } else {
                0
            };
            let stack_needed = cnt.saturating_sub(object_count as i64);
            let stack_order = if order > individual {
                order.saturating_sub(individual) as i64
            } else {
                1
            };
            let give_stack = if stack_needed > 0 {
                inventory_compat::counted_item_at(&body.object.inv_stack, item_name, stack_order)
                    .and_then(|(key, offset)| {
                        let have = body.object.inv_stack.get(&key).copied().unwrap_or(0);
                        let available = have.saturating_sub(offset - 1);
                        let moved = stack_needed.min(available);
                        (moved > 0).then_some((key, moved))
                    })
            } else {
                None
            };
            if object_count == 0 && give_stack.is_none() {
                return "no_item".to_string();
            }
            let requested_objects = if order <= individual && give_stack.is_none() {
                cnt_u
            } else {
                object_count
            };
            if let Ok(mut s) = spec_gi.lock() {
                *s = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: None,
                    give_gold: None,
                    give_item: (requested_objects > 0).then_some((
                        item_name.to_string(),
                        order,
                        requested_objects,
                    )),
                    give_item_stack: give_stack,
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
            let count = if order > 1 {
                1
            } else {
                count.clamp(1, 100) as usize
            };
            let individual = body
                .object
                .objs
                .iter()
                .filter(|item| {
                    item.lock().is_ok_and(|item| {
                        !item.getBool("inUse")
                            && (item.getName() == item_name
                                || inventory_compat::python_item_field_contains(
                                    &item,
                                    "반응이름",
                                    item_name,
                                ))
                    })
                })
                .count();
            let object_count = if order <= individual {
                count.min(individual.saturating_sub(order - 1))
            } else {
                0
            };
            let stack_needed = count.saturating_sub(object_count) as i64;
            let stack_order = if order > individual {
                order.saturating_sub(individual) as i64
            } else {
                1
            };
            let give_stack = if stack_needed > 0 {
                inventory_compat::counted_item_at(&body.object.inv_stack, item_name, stack_order)
                    .and_then(|(key, offset)| {
                        let have = body.object.inv_stack.get(&key).copied().unwrap_or(0);
                        let available = have.saturating_sub(offset - 1);
                        let moved = stack_needed.min(available);
                        (moved > 0).then_some((key, moved))
                    })
            } else {
                None
            };
            if object_count == 0 && give_stack.is_none() {
                return "no_item".into();
            }
            let requested_objects = if order <= individual && give_stack.is_none() {
                count
            } else {
                object_count
            };
            if let Ok(mut result) = spec_admin_give_item.lock() {
                *result = Some(CommandResult::GiveToPlayer {
                    target_name: target.to_string(),
                    giver_name: body.get_name(),
                    give_silver: None,
                    give_gold: None,
                    give_item: (requested_objects > 0).then_some((
                        item_name.to_string(),
                        order,
                        requested_objects,
                    )),
                    give_item_stack: give_stack,
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
                // Python Object.findObjName() searches the raw 반응이름
                // string with `name in ...` and never exposes 출력안함
                // objects as the initially selected item.  Keep that odd
                // substring behavior here because 줘/줘줘 derive the
                // canonical name and particle from this first lookup.
                if (item.getName() != name
                    && !inventory_compat::python_item_field_contains(&item, "반응이름", name))
                    || inventory_compat::python_item_field_contains(&item, "아이템속성", "출력안함")
                {
                    continue;
                }
                matched += 1;
                if matched < order.max(1) {
                    continue;
                }
                let actual = item.getName();
                result.insert("found".into(), Dynamic::from(true));
                result.insert("name".into(), Dynamic::from(actual.clone()));
                result.insert(
                    "post".into(),
                    Dynamic::from(format!("{}{}", actual, han_eul(&actual))),
                );
                return result;
            }
            let remaining = order.max(1).saturating_sub(matched);
            if let Some((key, _)) =
                inventory_compat::counted_item_at(&body.object.inv_stack, name, remaining)
            {
                if remaining > 0 {
                    if let Some((item, _)) = object_from_item_json(&key) {
                        if let Ok(item) = item.lock() {
                            let actual = item.getName();
                            result.insert("found".into(), Dynamic::from(true));
                            result.insert("name".into(), Dynamic::from(actual.clone()));
                            result.insert(
                                "post".into(),
                                Dynamic::from(format!("{}{}", actual, han_eul(&actual))),
                            );
                            return result;
                        }
                    }
                }
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
            let wanted_order = order.max(1) as usize;
            let item = body.object.findObjInven(name, wanted_order).or_else(|| {
                let individual = body
                    .object
                    .objs
                    .iter()
                    .filter(|item| {
                        item.lock().is_ok_and(|item| {
                            !item.getBool("inUse")
                                && (item.getName() == name
                                    || item.getString("반응이름").contains(name))
                        })
                    })
                    .count();
                let remaining = wanted_order.saturating_sub(individual) as i64;
                let (key, _) =
                    inventory_compat::counted_item_at(&body.object.inv_stack, name, remaining)?;
                object_from_item_json(&key).map(|(item, _)| item)
            });
            let Some(item) = item else {
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
                Dynamic::from(item.getString("사용스크립").replace("$아이템$", &item_name)),
            );
            result.insert(
                "post".into(),
                Dynamic::from(format!("{name_a}{}", han_eul(&item_name))),
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
            let mut stack_key = None;
            let arc = match body.object.findObjInven(name, order) {
                Some(a) => a,
                None => {
                    let individual = body
                        .object
                        .objs
                        .iter()
                        .filter(|item| {
                            item.lock().is_ok_and(|item| {
                                !item.getBool("inUse")
                                    && (item.getName() == name
                                        || item.getString("반응이름").contains(name))
                            })
                        })
                        .count();
                    let remaining = order.saturating_sub(individual) as i64;
                    let Some((key, _)) =
                        inventory_compat::counted_item_at(&body.object.inv_stack, name, remaining)
                    else {
                        return "no_item".to_string();
                    };
                    let Some((item, _)) = object_from_item_json(&key) else {
                        return "no_item".to_string();
                    };
                    stack_key = Some(key);
                    item
                }
            };
            // 아이템의 모든 속성 수집
            let (kind, slot, stats, mastery_required) = {
                let o = arc.lock().unwrap();
                let k = o.getString("종류");
                let s = o.getString("계층");
                if k != "방어구" && k != "무기" {
                    return "not_equippable".to_string();
                }
                let stats = equipment_stats(&o);
                let mastery_required = o.checkAttr("아이템속성", "올숙이천무기");
                (k, s, stats, mastery_required)
            };
            if mastery_required
                && (1..=5).any(|weapon| body.get_int(&format!("{weapon} 숙련도")) < 2000)
            {
                return "mastery_required".to_string();
            }
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
            if let Some(key) = stack_key {
                if !inventory_compat::remove_pristine_count(&mut body.object, &key, 1) {
                    return "no_item".to_string();
                }
                body.object.objs.insert(0, arc.clone());
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
            let stack_keys = body
                .object
                .inv_stack
                .iter()
                .filter(|(_, count)| **count > 0)
                .filter_map(|(key, _)| {
                    let (item, _) = object_from_item_json(key)?;
                    let equippable = item.lock().ok().is_some_and(|item| {
                        matches!(item.getString("종류").as_str(), "무기" | "방어구")
                    });
                    equippable.then_some(key.clone())
                })
                .collect::<Vec<_>>();
            let mut materialized = Vec::new();
            for key in stack_keys {
                if let Some(item) = inventory_compat::materialize_one(&mut body.object, &key, false)
                {
                    materialized.push(item);
                }
            }
            let inventory = body.object.objs.clone();
            let mut equipped = rhai::Array::new();
            for arc in inventory {
                let (kind, slot, stats, name, post, script, mastery_required) = {
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
                        {
                            let name = item.getName();
                            let name_a = if item.getString("안시").is_empty() {
                                format!("\x1b[0;36m{name}\x1b[37m")
                            } else {
                                format!("{}{name}\x1b[0;37m", item.getString("안시"))
                            };
                            format!("{name_a}{}", han_eul(&name))
                        },
                        item.getString("사용스크립")
                            .replace("$아이템$", &item.getName()),
                        required,
                    )
                };
                if mastery_required > 0
                    && (1..=5)
                        .any(|weapon| body.get_int(&format!("{weapon} 숙련도")) < mastery_required)
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
                event.insert("post".into(), Dynamic::from(post));
                event.insert("script".into(), Dynamic::from(script));
                equipped.push(Dynamic::from(event));
            }
            for item in materialized {
                let unused = item.lock().ok().is_some_and(|item| !item.getBool("inUse"));
                if unused && inventory_compat::absorb_pristine_object(&mut body.object, &item) {
                    body.object.remove(&item);
                }
            }
            equipped
        },
    );

    // item_unequip(ob, name, order): 상태 변경과 실제 아이템 표시형만 반환한다.
    // 아이템 해제 시 모든 속성 보너스 제거
    let body_ptr_ue = body_ptr;
    engine.register_fn(
        "item_unequip",
        move |_ob: &mut rhai::Map, name: &str, order: i64| -> rhai::Map {
            let mut result = rhai::Map::new();
            if name.is_empty() {
                result.insert("err".into(), Dynamic::from("usage"));
                result.insert("post".into(), Dynamic::from(""));
                return result;
            }
            let order = order.max(1) as usize;
            let body = unsafe { &mut *body_ptr_ue };
            let arc = match body.object.findObjInUse(name, order) {
                Some(a) => a,
                None => {
                    result.insert("err".into(), Dynamic::from("no_item"));
                    result.insert("post".into(), Dynamic::from(""));
                    return result;
                }
            };
            // 아이템의 모든 속성 수집 및 해제 처리
            let (is_weapon, stats, post) = {
                let mut o = arc.lock().unwrap();
                o.attr.remove("inUse");
                let w = o.getString("종류") == "무기";
                let stats = equipment_stats(&o);
                let post = item_han_obj(&o);
                (w, stats, post)
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
            if inventory_compat::absorb_pristine_object(&mut body.object, &arc) {
                body.object.remove(&arc);
            }
            result.insert("err".into(), Dynamic::from(""));
            result.insert("post".into(), Dynamic::from(post));
            result
        },
    );

    // item_unequip_all(ob): Python inventory order names for Rhai output,
    // while Body owns only the equipment state rollback.
    let body_ptr_ua = body_ptr;
    engine.register_fn(
        "item_unequip_all",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &mut *body_ptr_ua };
            let items = body
                .object
                .objs
                .iter()
                .filter_map(|item| {
                    let item = item.lock().ok()?;
                    item.getBool("inUse").then(|| {
                        let mut event = rhai::Map::new();
                        event.insert("post".into(), Dynamic::from(item_han_obj(&item)));
                        Dynamic::from(event)
                    })
                })
                .collect::<rhai::Array>();
            body.unwear_all();
            for item in body.object.objs.clone() {
                if inventory_compat::absorb_pristine_object(&mut body.object, &item) {
                    body.object.remove(&item);
                }
            }
            items
        },
    );

    // item_use_consumable(ob, name, order): 소비성 아이템 사용.
    // 상태 객체를 먼저 세고, 그 뒤의 원본 동일 아이템은 수량에서 직접 차감한다.
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

            let order = order.max(1) as usize;
            let object_item = body.object.findObjInven(name, order);
            let individual = body
                .object
                .objs
                .iter()
                .filter(|item| {
                    item.lock().is_ok_and(|item| {
                        !item.getBool("inUse")
                            && (item.getName() == name || item.getString("반응이름").contains(name))
                    })
                })
                .count();

            // 상태 객체 뒤에서 선택된 원본 동일 소비품은 객체화하지 않는다.
            if object_item.is_none() {
                if let Some((key, _)) = inventory_compat::counted_item_at(
                    &body.object.inv_stack,
                    name,
                    order.saturating_sub(individual) as i64,
                ) {
                    if is_stackable(&key) {
                        let have = *body.object.inv_stack.get(&key).unwrap_or(&0);
                        if have > 0 {
                            // A Python inventory always contains the actual Item,
                            // so 종류—not nonzero healing—decides whether it can
                            // be eaten. This matters for 백사주 and other food
                            // whose only effect is 내공증진.
                            let Some((template, _)) = object_from_item_json(&key) else {
                                m.insert("err".into(), Dynamic::from("not_consumable".to_string()));
                                m.insert("name".into(), Dynamic::from(String::new()));
                                return Dynamic::from(m);
                            };
                            let (item_name, hp, mp, script, ansi, mut max_mp_gain, continuous) = {
                                let item = template.lock().unwrap();
                                if item.getString("종류") != "먹는것" {
                                    m.insert(
                                        "err".into(),
                                        Dynamic::from("not_consumable".to_string()),
                                    );
                                    m.insert("name".into(), Dynamic::from(String::new()));
                                    return Dynamic::from(m);
                                }
                                (
                                    item.getName(),
                                    item.getInt("체력"),
                                    item.getInt("내공"),
                                    item.getString("사용스크립"),
                                    item.getString("안시"),
                                    item.getInt("내공증진"),
                                    item.checkAttr("아이템속성", "내공계속증진"),
                                )
                            };

                            // HP/MP 회복 적용
                            let max_hp = body.get_max_hp();
                            let max_mp = body.get_max_mp();
                            let cur_hp = body.get_hp();
                            let cur_mp = body.get_mp();
                            let new_hp = (cur_hp + hp).min(max_hp);
                            let new_mp = (cur_mp + mp).min(max_mp);
                            body.set("체력", new_hp);
                            body.set("내공", new_mp);
                            if max_mp_gain != 0 {
                                if !continuous {
                                    if body.object.checkAttr("내공증진아이템리스트", &item_name)
                                    {
                                        max_mp_gain = 0;
                                    } else {
                                        body.object.setAttr("내공증진아이템리스트", &item_name);
                                        body.set(
                                            "최고내공",
                                            body.get_int("최고내공") + max_mp_gain,
                                        );
                                    }
                                } else {
                                    body.set("최고내공", body.get_int("최고내공") + max_mp_gain);
                                }
                            }

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
                            m.insert("name".into(), Dynamic::from(item_name.clone()));
                            m.insert("hp".into(), Dynamic::from(hp));
                            m.insert("script".into(), Dynamic::from(script));
                            m.insert("ansi".into(), Dynamic::from(ansi));
                            m.insert("max_mp_gain".into(), Dynamic::from(max_mp_gain));
                            m.insert(
                                "remaining".into(),
                                Dynamic::from(inventory_exact_name_count(body, &item_name)),
                            );
                            return Dynamic::from(m);
                        }
                    }
                }
            }

            // 상태가 다른 개별 아이템은 해당 객체만 소비한다.
            let arc = match object_item {
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
            let remaining = inventory_exact_name_count(body, &item_name);
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
                return "user_not_found".into();
            }
            let last = player.get_int("마지막저장시간");
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if last > 0 && now.saturating_sub(last) < 259_200 {
                return "too_recent".into();
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
                return "save_failed".into();
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

    let body_ptr_connection_token = body_ptr;
    engine.register_fn(
        "get_connection_token",
        move |_ob: &mut rhai::Map| -> String {
            let body = unsafe { &*body_ptr_connection_token };
            match body.temp().get("_connection_token") {
                Some(Value::String(token)) if !token.is_empty() => token.clone(),
                _ => body.get_name(),
            }
        },
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
        let s_arg = sched.clone();
        let script_owned_arg = sn.to_string();
        engine.register_fn(
            "call_out_arg",
            move |target: &str, function: &str, delay: i64, argument: &str| {
                let d = Duration::from_secs(delay.max(0) as u64);
                s_arg.call_out(
                    target,
                    function,
                    d,
                    vec![serde_json::Value::String(argument.to_string())],
                    Some(script_owned_arg.clone()),
                );
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
            if !target.is_empty() {
                unsafe { &mut *body_ptr_change }.temp_mut().insert(
                    CHANGE_PLAYER_REQUEST.to_string(),
                    Value::String(target.to_string()),
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
                            || mob_data.reaction_names.iter().any(|reaction| {
                                reaction == mob_name || reaction.starts_with(mob_name)
                            });
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

    let body_ptr_room_object = body_ptr;
    engine.register_fn(
        "room_has_object",
        move |_ob: &mut rhai::Map, name: &str| -> bool {
            room_has_object_named(unsafe { &*body_ptr_room_object }, name)
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
        if world.mob_cache.get_mob(index).is_none() {
            let Some((zone, filename)) = index.split_once(':') else {
                return result;
            };
            if world.mob_cache.load_mob(zone, filename).is_err() {
                return result;
            }
        }
        let Some(data) = world.mob_cache.get_mob(index).cloned() else {
            return result;
        };
        let mut placed = 0_i64;
        for location in &data.locations {
            let room = location.clone();
            if world.room_cache.get_room(&data.zone, &room).is_err() {
                continue;
            }
            world
                .mob_cache
                .add_mob_instance(crate::world::MobInstance::new(
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
        let Ok(mut world) = get_world_state().write() else {
            return false;
        };
        // Python getItem(index) reloads an absent cache entry from its JSON
        // immediately before Item.Items.__delitem__().
        if world.item_cache.get_item(index).is_none() && world.item_cache.load_item(index).is_err()
        {
            return false;
        }
        world.item_cache.remove_item(index)
    });

    engine.register_fn("delete_room_definition", move |index: &str| -> bool {
        let Some((zone, room)) = index.split_once(':') else {
            return false;
        };
        if zone.is_empty() || room.is_empty() {
            return false;
        }
        let Ok(mut world) = get_world_state().write() else {
            return false;
        };
        if !world.room_cache.remove_room(zone, room) {
            return false;
        }
        // Python removes the Room object from Room.Zones. A later getRoom()
        // reconstructs attributes from disk rather than retaining an overlay
        // attached to the removed object.
        world.room_attrs.remove(&format!("{zone}:{room}"));
        true
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
                let mut recipes = rhai::Map::new();
                for (key, value) in &mob_data.attributes {
                    if !key.starts_with("조제 ") {
                        continue;
                    }
                    let lines = match value {
                        serde_json::Value::Array(values) => values
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>()
                            .join("\r\n"),
                        serde_json::Value::String(value) => value.clone(),
                        _ => String::new(),
                    };
                    recipes.insert(key.clone().into(), Dynamic::from(lines));
                }
                m.insert("recipes".into(), Dynamic::from(recipes));
                arr.push(Dynamic::from(m));
            }
        }
        arr
    });
    engine.register_fn("item_template_exists", |key: &str| -> bool {
        object_from_item_json(key).is_some()
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

    // Python 상태보기의 env.findObjName: 플레이어/몹/아이템을 방의
    // 통합 삽입 순서로 선택한다. 표현은 Rhai가 담당하고 여기서는
    // 선택된 종류와 런타임 식별자만 돌려준다.
    let body_ptr_status_target = body_ptr;
    engine.register_fn(
        "find_admin_status_target",
        move |_ob: &mut rhai::Map, query: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_status_target };
            let mut query = query.split_whitespace().next().unwrap_or("");
            if query.is_empty() {
                return Dynamic::UNIT;
            }
            let pure_numeric = query.chars().all(|ch| ch.is_ascii_digit());
            let digits = query.chars().take_while(|ch| ch.is_ascii_digit()).count();
            let order = if pure_numeric {
                query.parse::<usize>().unwrap_or(0)
            } else if digits == 0 {
                1
            } else {
                let parsed = query[..digits].parse::<usize>().unwrap_or(0);
                query = &query[digits..];
                parsed
            };
            if order == 0 || query.is_empty() {
                return Dynamic::UNIT;
            }
            let Some((zone, room)) = current_body_position(body) else {
                return Dynamic::UNIT;
            };
            let player_values = room_admin_player_values(body).unwrap_or_default();
            let Ok(world) = get_world_state().read() else {
                return Dynamic::UNIT;
            };
            let floor = world.get_room_objs(&zone, &room);
            let mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room);
            if pure_numeric {
                if let Some(mob) = mobs
                    .iter()
                    .filter(|mob| {
                        !matches!(mob.act, 2 | 3)
                            && world
                                .mob_cache
                                .get_mob(&mob.mob_key)
                                .is_some_and(|data| data.mob_type != 7)
                    })
                    .nth(order - 1)
                {
                    let mut target = rhai::Map::new();
                    target.insert("kind".into(), Dynamic::from("mob"));
                    target.insert("instance_id".into(), Dynamic::from(mob.instance_id as i64));
                    return Dynamic::from(target);
                }
                return Dynamic::UNIT;
            }
            let player_match_counts = |name: &str| -> Option<(bool, usize)> {
                if name == body.get_name() {
                    if body.get_int("투명상태") == 1 {
                        return None;
                    }
                    let reactions = body.get_string("반응이름");
                    let aliases = reaction_names(&reactions);
                    let exact = name == query || aliases.iter().any(|alias| alias == query);
                    let prefixes = if exact {
                        0
                    } else {
                        aliases
                            .iter()
                            .filter(|alias| alias.starts_with(query))
                            .count()
                    };
                    return (exact || prefixes > 0).then_some((exact, prefixes));
                }
                player_values.iter().find_map(|player| {
                    if player.get("name").and_then(|value| value.as_str()) != Some(name)
                        || player
                            .get("raw_attrs")
                            .and_then(|attrs| attrs.get("투명상태"))
                            .and_then(serde_json::Value::as_i64)
                            == Some(1)
                    {
                        return None;
                    }
                    let reactions = player
                        .get("raw_attrs")
                        .and_then(|attrs| attrs.get("반응이름"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let aliases = reaction_names(reactions);
                    let exact = name == query || aliases.iter().any(|alias| alias == query);
                    let prefixes = if exact {
                        0
                    } else {
                        aliases
                            .iter()
                            .filter(|alias| alias.starts_with(query))
                            .count()
                    };
                    (exact || prefixes > 0).then_some((exact, prefixes))
                })
            };
            let mut exact_count = 0usize;
            let mut prefix_count = 0usize;
            let ordered_objects = world.get_room_object_order(&zone, &room);
            let indexed_players = ordered_objects
                .iter()
                .filter_map(|object| match object {
                    RoomObjectRef::Player(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect::<std::collections::HashSet<_>>();
            let indexed_mobs = ordered_objects
                .iter()
                .filter_map(|object| match object {
                    RoomObjectRef::Mob(id) => Some(*id),
                    _ => None,
                })
                .collect::<std::collections::HashSet<_>>();
            for object in ordered_objects.iter().cloned() {
                let selected = match object {
                    RoomObjectRef::Player(name) => player_match_counts(&name).map(|counts| {
                        let mut target = rhai::Map::new();
                        target.insert("kind".into(), Dynamic::from("player"));
                        target.insert("name".into(), Dynamic::from(name));
                        (Dynamic::from(target), counts.0, counts.1)
                    }),
                    RoomObjectRef::Mob(id) => mobs
                        .iter()
                        .find(|mob| mob.instance_id == id)
                        .and_then(|mob| {
                            let data = world.mob_cache.get_mob(&mob.mob_key)?;
                            let transparent = mob
                                .runtime_attrs
                                .get("투명상태")
                                .is_some_and(|value| matches!(value, Value::Int(1)))
                                || data
                                    .attributes
                                    .get("투명상태")
                                    .and_then(serde_json::Value::as_i64)
                                    == Some(1);
                            if transparent || (query != "시체" && matches!(mob.act, 2 | 3)) {
                                return None;
                            }
                            let exact = (query == "시체" && mob.act == 2)
                                || data.name == query
                                || data.reaction_names.iter().any(|alias| alias == query);
                            let prefixes = if exact {
                                0
                            } else {
                                data.reaction_names
                                    .iter()
                                    .filter(|alias| alias.starts_with(query))
                                    .count()
                            };
                            (exact || prefixes > 0).then(|| {
                                let mut target = rhai::Map::new();
                                target.insert("kind".into(), Dynamic::from("mob"));
                                target.insert("instance_id".into(), Dynamic::from(id as i64));
                                (Dynamic::from(target), exact, prefixes)
                            })
                        }),
                    RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => floor
                        .iter()
                        .find(|item| Arc::as_ptr(item) as usize == pointer)
                        .and_then(|item| item.lock().ok())
                        .and_then(|item| {
                            if item.getInt("투명상태") == 1 {
                                return None;
                            }
                            let reactions = item.getString("반응이름");
                            let aliases = reaction_names(&reactions);
                            let exact = item.getName() == query
                                || aliases.iter().any(|alias| alias == query);
                            let prefixes = if exact {
                                0
                            } else {
                                aliases
                                    .iter()
                                    .filter(|alias| alias.starts_with(query))
                                    .count()
                            };
                            (exact || prefixes > 0).then(|| {
                                let mut target = rhai::Map::new();
                                target.insert("kind".into(), Dynamic::from("item"));
                                (Dynamic::from(target), exact, prefixes)
                            })
                        }),
                    RoomObjectRef::InstalledBox(ordinal) => {
                        box_commands::installed_boxes_for_room(&zone, &room)
                            .and_then(|boxes| boxes.get(ordinal).cloned())
                            .and_then(|item| {
                                item.lock().ok().and_then(|item| {
                                    if item.getInt("투명상태") == 1 {
                                        return None;
                                    }
                                    let reactions = item.getString("반응이름");
                                    let aliases = reaction_names(&reactions);
                                    let exact = item.getName() == query
                                        || aliases.iter().any(|alias| alias == query);
                                    let prefixes = if exact {
                                        0
                                    } else {
                                        aliases
                                            .iter()
                                            .filter(|alias| alias.starts_with(query))
                                            .count()
                                    };
                                    (exact || prefixes > 0).then_some((exact, prefixes))
                                })
                            })
                            .map(|(exact, prefixes)| {
                                let mut target = rhai::Map::new();
                                target.insert("kind".into(), Dynamic::from("item"));
                                (Dynamic::from(target), exact, prefixes)
                            })
                    }
                    RoomObjectRef::SummonedUser(id) => world
                        .summoned_users()
                        .iter()
                        .find(|user| user.id == id)
                        .filter(|user| user.body.get_int("투명상태") != 1)
                        .and_then(|user| {
                            let reactions = user.body.get_string("반응이름");
                            let aliases = reaction_names(&reactions);
                            let exact = user.body.get_name() == query
                                || aliases.iter().any(|alias| alias == query);
                            let prefixes = if exact {
                                0
                            } else {
                                aliases
                                    .iter()
                                    .filter(|alias| alias.starts_with(query))
                                    .count()
                            };
                            (exact || prefixes > 0).then_some((user, exact, prefixes))
                        })
                        .map(|(user, exact, prefixes)| {
                            let mut target = rhai::Map::new();
                            target.insert("kind".into(), Dynamic::from("player"));
                            target.insert("name".into(), Dynamic::from(user.body.get_name()));
                            (Dynamic::from(target), exact, prefixes)
                        }),
                    RoomObjectRef::Fixture(id) => world.get_fixture(id).and_then(|fixture| {
                        let (exact, prefixes) = fixture.match_counts(query);
                        (exact || prefixes > 0).then(|| {
                            let mut target = rhai::Map::new();
                            target.insert("kind".into(), Dynamic::from("fixture"));
                            target.insert("id".into(), Dynamic::from(id as i64));
                            (Dynamic::from(target), exact, prefixes)
                        })
                    }),
                };
                if let Some((selected, exact, prefixes)) = selected {
                    if exact {
                        exact_count += 1;
                        if exact_count == order {
                            return selected;
                        }
                    } else {
                        for _ in 0..prefixes {
                            prefix_count += 1;
                            if prefix_count == order {
                                return selected;
                            }
                        }
                    }
                }
            }
            // Legacy caches and focused fixtures can contain live objects
            // created before the unified index was populated. Preserve their
            // collection order as a fallback; production indexed rooms always
            // return above and therefore retain cross-type ordering.
            if !indexed_players.contains(body.get_name().as_str()) {
                if let Some((exact, prefixes)) = player_match_counts(&body.get_name()) {
                    let selected = if exact {
                        exact_count += 1;
                        exact_count == order
                    } else {
                        let before = prefix_count;
                        prefix_count += prefixes;
                        before < order && order <= prefix_count
                    };
                    if selected {
                        let mut target = rhai::Map::new();
                        target.insert("kind".into(), Dynamic::from("player"));
                        target.insert("name".into(), Dynamic::from(body.get_name()));
                        return Dynamic::from(target);
                    }
                }
            }
            for player in &player_values {
                let Some(name) = player.get("name").and_then(|value| value.as_str()) else {
                    continue;
                };
                if indexed_players.contains(name) {
                    continue;
                }
                if let Some((exact, prefixes)) = player_match_counts(name) {
                    let selected = if exact {
                        exact_count += 1;
                        exact_count == order
                    } else {
                        let before = prefix_count;
                        prefix_count += prefixes;
                        before < order && order <= prefix_count
                    };
                    if selected {
                        let mut target = rhai::Map::new();
                        target.insert("kind".into(), Dynamic::from("player"));
                        target.insert("name".into(), Dynamic::from(name.to_string()));
                        return Dynamic::from(target);
                    }
                }
            }
            for mob in mobs {
                if indexed_mobs.contains(&mob.instance_id) {
                    continue;
                }
                let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                    continue;
                };
                if matches!(mob.act, 2 | 3) && !(query == "시체" && mob.act == 2) {
                    continue;
                }
                let exact = (query == "시체" && mob.act == 2)
                    || data.name == query
                    || data.reaction_names.iter().any(|alias| alias == query);
                let prefixes = if exact {
                    0
                } else {
                    data.reaction_names
                        .iter()
                        .filter(|alias| alias.starts_with(query))
                        .count()
                };
                if exact || prefixes > 0 {
                    let selected = if exact {
                        exact_count += 1;
                        exact_count == order
                    } else {
                        let before = prefix_count;
                        prefix_count += prefixes;
                        before < order && order <= prefix_count
                    };
                    if selected {
                        let mut target = rhai::Map::new();
                        target.insert("kind".into(), Dynamic::from("mob"));
                        target.insert("instance_id".into(), Dynamic::from(mob.instance_id as i64));
                        return Dynamic::from(target);
                    }
                }
            }
            Dynamic::UNIT
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

            if let Some(values) = room_admin_player_values(body) {
                for value in values {
                    if value.get("name").and_then(|v| v.as_str()) == Some(viewer_name.as_str()) {
                        continue;
                    }
                    let mut m = rhai::Map::new();
                    if let Some(object) = value.as_object() {
                        for (key, value) in object {
                            if value.is_array() || value.is_object() {
                                m.insert(key.clone().into(), json_value_to_dynamic(value.clone()));
                            } else if let Some(number) = value.as_i64() {
                                m.insert(key.clone().into(), Dynamic::from(number));
                            } else if let Some(text) = value.as_str() {
                                m.insert(key.clone().into(), Dynamic::from(text.to_string()));
                            }
                        }
                    }
                    arr.push(Dynamic::from(m));
                }
                // Python channel.players also contains socket-less Players
                // created by 사용자몹소환. They are selected and rendered by
                // 상태보기 exactly like connected players.
                if let Ok(world) = get_world_state().read() {
                    if let Some(pos) = world.get_player_position(viewer_name.as_str()) {
                        for user in world.summoned_users_in_room(&pos.zone, &pos.room) {
                            let mut m = admin_combat::body_status(&user.body);
                            m.insert("name".into(), Dynamic::from(user.body.get_name()));
                            arr.push(Dynamic::from(m));
                        }
                    }
                }
                return arr;
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
            for user in w.summoned_users_in_room(&pos.zone, &pos.room) {
                let mut m = admin_combat::body_status(&user.body);
                m.insert("name".into(), Dynamic::from(user.body.get_name()));
                arr.push(Dynamic::from(m));
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

    // teach_skill_to_target(..., allow_duplicate) - 플레이어/몹 무공 전수
    let body_ptr_teach = body_ptr;
    engine.register_fn(
        "teach_skill_to_target",
        move |_teacher_ob: &mut rhai::Map,
              student_name: &str,
              skill_name: &str,
              allow_duplicate: bool|
              -> String {
            let body = unsafe { &mut *body_ptr_teach };
            if body.get_name() == student_name {
                if !allow_duplicate && body.skill_list.iter().any(|name| name == skill_name) {
                    return "duplicate".to_string();
                }
                body.skill_list.push(skill_name.to_string());
                body.sync_skill_state_to_attrs();
                return "ok".to_string();
            }
            let target = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                cell.borrow().as_ref().and_then(|targets| {
                    targets
                        .iter()
                        .find(|target| !target.transparent && target.name == student_name)
                        .cloned()
                })
            });
            let Some(target) = target else {
                return "not_found".to_string();
            };
            match target.kind {
                RoomMugongTargetKind::Player => {
                    if !allow_duplicate && target.skill_levels.contains_key(skill_name) {
                        return "duplicate".to_string();
                    }
                    let Ok(json) = serde_json::to_string(&(student_name, skill_name)) else {
                        return "not_found".to_string();
                    };
                    body.temp_mut()
                        .insert(TEACH_SKILL_REQUEST.to_string(), Value::String(json));
                    "ok".to_string()
                }
                RoomMugongTargetKind::Mob => {
                    let Some(position) = current_body_position(body) else {
                        return "not_found".to_string();
                    };
                    let Ok(mut world) = get_world_state().write() else {
                        return "not_found".to_string();
                    };
                    let configured = world
                        .mob_cache
                        .get_all_mobs_in_room(&position.0, &position.1)
                        .into_iter()
                        .find(|mob| mob.name == student_name)
                        .and_then(|mob| world.mob_cache.get_instance_data(mob))
                        .is_some_and(|data| {
                            data.skills.iter().any(|(name, _, _)| name == skill_name)
                        });
                    let Some(mobs) = world
                        .mob_cache
                        .get_all_mobs_in_room_mut(&position.0, &position.1)
                    else {
                        return "not_found".to_string();
                    };
                    let Some(mob) = mobs.iter_mut().find(|mob| mob.name == student_name) else {
                        return "not_found".to_string();
                    };
                    if !allow_duplicate
                        && (configured || mob.learned_skills.iter().any(|name| name == skill_name))
                    {
                        return "duplicate".to_string();
                    }
                    mob.learned_skills.push(skill_name.to_string());
                    "ok".to_string()
                }
                RoomMugongTargetKind::Item => "not_found".to_string(),
            }
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
                            && target
                                .active_skills
                                .iter()
                                .any(|skill| skill.name == skill_name)
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
    let raw_user_sends = user_sends.clone();
    engine.register_fn(
        "send_raw_to_player",
        move |player_name: &str, message: &str| -> bool {
            if player_name.is_empty() || message.is_empty() {
                return false;
            }
            if let Ok(mut sends) = raw_user_sends.lock() {
                sends.push((
                    player_name.to_string(),
                    format!("{}{}", RAW_USER_MESSAGE_PREFIX, message),
                ));
                true
            } else {
                false
            }
        },
    );
    let body_ptr_view_players = body_ptr;
    engine.register_fn(
        "get_room_view_players",
        move |_ob: &mut rhai::Map| -> rhai::Array {
            let body = unsafe { &*body_ptr_view_players };
            current_body_position(body)
                .map(|(zone, room)| room_view_player_snapshots(&zone, &room))
                .unwrap_or_default()
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

            if let Some(ref key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, item_name)
            {
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
            if let Some(RoomObjectRef::Mob(id)) = select_python_room_object(body, target) {
                let Some((zone, room)) = current_body_position(body) else {
                    return "not_found".into();
                };
                let Ok(mut world) = get_world_state().write() else {
                    return "not_found".into();
                };
                let Some(mobs) = world.mob_cache.get_all_mobs_in_room_mut(&zone, &room) else {
                    return "not_found".into();
                };
                let Some(mob) = mobs.iter_mut().find(|mob| mob.instance_id == id) else {
                    return "not_found".into();
                };
                mob.skill_map.insert(
                    skill.to_string(),
                    crate::player::SkillTraining::new(level, 199_999),
                );
                if !mob.learned_skills.iter().any(|name| name == skill) {
                    mob.learned_skills.push(skill.to_string());
                }
                return "ok".into();
            }
            if let Some(RoomObjectRef::SummonedUser(id)) = select_python_room_object(body, target) {
                let Ok(mut world) = get_world_state().write() else {
                    return "not_found".into();
                };
                let Some(user) = world.summoned_user_mut(id) else {
                    return "not_found".into();
                };
                user.body.skill_map.insert(
                    skill.to_string(),
                    crate::player::SkillTraining::new(level, 199_999),
                );
                return "ok".into();
            }
            if target == body.get_name() {
                if current_body_position(body).is_some()
                    && select_python_room_object(body, target)
                        != Some(RoomObjectRef::Player(body.get_name()))
                {
                    return "not_found".into();
                }
                body.skill_map.insert(
                    skill.to_string(),
                    crate::player::SkillTraining::new(level, 199_999),
                );
                return "ok".into();
            }
            let resolved = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                cell.borrow().as_ref().and_then(|targets| {
                    find_room_mugong_target(target, targets)
                        .filter(|candidate| candidate.kind == RoomMugongTargetKind::Player)
                })
            });
            let Some(resolved) = resolved else {
                return "not_found".into();
            };
            let Ok(json) = serde_json::to_string(&(resolved.name, skill, level)) else {
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
    // Spell execution helper (could use spell.json in future).
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

    // remove_active_skill_from_target(ob, target_name, skill_name) - 활성 무공 제거
    // 성공 시 true 반환
    let body_ptr_remove_skill = body_ptr;
    engine.register_fn(
        "remove_active_skill_from_target",
        move |_ob: &mut rhai::Map, target_name: &str, skill_name: &str| -> bool {
            let body = unsafe { &mut *body_ptr_remove_skill };
            if skill_name.is_empty() {
                return false;
            }
            if body.get_name() != target_name {
                let target = PRE_COMPUTED_ROOM_MUGONG_TARGETS.with(|cell| {
                    cell.borrow().as_ref().and_then(|targets| {
                        targets
                            .iter()
                            .find(|target| !target.transparent && target.name == target_name)
                            .cloned()
                    })
                });
                let Some(target) = target else {
                    return false;
                };
                if !target
                    .active_skills
                    .iter()
                    .any(|active| active.name == skill_name)
                {
                    return false;
                }
                match target.kind {
                    RoomMugongTargetKind::Player => {
                        let Ok(json) = serde_json::to_string(&(target_name, skill_name)) else {
                            return false;
                        };
                        body.temp_mut()
                            .insert(REMOVE_SKILL_REQUEST.to_string(), Value::String(json));
                        return true;
                    }
                    RoomMugongTargetKind::Mob => {
                        let Some(position) = current_body_position(body) else {
                            return false;
                        };
                        let Ok(mut world) = get_world_state().write() else {
                            return false;
                        };
                        let Some(mobs) = world
                            .mob_cache
                            .get_all_mobs_in_room_mut(&position.0, &position.1)
                        else {
                            return false;
                        };
                        return mobs
                            .iter_mut()
                            .find(|mob| mob.name == target_name)
                            .is_some_and(|mob| mob.remove_skill_effect_by_name(skill_name));
                    }
                    RoomMugongTargetKind::Item => return false,
                }
            }
            let removed = body.remove_active_skill_by_name(skill_name);
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
            if let Some(key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, obj_name)
            {
                if body.object.inv_stack.get(&key).copied().unwrap_or(0) > 0 {
                    if let Some((item, _)) = object_from_item_json(&key) {
                        if let Ok(item) = item.lock() {
                            let mut obj_data = rhai::Map::new();
                            obj_data.insert("이름".into(), Dynamic::from(item.getName()));
                            obj_data.insert("표시".into(), Dynamic::from(item.getNameA()));
                            obj_data.insert("종류".into(), Dynamic::from(item.getString("종류")));
                            return Dynamic::from(obj_data);
                        }
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
            let Some(available) =
                crate::book::dict_get(entry, "대여가능").and_then(serde_json::Value::as_bool)
            else {
                return "unavailable".into();
            };
            if !available {
                return "borrowed".into();
            }
            let key = crate::book::dict_get_string(entry, "인덱스");
            let Some(attributes) =
                crate::book::dict_get(entry, "attr").filter(|value| value.is_object())
            else {
                return "unavailable".into();
            };
            if key.is_empty() {
                return "unavailable".into();
            }
            let Some((item, _)) = object_from_item_json(&key) else {
                return "unavailable".into();
            };
            if let Ok(mut obj) = item.lock() {
                inventory_compat::replace_item_attributes_from_json(&mut obj, attributes);
                obj.set("고유번호", crate::book::dict_get_string(entry, "고유번호"));
            }
            if crate::book::mark_borrowed(&catalog_path, number as usize, &body.get_name()).is_err()
            {
                return "persist_failed".into();
            }
            let _ = inventory_compat::store_acquired_object(&mut body.object, item, false);
            let path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &path);
            "ok".into()
        },
    );

    let body_ptr_guard_qi = body_ptr;
    engine.register_fn("inject_guard_qi", move |_ob: &mut rhai::Map| -> Dynamic {
        let body = unsafe { &mut *body_ptr_guard_qi };
        let mut healed = rhai::Array::new();
        let mut actual_spent = 0i64;
        let mut last_cost = 0i64;
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
            // Python reuses this loop variable in the final presentation,
            // including the candidate that caused a shortage and broke.
            last_cost = cost;
            if body.get_int("내공") - actual_spent - cost < 0 {
                break;
            }
            let gain = max_hp * guard.getInt("체력증가") / 100;
            guard.set("체력", (hp + gain).min(max_hp));
            actual_spent += cost;
            let mut m = rhai::Map::new();
            m.insert("이름".into(), Dynamic::from(guard.getName()));
            m.insert("회복".into(), Dynamic::from(gain));
            healed.push(Dynamic::from(m));
        }
        if !healed.is_empty() {
            body.set("내공", body.get_int("내공") - actual_spent);
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
        let presentation_spent = last_cost.saturating_mul(healed.len() as i64);
        result.insert("status".into(), Dynamic::from(status));
        result.insert("healed".into(), Dynamic::from(healed));
        result.insert("spent".into(), Dynamic::from(presentation_spent));
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
        move |_ob: &mut rhai::Map, item_name: &str, order: i64| -> String {
            let body = unsafe { &mut *body_ptr_book_register };
            let catalog_path = book_catalog_path(body);
            let order = order.max(1) as usize;
            let mut matched = 0usize;
            let selected = body.object.objs.iter().find_map(|item| {
                let object = item.lock().ok()?;
                if object.getBool("inUse")
                    || (object.getName() != item_name
                        && !inventory_compat::python_item_field_contains(
                            &object,
                            "반응이름",
                            item_name,
                        ))
                {
                    return None;
                }
                matched += 1;
                (matched == order).then(|| item.clone())
            });
            let selected = match selected {
                Some(selected) => selected,
                None => {
                    let remaining = order.saturating_sub(matched) as i64;
                    let Some((key, _)) = inventory_compat::counted_item_at(
                        &body.object.inv_stack,
                        item_name,
                        remaining,
                    ) else {
                        return "no_item".into();
                    };
                    let Some(selected) =
                        inventory_compat::materialize_one(&mut body.object, &key, true)
                    else {
                        return "no_item".into();
                    };
                    selected
                }
            };
            let Some((pos, key, name, attributes)) = body
                .object
                .objs
                .iter()
                .position(|item| Arc::ptr_eq(item, &selected))
                .and_then(|pos| {
                    let obj = selected.lock().ok()?;
                    let attributes = inventory_compat::item_attributes_to_json(&obj)
                        .as_object()
                        .cloned()
                        .unwrap_or_default();
                    Some((pos, obj.getString("인덱스"), obj.getName(), attributes))
                })
            else {
                restore_pristine_inventory_object(body, &selected);
                return "no_item".into();
            };
            if key.is_empty() {
                restore_pristine_inventory_object(body, &selected);
                return "cannot_register".into();
            }
            let Some((kind, cannot_give, item_id)) = body.object.objs[pos].lock().ok().map(|o| {
                (
                    o.getString("종류"),
                    inventory_compat::python_item_field_contains(&o, "아이템속성", "줄수없음"),
                    o.getString("고유번호"),
                )
            }) else {
                restore_pristine_inventory_object(body, &selected);
                return "cannot_register".into();
            };
            if kind != "무기" || cannot_give || !item_id.is_empty() {
                restore_pristine_inventory_object(body, &selected);
                return "cannot_register".into();
            }
            if crate::book::register_item(&catalog_path, &key, &name, &body.get_name(), attributes)
                .is_err()
            {
                restore_pristine_inventory_object(body, &selected);
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
            let (wanted, order) =
                crate::command::parser::CommandParser::parse_name_order(item_name);
            let mut matched = 0usize;
            let Some((pos, item_id)) = body.object.objs.iter().enumerate().find_map(|(i, arc)| {
                let obj = arc.lock().ok()?;
                if obj.getBool("inUse")
                    || (obj.getName() != wanted
                        && !inventory_compat::python_item_field_contains(&obj, "반응이름", &wanted))
                {
                    return None;
                }
                matched += 1;
                (matched == order).then(|| (i, obj.getString("고유번호")))
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
            let Some(available) =
                crate::book::dict_get(candidate, "대여가능").and_then(serde_json::Value::as_bool)
            else {
                return "unavailable".into();
            };
            if !available {
                return "borrowed".into();
            }
            let key = crate::book::dict_get_string(candidate, "인덱스");
            if key.is_empty()
                || crate::book::dict_get(candidate, "attr")
                    .is_none_or(|attributes| !attributes.is_object())
                || object_from_item_json(&key).is_none()
            {
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
                inventory_compat::replace_item_attributes_from_json(
                    &mut obj,
                    crate::book::dict_get(&entry, "attr").expect("preflighted book attr"),
                );
                obj.set("고유번호", "");
            }
            let _ = inventory_compat::store_acquired_object(&mut body.object, item, false);
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
            let selected = select_python_room_object(body, line);
            if matches!(
                selected,
                Some(RoomObjectRef::Player(_) | RoomObjectRef::SummonedUser(_))
            ) {
                return result("cannot_save", String::new());
            }
            let mob_target = get_world_state().read().ok().and_then(|world| {
                world
                    .mob_cache
                    .get_all_mobs_in_room(&zone, &room)
                    .into_iter()
                    .find_map(|mob| {
                        if let Some(RoomObjectRef::Mob(id)) = selected.as_ref() {
                            if mob.instance_id != *id {
                                return None;
                            }
                        } else if selected.is_some() {
                            return None;
                        }
                        let data = world.get_mob_data(&mob.mob_key)?;
                        (matches!(selected, Some(RoomObjectRef::Mob(_)))
                            || mob.name == line
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
                let Some(info) = root
                    .get_mut("몹정보")
                    .and_then(|value| value.as_object_mut())
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
            let installed =
                box_commands::installed_boxes_for_room(&zone, &room).unwrap_or_default();
            let selected_item = match selected.as_ref() {
                Some(RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer)) => room_items
                    .iter()
                    .chain(installed.iter())
                    .find(|item| Arc::as_ptr(item) as usize == *pointer)
                    .cloned(),
                Some(RoomObjectRef::InstalledBox(ordinal)) => installed.get(*ordinal).cloned(),
                Some(_) => None,
                None => None,
            };
            let item = selected_item
                .or_else(|| {
                    (selected.is_none())
                        .then(|| {
                            room_items
                                .into_iter()
                                .find(|item| item.lock().is_ok_and(|object| matches(&object)))
                        })
                        .flatten()
                })
                .or_else(|| {
                    if selected.is_some() {
                        return None;
                    }
                    body.object
                        .objs
                        .iter()
                        .find(|item| item.lock().is_ok_and(|object| matches(&object)))
                        .cloned()
                })
                .or_else(|| {
                    if selected.is_some() {
                        return None;
                    }
                    let key =
                        inventory_compat::find_counted_item_key(&body.object.inv_stack, line)?;
                    object_from_item_json(&key).map(|(item, _)| item)
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
            if std::fs::write(
                &path,
                serde_json::to_string_pretty(&out).unwrap_or_default(),
            )
            .is_err()
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
                return "permission".into();
            };
            let selected = body.object.objs.iter().enumerate().find_map(|(i, arc)| {
                let obj = arc.lock().ok()?;
                (obj.getName() == item_name
                    || reaction_names(&obj.getString("반응이름"))
                        .iter()
                        .any(|alias| alias == item_name))
                .then(|| (Some(i), arc.clone(), None))
            });
            let selected = selected.or_else(|| {
                let key =
                    inventory_compat::find_counted_item_key(&body.object.inv_stack, item_name)?;
                (body.object.inv_stack.get(&key).copied().unwrap_or(0) > 0)
                    .then(|| object_from_item_json(&key).map(|(item, _)| (None, item, Some(key))))
                    .flatten()
            });
            let Some((pos, item, stack_key)) = selected else {
                return "missing".into();
            };
            let Ok(source) = item.lock() else {
                return "invalid".into();
            };
            if source.getString("종류") != "설치아이템" {
                return "invalid".into();
            }
            let name = source.getName();
            let path = format!("data/map/{zone}/{room}.json");
            let Ok(text) = std::fs::read_to_string(&path) else {
                return "permission".into();
            };
            let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&text) else {
                return "permission".into();
            };
            let Some(info) = root.get_mut("맵정보").and_then(|v| v.as_object_mut()) else {
                return "permission".into();
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
                return "permission".into();
            }
            if owner.is_empty() && guild_owner != body.get_string("소속") {
                return "permission".into();
            }
            // Python permits installation in a guild-owned room only for
            // items explicitly marked 공용보관함.
            if owner.is_empty() && !source.checkAttr("아이템속성", "공용보관함") {
                return "permission".into();
            }
            let installed = info
                .entry("설치리스트")
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
            if let Some(existing) = installed.as_str().map(str::to_string) {
                *installed = if existing.is_empty() {
                    serde_json::Value::Array(Vec::new())
                } else {
                    serde_json::Value::Array(vec![serde_json::Value::String(existing)])
                };
            }
            let Some(list) = installed.as_array_mut() else {
                return "duplicate".into();
            };
            if list.iter().any(|v| v.as_str() == Some(&name)) {
                return "duplicate".into();
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
                return "permission".into();
            }
            let mut boxed = Object::new();
            for (k, v) in &source.attr {
                boxed.attr.insert(k.clone(), v.clone());
            }
            boxed.set("주인", owner_name.clone());
            drop(source);
            if !box_commands::prepare_installed_box(&mut boxed, &owner_name, &name) {
                return "permission".into();
            }
            box_commands::register_installed_box(
                &zone,
                &room,
                std::sync::Arc::new(std::sync::Mutex::new(boxed)),
            );
            if let Some(pos) = pos {
                body.object.objs.remove(pos);
            } else if let Some(stack_key) = stack_key {
                let _ = inventory_compat::remove_pristine_count(&mut body.object, &stack_key, 1);
            }
            let save_path = format!("data/user/{}.json", body.get_name());
            let _ = save_body_to_json(body, &save_path);
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
            if let Some(ref key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, item_name)
            {
                if is_stackable(key) {
                    if let Some(&have) = body.object.inv_stack.get(key) {
                        if have > 0 && object_from_item_json(key).is_some() {
                            body.object.inv_stack.remove(key);
                            let mut restored = 0_i64;
                            for _ in 0..have {
                                if let Some((item, _)) = object_from_item_json(key) {
                                    w.get_room_objs_mut(target_zone, target_room)
                                        .insert(0, item.clone());
                                    w.record_floor_item(target_zone, target_room, &item);
                                    restored += 1;
                                }
                            }
                            if restored < have {
                                *w.get_room_objs_stack_mut(target_zone, target_room)
                                    .entry(key.clone())
                                    .or_insert(0) += have - restored;
                            }
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
                            target_room_objs.insert(0, arc.clone());
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
                        if !obj.attr.contains_key(attr) {
                            return Dynamic::UNIT;
                        }
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

            // 원본과 동일한 수량형 아이템은 객체화하지 않고 템플릿 속성을 읽는다.
            if let Some(key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, obj_name)
            {
                if let Some((item, _)) = object_from_item_json(&key) {
                    if let Ok(item) = item.lock() {
                        if !item.attr.contains_key(attr) {
                            return Dynamic::UNIT;
                        }
                        return match item.get(attr) {
                            crate::object::Value::Int(n) => Dynamic::from_int(n),
                            crate::object::Value::String(s) => Dynamic::from(s),
                            crate::object::Value::Float(f) => Dynamic::from(f),
                        };
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
                                if !obj.attr.contains_key(attr) {
                                    return Dynamic::UNIT;
                                }
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
    let body_ptr_resolve_room_player = body_ptr;
    engine.register_fn(
        "resolve_room_player_name",
        move |_ob: &mut rhai::Map, query: &str| -> String {
            let body = unsafe { &*body_ptr_resolve_room_player };
            let mut query = query.split_whitespace().next().unwrap_or("");
            if query.is_empty() || query == "." || query.chars().all(|ch| ch.is_ascii_digit()) {
                return String::new();
            }
            let digits = query.chars().take_while(|ch| ch.is_ascii_digit()).count();
            let order = if digits == 0 {
                1
            } else {
                let parsed = query[..digits].parse::<usize>().unwrap_or(0);
                query = &query[digits..];
                parsed
            };
            if order == 0 || query.is_empty() {
                return String::new();
            }
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                // Focused/legacy command contexts can provide party/player
                // snapshots without a WorldState position.  Production active
                // players always have one; retain exact-name fixture behavior
                // without weakening real room-object ordering.
                return if order == 1 {
                    query.to_string()
                } else {
                    String::new()
                };
            };
            let players = room_view_player_snapshots(&pos.zone, &pos.room)
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .collect::<Vec<_>>();
            let player_match = |name: &str| -> Option<(bool, usize)> {
                if name == body.get_name() {
                    if body.get_int("투명상태") == 1 {
                        return None;
                    }
                    let reactions = body.get_string("반응이름");
                    let aliases = reaction_names(&reactions);
                    let exact = name == query || reactions.contains(query);
                    let prefixes = if exact {
                        0
                    } else {
                        aliases
                            .iter()
                            .filter(|alias| alias.starts_with(query))
                            .count()
                    };
                    if exact || prefixes > 0 {
                        return Some((exact, prefixes));
                    }
                }
                players.iter().find_map(|player| {
                    let candidate = player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .unwrap_or_default();
                    if candidate != name
                        || player
                            .get("transparent")
                            .and_then(|value| value.as_bool().ok())
                            .unwrap_or(false)
                    {
                        return None;
                    }
                    let reactions = player
                        .get("반응이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .unwrap_or_default();
                    let exact = name == query || reactions.contains(query);
                    let prefixes = if exact {
                        0
                    } else {
                        reaction_names(&reactions)
                            .iter()
                            .filter(|reaction| reaction.starts_with(query))
                            .count()
                    };
                    Some((exact, prefixes))
                })
            };
            let Ok(world) = get_world_state().read() else {
                return String::new();
            };
            let floor = world.get_room_objs(&pos.zone, &pos.room);
            let mobs = world.mob_cache.get_all_mobs_in_room(&pos.zone, &pos.room);
            let mut exact_count = 0usize;
            let mut prefix_count = 0usize;
            let mut selected = |exact: bool, prefixes: usize| {
                if exact {
                    exact_count += 1;
                    exact_count == order
                } else {
                    for _ in 0..prefixes {
                        prefix_count += 1;
                        if prefix_count == order {
                            return true;
                        }
                    }
                    false
                }
            };
            for object in world.get_room_object_order(&pos.zone, &pos.room) {
                match object {
                    RoomObjectRef::Player(name) => {
                        if let Some((exact, prefixes)) = player_match(&name) {
                            if selected(exact, prefixes) {
                                return name;
                            }
                        }
                    }
                    RoomObjectRef::Mob(id) => {
                        let Some(mob) = mobs.iter().find(|mob| mob.instance_id == id) else {
                            continue;
                        };
                        let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                            continue;
                        };
                        if !mob.alive || matches!(mob.act, 2 | 3) {
                            continue;
                        }
                        let exact = data.name == query
                            || data.reaction_names.iter().any(|alias| alias == query);
                        let prefixes = if exact {
                            0
                        } else {
                            data.reaction_names
                                .iter()
                                .filter(|alias| alias.starts_with(query))
                                .count()
                        };
                        if selected(exact, prefixes) {
                            return String::new();
                        }
                    }
                    RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                        let Some(item) = floor
                            .iter()
                            .find(|item| Arc::as_ptr(item) as usize == pointer)
                        else {
                            continue;
                        };
                        let Ok(item) = item.lock() else { continue };
                        let reactions = item.getString("반응이름");
                        if item.getInt("투명상태") == 1 {
                            continue;
                        }
                        let exact = item.getName() == query || reactions.contains(query);
                        let prefixes = if exact {
                            0
                        } else {
                            reaction_names(&reactions)
                                .iter()
                                .filter(|alias| alias.starts_with(query))
                                .count()
                        };
                        if selected(exact, prefixes) {
                            return String::new();
                        }
                    }
                    RoomObjectRef::InstalledBox(ordinal) => {
                        let Some(boxes) =
                            box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                        else {
                            continue;
                        };
                        let Some(item) = boxes.get(ordinal) else {
                            continue;
                        };
                        let Ok(item) = item.lock() else { continue };
                        if item.getInt("투명상태") == 1 {
                            continue;
                        }
                        let reactions = item.getString("반응이름");
                        let exact = item.getName() == query || reactions.contains(query);
                        let prefixes = if exact {
                            0
                        } else {
                            reaction_names(&reactions)
                                .iter()
                                .filter(|alias| alias.starts_with(query))
                                .count()
                        };
                        if selected(exact, prefixes) {
                            return String::new();
                        }
                    }
                    RoomObjectRef::SummonedUser(id) => {
                        let Some(user) = world.summoned_users().iter().find(|user| user.id == id)
                        else {
                            continue;
                        };
                        if user.body.get_int("투명상태") == 1 {
                            continue;
                        }
                        let name = user.body.get_name();
                        let reactions = user.body.get_string("반응이름");
                        let exact = name == query || reactions.contains(query);
                        let prefixes = if exact {
                            0
                        } else {
                            reaction_names(&reactions)
                                .iter()
                                .filter(|alias| alias.starts_with(query))
                                .count()
                        };
                        if selected(exact, prefixes) {
                            return name;
                        }
                    }
                    RoomObjectRef::Fixture(id) => {
                        let (exact, prefixes) = world
                            .get_fixture(id)
                            .map(|fixture| fixture.match_counts(query))
                            .unwrap_or((false, 0));
                        if selected(exact, prefixes) {
                            return String::new();
                        }
                    }
                }
            }
            // Legacy snapshots without unified index entries retain their
            // player list order as a safe fallback.
            players
                .iter()
                .filter_map(|player| player.get("이름")?.clone().into_string().ok())
                .filter_map(|name| player_match(&name).map(|matched| (name, matched)))
                .find_map(|(name, (exact, prefixes))| selected(exact, prefixes).then_some(name))
                .unwrap_or_default()
        },
    );
    let body_ptr_nonplayer_target = body_ptr;
    engine.register_fn(
        "room_has_matching_nonplayer",
        move |_ob: &mut rhai::Map, raw_query: &str| -> bool {
            let body = unsafe { &*body_ptr_nonplayer_target };
            let Some((zone, room)) = current_body_position(body) else {
                return false;
            };
            let Ok(world) = get_world_state().read() else {
                return false;
            };
            admin_combat::python_named_room_selection_is_nonmob(&world, &zone, &room, raw_query)
        },
    );
    let body_ptr_room_object_kind = body_ptr;
    engine.register_fn(
        "selected_room_object_kind",
        move |_ob: &mut rhai::Map, query: &str| -> String {
            let body = unsafe { &*body_ptr_room_object_kind };
            match select_python_room_object(body, query) {
                Some(RoomObjectRef::Player(_) | RoomObjectRef::SummonedUser(_)) => "player".into(),
                Some(_) => "nonplayer".into(),
                None => String::new(),
            }
        },
    );
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
    let body_ptr_room_event = body_ptr;
    let body_ptr_room_target_event = body_ptr;
    engine.register_fn(
        "get_room_target_event_lines",
        move |_ob: &mut rhai::Map, query: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_room_target_event };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                return Dynamic::UNIT;
            };
            let players = room_view_player_snapshots(&pos.zone, &pos.room)
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .collect::<Vec<_>>();
            let Ok(world) = get_world_state().read() else {
                return Dynamic::UNIT;
            };
            let floor = world.get_room_objs(&pos.zone, &pos.room);
            let mobs = world.mob_cache.get_all_mobs_in_room(&pos.zone, &pos.room);
            let chars = |raw: String| {
                Dynamic::from_array(
                    raw.chars()
                        .map(|character| Dynamic::from(character.to_string()))
                        .collect(),
                )
            };
            if let Some(selected) = select_python_room_object(body, query) {
                match selected {
                    RoomObjectRef::Player(name) => {
                        if name == body.get_name() {
                            return chars(body.get_string("이벤트설정리스트"));
                        }
                        if let Some(player) = players.iter().find(|player| {
                            player
                                .get("이름")
                                .and_then(|value| value.clone().into_string().ok())
                                .as_deref()
                                == Some(name.as_str())
                        }) {
                            return chars(
                                player
                                    .get("이벤트설정리스트")
                                    .and_then(|value| value.clone().into_string().ok())
                                    .unwrap_or_default(),
                            );
                        }
                    }
                    RoomObjectRef::SummonedUser(id) => {
                        if let Some(user) = world.summoned_users().iter().find(|user| user.id == id)
                        {
                            return chars(user.body.get_string("이벤트설정리스트"));
                        }
                    }
                    RoomObjectRef::Mob(_) => return Dynamic::from_array(rhai::Array::new()),
                    RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                        let installed =
                            box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                                .unwrap_or_default();
                        if let Some(raw) = floor
                            .iter()
                            .chain(installed.iter())
                            .find(|object| Arc::as_ptr(object) as usize == pointer)
                            .and_then(|object| {
                                object
                                    .lock()
                                    .ok()
                                    .map(|object| object.getString("이벤트설정리스트"))
                            })
                        {
                            return chars(raw);
                        }
                    }
                    RoomObjectRef::InstalledBox(ordinal) => {
                        if let Some(raw) =
                            box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                                .and_then(|boxes| boxes.get(ordinal).cloned())
                                .and_then(|object| {
                                    object
                                        .lock()
                                        .ok()
                                        .map(|object| object.getString("이벤트설정리스트"))
                                })
                        {
                            return chars(raw);
                        }
                    }
                    RoomObjectRef::Fixture(_) => {
                        return Dynamic::from_array(rhai::Array::new());
                    }
                }
            }
            for object in world.get_room_object_order(&pos.zone, &pos.room) {
                match object {
                    RoomObjectRef::Player(name) => {
                        let Some(player) = players.iter().find(|player| {
                            player
                                .get("이름")
                                .and_then(|value| value.clone().into_string().ok())
                                .is_some_and(|candidate| candidate == name)
                        }) else {
                            continue;
                        };
                        let reactions = player
                            .get("반응이름")
                            .and_then(|value| value.clone().into_string().ok())
                            .unwrap_or_default();
                        if name == query || reactions.contains(query) {
                            let raw = player
                                .get("이벤트설정리스트")
                                .and_then(|value| value.clone().into_string().ok())
                                .unwrap_or_default();
                            return chars(raw);
                        }
                    }
                    RoomObjectRef::FloorItem(pointer) => {
                        let Some(object) = floor
                            .iter()
                            .find(|object| Arc::as_ptr(object) as usize == pointer)
                        else {
                            continue;
                        };
                        let Ok(object) = object.lock() else { continue };
                        if object.getName() == query || object.getString("반응이름").contains(query)
                        {
                            return chars(object.getString("이벤트설정리스트"));
                        }
                    }
                    RoomObjectRef::Mob(id) => {
                        let Some(mob) = mobs.iter().find(|mob| mob.instance_id == id) else {
                            continue;
                        };
                        let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                            continue;
                        };
                        if data.name == query
                            || data.reaction_names.iter().any(|name| name.contains(query))
                        {
                            // Python Mob has no 이벤트설정리스트 field in its
                            // templates; Object.__getitem__ returns the empty
                            // default and the command emits no lines.
                            return Dynamic::from_array(rhai::Array::new());
                        }
                    }
                    RoomObjectRef::SummonedUser(id) => {
                        let Some(user) = world.summoned_users().iter().find(|user| user.id == id)
                        else {
                            continue;
                        };
                        if user.body.get_name() == query
                            || user.body.get_string("반응이름").contains(query)
                        {
                            return chars(user.body.get_string("이벤트설정리스트"));
                        }
                    }
                    _ => {}
                }
            }
            // Caches created before the unified room index (and focused unit
            // fixtures) still expose objects through the legacy collections.
            for object in floor {
                let Ok(object) = object.lock() else { continue };
                if object.getName() == query || object.getString("반응이름").contains(query) {
                    return chars(object.getString("이벤트설정리스트"));
                }
            }
            for mob in mobs {
                let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                    continue;
                };
                if data.name == query || data.reaction_names.iter().any(|name| name.contains(query))
                {
                    return Dynamic::from_array(rhai::Array::new());
                }
            }
            Dynamic::UNIT
        },
    );
    engine.register_fn(
        "get_room_player_event_lines",
        move |_ob: &mut rhai::Map, target: &str| -> Dynamic {
            let body = unsafe { &*body_ptr_room_event };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                return Dynamic::UNIT;
            };
            let selected = room_view_player_snapshots(&pos.zone, &pos.room)
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .find(|player| {
                    let name = player
                        .get("이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .unwrap_or_default();
                    let reactions = player
                        .get("반응이름")
                        .and_then(|value| value.clone().into_string().ok())
                        .unwrap_or_default();
                    name == target || reactions.contains(target)
                });
            let Some(player) = selected else {
                return Dynamic::UNIT;
            };
            let raw = player
                .get("이벤트설정리스트")
                .and_then(|value| value.clone().into_string().ok())
                .unwrap_or_default();
            // Current Python-compatible Body storage is a string. Python's
            // `for l in target[attr]` therefore emits one Unicode character
            // per sendLine, including a literal newline character.
            raw.chars()
                .map(|character| Dynamic::from(character.to_string()))
                .collect::<rhai::Array>()
                .into()
        },
    );
    let body_ptr_delete_room_event = body_ptr;
    engine.register_fn(
        "delete_room_target_event",
        move |_ob: &mut rhai::Map, query: &str, event: &str| -> String {
            let body = unsafe { &mut *body_ptr_delete_room_event };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                return "missing".into();
            };

            if let Some(selected) = select_python_room_object(body, query) {
                let delete_raw = |raw: &str| {
                    if event.is_empty() || !raw.contains(event) {
                        None
                    } else {
                        Some(
                            raw.split('\n')
                                .filter(|item| *item != event)
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                    }
                };
                match selected {
                    RoomObjectRef::Player(name) => {
                        let raw = if name == body.get_name() {
                            body.get_string("이벤트설정리스트")
                        } else {
                            room_view_player_snapshots(&pos.zone, &pos.room)
                                .into_iter()
                                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                                .find(|player| {
                                    player
                                        .get("이름")
                                        .and_then(|value| value.clone().into_string().ok())
                                        .as_deref()
                                        == Some(name.as_str())
                                })
                                .and_then(|player| player.get("이벤트설정리스트").cloned())
                                .and_then(|value| value.into_string().ok())
                                .unwrap_or_default()
                        };
                        let Some(updated) = delete_raw(&raw) else {
                            return "not_set".into();
                        };
                        if name == body.get_name() {
                            body.set("이벤트설정리스트", updated);
                        } else {
                            body.temp_mut().insert(
                                ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
                                Value::String(
                                    serde_json::to_string(&(
                                        name,
                                        "이벤트설정리스트".to_string(),
                                        serde_json::Value::String(updated),
                                    ))
                                    .unwrap_or_default(),
                                ),
                            );
                        }
                        return "deleted".into();
                    }
                    RoomObjectRef::Mob(id) => {
                        let key = get_world_state().read().ok().and_then(|world| {
                            world
                                .mob_cache
                                .get_all_mobs_in_room(&pos.zone, &pos.room)
                                .into_iter()
                                .find(|mob| mob.instance_id == id)
                                .map(|mob| mob.mob_key.clone())
                        });
                        let Some(key) = key else {
                            return "missing".into();
                        };
                        let mut world = get_world_state().write().unwrap();
                        if !world.mob_cache.check_mob_event(&key, event) {
                            return "not_set".into();
                        }
                        world.mob_cache.del_mob_event(&key, event);
                        return "deleted".into();
                    }
                    RoomObjectRef::SummonedUser(id) => {
                        let mut world = get_world_state().write().unwrap();
                        let Some(user) = world.summoned_user_mut(id) else {
                            return "missing".into();
                        };
                        let raw = user.body.get_string("이벤트설정리스트");
                        let Some(updated) = delete_raw(&raw) else {
                            return "not_set".into();
                        };
                        user.body.set("이벤트설정리스트", updated);
                        return "deleted".into();
                    }
                    RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                        let floor = get_world_state()
                            .read()
                            .ok()
                            .map(|world| world.get_room_objs(&pos.zone, &pos.room))
                            .unwrap_or_default();
                        let installed =
                            box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                                .unwrap_or_default();
                        let object = floor
                            .into_iter()
                            .chain(installed)
                            .find(|object| Arc::as_ptr(object) as usize == pointer);
                        let Some(object) = object else {
                            return "missing".into();
                        };
                        let Ok(mut object) = object.lock() else {
                            return "missing".into();
                        };
                        let raw = object.getString("이벤트설정리스트");
                        let Some(updated) = delete_raw(&raw) else {
                            return "not_set".into();
                        };
                        object.set("이벤트설정리스트", updated);
                        return "deleted".into();
                    }
                    RoomObjectRef::InstalledBox(ordinal) => {
                        let object = box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                            .and_then(|boxes| boxes.get(ordinal).cloned());
                        let Some(object) = object else {
                            return "missing".into();
                        };
                        let Ok(mut object) = object.lock() else {
                            return "missing".into();
                        };
                        let raw = object.getString("이벤트설정리스트");
                        let Some(updated) = delete_raw(&raw) else {
                            return "not_set".into();
                        };
                        object.set("이벤트설정리스트", updated);
                        return "deleted".into();
                    }
                    RoomObjectRef::Fixture(_) => return "missing".into(),
                }
            }

            // Python Room.findObjName walks the unified room object list.  Pick
            // the player or mob in that same insertion order instead of giving
            // either type an artificial priority.
            enum EventTarget {
                Player(String, String),
                Mob(String),
            }
            let player_snapshots = room_view_player_snapshots(&pos.zone, &pos.room)
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .collect::<Vec<_>>();
            let target = {
                let Ok(world) = get_world_state().read() else {
                    return "missing".into();
                };
                let mobs = world.mob_cache.get_all_mobs_in_room(&pos.zone, &pos.room);
                let ordered = world
                    .get_room_object_order(&pos.zone, &pos.room)
                    .into_iter()
                    .find_map(|entry| match entry {
                        RoomObjectRef::Player(name) => player_snapshots
                            .iter()
                            .find(|player| {
                                player
                                    .get("이름")
                                    .and_then(|value| value.clone().into_string().ok())
                                    .is_some_and(|candidate| candidate == name)
                            })
                            .and_then(|player| {
                                let reactions = player
                                    .get("반응이름")
                                    .and_then(|value| value.clone().into_string().ok())
                                    .unwrap_or_default();
                                (name == query || reactions.contains(query)).then(|| {
                                    let events = player
                                        .get("이벤트설정리스트")
                                        .and_then(|value| value.clone().into_string().ok())
                                        .unwrap_or_default();
                                    EventTarget::Player(name, events)
                                })
                            }),
                        RoomObjectRef::Mob(id) => mobs
                            .iter()
                            .find(|mob| mob.instance_id == id)
                            .and_then(|mob| {
                                let data = world.mob_cache.get_mob(&mob.mob_key)?;
                                (data.name == query
                                    || data.reaction_names.iter().any(|name| name.contains(query)))
                                .then(|| EventTarget::Mob(mob.mob_key.clone()))
                            }),
                        _ => None,
                    });
                ordered.or_else(|| {
                    // Legacy/test caches may predate the unified order index.
                    // Python still sees those mobs through room.objs, so keep
                    // cache order as the compatibility fallback.
                    mobs.iter().find_map(|mob| {
                        let data = world.mob_cache.get_mob(&mob.mob_key)?;
                        (data.name == query
                            || data.reaction_names.iter().any(|name| name.contains(query)))
                        .then(|| EventTarget::Mob(mob.mob_key.clone()))
                    })
                })
            };

            match target {
                Some(EventTarget::Player(name, raw)) => {
                    // Object.checkAttr uses substring membership, while
                    // Object.delAttr removes only an exact list element.
                    if event.is_empty() || !raw.contains(event) {
                        return "not_set".into();
                    }
                    let updated = raw
                        .split('\n')
                        .filter(|item| *item != event)
                        .collect::<Vec<_>>()
                        .join("\n");
                    if name == body.get_name() {
                        body.set("이벤트설정리스트", updated);
                    } else {
                        body.temp_mut().insert(
                            ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
                            Value::String(
                                serde_json::to_string(&(
                                    name,
                                    "이벤트설정리스트".to_string(),
                                    serde_json::Value::String(updated),
                                ))
                                .unwrap_or_default(),
                            ),
                        );
                    }
                    "deleted".into()
                }
                Some(EventTarget::Mob(key)) => {
                    let mut world = get_world_state().write().unwrap();
                    if !world.mob_cache.check_mob_event(&key, event) {
                        "not_set".into()
                    } else {
                        world.mob_cache.del_mob_event(&key, event);
                        "deleted".into()
                    }
                }
                None => "missing".into(),
            }
        },
    );
    let body_ptr_set_room_event = body_ptr;
    engine.register_fn(
        "set_room_target_event",
        move |_ob: &mut rhai::Map, query: &str, event: &str| -> String {
            let body = unsafe { &mut *body_ptr_set_room_event };
            let Some(pos) = get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&body.get_name()).cloned())
            else {
                return "missing".into();
            };
            if let Some(selected) = select_python_room_object(body, query) {
                let update_raw = |raw: &str| {
                    if event.is_empty() || raw.contains(event) {
                        None
                    } else if raw.is_empty() {
                        Some(event.to_string())
                    } else {
                        Some(format!("{raw}\n{event}"))
                    }
                };
                match selected {
                    RoomObjectRef::Player(name) => {
                        let raw = if name == body.get_name() {
                            body.get_string("이벤트설정리스트")
                        } else {
                            room_view_player_snapshots(&pos.zone, &pos.room)
                                .into_iter()
                                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                                .find(|player| {
                                    player
                                        .get("이름")
                                        .and_then(|value| value.clone().into_string().ok())
                                        .as_deref()
                                        == Some(name.as_str())
                                })
                                .and_then(|player| player.get("이벤트설정리스트").cloned())
                                .and_then(|value| value.into_string().ok())
                                .unwrap_or_default()
                        };
                        let Some(updated) = update_raw(&raw) else {
                            return "already_set".into();
                        };
                        if name == body.get_name() {
                            body.set("이벤트설정리스트", updated);
                        } else {
                            body.temp_mut().insert(
                                ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
                                Value::String(
                                    serde_json::to_string(&(
                                        name,
                                        "이벤트설정리스트".to_string(),
                                        serde_json::Value::String(updated),
                                    ))
                                    .unwrap_or_default(),
                                ),
                            );
                        }
                        return "set".into();
                    }
                    RoomObjectRef::Mob(id) => {
                        let key = get_world_state().read().ok().and_then(|world| {
                            world
                                .mob_cache
                                .get_all_mobs_in_room(&pos.zone, &pos.room)
                                .into_iter()
                                .find(|mob| mob.instance_id == id)
                                .map(|mob| mob.mob_key.clone())
                        });
                        let Some(key) = key else {
                            return "missing".into();
                        };
                        let mut world = get_world_state().write().unwrap();
                        if world.mob_cache.check_mob_event(&key, event) {
                            return "already_set".into();
                        }
                        world.mob_cache.set_mob_event(&key, event);
                        return "set".into();
                    }
                    RoomObjectRef::SummonedUser(id) => {
                        let mut world = get_world_state().write().unwrap();
                        let Some(user) = world.summoned_user_mut(id) else {
                            return "missing".into();
                        };
                        let raw = user.body.get_string("이벤트설정리스트");
                        let Some(updated) = update_raw(&raw) else {
                            return "already_set".into();
                        };
                        user.body.set("이벤트설정리스트", updated);
                        return "set".into();
                    }
                    RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer) => {
                        let world = get_world_state().read().unwrap();
                        let floor = world.get_room_objs(&pos.zone, &pos.room);
                        drop(world);
                        let installed =
                            box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                                .unwrap_or_default();
                        let object = floor
                            .into_iter()
                            .chain(installed)
                            .find(|object| Arc::as_ptr(object) as usize == pointer);
                        let Some(object) = object else {
                            return "missing".into();
                        };
                        let Ok(mut object) = object.lock() else {
                            return "missing".into();
                        };
                        let raw = object.getString("이벤트설정리스트");
                        let Some(updated) = update_raw(&raw) else {
                            return "already_set".into();
                        };
                        object.set("이벤트설정리스트", updated);
                        return "set".into();
                    }
                    RoomObjectRef::InstalledBox(ordinal) => {
                        let object = box_commands::installed_boxes_for_room(&pos.zone, &pos.room)
                            .and_then(|boxes| boxes.get(ordinal).cloned());
                        let Some(object) = object else {
                            return "missing".into();
                        };
                        let Ok(mut object) = object.lock() else {
                            return "missing".into();
                        };
                        let raw = object.getString("이벤트설정리스트");
                        let Some(updated) = update_raw(&raw) else {
                            return "already_set".into();
                        };
                        object.set("이벤트설정리스트", updated);
                        return "set".into();
                    }
                    RoomObjectRef::Fixture(_) => return "missing".into(),
                }
            }
            enum EventTarget {
                Player(String, String),
                Mob(String),
            }
            let player_snapshots = room_view_player_snapshots(&pos.zone, &pos.room)
                .into_iter()
                .filter_map(|entry| entry.try_cast::<rhai::Map>())
                .collect::<Vec<_>>();
            let target = {
                let Ok(world) = get_world_state().read() else {
                    return "missing".into();
                };
                let mobs = world.mob_cache.get_all_mobs_in_room(&pos.zone, &pos.room);
                let ordered = world
                    .get_room_object_order(&pos.zone, &pos.room)
                    .into_iter()
                    .find_map(|entry| match entry {
                        RoomObjectRef::Player(name) => player_snapshots
                            .iter()
                            .find(|player| {
                                player
                                    .get("이름")
                                    .and_then(|value| value.clone().into_string().ok())
                                    .is_some_and(|candidate| candidate == name)
                            })
                            .and_then(|player| {
                                let reactions = player
                                    .get("반응이름")
                                    .and_then(|value| value.clone().into_string().ok())
                                    .unwrap_or_default();
                                (name == query || reactions.contains(query)).then(|| {
                                    let events = player
                                        .get("이벤트설정리스트")
                                        .and_then(|value| value.clone().into_string().ok())
                                        .unwrap_or_default();
                                    EventTarget::Player(name, events)
                                })
                            }),
                        RoomObjectRef::Mob(id) => mobs
                            .iter()
                            .find(|mob| mob.instance_id == id)
                            .and_then(|mob| {
                                let data = world.mob_cache.get_mob(&mob.mob_key)?;
                                (data.name == query
                                    || data.reaction_names.iter().any(|name| name.contains(query)))
                                .then(|| EventTarget::Mob(mob.mob_key.clone()))
                            }),
                        _ => None,
                    });
                ordered.or_else(|| {
                    mobs.iter().find_map(|mob| {
                        let data = world.mob_cache.get_mob(&mob.mob_key)?;
                        (data.name == query
                            || data.reaction_names.iter().any(|name| name.contains(query)))
                        .then(|| EventTarget::Mob(mob.mob_key.clone()))
                    })
                })
            };
            match target {
                Some(EventTarget::Player(name, raw)) => {
                    if event.is_empty() || raw.contains(event) {
                        return "already_set".into();
                    }
                    let updated = if raw.is_empty() {
                        event.to_string()
                    } else {
                        format!("{raw}\n{event}")
                    };
                    if name == body.get_name() {
                        body.set("이벤트설정리스트", updated);
                    } else {
                        body.temp_mut().insert(
                            ADMIN_SET_PLAYER_VALUE_REQUEST.to_string(),
                            Value::String(
                                serde_json::to_string(&(
                                    name,
                                    "이벤트설정리스트".to_string(),
                                    serde_json::Value::String(updated),
                                ))
                                .unwrap_or_default(),
                            ),
                        );
                    }
                    "set".into()
                }
                Some(EventTarget::Mob(key)) => {
                    let mut world = get_world_state().write().unwrap();
                    if world.mob_cache.check_mob_event(&key, event) {
                        "already_set".into()
                    } else {
                        world.mob_cache.set_mob_event(&key, event);
                        "set".into()
                    }
                }
                None => "missing".into(),
            }
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
            if target.is_empty() {
                if let Ok(w) = get_world_state().read() {
                    if let Some(pos) = w.get_player_position(&name) {
                        let path = format!("data/map/{}/{}/", pos.zone, pos.room);
                        let path = path.trim_end_matches('/').to_string() + ".json";
                        if let Ok(raw) = std::fs::read_to_string(path) {
                            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&raw) {
                                if let Some(info) = root.get("맵정보").and_then(|v| v.as_object())
                                {
                                    found = true;
                                    attrs.extend(
                                        info.iter()
                                            .map(|(key, value)| (key.clone(), json_text(value))),
                                    );
                                }
                            }
                        }
                        let room_key = format!("{}:{}", pos.zone, pos.room);
                        if let Some(room) = w.room_attrs.get(&room_key) {
                            found = true;
                            for (key, value) in room {
                                if let Some(existing) =
                                    attrs.iter_mut().find(|(name, _)| name == key)
                                {
                                    existing.1 = value.clone();
                                } else {
                                    attrs.push((key.clone(), value.clone()));
                                }
                            }
                        }
                    }
                }
            } else if target == name {
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
                // Room.findObjName()은 공백이 든 입력에서 첫 토큰만 사용한다.
                // 실패 뒤의 ob.findObjName()에는 원문 전체가 전달된다.
                let room_target = target.split_whitespace().next().unwrap_or(target);
                let position = current_body_position(body);
                if let Some((zone, room)) = position.as_ref() {
                    let ordered_target = select_python_room_object(body, target);
                    if let Some(selected) = ordered_target.as_ref() {
                        match selected {
                            RoomObjectRef::Player(selected_name)
                                if selected_name == &body.get_name() =>
                            {
                                found = true;
                                attrs.extend(body.object.attr.iter().map(|(key, value)| {
                                    let value = match value {
                                        Value::Int(value) => value.to_string(),
                                        Value::Float(value) => value.to_string(),
                                        Value::String(value) => value.clone(),
                                    };
                                    (key.clone(), value)
                                }));
                            }
                            RoomObjectRef::Player(selected_name) => {
                                if let Some(raw) = room_admin_player_values(body)
                                    .and_then(|players| {
                                        players.into_iter().find(|player| {
                                            player.get("name").and_then(|value| value.as_str())
                                                == Some(selected_name.as_str())
                                        })
                                    })
                                    .and_then(|player| player.get("raw_attrs").cloned())
                                    .and_then(|attrs| attrs.as_object().cloned())
                                {
                                    found = true;
                                    attrs.extend(
                                        raw.iter()
                                            .map(|(key, value)| (key.clone(), json_text(value))),
                                    );
                                }
                            }
                            RoomObjectRef::SummonedUser(id) => {
                                if let Ok(world) = get_world_state().read() {
                                    if let Some(user) =
                                        world.summoned_users().iter().find(|user| user.id == *id)
                                    {
                                        found = true;
                                        attrs.extend(user.body.object.attr.iter().map(
                                            |(key, value)| {
                                                let value = match value {
                                                    Value::Int(value) => value.to_string(),
                                                    Value::Float(value) => value.to_string(),
                                                    Value::String(value) => value.clone(),
                                                };
                                                (key.clone(), value)
                                            },
                                        ));
                                    }
                                }
                            }
                            RoomObjectRef::Box(pointer) | RoomObjectRef::InstalledBox(pointer) => {
                                let candidates = box_commands::installed_boxes_for_room(zone, room)
                                    .unwrap_or_default();
                                let object = if matches!(selected, RoomObjectRef::Box(_)) {
                                    candidates
                                        .into_iter()
                                        .find(|object| Arc::as_ptr(object) as usize == *pointer)
                                } else {
                                    candidates.get(*pointer).cloned()
                                };
                                if let Some(object) = object.and_then(|object| {
                                    object.lock().ok().map(|guard| {
                                        guard
                                            .attr
                                            .iter()
                                            .map(|(key, value)| {
                                                let value = match value {
                                                    Value::Int(value) => value.to_string(),
                                                    Value::Float(value) => value.to_string(),
                                                    Value::String(value) => value.clone(),
                                                };
                                                (key.clone(), value)
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                }) {
                                    found = true;
                                    attrs.extend(object);
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Ok(order) = room_target.parse::<usize>() {
                        if order > 0 {
                            if let Ok(world) = get_world_state().read() {
                                if let Some(mob) = world
                                    .mob_cache
                                    .get_all_mobs_in_room(zone, room)
                                    .into_iter()
                                    .filter(|mob| {
                                        mob.mob_type != 7
                                            && mob.alive
                                            && mob.act != 2
                                            && mob.act != 3
                                    })
                                    .nth(order - 1)
                                {
                                    if let Some(data) = world.get_mob_data(&mob.mob_key) {
                                        found = true;
                                        attrs.extend(
                                            data.attributes.iter().map(|(key, value)| {
                                                (key.clone(), json_text(value))
                                            }),
                                        );
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
                        }
                    }
                    if !found {
                        let floor = get_world_state()
                            .read()
                            .ok()
                            .map(|world| world.get_room_objs(zone, room).to_vec())
                            .unwrap_or_default();
                        for arc in floor {
                            let selected_by_identity =
                                ordered_target.as_ref().is_some_and(|selected| {
                                    matches!(selected,
                                    RoomObjectRef::FloorItem(pointer) | RoomObjectRef::Box(pointer)
                                    if Arc::as_ptr(&arc) as usize == *pointer)
                                });
                            if let Some(selected) = ordered_target.as_ref() {
                                match selected {
                                    RoomObjectRef::FloorItem(pointer)
                                    | RoomObjectRef::Box(pointer)
                                        if Arc::as_ptr(&arc) as usize == *pointer => {}
                                    _ => continue,
                                }
                            }
                            let Ok(obj) = arc.lock() else { continue };
                            if selected_by_identity
                                || obj.getName() == room_target
                                || reaction_names(&obj.getString("반응이름"))
                                    .iter()
                                    .any(|alias| {
                                        alias == room_target || alias.starts_with(room_target)
                                    })
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
                    }
                    if !found {
                        if let Ok(world) = get_world_state().read() {
                            if let Some((mob, data)) = world
                                .mob_cache
                                .get_all_mobs_in_room(zone, room)
                                .into_iter()
                                .find_map(|mob| {
                                    let selected_by_identity = ordered_target.as_ref()
                                        == Some(&RoomObjectRef::Mob(mob.instance_id));
                                    if ordered_target.is_some() {
                                        if !selected_by_identity {
                                            return None;
                                        }
                                    }
                                    let data = world.get_mob_data(&mob.mob_key)?;
                                    (selected_by_identity
                                        || mob.name == room_target
                                        || data.name == room_target
                                        || data.reaction_names.iter().any(|alias| {
                                            alias == room_target || alias.starts_with(room_target)
                                        }))
                                    .then_some((mob, data))
                                })
                            {
                                found = true;
                                attrs.extend(
                                    data.attributes
                                        .iter()
                                        .map(|(key, value)| (key.clone(), json_text(value))),
                                );
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
                                    if let Some(RoomObjectRef::Player(selected)) =
                                        ordered_target.as_ref()
                                    {
                                        return player.as_str() == selected.as_str();
                                    }
                                    ordered_target.is_none()
                                        && (player.as_str() == room_target
                                            || player.starts_with(room_target))
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
                                        attrs.extend(
                                            values.iter().map(|(key, value)| {
                                                (key.clone(), json_text(value))
                                            }),
                                        );
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
                            if obj.checkAttr("아이템속성", "출력안함") {
                                continue;
                            }
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
                if !found {
                    if let Some(key) =
                        inventory_compat::find_counted_item_key(&body.object.inv_stack, target)
                    {
                        if body.object.inv_stack.get(&key).copied().unwrap_or(0) > 0 {
                            if let Some((item, _)) = object_from_item_json(&key) {
                                if let Ok(item) = item.lock() {
                                    if !item.checkAttr("아이템속성", "출력안함") {
                                        found = true;
                                        attrs.extend(item.attr.iter().map(|(key, value)| {
                                            let value = match value {
                                                Value::Int(value) => value.to_string(),
                                                Value::Float(value) => value.to_string(),
                                                Value::String(value) => value.clone(),
                                            };
                                            (key.clone(), value)
                                        }));
                                    }
                                }
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
            let Ok(world) = get_world_state().read() else {
                return result;
            };
            for (zone, room_id) in world.room_cache.loaded_rooms_in_python_zone_order() {
                let Some(room_arc) = world.room_cache.get_room_cached(&zone, &room_id) else {
                    continue;
                };
                let Ok(room) = room_arc.read() else { continue };
                let room_objects = world.get_room_objs(&zone, &room_id);
                let room_mobs = world.mob_cache.get_all_mobs_in_room(&zone, &room_id);
                let installed =
                    box_commands::installed_boxes_for_room(&zone, &room_id).unwrap_or_default();
                let mut matches = 0usize;
                for object in world.get_room_object_order(&zone, &room_id) {
                    let name = match object {
                        crate::world::RoomObjectRef::Player(name) => Some(name),
                        crate::world::RoomObjectRef::SummonedUser(id) => {
                            world.get_summoned_user_name(id)
                        }
                        crate::world::RoomObjectRef::Mob(id) => room_mobs
                            .iter()
                            .find(|mob| mob.instance_id == id)
                            .map(|mob| mob.name.clone()),
                        crate::world::RoomObjectRef::FloorItem(pointer)
                        | crate::world::RoomObjectRef::Box(pointer) => room_objects
                            .iter()
                            .find(|object| Arc::as_ptr(object) as usize == pointer)
                            .and_then(|object| object.lock().ok().map(|object| object.getName())),
                        crate::world::RoomObjectRef::InstalledBox(ordinal) => installed
                            .get(ordinal)
                            .and_then(|object| object.lock().ok().map(|object| object.getName())),
                        crate::world::RoomObjectRef::Fixture(id) => world
                            .get_fixture(id)
                            .map(|fixture| fixture.name().to_string()),
                    };
                    if name.as_deref() == Some(wanted) {
                        matches += 1;
                    }
                }
                for _ in 0..matches {
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
            destroy_inventory_for_command(body, item_name, 1, count, false)
                .get("count")
                .and_then(|value| value.as_int().ok())
                .unwrap_or(0)
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

            // Python inventory order is represented by stateful objects first
            // and pristine counted copies after them. Preserve a changed or
            // UUID-bearing first occurrence when handing one item to a mob.
            let selected_object = body.object.objs.iter().find_map(|obj| {
                let object = obj.lock().ok()?;
                let matches = object.getName() == item_name
                    || inventory_compat::python_item_field_contains(&object, "반응이름", item_name);
                (matches && !object.getBool("inUse")).then(|| obj.clone())
            });

            let given = if let Some(item) = selected_object {
                if let Some(mob) = w
                    .mob_cache
                    .get_all_mobs_in_room_mut(&zone, &room)
                    .and_then(|mobs| mobs.iter_mut().find(|m| m.mob_key == mob_key))
                {
                    body.object.remove(&item);
                    mob.inventory.push(item);
                    true
                } else {
                    false
                }
            } else if let Some(key) =
                inventory_compat::find_counted_item_key(&body.object.inv_stack, item_name)
            {
                let item = object_from_item_json(&key).map(|(item, _)| item);
                if let Some(item) = item {
                    if inventory_compat::remove_pristine_count(&mut body.object, &key, 1) {
                        if let Some(mob) = w
                            .mob_cache
                            .get_all_mobs_in_room_mut(&zone, &room)
                            .and_then(|mobs| mobs.iter_mut().find(|m| m.mob_key == mob_key))
                        {
                            mob.inventory.push(item);
                            true
                        } else {
                            inventory_compat::add_pristine_count(&mut body.object, &key, 1);
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

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
            let target_pos = w.get_player_position(target_name).cloned().or_else(|| {
                w.summoned_users()
                    .iter()
                    .find(|user| user.body.get_name() == target_name)
                    .map(|user| user.position.clone())
            });
            let Some(target_pos) = target_pos else {
                return "대상을 찾을 수 없습니다.".to_string();
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
            requests.push((target_name.to_string(), admin_pos.zone, admin_pos.room));
            if let Ok(request) = serde_json::to_string(&requests) {
                body.temp_mut()
                    .insert(SUMMON_PLAYER_REQUEST.to_string(), Value::String(request));
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
                return "denied".to_string();
            }

            let body = unsafe { &*body_ptr_kick };
            let admin_name = body.get_name();

            // 빈 대상 이름 체크
            if target_name.trim().is_empty() {
                return "missing_target".to_string();
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
            if !load_body_from_json(&mut summoned, &format!("data/user/{}.json", mob_name)) {
                return "존재하지않는 사용자입니다.".to_string();
            }
            summoned.act = crate::player::ActState::Stand;
            let loaded_name = summoned.get_name();
            get_world_state()
                .write()
                .unwrap()
                .add_summoned_user(summoned, crate::world::PlayerPosition::new(zone, room));
            loaded_name
        },
    );

    let body_ptr_remove_room_user = body_ptr;
    engine.register_fn(
        "remove_room_user_mob",
        move |_admin_ob: &mut rhai::Map, query: &str| -> bool {
            let body = unsafe { &*body_ptr_remove_room_user };
            let Some(crate::world::RoomObjectRef::SummonedUser(id)) =
                select_python_room_object(body, query)
            else {
                return false;
            };
            get_world_state()
                .write()
                .map(|mut world| world.remove_summoned_user_by_id(id))
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
            if admin_combat::python_named_room_selection_is_nonmob(&world, &zone, &room, mob_name) {
                return false;
            }
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
            let instance_id = world
                .mob_cache
                .get_all_mobs_in_room(&zone, &room)
                .get(index)
                .map(|mob| mob.instance_id);
            instance_id.is_some_and(|id| world.remove_room_mob_instance(&zone, &room, id))
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
                return "missing_command".to_string();
            }

            // 플레이어가 접속 중인지 확인
            let online = if let Ok(w) = get_world_state().try_read() {
                w.get_player_position(target_name).is_some()
            } else {
                return "unavailable".to_string();
            };

            if !online {
                return "offline".to_string();
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
                return "queue_failed".to_string();
            };
            body.temp_mut()
                .insert(FORCE_COMMAND_REQUEST.to_string(), Value::String(request));
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
            if !name.is_empty() && !command.is_empty() {
                if let Ok(mut sends) = user_sends_delayed.lock() {
                    sends.push((name, command.to_string()));
                }
            }
        },
    );

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
            let mut total = 0i64;
            let mut to_remove: Vec<Arc<Mutex<Object>>> = Vec::new();

            // Stateful objects precede pristine counts in the runtime
            // inventory order. A bulk legacy sale may continue into counts.
            for obj in &body.object.objs {
                if let Ok(o) = obj.lock() {
                    let nm = o.getName();
                    let match_ = nm == item_name
                        || inventory_compat::python_item_field_contains(&o, "반응이름", item_name);
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

            let mut stack_removals = Vec::<(String, i64)>::new();
            let mut remaining = count.saturating_sub(to_remove.len()) as i64;
            if remaining > 0 {
                for key in inventory_compat::counted_item_keys(&body.object.inv_stack, item_name) {
                    let have = body.object.inv_stack.get(&key).copied().unwrap_or(0).max(0);
                    if have == 0 {
                        continue;
                    }
                    let Some((template, _)) = object_from_item_json(&key) else {
                        continue;
                    };
                    let Ok(template) = template.lock() else {
                        continue;
                    };
                    if template.checkAttr("아이템속성", "팔지못함") {
                        if to_remove.is_empty() && stack_removals.is_empty() {
                            return "cant_sell".to_string();
                        }
                        continue;
                    }
                    if template.checkAttr("아이템속성", "출력안함") {
                        continue;
                    }
                    let sold = remaining.min(have);
                    if sold > 0 {
                        total += (template.getInt("판매가격") * buy_percent / 100) * sold;
                        stack_removals.push((key, sold));
                        remaining -= sold;
                    }
                    if remaining == 0 {
                        break;
                    }
                }
            }

            if to_remove.is_empty() && stack_removals.is_empty() {
                return "no_item".to_string();
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
            for (key, sold) in stack_removals {
                let _ = inventory_compat::remove_pristine_count(&mut body.object, &key, sold);
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
                .split(['\r', '\n', ',', '|'])
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
            let Some(index) = names.iter().position(|name| name == member) else {
                return "not_requested".into();
            };
            let guild = body.get_string("소속");
            if guild.is_empty() || !crate::world::guild::guild_has(&guild) {
                return "failed".into();
            }
            let already_member = ["방주", "부방주", "장로", "방파인"].iter().any(|role| {
                crate::world::guild::guild_role_members(&guild, role)
                    .iter()
                    .any(|name| name == member)
            });
            if !already_member && !crate::world::guild::guild_add_member(&guild, "방파인", member)
            {
                return "failed".into();
            }
            // Consume the application only after the guild mutation is known
            // to have succeeded. Python can append duplicates here; Rust keeps
            // the repaired invariant while preserving the successful command.
            names.remove(index);
            body.set("입문신청자", names.join("\r\n"));
            body.temp_mut().insert(
                GUILD_ACCEPT_REQUEST.to_string(),
                Value::String(
                    serde_json::to_string(&(member.to_string(), guild)).unwrap_or_default(),
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
            let request =
                serde_json::to_string(&(target.to_string(), applicant)).unwrap_or_default();
            body.temp_mut()
                .insert(GUILD_APPLY_REQUEST.to_string(), Value::String(request));
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
            if attrs.get("소속").and_then(|v| v.as_str()).unwrap_or("") != guild {
                return "wrong_guild".into();
            }
            attrs.insert(
                "방파별호".into(),
                serde_json::Value::String(nickname.to_string()),
            );
            if let Ok(saved) = serde_json::to_string_pretty(&json) {
                if std::fs::write(path, saved).is_ok() {
                    if let Ok(request) =
                        serde_json::to_string(&(member.to_string(), nickname.to_string()))
                    {
                        body.temp_mut()
                            .insert(GUILD_NICKNAME_REQUEST.to_string(), Value::String(request));
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
                            attrs.insert("소속".into(), serde_json::Value::String(String::new()));
                            attrs.insert("직위".into(), serde_json::Value::String(String::new()));
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
                if !crate::world::guild::guild_transfer_leader(&guild, &body.get_name(), target) {
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
            if !crate::world::guild::guild_transfer_leader(&guild, &body.get_name(), target) {
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
                if let Ok(request) =
                    serde_json::to_string(&(member_name.to_string(), position.to_string()))
                {
                    body.temp_mut()
                        .insert(GUILD_POSITION_REQUEST.to_string(), Value::String(request));
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
        if guild_name.is_empty() || !crate::world::guild::guild_has(&guild_name) {
            return Dynamic::UNIT;
        }

        let mut info = rhai::Map::new();
        info.insert("이름".into(), Dynamic::from(guild_name.clone()));

        // Guild 모듈에서 정보 가져오기
        let leader = crate::world::guild::guild_get(&guild_name, "방주이름");
        info.insert("방주".into(), Dynamic::from(leader));
        for (out_key, role) in [("부방주", "부방주"), ("장로", "장로"), ("방파인", "방파인")]
        {
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

    // Keep this standalone global-data engine consistent with the command
    // engine: `get_skill_data` must observe the in-memory skill snapshot.
    register_cached_skill_data_efun(&mut engine, global_data.clone());
    register_cached_murim_config_efun(&mut engine, global_data.clone());

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

/// Register the cached `skill.json` projection for engines that own
/// `GlobalData`.  Engines without GlobalData retain the file-backed fallback
/// for isolated tools and tests.
fn register_cached_skill_data_efun(engine: &mut Engine, global_data: SharedGlobalData) {
    engine.register_fn("get_skill_data", move |name: &str| -> Dynamic {
        global_data
            .try_read()
            .ok()
            .and_then(|data| data.get_skill(name).map(crate::data::json_to_dynamic))
            .unwrap_or(Dynamic::UNIT)
    });
}

/// Register the cached `murim.json` main-configuration projection. Engines
/// without GlobalData retain the file-backed fallback used by isolated tools.
fn register_cached_murim_config_efun(engine: &mut Engine, global_data: SharedGlobalData) {
    engine.register_fn("get_murim_config", move |key: &str| -> Dynamic {
        global_data
            .try_read()
            .ok()
            .and_then(|data| data.get_murim_config(key).map(crate::data::json_to_dynamic))
            .unwrap_or(Dynamic::UNIT)
    });
}

fn python_json_repr(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(value) => if *value { "True" } else { "False" }.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => {
            format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
        }
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

/// Script storage - stores source metadata and compiled Rhai ASTs.
pub struct ScriptStorage {
    scripts: HashMap<String, StoredScript>,
    /// Library scripts loaded from lib/ directory (hot-reloadable)
    libraries: HashMap<String, String>,
    /// Modification times used to detect added, removed, and changed libraries.
    library_modified: HashMap<String, std::time::SystemTime>,
    /// Libraries are combined only when a library file changes.
    library_source: String,
    config: ScriptConfig,
    /// 글로벌 데이터 캐시 참조
    global_data: Option<SharedGlobalData>,
}

static NEXT_SCRIPT_REVISION: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Rhai without its `sync` feature uses `Rc` inside AST, so an AST must not
    /// be shared across Tokio worker threads. Each worker keeps a compiled AST
    /// per script revision; ordinary executions on that worker only evaluate.
    static SCRIPT_AST_CACHE: RefCell<HashMap<(String, u64), (AST, AST)>> = RefCell::new(HashMap::new());
}

impl ScriptStorage {
    fn compile_script_asts(
        name: &str,
        source: &str,
        library_source: &str,
    ) -> Result<(AST, AST), Box<dyn std::error::Error>> {
        let engine = Engine::new();
        let ast = engine
            .compile(source)
            .map_err(|error| format!("compile {name}: {error}"))?;
        let command_source = format!("{library_source}\n{source}\nmain(ob, cmdline)");
        let command_ast = engine
            .compile(&command_source)
            .map_err(|error| format!("compile {name} with libraries: {error}"))?;
        Ok((ast, command_ast))
    }

    fn next_revision() -> u64 {
        NEXT_SCRIPT_REVISION.fetch_add(1, Ordering::Relaxed)
    }

    fn cache_asts(name: &str, revision: u64, asts: (AST, AST)) {
        SCRIPT_AST_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.retain(|(cached_name, _), _| cached_name != name);
            cache.insert((name.to_string(), revision), asts);
        });
    }

    fn cached_asts(
        &self,
        name: &str,
        script: &StoredScript,
    ) -> Result<(AST, AST), Box<dyn std::error::Error>> {
        let key = (name.to_string(), script.revision);
        if let Some(asts) = SCRIPT_AST_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
            return Ok(asts);
        }
        let compile_started = Instant::now();
        let asts = Self::compile_script_asts(name, &script.source, &self.library_source)?;
        Self::cache_asts(name, script.revision, asts.clone());
        tracing::debug!(
            script = name,
            revision = script.revision,
            compile_us = compile_started.elapsed().as_micros(),
            "Warmed Rhai AST cache on worker thread"
        );
        Ok(asts)
    }

    fn combined_library_source(libraries: &HashMap<String, String>) -> String {
        let mut combined = String::new();
        for (name, source) in libraries {
            combined.push_str("// Library: ");
            combined.push_str(name);
            combined.push('\n');
            combined.push_str(source);
            combined.push('\n');
        }
        combined
    }

    fn read_libraries_recursive(
        root: &Path,
        dir: &Path,
        libraries: &mut HashMap<String, String>,
        modified: &mut HashMap<String, std::time::SystemTime>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name == "std" || name == "doumi")
                {
                    continue;
                }
                Self::read_libraries_recursive(root, &path, libraries, modified)?;
                continue;
            }
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rhai") {
                continue;
            }
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            if rel_path.starts_with("std/") || rel_path.starts_with("doumi/") {
                continue;
            }
            let name = rel_path
                .strip_suffix(".rhai")
                .unwrap_or(&rel_path)
                .to_string();
            let metadata = std::fs::metadata(&path)?;
            libraries.insert(name.clone(), std::fs::read_to_string(&path)?);
            modified.insert(name.clone(), metadata.modified()?);
            debug!(library = name, path = %path.display(), "Loaded Rhai library");
        }
        Ok(())
    }

    fn read_library_snapshot(
        &self,
    ) -> Result<
        (
            HashMap<String, String>,
            HashMap<String, std::time::SystemTime>,
        ),
        Box<dyn std::error::Error>,
    > {
        let mut libraries = HashMap::new();
        let mut modified = HashMap::new();
        if self.config.lib_dir.exists() {
            Self::read_libraries_recursive(
                &self.config.lib_dir,
                &self.config.lib_dir,
                &mut libraries,
                &mut modified,
            )?;
        }
        Ok((libraries, modified))
    }

    fn install_library_snapshot(
        &mut self,
        libraries: HashMap<String, String>,
        modified: HashMap<String, std::time::SystemTime>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let library_source = Self::combined_library_source(&libraries);
        let mut compiled = HashMap::with_capacity(self.scripts.len());
        for (name, script) in &self.scripts {
            let asts = Self::compile_script_asts(name, &script.source, &library_source)?;
            compiled.insert(name.clone(), asts);
        }
        for (name, asts) in compiled {
            let revision = Self::next_revision();
            Self::cache_asts(&name, revision, asts);
            if let Some(script) = self.scripts.get_mut(&name) {
                script.revision = revision;
            }
        }
        self.libraries = libraries;
        self.library_modified = modified;
        self.library_source = library_source;
        Ok(())
    }

    pub fn new(config: ScriptConfig) -> Self {
        let mut storage = Self {
            scripts: HashMap::new(),
            libraries: HashMap::new(),
            library_modified: HashMap::new(),
            library_source: String::new(),
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
            library_modified: HashMap::new(),
            library_source: String::new(),
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
            let asts = Self::compile_script_asts(&name, &source, &self.library_source)
                .map_err(|error| format!("{}: {}", path.display(), error))?;
            let revision = Self::next_revision();
            Self::cache_asts(&name, revision, asts);
            let modified = std::fs::metadata(&path)?.modified()?;
            self.scripts.insert(
                name.clone(),
                StoredScript {
                    source,
                    revision,
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

        let (libraries, modified) = self.read_library_snapshot()?;
        self.install_library_snapshot(libraries, modified)?;

        info!(
            "Loaded {} library scripts from {:?}",
            self.libraries.len(),
            dir
        );
        Ok(())
    }

    /// Reload all library scripts from lib/ directory
    pub fn reload_libraries(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let (libraries, modified) = self.read_library_snapshot()?;
        self.install_library_snapshot(libraries, modified)?;
        Ok(self.libraries.len())
    }

    /// Get combined library source code to prepend to scripts
    pub fn get_library_source(&self) -> String {
        self.library_source.clone()
    }

    pub fn load_script(
        &mut self,
        name: &str,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let source = std::fs::read_to_string(path)?;
        let asts = Self::compile_script_asts(name, &source, &self.library_source)?;
        let revision = Self::next_revision();
        Self::cache_asts(name, revision, asts);
        self.scripts.insert(
            name.to_string(),
            StoredScript {
                source,
                revision,
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
        let asts = Self::compile_script_asts(name, &source, &self.library_source)?;
        let revision = Self::next_revision();
        Self::cache_asts(name, revision, asts);
        self.scripts.insert(
            name.to_string(),
            StoredScript {
                source,
                revision,
                modified,
                _name: name.to_string(),
            },
        );

        info!("Reloaded script: {}", name);
        Ok(true)
    }

    pub fn reload_all(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut reloaded = 0;
        let (libraries, modified) = self.read_library_snapshot()?;
        if modified != self.library_modified {
            self.install_library_snapshot(libraries, modified)?;
            reloaded += self.scripts.len();
            info!(
                scripts = self.scripts.len(),
                "Recompiled Rhai ASTs after library change"
            );
        }
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
        let total_started = Instant::now();
        tracing::debug!(script = name, "Executing Rhai script");
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("Script not found: {}", name))?;
        let (_, command_ast) = self.cached_asts(name, script)?;

        let output_collector = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output_collector.clone();
        let special_collector = Arc::new(Mutex::new(None));
        let user_sends = Arc::new(Mutex::new(Vec::new()));

        let engine_started = Instant::now();
        // Native efuns capture this invocation's exclusively borrowed Body.
        // The Engine therefore stays command-local, while a worker-local AST
        // is reused without a process-wide execution mutex.
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
        let engine_setup = engine_started.elapsed();
        let scope_started = Instant::now();
        let mut scope = Scope::new();

        let player_data = build_ob_from_body(player);
        scope.push("player", player_data.clone());
        scope.push("me", player_data.clone());
        scope.push("ob", player_data.clone());
        scope.push("this", player_data); // For std library functions that use 'this'
                                         // Python's Player.parse_command strips surrounding whitespace before
                                         // dispatching the argument to CmdObj.cmd.  Keep direct Rhai execution
                                         // (tests, internal commands, and network commands) on that same
                                         // boundary contract.
        scope.push("cmdline", rhai::Dynamic::from(line.trim().to_string()));

        // DOUMI system global variables for script suspension/resumption
        scope.push("_doumi_resume_op", "" as &str);
        scope.push("_doumi_resume_input", "" as &str);
        let scope_setup = scope_started.elapsed();
        tracing::debug!(script = name, "Running cached Rhai AST");
        let eval_started = Instant::now();
        let result = engine.run_ast_with_scope(&mut scope, &command_ast);
        let eval_elapsed = eval_started.elapsed();
        tracing::debug!(
            script = name,
            success = result.is_ok(),
            engine_setup_us = engine_setup.as_micros(),
            scope_setup_us = scope_setup.as_micros(),
            rhai_and_efun_us = eval_elapsed.as_micros(),
            "Rhai script finished"
        );
        result.map_err(|e| format!("스크립트 실행 오류: {}", e))?;
        let postprocess_started = Instant::now();

        // Affix commands also update their command-local `ob` map. Read that
        // authoritative per-evaluation value back so native pointer context
        // cannot leak across direct Engine runs in another thread.
        if let Some(ob) = scope.get_value::<rhai::Map>("ob") {
            for key in ["머리말", "꼬리말"] {
                if let Some(value) = ob
                    .get(key)
                    .and_then(|value| value.clone().into_string().ok())
                {
                    player
                        .object
                        .attr
                        .insert(key.to_string(), Value::String(value));
                }
            }
        }
        // These four commands have a single, unambiguous state transition.
        // Finalize it from this execute call's own name/input, independent of
        // native-function pointer dispatch used during Rhai evaluation.
        let normalized_line = line.trim();
        match name {
            "머리말" if normalized_line.chars().count() <= 20 && !normalized_line.is_empty() => {
                player.object.attr.insert(
                    "머리말".to_string(),
                    Value::String(normalized_line.to_string()),
                );
            }
            "꼬리말" if normalized_line.chars().count() <= 20 && !normalized_line.is_empty() => {
                player.object.attr.insert(
                    "꼬리말".to_string(),
                    Value::String(normalized_line.to_string()),
                );
            }
            "머리말제거" => player.set("머리말", ""),
            "꼬리말제거" => player.set("꼬리말", ""),
            _ => {}
        }

        let outputs = output_collector.lock().unwrap().clone();
        tracing::debug!(
            script = name,
            outputs = outputs.len(),
            "Collected Rhai output"
        );
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
        tracing::debug!(
            script = name,
            postprocess_us = postprocess_started.elapsed().as_micros(),
            total_us = total_started.elapsed().as_micros(),
            "Completed Rhai command processing"
        );
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
        let (ast, _) = self.cached_asts(name, script)?;
        let engine = create_engine();
        engine.run_ast_with_scope(scope, &ast)?;
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

    /// Get the cached definition AST for call_out and other driver applies.
    pub fn get_script_ast(&self, name: &str) -> Result<Option<AST>, String> {
        let Some(script) = self.scripts.get(name) else {
            return Ok(None);
        };
        self.cached_asts(name, script)
            .map(|(ast, _)| Some(ast))
            .map_err(|error| error.to_string())
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
        let (ast, _) = self
            .cached_asts(name, script)
            .map_err(|error| error.to_string())?;
        let engine = create_engine();
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
        let (ast, _) = self
            .cached_asts(name, script)
            .map_err(|error| error.to_string())?;
        let engine = create_engine();
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
        let (ast, _) = self
            .cached_asts(name, script)
            .map_err(|error| error.to_string())?;
        let engine = create_engine();
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
        let (ast, _) = self
            .cached_asts(name, script)
            .map_err(|error| error.to_string())?;
        let engine = create_engine();
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
        let (ast, _) = self
            .cached_asts(name, script)
            .map_err(|error| error.to_string())?;
        let engine = create_engine();
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
        let (ast, _) = self
            .cached_asts(name, script)
            .map_err(|error| error.to_string())?;
        let engine = create_engine();
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
    // Python 점수/소지 제한은 저장 속성이 아니라 현재 인벤토리 객체를
    // 순회한 getItemWeight()를 사용한다.
    m.insert("소지품무게".into(), body.get_item_weight().into());
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
    create_call_out_script_runner_with_scheduler(script_storage, broadcaster, None)
}

pub fn create_call_out_script_runner_with_scheduler(
    script_storage: Arc<tokio::sync::RwLock<ScriptStorage>>,
    broadcaster: Arc<Broadcaster>,
    scheduler: Option<Arc<CallOutScheduler>>,
) -> Arc<dyn Fn(&str, Option<&str>, &str, Vec<serde_json::Value>) -> Result<(), String> + Send + Sync>
{
    Arc::new(
        move |target: &str, script: Option<&str>, function: &str, args: Vec<serde_json::Value>| {
            let script = script.ok_or_else(|| "call_out: script name required".to_string())?;
            // process_due는 tokio 워커에서 호출되므로 blocking_read 전에 block_in_place로 블로킹 허용
            let (ast, global_data) = tokio::task::block_in_place(|| {
                let storage = script_storage.blocking_read();
                (storage.get_script_ast(script), storage.global_data.clone())
            });
            let ast = ast?.ok_or_else(|| format!("script not found: {}", script))?;

            // 클로저 안에서는 clients 락이 잡혀 있으므로 send_to_by_player_name(→clients.lock()) 호출 금지.
            // 메시지만 수집하고, 락 해제 후 밖에서 전송.
            let target_addr = broadcaster.find_addr_by_connection_token(target);
            let execute_for_body = |body: &mut Body| {
                let output_collector = Arc::new(Mutex::new(Vec::new()));
                let special_collector = Arc::new(Mutex::new(None));
                let user_sends = Arc::new(Mutex::new(Vec::new()));
                let engine = create_engine_with_body_and_output(
                    body,
                    output_collector.clone(),
                    None,
                    None,
                    special_collector,
                    user_sends.clone(),
                    scheduler.clone(),
                    Some(script),
                    global_data.clone(),
                );
                let mut scope = Scope::new();
                let ob = Dynamic::from(build_ob_from_body(body));
                if let Some(argument) = args.first().and_then(serde_json::Value::as_str) {
                    let _ = engine
                        .call_fn::<Dynamic>(&mut scope, &ast, function, (ob, argument.to_string()))
                        .map_err(|e| format!("call_fn {}: {}", function, e))?;
                } else {
                    let _ = engine
                        .call_fn::<Dynamic>(&mut scope, &ast, function, (ob,))
                        .map_err(|e| format!("call_fn {}: {}", function, e))?;
                }

                let mut outputs = output_collector.lock().unwrap().clone();
                let queued = user_sends.lock().unwrap().clone();
                for (recipient, command) in queued {
                    if recipient != target {
                        continue;
                    }
                    let parsed = CommandParser::parse(&command);
                    if parsed.is_empty() {
                        continue;
                    }
                    let delayed = tokio::task::block_in_place(|| {
                        let storage = script_storage.blocking_read();
                        if !storage.has_script(&parsed.command) {
                            return Ok((Vec::new(), None));
                        }
                        storage
                            .execute(&parsed.command, body, &parsed.args, None, None, None)
                            .map_err(|error| error.to_string())
                    })?;
                    outputs.extend(delayed.0);
                }
                let messages: Vec<String> = outputs
                    .iter()
                    .map(|line| {
                        let expanded = expand_abbreviated_ansi(line);
                        format!("{}\r\n", expanded)
                    })
                    .collect();
                Ok::<_, String>(messages)
            };
            let to_send = if target_addr.is_some() {
                broadcaster.with_player_body_by_connection_token(target, execute_for_body)
            } else {
                // Direct/unit callers created before a network connection use
                // the historical player-name fallback.
                broadcaster.with_player_body_by_name(target, execute_for_body)
            }
            .ok_or_else(|| format!("player not found: {}", target))?;

            let messages = to_send?;
            for msg in messages {
                if let Some(addr) = target_addr {
                    let _ = broadcaster.send_to(addr, &msg);
                } else {
                    let _ = broadcaster.send_to_by_player_name(target, &msg);
                }
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
mod admin_command_list_test;
#[cfg(test)]
mod admin_force_test;
#[cfg(test)]
mod admin_lookup_test;
#[cfg(test)]
mod admin_maintenance_test;
#[cfg(test)]
mod admin_mob_commands_test;
#[cfg(test)]
mod admin_user_mob_commands_test;
#[cfg(test)]
mod admin_values_test;
#[cfg(test)]
mod affix_commands_test;
#[cfg(test)]
mod body_persistence_test;
#[cfg(test)]
mod chat_commands_test;
#[cfg(test)]
mod delayed_commands_test;
#[cfg(test)]
mod economy_commands_test;
#[cfg(test)]
mod equipment_commands_test;
#[cfg(test)]
mod event_commands_test;
#[cfg(test)]
mod food_commands_test;
#[cfg(test)]
mod formatting_test;
#[cfg(test)]
mod guard_commands_test;
#[cfg(test)]
mod guild_commands_test;
#[cfg(test)]
mod help_commands_test;
#[cfg(test)]
mod inventory_commands_test;
#[cfg(test)]
mod item_commands_test;
#[cfg(test)]
mod item_magic_commands_test;
#[cfg(test)]
mod map_commands_test;
#[cfg(test)]
mod note_commands_test;
#[cfg(test)]
mod notice_commands_test;
#[cfg(test)]
mod object_attributes_test;
#[cfg(test)]
mod oneitem_commands_test;
#[cfg(test)]
mod party_commands_test;
#[cfg(test)]
mod script_runtime_test;
#[cfg(test)]
mod settings_commands_test;
#[cfg(test)]
mod skill_commands_test;
#[cfg(test)]
mod stat_commands_test;
#[cfg(test)]
mod status_commands_test;
#[cfg(test)]
mod tell_commands_test;
#[cfg(test)]
mod view_commands_test;
