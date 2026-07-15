//! World module for MUD game world
//!
//! This module contains structures and functionality for managing
//! the game world including rooms, zones, mobs, items, and navigation.

pub mod difficulty;
pub mod event;
pub mod event_binding;
pub mod fixture;
pub mod guild;
pub mod item;
pub mod mob;
pub mod nickname;
pub mod rank;
pub mod room;
pub mod skill;
pub mod tracking;
pub mod user_home;

#[cfg(test)]
mod event_directive_test;

// Re-export commonly used types
pub use difficulty::{base_zone_name, difficulty_from_zone, DifficultyConfig, DifficultyLevel};
pub use event_binding::{EventBindings, EventScript};
pub use fixture::{Fixture, FixtureKind, FixturePlacement};

pub use room::{
    format_exits_long, format_room_header, get_room, handle_player_enter, handle_player_exit,
    Direction, EnterMode, Exit, ExitMode, Room, RoomCache, RoomError,
};

pub use mob::{get_mob_cache, MobCache, MobError, MobInstance, MobSkillEffect, RawMobData};

pub use skill::{
    calculate_normal_attacks, get_skill, get_skill_cache, get_skill_defense_head, PatternAction,
    PatternElement, Skill, SkillCache, SkillType,
};

pub use item::{
    create_item, find_or_create_item, get_item_cache, ItemCache, ItemError, ItemInstance,
    RawItemData,
};

use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::object::{Object, Value};

const FLOOR_ITEM_LIFETIME_SECONDS: f64 = 600.0;
const FLOOR_ITEM_DROP_TIME_KEY: &str = "timeofdrop";

/// Player position in the world
#[derive(Debug, Clone)]
pub struct PlayerPosition {
    /// Zone name
    pub zone: String,
    /// Room id (숫자 "1","42" 또는 사용자맵 "이름")
    pub room: String,
    /// Last movement timestamp
    pub last_move: i64,
}

/// Python `Room.objs` identity used for cross-type lookup ordering.
///
/// The existing room_players index is intentionally kept separate because it
/// has callers that need only player names. This list records the Python
/// insert-at-front event order for commands that must distinguish players
/// from mobs/items/boxes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomObjectRef {
    Player(String),
    SummonedUser(u64),
    /// Stable runtime identity, allowing multiple clones of one template.
    Mob(u64),
    FloorItem(usize),
    /// Stable ordinal into the lazily loaded Python `설치리스트` Box vector.
    InstalledBox(usize),
    Box(usize),
    /// Stable runtime identity of a room-bound interactive fixture.
    Fixture(u64),
}

impl PlayerPosition {
    /// Create a new position. room은 숫자 문자열("1") 또는 사용자맵 방 이름("도토리") 등.
    pub fn new(zone: String, room: String) -> Self {
        Self {
            zone,
            room,
            last_move: chrono::Utc::now().timestamp(),
        }
    }

    /// Create starting position (Python 호환: 낙양성:42 왕대협)
    pub fn start() -> Self {
        Self::new("낙양성".to_string(), "42".to_string())
    }

    /// Create fallback starting position (same as start - Python 호환: 낙양성:42 왕대협)
    pub fn start_fallback() -> Self {
        Self::new("낙양성".to_string(), "42".to_string())
    }

    /// Get the position key for room lookup
    pub fn room_key(&self) -> String {
        format!("{}:{}", self.zone, self.room)
    }
}

/// Full player data for room display
#[derive(Debug, Clone)]
pub struct PlayerRoomData {
    /// Player name
    pub name: String,
    /// Player level
    pub level: i64,
    /// Current HP
    pub hp: i64,
    /// Maximum HP
    pub max_hp: i64,
    /// Guild name (if any)
    pub guild: Option<String>,
    /// Rank/position in guild (if any)
    pub rank: Option<String>,
    /// Current action state (Stand, Fight, etc.)
    pub act_state: String,
}

/// World state containing all active entities
#[derive(Debug)]
pub struct WorldState {
    /// Player positions indexed by player name
    player_positions: HashMap<String, PlayerPosition>,
    /// Same-room player index. Values retain room insertion order so Rhai
    /// room-local commands do not have to scan every online position.
    room_players: HashMap<String, Vec<String>>,
    /// Socket-less Player objects created by Python `사용자몹소환`.
    summoned_users: Vec<SummonedUser>,
    next_summoned_user_id: u64,
    /// Runtime fixtures keyed by stable identity.
    fixtures: HashMap<u64, Fixture>,
    /// Per-room fixture identity index. Values preserve placement order.
    room_fixtures: HashMap<String, Vec<u64>>,
    next_fixture_id: u64,
    /// Room cache
    pub room_cache: RoomCache,
    /// Mob cache
    pub mob_cache: MobCache,
    /// Item cache
    pub item_cache: ItemCache,
    /// Objects on room floor: key = "zone:room" (e.g. "낙양성:1"), value = items (non-stackable)
    pub room_objs: HashMap<String, Vec<Arc<Mutex<Object>>>>,
    /// Stackable items on room floor: "zone:room" -> (인덱스 -> 수량)
    pub room_inv_stack: HashMap<String, HashMap<String, i64>>,
    /// Python-compatible unified Room.objs order (newest object first).
    room_object_order: HashMap<String, Vec<RoomObjectRef>>,
    /// 방별 임의 속성 (값설정 "방" 키 값). key = "zone:room", value = attr map.
    pub room_attrs: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug)]
pub struct SummonedUser {
    pub id: u64,
    pub body: crate::player::Body,
    pub position: PlayerPosition,
}

/// A Python `Room.writeRoom` payload produced by a periodic mob update.
/// Formatting remains in the source mob data; delivery is performed by the
/// network loop after releasing the world lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMobMessage {
    pub zone: String,
    pub room: String,
    pub kind: RoomMobMessageKind,
    pub message: String,
    pub mob_name: String,
    pub revealed_items: Vec<RevealedFloorItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealedFloorItem {
    pub name: String,
    pub ansi: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomMobMessageKind {
    Speech,
    CorpseGone,
    Respawn,
}

/// Data-only record of a Python `Item.update()` expiration.  Rhai owns the
/// eventual Korean/ANSI room notice formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredFloorItem {
    pub zone: String,
    pub room: String,
    pub name: String,
}

/// A loaded room branch that Rust cannot yet update with Python `Room.update()`
/// semantics. The updater preflights these branches to avoid partial world
/// mutation; the server may still continue an administrator-requested reboot
/// because this one-tick room state is transient and is not persisted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RebootRoomUpdateBlock {
    #[error("loaded room disappeared from cache: {zone}:{room}")]
    MissingRoom { zone: String, room: String },
    #[error("loaded room lock is poisoned: {zone}:{room}")]
    PoisonedRoom { zone: String, room: String },
    #[error("floor-item lifetime is not represented: {zone}:{room}")]
    FloorItems { zone: String, room: String },
    #[error("configured mobs have no runtime instances: {zone}:{room}")]
    MissingMobInstances { zone: String, room: String },
    #[error("mob template is unavailable: {zone}:{room}:{mob}")]
    MissingMobTemplate {
        zone: String,
        room: String,
        mob: String,
    },
    #[error("mob update branch is not represented: {zone}:{room}:{mob}")]
    UnsupportedMobUpdate {
        zone: String,
        room: String,
        mob: String,
    },
    #[error("mob tick cannot be incremented: {zone}:{room}:{mob}")]
    MobTickOverflow {
        zone: String,
        room: String,
        mob: String,
    },
}

fn reboot_mob_update_is_exact_noop(mob: &MobInstance, data: &RawMobData) -> bool {
    if !mob.alive
        || mob.act != 0
        || mob.hp != mob.max_hp
        || mob.mp != mob.max_mp
        || !mob.targets.is_empty()
        || !mob.skills.is_empty()
        || !mob.skill_effects.is_empty()
        || mob.str_modifier != 0
        || mob.dex_modifier != 0
        || mob.arm_modifier != 0
        || mob.mp_modifier != 0
        || mob.max_mp_modifier != 0
        || mob.hp_modifier != 0
        || mob.max_hp_modifier != 0
    {
        return false;
    }

    // Python's type-6 item regeneration needs `timeofregen`, which Rust does
    // not retain.  Aggressive combat needs live player visibility/targets.
    if data.mob_type == 6 || data.combat_type == 1 {
        return false;
    }

    // A matching talk tick consumes Python RNG, may speak, and makes
    // Room.update print prompts.  Between matching ticks it is a no-op.
    let Some(next_tick) = mob.tick.checked_add(1) else {
        return false;
    };
    data.talk_tick == 0 || next_tick % data.talk_tick != 0
}

impl WorldState {
    /// Create a new world state
    pub fn new() -> Self {
        Self {
            player_positions: HashMap::new(),
            room_players: HashMap::new(),
            summoned_users: Vec::new(),
            next_summoned_user_id: 1,
            fixtures: HashMap::new(),
            room_fixtures: HashMap::new(),
            next_fixture_id: 1,
            room_cache: RoomCache::new(),
            mob_cache: MobCache::new(),
            item_cache: ItemCache::new(),
            room_objs: HashMap::new(),
            room_inv_stack: HashMap::new(),
            room_object_order: HashMap::new(),
            room_attrs: HashMap::new(),
        }
    }

    pub fn add_summoned_user(
        &mut self,
        body: crate::player::Body,
        position: PlayerPosition,
    ) -> u64 {
        let id = self.next_summoned_user_id;
        self.next_summoned_user_id = self.next_summoned_user_id.saturating_add(1);
        self.record_room_object(
            &position.zone,
            &position.room,
            RoomObjectRef::SummonedUser(id),
        );
        self.summoned_users
            .push(SummonedUser { id, body, position });
        id
    }

    pub fn summoned_users(&self) -> &[SummonedUser] {
        &self.summoned_users
    }

    pub fn summoned_user_mut(&mut self, id: u64) -> Option<&mut SummonedUser> {
        self.summoned_users.iter_mut().find(|user| user.id == id)
    }

    pub fn take_summoned_user_by_name(&mut self, name: &str) -> Option<SummonedUser> {
        let index = self
            .summoned_users
            .iter()
            .position(|user| user.body.get_name() == name)?;
        let user = self.summoned_users.remove(index);
        self.remove_room_object(
            &user.position.zone,
            &user.position.room,
            &RoomObjectRef::SummonedUser(user.id),
        );
        Some(user)
    }

    pub fn restore_summoned_user(&mut self, mut user: SummonedUser, position: PlayerPosition) {
        user.position = position;
        self.record_room_object(
            &user.position.zone,
            &user.position.room,
            RoomObjectRef::SummonedUser(user.id),
        );
        self.summoned_users.push(user);
    }

    pub fn summoned_users_in_room(&self, zone: &str, room: &str) -> Vec<&SummonedUser> {
        self.summoned_users
            .iter()
            .filter(|user| user.position.zone == zone && user.position.room == room)
            .collect()
    }

    /// Python 제거 명령 scans channel.players and deletes the first matching
    /// socket-less Player regardless of the administrator's current room.
    pub fn remove_summoned_user(&mut self, name: &str) -> bool {
        let Some(index) = self
            .summoned_users
            .iter()
            .position(|user| user.body.get_name() == name)
        else {
            return false;
        };
        let user = self.summoned_users.remove(index);
        self.remove_room_object(
            &user.position.zone,
            &user.position.room,
            &RoomObjectRef::SummonedUser(user.id),
        );
        true
    }

    pub fn remove_summoned_user_by_id(&mut self, id: u64) -> bool {
        let Some(index) = self.summoned_users.iter().position(|user| user.id == id) else {
            return false;
        };
        let user = self.summoned_users.remove(index);
        self.remove_room_object(
            &user.position.zone,
            &user.position.room,
            &RoomObjectRef::SummonedUser(user.id),
        );
        true
    }

    /// Safe counterpart of Python `사용자몹제거`: select only a socket-less
    /// summoned Player in the administrator's current room.  The legacy code
    /// could also detach a real connected player from channel.players, which
    /// is an unsafe Python bug and is intentionally not reproduced.
    pub fn remove_summoned_user_in_room(&mut self, zone: &str, room: &str, query: &str) -> bool {
        let first = query.split_whitespace().next().unwrap_or("");
        if first.is_empty() || first.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        let room_key = format!("{zone}:{room}");
        let ordered = self
            .room_object_order
            .get(&room_key)
            .cloned()
            .unwrap_or_default();
        let selected = ordered.into_iter().find_map(|object| {
            let RoomObjectRef::SummonedUser(id) = object else {
                return None;
            };
            let user = self.summoned_users.iter().find(|user| user.id == id)?;
            let name = user.body.get_name();
            let aliases = user.body.get_string("반응이름");
            (name == first
                || aliases
                    .split(['\r', '\n'])
                    .any(|alias| !alias.is_empty() && (alias == first || alias.starts_with(first))))
            .then_some(id)
        });
        let Some(id) = selected else { return false };
        let Some(index) = self.summoned_users.iter().position(|user| user.id == id) else {
            return false;
        };
        self.summoned_users.remove(index);
        self.remove_room_object(zone, room, &RoomObjectRef::SummonedUser(id));
        true
    }

    /// 방 속성 (값설정 "방"용). key = "zone:room". room은 "1" 또는 사용자맵 "이름" 등.
    pub fn get_room_attrs_mut(&mut self, zone: &str, room: &str) -> &mut HashMap<String, String> {
        let key = format!("{}:{}", zone, room);
        self.room_attrs.entry(key).or_default()
    }

    /// Get or create the list of objects on a room's floor. Key: "zone:room".
    pub fn get_room_objs_mut(&mut self, zone: &str, room: &str) -> &mut Vec<Arc<Mutex<Object>>> {
        let key = format!("{}:{}", zone, room);
        self.room_objs.entry(key).or_default()
    }

    /// Get a copy of the list of objects on a room's floor (for display). Key: "zone:room".
    pub fn get_room_objs(&self, zone: &str, room: &str) -> Vec<Arc<Mutex<Object>>> {
        let key = format!("{}:{}", zone, room);
        self.room_objs.get(&key).cloned().unwrap_or_default()
    }

    /// Return the currently recorded Python `Room.objs` order. The list is
    /// newest-first, matching `Object.insert()`'s `insert(0, obj)` behavior.
    pub fn get_room_object_order(&self, zone: &str, room: &str) -> Vec<RoomObjectRef> {
        self.room_object_order
            .get(&format!("{}:{}", zone, room))
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_summoned_user_name(&self, id: u64) -> Option<String> {
        self.summoned_users
            .iter()
            .find(|user| user.id == id)
            .map(|user| user.body.get_name())
    }

    /// Place a new fixture at the front of Python-compatible `Room.objs`.
    pub fn create_fixture(
        &mut self,
        zone: &str,
        room: &str,
        kind: FixtureKind,
        attributes: HashMap<String, serde_json::Value>,
    ) -> u64 {
        let id = self.next_fixture_id;
        self.next_fixture_id = self.next_fixture_id.saturating_add(1);
        self.fixtures
            .insert(id, Fixture::new(id, kind, zone, room, attributes));
        self.room_fixtures
            .entry(format!("{zone}:{room}"))
            .or_default()
            .insert(0, id);
        self.record_room_object(zone, room, RoomObjectRef::Fixture(id));
        id
    }

    pub fn get_fixture(&self, id: u64) -> Option<&Fixture> {
        self.fixtures.get(&id)
    }

    pub fn get_fixture_mut(&mut self, id: u64) -> Option<&mut Fixture> {
        self.fixtures.get_mut(&id)
    }

    /// Return room fixtures newest-first, matching object insertion order.
    pub fn get_room_fixtures(&self, zone: &str, room: &str) -> Vec<&Fixture> {
        self.room_fixtures
            .get(&format!("{zone}:{room}"))
            .into_iter()
            .flatten()
            .filter_map(|id| self.fixtures.get(id))
            .collect()
    }

    pub fn move_fixture(&mut self, id: u64, zone: &str, room: &str) -> bool {
        let Some(previous) = self
            .fixtures
            .get(&id)
            .map(|fixture| (fixture.zone.clone(), fixture.room.clone()))
        else {
            return false;
        };
        if previous.0 == zone && previous.1 == room {
            return true;
        }
        self.remove_fixture_from_room_index(&previous.0, &previous.1, id);
        self.remove_room_object(&previous.0, &previous.1, &RoomObjectRef::Fixture(id));
        let Some(fixture) = self.fixtures.get_mut(&id) else {
            return false;
        };
        fixture.zone = zone.to_string();
        fixture.room = room.to_string();
        self.room_fixtures
            .entry(format!("{zone}:{room}"))
            .or_default()
            .insert(0, id);
        self.record_room_object(zone, room, RoomObjectRef::Fixture(id));
        true
    }

    pub fn remove_fixture(&mut self, id: u64) -> Option<Fixture> {
        let fixture = self.fixtures.remove(&id)?;
        self.remove_fixture_from_room_index(&fixture.zone, &fixture.room, id);
        self.remove_room_object(&fixture.zone, &fixture.room, &RoomObjectRef::Fixture(id));
        Some(fixture)
    }

    fn remove_fixture_from_room_index(&mut self, zone: &str, room: &str, id: u64) {
        let key = format!("{zone}:{room}");
        let empty = if let Some(fixtures) = self.room_fixtures.get_mut(&key) {
            fixtures.retain(|fixture_id| *fixture_id != id);
            fixtures.is_empty()
        } else {
            false
        };
        if empty {
            self.room_fixtures.remove(&key);
        }
    }

    fn record_room_object(&mut self, zone: &str, room: &str, object: RoomObjectRef) {
        let objects = self
            .room_object_order
            .entry(format!("{}:{}", zone, room))
            .or_default();
        objects.retain(|existing| existing != &object);
        objects.insert(0, object);
    }

    #[cfg(test)]
    pub(crate) fn record_test_room_object(
        &mut self,
        zone: &str,
        room: &str,
        object: RoomObjectRef,
    ) {
        self.record_room_object(zone, room, object);
    }

    /// Record a non-stackable floor object after Python `Room.insert(obj)`.
    pub fn record_floor_item(&mut self, zone: &str, room: &str, item: &Arc<Mutex<Object>>) {
        self.record_room_object(
            zone,
            room,
            RoomObjectRef::FloorItem(Arc::as_ptr(item) as usize),
        );
    }

    /// Reconcile floor objects inserted through a bulk operation with the
    /// unified Python Room.objs order. `room_objs` is newest-first, so unseen
    /// entries are recorded oldest-first to preserve every prepend event.
    pub fn sync_floor_item_order(&mut self, zone: &str, room: &str) {
        let floor = self.get_room_objs(zone, room);
        let recorded = self.get_room_object_order(zone, room);
        for item in floor.iter().rev() {
            let reference = RoomObjectRef::FloorItem(Arc::as_ptr(item) as usize);
            if !recorded.contains(&reference) {
                self.record_room_object(zone, room, reference);
            }
        }
    }

    pub fn remove_floor_item_record(&mut self, zone: &str, room: &str, item: &Arc<Mutex<Object>>) {
        self.remove_room_object(
            zone,
            room,
            &RoomObjectRef::FloorItem(Arc::as_ptr(item) as usize),
        );
    }

    /// Record an installed Box after Python `Room.insert(box)`.
    pub fn record_box(&mut self, zone: &str, room: &str, object: &Arc<Mutex<Object>>) {
        self.record_room_object(zone, room, RoomObjectRef::Box(Arc::as_ptr(object) as usize));
    }

    fn remove_room_object(&mut self, zone: &str, room: &str, object: &RoomObjectRef) {
        let key = format!("{}:{}", zone, room);
        if let Some(objects) = self.room_object_order.get_mut(&key) {
            objects.retain(|existing| existing != object);
            if objects.is_empty() {
                self.room_object_order.remove(&key);
            }
        }
    }

    /// Remove one concrete mob object from a room, including its Python-style
    /// `Room.objs` ordering entry.
    pub fn remove_room_mob_instance(&mut self, zone: &str, room: &str, instance_id: u64) -> bool {
        let Some(mobs) = self.mob_cache.get_all_mobs_in_room_mut(zone, room) else {
            return false;
        };
        let Some(index) = mobs.iter().position(|mob| mob.instance_id == instance_id) else {
            return false;
        };
        mobs.remove(index);
        self.remove_room_object(zone, room, &RoomObjectRef::Mob(instance_id));
        true
    }

    /// Get or create stackable items on a room's floor. Key: "zone:room", inner: 인덱스 -> 수량.
    pub fn get_room_objs_stack_mut(&mut self, zone: &str, room: &str) -> &mut HashMap<String, i64> {
        let key = format!("{}:{}", zone, room);
        self.room_inv_stack.entry(key).or_default()
    }

    /// Get stackable counts on a room's floor (for display).
    pub fn get_room_objs_stack(&self, zone: &str, room: &str) -> HashMap<String, i64> {
        let key = format!("{}:{}", zone, room);
        self.room_inv_stack.get(&key).cloned().unwrap_or_default()
    }

    /// Apply Python `Room.update()` / `Item.update()` lifetime state to
    /// individual floor objects.  An item without a timestamp receives its
    /// first timestamp and survives this update, just like Python's first
    /// `Item.update()` call.  The legacy compressed stack has no per-object
    /// timestamps, so it deliberately remains outside this exact branch.
    pub(crate) fn expire_floor_items_at(
        &mut self,
        rooms: &[(String, String)],
        now: f64,
    ) -> Vec<ExpiredFloorItem> {
        let mut expired = Vec::new();
        for (zone, room) in rooms {
            let key = format!("{zone}:{room}");
            let Some(objects) = self.room_objs.get_mut(&key) else {
                continue;
            };
            let mut removed_ids = Vec::new();
            objects.retain(|arc| {
                let Ok(mut object) = arc.lock() else {
                    return true;
                };
                let dropped_at = match object.temp.get(FLOOR_ITEM_DROP_TIME_KEY) {
                    Some(Value::Float(value)) => Some(*value),
                    Some(Value::Int(value)) => Some(*value as f64),
                    _ => None,
                };
                let Some(dropped_at) = dropped_at else {
                    object
                        .temp
                        .insert(FLOOR_ITEM_DROP_TIME_KEY.to_string(), Value::Float(now));
                    return true;
                };
                if now - dropped_at < FLOOR_ITEM_LIFETIME_SECONDS {
                    return true;
                }
                let index = object.getString("인덱스");
                if object.checkAttr("아이템속성", "단일아이템") && !index.is_empty() {
                    let _ = crate::oneitem::oneitem_destroy(&index);
                }
                expired.push(ExpiredFloorItem {
                    zone: zone.clone(),
                    room: room.clone(),
                    name: object.getName(),
                });
                removed_ids.push(Arc::as_ptr(arc) as usize);
                false
            });
            for id in removed_ids {
                self.remove_room_object(zone, room, &RoomObjectRef::FloorItem(id));
            }
        }
        expired
    }

    /// Initialize the world (load initial data)
    ///
    /// 방/몹은 사용자 진입 시점에 get_room, spawn_mobs_for_room 경로로 동적 로딩.
    /// 서버 기동 시 낙양성 전체를 프리로드하지 않아 메모리 절약.
    pub fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Get a player's position
    pub fn get_player_position(&self, player_name: &str) -> Option<&PlayerPosition> {
        self.player_positions.get(player_name)
    }

    /// Set a player's position
    pub fn set_player_position(&mut self, player_name: &str, pos: PlayerPosition) {
        if let Some(previous) = self.player_positions.get(player_name).cloned() {
            let previous_key = previous.room_key();
            if previous_key == pos.room_key() {
                self.player_positions.insert(player_name.to_string(), pos);
                let inserted = {
                    let players = self.room_players.entry(previous_key.clone()).or_default();
                    if !players.iter().any(|name| name == player_name) {
                        players.push(player_name.to_string());
                        true
                    } else {
                        false
                    }
                };
                if inserted {
                    if let Some((zone, room)) = previous_key.split_once(':') {
                        self.record_room_object(
                            zone,
                            room,
                            RoomObjectRef::Player(player_name.to_string()),
                        );
                    }
                }
                return;
            }
            self.remove_room_object(
                &previous.zone,
                &previous.room,
                &RoomObjectRef::Player(player_name.to_string()),
            );
            self.remove_from_room_index(&previous_key, player_name);
        }

        let room_key = pos.room_key();
        let zone = pos.zone.clone();
        let room = pos.room.clone();
        self.player_positions.insert(player_name.to_string(), pos);
        let inserted = {
            let players = self.room_players.entry(room_key).or_default();
            if !players.iter().any(|name| name == player_name) {
                players.push(player_name.to_string());
                true
            } else {
                false
            }
        };
        if inserted {
            self.record_room_object(&zone, &room, RoomObjectRef::Player(player_name.to_string()));
        }
    }

    /// Remove a player's position (e.g. when kicked due to duplicate login)
    pub fn remove_player_position(&mut self, player_name: &str) -> Option<PlayerPosition> {
        let removed = self.player_positions.remove(player_name)?;
        self.remove_room_object(
            &removed.zone,
            &removed.room,
            &RoomObjectRef::Player(player_name.to_string()),
        );
        self.remove_from_room_index(&removed.room_key(), player_name);
        Some(removed)
    }

    fn remove_from_room_index(&mut self, room_key: &str, player_name: &str) {
        let should_remove_room = if let Some(players) = self.room_players.get_mut(room_key) {
            players.retain(|name| name != player_name);
            players.is_empty()
        } else {
            false
        };
        if should_remove_room {
            self.room_players.remove(room_key);
        }
    }

    /// Move a player in a direction
    pub fn move_player(
        &mut self,
        player_name: &str,
        direction: Direction,
    ) -> Result<(String, String), String> {
        let current_pos = self
            .player_positions
            .get(player_name)
            .ok_or("Player not in world")?;

        // Get current room
        let room = self
            .room_cache
            .get_room(&current_pos.zone, &current_pos.room)
            .map_err(|e| format!("Failed to get room: {}", e))?;

        let room_read = room.read().unwrap();
        let exit = room_read
            .get_exit(direction)
            .ok_or(format!("{}쪽으로 갈 수 없습니다.", direction.korean_name()))?;

        let dest = exit
            .destination(&current_pos.zone)
            .ok_or("Invalid exit destination")?;

        // Update player position. Exit.destination is (zone, room_id: String).
        let new_pos = PlayerPosition::new(dest.0.clone(), dest.1.clone());
        drop(room_read);
        self.set_player_position(player_name, new_pos.clone());

        Ok((dest.0, dest.1))
    }

    /// Kill a mob in a specific room
    pub fn kill_mob(&mut self, zone: &str, room: &str, mob_key: &str) -> bool {
        self.mob_cache.kill_mob(zone, room, mob_key)
    }

    /// Damage a mob in a specific room
    /// Returns (new_hp, died) if mob was found and damaged
    pub fn damage_mob(
        &mut self,
        zone: &str,
        room: &str,
        mob_key: &str,
        damage: i64,
    ) -> Option<(i64, bool)> {
        self.mob_cache.damage_mob(zone, room, mob_key, damage)
    }

    /// 고유 명칭 또는 방향명("초보수련장", "출구", "북" 등)으로 이동.
    /// 반환: (new_zone, new_room, 이동 메시지용 이름 "북쪽" or "초보수련장")
    pub fn move_player_by_name(
        &mut self,
        player_name: &str,
        exit_name: &str,
    ) -> Result<(String, String, String), String> {
        let current_pos = self
            .player_positions
            .get(player_name)
            .ok_or("Player not in world")?;

        let room = self
            .room_cache
            .get_room(&current_pos.zone, &current_pos.room)
            .map_err(|e| format!("Failed to get room: {}", e))?;

        let (dest, msg_name) = {
            let room_read = room.read().unwrap();
            let exit = room_read
                .get_exit_by_name(exit_name)
                .ok_or_else(|| format!("{} (으)로 갈 수 없습니다.", exit_name))?;
            let d = exit.destination("").ok_or("Invalid exit destination")?;
            (d, exit.exit_message_name().to_string())
        };

        let new_pos = PlayerPosition::new(dest.0.clone(), dest.1.clone());
        self.set_player_position(player_name, new_pos);
        self.spawn_mobs_for_room(&dest.0, &dest.1);
        Ok((dest.0, dest.1, msg_name))
    }

    /// Get mobs in a player's current room
    pub fn get_mobs_for_player(&self, player_name: &str) -> Vec<&MobInstance> {
        if let Some(pos) = self.player_positions.get(player_name) {
            self.mob_cache.get_mobs_in_room(&pos.zone, &pos.room)
        } else {
            Vec::new()
        }
    }

    /// 같은 방에 있는 플레이어 이름 목록. 방별 인덱스를 사용하며 해당
    /// 방에 들어온 플레이어 순서를 보존한다.
    pub fn get_players_in_room(&self, zone: &str, room: &str) -> Vec<String> {
        self.room_players
            .get(&format!("{}:{}", zone, room))
            .cloned()
            .unwrap_or_default()
    }

    /// Spawn mobs for a room from 맵정보.몹 (called when player enters, on-demand load). room은 "1" 또는 사용자맵 "이름".
    pub fn spawn_mobs_for_room(&mut self, zone: &str, room: &str) {
        self.spawn_mobs_for_room_with_difficulty(zone, room, 0)
    }

    /// Spawn mobs for a room with difficulty support.
    /// Loads room data from base zone and spawns mobs with difficulty-adjusted stats.
    ///
    /// # Arguments
    /// * `zone` - Zone name (can include difficulty suffix)
    /// * `room` - Room id
    /// * `difficulty` - Difficulty level (0-7)
    pub fn spawn_mobs_for_room_with_difficulty(
        &mut self,
        zone: &str,
        room: &str,
        difficulty: DifficultyLevel,
    ) {
        use crate::world::difficulty::{base_zone_name, difficulty_from_zone};

        let effective_difficulty = if difficulty > 0 {
            difficulty
        } else {
            difficulty_from_zone(zone)
        };

        // Get mob_ids from room (load from base zone)
        let base_zone = base_zone_name(zone);
        let (mob_ids, installed_box_count, fixture_placements) = self
            .room_cache
            .get_room(base_zone, room)
            .ok()
            .and_then(|r| {
                r.read().ok().map(|g| {
                    (
                        g.mob_ids.clone(),
                        g.installed_box_count,
                        g.fixture_placements.clone(),
                    )
                })
            })
            .unwrap_or_default();

        for placement in fixture_placements {
            let already_placed = self.get_room_fixtures(zone, room).iter().any(|fixture| {
                fixture
                    .attribute("placement_key")
                    .and_then(serde_json::Value::as_str)
                    == Some(placement.key.as_str())
            });
            if already_placed {
                continue;
            }
            let mut attributes = placement.attributes;
            attributes.insert(
                "placement_key".to_string(),
                serde_json::Value::String(placement.key),
            );
            self.create_fixture(zone, room, placement.kind, attributes);
        }

        // Python Room.create inserts every installation Box at index zero
        // before mobs are placed. The Box registry exposes the resulting
        // reverse insertion vector, so record its ordinals in that order.
        // Python Room.create skips installation Boxes in difficulty zones
        // (the zone name's final character is a digit).
        let boxes_allowed = !zone
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_digit());
        let has_installed_boxes = self
            .get_room_object_order(zone, room)
            .iter()
            .any(|object| matches!(object, RoomObjectRef::InstalledBox(_)));
        if boxes_allowed && !has_installed_boxes {
            for ordinal in (0..installed_box_count).rev() {
                self.record_room_object(zone, room, RoomObjectRef::InstalledBox(ordinal));
            }
        }

        self.mob_cache.spawn_mobs_for_room_with_difficulty(
            zone,
            room,
            &mob_ids,
            effective_difficulty,
        );
        let mob_ids: Vec<u64> = self
            .mob_cache
            .get_all_mobs_in_room(zone, room)
            .into_iter()
            .map(|mob| mob.instance_id)
            .collect();
        let recorded = self.get_room_object_order(zone, room);
        for instance_id in mob_ids {
            if !recorded.contains(&RoomObjectRef::Mob(instance_id)) {
                self.record_room_object(zone, room, RoomObjectRef::Mob(instance_id));
            }
        }
    }

    /// Update world state (respawns, etc.)
    pub fn update(&mut self) {
        self.mob_cache.update_respawns();
    }

    /// Python `Loop.updateRooms` subset for occupied rooms: advance
    /// corpse/regen state, mob ticks/recovery, and idle automatic speech.
    /// The returned `writeRoom` payloads must be delivered after releasing
    /// the world lock, exactly as `Room.update()` does.
    pub(crate) fn update_occupied_room_mobs(
        &mut self,
        rooms: &[(String, String)],
    ) -> Vec<RoomMobMessage> {
        let now = chrono::Utc::now().timestamp();
        let mut corpse_drops = Vec::new();
        let mut respawning_instances = Vec::new();
        for (zone, room) in rooms {
            let templates = self
                .mob_cache
                .get_all_mobs_in_room(zone, room)
                .into_iter()
                .filter_map(|mob| {
                    self.mob_cache
                        .get_mob(&mob.mob_key)
                        .cloned()
                        .map(|data| (mob.mob_key.clone(), data))
                })
                .collect::<HashMap<_, _>>();
            let Some(mobs) = self.mob_cache.get_all_mobs_in_room_mut(zone, room) else {
                continue;
            };
            for mob in mobs {
                if mob.alive && mob.mob_type == 6 {
                    if let Some(data) = templates.get(&mob.mob_key) {
                        if now.saturating_sub(mob.time_of_regen) >= data.item_regen {
                            mob.time_of_regen = now;
                            crate::server::game_loop::generate_mob_corpse_items(
                                mob,
                                data,
                                &mut |upper| {
                                    rand::Rng::gen_range(&mut rand::thread_rng(), 0..=upper)
                                },
                            );
                        }
                    }
                }
                if !mob.alive
                    && now.saturating_sub(mob.death_time)
                        >= templates
                            .get(&mob.mob_key)
                            .map(|data| data.corpse_time.saturating_add(data.regen))
                            .unwrap_or(i64::MAX)
                {
                    respawning_instances.push(mob.instance_id);
                }
                if mob.alive || mob.act != 2 {
                    continue;
                }
                let Some(data) = templates.get(&mob.mob_key) else {
                    continue;
                };
                let elapsed = now.saturating_sub(mob.death_time);
                if elapsed < data.corpse_time {
                    continue;
                }
                let drop_age = if elapsed >= data.corpse_time.saturating_add(data.regen) {
                    elapsed.saturating_sub(data.corpse_time)
                } else {
                    0
                };
                corpse_drops.push((
                    zone.clone(),
                    room.clone(),
                    mob.name.clone(),
                    drop_age,
                    std::mem::take(&mut mob.inventory),
                ));
            }
        }
        self.mob_cache.update_respawns_in_rooms_at(rooms, now);
        let mut respawn_notices = Vec::new();
        for (instance_id, old_zone, old_room, origin_zone, origin_room) in self
            .mob_cache
            .return_respawned_to_origins(&respawning_instances)
        {
            // Python removes the regenerated object from its wandering room
            // and inserts it at the front of its fixed origin room.
            self.remove_room_object(&old_zone, &old_room, &RoomObjectRef::Mob(instance_id));
            self.record_room_object(&origin_zone, &origin_room, RoomObjectRef::Mob(instance_id));
        }
        for instance_id in respawning_instances {
            let Some(mob) = self
                .mob_cache
                .all_instances()
                .find(|mob| mob.instance_id == instance_id)
            else {
                continue;
            };
            let description = self
                .mob_cache
                .get_mob(&mob.mob_key)
                .map(|data| data.desc3.clone())
                .unwrap_or_default();
            respawn_notices.push((
                mob.zone.clone(),
                mob.room.clone(),
                mob.name.clone(),
                description,
            ));
        }

        let mut messages = Vec::new();
        for (zone, room, mob_name, drop_age, items) in corpse_drops {
            let mut revealed_items = Vec::new();
            for item in items {
                if let Ok(mut object) = item.lock() {
                    object.temp.insert(
                        FLOOR_ITEM_DROP_TIME_KEY.to_string(),
                        Value::Float(now as f64 - drop_age as f64),
                    );
                    revealed_items.push(RevealedFloorItem {
                        name: object.getName(),
                        ansi: object.getString("안시"),
                    });
                }
                // Python repeatedly calls Room.insert(), so every revealed
                // corpse item becomes the newest room object.
                self.get_room_objs_mut(&zone, &room).insert(0, item.clone());
                self.record_floor_item(&zone, &room, &item);
            }
            messages.push(RoomMobMessage {
                zone,
                room,
                kind: RoomMobMessageKind::CorpseGone,
                message: String::new(),
                mob_name,
                revealed_items,
            });
        }
        for (zone, room, mob_name, description) in respawn_notices {
            messages.push(RoomMobMessage {
                zone,
                room,
                kind: RoomMobMessageKind::Respawn,
                message: description,
                mob_name,
                revealed_items: Vec::new(),
            });
        }
        for (zone, room) in rooms {
            let templates = self
                .mob_cache
                .get_all_mobs_in_room(zone, room)
                .into_iter()
                .filter_map(|mob| {
                    self.mob_cache
                        .get_mob(&mob.mob_key)
                        .cloned()
                        .map(|data| (mob.mob_key.clone(), data))
                })
                .collect::<HashMap<_, _>>();
            let Some(mobs) = self.mob_cache.get_all_mobs_in_room_mut(zone, room) else {
                continue;
            };
            for mob in mobs {
                let Some(data) = templates.get(&mob.mob_key) else {
                    continue;
                };
                // `Mob.update()` increments tick before every branch.
                mob.tick = mob.tick.saturating_add(1);
                if mob.tick % 60 == 0 {
                    let divisor = match mob.act {
                        0 => Some(10), // stand: 10%
                        1 => Some(20), // fight: 5%
                        4 => Some(5),  // rest: 20%
                        _ => None,
                    };
                    if let Some(divisor) = divisor {
                        mob.hp = (mob.hp + mob.max_hp / divisor).min(mob.max_hp);
                        mob.mp = (mob.mp + mob.max_mp / divisor).min(mob.max_mp);
                    }
                }
                if mob.alive
                    && mob.act == 0
                    && data.talk_tick > 0
                    && mob.tick % data.talk_tick == 0
                    && !data.auto_scripts.is_empty()
                    && rand::thread_rng().gen_range(0..3) == 0
                {
                    let index = rand::thread_rng().gen_range(0..data.auto_scripts.len());
                    messages.push(RoomMobMessage {
                        zone: zone.clone(),
                        room: room.clone(),
                        kind: RoomMobMessageKind::Speech,
                        message: data.auto_scripts[index].clone(),
                        mob_name: String::new(),
                        revealed_items: Vec::new(),
                    });
                }
            }
        }
        messages
    }

    /// Python `Mob.updateMoving`: inspect at most 30 moving clones per
    /// second, wait their configured movement tick, then choose a valid
    /// directional exit whose destination is in that mob's move list.
    pub(crate) fn update_moving_mobs_at(&mut self, now_millis: i64) {
        let candidates = self.mob_cache.moving_instances_snapshot();
        if candidates.is_empty() {
            return;
        }
        // Python Loop.updateMovings() always calls movingMobs[0].updateMoving()
        // and that method examines the first MAXPROCESSMOVING entries.  It
        // does not round-robin through the complete global list.
        for (zone, room, key, last_move) in candidates.iter().take(30) {
            let Some(data) = self.mob_cache.get_mob(key).cloned() else {
                continue;
            };
            if data.move_rooms.is_empty() {
                continue;
            }
            if *last_move == 0 {
                self.mob_cache.set_move_time(zone, room, key, now_millis);
                continue;
            }
            if now_millis.saturating_sub(*last_move) < data.move_tick.saturating_mul(1_000) {
                continue;
            }
            // Python only moves on one third of due checks.
            if rand::thread_rng().gen_range(0..3) != 0 {
                continue;
            }
            let Ok(room_arc) = self.room_cache.get_room(zone, room) else {
                continue;
            };
            let exits = room_arc
                .read()
                .ok()
                .map(|r| r.exits.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            // Python chooses a member of the entire sorted exitList first;
            // it does not reroll until it finds a permitted move route.
            let direction_order = [
                "동", "서", "남", "북", "위", "아래", "남동", "남서", "북동", "북서",
            ];
            let ordered = direction_order
                .iter()
                .filter_map(|name| {
                    exits
                        .iter()
                        .find(|exit| exit.display_name == *name)
                        .cloned()
                })
                .collect::<Vec<_>>();
            if ordered.is_empty() {
                continue;
            }
            let Some(exit) = ordered.get(rand::thread_rng().gen_range(0..ordered.len())) else {
                continue;
            };
            let Some((dest_zone, dest_room)) = exit.destination(zone) else {
                continue;
            };
            if dest_zone != *zone || !data.move_rooms.iter().any(|allowed| allowed == &dest_room) {
                continue;
            }
            // Python created every Room and its configured mobs before any
            // wandering heartbeat. Materialize the destination placement
            // before inserting the moving object so Room.objs order matches.
            self.spawn_mobs_for_room(&dest_zone, &dest_room);
            if let Some(instance_id) = self
                .mob_cache
                .move_instance(zone, room, key, &dest_zone, &dest_room, now_millis)
            {
                self.remove_room_object(zone, room, &RoomObjectRef::Mob(instance_id));
                self.record_room_object(&dest_zone, &dest_room, RoomObjectRef::Mob(instance_id));
            }
        }
    }

    /// Apply the exact subset of Python `CmdObj.updateZones()` that the
    /// current runtime state can represent, in `Room.Zones` insertion order.
    ///
    /// This deliberately preflights every due room before changing any of
    /// them.  Missing floor-item timestamps, mob death/regeneration, active
    /// effects, item-regeneration, aggression, and a due talk tick are
    /// structural blocks rather than guessed success.
    pub fn update_loaded_rooms_before_reboot(&mut self) -> Result<(), RebootRoomUpdateBlock> {
        let loaded = self.room_cache.loaded_rooms_in_python_zone_order();
        let mut zone_order = Vec::new();
        for (zone, _) in &loaded {
            if !zone_order.iter().any(|loaded_zone| loaded_zone == zone) {
                zone_order.push(zone.clone());
            }
        }
        let now_millis = chrono::Utc::now().timestamp_millis();
        let mut due = Vec::new();

        for (zone, room) in loaded {
            let room_arc = self
                .room_cache
                .get_room_cached(&zone, &room)
                .ok_or_else(|| RebootRoomUpdateBlock::MissingRoom {
                    zone: zone.clone(),
                    room: room.clone(),
                })?;
            let (last_update_millis, configured_mobs) = room_arc
                .read()
                .map(|loaded_room| (loaded_room.last_update_millis, loaded_room.mob_ids.clone()))
                .map_err(|_| RebootRoomUpdateBlock::PoisonedRoom {
                    zone: zone.clone(),
                    room: room.clone(),
                })?;

            if now_millis.saturating_sub(last_update_millis) < 1_000 {
                continue;
            }

            let room_key = format!("{zone}:{room}");
            let has_floor_objects = self
                .room_objs
                .get(&room_key)
                .is_some_and(|objects| !objects.is_empty());
            let has_floor_stacks = self
                .room_inv_stack
                .get(&room_key)
                .is_some_and(|stacks| stacks.values().any(|count| *count > 0));
            if has_floor_objects || has_floor_stacks {
                return Err(RebootRoomUpdateBlock::FloorItems { zone, room });
            }

            if !configured_mobs.is_empty() && !self.mob_cache.has_room_instance_state(&zone, &room)
            {
                return Err(RebootRoomUpdateBlock::MissingMobInstances { zone, room });
            }

            let room_mobs = self.mob_cache.get_all_mobs_in_room(&zone, &room);
            let base_zone = base_zone_name(&zone);
            if configured_mobs.iter().any(|mob_id| {
                let expected_key = format!("{base_zone}:{mob_id}");
                !room_mobs.iter().any(|mob| mob.mob_key == expected_key)
            }) {
                return Err(RebootRoomUpdateBlock::MissingMobInstances { zone, room });
            }

            for mob in room_mobs {
                let data = self.mob_cache.get_mob(&mob.mob_key).ok_or_else(|| {
                    RebootRoomUpdateBlock::MissingMobTemplate {
                        zone: zone.clone(),
                        room: room.clone(),
                        mob: mob.mob_key.clone(),
                    }
                })?;
                if mob.tick.checked_add(1).is_none() {
                    return Err(RebootRoomUpdateBlock::MobTickOverflow {
                        zone: zone.clone(),
                        room: room.clone(),
                        mob: mob.mob_key.clone(),
                    });
                }
                if !reboot_mob_update_is_exact_noop(mob, data) {
                    return Err(RebootRoomUpdateBlock::UnsupportedMobUpdate {
                        zone: zone.clone(),
                        room: room.clone(),
                        mob: mob.mob_key.clone(),
                    });
                }
            }

            due.push((zone, room, room_arc));
        }

        for zone in zone_order {
            // Python `리부팅.py` prints this to the server console, not to users.
            println!("update zones...{zone}");
        }
        for (zone, room, room_arc) in due {
            if let Some(mobs) = self.mob_cache.get_all_mobs_in_room_mut(&zone, &room) {
                for mob in mobs {
                    // Overflow was rejected during the all-room preflight.
                    mob.tick += 1;
                }
            }
            room_arc
                .write()
                .map_err(|_| RebootRoomUpdateBlock::PoisonedRoom { zone, room })?
                .last_update_millis = now_millis;
        }

        Ok(())
    }

    /// Get mobs in a room (convenience method)
    pub fn get_mobs_in_room(&self, zone: &str, room: &str) -> Vec<&MobInstance> {
        self.mob_cache.get_mobs_in_room(zone, room)
    }

    /// Get mob data by key
    pub fn get_mob_data(&self, mob_key: &str) -> Option<&RawMobData> {
        self.mob_cache.get_mob(mob_key)
    }

    /// Get mob instance in a specific room
    pub fn get_mob(&self, zone: &str, room: &str, mob_key: &str) -> Option<&MobInstance> {
        let mobs = self.mob_cache.get_mobs_in_room(zone, room);
        mobs.iter()
            .find(|m| m.mob_key == mob_key && m.alive)
            .copied()
    }

    /// Spawn a specific mob at a location (for admin commands)
    /// mob_key should be "zone:filename" format (e.g., "낙양성:밍밍")
    /// Returns Ok(()) on success, Err(message) on failure
    pub fn spawn_mob_at(
        &mut self,
        mob_key: &str,
        dest_zone: &str,
        dest_room: &str,
    ) -> Result<(), String> {
        // Parse mob_key to get zone and filename
        let parts: Vec<&str> = mob_key.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(format!(
                "잘못된 몹 키 형식: {} (존:파일명 형식이어야 함)",
                mob_key
            ));
        }
        let mob_zone = parts[0];
        let mob_filename = parts[1];

        // Load mob data
        let mob_data = self
            .mob_cache
            .load_mob(mob_zone, mob_filename)
            .map_err(|e| format!("몹 로드 실패: {:?}", e))?;

        // Create mob instance at destination
        let instance = MobInstance::new(
            mob_key.to_string(),
            dest_zone.to_string(),
            dest_room.to_string(),
            &mob_data,
        );
        let instance_id = instance.instance_id;

        // Add to cache
        self.mob_cache.add_mob_instance(instance);
        self.record_room_object(dest_zone, dest_room, RoomObjectRef::Mob(instance_id));

        Ok(())
    }

    /// Search for mobs by name pattern across all rooms
    /// Returns Vec of (zone, room, mob_name, mob_display_name, hp, max_hp)
    pub fn search_mobs_by_name(
        &self,
        name_pattern: &str,
    ) -> Vec<(String, String, String, String, i64, i64)> {
        let mut results = Vec::new();
        let pattern_lower = name_pattern.to_lowercase();

        // Search through all mob instances using public method
        for (room_key, instances) in self.mob_cache.get_all_instances() {
            // Parse room_key into zone and room
            let parts: Vec<&str> = room_key.splitn(2, ':').collect();
            let _zone = parts.first().copied().unwrap_or("");
            let _room = parts.get(1).copied().unwrap_or("");

            for mob in instances {
                // Check mob data for display name
                if let Some(mob_data) = self.mob_cache.get_mob(&mob.mob_key) {
                    let display_name = &mob_data.desc1;
                    let display_name_lower = display_name.to_lowercase();

                    // Match by display name or reaction names
                    if display_name_lower.contains(&pattern_lower)
                        || mob_data.name.to_lowercase().contains(&pattern_lower)
                    {
                        results.push((
                            mob.zone.clone(),
                            mob.room.clone(),
                            mob_data.name.clone(),
                            display_name.clone(),
                            mob.hp,
                            mob.max_hp,
                        ));
                    } else {
                        // Check reaction names
                        for reaction in &mob_data.reaction_names {
                            if reaction.to_lowercase().contains(&pattern_lower) {
                                results.push((
                                    mob.zone.clone(),
                                    mob.room.clone(),
                                    mob_data.name.clone(),
                                    display_name.clone(),
                                    mob.hp,
                                    mob.max_hp,
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        }

        results
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global world state accessor
pub fn get_world_state() -> &'static RwLock<WorldState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<RwLock<WorldState>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut world = WorldState::new();
        // Try to initialize, log error but don't panic
        if let Err(e) = world.initialize() {
            eprintln!("Failed to initialize world: {}", e);
        }
        RwLock::new(world)
    })
}

#[cfg(test)]
mod world_state_tests {
    use super::*;

    fn temporary_room_cache(label: &str) -> (std::path::PathBuf, RoomCache) {
        let root = std::env::temp_dir().join(format!(
            "muc_world_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let zone = root.join("시험존");
        std::fs::create_dir_all(&zone).unwrap();
        for room in ["1", "2"] {
            std::fs::write(
                zone.join(format!("{room}.json")),
                format!(
                    r#"{{"맵정보":{{"이름":"시험방{room}","존이름":"시험존","설명":[],"출구":[],"몹":[],"맵속성":[]}}}}"#
                ),
            )
            .unwrap();
        }
        let cache = RoomCache::with_data_dir(&root);
        (root, cache)
    }

    #[test]
    fn room_player_index_tracks_insert_move_and_remove_in_room_order() {
        let mut world = WorldState::new();
        world.set_player_position(
            "첫째",
            PlayerPosition::new("시험존".to_string(), "1".to_string()),
        );
        world.set_player_position(
            "둘째",
            PlayerPosition::new("시험존".to_string(), "1".to_string()),
        );
        world.set_player_position(
            "셋째",
            PlayerPosition::new("시험존".to_string(), "2".to_string()),
        );

        assert_eq!(world.get_players_in_room("시험존", "1"), ["첫째", "둘째"]);
        assert_eq!(world.get_players_in_room("시험존", "2"), ["셋째"]);
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [
                RoomObjectRef::Player("둘째".to_string()),
                RoomObjectRef::Player("첫째".to_string())
            ]
        );

        // Reasserting the same position must not duplicate or reorder a player.
        world.set_player_position(
            "첫째",
            PlayerPosition::new("시험존".to_string(), "1".to_string()),
        );
        assert_eq!(world.get_players_in_room("시험존", "1"), ["첫째", "둘째"]);

        world.set_player_position(
            "첫째",
            PlayerPosition::new("시험존".to_string(), "2".to_string()),
        );
        assert_eq!(world.get_players_in_room("시험존", "1"), ["둘째"]);
        assert_eq!(world.get_players_in_room("시험존", "2"), ["셋째", "첫째"]);
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [RoomObjectRef::Player("둘째".to_string())]
        );
        assert_eq!(
            world.get_room_object_order("시험존", "2"),
            [
                RoomObjectRef::Player("첫째".to_string()),
                RoomObjectRef::Player("셋째".to_string())
            ]
        );

        assert!(world.remove_player_position("셋째").is_some());
        assert_eq!(world.get_players_in_room("시험존", "2"), ["첫째"]);
        assert!(world.remove_player_position("없는이").is_none());
    }

    #[test]
    fn installed_boxes_keep_python_reverse_insert_order_before_mobs_and_players() {
        let mut world = WorldState::new();
        // Python Room.create inserts A then B at index zero, leaving B,A.
        // Mob loading and player entry then prepend in that order.
        world.record_room_object("시험존", "1", RoomObjectRef::InstalledBox(1));
        world.record_room_object("시험존", "1", RoomObjectRef::InstalledBox(0));
        world.record_room_object("시험존", "1", RoomObjectRef::Mob(77));
        world.set_player_position(
            "플레이어",
            PlayerPosition::new("시험존".to_string(), "1".to_string()),
        );

        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [
                RoomObjectRef::Player("플레이어".to_string()),
                RoomObjectRef::Mob(77),
                RoomObjectRef::InstalledBox(0),
                RoomObjectRef::InstalledBox(1),
            ]
        );
    }

    #[test]
    fn runtime_box_and_summoned_user_lifecycle_preserves_prepend_and_removal() {
        let mut world = WorldState::new();
        let runtime_box = Arc::new(Mutex::new(Object::new()));
        world.record_box("시험존", "1", &runtime_box);
        let mut body = crate::player::Body::new();
        body.set("이름", "소환사용자");
        let summoned_id = world.add_summoned_user(
            body,
            PlayerPosition::new("시험존".to_string(), "1".to_string()),
        );
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [
                RoomObjectRef::SummonedUser(summoned_id),
                RoomObjectRef::Box(Arc::as_ptr(&runtime_box) as usize),
            ]
        );

        let extracted = world.take_summoned_user_by_name("소환사용자").unwrap();
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [RoomObjectRef::Box(Arc::as_ptr(&runtime_box) as usize)]
        );
        world.restore_summoned_user(
            extracted,
            PlayerPosition::new("시험존".to_string(), "2".to_string()),
        );
        assert_eq!(
            world.get_room_object_order("시험존", "2"),
            [RoomObjectRef::SummonedUser(summoned_id)]
        );
        assert!(world.remove_summoned_user_by_id(summoned_id));
        assert!(world.get_room_object_order("시험존", "2").is_empty());
    }

    #[test]
    fn occupied_room_mob_tick_recovers_like_python_without_unspecified_talk() {
        let mut world = WorldState::new();
        let mut data = RawMobData::new();
        data.name = "시험몹".to_string();
        data.hp = 100;
        data.max_hp = 100;
        data.inner_power = 50;
        data.talk_tick = 0; // absent `대화틱` must not default to speech.
        let key = "시험존:시험몹".to_string();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let mut instance = MobInstance::new(key, "시험존".to_string(), "1", &data);
        instance.hp = 1;
        instance.mp = 1;
        instance.tick = 59;
        world.mob_cache.add_mob_instance(instance);

        let messages = world.update_occupied_room_mobs(&[("시험존".to_string(), "1".to_string())]);
        assert!(messages.is_empty());
        let mob = world.mob_cache.get_all_mobs_in_room("시험존", "1")[0];
        assert_eq!(mob.tick, 60);
        assert_eq!(mob.hp, 11);
        assert_eq!(mob.mp, 6);
    }

    #[test]
    fn corpse_transition_moves_inventory_to_floor_with_python_drop_time() {
        let mut world = WorldState::new();
        let mut data = RawMobData::new();
        data.name = "시체몹".to_string();
        data.corpse_time = 1;
        data.regen = 100;
        let key = "시험존:시체몹".to_string();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let item = Arc::new(Mutex::new(Object::new()));
        item.lock().unwrap().set("이름", "첫유품");
        let second_item = Arc::new(Mutex::new(Object::new()));
        second_item.lock().unwrap().set("이름", "둘째유품");
        let mut mob = MobInstance::new(key, "시험존".to_string(), "1", &data);
        mob.alive = false;
        mob.act = 2;
        mob.death_time = chrono::Utc::now().timestamp() - 2;
        mob.inventory.push(item.clone());
        mob.inventory.push(second_item.clone());
        world.mob_cache.add_mob_instance(mob);

        let updates = world.update_occupied_room_mobs(&[("시험존".into(), "1".into())]);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].kind, RoomMobMessageKind::CorpseGone);
        assert_eq!(updates[0].mob_name, "시체몹");
        assert_eq!(updates[0].revealed_items[0].name, "첫유품");
        assert_eq!(updates[0].revealed_items[1].name, "둘째유품");
        let floor = world.get_room_objs("시험존", "1");
        assert_eq!(floor.len(), 2);
        assert!(Arc::ptr_eq(&floor[0], &second_item));
        assert!(Arc::ptr_eq(&floor[1], &item));
        assert!(matches!(
            item.lock().unwrap().temp.get(FLOOR_ITEM_DROP_TIME_KEY),
            Some(Value::Float(value)) if *value >= chrono::Utc::now().timestamp() as f64 - 1.0
        ));
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [
                RoomObjectRef::FloorItem(Arc::as_ptr(&second_item) as usize),
                RoomObjectRef::FloorItem(Arc::as_ptr(&item) as usize),
            ]
        );
        let mob = world.mob_cache.get_all_mobs_in_room("시험존", "1")[0];
        assert_eq!(mob.act, 3);
        assert!(mob.inventory.is_empty());
    }

    #[test]
    fn wandering_mob_regen_returns_to_origin_and_prepends_room_order() {
        let mut world = WorldState::new();
        let mut data = RawMobData::new();
        data.name = "귀소몹".to_string();
        data.desc3 = "귀소몹이 원래 자리에서 다시 나타납니다.".to_string();
        data.corpse_time = 0;
        data.regen = 0;
        let key = "시험존:귀소몹".to_string();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let instance = MobInstance::new(key.clone(), "시험존".to_string(), "1", &data);
        let instance_id = instance.instance_id;
        world.mob_cache.add_mob_instance(instance);
        world.record_room_object("시험존", "1", RoomObjectRef::Mob(instance_id));

        assert_eq!(
            world
                .mob_cache
                .move_instance("시험존", "1", &key, "시험존", "2", 1),
            Some(instance_id)
        );
        world.remove_room_object("시험존", "1", &RoomObjectRef::Mob(instance_id));
        world.record_room_object("시험존", "2", RoomObjectRef::Mob(instance_id));
        world.set_player_position(
            "원래방목격자",
            PlayerPosition::new("시험존".to_string(), "1".to_string()),
        );
        {
            let mob = world
                .mob_cache
                .get_all_mobs_in_room_mut("시험존", "2")
                .unwrap()
                .iter_mut()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            mob.kill();
            mob.death_time = chrono::Utc::now().timestamp() - 1;
        }

        let messages = world.update_occupied_room_mobs(&[("시험존".to_string(), "2".to_string())]);

        assert!(world
            .mob_cache
            .get_all_mobs_in_room("시험존", "2")
            .iter()
            .all(|mob| mob.instance_id != instance_id));
        let origin_mob = world
            .mob_cache
            .get_all_mobs_in_room("시험존", "1")
            .into_iter()
            .find(|mob| mob.instance_id == instance_id)
            .unwrap();
        assert!(origin_mob.alive);
        assert_eq!(origin_mob.act, 0);
        assert_eq!(origin_mob.origin_zone, "시험존");
        assert_eq!(origin_mob.origin_room, "1");
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [
                RoomObjectRef::Mob(instance_id),
                RoomObjectRef::Player("원래방목격자".to_string()),
            ]
        );
        assert!(world
            .get_room_object_order("시험존", "2")
            .iter()
            .all(|object| object != &RoomObjectRef::Mob(instance_id)));
        assert!(messages.iter().any(|message| {
            message.kind == RoomMobMessageKind::Respawn
                && message.zone == "시험존"
                && message.room == "1"
                && message.message == "귀소몹이 원래 자리에서 다시 나타납니다."
        }));
    }

    #[test]
    fn stationary_mob_regen_keeps_numeric_and_unified_room_order() {
        let mut world = WorldState::new();
        let mut data = RawMobData::new();
        data.name = "제자리몹".to_string();
        data.corpse_time = 0;
        data.regen = 0;
        let key = "시험존:제자리몹".to_string();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let first = MobInstance::new(key.clone(), "시험존".to_string(), "1", &data);
        let first_id = first.instance_id;
        let second = MobInstance::new(key, "시험존".to_string(), "1", &data);
        let second_id = second.instance_id;
        world.mob_cache.add_mob_instance(first);
        world.mob_cache.add_mob_instance(second);
        world.record_room_object("시험존", "1", RoomObjectRef::Mob(first_id));
        world.record_room_object("시험존", "1", RoomObjectRef::Mob(second_id));
        let numeric_order_before = world
            .mob_cache
            .get_all_mobs_in_room("시험존", "1")
            .into_iter()
            .map(|mob| mob.instance_id)
            .collect::<Vec<_>>();
        {
            let mobs = world
                .mob_cache
                .get_all_mobs_in_room_mut("시험존", "1")
                .unwrap();
            mobs[0].kill();
            mobs[0].death_time = chrono::Utc::now().timestamp() - 1;
        }

        world.update_occupied_room_mobs(&[("시험존".to_string(), "1".to_string())]);

        assert_eq!(
            world
                .mob_cache
                .get_all_mobs_in_room("시험존", "1")
                .into_iter()
                .map(|mob| mob.instance_id)
                .collect::<Vec<_>>(),
            numeric_order_before
        );
        assert_eq!(
            world.get_room_object_order("시험존", "1"),
            [RoomObjectRef::Mob(second_id), RoomObjectRef::Mob(first_id)]
        );
    }

    #[test]
    fn floor_item_expiry_uses_python_first_tick_and_ten_minute_boundary() {
        let mut world = WorldState::new();
        let item = Arc::new(Mutex::new(Object::new()));
        item.lock().unwrap().set("이름", "시험검");
        world.get_room_objs_mut("시험존", "1").push(item.clone());
        world.record_floor_item("시험존", "1", &item);
        let rooms = [("시험존".to_string(), "1".to_string())];

        assert!(world.expire_floor_items_at(&rooms, 100.0).is_empty());
        assert_eq!(
            item.lock().unwrap().temp.get(FLOOR_ITEM_DROP_TIME_KEY),
            Some(&Value::Float(100.0))
        );
        assert!(world.expire_floor_items_at(&rooms, 699.999).is_empty());
        let expired = world.expire_floor_items_at(&rooms, 700.0);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].name, "시험검");
        assert!(world.get_room_objs("시험존", "1").is_empty());
        assert!(world.get_room_object_order("시험존", "1").is_empty());
    }

    #[test]
    fn type_six_box_refills_only_when_item_regen_is_due_and_empty() {
        let mut world = WorldState::new();
        let mut data = RawMobData::new();
        data.name = "회귀상자".into();
        data.zone = "시험존".into();
        data.mob_type = 6;
        data.item_regen = 180;
        data.drop_items.push(("892".into(), 1, 100, 1));
        let key = "시험존:회귀상자".to_string();
        world.mob_cache.insert_mob_data(key.clone(), data.clone());
        let mut instance = MobInstance::new(key, "시험존".into(), "1", &data);
        instance.time_of_regen = chrono::Utc::now().timestamp() - 180;
        world.mob_cache.add_mob_instance(instance);

        world.update_occupied_room_mobs(&[("시험존".into(), "1".into())]);
        let mobs = world.mob_cache.get_all_mobs_in_room("시험존", "1");
        assert_eq!(mobs[0].inventory.len(), 1);
        assert_eq!(
            mobs[0].inventory[0].lock().unwrap().getString("인덱스"),
            "892"
        );
    }

    #[test]
    fn difficulty_zone_does_not_record_python_skipped_installed_boxes() {
        let mut world = WorldState::new();
        // This is the same condition as Room.create's `zone[-1].isdigit()`.
        let zone = "시험존1";
        let boxes_allowed = !zone
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_digit());
        assert!(!boxes_allowed);
        if boxes_allowed {
            world.record_room_object(zone, "1", RoomObjectRef::InstalledBox(0));
        }
        assert!(world.get_room_object_order(zone, "1").is_empty());
    }

    #[test]
    fn reboot_room_update_preflights_all_rooms_before_mutating_any() {
        let (root, cache) = temporary_room_cache("reboot_preflight");
        let mut world = WorldState::new();
        world.room_cache = cache;
        let first = world.room_cache.get_room("시험존", "1").unwrap();
        let second = world.room_cache.get_room("시험존", "2").unwrap();
        first.write().unwrap().last_update_millis = 0;
        second.write().unwrap().last_update_millis = 0;
        world.room_objs.insert(
            "시험존:2".to_string(),
            vec![Arc::new(Mutex::new(Object::new()))],
        );

        assert_eq!(
            world.update_loaded_rooms_before_reboot(),
            Err(RebootRoomUpdateBlock::FloorItems {
                zone: "시험존".to_string(),
                room: "2".to_string(),
            })
        );
        assert_eq!(first.read().unwrap().last_update_millis, 0);
        assert_eq!(second.read().unwrap().last_update_millis, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reboot_room_update_advances_a_safe_loaded_room() {
        let (root, cache) = temporary_room_cache("reboot_safe");
        let mut world = WorldState::new();
        world.room_cache = cache;
        let room = world.room_cache.get_room("시험존", "1").unwrap();
        room.write().unwrap().last_update_millis = 0;

        assert_eq!(world.update_loaded_rooms_before_reboot(), Ok(()));
        assert!(room.read().unwrap().last_update_millis > 0);
        let _ = std::fs::remove_dir_all(root);
    }
}
