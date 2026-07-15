//! Runtime world-authoring efuns.
//!
//! Rust enforces paths, permissions, persistence and live-cache coherence.
//! Rhai commands/events own presentation and decide which data to author.

use rhai::{Dynamic, Engine, Map};
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::player::Body;
use crate::world::{get_world_state, WorldEditor};

fn result(ok: bool, code: &str, detail: impl Into<String>) -> Map {
    let mut value = Map::new();
    value.insert("ok".into(), Dynamic::from(ok));
    value.insert("code".into(), Dynamic::from(code.to_string()));
    value.insert("detail".into(), Dynamic::from(detail.into()));
    value
}

fn field<'a>(payload: &'a JsonMap<String, JsonValue>, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing field: {key}"))
}

fn attrs(payload: &JsonMap<String, JsonValue>) -> Result<JsonMap<String, JsonValue>, String> {
    payload
        .get("attrs")
        .and_then(JsonValue::as_object)
        .cloned()
        .ok_or_else(|| "missing field: attrs".to_string())
}

fn reload_room(world: &mut crate::world::WorldState, zone: &str, room: &str) -> Result<(), String> {
    world
        .room_cache
        .reload_room(zone, room)
        .map_err(|error| format!("{error:?}"))?;
    world.reload_room_fixtures(zone, room)
}

fn apply(operation: &str, payload: JsonMap<String, JsonValue>) -> Result<String, String> {
    let editor = WorldEditor::default();
    let mut world = get_world_state()
        .write()
        .map_err(|_| "world lock poisoned".to_string())?;
    match operation {
        "room" | "방" => {
            let zone = field(&payload, "zone")?;
            let room = field(&payload, "room")?;
            editor
                .upsert_room(
                    zone,
                    room,
                    attrs(&payload)?,
                    payload
                        .get("create_only")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false),
                )
                .map_err(|error| error.to_string())?;
            reload_room(&mut world, zone, room)?;
            Ok(format!("{zone}:{room}"))
        }
        "exit" | "출구" => {
            let zone = field(&payload, "zone")?;
            let room = field(&payload, "room")?;
            editor
                .upsert_exit(
                    zone,
                    room,
                    field(&payload, "name")?,
                    field(&payload, "destination_zone")?,
                    field(&payload, "destination_room")?,
                    payload
                        .get("hidden")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false),
                )
                .map_err(|error| error.to_string())?;
            reload_room(&mut world, zone, room)?;
            Ok(format!("{zone}:{room}"))
        }
        "mob" | "몹" => {
            let zone = field(&payload, "zone")?;
            let key = field(&payload, "key")?;
            editor
                .upsert_mob(
                    zone,
                    key,
                    attrs(&payload)?,
                    payload
                        .get("create_only")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false),
                )
                .map_err(|error| error.to_string())?;
            world
                .mob_cache
                .reload_mob(zone, key)
                .map_err(|error| format!("{error:?}"))?;
            Ok(format!("{zone}:{key}"))
        }
        "mob_place" | "몹배치" => {
            let zone = field(&payload, "zone")?;
            let room = field(&payload, "room")?;
            let mob_key = field(&payload, "mob_key")?;
            editor
                .place_mob(zone, room, mob_key)
                .map_err(|error| error.to_string())?;
            reload_room(&mut world, zone, room)?;
            world.spawn_mob_at(mob_key, zone, room)?;
            Ok(format!("{zone}:{room}"))
        }
        "item" | "아이템" => {
            let key = field(&payload, "key")?;
            editor
                .upsert_item(
                    key,
                    attrs(&payload)?,
                    payload
                        .get("create_only")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false),
                )
                .map_err(|error| error.to_string())?;
            world
                .item_cache
                .reload_item(key)
                .map_err(|error| format!("{error:?}"))?;
            Ok(key.to_string())
        }
        "fixture" | "고정물" => {
            let zone = field(&payload, "zone")?;
            let room = field(&payload, "room")?;
            editor
                .upsert_fixture(
                    zone,
                    room,
                    field(&payload, "key")?,
                    field(&payload, "kind")?,
                    attrs(&payload)?,
                )
                .map_err(|error| error.to_string())?;
            reload_room(&mut world, zone, room)?;
            Ok(format!("{zone}:{room}"))
        }
        "event" | "이벤트" => {
            let zone = field(&payload, "zone")?;
            let path = field(&payload, "path")?;
            editor
                .write_event(zone, path, field(&payload, "source")?)
                .map_err(|error| error.to_string())?;
            Ok(format!("{zone}:{path}"))
        }
        _ => Err(format!("unknown operation: {operation}")),
    }
}

fn map_payload(payload: Map) -> Result<JsonMap<String, JsonValue>, String> {
    rhai::serde::from_dynamic::<JsonValue>(&Dynamic::from(payload))
        .map_err(|error| error.to_string())?
        .as_object()
        .cloned()
        .ok_or_else(|| "payload must be an object".to_string())
}

fn json_payload(source: &str) -> Result<JsonMap<String, JsonValue>, String> {
    serde_json::from_str::<JsonValue>(source)
        .map_err(|error| error.to_string())?
        .as_object()
        .cloned()
        .ok_or_else(|| "payload must be a JSON object".to_string())
}

fn home_authorized(owner: &str, payload: &JsonMap<String, JsonValue>) -> Result<(), String> {
    let room = field(payload, "room")?;
    let editor = WorldEditor::default();
    match editor.room_owner("사용자맵", room) {
        Some(saved_owner) if saved_owner == owner => Ok(()),
        None if room == owner || room.starts_with(&format!("{owner}_")) => Ok(()),
        _ => Err("room is not owned by actor".into()),
    }
}

fn apply_home(
    owner: &str,
    operation: &str,
    mut payload: JsonMap<String, JsonValue>,
) -> Result<String, String> {
    if !matches!(
        operation,
        "room" | "방" | "exit" | "출구" | "fixture" | "고정물" | "mob_place" | "몹배치"
    ) {
        return Err("operation is not available to home events".into());
    }
    home_authorized(owner, &payload)?;
    payload.insert("zone".into(), JsonValue::String("사용자맵".into()));
    if matches!(operation, "room" | "방") {
        let mut properties = attrs(&payload)?;
        properties.insert("주인".into(), JsonValue::String(owner.to_string()));
        payload.insert("attrs".into(), JsonValue::Object(properties));
    }
    apply(operation, payload)
}

struct ItemToolGrant {
    capabilities: Vec<String>,
    zones: Vec<String>,
}

fn item_tool_grant(item_key: &str) -> Result<ItemToolGrant, String> {
    let mut world = get_world_state()
        .write()
        .map_err(|_| "world lock poisoned".to_string())?;
    if world.item_cache.get_item(item_key).is_none() {
        world
            .item_cache
            .load_item(item_key)
            .map_err(|error| format!("{error:?}"))?;
    }
    let item = world
        .item_cache
        .get_item(item_key)
        .ok_or_else(|| "item definition not found".to_string())?;
    let flags = item
        .attributes
        .get("아이템속성")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .collect::<Vec<_>>();
    if !flags
        .iter()
        .any(|flag| matches!(*flag, "줄수없음" | "양도불가" | "거래불가"))
    {
        return Err("world-authoring item must be non-transferable".into());
    }
    let values = item
        .attributes
        .get("world_capabilities")
        .or_else(|| item.attributes.get("월드권한"))
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "item has no world capabilities".to_string())?;
    let capabilities = values
        .iter()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect();
    let zones = item
        .attributes
        .get("world_zones")
        .or_else(|| item.attributes.get("월드존"))
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect();
    Ok(ItemToolGrant {
        capabilities,
        zones,
    })
}

fn apply_zone_creator(
    grant: &ItemToolGrant,
    operation: &str,
    payload: JsonMap<String, JsonValue>,
) -> Result<String, String> {
    let zone = field(&payload, "zone")?;
    if grant.zones.is_empty() || !grant.zones.iter().any(|allowed| allowed == zone) {
        return Err(format!("item is not delegated to zone: {zone}"));
    }
    let specific = match operation {
        "room" | "방" => "zone.room",
        "exit" | "출구" => "zone.exit",
        "fixture" | "고정물" => "zone.fixture",
        "mob" | "몹" => "zone.mob.author",
        "mob_place" | "몹배치" => "zone.mob.place",
        "event" | "이벤트" => "zone.event",
        _ => return Err("operation is not available to zone creators".into()),
    };
    if !grant
        .capabilities
        .iter()
        .any(|capability| capability == "zone.creator" || capability == specific)
    {
        return Err(format!("item lacks capability: {specific}"));
    }
    if matches!(operation, "exit" | "출구") {
        let destination = field(&payload, "destination_zone")?;
        if !grant.zones.iter().any(|allowed| allowed == destination) {
            return Err("exit destination is outside delegated zones".into());
        }
    }
    if matches!(operation, "mob_place" | "몹배치") {
        let mob_zone = field(&payload, "mob_key")?
            .split_once(':')
            .map(|(mob_zone, _)| mob_zone)
            .ok_or_else(|| "invalid mob key".to_string())?;
        if !grant.zones.iter().any(|allowed| allowed == mob_zone) {
            return Err("mob template is outside delegated zones".into());
        }
    }
    apply(operation, payload)
}

fn apply_item_tool(
    owner: &str,
    item_key: &str,
    operation: &str,
    mut payload: JsonMap<String, JsonValue>,
) -> Result<String, String> {
    let grant = item_tool_grant(item_key)?;
    if grant
        .capabilities
        .iter()
        .any(|capability| capability == "zone.creator" || capability.starts_with("zone."))
    {
        return apply_zone_creator(&grant, operation, payload);
    }
    let required = match operation {
        "room" | "방" => "home.room",
        "exit" | "출구" => "home.exit",
        "fixture" | "고정물" => "home.fixture",
        "mob_place" | "몹배치" => "home.mob.place",
        "mob" | "몹" => "home.mob.author",
        _ => return Err("operation is not available to item world tools".into()),
    };
    if !grant.capabilities.iter().any(|saved| saved == required) {
        return Err(format!("item lacks capability: {required}"));
    }
    if matches!(operation, "mob" | "몹") {
        let key = field(&payload, "key")?;
        let editor = WorldEditor::default();
        let mob_exists = editor
            .mob_path("사용자맵", key)
            .map_err(|error| error.to_string())?
            .exists();
        match editor.mob_owner("사용자맵", key) {
            Some(saved) if saved != owner => return Err("mob is not owned by actor".into()),
            Some(_) => {}
            None if mob_exists => return Err("mob is not owned by actor".into()),
            None => {}
        }
        let mut properties = attrs(&payload)?;
        properties.insert("주인".into(), JsonValue::String(owner.to_string()));
        payload.insert("attrs".into(), JsonValue::Object(properties));
        payload.insert("zone".into(), JsonValue::String("사용자맵".into()));
        return apply(operation, payload);
    }
    if matches!(operation, "mob_place" | "몹배치") {
        let mob_key = field(&payload, "mob_key")?;
        let Some((zone, key)) = mob_key.split_once(':') else {
            return Err("invalid mob key".into());
        };
        if zone != "사용자맵"
            || WorldEditor::default().mob_owner(zone, key).as_deref() != Some(owner)
        {
            return Err("item may place only actor-owned mobs".into());
        }
    }
    apply_home(owner, operation, payload)
}

pub(super) fn register_world_edit_efuns(
    engine: &mut Engine,
    body_ptr: *mut Body,
    allow_direct_home_edit: bool,
) {
    let admin_map_body = body_ptr;
    engine.register_fn(
        "world_edit",
        move |_ob: &mut Map, operation: &str, payload: Map| -> Map {
            let body = unsafe { &*admin_map_body };
            if body.get_int("관리자등급") < 2000 {
                return result(false, "forbidden", "administrator level 2000 required");
            }
            match map_payload(payload).and_then(|payload| apply(operation, payload)) {
                Ok(detail) => result(true, "ok", detail),
                Err(error) => result(false, "error", error),
            }
        },
    );

    let admin_json_body = body_ptr;
    engine.register_fn(
        "world_edit_json",
        move |_ob: &mut Map, operation: &str, source: &str| -> Map {
            let body = unsafe { &*admin_json_body };
            if body.get_int("관리자등급") < 2000 {
                return result(false, "forbidden", "administrator level 2000 required");
            }
            match json_payload(source).and_then(|payload| apply(operation, payload)) {
                Ok(detail) => result(true, "ok", detail),
                Err(error) => result(false, "error", error),
            }
        },
    );

    if allow_direct_home_edit {
        let home_body = body_ptr;
        engine.register_fn(
            "home_world_edit",
            move |_ob: &mut Map, operation: &str, payload: Map| -> Map {
                let body = unsafe { &*home_body };
                let owner = body.get_name();
                match map_payload(payload)
                    .and_then(|payload| apply_home(&owner, operation, payload))
                {
                    Ok(detail) => result(true, "ok", detail),
                    Err(error) => result(false, "error", error),
                }
            },
        );
    }
}

pub(super) fn register_item_world_edit_efun(
    engine: &mut Engine,
    body_ptr: *mut Body,
    selected_item_key: String,
) {
    engine.register_fn(
        "item_world_edit",
        move |_ob: &mut Map, operation: &str, payload: Map| -> Map {
            let body = unsafe { &*body_ptr };
            let owner = body.get_name();
            match map_payload(payload)
                .and_then(|payload| apply_item_tool(&owner, &selected_item_key, operation, payload))
            {
                Ok(detail) => result(true, "ok", detail),
                Err(error) => result(false, "error", error),
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Scope;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique(label: &str) -> String {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("__world_edit_{label}_{}_{nonce}", std::process::id())
    }

    fn object(value: JsonValue) -> JsonMap<String, JsonValue> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn authoring_is_persistent_and_live_without_restart() {
        // Zone names ending in a digit are difficulty instances, so keep the
        // generated base zone name non-numeric.
        let zone = format!("{}존", unique("zone"));
        let item = unique("item");
        let event = unique("event");
        apply(
            "방",
            object(json!({"zone": zone, "room": "1", "attrs": {
                "이름": "첫 이름", "설명": ["첫 설명"], "출구": []
            }})),
        )
        .unwrap();
        let first_arc = get_world_state()
            .read()
            .unwrap()
            .room_cache
            .get_room_cached(&zone, "1")
            .unwrap();
        first_arc.write().unwrap().players.push("잔류자".into());
        apply(
            "방",
            object(json!({"zone": zone, "room": "1", "attrs": {"이름": "바뀐 이름"}})),
        )
        .unwrap();
        apply(
            "출구",
            object(json!({"zone": zone, "room": "1", "name": "안쪽",
            "destination_zone": zone, "destination_room": "1"})),
        )
        .unwrap();
        apply(
            "몹",
            object(json!({"zone": zone, "key": "검객", "attrs": {
                "이름": "청의검객", "체력": 77
            }})),
        )
        .unwrap();
        apply(
            "몹배치",
            object(json!({"zone": zone, "room": "1", "mob_key": format!("{zone}:검객")})),
        )
        .unwrap();
        apply(
            "아이템",
            object(json!({"key": item, "attrs": {"이름": "비취비수", "종류": "무기"}})),
        )
        .unwrap();
        apply("고정물", object(json!({"zone": zone, "room": "1", "key": "벽화",
            "kind": "mechanism", "attrs": {"name": "낡은 벽화", "설명1": "낡은 벽화가 걸려 있다."}}))).unwrap();
        apply(
            "이벤트",
            object(json!({"zone": zone, "path": event,
            "source": "fn main(ob, fixture_id, cmdline) { ob[\"version\"] = 1; }"})),
        )
        .unwrap();

        {
            let world = get_world_state().read().unwrap();
            let current = world.room_cache.get_room_cached(&zone, "1").unwrap();
            assert!(Arc::ptr_eq(&first_arc, &current));
            let room = current.read().unwrap();
            assert_eq!(room.display_name, "바뀐 이름");
            assert_eq!(room.players, vec!["잔류자"]);
            assert!(room.get_exit_by_name("안쪽").is_some());
            assert_eq!(
                world.mob_cache.get_mob_by_zone(&zone, "검객").unwrap().hp,
                77
            );
            assert_eq!(world.get_mobs_in_room(&zone, "1").len(), 1);
            assert_eq!(world.item_cache.get_item(&item).unwrap().name, "비취비수");
            assert_eq!(world.get_room_fixtures(&zone, "1").len(), 1);
        }
        assert!(std::path::Path::new(&format!("data/script/{zone}/{event}.rhai")).exists());
        let _ = std::fs::remove_dir_all(format!("data/map/{zone}"));
        let _ = std::fs::remove_dir_all(format!("data/mob/{zone}"));
        let _ = std::fs::remove_file(format!("data/item/{item}.json"));
        let _ = std::fs::remove_dir_all(format!("data/script/{zone}"));
    }

    #[test]
    fn permissions_separate_admin_and_owned_home_editing() {
        let mut ordinary = Body::new();
        ordinary.set("이름", "소유자");
        ordinary.set("관리자등급", 0_i64);
        let mut engine = Engine::new();
        register_world_edit_efuns(&mut engine, &mut ordinary, true);
        let ast = engine
            .compile("world_edit(ob, \"방\", #{ zone: \"금지\", room: \"1\", attrs: #{} })")
            .unwrap();
        let mut scope = Scope::new();
        scope.push("ob", Map::new());
        let denied = engine.eval_ast_with_scope::<Map>(&mut scope, &ast).unwrap();
        assert!(!denied["ok"].as_bool().unwrap());

        let owner = unique("owner");
        let owned_room = format!("{owner}_별채");
        apply_home(
            &owner,
            "방",
            object(json!({"room": owned_room,
            "attrs": {"이름": "별채", "설명": []}})),
        )
        .unwrap();
        assert_eq!(
            WorldEditor::default().room_owner("사용자맵", &owned_room),
            Some(owner.clone())
        );
        assert!(apply_home(
            "다른사람",
            "고정물",
            object(json!({"room": owned_room,
            "key": "침범", "kind": "fixture", "attrs": {}}))
        )
        .is_err());
        assert!(apply_home(
            &owner,
            "이벤트",
            object(json!({"room": owned_room,
            "path": "침범", "source": ""}))
        )
        .is_err());
        let transferable = unique("transferable_tool");
        apply(
            "아이템",
            object(json!({"key": transferable, "attrs": {
                "이름": "떠도는 도면", "월드권한": ["home.room"]
            }})),
        )
        .unwrap();
        assert!(apply_item_tool(
            &owner,
            &transferable,
            "방",
            object(json!({"room": format!("{owner}_침범"), "attrs": {}}))
        )
        .unwrap_err()
        .contains("non-transferable"));
        let _ = std::fs::remove_file(format!("data/map/사용자맵/{owned_room}.json"));
        let _ = std::fs::remove_file(format!("data/item/{transferable}.json"));
    }

    #[test]
    fn non_transferable_book_event_uses_scoped_world_capabilities_live() {
        let owner = unique("item_owner");
        let room = format!("{owner}_공방");
        let annex = format!("{owner}_별채");
        let mob = format!("{owner}_목우병");
        let item = unique("home_token");
        let event = unique("home_event");
        apply(
            "아이템",
            object(json!({"key": item, "attrs": {
                "이름": "천공개물 잔권", "종류": "서책", "반응이름": ["천공개물"],
                "아이템속성": ["줄수없음", "버리지못함"],
                "월드권한": ["home.room", "home.exit", "home.mob.author", "home.mob.place"],
                "events": {"사용": format!("{event}.rhai")}
            }})),
        )
        .unwrap();
        let reserved_mob = format!("{owner}_관리자보호몹");
        apply(
            "몹",
            object(json!({"zone": "사용자맵", "key": reserved_mob,
                "attrs": {"이름": "관리자 보호몹", "체력": 10}})),
        )
        .unwrap();
        assert!(apply_item_tool(
            &owner,
            &item,
            "몹",
            object(json!({"key": reserved_mob, "attrs": {"체력": 999}}))
        )
        .unwrap_err()
        .contains("not owned"));
        let free_mob = unique("접두사없는_목우병");
        apply_item_tool(
            &owner,
            &item,
            "몹",
            object(json!({"key": free_mob, "attrs": {"이름": "자유이름 목우병", "체력": 12}})),
        )
        .unwrap();
        assert_eq!(
            WorldEditor::default().mob_owner("사용자맵", &free_mob),
            Some(owner.clone())
        );
        apply(
            "이벤트",
            object(json!({"zone": "아이템", "path": event, "source":
                "fn main(ob, item_id, item_key, cmdline) {\n\
                   let base = ob[\"이름\"] + \"_공방\";\n\
                   let annex = ob[\"이름\"] + \"_별채\";\n\
                   let mob = ob[\"이름\"] + \"_목우병\";\n\
                   let a = item_world_edit(ob, \"방\", #{ room: base, attrs: #{ \"이름\": \"천공 공방\", \"설명\": [\"고대 목우와 기관 부품이 놓여 있다.\"] } });\n\
                   let b = item_world_edit(ob, \"방\", #{ room: annex, attrs: #{ \"이름\": \"기관 별채\", \"설명\": [\"목제 회랑이 이어진 별채다.\"] } });\n\
                   let c = item_world_edit(ob, \"출구\", #{ room: base, name: \"회랑\", destination_zone: \"사용자맵\", destination_room: annex });\n\
                   let d = item_world_edit(ob, \"몹\", #{ key: mob, attrs: #{ \"이름\": \"목우병\", \"체력\": 45, \"레벨\": 3 } });\n\
                   let e = item_world_edit(ob, \"몹배치\", #{ room: base, mob_key: \"사용자맵:\" + mob });\n\
                   if a[\"ok\"] && b[\"ok\"] && c[\"ok\"] && d[\"ok\"] && e[\"ok\"] { send_line(ob, \"천공개물의 도해가 공방과 목우병으로 펼쳐집니다.\"); }\n\
                 }"
            })),
        )
        .unwrap();
        let mut body = Body::new();
        body.set("이름", owner.clone());
        body.object.inv_stack.insert(item.clone(), 1);
        let outcome = super::super::try_item_event(&mut body, "사용자맵", "천공개물 사용")
            .expect("item event must run");
        let crate::command::CommandResult::MobEvent { output_lines, .. } = outcome else {
            panic!("unexpected item event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("천공개물")));
        assert_eq!(
            WorldEditor::default().room_owner("사용자맵", &room),
            Some(owner.clone())
        );
        assert!(get_world_state()
            .read()
            .unwrap()
            .room_cache
            .get_room_cached("사용자맵", &room)
            .is_some());
        let world = get_world_state().read().unwrap();
        let room_data = world.room_cache.get_room_cached("사용자맵", &room).unwrap();
        assert_eq!(
            room_data
                .read()
                .unwrap()
                .get_exit_by_name("회랑")
                .and_then(|exit| exit.destination("사용자맵")),
            Some(("사용자맵".into(), annex.clone()))
        );
        assert_eq!(
            world
                .mob_cache
                .get_mob_by_zone("사용자맵", &mob)
                .unwrap()
                .hp,
            45
        );
        assert_eq!(world.get_mobs_in_room("사용자맵", &room).len(), 1);
        drop(world);

        let _ = std::fs::remove_file(format!("data/map/사용자맵/{room}.json"));
        let _ = std::fs::remove_file(format!("data/map/사용자맵/{annex}.json"));
        let _ = std::fs::remove_file(format!("data/mob/사용자맵/{mob}.json"));
        let _ = std::fs::remove_file(format!("data/mob/사용자맵/{reserved_mob}.json"));
        let _ = std::fs::remove_file(format!("data/mob/사용자맵/{free_mob}.json"));
        let _ = std::fs::remove_file(format!("data/item/{item}.json"));
        let _ = std::fs::remove_file(format!("data/script/아이템/{event}.rhai"));
        let _ = std::fs::remove_file(format!("data/user/{owner}.json"));
    }

    #[test]
    fn delegated_zone_book_makes_an_ordinary_user_a_scoped_creator() {
        let owner = unique("zone_creator");
        let zone = format!("{}존", unique("delegated"));
        let outside = format!("{}존", unique("outside"));
        let item = unique("art_of_war");
        let event = unique("art_of_war_event");
        apply(
            "아이템",
            object(json!({"key": item, "attrs": {
                "이름": "손자병법 위임본", "종류": "서책", "반응이름": ["손자병법"],
                "아이템속성": ["양도불가"],
                "월드권한": ["zone.creator"], "월드존": [zone],
                "events": {"사용": format!("{event}.rhai")}
            }})),
        )
        .unwrap();
        apply(
            "몹",
            object(json!({"zone": zone, "key": "백전노장",
                "attrs": {"이름": "백전노장", "체력": 10}})),
        )
        .unwrap();
        let source = r#"fn main(ob, item_id, item_key, cmdline) {
            let a = item_world_edit(ob, "방", #{ zone: "__ZONE__", room: "본영", attrs: #{ "이름": "병법 본영", "설명": ["진법도가 펼쳐진 본영이다."] } });
            let b = item_world_edit(ob, "방", #{ zone: "__ZONE__", room: "연무장", attrs: #{ "이름": "진법 연무장", "설명": [] } });
            let c = item_world_edit(ob, "출구", #{ zone: "__ZONE__", room: "본영", name: "연무장", destination_zone: "__ZONE__", destination_room: "연무장" });
            let d = item_world_edit(ob, "몹", #{ zone: "__ZONE__", key: "백전노장", attrs: #{ "이름": "백전노장", "체력": 88, "레벨": 8 } });
            let e = item_world_edit(ob, "몹배치", #{ zone: "__ZONE__", room: "본영", mob_key: "__ZONE__:백전노장" });
            let f = item_world_edit(ob, "고정물", #{ zone: "__ZONE__", room: "본영", key: "진법도", kind: "fixture", attrs: #{ name: "팔문진법도", "설명1": "팔문진법도가 펼쳐져 있다." } });
            if a["ok"] && b["ok"] && c["ok"] && d["ok"] && e["ok"] && f["ok"] { send_line(ob, "위임된 존에 병법 본영을 세웁니다."); }
        }"#
        .replace("__ZONE__", &zone);
        apply(
            "이벤트",
            object(json!({"zone": "아이템", "path": event, "source": source})),
        )
        .unwrap();

        let mut body = Body::new();
        body.set("이름", owner.clone());
        body.object.inv_stack.insert(item.clone(), 1);
        let outcome = super::super::try_item_event(&mut body, "사용자맵", "손자병법 사용")
            .expect("delegated creator event must run");
        let crate::command::CommandResult::MobEvent { output_lines, .. } = outcome else {
            panic!("unexpected item event result");
        };
        assert!(output_lines.iter().any(|line| line.contains("위임된 존")));
        {
            let world = get_world_state().read().unwrap();
            let room = world.room_cache.get_room_cached(&zone, "본영").unwrap();
            assert!(room.read().unwrap().get_exit_by_name("연무장").is_some());
            assert_eq!(
                world
                    .mob_cache
                    .get_mob_by_zone(&zone, "백전노장")
                    .unwrap()
                    .hp,
                88
            );
            assert_eq!(world.get_mobs_in_room(&zone, "본영").len(), 1);
            assert_eq!(world.get_room_fixtures(&zone, "본영").len(), 1);
        }
        apply_item_tool(
            &owner,
            &item,
            "이벤트",
            object(json!({"zone": zone, "path": "병법_점검.rhai",
                "source": "fn main(ob, fixture_id, cmdline) { }"})),
        )
        .unwrap();
        assert!(std::path::Path::new(&format!("data/script/{zone}/병법_점검.rhai")).exists());
        assert!(apply_item_tool(
            &owner,
            &item,
            "방",
            object(json!({"zone": outside, "room": "침범", "attrs": {}}))
        )
        .unwrap_err()
        .contains("not delegated"));

        let _ = std::fs::remove_dir_all(format!("data/map/{zone}"));
        let _ = std::fs::remove_dir_all(format!("data/mob/{zone}"));
        let _ = std::fs::remove_dir_all(format!("data/script/{zone}"));
        let _ = std::fs::remove_file(format!("data/item/{item}.json"));
        let _ = std::fs::remove_file(format!("data/script/아이템/{event}.rhai"));
        let _ = std::fs::remove_file(format!("data/user/{owner}.json"));
    }
}
