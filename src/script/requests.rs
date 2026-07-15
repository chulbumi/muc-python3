//! Deferred state-change requests emitted by Rhai commands and consumed by the network layer.

use crate::player::Body;

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
pub(crate) const SOUL_SWITCH_REQUEST: &str = "_soul_switch_request";
pub(crate) const SOUL_ATTACH_REQUEST: &str = "_soul_attach_request";
pub(crate) const SUMMON_PLAYER_REQUEST: &str = "_summon_player_request";
pub(crate) const FORCE_COMMAND_REQUEST: &str = "_force_command_request";
/// NPC event `$별호변경`처럼 Python `do_command()`가 현재 사용자의 다음
/// 명령을 같은 입력 처리 안에서 동기 실행해야 할 때 쓴다.
pub(crate) const EVENT_COMMAND_REQUEST: &str = "_event_command_request";

fn take_json<T: serde::de::DeserializeOwned>(body: &mut Body, key: &str) -> Option<T> {
    body.temp_mut()
        .remove(key)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub(crate) fn take_summon_player_request(body: &mut Body) -> Vec<(String, String, String)> {
    take_json(body, SUMMON_PLAYER_REQUEST).unwrap_or_default()
}

pub(crate) fn take_force_command_request(body: &mut Body) -> Vec<(String, String)> {
    take_json(body, FORCE_COMMAND_REQUEST).unwrap_or_default()
}

pub(crate) fn take_event_command_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(EVENT_COMMAND_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_guild_accept_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, GUILD_ACCEPT_REQUEST)
}

pub(crate) fn take_guild_apply_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, GUILD_APPLY_REQUEST)
}

pub(crate) fn take_guild_reset_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(GUILD_RESET_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_admin_set_player_value_request(
    body: &mut Body,
) -> Option<(String, String, serde_json::Value)> {
    take_json(body, ADMIN_SET_PLAYER_VALUE_REQUEST)
}

pub(crate) fn take_change_player_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(CHANGE_PLAYER_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_soul_switch_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(SOUL_SWITCH_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_soul_attach_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, SOUL_ATTACH_REQUEST)
}

pub(crate) fn take_teach_skill_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, TEACH_SKILL_REQUEST)
}

pub(crate) fn take_remove_skill_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, REMOVE_SKILL_REQUEST)
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
    take_json(body, SET_SKILL_REQUEST)
}

pub(crate) fn take_guild_transfer_request(body: &mut Body) -> Option<String> {
    body.temp_mut()
        .remove(GUILD_TRANSFER_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn take_guild_position_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, GUILD_POSITION_REQUEST)
}

pub(crate) fn take_guild_nickname_request(body: &mut Body) -> Option<(String, String)> {
    take_json(body, GUILD_NICKNAME_REQUEST)
}

pub(crate) fn take_set_player_attr_request(body: &mut Body) -> Option<(String, String, i64)> {
    take_json(body, SET_PLAYER_ATTR_REQUEST)
}
