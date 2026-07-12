//! Data/action efuns for Python-compatible follower and Party Rhai commands.
//!
//! Rust exposes only connection-scoped snapshots, state requests, and opaque
//! delivery routing requests.  Every visible byte remains authored by Rhai.

use crate::network::social::{RelationState, SocialAction};
use crate::object::Value;
use crate::player::Body;
use rhai::{Array, Dynamic, Engine, Map};
use std::cell::RefCell;

pub(crate) const PARTY_DISCONNECT_REQUEST: &str = "_party_disconnect";
const PARTY_ACTION_REQUEST: &str = "_party_action";
const PARTY_DELIVERY_REQUESTS: &str = "_party_deliveries";

thread_local! {
    static PRECOMPUTED_PARTY_CONTEXT: RefCell<Option<Map>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(crate) struct PartyDelivery {
    pub connection_id: String,
    pub raw_text: String,
}

pub(crate) fn build_party_person_snapshot(
    connection_id: String,
    body: &Body,
    relation: RelationState,
    interactive: i32,
) -> Dynamic {
    let config = super::parse_config_string(&body.get_string("설정상태"));
    let mut person = Map::new();
    person.insert("id".into(), Dynamic::from(connection_id));
    person.insert("is_player".into(), Dynamic::from(true));
    person.insert("name".into(), Dynamic::from(body.get_name()));
    person.insert(
        "nickname".into(),
        Dynamic::from(body.get_string("무림별호")),
    );
    person.insert("personality".into(), Dynamic::from(body.get_string("성격")));
    person.insert("hp".into(), Dynamic::from(body.get_hp()));
    person.insert("max_hp".into(), Dynamic::from(body.get_max_hp()));
    person.insert("mp".into(), Dynamic::from(body.get_mp()));
    person.insert("act".into(), Dynamic::from(body.act as i64));
    person.insert("max_mp".into(), Dynamic::from(body.get_max_mp()));
    person.insert(
        "transparent".into(),
        Dynamic::from(body.get_int("투명상태") == 1),
    );
    person.insert(
        "refuses_follow".into(),
        Dynamic::from(config.get("동행거부").map(String::as_str) == Some("1")),
    );
    person.insert(
        "show_prompt".into(),
        Dynamic::from(interactive == 1 && config.get("엘피출력").map(String::as_str) != Some("1")),
    );
    person.insert(
        "follow_id".into(),
        Dynamic::from(relation.follow.unwrap_or_default()),
    );
    person.insert(
        "party_leader_id".into(),
        Dynamic::from(relation.party_leader.unwrap_or_default()),
    );
    let combat_targets = body
        .temp()
        .get("_combat_target_ids")
        .and_then(Value::as_str)
        .map(|raw| {
            raw.split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| Dynamic::from(s.to_string()))
                .collect::<Array>()
        })
        .unwrap_or_default();
    person.insert("combat_target_ids".into(), Dynamic::from(combat_targets));
    person.insert(
        "reaction_names".into(),
        Dynamic::from(
            super::reaction_names(&body.get_string("반응이름"))
                .into_iter()
                .map(Dynamic::from)
                .collect::<Array>(),
        ),
    );
    person.insert("lookup_supported".into(), Dynamic::from(true));
    Dynamic::from(person)
}

/// Convert a room-local non-player target snapshot into lookup data only.
/// Rust intentionally does not assign it a position relative to players:
/// `WorldState` does not yet preserve Python's unified `room.objs` order.
pub(crate) fn build_party_nonplayer_snapshot(target: &super::RoomMugongTargetSnapshot) -> Dynamic {
    let mut person = Map::new();
    person.insert("name".into(), Dynamic::from(target.name.clone()));
    person.insert(
        "reaction_names".into(),
        Dynamic::from(
            target
                .reaction_names
                .iter()
                .cloned()
                .map(Dynamic::from)
                .collect::<Array>(),
        ),
    );
    person.insert("reaction_raw".into(), Dynamic::from(String::new()));
    person.insert("reaction_is_array".into(), Dynamic::from(true));
    person.insert("transparent".into(), Dynamic::from(target.transparent));
    person.insert(
        "kind".into(),
        Dynamic::from(match target.kind {
            super::RoomMugongTargetKind::Mob => "mob",
            super::RoomMugongTargetKind::Item => "item",
            super::RoomMugongTargetKind::Player => "player",
        }),
    );
    person.insert("act".into(), Dynamic::from(i64::from(target.act)));
    Dynamic::from(person)
}

/// Load Python `Room.create()` installation-list Boxes through the shared Box
/// registry. `None` means a registry/object lock could not be inspected, so
/// callers must disable follow lookup instead of assuming no Box exists.
pub(crate) fn installed_box_party_snapshots(zone: &str, room: &str) -> Option<Array> {
    let boxes = super::box_commands::installed_boxes_for_room(zone, room)?;
    let mut snapshots = Array::new();
    for box_object in boxes {
        let box_object = box_object.lock().ok()?;
        let reaction_raw = box_object.getString("반응이름");
        let reaction_is_array = box_object.temp.contains_key("_python_json_array:반응이름");
        let reaction_names = if reaction_is_array {
            reaction_raw
                .split('\n')
                .map(str::to_string)
                .map(Dynamic::from)
                .collect::<Array>()
        } else {
            Array::new()
        };
        let mut snapshot = Map::new();
        snapshot.insert("name".into(), Dynamic::from(box_object.getName()));
        snapshot.insert("reaction_names".into(), Dynamic::from(reaction_names));
        snapshot.insert("reaction_raw".into(), Dynamic::from(reaction_raw));
        snapshot.insert("reaction_is_array".into(), Dynamic::from(reaction_is_array));
        snapshot.insert(
            "transparent".into(),
            Dynamic::from(box_object.getInt("투명상태") == 1),
        );
        snapshot.insert("kind".into(), Dynamic::from("box"));
        snapshot.insert("act".into(), Dynamic::from(0_i64));
        snapshots.push(Dynamic::from(snapshot));
    }
    Some(snapshots)
}

pub(crate) fn missing_party_person(connection_id: String, relation: RelationState) -> Dynamic {
    let mut person = Map::new();
    person.insert("id".into(), Dynamic::from(connection_id));
    person.insert("is_player".into(), Dynamic::from(false));
    person.insert("name".into(), Dynamic::from(String::new()));
    person.insert("nickname".into(), Dynamic::from(String::new()));
    person.insert("personality".into(), Dynamic::from(String::new()));
    person.insert("hp".into(), Dynamic::from(0_i64));
    person.insert("max_hp".into(), Dynamic::from(0_i64));
    person.insert("mp".into(), Dynamic::from(0_i64));
    person.insert("act".into(), Dynamic::from(0_i64));
    person.insert("max_mp".into(), Dynamic::from(0_i64));
    person.insert("transparent".into(), Dynamic::from(false));
    person.insert("refuses_follow".into(), Dynamic::from(false));
    person.insert("show_prompt".into(), Dynamic::from(false));
    person.insert(
        "follow_id".into(),
        Dynamic::from(relation.follow.unwrap_or_default()),
    );
    person.insert(
        "party_leader_id".into(),
        Dynamic::from(relation.party_leader.unwrap_or_default()),
    );
    person.insert("reaction_names".into(), Dynamic::from(Array::new()));
    person.insert("lookup_supported".into(), Dynamic::from(true));
    Dynamic::from(person)
}

pub(crate) fn set_precomputed_party_context(context: Map) {
    PRECOMPUTED_PARTY_CONTEXT.with(|slot| *slot.borrow_mut() = Some(context));
}

pub(crate) fn clear_precomputed_party_context() {
    PRECOMPUTED_PARTY_CONTEXT.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn take_party_requests(body: &mut Body) -> (Option<SocialAction>, Vec<PartyDelivery>) {
    let action = body
        .temp_mut()
        .remove(PARTY_ACTION_REQUEST)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok());
    let deliveries = body
        .temp_mut()
        .remove(PARTY_DELIVERY_REQUESTS)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    body.temp_mut().remove(PARTY_DISCONNECT_REQUEST);
    (action, deliveries)
}

pub(crate) fn register_party_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    engine.register_fn("get_party_context", || -> Dynamic {
        Dynamic::from(
            PRECOMPUTED_PARTY_CONTEXT
                .with(|slot| slot.borrow().clone())
                .unwrap_or_else(empty_context),
        )
    });

    engine.register_fn("find_follow_player", |line: &str| -> Dynamic {
        find_follow_player(line)
    });

    let body_ptr_follow = body_ptr;
    engine.register_fn(
        "party_request_follow",
        move |_ob: &mut Map, target_id: &str| -> bool {
            if !context_array_contains("room_players", target_id) {
                return false;
            }
            store_action(
                unsafe { &mut *body_ptr_follow },
                SocialAction::Follow {
                    target: target_id.to_string(),
                },
            )
        },
    );

    let body_ptr_stop = body_ptr;
    engine.register_fn(
        "party_request_stop_following",
        move |_ob: &mut Map| -> bool {
            store_action(unsafe { &mut *body_ptr_stop }, SocialAction::StopFollowing)
        },
    );

    let body_ptr_leave = body_ptr;
    engine.register_fn("party_request_leave", move |_ob: &mut Map| -> bool {
        store_action(unsafe { &mut *body_ptr_leave }, SocialAction::LeaveParty)
    });

    let body_ptr_add = body_ptr;
    engine.register_fn(
        "party_request_add_members",
        move |_ob: &mut Map, member_ids: Array| -> bool {
            let members = validated_ids("followers", member_ids);
            store_action(
                unsafe { &mut *body_ptr_add },
                SocialAction::AddPartyMembers { members },
            )
        },
    );

    let body_ptr_remove = body_ptr;
    engine.register_fn(
        "party_request_remove_followers",
        move |_ob: &mut Map, member_ids: Array| -> bool {
            let members = validated_ids("followers", member_ids);
            store_action(
                unsafe { &mut *body_ptr_remove },
                SocialAction::RemoveFollowers { members },
            )
        },
    );

    let body_ptr_disconnect = body_ptr;
    engine.register_fn("party_request_disconnect", move |_ob: &mut Map| -> bool {
        store_action(
            unsafe { &mut *body_ptr_disconnect },
            SocialAction::Disconnect,
        )
    });

    let body_ptr_targets = body_ptr;
    engine.register_fn(
        "party_request_set_combat_targets",
        move |_ob: &mut Map, owner: &str, targets: Array| -> bool {
            let values = targets
                .into_iter()
                .filter_map(|v| v.into_string().ok())
                .filter(|s| !s.is_empty())
                .collect();
            store_action(
                unsafe { &mut *body_ptr_targets },
                SocialAction::SetCombatTargets {
                    owner: owner.to_string(),
                    targets: values,
                },
            )
        },
    );

    let body_ptr_group_targets = body_ptr;
    engine.register_fn(
        "party_request_set_group_combat_targets",
        move |_ob: &mut Map, owners: Array, targets: Array, tanker: &str| -> bool {
            let owners = owners
                .into_iter()
                .filter_map(|value| value.into_string().ok())
                .filter(|owner| !owner.is_empty() && context_knows(owner))
                .collect::<Vec<_>>();
            let targets = targets
                .into_iter()
                .filter_map(|value| value.into_string().ok())
                .filter(|target| !target.is_empty())
                .collect::<Vec<_>>();
            let assignments = owners
                .into_iter()
                .map(|owner| (owner, targets.clone()))
                .collect();
            store_action(
                unsafe { &mut *body_ptr_group_targets },
                SocialAction::SetPartyCombatTargets {
                    assignments,
                    tanker: (!tanker.is_empty()).then(|| tanker.to_string()),
                },
            )
        },
    );

    let body_ptr_is_disconnect = body_ptr;
    engine.register_fn("is_party_disconnect", move |_ob: &mut Map| -> bool {
        matches!(
            unsafe { &*body_ptr_is_disconnect }
                .temp()
                .get(PARTY_DISCONNECT_REQUEST),
            Some(Value::Int(1))
        )
    });

    let body_ptr_send = body_ptr;
    engine.register_fn(
        "party_send_raw",
        move |_ob: &mut Map, connection_id: &str, raw_text: &str| -> bool {
            if raw_text.is_empty() || !context_knows(connection_id) {
                return false;
            }
            let body = unsafe { &mut *body_ptr_send };
            let current = body
                .temp()
                .get(PARTY_DELIVERY_REQUESTS)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mut deliveries: Vec<PartyDelivery> =
                serde_json::from_str(&current).unwrap_or_default();
            deliveries.push(PartyDelivery {
                connection_id: connection_id.to_string(),
                raw_text: raw_text.to_string(),
            });
            let Ok(json) = serde_json::to_string(&deliveries) else {
                return false;
            };
            body.temp_mut()
                .insert(PARTY_DELIVERY_REQUESTS.to_string(), Value::String(json));
            true
        },
    );
}

fn store_action(body: &mut Body, action: SocialAction) -> bool {
    let Ok(json) = serde_json::to_string(&action) else {
        return false;
    };
    body.temp_mut()
        .insert(PARTY_ACTION_REQUEST.to_string(), Value::String(json));
    true
}

fn empty_context() -> Map {
    let mut context = Map::new();
    context.insert("self_id".into(), Dynamic::from(String::new()));
    context.insert(
        "self".into(),
        missing_party_person(String::new(), RelationState::default()),
    );
    context.insert(
        "follow".into(),
        missing_party_person(String::new(), RelationState::default()),
    );
    context.insert("followers".into(), Dynamic::from(Array::new()));
    context.insert(
        "party_leader".into(),
        missing_party_person(String::new(), RelationState::default()),
    );
    context.insert("party_members".into(), Dynamic::from(Array::new()));
    context.insert("room_players".into(), Dynamic::from(Array::new()));
    context.insert("room_nonplayers".into(), Dynamic::from(Array::new()));
    context.insert("room_object_lookup_supported".into(), Dynamic::from(true));
    context
}

fn context_array(key: &str) -> Array {
    PRECOMPUTED_PARTY_CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|context| context.get(key))
            .and_then(|value| value.clone().try_cast::<Array>())
            .unwrap_or_default()
    })
}

fn context_array_contains(key: &str, connection_id: &str) -> bool {
    PRECOMPUTED_PARTY_CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|context| map_array_contains(context, key, connection_id))
    })
}

fn map_array_contains(context: &Map, key: &str, connection_id: &str) -> bool {
    context
        .get(key)
        .and_then(|value| value.clone().try_cast::<Array>())
        .unwrap_or_default()
        .iter()
        .any(|person| {
            person
                .clone()
                .try_cast::<Map>()
                .and_then(|person| person.get("id").cloned())
                .and_then(|id| id.into_string().ok())
                .is_some_and(|id| id == connection_id)
        })
}

fn context_knows(connection_id: &str) -> bool {
    if connection_id.is_empty() {
        return false;
    }
    PRECOMPUTED_PARTY_CONTEXT.with(|slot| {
        let context = slot.borrow();
        let Some(context) = context.as_ref() else {
            return false;
        };
        if context
            .get("self_id")
            .and_then(|value| value.clone().into_string().ok())
            .is_some_and(|id| id == connection_id)
        {
            return true;
        }
        ["followers", "party_members", "room_players"]
            .iter()
            .any(|key| map_array_contains(context, key, connection_id))
            || ["follow", "party_leader"].iter().any(|key| {
                context
                    .get(*key)
                    .and_then(|value| value.clone().try_cast::<Map>())
                    .and_then(|person| person.get("id").cloned())
                    .and_then(|id| id.into_string().ok())
                    .is_some_and(|id| id == connection_id)
            })
    })
}

fn validated_ids(key: &str, values: Array) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        let Ok(id) = value.into_string() else {
            continue;
        };
        if context_array_contains(key, &id) && !result.contains(&id) {
            result.push(id);
        }
    }
    result
}

fn find_follow_player(line: &str) -> Dynamic {
    let mut query = line.split_whitespace().next().unwrap_or("").to_string();
    if query.is_empty() || query == "." || query.chars().all(|character| character.is_ascii_digit())
    {
        return missing_party_person(String::new(), RelationState::default());
    }
    let digit_count = query.chars().take_while(char::is_ascii_digit).count();
    let order = if digit_count == 0 {
        1
    } else {
        query[..digit_count].parse::<usize>().unwrap_or(1)
    };
    if digit_count != 0 {
        query = query[digit_count..].to_string();
    }
    if query.is_empty() || order == 0 {
        return missing_party_person(String::new(), RelationState::default());
    }

    let lookup_supported = PRECOMPUTED_PARTY_CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|context| context.get("room_object_lookup_supported"))
            .and_then(|value| value.as_bool().ok())
            .unwrap_or(true)
    });
    if !lookup_supported {
        return unsupported_party_person();
    }

    // Python selects from one insertion-ordered room.objs list and only then
    // checks is_player(). Rust currently has separate player/mob/item stores.
    // If any non-player can compete for this query, do not guess which object
    // Python would have selected and do not allow a relation mutation.
    if context_array("room_nonplayers")
        .into_iter()
        .any(|candidate| nonplayer_matches(candidate, &query))
    {
        return unsupported_party_person();
    }

    let mut exact_count = 0usize;
    let mut prefix_count = 0usize;
    for person in context_array("room_players") {
        let Some(person) = person.try_cast::<Map>() else {
            continue;
        };
        if person
            .get("transparent")
            .and_then(|value| value.as_bool().ok())
            .unwrap_or(false)
        {
            continue;
        }
        let name = person
            .get("name")
            .and_then(|value| value.clone().into_string().ok())
            .unwrap_or_default();
        let reactions = person
            .get("reaction_names")
            .and_then(|value| value.clone().try_cast::<Array>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.into_string().ok())
            .collect::<Vec<_>>();

        if name == query || reactions.iter().any(|alias| alias == &query) {
            exact_count += 1;
            if exact_count == order {
                return Dynamic::from(person);
            }
        } else {
            for alias in reactions {
                if alias.starts_with(&query) {
                    prefix_count += 1;
                    if prefix_count == order {
                        return Dynamic::from(person);
                    }
                }
            }
        }
    }
    missing_party_person(String::new(), RelationState::default())
}

fn unsupported_party_person() -> Dynamic {
    let Some(mut unsupported) =
        missing_party_person(String::new(), RelationState::default()).try_cast::<Map>()
    else {
        return Dynamic::UNIT;
    };
    unsupported.insert("lookup_supported".into(), Dynamic::from(false));
    Dynamic::from(unsupported)
}

fn nonplayer_matches(candidate: Dynamic, query: &str) -> bool {
    let Some(candidate) = candidate.try_cast::<Map>() else {
        return false;
    };
    if candidate
        .get("transparent")
        .and_then(|value| value.as_bool().ok())
        .unwrap_or(false)
    {
        return false;
    }
    let kind = candidate
        .get("kind")
        .and_then(|value| value.clone().into_string().ok())
        .unwrap_or_default();
    let act = candidate
        .get("act")
        .and_then(|value| value.as_int().ok())
        .unwrap_or_default();
    if kind == "mob" {
        if query == "시체" && act == 2 {
            return true;
        }
        if query != "시체" && (act == 2 || act == 3) {
            return false;
        }
    }
    let name = candidate
        .get("name")
        .and_then(|value| value.clone().into_string().ok())
        .unwrap_or_default();
    if name == query {
        return true;
    }
    let reaction_is_array = candidate
        .get("reaction_is_array")
        .and_then(|value| value.as_bool().ok())
        .unwrap_or(true);
    if !reaction_is_array {
        // Python `name in obj.get('반응이름')` uses substring
        // membership when a legacy Box stores this field as a scalar string.
        return candidate
            .get("reaction_raw")
            .and_then(|value| value.clone().into_string().ok())
            .is_some_and(|raw| raw.contains(query));
    }
    candidate
        .get("reaction_names")
        .and_then(|value| value.clone().try_cast::<Array>())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.into_string().ok())
        .any(|alias| alias == query || alias.starts_with(query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{ScriptConfig, ScriptStorage};
    use crate::world::{get_world_state, PlayerPosition};

    fn body(name: &str, nickname: &str, personality: &str, hp: i64, max_hp: i64) -> Body {
        let mut body = Body::new();
        body.set("이름", name);
        body.set("무림별호", nickname);
        body.set("성격", personality);
        body.set("체력", hp);
        body.set("최고체력", max_hp);
        body.set("내공", 7_i64);
        body.set("최고내공", 9_i64);
        body.set("설정상태", "엘피출력 0 동행거부 0");
        body.set("위치", "낙양성:42");
        body
    }

    fn person(id: &str, body: &Body, follow: Option<&str>, party: Option<&str>) -> Dynamic {
        build_party_person_snapshot(
            id.to_string(),
            body,
            RelationState {
                follow: follow.map(str::to_string),
                party_leader: party.map(str::to_string),
            },
            1,
        )
    }

    fn context(
        self_person: Dynamic,
        follow: Option<Dynamic>,
        followers: Vec<Dynamic>,
        leader: Option<Dynamic>,
        members: Vec<Dynamic>,
        room: Vec<Dynamic>,
    ) -> Map {
        let self_id = self_person.clone().try_cast::<Map>().unwrap()["id"]
            .clone()
            .into_string()
            .unwrap();
        let mut context = Map::new();
        context.insert("self_id".into(), Dynamic::from(self_id));
        context.insert("self".into(), self_person);
        context.insert(
            "follow".into(),
            follow.unwrap_or_else(|| missing_party_person(String::new(), RelationState::default())),
        );
        context.insert("followers".into(), Dynamic::from(followers));
        context.insert(
            "party_leader".into(),
            leader.unwrap_or_else(|| missing_party_person(String::new(), RelationState::default())),
        );
        context.insert("party_members".into(), Dynamic::from(members));
        context.insert("room_players".into(), Dynamic::from(room));
        context
    }

    fn run(script: &str, actor: &mut Body, line: &str, context: Map) -> Vec<String> {
        set_precomputed_party_context(context);
        let storage = ScriptStorage::new(ScriptConfig::default());
        storage
            .execute(script, actor, line, None, None, None)
            .unwrap()
            .0
    }

    #[test]
    fn party_combat_commands_compile_and_keep_python_no_party_guard() {
        let mut actor = body("전투무리없음", "", "정파", 45, 90);
        let actor_person = person("actor-token", &actor, None, None);
        let empty = context(actor_person, None, vec![], None, vec![], vec![]);
        assert_eq!(
            run("무리합", &mut actor, "", empty.clone()),
            vec!["☞ 당신이 속한 무리가 없어요. ^^"]
        );
        // Python 방어지정 emits usage and continues into the no-party guard.
        assert_eq!(
            run("방어지정", &mut actor, "", empty),
            vec![
                "☞ 사용법 : [무리원이름] 방어지정",
                "☞ 당신이 속한 무리가 없어요. ^^"
            ]
        );
    }

    #[test]
    fn party_join_batches_every_nonfighter_instead_of_overwriting_requests() {
        let mut leader = body("대장", "", "정파", 45, 90);
        let mut fighter = body("싸움꾼", "", "정파", 45, 90);
        fighter.act = crate::player::ActState::Fight;
        fighter.temp_mut().insert(
            "_combat_target_ids".to_string(),
            Value::String("몹:하나\n몹:둘".to_string()),
        );
        let idle = body("대기자", "", "정파", 45, 90);
        let leader_person = person("leader", &leader, None, Some("leader"));
        let fighter_person = person("fighter", &fighter, None, Some("leader"));
        let idle_person = person("idle", &idle, None, Some("leader"));
        let ctx = context(
            leader_person.clone(),
            None,
            vec![],
            Some(leader_person),
            vec![fighter_person, idle_person],
            vec![],
        );
        assert_eq!(
            run("무리합", &mut leader, "", ctx),
            vec!["당신이 속한 무리가 무리합동 공격을 시작 합니다."]
        );
        let (action, _) = take_party_requests(&mut leader);
        assert!(matches!(
            action,
            Some(SocialAction::SetPartyCombatTargets { assignments, tanker: None })
                if assignments == vec![
                    ("leader".to_string(), vec!["몹:하나".to_string(), "몹:둘".to_string()]),
                    ("idle".to_string(), vec!["몹:하나".to_string(), "몹:둘".to_string()]),
                ]
        ));
    }

    #[test]
    fn follow_script_uses_room_snapshot_and_python_body_messages() {
        let mut actor = body("철수", "", "정파", 45, 90);
        let target = body("영희", "", "정파", 31, 45);
        let actor_person = person("actor-token", &actor, None, None);
        let target_person = person("target-token", &target, None, None);
        let context = context(
            actor_person.clone(),
            None,
            vec![],
            None,
            vec![],
            vec![actor_person, target_person],
        );

        let output = run("따라", &mut actor, "영희", context);
        assert_eq!(
            output,
            ["당신은 \x1b[1m영희\x1b[0;37m를 따라다니기 시작합니다."]
        );
        let (action, deliveries) = take_party_requests(&mut actor);
        assert_eq!(
            action,
            Some(SocialAction::Follow {
                target: "target-token".to_string()
            })
        );
        assert_eq!(
            deliveries,
            [PartyDelivery {
                connection_id: "target-token".to_string(),
                raw_text: concat!(
                    "\r\n\x1b[1m철수\x1b[0;37m가 당신을 따라다니기 시작합니다.\r\n",
                    "\r\n\x1b[0;37;40m[ 31/45, 7/9 ] "
                )
                .to_string()
            }]
        );
        clear_precomputed_party_context();
    }

    #[test]
    fn follow_script_honors_python_refusal_before_any_relation_request() {
        let mut actor = body("철수거부검사", "", "정파", 45, 90);
        let mut target = body("영희거부중", "", "정파", 31, 45);
        target.set("설정상태", "엘피출력 0\n동행거부 1");
        let actor_person = person("actor-token", &actor, None, None);
        let target_person = person("target-token", &target, None, None);
        let refusal_context = context(
            actor_person.clone(),
            None,
            vec![],
            None,
            vec![],
            vec![actor_person, target_person],
        );

        let output = run("따라", &mut actor, "영희거부중", refusal_context);
        assert_eq!(output, ["[1m영희거부중[0;37m이 동행거부중 입니다."]);
        let (action, deliveries) = take_party_requests(&mut actor);
        assert_eq!(action, None);
        assert!(deliveries.is_empty());
        clear_precomputed_party_context();
    }

    #[test]
    fn compressed_floor_item_competitor_silently_blocks_unordered_follow_lookup() {
        let compressed = super::super::build_room_mugong_stack_item_snapshot("289", 1)
            .expect("data/item/289.json is required by existing item command tests");
        let competitor = build_party_nonplayer_snapshot(&compressed);
        let target_name = competitor.clone().try_cast::<Map>().unwrap()["name"]
            .clone()
            .into_string()
            .unwrap();
        let mut actor = body("압축바닥경쟁검사", "", "정파", 45, 90);
        let target = body(&target_name, "", "정파", 31, 45);
        let actor_person = person("actor-token", &actor, None, None);
        let target_person = person("target-token", &target, None, None);
        let mut ambiguous_context = context(
            actor_person.clone(),
            None,
            vec![],
            None,
            vec![],
            vec![actor_person, target_person],
        );
        ambiguous_context.insert("room_nonplayers".into(), Dynamic::from(vec![competitor]));

        let output = run("따라", &mut actor, &target_name, ambiguous_context);
        assert!(output.is_empty());
        let (action, deliveries) = take_party_requests(&mut actor);
        assert_eq!(action, None);
        assert!(deliveries.is_empty());
        clear_precomputed_party_context();
    }

    #[test]
    fn legacy_scalar_box_alias_competes_but_failed_empty_box_does_not() {
        let mut legacy_box = Map::new();
        legacy_box.insert("name".into(), Dynamic::from("보관상자"));
        legacy_box.insert("reaction_names".into(), Dynamic::from(Array::new()));
        legacy_box.insert("reaction_raw".into(), Dynamic::from("창고보관함"));
        legacy_box.insert("reaction_is_array".into(), Dynamic::from(false));
        legacy_box.insert("transparent".into(), Dynamic::from(false));
        legacy_box.insert("kind".into(), Dynamic::from("box"));
        legacy_box.insert("act".into(), Dynamic::from(0_i64));
        assert!(nonplayer_matches(Dynamic::from(legacy_box), "고보"));

        let mut failed_empty_box = Map::new();
        failed_empty_box.insert("name".into(), Dynamic::from(String::new()));
        failed_empty_box.insert("reaction_names".into(), Dynamic::from(Array::new()));
        failed_empty_box.insert("reaction_raw".into(), Dynamic::from(String::new()));
        failed_empty_box.insert("reaction_is_array".into(), Dynamic::from(false));
        failed_empty_box.insert("transparent".into(), Dynamic::from(false));
        failed_empty_box.insert("kind".into(), Dynamic::from("box"));
        failed_empty_box.insert("act".into(), Dynamic::from(0_i64));
        assert!(!nonplayer_matches(
            Dynamic::from(failed_empty_box),
            "보관상자"
        ));
    }

    #[test]
    fn party_add_and_chat_keep_member_room_order_and_raw_prompts() {
        let mut leader = body("대장", "별호", "정파", 90, 90);
        let member = body("무리원", "", "기인", 45, 90);
        let observer = body("구경꾼", "", "정파", 20, 40);
        let leader_person = person("leader", &leader, None, None);
        let member_person = person("member", &member, Some("leader"), None);
        let observer_person = person("observer", &observer, None, None);
        let add_context = context(
            leader_person.clone(),
            None,
            vec![member_person.clone()],
            None,
            vec![],
            vec![
                leader_person.clone(),
                member_person.clone(),
                observer_person,
            ],
        );

        let output = run("무리", &mut leader, "무리원", add_context);
        assert_eq!(
            output,
            ["당신의 무리에 \x1b[1m무리원\x1b[0m\x1b[40m\x1b[37m이 들어옵니다."]
        );
        let (action, deliveries) = take_party_requests(&mut leader);
        assert_eq!(
            action,
            Some(SocialAction::AddPartyMembers {
                members: vec!["member".to_string()]
            })
        );
        assert_eq!(deliveries.len(), 2);
        assert_eq!(deliveries[0].connection_id, "member");
        assert!(deliveries[0]
            .raw_text
            .ends_with("\r\n\x1b[0;37;40m[ 45/90, 7/9 ] "));
        assert_eq!(deliveries[1].connection_id, "observer");

        let member_as_actor = person("member", &member, Some("leader"), Some("leader"));
        let leader_in_party = person("leader", &leader, None, Some("leader"));
        let chat_context = context(
            member_as_actor.clone(),
            Some(leader_in_party.clone()),
            vec![],
            Some(leader_in_party),
            vec![member_as_actor],
            vec![],
        );
        let mut member_body = member;
        get_world_state().write().unwrap().set_player_position(
            "무리원",
            PlayerPosition::new("낙양성".to_string(), "42".to_string()),
        );
        let chat_output = run("무리말", &mut member_body, "안녕", chat_context);
        get_world_state()
            .write()
            .unwrap()
            .remove_player_position("무리원");
        assert_eq!(chat_output.len(), 1);
        assert!(chat_output[0].contains("◁"));
        let (_, chat_deliveries) = take_party_requests(&mut member_body);
        assert_eq!(chat_deliveries.len(), 1);
        assert_eq!(chat_deliveries[0].connection_id, "leader");
        assert!(chat_deliveries[0]
            .raw_text
            .ends_with("\r\n\x1b[0;37;40m[ 90/90, 7/9 ] "));
        clear_precomputed_party_context();
    }

    #[test]
    fn party_view_uses_python_hp_bar_and_percent_width() {
        let mut leader = body("대장", "별호", "정파", 45, 90);
        let member = body("무리원", "", "기인", 90, 90);
        let leader_person = person("leader", &leader, None, Some("leader"));
        let member_person = person("member", &member, Some("leader"), Some("leader"));
        let view_context = context(
            leader_person.clone(),
            None,
            vec![member_person.clone()],
            Some(leader_person),
            vec![member_person],
            vec![],
        );

        let output = run("무리", &mut leader, "", view_context);
        assert_eq!(output[0], "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        assert_eq!(output[2], "────────────────────────────");
        assert!(output[3].starts_with("▶ \x1b[1m\x1b[33m\x1b[40m별호"));
        assert!(output[3].contains(" 50 \x1b[0m\x1b[43m\x1b[30m"));
        assert!(output[3].ends_with("    7/9    "));
        assert!(output[4].starts_with("　 무명객"));
        assert_eq!(output[6], "동행중");
        assert_eq!(output.last().unwrap(), "━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let (action, deliveries) = take_party_requests(&mut leader);
        assert_eq!(action, None);
        assert!(deliveries.is_empty());
        clear_precomputed_party_context();
    }

    #[test]
    fn party_remove_all_keeps_python_follower_and_party_member_iteration_order() {
        let mut leader = body("제외대장", "", "정파", 90, 90);
        let first = body("일원", "", "정파", 80, 90);
        let second = body("이원", "", "정파", 70, 90);
        let loose = body("동행자", "", "정파", 60, 90);
        let leader_person = person("leader", &leader, None, Some("leader"));
        let first_person = person("first", &first, Some("leader"), Some("leader"));
        let second_person = person("second", &second, Some("leader"), Some("leader"));
        let loose_person = person("loose", &loose, Some("leader"), None);
        let remove_context = context(
            leader_person.clone(),
            None,
            vec![first_person.clone(), second_person.clone(), loose_person],
            Some(leader_person),
            vec![first_person, second_person],
            vec![],
        );

        let output = run("무리제외", &mut leader, "모두", remove_context);
        assert_eq!(output.len(), 3);
        assert!(output[0].contains("일원"));
        assert!(output[1].contains("이원"));
        assert!(output[2].contains("동행자"));
        let (action, deliveries) = take_party_requests(&mut leader);
        assert_eq!(
            action,
            Some(SocialAction::RemoveFollowers {
                members: vec![
                    "first".to_string(),
                    "second".to_string(),
                    "loose".to_string()
                ]
            })
        );
        assert_eq!(
            deliveries
                .iter()
                .map(|delivery| delivery.connection_id.as_str())
                .collect::<Vec<_>>(),
            ["second", "first", "second", "loose"]
        );
        assert!(deliveries[0].raw_text.contains("일원"));
        assert!(deliveries[1]
            .raw_text
            .contains("의 무리에서 당신을 제외시킵니다."));
        assert!(deliveries[3]
            .raw_text
            .contains("더이상 따라다니지 못하게 합니다."));
        clear_precomputed_party_context();
    }

    #[test]
    fn member_logout_keeps_bound_leader_send_to_party_crlf_and_prompt_order() {
        let leader = body("대장로그아웃", "", "정파", 90, 90);
        let mut actor = body("나가는이", "", "정파", 45, 90);
        let other = body("남은이", "", "정파", 60, 90);
        let leader_person = person("leader", &leader, None, Some("leader"));
        let actor_person = person("actor", &actor, Some("leader"), Some("leader"));
        let other_person = person("other", &other, Some("leader"), Some("leader"));
        let logout_context = context(
            actor_person.clone(),
            Some(leader_person.clone()),
            vec![],
            Some(leader_person),
            vec![actor_person, other_person],
            vec![],
        );
        actor
            .temp_mut()
            .insert(PARTY_DISCONNECT_REQUEST.to_string(), Value::Int(1));

        let output = run("무리", &mut actor, "", logout_context);
        assert!(output.is_empty());
        let (action, deliveries) = take_party_requests(&mut actor);
        assert_eq!(action, Some(SocialAction::Disconnect));
        assert_eq!(deliveries.len(), 5);
        assert_eq!(deliveries[0].connection_id, "leader");
        assert_eq!(
            deliveries[0].raw_text,
            "\r\n\x1b[1m나가는이\x1b[0;37m가 무리에서 이탈 하였습니다.\r\n"
        );
        assert_eq!(deliveries[1].connection_id, "other");
        assert_eq!(
            deliveries[1].raw_text,
            concat!(
                "\r\n\r\n\x1b[1m나가는이\x1b[0;37m가 무리에서 이탈 하였습니다.\r\n",
                "\r\n\x1b[0;37;40m[ 60/90, 7/9 ] "
            )
        );
        assert_eq!(
            deliveries[2],
            PartyDelivery {
                connection_id: "leader".to_string(),
                raw_text: "\r\n\x1b[0;37;40m[ 90/90, 7/9 ] ".to_string(),
            }
        );
        assert_eq!(deliveries[3].connection_id, "actor");
        assert_eq!(deliveries[4].connection_id, "leader");
        clear_precomputed_party_context();
    }
}
