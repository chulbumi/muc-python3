//! One-second runtime loop for connected MUD clients.
//!
//! Python's `Loop.run()` iterates `Client.players`, not a second player store.  The Rust
//! network layer keeps the authoritative `Player` inside `Broadcaster.clients`, so this loop
//! deliberately ticks that collection directly.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::interval;
use tracing::{debug, warn};

use crate::combat::processor::{calculate_exp_reward_for_level, calculate_gold_reward_for_level};
use crate::combat::{calculate_skill_damage_against, process_mob_strike, process_player_strike};
use crate::command::handler::PendingInput;
use crate::command::CommandRegistry;
use crate::network::client::{handle_game_command, send_collected_user_message};
use crate::network::client::{Client, ClientState, DISCONNECT_SENTINEL};
use crate::network::Broadcaster;
use crate::player::{ActState, Body, Player};
use crate::scheduler::CallOutScheduler;
use crate::script::{
    clear_cast_room_players, clear_precomputed_all_online, save_body_to_json,
    set_cast_room_players, set_precomputed_all_online, CastRoomPlayerRef,
};
use crate::world::event::{run_script_chunk, run_script_chunk_rhai, ScriptNext};
use crate::world::{get_world_state, RoomCache};

/// Game-loop timing that has a direct equivalent in `loop.py`/`objs/player.py`.
#[derive(Debug, Clone)]
pub struct GameLoopConfig {
    /// Python schedules `Loop.run` once per second.
    pub tick_interval: Duration,
    /// `INACTIVE` login timeout from `loop.py`.
    pub inactive_timeout: u64,
    /// Non-inactive/active timeout from `loop.py`.
    pub active_timeout: u64,
    /// `Player.update`: recover every 30 ticks.
    pub recovery_interval: u64,
    /// `Player.update`: save every 600 ticks.
    pub save_interval: u64,
    /// Production user directory. Tests override this with an isolated temporary directory.
    pub user_data_dir: PathBuf,
}

impl Default for GameLoopConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(1),
            inactive_timeout: 10,
            active_timeout: 180,
            recovery_interval: 30,
            save_interval: 600,
            user_data_dir: PathBuf::from("data/user"),
        }
    }
}

/// State retained between one-second updates.
pub struct GameLoop {
    config: GameLoopConfig,
    tick_count: u64,
    last_tick: Instant,
    after_fight: Vec<SocketAddr>,
    newly_dead: Vec<SocketAddr>,
    combat_render: Vec<SocketAddr>,
    auto_consume: Vec<(SocketAddr, String)>,
}

#[derive(Debug, Clone)]
struct PendingMobReward {
    instance_id: u64,
    mob_name: String,
    mob_level: i64,
    mob_gold: i64,
    difficulty: u8,
    personality: i64,
    zone: String,
    room: String,
    targets: Vec<String>,
    damage_map: std::collections::HashMap<String, i64>,
    mob_key: String,
    mob_data: crate::world::RawMobData,
}

static ADMIN_MOB_REWARDS: std::sync::LazyLock<std::sync::Mutex<Vec<PendingMobReward>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Run the same death snapshot/drop generation used by a lethal combat tick,
/// then defer connected-player reward application to the next game tick.
pub(crate) fn queue_admin_mob_death(
    mob: &mut crate::world::MobInstance,
    data: &crate::world::RawMobData,
) -> bool {
    if !mob.alive {
        return false;
    }
    let mut round = crate::combat::CombatRound::new();
    let mut rewards = Vec::new();
    award_mob_death(mob, data, &mut round, &mut rewards);
    if rewards.is_empty() {
        return false;
    }
    let mut queued = ADMIN_MOB_REWARDS.lock().unwrap();
    queued.extend(
        rewards
            .into_iter()
            .filter(|reward| !reward.targets.is_empty() || !reward.damage_map.is_empty()),
    );
    if queued.len() > 1_024 {
        let excess = queued.len() - 1_024;
        queued.drain(..excess);
    }
    true
}

impl GameLoop {
    pub fn new(config: GameLoopConfig) -> Self {
        Self {
            config,
            tick_count: 0,
            last_tick: Instant::now(),
            after_fight: Vec::new(),
            newly_dead: Vec::new(),
            combat_render: Vec::new(),
            auto_consume: Vec::new(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(GameLoopConfig::default())
    }

    /// Tick the authoritative connected-client collection once.
    pub fn tick(&mut self, broadcaster: &Broadcaster) -> bool {
        self.tick_at(broadcaster, Instant::now())
    }

    fn take_after_fight(&mut self) -> Vec<SocketAddr> {
        std::mem::take(&mut self.after_fight)
    }

    fn take_newly_dead(&mut self) -> Vec<SocketAddr> {
        std::mem::take(&mut self.newly_dead)
    }

    fn take_combat_render(&mut self) -> Vec<SocketAddr> {
        std::mem::take(&mut self.combat_render)
    }

    fn take_auto_consume(&mut self) -> Vec<(SocketAddr, String)> {
        std::mem::take(&mut self.auto_consume)
    }

    fn tick_at(&mut self, broadcaster: &Broadcaster, now: Instant) -> bool {
        self.tick_count += 1;

        let mut occupied_player_names = Vec::new();
        let mut room_messages: Vec<(String, String)> = Vec::new();
        let mut pending_rewards = Vec::new();
        let mut clients = broadcaster.clients.lock();
        for client in clients.values_mut() {
            let client_addr = client.addr;
            if client.disconnect_requested {
                continue;
            }

            let timeout = if client.uses_inactive_timeout() {
                self.config.inactive_timeout
            } else {
                self.config.active_timeout
            };
            if now.saturating_duration_since(client.last_input).as_secs() >= timeout {
                // `sendLine()` in Python appends CRLF to a string that already ends in CRLF.
                let message = if client.uses_inactive_timeout() {
                    "\r\n\r\n입력 제한시간을 초과하였습니다.\r\n\r\n"
                } else {
                    "\r\n\r\n3분 동안 입력이 없어 접속을 종료합니다.\r\n\r\n"
                };
                let _ = client.sender.send(message.to_string());
                let _ = client.sender.send(DISCONNECT_SENTINEL.to_string());
                client.disconnect_requested = true;
                continue;
            }

            if client.state == ClientState::Active {
                if let Some(player) = client.player.as_mut() {
                    if i64::from(player.cmd_cnt)
                        > crate::script::get_murim_config_int("입력초과에러수")
                    {
                        player.body.set("강제종료", chrono::Utc::now().timestamp());
                        let _ = client.sender.send(DISCONNECT_SENTINEL.to_string());
                        client.disconnect_requested = true;
                        continue;
                    }
                    player.cmd_cnt = 0;
                    let was_dead = player.body.act == ActState::Death;
                    let death_step = player.body.get_death_step();
                    update_active_player(player, &self.config);
                    // Python doDeath step 3 performs enterRoom("낙양성:7",
                    // "사망", "사망") while the player is still dead.
                    if was_dead && death_step == 3 {
                        let name = player.body.get_name();
                        let position = crate::world::PlayerPosition::new(
                            "낙양성".to_string(),
                            "7".to_string(),
                        );
                        if let Ok(mut world) = get_world_state().write() {
                            let old_players = world
                                .get_player_position(&name)
                                .map(|old| world.get_players_in_room(&old.zone, &old.room))
                                .unwrap_or_default();
                            let new_players = world.get_players_in_room("낙양성", "7");
                            world.spawn_mobs_for_room("낙양성", "7");
                            let update_now = chrono::Utc::now().timestamp_millis();
                            let update_due = world
                                .room_cache
                                .get_room_cached("낙양성", "7")
                                .and_then(|room| {
                                    room.read().ok().map(|room| room.last_update_millis)
                                })
                                .is_some_and(|last| update_now.saturating_sub(last) >= 1_000);
                            let (expired_items, update_messages) = if update_due {
                                let expired = world.expire_floor_items_at(
                                    &[("낙양성".to_string(), "7".to_string())],
                                    update_now as f64 / 1_000.0,
                                );
                                let messages = world.update_occupied_room_mobs(&[(
                                    "낙양성".to_string(),
                                    "7".to_string(),
                                )]);
                                if let Some(room) = world.room_cache.get_room_cached("낙양성", "7")
                                {
                                    if let Ok(mut room) = room.write() {
                                        room.last_update_millis = update_now;
                                    }
                                }
                                (
                                    expired
                                        .into_iter()
                                        .map(|item| item.name)
                                        .collect::<Vec<_>>(),
                                    messages
                                        .into_iter()
                                        .map(|message| message.message)
                                        .collect::<Vec<_>>(),
                                )
                            } else {
                                (Vec::new(), Vec::new())
                            };
                            world.set_player_position(&name, position);
                            crate::script::combat_commands::queue_combat_presentation_event(
                                &mut player.body,
                                serde_json::json!({
                                    "kind": "death_room_transition",
                                    "player": name,
                                    "old_players": old_players,
                                    "new_players": new_players,
                                    "expired_items": expired_items,
                                    "update_messages": update_messages,
                                }),
                            );
                        }
                        player.body.set("위치", "낙양성:7");
                        player.body.set("현재방", "낙양성:7");
                    }
                    if player.body.act == ActState::Fight
                        && crate::script::combat_commands::pvp_target(&player.body).is_none()
                    {
                        let messages = process_combat_tick(player, &mut pending_rewards);
                        for (room, message) in messages {
                            room_messages.push((room, message));
                        }
                        if player.body.act == ActState::Stand && player.body.targets.is_empty() {
                            self.after_fight.push(client_addr);
                        } else if player.body.act == ActState::Death {
                            self.newly_dead.push(client_addr);
                        }
                    }
                    // Python calls autoHpEat() and then autoMpEat() after the
                    // combat/recovery branch on every heartbeat while standing
                    // or fighting.  Re-enter the ordinary `먹어` command path
                    // after rendering this tick so item selection, output and
                    // state changes stay owned by the Rhai command.
                    for command in automatic_consumable_commands(player) {
                        self.auto_consume.push((client_addr, command));
                    }
                    expire_player_skill_effects(&mut player.body);
                    if let Some(crate::object::Value::String(expired)) =
                        player.body.temp_mut().remove("_expired_auto_skills")
                    {
                        let configured = player
                            .alias
                            .get("자동무공")
                            .map(|value| value.split(';').collect::<Vec<_>>())
                            .unwrap_or_default();
                        for skill in expired.split('\n').filter(|skill| !skill.is_empty()) {
                            if configured.contains(&skill) {
                                self.auto_consume
                                    .push((client_addr, format!("{skill} 시전")));
                            }
                        }
                    }
                    if player
                        .body
                        .temp()
                        .contains_key(crate::script::combat_commands::COMBAT_PRESENTATION_EVENTS)
                        && !self.combat_render.contains(&client_addr)
                    {
                        self.combat_render.push(client_addr);
                    }
                }
            }
            if let Some(player) = client.player.as_ref() {
                let name = player.body.get_name();
                if !name.is_empty() {
                    occupied_player_names.push(name);
                }
            }
        }
        // Claim only administrator deaths that belong to this broadcaster.
        // This also prevents independent test/server instances from consuming
        // another instance's deferred reward snapshot.
        let connected_names = clients
            .values()
            .filter_map(|client| client.player.as_ref().map(|player| player.body.get_name()))
            .collect::<std::collections::HashSet<_>>();
        let mut queued_admin = ADMIN_MOB_REWARDS.lock().unwrap();
        let mut retained = Vec::new();
        for reward in queued_admin.drain(..) {
            if reward
                .targets
                .iter()
                .any(|target| connected_names.contains(target))
            {
                pending_rewards.push(reward);
            } else {
                retained.push(reward);
            }
        }
        *queued_admin = retained;
        drop(queued_admin);
        process_pvp_ticks(&mut clients, &mut self.combat_render, &mut self.newly_dead);
        if !pending_rewards.is_empty() {
            let positions = get_world_state().read().ok();
            let snapshots = clients
                .values()
                .filter_map(|client| {
                    let player = client.player.as_ref()?;
                    let name = player.body.get_name();
                    let position = positions.as_ref()?.get_player_position(&name)?;
                    Some((
                        name,
                        (
                            player.body.get_int("레벨"),
                            position.zone.clone(),
                            position.room.clone(),
                        ),
                    ))
                })
                .collect::<std::collections::HashMap<_, _>>();
            drop(positions);
            for reward in pending_rewards {
                let same_room_targets = reward
                    .targets
                    .iter()
                    .filter(|name| {
                        snapshots.get(*name).is_some_and(|(_, zone, room)| {
                            zone == &reward.zone && room == &reward.room
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let count = same_room_targets.len() as i64;
                let Some(max_level) = same_room_targets
                    .iter()
                    .filter_map(|name| snapshots.get(name).map(|(level, _, _)| *level))
                    .max()
                else {
                    continue;
                };
                if count == 0 {
                    continue;
                }
                if let Some(first_target) = reward.targets.first() {
                    if let Some((target_level, zone, room)) = snapshots.get(first_target) {
                        if zone == &reward.zone
                            && room == &reward.room
                            && reward.mob_level >= *target_level
                        {
                            let cap = crate::script::get_murim_config_int("약초나올확률") as f64;
                            let chance = (((reward.mob_level - *target_level) as f64 * 0.01)
                                + 0.05
                                + f64::from(reward.difficulty))
                            .min(cap);
                            if chance
                                >= rand::Rng::gen_range(&mut rand::thread_rng(), 0..=99) as f64
                            {
                                let herbs =
                                    crate::script::get_murim_main_config_list("내공아이템리스트");
                                if !herbs.is_empty() {
                                    let selected = rand::Rng::gen_range(
                                        &mut rand::thread_rng(),
                                        0..herbs.len(),
                                    );
                                    if let Some(key) = herbs[selected].clone().into_string().ok() {
                                        if let Some((item, _)) =
                                            crate::script::object_from_item_json(&key)
                                        {
                                            if let Some(player) =
                                                clients.values_mut().find_map(|client| {
                                                    client.player.as_mut().filter(|player| {
                                                        player.body.get_name() == *first_target
                                                    })
                                                })
                                            {
                                                let _ = crate::script::inventory_compat::store_acquired_object(
                                                    &mut player.body.object,
                                                    item,
                                                    true,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let mut first_contributor = true;
                for contributor in same_room_targets {
                    if !reward.damage_map.contains_key(&contributor) {
                        continue;
                    }
                    let exp = calculate_exp_reward_for_level(reward.mob_level, max_level)
                        .div_euclid(count);
                    let gold = calculate_gold_reward_for_level(reward.mob_level, reward.mob_gold)
                        .div_euclid(count);
                    let difficulty = crate::world::DifficultyConfig::get(reward.difficulty);
                    let bonus_exp = difficulty.bonus_exp(exp);
                    let bonus_gold = difficulty.bonus_gold(gold);
                    let Some(client) = clients.values_mut().find(|client| {
                        client
                            .player
                            .as_ref()
                            .is_some_and(|player| player.body.get_name() == contributor)
                    }) else {
                        continue;
                    };
                    let Some(player) = client.player.as_mut() else {
                        continue;
                    };
                    let old_max_hp = player.body.get_int("최고체력");
                    let leveled = player.body.add_exp(exp.saturating_add(bonus_exp));
                    let hp_increase = player.body.get_int("최고체력") - old_max_hp;
                    player.body.set(
                        "은전",
                        player
                            .body
                            .get_int("은전")
                            .saturating_add(gold)
                            .saturating_add(bonus_gold),
                    );
                    let kill_key = format!("{} 성격플킬", reward.personality);
                    player
                        .body
                        .set(&kill_key, player.body.get_int(&kill_key).saturating_add(1));
                    crate::script::combat_commands::queue_combat_presentation_event(
                        &mut player.body,
                        serde_json::json!({
                            "kind": "reward", "mob": reward.mob_name,
                            "exp": exp, "gold": gold,
                            "bonus_exp": bonus_exp, "bonus_gold": bonus_gold,
                        }),
                    );
                    let mut observer_loot = Vec::new();
                    if first_contributor
                        && crate::script::config_is_enabled(
                            &player.body.get_string("설정상태"),
                            "자동습득",
                        )
                    {
                        let max_items =
                            crate::script::get_murim_config_int("사용자아이템갯수").max(0) as usize;
                        if reward.mob_level >= 2_000
                            && rand::Rng::gen_range(&mut rand::thread_rng(), 0..=99) < 1
                        {
                            let keys = special_drop_keys();
                            if !keys.is_empty() {
                                let key = &keys
                                    [rand::Rng::gen_range(&mut rand::thread_rng(), 0..keys.len())];
                                if let Some((template, _)) =
                                    crate::script::object_from_item_json(key)
                                {
                                    if let Ok(template) = template.lock() {
                                        let weight = template.getInt("무게");
                                        if player.body.get_item_count() <= max_items
                                            && player.body.get_item_weight().saturating_add(weight)
                                                < player.body.get_str().saturating_mul(10)
                                        {
                                            let mut item = template.deepclone();
                                            if rand::Rng::gen_range(&mut rand::thread_rng(), 0..=99)
                                                < 30
                                            {
                                                crate::script::apply_item_magic_with_roll(
                                                    &mut item,
                                                    reward.mob_level,
                                                    0,
                                                    false,
                                                    &mut |min, max| {
                                                        rand::Rng::gen_range(
                                                            &mut rand::thread_rng(),
                                                            min..=max,
                                                        )
                                                    },
                                                );
                                            }
                                            let name = item.getString("이름");
                                            observer_loot.push(name.clone());
                                            let _ = crate::script::inventory_compat::store_acquired_object(
                                                &mut player.body.object,
                                                Arc::new(std::sync::Mutex::new(item)),
                                                true,
                                            );
                                            crate::script::combat_commands::queue_combat_presentation_event(
                                                &mut player.body,
                                                serde_json::json!({ "kind": "loot", "item": name }),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(mut world) = get_world_state().write() {
                            if let Some(mobs) = world
                                .mob_cache
                                .get_all_mobs_in_room_mut(&reward.zone, &reward.room)
                            {
                                if let Some(mob) = mobs
                                    .iter_mut()
                                    .find(|mob| mob.instance_id == reward.instance_id)
                                {
                                    while let Some(item) = mob.inventory.first().cloned() {
                                        let (weight, name, index, one_item) = match item.lock() {
                                            Ok(item) => (
                                                item.getInt("무게"),
                                                item.getString("이름"),
                                                item.getString("인덱스"),
                                                item.checkAttr("아이템속성", "단일아이템"),
                                            ),
                                            Err(_) => break,
                                        };
                                        if player.body.get_item_count() > max_items
                                            || player.body.get_item_weight().saturating_add(weight)
                                                > player.body.get_str().saturating_mul(10)
                                        {
                                            break;
                                        }
                                        if item.lock().ok().is_some_and(|item| {
                                            !crate::script::inventory_compat::can_accept_object(
                                                &player.body.object,
                                                &item,
                                            )
                                        }) {
                                            break;
                                        }
                                        let item = mob.inventory.remove(0);
                                        observer_loot.push(name.clone());
                                        let accepted =
                                            crate::script::inventory_compat::store_acquired_object(
                                                &mut player.body.object,
                                                item,
                                                true,
                                            );
                                        if accepted && one_item {
                                            crate::oneitem::oneitem_have(&index, &contributor);
                                        }
                                        crate::script::combat_commands::queue_combat_presentation_event(
                                            &mut player.body,
                                            serde_json::json!({
                                                "kind": "loot", "item": name,
                                            }),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    crate::script::combat_commands::queue_combat_presentation_event(
                        &mut player.body,
                        serde_json::json!({
                            "kind": "reward_observer", "player": contributor,
                            "loot": observer_loot,
                        }),
                    );
                    if leveled {
                        let max_hp = player.body.get_int("최고체력");
                        let armor = player.body.get_int("맷집");
                        crate::script::combat_commands::queue_combat_presentation_event(
                            &mut player.body,
                            serde_json::json!({
                                "kind": "level_up", "hp_increase": hp_increase,
                                "max_hp": max_hp, "armor": armor,
                            }),
                        );
                    }
                    first_contributor = false;
                    if !self.combat_render.contains(&client.addr) {
                        self.combat_render.push(client.addr);
                    }
                }
                let death_event = ["이벤트 $%소멸이벤트%", "이벤트: $%소멸이벤트%"]
                    .into_iter()
                    .find(|key| reward.mob_data.events.contains_key(*key));
                if let (Some(event_key), Some(last_target)) = (death_event, reward.targets.last()) {
                    let result = clients.values_mut().find_map(|client| {
                        let player = client.player.as_mut()?;
                        (player.body.get_name() == *last_target).then(|| {
                            crate::world::event::do_event(
                                &mut player.body,
                                &reward.mob_data,
                                event_key,
                                &[],
                                &reward.mob_key,
                                None,
                                None,
                            )
                        })
                    });
                    match result {
                        Some(crate::command::CommandResult::MobEventEnter {
                            output_lines,
                            set_position,
                            mob_key,
                            event_key,
                            words,
                            line_num,
                            prompt,
                            resume_func,
                            ..
                        }) => {
                            if let Some((zone, room)) = set_position {
                                if let Ok(mut world) = get_world_state().write() {
                                    if world.room_cache.get_room(&zone, &room).is_ok() {
                                        world.set_player_position(
                                            last_target,
                                            crate::world::PlayerPosition::new(
                                                zone.clone(),
                                                room.clone(),
                                            ),
                                        );
                                        world.spawn_mobs_for_room(&zone, &room);
                                    }
                                }
                            }
                            if let Some(client) = clients.values_mut().find(|client| {
                                client
                                    .player
                                    .as_ref()
                                    .is_some_and(|player| player.body.get_name() == *last_target)
                            }) {
                                client.pending_input = Some(PendingInput::EventEnter {
                                    mob_key,
                                    event_key,
                                    words,
                                    line_num,
                                    resume_func,
                                });
                                let mut output = output_lines.join("\r\n");
                                if !output.is_empty() {
                                    output.push_str("\r\n");
                                }
                                output.push_str(&prompt);
                                output.push_str("\r\n");
                                let _ = client.sender.send(output);
                            }
                        }
                        Some(crate::command::CommandResult::StartScript {
                            script_name,
                            lines,
                            use_rhai,
                        }) => {
                            if let Some(client) = clients.values_mut().find(|client| {
                                client
                                    .player
                                    .as_ref()
                                    .is_some_and(|player| player.body.get_name() == *last_target)
                            }) {
                                if let Some(player) = client.player.as_mut() {
                                    let (out_lines, next) = if use_rhai {
                                        run_script_chunk_rhai(
                                            &mut player.body,
                                            &script_name,
                                            None,
                                            None,
                                            None,
                                            None,
                                        )
                                    } else {
                                        run_script_chunk(&mut player.body, &lines, 0, None, None)
                                    };
                                    let mut output = out_lines.join("\r\n");
                                    if !output.is_empty() {
                                        output.push_str("\r\n");
                                    }
                                    if let ScriptNext::Wait {
                                        line_num,
                                        prompt,
                                        persist_temp,
                                        from_confirm,
                                        script_ob,
                                        script_resume_op,
                                    } = next
                                    {
                                        client.pending_input = Some(PendingInput::Script {
                                            name: script_name,
                                            lines: if use_rhai { vec![] } else { lines },
                                            line_num,
                                            temp_input: persist_temp,
                                            from_confirm,
                                            script_ob,
                                            script_resume_op,
                                        });
                                        output.push_str(&prompt);
                                        output.push_str("\r\n");
                                    }
                                    if !output.is_empty() {
                                        let _ = client.sender.send(output);
                                    }
                                }
                            }
                        }
                        Some(crate::command::CommandResult::MobEvent {
                            output_lines,
                            set_position,
                            ..
                        }) => {
                            if let Some((zone, room)) = set_position {
                                if let Ok(mut world) = get_world_state().write() {
                                    if world.room_cache.get_room(&zone, &room).is_ok() {
                                        world.set_player_position(
                                            last_target,
                                            crate::world::PlayerPosition::new(
                                                zone.clone(),
                                                room.clone(),
                                            ),
                                        );
                                        world.spawn_mobs_for_room(&zone, &room);
                                    }
                                }
                            }
                            let output = output_lines.join("\r\n");
                            if !output.is_empty() {
                                if let Some(client) = clients.values().find(|client| {
                                    client.player.as_ref().is_some_and(|player| {
                                        player.body.get_name() == *last_target
                                    })
                                }) {
                                    let _ = client.sender.send(output + "\r\n");
                                }
                            }
                        }
                        Some(crate::command::CommandResult::Output(output)) => {
                            if let Some(client) = clients.values().find(|client| {
                                client
                                    .player
                                    .as_ref()
                                    .is_some_and(|player| player.body.get_name() == *last_target)
                            }) {
                                let _ = client.sender.send(output + "\r\n");
                            }
                        }
                        Some(crate::command::CommandResult::OutputAndSendToUsers(
                            output,
                            deliveries,
                        )) => {
                            if let Some(client) = clients.values().find(|client| {
                                client
                                    .player
                                    .as_ref()
                                    .is_some_and(|player| player.body.get_name() == *last_target)
                            }) {
                                let _ = client.sender.send(output + "\r\n");
                            }
                            for (name, message) in deliveries {
                                if let Some(client) = clients.values().find(|client| {
                                    client
                                        .player
                                        .as_ref()
                                        .is_some_and(|player| player.body.get_name() == name)
                                }) {
                                    let _ = client.sender.send(message + "\r\n");
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        drop(clients);

        for (room, message) in room_messages {
            broadcaster.tell_room(&room, &message, None);
        }

        if !occupied_player_names.is_empty() {
            let rooms = {
                let world = get_world_state().read().ok();
                let mut rooms = Vec::new();
                if let Some(world) = world {
                    for name in occupied_player_names {
                        if let Some(position) = world.get_player_position(&name) {
                            let room = (position.zone.clone(), position.room.clone());
                            if !rooms.contains(&room) {
                                rooms.push(room);
                            }
                        }
                    }
                }
                rooms
            };
            if !rooms.is_empty() {
                let messages = if let Ok(mut world) = get_world_state().write() {
                    // Python updates floor items before mobs in Room.update.
                    // Expiration notifications are data-only for now; their
                    // Rhai presentation hook is installed separately.
                    let _expired = world.expire_floor_items_at(
                        &rooms,
                        chrono::Utc::now().timestamp_millis() as f64 / 1_000.0,
                    );
                    world.update_moving_mobs_at(chrono::Utc::now().timestamp_millis());
                    world.update_occupied_room_mobs(&rooms)
                } else {
                    Vec::new()
                };
                // Python Mob.say uses Room.writeRoom: no prompt is appended.
                for message in messages {
                    match message.kind {
                        crate::world::RoomMobMessageKind::Speech => broadcaster.tell_room(
                            &format!("{}:{}", message.zone, message.room),
                            &("\r\n".to_string() + message.message.as_str()),
                            None,
                        ),
                        crate::world::RoomMobMessageKind::CorpseGone => {
                            let player_names = get_world_state()
                                .read()
                                .ok()
                                .map(|world| {
                                    world.get_players_in_room(&message.zone, &message.room)
                                })
                                .unwrap_or_default();
                            let items = message
                                .revealed_items
                                .into_iter()
                                .map(|item| {
                                    serde_json::json!({
                                        "name": item.name, "ansi": item.ansi,
                                    })
                                })
                                .collect::<Vec<_>>();
                            let mut clients = broadcaster.clients.lock();
                            for player_name in player_names {
                                let Some(client) = clients.values_mut().find(|client| {
                                    client
                                        .player
                                        .as_ref()
                                        .is_some_and(|player| player.body.get_name() == player_name)
                                }) else {
                                    continue;
                                };
                                let Some(player) = client.player.as_mut() else {
                                    continue;
                                };
                                crate::script::combat_commands::queue_combat_presentation_event(
                                    &mut player.body,
                                    serde_json::json!({
                                        "kind": "room_corpse_update",
                                        "mob": message.mob_name,
                                        "items": items,
                                    }),
                                );
                                if !self.combat_render.contains(&client.addr) {
                                    self.combat_render.push(client.addr);
                                }
                            }
                        }
                        crate::world::RoomMobMessageKind::Respawn => {
                            let player_names = get_world_state()
                                .read()
                                .ok()
                                .map(|world| {
                                    world.get_players_in_room(&message.zone, &message.room)
                                })
                                .unwrap_or_default();
                            let mut clients = broadcaster.clients.lock();
                            for player_name in player_names {
                                let Some(client) = clients.values_mut().find(|client| {
                                    client
                                        .player
                                        .as_ref()
                                        .is_some_and(|player| player.body.get_name() == player_name)
                                }) else {
                                    continue;
                                };
                                let Some(player) = client.player.as_mut() else {
                                    continue;
                                };
                                crate::script::combat_commands::queue_combat_presentation_event(
                                    &mut player.body,
                                    serde_json::json!({
                                        "kind": "room_mob_respawn",
                                        "text": message.message,
                                    }),
                                );
                                if !self.combat_render.contains(&client.addr) {
                                    self.combat_render.push(client.addr);
                                }
                            }
                        }
                    }
                }
            }
        }

        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        if elapsed >= self.config.tick_interval {
            debug!("Game tick {} took {:?}", self.tick_count, elapsed);
        }
        true
    }
}

/// The subset of Python `Player.update()` whose state and ordering are represented exactly by
/// the current Rust model. Combat, death progression, auto-consume and timed defence effects are
/// intentionally not approximated here; they require their own faithful runtime state migration.
fn update_active_player(player: &mut Player, config: &GameLoopConfig) {
    player.body.tick = player.body.tick.saturating_add(1);
    let age_tick = player.body.get_int("나이오름틱").saturating_add(1);
    player.body.set("나이오름틱", age_tick);
    let age_interval = crate::script::get_murim_config_int("나이오름틱");
    if age_interval > 0 && age_tick >= age_interval {
        player.body.set("나이오름틱", 0_i64);
        let age = player.body.get_int("나이").saturating_add(1);
        player.body.set("나이", age);
        let mp_gain = if age % 60 == 0 { 60 } else { 1 };
        player.body.set(
            "최고내공",
            player.body.get_int("최고내공").saturating_add(mp_gain),
        );
        crate::script::combat_commands::queue_combat_presentation_event(
            &mut player.body,
            serde_json::json!({ "kind": "age_up" }),
        );
    }
    if player.body.tick.is_multiple_of(60)
        && player.body.get_string("무림별호").is_empty()
        && player.body.get_int("0 성격플킬")
            + player.body.get_int("1 성격플킬")
            + player.body.get_int("2 성격플킬")
            >= crate::script::get_murim_config_int("무림별호이벤트킬수")
    {
        crate::script::combat_commands::queue_combat_presentation_event(
            &mut player.body,
            serde_json::json!({ "kind": "nickname_invitation" }),
        );
    }

    // Python saves before fight/death/recovery processing. Preserve that ordering: on tick 600
    // the persisted HP is the pre-recovery value, while the live Body recovers afterwards.
    if config.save_interval != 0 && player.body.tick.is_multiple_of(config.save_interval) {
        let name = player.body.get_name();
        if !name.is_empty() {
            let path = config.user_data_dir.join(format!("{name}.json"));
            if !save_body_to_json(&mut player.body, &path.to_string_lossy()) {
                warn!("Failed periodic save for player '{}'", name);
            }
        }
    }

    match player.body.act {
        ActState::Death => {
            let (step, insured_items) = player.body.advance_death();
            crate::script::combat_commands::queue_combat_presentation_event(
                &mut player.body,
                serde_json::json!({
                    "kind": "death_progress", "step": step,
                    "insured_items": insured_items,
                }),
            );
            return;
        }
        ActState::Fight => {}
        _ => {
            if player.body.skill.is_some() {
                player.body.stop_skill();
            }
            if !player.body.targets.is_empty() {
                player.body.clear_target(None);
            }
        }
    }

    if config.recovery_interval != 0 && player.body.tick.is_multiple_of(config.recovery_interval) {
        recover_like_python(&mut player.body);
    }
}

fn expire_player_skill_effects(body: &mut Body) {
    let mut retained = Vec::new();
    let mut released = Vec::new();
    let mut released_names = Vec::new();
    for mut effect in body.active_skills.drain(..) {
        effect.start_time -= 1;
        if effect.start_time < 0 {
            body._str -= effect.str_bonus;
            body._dex -= effect.dex_bonus;
            body._arm -= effect.arm_bonus;
            body._mp -= effect.mp_bonus;
            body._maxmp -= effect.max_mp_bonus;
            body._hp -= effect.hp_bonus;
            body._maxhp -= effect.max_hp_bonus;
            released_names.push(effect.name);
            released.push(effect.release_script);
        } else {
            retained.push(effect);
        }
    }
    body.active_skills = retained;
    body.sync_active_skills_to_attrs();
    if !released.is_empty() {
        crate::script::combat_commands::queue_combat_presentation_event(
            body,
            serde_json::json!({ "kind": "defense_expired", "scripts": released }),
        );
        body.temp_mut().insert(
            "_expired_auto_skills".to_string(),
            crate::object::Value::String(released_names.join("\n")),
        );
    }
}

fn append_round(target: &mut crate::combat::CombatRound, source: crate::combat::CombatRound) {
    target.damage_dealt = target.damage_dealt.saturating_add(source.damage_dealt);
    target.damage_taken = target.damage_taken.saturating_add(source.damage_taken);
    target.combat_ended |= source.combat_ended;
    target.player_died |= source.player_died;
    target.target_died |= source.target_died;
    target
        .presentation_events
        .extend(source.presentation_events);
}

fn award_mob_death(
    mob: &mut crate::world::MobInstance,
    data: &crate::world::RawMobData,
    round: &mut crate::combat::CombatRound,
    pending_rewards: &mut Vec<PendingMobReward>,
) {
    if !mob.alive {
        return;
    }
    if mob.inventory.is_empty() {
        generate_mob_corpse_items(mob, data, &mut |upper| {
            rand::Rng::gen_range(&mut rand::thread_rng(), 0..=upper)
        });
    }
    let reward = PendingMobReward {
        instance_id: mob.instance_id,
        mob_name: mob.name.clone(),
        mob_level: mob.level,
        mob_gold: data.gold,
        difficulty: mob.difficulty,
        personality: data.personality,
        zone: mob.zone.clone(),
        room: mob.room.clone(),
        targets: mob.targets.clone(),
        damage_map: mob.damage_map.clone(),
        mob_key: mob.mob_key.clone(),
        mob_data: data.clone(),
    };
    mob.hp = 0;
    mob.alive = false;
    mob.act = 2;
    mob.death_time = chrono::Utc::now().timestamp();
    mob.targets.clear();
    round.presentation_events.push(serde_json::json!({
        "kind": "mob_death", "mob": mob.name, "script": data.death_script,
    }));
    pending_rewards.push(reward);
}

pub(crate) fn generate_mob_corpse_items(
    mob: &mut crate::world::MobInstance,
    data: &crate::world::RawMobData,
    roll: &mut dyn FnMut(i64) -> i64,
) {
    if !mob.inventory.is_empty() {
        return;
    }
    let difficulty_multiplier = if mob.difficulty == 0 {
        1.0
    } else {
        crate::world::DifficultyConfig::get(mob.difficulty).bonus_exp(100) as f64 / 100.0
    };
    let declarations = data
        .drop_items
        .iter()
        .map(|(key, count, chance, scale)| (key, *count, *chance, *scale, true))
        .chain(
            data.use_items
                .iter()
                .map(|(key, count, chance, scale)| (key, *count, *chance, *scale, false)),
        )
        .collect::<Vec<_>>();
    for (key, count, base_chance, scale, difficulty_scaled) in declarations {
        let chance = if difficulty_scaled {
            (base_chance as f64 * difficulty_multiplier) as i64
        } else {
            base_chance
        };
        for _ in 0..count.max(0) {
            if chance < roll(100_i64.saturating_mul(scale.max(0))) {
                continue;
            }
            let Some((template, _)) = crate::script::object_from_item_json(key) else {
                continue;
            };
            let Ok(template) = template.lock() else {
                continue;
            };
            if template.checkAttr("아이템속성", "단일아이템")
                && !crate::oneitem::oneitem_get(key).is_empty()
            {
                continue;
            }
            let mut dropped = template.deepclone();
            crate::script::apply_item_magic_with_roll(
                &mut dropped,
                mob.level,
                0,
                false,
                &mut |min, max| rand::Rng::gen_range(&mut rand::thread_rng(), min..=max),
            );
            mob.inventory
                .insert(0, Arc::new(std::sync::Mutex::new(dropped)));
        }
    }
}

fn special_drop_keys() -> Vec<String> {
    std::fs::read_to_string("data/config/dropitem.json")
        .ok()
        .and_then(|source| serde_json::from_str::<Vec<String>>(&source).ok())
        .unwrap_or_default()
}

fn process_one_mob_phase(
    player: &mut Body,
    mob: &mut crate::world::MobInstance,
    data: &crate::world::RawMobData,
    round: &mut crate::combat::CombatRound,
) {
    if round.player_died || player.get_hp() <= 0 || !mob.alive || mob.hp <= 0 {
        return;
    }
    let player_name = player.get_name();
    let mob_name = mob.name.clone();
    expire_mob_skill_effects(mob, chrono::Utc::now().timestamp());
    if let Some((script, absorbed)) = try_start_mob_defense_skill(mob, data, player, &mut || {
        rand::Rng::gen_range(&mut rand::thread_rng(), 0..=100)
    }) {
        round.presentation_events.push(serde_json::json!({
            "kind": "mob_defense", "mob": mob_name, "player": player_name,
            "script": script, "damage": absorbed,
        }));
    }
    let learned_candidates = mob_skill_candidates(mob, data);
    if mob.active_attack_skill.is_none() && !learned_candidates.is_empty() {
        let selected = rand::Rng::gen_range(&mut rand::thread_rng(), 0..learned_candidates.len());
        let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0..=100);
        try_start_mob_attack_skill(mob, data, selected, roll);
    }
    let agility = (mob.agility + mob.dex_modifier).max(0);
    if mob.combat_dex >= agility + 700 {
        mob.combat_dex = 0;
    }
    mob.combat_dex += agility + 700;
    let mut run_normal = mob.active_attack_skill.is_none();
    if let Some(mut skill) = mob.active_attack_skill.take() {
        let (steps, more, remaining_dex) = skill.get_script(mob.combat_dex);
        mob.combat_dex = remaining_dex;
        let has_wait = steps
            .iter()
            .any(|step| matches!(step.action, crate::world::skill::PatternAction::Wait));
        let mut vision_checked = false;
        for step in steps {
            match step.action {
                crate::world::skill::PatternAction::Opening => {
                    round.presentation_events.push(serde_json::json!({
                        "kind": "mob_form", "mob": mob_name, "player": player_name,
                        "script": step.message, "damage": 0,
                    }));
                }
                crate::world::skill::PatternAction::Wait => {}
                crate::world::skill::PatternAction::Attack => {
                    let training_level = mob
                        .skill_map
                        .get(&skill.name)
                        .map(|training| training.level)
                        .unwrap_or(1);
                    let mastery_bonus = match training_level {
                        11 => 10.0,
                        12 => 20.0,
                        _ => 0.0,
                    };
                    let chance = skill.probability as f64
                        + training_level as f64
                            * crate::script::get_murim_config_float("기술확률배수")
                        - ((mob.level - player.get_int("레벨") + 90).div_euclid(3)) as f64
                        + data.hit as f64 * 0.2
                        - player.get_miss() as f64 * 0.2
                        + mastery_bonus;
                    if chance < rand::Rng::gen_range(&mut rand::thread_rng(), 0..=100) as f64 {
                        round.presentation_events.push(serde_json::json!({
                            "kind": "mob_fail", "mob": mob_name, "player": player_name,
                            "script": skill.fail_message, "damage": 0,
                        }));
                        continue;
                    }
                    if !vision_checked {
                        let learned = player.check_vision_training(&skill.name);
                        if learned {
                            round.presentation_events.push(serde_json::json!({
                                "kind": "vision_learned", "skill": skill.name,
                            }));
                        }
                        vision_checked = true;
                    }
                    let mut damage = crate::combat::processor::calculate_mob_skill_damage(
                        mob,
                        data,
                        player,
                        &skill,
                        rand::Rng::gen_range(&mut rand::thread_rng(), 0..=100),
                    );
                    if player.get_vision_damage_modifier(&skill.name) == 0.5 {
                        damage = damage.div_euclid(2);
                    }
                    let lethal = player.minus_hp(damage);
                    round.damage_taken = round.damage_taken.saturating_add(damage);
                    round.presentation_events.push(serde_json::json!({
                        "kind": "mob_attack", "mob": mob_name, "player": player_name,
                        "script": step.message, "damage": damage,
                    }));
                    if lethal {
                        player.act = ActState::Death;
                        player.unwear_all();
                        player.clear_targets_death();
                        player.clear_skills();
                        player.set_death_step(0);
                        round
                            .presentation_events
                            .push(serde_json::json!({ "kind": "player_death" }));
                        round.player_died = true;
                        round.combat_ended = true;
                        break;
                    }
                }
            }
        }
        if more && !round.player_died {
            mob.active_attack_skill = Some(skill);
        }
        run_normal = !more || has_wait;
    }
    if run_normal && !round.player_died {
        let count = consume_mob_actions(&mut mob.combat_dex);
        let snapshot = mob.clone();
        for _ in 0..count {
            let strike = process_mob_strike(player, &snapshot, data);
            let ended = strike.combat_ended;
            append_round(round, strike);
            if ended {
                break;
            }
        }
    }
}

fn consume_player_actions(dex: &mut i32) -> usize {
    let count = i64::from(*dex).div_euclid(700).max(0) as usize;
    *dex = i64::from(*dex).rem_euclid(700) as i32;
    count
}

fn consume_mob_actions(dex: &mut i64) -> usize {
    let count = dex.div_euclid(700).max(0) as usize;
    *dex = dex.rem_euclid(700);
    count
}

/// Advance one Python `Player.update()` combat tick.
///
/// The Rhai attack command stores the target mob key in the body temporary
/// attributes.  The previous loop never consumed that state, so combat could
/// enter FIGHT but could never make a round.  Keep the visible formatting in
/// the existing combat processor and only perform state routing here.
fn process_combat_tick(
    player: &mut Player,
    pending_rewards: &mut Vec<PendingMobReward>,
) -> Vec<(String, String)> {
    let name = player.body.get_name();
    let target_keys = crate::script::combat_commands::combat_target_ids(&player.body);
    let target_instance_ids =
        crate::script::combat_commands::combat_target_instance_ids(&player.body);
    let Some(target_key) = target_keys.first().cloned() else {
        player.body.act = ActState::Stand;
        return Vec::new();
    };

    let mut world = match get_world_state().write() {
        Ok(world) => world,
        Err(_) => return Vec::new(),
    };
    let Some(position) = world.get_player_position(&name).cloned() else {
        return Vec::new();
    };
    let Some(data) = world.mob_cache.get_mob(&target_key).cloned() else {
        return Vec::new();
    };
    let target_metadata = target_keys
        .iter()
        .filter_map(|key| {
            world
                .mob_cache
                .get_mob(key)
                .cloned()
                .map(|data| (key.clone(), data))
        })
        .collect::<Vec<_>>();
    let Some(mobs) = world
        .mob_cache
        .get_all_mobs_in_room_mut(&position.zone, &position.room)
    else {
        player.body.act = ActState::Stand;
        return Vec::new();
    };
    let Some(index) = mobs.iter().position(|mob| {
        target_instance_ids
            .first()
            .map_or(mob.mob_key == target_key, |id| mob.instance_id == *id)
    }) else {
        player.body.clear_target(None);
        player.body.act = ActState::Stand;
        return Vec::new();
    };
    let instance = mobs[index].clone();
    let effective_player_dex = player.body.get_dex();
    player.body.dex = i64::from(player.body.dex)
        .saturating_add(effective_player_dex.max(0) + 700)
        .min(i32::MAX as i64) as i32;
    let mut round = crate::combat::CombatRound::new();
    player.body.act = ActState::Fight;
    let mut run_player_normal = true;
    if target_instance_ids.is_empty() {
        for (key, target_data) in &target_metadata {
            let Some(target_index) = mobs.iter().position(|mob| {
                mob.mob_key == *key && mob.alive && mob.targets.iter().any(|target| target == &name)
            }) else {
                continue;
            };
            process_one_mob_phase(
                &mut player.body,
                &mut mobs[target_index],
                target_data,
                &mut round,
            );
            if round.player_died {
                break;
            }
        }
    } else {
        for instance_id in &target_instance_ids {
            let Some(target_index) = mobs.iter().position(|mob| {
                mob.instance_id == *instance_id
                    && mob.alive
                    && mob.targets.iter().any(|target| target == &name)
            }) else {
                continue;
            };
            let Some(target_data) = target_metadata
                .iter()
                .find(|(key, _)| key == &mobs[target_index].mob_key)
                .map(|(_, data)| data)
            else {
                continue;
            };
            process_one_mob_phase(
                &mut player.body,
                &mut mobs[target_index],
                target_data,
                &mut round,
            );
            if round.player_died {
                break;
            }
        }
    }

    // Python Player.doSkill(): automatic martial arts are attempted at the
    // start of a combat round when enabled and resources permit it.
    if !round.target_died && !round.combat_ended {
        let already_active = player.body.skill.clone();
        let auto_enabled =
            crate::script::config_is_enabled(&player.body.get_string("설정상태"), "자동무공시전");
        let auto_name = already_active
            .clone()
            .unwrap_or_else(|| player.body.get_string("자동무공"));
        if !auto_name.is_empty() {
            if let Some(skill) = crate::world::skill::get_skill(&auto_name) {
                let training = player
                    .body
                    .get_skill_training(&auto_name)
                    .map(|value| value.level as i32)
                    .unwrap_or(1);
                let mp_cost = match training {
                    11 => skill.mp_cost * 9 / 10,
                    12 => skill.mp_cost * 8 / 10,
                    _ => skill.mp_cost,
                };
                let max_hp = player.body.get_max_hp();
                let hp_cost = max_hp * skill.hp_cost / 100;
                let hp_req = max_hp * skill.hp_requirement / 100;
                let can_continue = if already_active.is_some() {
                    true
                } else if auto_enabled
                    && player.body.get_hp() >= hp_cost
                    && player.body.get_hp() >= hp_req
                    && player.body.get_mp() >= skill.mp_cost
                {
                    player
                        .body
                        .set("내공", player.body.get_int("내공") - mp_cost);
                    player.body.set("체력", player.body.get_hp() - hp_cost);
                    player.body.get_skill(&auto_name);
                    player.body.add_str(skill.bonus as i32, false);
                    true
                } else {
                    false
                };
                if can_continue {
                    let mut runtime_skill = skill.clone();
                    runtime_skill.end = player
                        .body
                        .temp()
                        .get("_skill_turn")
                        .and_then(|value| match value {
                            crate::object::Value::Int(value) => Some((*value - 1).max(0) as i32),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let (steps, more, remaining_dex) =
                        runtime_skill.get_script(i64::from(player.body.dex));
                    player.body.dex = remaining_dex.clamp(0, i32::MAX as i64) as i32;
                    let has_wait = steps.iter().any(|step| {
                        matches!(step.action, crate::world::skill::PatternAction::Wait)
                    });
                    run_player_normal = !more || has_wait;
                    for step in steps {
                        if !matches!(step.action, crate::world::skill::PatternAction::Attack) {
                            if matches!(step.action, crate::world::skill::PatternAction::Opening) {
                                round.presentation_events.push(serde_json::json!({
                                    "kind": "player_skill_form",
                                    "mob": instance.name,
                                    "player": player.body.get_name(),
                                    "weapon": player.body.get_weapon_name(),
                                    "script": step.message,
                                }));
                            }
                            continue;
                        }
                        let attack_indices = if skill.is_all_attack() {
                            if target_instance_ids.is_empty() {
                                target_metadata
                                    .iter()
                                    .filter_map(|(key, _)| {
                                        mobs.iter().position(|mob| mob.mob_key == *key && mob.alive)
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                target_instance_ids
                                    .iter()
                                    .filter_map(|id| {
                                        mobs.iter()
                                            .position(|mob| mob.instance_id == *id && mob.alive)
                                    })
                                    .collect::<Vec<_>>()
                            }
                        } else if mobs[index].alive {
                            vec![index]
                        } else {
                            Vec::new()
                        };
                        for target_index in attack_indices {
                            let target = mobs[target_index].clone();
                            let target_data = target_metadata
                                .iter()
                                .find(|(key, _)| key == &target.mob_key)
                                .map(|(_, data)| data)
                                .unwrap_or(&data);
                            let skill_round = calculate_skill_damage_against(
                                &player.body,
                                &skill,
                                training,
                                target_data,
                                &target,
                                &target.name,
                            );
                            if skill_round.hit {
                                let old_hp = mobs[target_index].hp;
                                mobs[target_index].hp = mobs[target_index]
                                    .hp
                                    .saturating_sub(skill_round.final_damage);
                                let applied = old_hp.saturating_sub(mobs[target_index].hp);
                                mobs[target_index].record_player_damage(&name, applied);
                                round.presentation_events.push(serde_json::json!({
                                    "kind": "player_skill_attack", "mob": target.name,
                                    "player": player.body.get_name(),
                                    "weapon": player.body.get_weapon_name(),
                                    "script": step.message.clone(), "damage": skill_round.final_damage,
                                }));
                                crate::combat::processor::apply_player_attack_training(
                                    &mut player.body,
                                    true,
                                    &mut round,
                                );
                                if mobs[target_index].hp <= 0 {
                                    award_mob_death(
                                        &mut mobs[target_index],
                                        target_data,
                                        &mut round,
                                        pending_rewards,
                                    );
                                    if target_instance_ids.is_empty() {
                                        crate::script::combat_commands::remove_combat_target_id(
                                            &mut player.body,
                                            &target.mob_key,
                                        );
                                    } else {
                                        crate::script::combat_commands::remove_combat_target_instance_id(
                                            &mut player.body,
                                            target.instance_id,
                                        );
                                    }
                                    if target_index == index {
                                        round.target_died = true;
                                    }
                                }
                            } else {
                                round.presentation_events.push(serde_json::json!({
                                    "kind": "player_skill_fail", "mob": target.name,
                                    "player": player.body.get_name(),
                                    "weapon": player.body.get_weapon_name(),
                                    "script": skill.fail_message.clone(), "damage": 0,
                                }));
                                crate::combat::processor::apply_player_attack_training(
                                    &mut player.body,
                                    false,
                                    &mut round,
                                );
                            }
                        }
                    }
                    if !more {
                        let (_, leveled, _) =
                            crate::script::skill_up_python(&mut player.body, &skill);
                        if leveled {
                            round
                                .presentation_events
                                .push(serde_json::json!({ "kind": "skill_level_up" }));
                        }
                        player.body.stop_skill();
                        player.body.temp_mut().remove("_skill_turn");
                    } else {
                        player.body.temp_mut().insert(
                            "_skill_turn".to_string(),
                            crate::object::Value::Int(i64::from(runtime_skill.end + 1)),
                        );
                    }
                }
            }
        }
    }
    if run_player_normal && !round.target_died && !round.combat_ended {
        let player_actions = consume_player_actions(&mut player.body.dex);
        let mut strike_target = mobs[index].clone();
        for _ in 0..player_actions {
            let strike = process_player_strike(&mut player.body, &strike_target, &data);
            strike_target.hp = strike_target.hp.saturating_sub(strike.damage_dealt);
            let ended = strike.combat_ended;
            append_round(&mut round, strike);
            if ended {
                break;
            }
        }
    }
    if round.damage_dealt > 0 {
        let old_hp = mobs[index].hp;
        mobs[index].hp = mobs[index].hp.saturating_sub(round.damage_dealt);
        let applied = old_hp.saturating_sub(mobs[index].hp);
        mobs[index].record_player_damage(&name, applied);
    }
    if mobs[index].hp <= 0 && mobs[index].alive {
        award_mob_death(&mut mobs[index], &data, &mut round, pending_rewards);
        round.target_died = true;
        round.combat_ended = true;
    }
    if round.target_died || mobs[index].hp <= 0 {
        mobs[index].hp = 0;
        mobs[index].alive = false;
        mobs[index].act = 2;
        mobs[index].targets.clear();
        let next_target = if target_instance_ids.is_empty() {
            crate::script::combat_commands::remove_combat_target_id(&mut player.body, &target_key)
                .and_then(|next_key| {
                    mobs.iter()
                        .find(|mob| mob.mob_key == next_key && mob.alive)
                        .map(|mob| (next_key, mob.name.clone()))
                })
        } else {
            crate::script::combat_commands::remove_combat_target_instance_id(
                &mut player.body,
                mobs[index].instance_id,
            );
            crate::script::combat_commands::combat_target_instance_ids(&player.body)
                .into_iter()
                .find_map(|id| {
                    mobs.iter()
                        .find(|mob| mob.instance_id == id && mob.alive)
                        .map(|mob| (mob.mob_key.clone(), mob.name.clone()))
                })
        };
        if let Some((next_key, next_name)) = next_target {
            player.body.act = ActState::Fight;
            player.body.temp_mut().insert(
                "_attack_target_key".to_string(),
                crate::object::Value::String(next_key),
            );
            player.body.temp_mut().insert(
                "_attack_target".to_string(),
                crate::object::Value::String(next_name),
            );
        } else {
            player.body.clear_target(None);
            player.body.act = ActState::Stand;
            player.body.temp_mut().remove("_combat_target_ids");
            player.body.temp_mut().remove("_combat_target_instance_ids");
            player.body.temp_mut().remove("_attack_target_key");
            player.body.temp_mut().remove("_attack_target");
            player.body.temp_mut().remove("_attack_target_index");
        }
    }
    if round.player_died {
        // Python Player.die(): a lethal hit cancels the complete automatic
        // route so it cannot resume after the funeral-home recovery sequence.
        player.auto_move_list.clear();
        player.body.clear_target(None);
        player.body.temp_mut().remove("_combat_target_ids");
        player.body.temp_mut().remove("_combat_target_instance_ids");
        player.body.temp_mut().remove("_attack_target_key");
    } else if round.damage_taken != 0 {
        apply_python_damage_recovery(
            &mut player.body,
            round.damage_taken,
            crate::world::difficulty::difficulty_from_zone(&position.zone) > 0,
            &mut round,
        );
    }
    for event in round.presentation_events.drain(..) {
        crate::script::combat_commands::queue_combat_presentation_event(&mut player.body, event);
    }
    if !round.player_died {
        crate::script::combat_commands::queue_combat_presentation_event(
            &mut player.body,
            serde_json::json!({ "kind": "combat_prompt" }),
        );
    }
    Vec::new()
}

fn apply_python_damage_recovery(
    body: &mut Body,
    damage: i64,
    difficult: bool,
    round: &mut crate::combat::CombatRound,
) {
    let maximum_hp = body.get_int("최고체력");
    let armor_ratio = crate::script::get_murim_config_int("맷집증가비율");
    // Preserve Python's `float(ratio) // 100` before multiplying by HP.
    let threshold = maximum_hp.saturating_mul(armor_ratio.div_euclid(100));
    let mut recovery_damage = damage;
    if damage > threshold {
        recovery_damage = damage.saturating_sub(threshold);
        let extra = if threshold > 0 {
            recovery_damage.div_euclid(threshold)
        } else {
            recovery_damage
        };
        let armor_exp = 1_i64
            .saturating_add(extra)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX));
        if body.add_arm(armor_exp as i32) {
            round
                .presentation_events
                .push(serde_json::json!({ "kind": "armor_up" }));
        }
    }
    let effects = body.active_skills.clone();
    for effect in effects {
        if effect.category != "전투회복" {
            continue;
        }
        let percent = if difficult {
            effect.recovery_percent.div_euclid(2)
        } else {
            effect.recovery_percent
        };
        let recovered = recovery_damage.saturating_mul(percent).div_euclid(100);
        body.set("체력", body.get_hp().saturating_add(recovered));
        round.presentation_events.push(serde_json::json!({
            "kind": "combat_recovery", "script": effect.recovery_script,
            "amount": recovered,
        }));
    }
}

pub(crate) fn try_start_mob_attack_skill(
    mob: &mut crate::world::MobInstance,
    data: &crate::world::RawMobData,
    selected: usize,
    probability_roll: i64,
) -> bool {
    if mob.active_attack_skill.is_some() {
        return false;
    }
    let candidates = mob_skill_candidates(mob, data);
    let Some((skill_name, hp_threshold, probability)) = candidates.get(selected) else {
        return false;
    };
    let Some(skill) = crate::world::skill::get_skill(skill_name) else {
        return false;
    };
    if !matches!(skill.skill_type, crate::world::skill::SkillType::Combat)
        || mob.hp > mob.max_hp.saturating_mul(*hp_threshold).div_euclid(100)
        || *probability < probability_roll
        || skill.mp_cost > mob.mp
    {
        return false;
    }
    mob.mp -= skill.mp_cost;
    mob.active_attack_skill = Some(skill);
    mob.combat_dex = 0;
    true
}

fn mob_skill_candidates(
    mob: &crate::world::MobInstance,
    data: &crate::world::RawMobData,
) -> Vec<(String, i64, i64)> {
    let mut candidates = data.skills.clone();
    candidates.extend(
        mob.learned_skills
            .iter()
            .filter(|name| !data.skills.iter().any(|(saved, _, _)| saved == *name))
            .map(|name| (name.clone(), 100, 100)),
    );
    candidates
}

fn expire_mob_skill_effects(mob: &mut crate::world::MobInstance, now: i64) {
    let mut retained = Vec::new();
    for effect in mob.skill_effects.drain(..) {
        if effect.expires_at < now {
            mob.str_modifier -= effect.str_bonus;
            mob.dex_modifier -= effect.dex_bonus;
            mob.arm_modifier -= effect.arm_bonus;
            mob.mp_modifier -= effect.mp_bonus;
            mob.max_mp_modifier -= effect.max_mp_bonus;
            mob.hp_modifier -= effect.hp_bonus;
            mob.max_hp_modifier -= effect.max_hp_bonus;
            mob.skills.retain(|name| name != &effect.name);
        } else {
            retained.push(effect);
        }
    }
    mob.skill_effects = retained;
}

pub(crate) fn try_start_mob_defense_skill(
    mob: &mut crate::world::MobInstance,
    data: &crate::world::RawMobData,
    player: &mut Body,
    roll: &mut dyn FnMut() -> i64,
) -> Option<(String, i64)> {
    for (skill_name, hp_threshold, probability) in mob_skill_candidates(mob, data) {
        let Some(skill) = crate::world::skill::get_skill(&skill_name) else {
            continue;
        };
        if matches!(skill.skill_type, crate::world::skill::SkillType::Combat)
            || mob.hp > mob.max_hp.saturating_mul(hp_threshold).div_euclid(100)
            || probability < roll()
            || skill.mp_cost > mob.mp
            || mob.skill_effects.iter().any(|effect| {
                effect.name == skill.name
                    || (!skill.category.is_empty() && skill.category == effect.anti_type)
            })
            || player.active_skills.iter().any(|effect| {
                effect.name == skill.name
                    || (!skill.category.is_empty() && skill.category == effect.anti_type)
            })
        {
            continue;
        }
        mob.mp -= skill.mp_cost;
        let training_level = mob
            .skill_map
            .get(&skill.name)
            .map(|training| training.level)
            .unwrap_or(1);
        let duration = skill.defense_time.saturating_add(
            skill
                .defense_time_increase
                .saturating_mul(training_level.saturating_sub(1)),
        );
        let mut absorbed = 0_i64;
        if let Some(against_name) = &skill.against_skill {
            if let Some(against) = crate::world::skill::get_skill(against_name) {
                match skill.category.as_str() {
                    "내공흡수" if player.get_mp() > 0 => {
                        absorbed = (player._mp as i64 * against.mp_bonus)
                            .div_euclid(100)
                            .saturating_mul(-1)
                            .max(0)
                            .min((mob.max_mp - mob.mp).max(0));
                        mob.mp += absorbed;
                        player._mp -= absorbed as i32;
                    }
                    "체력흡수" if player.get_hp() > 0 => {
                        absorbed = (player._hp as i64 * against.hp_bonus)
                            .div_euclid(100)
                            .saturating_mul(-1)
                            .max(0)
                            .min((mob.max_hp - mob.hp).max(0));
                        mob.hp += absorbed;
                        player._hp -= absorbed as i32;
                    }
                    "내공감소" => {
                        let mut effect = crate::player::ActiveSkill::new(
                            against.name.clone(),
                            against
                                .defense_time
                                .saturating_add(
                                    against
                                        .defense_time_increase
                                        .saturating_mul(training_level.saturating_sub(1)),
                                )
                                .clamp(i32::MIN as i64, i32::MAX as i64)
                                as i32,
                        );
                        effect.mp_bonus = against.mp_bonus as i32;
                        effect.max_mp_bonus = against.max_mp_bonus as i32;
                        effect.anti_type = against.deny.clone();
                        player._mp += effect.mp_bonus;
                        player._maxmp += effect.max_mp_bonus;
                        player.active_skills.push(effect);
                    }
                    "체력감소" => {
                        let mut effect = crate::player::ActiveSkill::new(
                            against.name.clone(),
                            against
                                .defense_time
                                .saturating_add(
                                    against
                                        .defense_time_increase
                                        .saturating_mul(training_level.saturating_sub(1)),
                                )
                                .clamp(i32::MIN as i64, i32::MAX as i64)
                                as i32,
                        );
                        effect.hp_bonus = against.hp_bonus as i32;
                        effect.max_hp_bonus = against.max_hp_bonus as i32;
                        effect.anti_type = against.deny.clone();
                        player._hp += effect.hp_bonus;
                        player._maxhp += effect.max_hp_bonus;
                        player.active_skills.push(effect);
                    }
                    _ => {}
                }
            }
        }
        let effect = crate::world::MobSkillEffect {
            name: skill.name.clone(),
            anti_type: skill.deny.clone(),
            expires_at: chrono::Utc::now().timestamp().saturating_add(duration),
            str_bonus: skill.str_bonus,
            dex_bonus: skill.dex_bonus,
            arm_bonus: skill.arm_bonus,
            mp_bonus: skill.mp_bonus,
            max_mp_bonus: skill.max_mp_bonus,
            hp_bonus: skill.hp_bonus,
            max_hp_bonus: skill.max_hp_bonus,
        };
        mob.str_modifier += effect.str_bonus;
        mob.dex_modifier += effect.dex_bonus;
        mob.arm_modifier += effect.arm_bonus;
        mob.mp_modifier += effect.mp_bonus;
        mob.max_mp_modifier += effect.max_mp_bonus;
        mob.hp_modifier += effect.hp_bonus;
        mob.max_hp_modifier += effect.max_hp_bonus;
        mob.skills.push(skill.name.clone());
        mob.skill_effects.push(effect);
        let script = match skill.mugong_script.as_str() {
            "" => skill.name,
            script => script.to_string(),
        };
        return Some((script, absorbed));
    }
    None
}

/// `Player.recover()` from Python: stand 10%, rest 20%, fight 5%, integer truncation and clamp.
fn recover_like_python(body: &mut Body) {
    let rate_percent = match body.act {
        ActState::Stand => 10,
        ActState::Rest => 20,
        ActState::Fight => 5,
        _ => 0,
    };
    if rate_percent == 0 {
        return;
    }

    let hp = body.get_hp();
    let max_hp = body.get_max_hp();
    if hp < max_hp {
        body.set("체력", (hp + max_hp * rate_percent / 100).min(max_hp));
    }

    let mp = body.get_mp();
    let max_mp = body.get_max_mp();
    if mp < max_mp {
        body.set("내공", (mp + max_mp * rate_percent / 100).min(max_mp));
    }
}

fn finish_pvp_death(body: &mut Body) {
    body.act = ActState::Death;
    body.unwear_all();
    body.clear_targets_death();
    body.clear_skills();
    body.set_death_step(0);
    crate::script::combat_commands::clear_pvp_target(body);
}

fn pvp_skill_phase(attacker: &mut Body, defender: &mut Body) -> (i64, bool) {
    let Some(skill_name) = attacker.skill.clone() else {
        return (0, true);
    };
    let Some(skill) = crate::world::skill::get_skill(&skill_name) else {
        attacker.stop_skill();
        return (0, true);
    };
    let training_level = attacker
        .get_skill_training(&skill_name)
        .map(|training| training.level as i32)
        .unwrap_or(1);
    let mut runtime = skill.clone();
    runtime.end = attacker
        .temp()
        .get("_skill_turn")
        .and_then(|value| match value {
            crate::object::Value::Int(value) => Some((*value - 1).max(0) as i32),
            _ => None,
        })
        .unwrap_or(0);
    let (steps, more, remaining_dex) = runtime.get_script(i64::from(attacker.dex));
    attacker.dex = remaining_dex.clamp(0, i32::MAX as i64) as i32;
    let has_wait = steps
        .iter()
        .any(|step| matches!(step.action, crate::world::skill::PatternAction::Wait));
    let attacker_name = attacker.get_name();
    let defender_name = defender.get_name();
    let mut total_damage = 0_i64;
    for step in steps {
        if matches!(step.action, crate::world::skill::PatternAction::Opening) {
            crate::script::combat_commands::queue_combat_presentation_event(
                attacker,
                serde_json::json!({
                    "kind": "pvp_skill_form_out", "target": defender_name,
                    "script": step.message, "weapon": attacker.get_weapon_name(),
                }),
            );
            crate::script::combat_commands::queue_combat_presentation_event(
                defender,
                serde_json::json!({
                    "kind": "pvp_skill_form_in", "attacker": attacker_name,
                    "script": step.message, "weapon": attacker.get_weapon_name(),
                }),
            );
            continue;
        }
        if !matches!(step.action, crate::world::skill::PatternAction::Attack) {
            continue;
        }
        let result =
            crate::combat::calculate_pvp_skill_damage(attacker, defender, &skill, training_level);
        let (out_kind, in_kind, script, damage) = if result.hit {
            (
                "pvp_skill_out",
                "pvp_skill_in",
                step.message,
                result.final_damage,
            )
        } else {
            (
                "pvp_skill_fail_out",
                "pvp_skill_fail_in",
                skill.fail_message.clone(),
                0,
            )
        };
        crate::script::combat_commands::queue_combat_presentation_event(
            attacker,
            serde_json::json!({
                "kind": out_kind, "target": defender_name,
                "script": script.clone(), "damage": damage, "weapon": attacker.get_weapon_name(),
            }),
        );
        crate::script::combat_commands::queue_combat_presentation_event(
            defender,
            serde_json::json!({
                "kind": in_kind, "attacker": attacker_name,
                "script": script, "damage": damage, "weapon": attacker.get_weapon_name(),
            }),
        );
        let mut training = crate::combat::CombatRound::new();
        crate::combat::processor::apply_player_attack_training(attacker, result.hit, &mut training);
        for event in training.presentation_events {
            crate::script::combat_commands::queue_combat_presentation_event(attacker, event);
        }
        if result.hit {
            total_damage = total_damage.saturating_add(result.final_damage);
        }
    }
    if !more {
        let (_, leveled, _) = crate::script::skill_up_python(attacker, &skill);
        if leveled {
            crate::script::combat_commands::queue_combat_presentation_event(
                attacker,
                serde_json::json!({ "kind": "skill_level_up" }),
            );
        }
        attacker.stop_skill();
        attacker.temp_mut().remove("_skill_turn");
    } else {
        attacker.temp_mut().insert(
            "_skill_turn".to_string(),
            crate::object::Value::Int(i64::from(runtime.end + 1)),
        );
    }
    (total_damage, !more || has_wait)
}

fn pvp_strike(attacker: &mut Body, defender: &mut Body) -> i64 {
    attacker.dex += (attacker.get_dex() + 700).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let (mut total_damage, run_normal) = pvp_skill_phase(attacker, defender);
    if !run_normal {
        return total_damage;
    }
    if attacker.dex < 700 {
        return total_damage;
    }
    attacker.dex %= 700;
    let attacker_name = attacker.get_name();
    let defender_name = defender.get_name();
    let weapon = attacker.get_weapon_name();
    let weapon_type = attacker.get_fight_script_type();
    let chance = 100.0
        - ((defender.get_int("레벨") - attacker.get_int("레벨") + 90).div_euclid(3)) as f64
        + attacker.get_hit() as f64 * 0.2
        - defender.get_miss() as f64 * 0.2;
    if chance < rand::random::<u8>() as f64 * (100.0 / u8::MAX as f64) {
        crate::script::combat_commands::queue_combat_presentation_event(
            attacker,
            serde_json::json!({
                "kind": "pvp_miss_out", "target": defender_name,
                "weapon": weapon, "weapon_type": weapon_type,
            }),
        );
        crate::script::combat_commands::queue_combat_presentation_event(
            defender,
            serde_json::json!({
                "kind": "pvp_miss_in", "attacker": attacker_name,
                "weapon": weapon, "weapon_type": weapon_type,
            }),
        );
        let mut training = crate::combat::CombatRound::new();
        crate::combat::processor::apply_player_attack_training(attacker, false, &mut training);
        for event in training.presentation_events {
            crate::script::combat_commands::queue_combat_presentation_event(attacker, event);
        }
        return total_damage;
    }
    let damage = crate::combat::calculate_pvp_damage(attacker, defender);
    total_damage = total_damage.saturating_add(damage);
    crate::script::combat_commands::queue_combat_presentation_event(
        attacker,
        serde_json::json!({
            "kind": "pvp_attack_out", "target": defender_name,
            "damage": damage, "weapon": weapon, "weapon_type": weapon_type,
        }),
    );
    crate::script::combat_commands::queue_combat_presentation_event(
        defender,
        serde_json::json!({
            "kind": "pvp_attack_in", "attacker": attacker_name,
            "damage": damage, "weapon": weapon, "weapon_type": weapon_type,
        }),
    );
    let mut training = crate::combat::CombatRound::new();
    crate::combat::processor::apply_player_attack_training(attacker, true, &mut training);
    for event in training.presentation_events {
        crate::script::combat_commands::queue_combat_presentation_event(attacker, event);
    }
    total_damage
}

/// Resolve one PvP action against the opponent's tick-start state.
///
/// `pvp_strike` also queues the messages received by the defender.  Run it on
/// a snapshot and copy only those presentation events back to the live body;
/// combat state changed by the first resolved action must not influence the
/// opponent's action in the same tick.
fn pvp_strike_against_snapshot(
    attacker: &mut Body,
    defender: &mut Body,
    defender_at_tick_start: &Body,
) -> i64 {
    let mut view = defender_at_tick_start.clone();
    view.temp_mut()
        .remove(crate::script::combat_commands::COMBAT_PRESENTATION_EVENTS);
    let damage = pvp_strike(attacker, &mut view);
    let events = view
        .temp_mut()
        .remove(crate::script::combat_commands::COMBAT_PRESENTATION_EVENTS)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|serialized| serde_json::from_str::<Vec<serde_json::Value>>(&serialized).ok())
        .unwrap_or_default();
    for event in events {
        crate::script::combat_commands::queue_combat_presentation_event(defender, event);
    }
    damage
}

fn apply_simultaneous_pvp_damage(
    first: &mut Body,
    second: &mut Body,
    damage_to_first: i64,
    damage_to_second: i64,
) -> (bool, bool) {
    let first_died = damage_to_first >= first.get_hp();
    let second_died = damage_to_second >= second.get_hp();
    if damage_to_first > 0 {
        first.minus_hp(damage_to_first);
    }
    if damage_to_second > 0 {
        second.minus_hp(damage_to_second);
    }
    if first_died {
        finish_pvp_death(first);
        crate::script::combat_commands::queue_combat_presentation_event(
            first,
            serde_json::json!({ "kind": "player_death" }),
        );
    }
    if second_died {
        finish_pvp_death(second);
        crate::script::combat_commands::queue_combat_presentation_event(
            second,
            serde_json::json!({ "kind": "player_death" }),
        );
    }
    if first_died || second_died {
        crate::script::combat_commands::clear_pvp_target(first);
        crate::script::combat_commands::clear_pvp_target(second);
    }
    (first_died, second_died)
}

fn process_pvp_ticks(
    clients: &mut std::collections::HashMap<SocketAddr, Client>,
    render: &mut Vec<SocketAddr>,
    newly_dead: &mut Vec<SocketAddr>,
) {
    let names = clients
        .iter()
        .filter_map(|(addr, client)| {
            let player = client.player.as_ref()?;
            (client.state == ClientState::Active && !client.disconnect_requested)
                .then(|| (player.body.get_name(), *addr))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut pairs = Vec::new();
    let mut orphaned = Vec::new();
    for (name, addr) in &names {
        let Some(target) = clients
            .get(addr)
            .and_then(|client| client.player.as_ref())
            .and_then(|player| crate::script::combat_commands::pvp_target(&player.body))
        else {
            continue;
        };
        let Some(target_addr) = names.get(&target).copied() else {
            orphaned.push(*addr);
            continue;
        };
        let reciprocal = clients
            .get(&target_addr)
            .and_then(|client| client.player.as_ref())
            .and_then(|player| crate::script::combat_commands::pvp_target(&player.body));
        if reciprocal.as_deref() == Some(name) {
            if addr < &target_addr {
                pairs.push((*addr, target_addr));
            }
        } else {
            orphaned.push(*addr);
        }
    }

    for addr in orphaned {
        if let Some(body) = clients
            .get_mut(&addr)
            .and_then(|client| client.player.as_mut())
            .map(|player| &mut player.body)
        {
            crate::script::combat_commands::clear_pvp_target(body);
        }
    }

    for (first_addr, second_addr) in pairs {
        let Some(mut first_client) = clients.remove(&first_addr) else {
            continue;
        };
        let Some(second_client) = clients.get_mut(&second_addr) else {
            clients.insert(first_addr, first_client);
            continue;
        };
        let (Some(first), Some(second)) =
            (first_client.player.as_mut(), second_client.player.as_mut())
        else {
            clients.insert(first_addr, first_client);
            continue;
        };
        let same_room = get_world_state().read().is_ok_and(|world| {
            let first_position = world.get_player_position(&first.body.get_name());
            let second_position = world.get_player_position(&second.body.get_name());
            first_position
                .zip(second_position)
                .is_some_and(|(first, second)| {
                    first.zone == second.zone && first.room == second.room
                })
        });
        if !same_room || first.body.act == ActState::Death || second.body.act == ActState::Death {
            crate::script::combat_commands::clear_pvp_target(&mut first.body);
            crate::script::combat_commands::clear_pvp_target(&mut second.body);
        } else {
            // Both actions are resolved against the same tick-start combat
            // state. Damage is committed only after both sides have acted, so
            // client/hash iteration order can never suppress a lethal return.
            let first_at_tick_start = first.body.clone();
            let second_at_tick_start = second.body.clone();
            let damage_to_second = pvp_strike_against_snapshot(
                &mut first.body,
                &mut second.body,
                &second_at_tick_start,
            );
            let damage_to_first = pvp_strike_against_snapshot(
                &mut second.body,
                &mut first.body,
                &first_at_tick_start,
            );
            apply_simultaneous_pvp_damage(
                &mut first.body,
                &mut second.body,
                damage_to_first,
                damage_to_second,
            );
            // PvP shares the same terminal contract as mob combat: resolve
            // both actions (and simultaneous deaths) first, then leave each
            // surviving player at one final, unterminated input prompt.
            for body in [&mut first.body, &mut second.body] {
                if body.act != ActState::Death {
                    crate::script::combat_commands::queue_combat_presentation_event(
                        body,
                        serde_json::json!({ "kind": "combat_prompt" }),
                    );
                }
            }
        }
        for (addr, player) in [
            (first_addr, first_client.player.as_ref()),
            (second_addr, second_client.player.as_ref()),
        ] {
            if player.is_some_and(|player| {
                player
                    .body
                    .temp()
                    .contains_key(crate::script::combat_commands::COMBAT_PRESENTATION_EVENTS)
            }) && !render.contains(&addr)
            {
                render.push(addr);
            }
            if player.is_some_and(|player| player.body.act == ActState::Death)
                && !newly_dead.contains(&addr)
            {
                newly_dead.push(addr);
            }
        }
        clients.insert(first_addr, first_client);
    }
}

fn render_combat_presentation(
    broadcaster: &Arc<Broadcaster>,
    command_registry: &Arc<CommandRegistry>,
    addr: SocketAddr,
) {
    let (result, break_after_prompt) = {
        let mut clients = broadcaster.clients.lock();
        let actor_name = clients
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            .map(|player| player.body.get_name())
            .unwrap_or_default();
        let room_names = get_world_state()
            .read()
            .ok()
            .and_then(|world| {
                world
                    .get_player_position(&actor_name)
                    .cloned()
                    .map(|position| world.get_players_in_room(&position.zone, &position.room))
            })
            .unwrap_or_default();
        let mut observers = Vec::new();
        let mut online = rhai::Array::new();
        for (client_addr, client) in clients.iter_mut() {
            if client.state != ClientState::Active {
                continue;
            }
            let Some(other) = client.player.as_mut() else {
                continue;
            };
            let mut details = rhai::Map::new();
            details.insert("이름".into(), rhai::Dynamic::from(other.body.get_name()));
            details.insert(
                "show_prompt".into(),
                rhai::Dynamic::from(
                    other.interactive == 1
                        && !crate::script::config_is_enabled(
                            &other.body.get_string("설정상태"),
                            "엘피출력",
                        ),
                ),
            );
            details.insert("현재체력".into(), rhai::Dynamic::from(other.body.get_hp()));
            details.insert(
                "현재최고체력".into(),
                rhai::Dynamic::from(other.body.get_max_hp()),
            );
            details.insert("현재내공".into(), rhai::Dynamic::from(other.body.get_mp()));
            details.insert(
                "현재최고내공".into(),
                rhai::Dynamic::from(other.body.get_max_mp()),
            );
            online.push(rhai::Dynamic::from(details));
            if *client_addr != addr && room_names.contains(&other.body.get_name()) {
                observers.push(CastRoomPlayerRef::new_with_interactive(
                    &mut other.body,
                    other.interactive,
                ));
            }
        }
        set_cast_room_players(observers);
        set_precomputed_all_online(online);
        let break_after_prompt = clients
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            // This boundary follows what can physically be on the terminal,
            // not the config bit.  The ordinary command path may have just
            // written an unterminated prompt, while combat presentation is
            // asynchronous.  Starting it on a fresh line is harmless when a
            // prompt was suppressed and required when one was visible.
            .is_some_and(|player| player.interactive == 1);
        let result = clients
            .get_mut(&addr)
            .and_then(|client| client.player.as_mut())
            .and_then(|player| {
                command_registry
                    .get_internal("combat_tick")
                    .map(|handler| handler(&mut player.body, &[]))
            });
        clear_cast_room_players();
        clear_precomputed_all_online();
        (result, break_after_prompt)
    };
    match result {
        Some(crate::command::CommandResult::Output(output)) if !output.is_empty() => {
            let prefix = if break_after_prompt && !output.starts_with("\r\n") {
                "\r\n"
            } else {
                ""
            };
            let _ = broadcaster.send_to(addr, &format!("{prefix}{output}\r\n"));
        }
        Some(crate::command::CommandResult::OutputAndSendToUsers(output, deliveries)) => {
            if !output.is_empty() {
                let prefix = if break_after_prompt && !output.starts_with("\r\n") {
                    "\r\n"
                } else {
                    ""
                };
                let _ = broadcaster.send_to(addr, &format!("{prefix}{output}\r\n"));
            }
            for (name, message) in deliveries {
                send_collected_user_message(broadcaster, &name, &message);
            }
        }
        _ => {}
    }
}

/// Run the connected-client loop and the existing call-out scheduler on the same one-second
/// cadence as Python's reactor loop.
pub async fn run_game_loop(
    broadcaster: Arc<Broadcaster>,
    config: GameLoopConfig,
    call_out_scheduler: Option<Arc<CallOutScheduler>>,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
    shutdown_notify: Option<Arc<tokio::sync::Notify>>,
) {
    let mut timer = interval(config.tick_interval);
    timer.tick().await; // Tokio intervals fire immediately once; Python does not.

    let mut game_loop = GameLoop::new(config);
    loop {
        timer.tick().await;
        if let Some(scheduler) = &call_out_scheduler {
            let _ = scheduler.process_due();
        }
        game_loop.tick(&broadcaster);
        for addr in game_loop.take_combat_render() {
            render_combat_presentation(&broadcaster, &command_registry, addr);
        }
        for addr in game_loop.take_newly_dead() {
            let result = {
                let mut clients = broadcaster.clients.lock();
                let Some(player) = clients
                    .get_mut(&addr)
                    .and_then(|client| client.player.as_mut())
                else {
                    continue;
                };
                command_registry
                    .get_internal("death")
                    .map(|handler| handler(&mut player.body, &[]))
            };
            match result {
                Some(crate::command::CommandResult::Output(output)) if !output.is_empty() => {
                    let _ = broadcaster.send_to(addr, &(output + "\r\n"));
                }
                Some(crate::command::CommandResult::OutputAndSendToUsers(output, deliveries)) => {
                    if !output.is_empty() {
                        let _ = broadcaster.send_to(addr, &(output + "\r\n"));
                    }
                    for (name, message) in deliveries {
                        if let Some(target) = broadcaster.find_addr_by_player_name(&name) {
                            let _ = broadcaster.send_to(target, &(message + "\r\n"));
                        }
                    }
                }
                _ => {}
            }
        }
        for (addr, command) in game_loop.take_auto_consume() {
            let _ = handle_game_command(
                &broadcaster,
                addr,
                &command,
                command_registry.clone(),
                room_cache.clone(),
                shutdown_notify.clone(),
            )
            .await;
        }
        for addr in game_loop.take_after_fight() {
            let attack = {
                let clients = broadcaster.clients.lock();
                clients
                    .get(&addr)
                    .filter(|client| !client.disconnect_requested)
                    .and_then(|client| client.player.as_ref())
                    .and_then(|player| player.alias.get("공격"))
                    .cloned()
                    .unwrap_or_default()
            };
            if !attack.is_empty() {
                let _ = handle_game_command(
                    &broadcaster,
                    addr,
                    &attack,
                    command_registry.clone(),
                    room_cache.clone(),
                    shutdown_notify.clone(),
                )
                .await;
            }

            let next = {
                let mut clients = broadcaster.clients.lock();
                let Some(client) = clients.get_mut(&addr) else {
                    continue;
                };
                let Some(player) = client.player.as_mut() else {
                    continue;
                };
                if player.body.act == ActState::Fight || !player.body.targets.is_empty() {
                    None
                } else if player.auto_move_list.is_empty() {
                    None
                } else {
                    let command = player.auto_move_list.remove(0);
                    if player.auto_move_list.is_empty() {
                        player.body.temp_mut().insert(
                            "_after_fight_route_finished".to_string(),
                            crate::object::Value::Int(1),
                        );
                    }
                    Some(command)
                }
            };
            if let Some(next) = next {
                let _ = handle_game_command(
                    &broadcaster,
                    addr,
                    &next,
                    command_registry.clone(),
                    room_cache.clone(),
                    shutdown_notify.clone(),
                )
                .await;
            }
        }
    }
}

fn alias_int(value: Option<&String>) -> i64 {
    let Some(value) = value else { return 0 };
    let value = value.trim_start();
    let bytes = value.as_bytes();
    let mut end = usize::from(bytes.first() == Some(&b'-'));
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || (end == 1 && bytes.first() == Some(&b'-')) {
        0
    } else {
        value[..end].parse().unwrap_or(0)
    }
}

fn automatic_consumable_commands(player: &Player) -> Vec<String> {
    if !matches!(player.body.act, ActState::Stand | ActState::Fight) {
        return Vec::new();
    }
    let mut commands = Vec::new();
    let hp_threshold = alias_int(player.alias.get("체력"));
    if hp_threshold != 0
        && player.body.get_hp() < hp_threshold.min(player.body.get_max_hp())
        && player
            .alias
            .get("체력약")
            .is_some_and(|item| !item.is_empty())
    {
        commands.push(format!("{} 먹어", player.alias["체력약"]));
    }
    let mp_threshold = alias_int(player.alias.get("내공"));
    if mp_threshold != 0
        && player.body.get_mp() < mp_threshold.min(player.body.get_max_mp())
        && player
            .alias
            .get("내공약")
            .is_some_and(|item| !item.is_empty())
    {
        commands.push(format!("{} 먹어", player.alias["내공약"]));
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::client::Client;
    use crate::player::STATE_ACTIVE;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::sync::mpsc;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    #[tokio::test]
    async fn combat_renderer_supplies_live_room_prompt_and_routes_raw_wire() {
        let suffix = std::process::id();
        let zone = format!("틱망존-{suffix}");
        let actor_name = format!("틱망공격자-{suffix}");
        let observer_name = format!("틱망관전자-{suffix}");
        let actor_addr = addr(18301);
        let observer_addr = addr(18302);
        let (actor_tx, mut actor_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();
        let make_client = |address, sender, name: &str, hp| {
            let mut client = Client::new(address, sender);
            client.complete_login();
            let mut player = Player::new();
            player.state = STATE_ACTIVE;
            player.interactive = 1;
            player.body.set("이름", name);
            player.body.set("체력", hp);
            player.body.set("최고체력", 45_i64);
            player.body.set("내공", 7_i64);
            player.body.set("최고내공", 9_i64);
            player.body.set("설정상태", "엘피출력 0 타인전투출력거부 0");
            client.player = Some(player);
            client
        };
        let mut actor_client = make_client(actor_addr, actor_tx, &actor_name, 40_i64);
        crate::script::combat_commands::queue_combat_presentation_event(
            &mut actor_client.player.as_mut().unwrap().body,
            serde_json::json!({"kind": "anger_100"}),
        );
        crate::script::combat_commands::queue_combat_presentation_event(
            &mut actor_client.player.as_mut().unwrap().body,
            serde_json::json!({"kind": "combat_prompt"}),
        );
        let broadcaster = Arc::new(Broadcaster::new());
        broadcaster.add_client(actor_client);
        broadcaster.add_client(make_client(
            observer_addr,
            observer_tx,
            &observer_name,
            31_i64,
        ));
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                &actor_name,
                crate::world::PlayerPosition::new(zone.clone(), "1".into()),
            );
            world.set_player_position(
                &observer_name,
                crate::world::PlayerPosition::new(zone, "1".into()),
            );
        }
        let storage = Arc::new(tokio::sync::RwLock::new(
            crate::script::ScriptStorage::default(),
        ));
        let mut registry = CommandRegistry::new();
        crate::command::commands::script::register_script_commands(
            &mut registry,
            storage,
            None,
            None,
            None,
        )
        .await;

        render_combat_presentation(&broadcaster, &Arc::new(registry), actor_addr);
        let mut actor_wire = String::new();
        while let Ok(message) = actor_rx.try_recv() {
            actor_wire.push_str(&message);
        }
        assert!(
            actor_wire.starts_with("\r\n당신이 갑자기"),
            "the next tick must end the previous input prompt first: {actor_wire:?}"
        );
        assert!(
            actor_wire.ends_with("\r\n\x1b[0;37;40m[ 40/45, 7/9 ] "),
            "the replacement prompt must remain the final unterminated wire: {actor_wire:?}"
        );
        let mut wire = String::new();
        while let Ok(message) = observer_rx.try_recv() {
            wire.push_str(&message);
        }
        assert_eq!(
            wire,
            format!(
                "\r\n\x1b[1m{actor_name}\x1b[0;37m 갑자기 \x1b[1;40;31m괴성\x1b[0;40;37m을 지르며 \x1b[1;40;31m난동\x1b[0;40;37m을 부립니다. '끄오오오오오~~'\r\n\r\n\x1b[0;37;40m[ 31/45, 7/9 ] "
            )
        );
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&actor_name);
        world.remove_player_position(&observer_name);
    }

    #[test]
    fn automatic_consumables_follow_python_hp_then_mp_alias_order() {
        let mut player = Player::new();
        player.body.act = ActState::Fight;
        player.body.set("체력", 40_i64);
        player.body.set("최고체력", 100_i64);
        player.body.set("내공", 30_i64);
        player.body.set("최고내공", 100_i64);
        player
            .alias
            .insert("체력".to_string(), "50이하".to_string());
        player
            .alias
            .insert("체력약".to_string(), "금창약".to_string());
        player.alias.insert("내공".to_string(), "-1".to_string());
        player
            .alias
            .insert("내공약".to_string(), "청심단".to_string());

        assert_eq!(
            automatic_consumable_commands(&player),
            vec!["금창약 먹어".to_string()]
        );

        player.alias.insert("내공".to_string(), "60".to_string());
        assert_eq!(
            automatic_consumable_commands(&player),
            vec!["금창약 먹어".to_string(), "청심단 먹어".to_string()]
        );
        player.body.act = ActState::Death;
        assert!(automatic_consumable_commands(&player).is_empty());
    }

    #[test]
    fn damage_recovery_preserves_python_armor_ratio_floor_and_healing() {
        let mut body = Body::new();
        body.set("최고체력", 1_000_i64);
        body.set("체력", 100_i64);
        body.set("맷집", 11_i64);
        let mut effect = crate::player::ActiveSkill::new("회복공".to_string(), 10);
        effect.category = "전투회복".to_string();
        effect.recovery_percent = 50;
        effect.recovery_script = "[공]이 회복합니다".to_string();
        body.active_skills.push(effect);
        let mut round = crate::combat::CombatRound::new();

        apply_python_damage_recovery(&mut body, 10, false, &mut round);

        // murim.json has 40; Python evaluates float(40)//100 first, so the
        // armor threshold is zero and adds 1 + damage experience.
        assert_eq!(body.get_int("맷집경험치"), 11);
        assert_eq!(body.get_hp(), 105);
        assert!(round
            .presentation_events
            .iter()
            .any(|event| { event["kind"] == "combat_recovery" && event["amount"] == 5 }));
    }

    #[test]
    fn heartbeat_applies_python_age_and_nickname_invitation_before_combat() {
        let mut player = Player::new();
        player.body.set("이름", "하트비트검사");
        player.body.set("나이", 59_i64);
        player.body.set("최고내공", 100_i64);
        player.body.set(
            "나이오름틱",
            crate::script::get_murim_config_int("나이오름틱") - 1,
        );
        player.body.tick = 59;
        player.body.set("0 성격플킬", 35_i64);

        update_active_player(&mut player, &GameLoopConfig::default());

        assert_eq!(player.body.get_int("나이"), 60);
        assert_eq!(player.body.get_int("최고내공"), 160);
        assert_eq!(player.body.get_int("나이오름틱"), 0);
        let events =
            crate::script::combat_commands::take_combat_presentation_events(&mut player.body);
        let kinds = events
            .into_iter()
            .map(|event| {
                event.cast::<rhai::Map>()["kind"]
                    .clone()
                    .into_string()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(kinds, vec!["age_up", "nickname_invitation"]);
    }

    #[test]
    fn combat_dex_consumes_every_complete_python_seven_hundred_step() {
        let mut player_dex = 2_150_i32;
        assert_eq!(consume_player_actions(&mut player_dex), 3);
        assert_eq!(player_dex, 50);

        let mut mob_dex = 1_499_i64;
        assert_eq!(consume_mob_actions(&mut mob_dex), 2);
        assert_eq!(mob_dex, 99);
    }

    #[test]
    fn mob_death_reward_uses_python_difficulty_bonus_columns() {
        let mut data = crate::world::RawMobData::new();
        data.name = "난이도보상몹".to_string();
        data.level = 10;
        let mut mob = crate::world::MobInstance::with_difficulty(
            "시험:난이도보상몹".to_string(),
            "시험6".to_string(),
            "1",
            &data,
            6,
        );
        mob.targets.push("기여자".to_string());
        mob.record_player_damage("기여자", 10);
        let mut round = crate::combat::CombatRound::new();
        let mut pending = Vec::new();
        award_mob_death(&mut mob, &data, &mut round, &mut pending);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].difficulty, 6);
        assert_eq!(pending[0].targets, vec!["기여자"]);
        assert_eq!(pending[0].damage_map["기여자"], 10);
    }

    #[test]
    fn corpse_items_use_python_inclusive_roll_and_skip_when_inventory_exists() {
        let mut data = crate::world::RawMobData::new();
        data.drop_items.push(("1".to_string(), 2, 1, 1));
        data.use_items.push(("2".to_string(), 1, 1, 1));
        let mut mob =
            crate::world::MobInstance::new("시험:드롭".to_string(), "시험".to_string(), "1", &data);
        let mut rolls = [1_i64, 2, 1].into_iter();
        generate_mob_corpse_items(&mut mob, &data, &mut |_| rolls.next().unwrap());
        assert_eq!(mob.inventory.len(), 2);
        let first_snapshot = mob
            .inventory
            .iter()
            .map(|item| item.lock().unwrap().getString("인덱스"))
            .collect::<Vec<_>>();
        generate_mob_corpse_items(&mut mob, &data, &mut |_| 0);
        assert_eq!(
            mob.inventory
                .iter()
                .map(|item| item.lock().unwrap().getString("인덱스"))
                .collect::<Vec<_>>(),
            first_snapshot
        );
    }

    #[test]
    fn item_magic_matches_python_option_count_value_and_valuable_flags() {
        let (template, _) = crate::script::object_from_item_json("160-5").unwrap();
        let mut item = template.lock().unwrap().deepclone();
        assert!(crate::script::apply_item_magic_with_roll(
            &mut item,
            10_000,
            1,
            true,
            &mut |min, _| min,
        ));
        assert_eq!(item.getInt("레벨"), 10_000);
        assert!(item.getString("옵션").contains("힘"));
        assert!(item.checkAttr("아이템속성", "버리지못함"));
        assert!(item.checkAttr("아이템속성", "줄수없음"));
        assert!(item.getString("이름").starts_with("\x1b[1;34m"));
    }

    #[test]
    fn migrated_special_drop_catalogue_is_complete_and_loadable() {
        let keys = special_drop_keys();
        assert_eq!(keys.len(), 906);
        assert_eq!(keys.first().map(String::as_str), Some("1"));
        assert_eq!(keys.last().map(String::as_str), Some("황룡마조-5"));
        assert!(keys.iter().all(|key| std::path::Path::new("data/item")
            .join(format!("{key}.json"))
            .is_file()));
    }

    fn active_client(
        port: u16,
        name: &str,
        act: ActState,
    ) -> (Client, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut client = Client::new(addr(port), tx);
        let mut player = Player::new();
        player.state = STATE_ACTIVE;
        player.body.set("이름", name);
        player.body.set("체력", 10_i64);
        player.body.set("최고체력", 100_i64);
        player.body.set("맷집", 0_i64);
        player.body.set("내공", 10_i64);
        player.body.set("최고내공", 100_i64);
        player.body.act = act;
        client.state = ClientState::Active;
        client.set_player(player);
        (client, rx)
    }

    #[test]
    fn pvp_tick_applies_lethal_damage_simultaneously_and_allows_double_death() {
        let first_name = "비무첫째";
        let second_name = "비무둘째";
        let (mut first_client, _first_rx) = active_client(41301, first_name, ActState::Fight);
        let (mut second_client, _second_rx) = active_client(41302, second_name, ActState::Fight);
        let first = first_client.player.as_mut().unwrap();
        first.body.set("힘", 20_i64);
        first.body.set("명중", 10_000_i64);
        first.body.set("최고내공", 100_i64);
        first.body.temp_mut().insert(
            crate::script::combat_commands::PVP_TARGET.to_string(),
            crate::object::Value::String(second_name.to_string()),
        );
        let second = second_client.player.as_mut().unwrap();
        second.body.set("힘", 20_i64);
        second.body.set("명중", 10_000_i64);
        second.body.set("최고내공", 100_i64);
        second.body.temp_mut().insert(
            crate::script::combat_commands::PVP_TARGET.to_string(),
            crate::object::Value::String(first_name.to_string()),
        );
        let zone = format!("비무회귀존-{}", std::process::id());
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                first_name,
                crate::world::PlayerPosition::new(zone.clone(), "1".to_string()),
            );
            world.set_player_position(
                second_name,
                crate::world::PlayerPosition::new(zone, "1".to_string()),
            );
        }
        let mut clients = std::collections::HashMap::from([
            (addr(41301), first_client),
            (addr(41302), second_client),
        ]);
        let mut render = Vec::new();
        let mut dead = Vec::new();
        process_pvp_ticks(&mut clients, &mut render, &mut dead);
        let first = &clients[&addr(41301)].player.as_ref().unwrap().body;
        let second = &clients[&addr(41302)].player.as_ref().unwrap().body;
        assert_eq!(first.act, ActState::Death);
        assert_eq!(second.act, ActState::Death);
        assert!(render.contains(&addr(41301)));
        assert!(render.contains(&addr(41302)));
        assert!(crate::script::combat_commands::pvp_target(first).is_none());
        assert!(crate::script::combat_commands::pvp_target(second).is_none());
        assert_eq!(dead.len(), 2);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(first_name);
        world.remove_player_position(second_name);
    }

    #[test]
    fn pvp_tick_queues_one_final_prompt_for_each_survivor() {
        let first_name = "비무프롬프트첫째";
        let second_name = "비무프롬프트둘째";
        let (mut first_client, _first_rx) = active_client(41305, first_name, ActState::Fight);
        let (mut second_client, _second_rx) = active_client(41306, second_name, ActState::Fight);
        for (client, target) in [
            (&mut first_client, second_name),
            (&mut second_client, first_name),
        ] {
            let body = &mut client.player.as_mut().unwrap().body;
            body.set("체력", 1_000_000_i64);
            body.set("최고체력", 1_000_000_i64);
            body.set("명중", 10_000_i64);
            body.temp_mut().insert(
                crate::script::combat_commands::PVP_TARGET.to_string(),
                crate::object::Value::String(target.to_string()),
            );
        }
        let zone = format!("비무프롬프트존-{}", std::process::id());
        {
            let mut world = get_world_state().write().unwrap();
            for name in [first_name, second_name] {
                world.set_player_position(
                    name,
                    crate::world::PlayerPosition::new(zone.clone(), "1".to_string()),
                );
            }
        }
        let mut clients = std::collections::HashMap::from([
            (addr(41305), first_client),
            (addr(41306), second_client),
        ]);
        let mut render = Vec::new();
        process_pvp_ticks(&mut clients, &mut render, &mut Vec::new());

        for address in [addr(41305), addr(41306)] {
            assert!(render.contains(&address));
            let body = &mut clients
                .get_mut(&address)
                .unwrap()
                .player
                .as_mut()
                .unwrap()
                .body;
            let events = crate::script::combat_commands::take_combat_presentation_events(body);
            let kinds = events
                .into_iter()
                .filter_map(|event| event.try_cast::<rhai::Map>())
                .filter_map(|event| event["kind"].clone().into_string().ok())
                .collect::<Vec<_>>();
            assert_eq!(
                kinds
                    .iter()
                    .filter(|kind| kind.as_str() == "combat_prompt")
                    .count(),
                1
            );
            assert_eq!(kinds.last().map(String::as_str), Some("combat_prompt"));
        }
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(first_name);
        world.remove_player_position(second_name);
    }

    #[test]
    fn pvp_tick_result_is_independent_of_which_player_has_the_lower_connection_address() {
        for (case, first_port, second_port) in [("정방향", 41321, 41322), ("역방향", 41324, 41323)]
        {
            let first_name = format!("비무순서첫째{case}");
            let second_name = format!("비무순서둘째{case}");
            let (mut first_client, _first_rx) =
                active_client(first_port, &first_name, ActState::Fight);
            let (mut second_client, _second_rx) =
                active_client(second_port, &second_name, ActState::Fight);
            for (client, target) in [
                (&mut first_client, second_name.as_str()),
                (&mut second_client, first_name.as_str()),
            ] {
                let body = &mut client.player.as_mut().unwrap().body;
                body.set("힘", 20_i64);
                body.set("명중", 10_000_i64);
                body.set("최고내공", 100_i64);
                body.temp_mut().insert(
                    crate::script::combat_commands::PVP_TARGET.to_string(),
                    crate::object::Value::String(target.to_string()),
                );
            }
            let zone = format!("비무순서존-{case}-{}", std::process::id());
            {
                let mut world = get_world_state().write().unwrap();
                for name in [&first_name, &second_name] {
                    world.set_player_position(
                        name,
                        crate::world::PlayerPosition::new(zone.clone(), "1".to_string()),
                    );
                }
            }
            let mut clients = std::collections::HashMap::from([
                (addr(first_port), first_client),
                (addr(second_port), second_client),
            ]);
            let mut dead = Vec::new();
            process_pvp_ticks(&mut clients, &mut Vec::new(), &mut dead);
            assert_eq!(
                clients[&addr(first_port)].player.as_ref().unwrap().body.act,
                ActState::Death
            );
            assert_eq!(
                clients[&addr(second_port)]
                    .player
                    .as_ref()
                    .unwrap()
                    .body
                    .act,
                ActState::Death
            );
            assert_eq!(dead.len(), 2);
            let mut world = get_world_state().write().unwrap();
            world.remove_player_position(&first_name);
            world.remove_player_position(&second_name);
        }
    }

    #[test]
    fn simultaneous_pvp_commit_has_no_first_actor_survival_advantage() {
        fn fighter(name: &str, target: &str) -> Body {
            let mut body = Body::new();
            body.set("이름", name);
            body.set("체력", 50_i64);
            body.set("최고체력", 50_i64);
            body.act = ActState::Fight;
            body.temp_mut().insert(
                crate::script::combat_commands::PVP_TARGET.to_string(),
                crate::object::Value::String(target.to_string()),
            );
            body
        }
        let mut first = fighter("동시첫째", "동시둘째");
        let mut second = fighter("동시둘째", "동시첫째");
        assert_eq!(
            apply_simultaneous_pvp_damage(&mut first, &mut second, 50, 50),
            (true, true)
        );
        assert_eq!(first.act, ActState::Death);
        assert_eq!(second.act, ActState::Death);

        let mut reversed_first = fighter("동시둘째", "동시첫째");
        let mut reversed_second = fighter("동시첫째", "동시둘째");
        assert_eq!(
            apply_simultaneous_pvp_damage(&mut reversed_first, &mut reversed_second, 50, 50,),
            (true, true)
        );
        assert_eq!(reversed_first.act, ActState::Death);
        assert_eq!(reversed_second.act, ActState::Death);
    }

    #[test]
    fn one_sided_pvp_death_clears_both_targets_and_survivor_fight_state() {
        let mut defeated = Body::new();
        defeated.set("이름", "패자");
        defeated.set("체력", 10_i64);
        defeated.set("최고체력", 10_i64);
        defeated.act = ActState::Fight;
        defeated.temp_mut().insert(
            crate::script::combat_commands::PVP_TARGET.to_string(),
            crate::object::Value::String("승자".to_string()),
        );
        let mut survivor = Body::new();
        survivor.set("이름", "승자");
        survivor.set("체력", 100_i64);
        survivor.set("최고체력", 100_i64);
        survivor.act = ActState::Fight;
        survivor.temp_mut().insert(
            crate::script::combat_commands::PVP_TARGET.to_string(),
            crate::object::Value::String("패자".to_string()),
        );

        assert_eq!(
            apply_simultaneous_pvp_damage(&mut defeated, &mut survivor, 10, 1),
            (true, false)
        );
        assert_eq!(defeated.act, ActState::Death);
        assert_eq!(survivor.act, ActState::Stand);
        assert!(crate::script::combat_commands::pvp_target(&defeated).is_none());
        assert!(crate::script::combat_commands::pvp_target(&survivor).is_none());
    }

    #[test]
    fn pvp_tick_clears_stale_fight_when_target_disconnects_or_leaves_room() {
        let survivor_name = "비무잔류자";
        let missing_name = "비무이탈자";
        let (mut survivor_client, _rx) = active_client(41311, survivor_name, ActState::Fight);
        survivor_client
            .player
            .as_mut()
            .unwrap()
            .body
            .temp_mut()
            .insert(
                crate::script::combat_commands::PVP_TARGET.to_string(),
                crate::object::Value::String(missing_name.to_string()),
            );
        let mut clients = std::collections::HashMap::from([(addr(41311), survivor_client)]);
        process_pvp_ticks(&mut clients, &mut Vec::new(), &mut Vec::new());
        let survivor = &clients[&addr(41311)].player.as_ref().unwrap().body;
        assert_eq!(survivor.act, ActState::Stand);
        assert!(crate::script::combat_commands::pvp_target(survivor).is_none());

        let (mut first_client, _first_rx) = active_client(41312, "다른방첫째", ActState::Fight);
        let (mut second_client, _second_rx) = active_client(41313, "다른방둘째", ActState::Fight);
        for (client, target) in [
            (&mut first_client, "다른방둘째"),
            (&mut second_client, "다른방첫째"),
        ] {
            client.player.as_mut().unwrap().body.temp_mut().insert(
                crate::script::combat_commands::PVP_TARGET.to_string(),
                crate::object::Value::String(target.to_string()),
            );
        }
        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                "다른방첫째",
                crate::world::PlayerPosition::new("비무이탈존".to_string(), "1".to_string()),
            );
            world.set_player_position(
                "다른방둘째",
                crate::world::PlayerPosition::new("비무이탈존".to_string(), "2".to_string()),
            );
        }
        let mut clients = std::collections::HashMap::from([
            (addr(41312), first_client),
            (addr(41313), second_client),
        ]);
        process_pvp_ticks(&mut clients, &mut Vec::new(), &mut Vec::new());
        for address in [addr(41312), addr(41313)] {
            let body = &clients[&address].player.as_ref().unwrap().body;
            assert_eq!(body.act, ActState::Stand);
            assert!(crate::script::combat_commands::pvp_target(body).is_none());
        }
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position("다른방첫째");
        world.remove_player_position("다른방둘째");
    }

    #[test]
    fn pvp_skill_pattern_advances_and_produces_deferred_damage() {
        let mut attacker = Body::new();
        attacker.set("이름", "비무무공공격자");
        attacker.set("힘", 100_i64);
        attacker.set("명중", 10_000_i64);
        attacker.set("최고내공", 1_000_i64);
        attacker.skill_map.insert(
            "강룡십팔장".to_string(),
            crate::player::SkillTraining::new(12, 0),
        );
        attacker.get_skill("강룡십팔장");
        let mut defender = Body::new();
        defender.set("이름", "비무무공방어자");
        defender.set("레벨", 1_i64);
        defender.set("체력", 10_000_i64);
        defender.set("최고체력", 10_000_i64);
        let mut deferred_damage = 0_i64;
        for _ in 0..20 {
            attacker.dex = attacker.dex.saturating_add(1_400);
            let (damage, _) = pvp_skill_phase(&mut attacker, &mut defender);
            deferred_damage = deferred_damage.saturating_add(damage);
            assert_eq!(
                defender.get_hp(),
                10_000,
                "phase calculation must not commit damage"
            );
            if attacker.skill.is_none() {
                break;
            }
        }
        assert!(deferred_damage > 0);
        assert!(
            attacker.skill.is_none(),
            "the PvP skill pattern must finish"
        );
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "muc_game_loop_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn default_timing_matches_python_sources() {
        let config = GameLoopConfig::default();
        assert_eq!(config.tick_interval, Duration::from_secs(1));
        assert_eq!(config.inactive_timeout, 10);
        assert_eq!(config.active_timeout, 180);
        assert_eq!(config.recovery_interval, 30);
        assert_eq!(config.save_interval, 600);
    }

    #[test]
    fn death_step_three_runs_python_funeral_room_entry_presentation() {
        let suffix = std::process::id();
        let dead = format!("사망전환-{suffix}");
        let old_observer = format!("사망구방-{suffix}");
        let new_observer = format!("사망신방-{suffix}");
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("낙양성", "8").unwrap();
            world.room_cache.get_room("낙양성", "7").unwrap();
            world.set_player_position(
                &dead,
                crate::world::PlayerPosition::new("낙양성".to_string(), "8".to_string()),
            );
            world.set_player_position(
                &old_observer,
                crate::world::PlayerPosition::new("낙양성".to_string(), "8".to_string()),
            );
            world.set_player_position(
                &new_observer,
                crate::world::PlayerPosition::new("낙양성".to_string(), "7".to_string()),
            );
        }
        let broadcaster = Broadcaster::new();
        let (mut dead_client, _dead_rx) = active_client(41070, &dead, ActState::Death);
        dead_client.player.as_mut().unwrap().body.set_death_step(3);
        let (old_client, _old_rx) = active_client(41071, &old_observer, ActState::Stand);
        let (new_client, _new_rx) = active_client(41072, &new_observer, ActState::Stand);
        broadcaster.add_client(dead_client);
        broadcaster.add_client(old_client);
        broadcaster.add_client(new_client);

        let mut loop_ = GameLoop::new(GameLoopConfig::default());
        loop_.tick_at(&broadcaster, Instant::now());
        assert_eq!(loop_.take_combat_render(), vec![addr(41070)]);
        let mut clients = broadcaster.clients.lock();
        let dead_body = &mut clients
            .get_mut(&addr(41070))
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .body;
        let events = crate::script::combat_commands::take_combat_presentation_events(dead_body);
        let transition = events
            .iter()
            .filter_map(|event| event.clone().try_cast::<rhai::Map>())
            .find(|event| {
                event["kind"].clone().into_string().ok().as_deref() == Some("death_room_transition")
            })
            .unwrap();
        let old_players = transition["old_players"]
            .clone()
            .try_cast::<rhai::Array>()
            .unwrap();
        let new_players = transition["new_players"]
            .clone()
            .try_cast::<rhai::Array>()
            .unwrap();
        assert_eq!(old_players[1].clone().into_string().unwrap(), old_observer);
        assert_eq!(new_players[0].clone().into_string().unwrap(), new_observer);
        drop(clients);
        let mut world = get_world_state().write().unwrap();
        let position = world.get_player_position(&dead).unwrap();
        assert_eq!(position.zone, "낙양성");
        assert_eq!(position.room, "7");
        for name in [&dead, &old_observer, &new_observer] {
            world.remove_player_position(name);
        }
    }

    #[test]
    fn migrated_schedule_is_anchored_to_the_python_runtime_sources() {
        let loop_source = std::fs::read_to_string("loop.py").unwrap();
        let client_source = std::fs::read_to_string("client.py").unwrap();
        let player_source = std::fs::read_to_string("objs/player.py").unwrap();

        assert!(loop_source.contains("player.state == INACTIVE and t1 - player.idle >= 10"));
        assert!(loop_source.contains("player.state != INACTIVE and t1 - player.idle >= 180"));
        assert!(loop_source.contains("if player.state == ACTIVE:"));
        assert!(client_source.contains("self.player.idle = time.time()"));
        assert!(player_source.contains("if self.tick % 600 == 0:"));
        assert!(player_source.contains("if self.tick % 30 == 0:"));
        assert!(player_source.contains("r = 0.1"));
        assert!(player_source.contains("r = 0.2"));
        assert!(player_source.contains("r = 0.05"));
        let rust_source = std::fs::read_to_string("src/server/game_loop.rs").unwrap();
        let runtime = rust_source
            .split("pub async fn run_game_loop")
            .nth(1)
            .unwrap();
        let combat_render = runtime.find("take_combat_render()").unwrap();
        let death_drop = runtime.find("take_newly_dead()").unwrap();
        assert!(
            combat_render < death_drop,
            "combat/death presentation must preserve attack -> death -> drop order"
        );
    }

    #[test]
    fn ticks_the_player_stored_in_the_real_broadcaster() {
        let broadcaster = Broadcaster::new();
        let (client, _rx) = active_client(41001, "주기시험", ActState::Stand);
        broadcaster.add_client(client);
        let now = Instant::now();
        let mut game_loop = GameLoop::new(GameLoopConfig::default());

        for _ in 0..29 {
            game_loop.tick_at(&broadcaster, now);
        }
        {
            let clients = broadcaster.clients.lock();
            let body = &clients[&addr(41001)].player.as_ref().unwrap().body;
            assert_eq!(body.tick, 29);
            assert_eq!(body.get_hp(), 10);
            assert_eq!(body.get_mp(), 10);
        }

        game_loop.tick_at(&broadcaster, now);
        let clients = broadcaster.clients.lock();
        let body = &clients[&addr(41001)].player.as_ref().unwrap().body;
        assert_eq!(body.tick, 30);
        assert_eq!(body.get_hp(), 20);
        assert_eq!(body.get_mp(), 20);
    }

    #[test]
    fn occupied_room_tick_advances_only_the_represented_mob_respawn_branch() {
        let name = format!("리젠주기시험-{}", std::process::id());
        let zone = format!("리젠주기존-{}", std::process::id());
        let room = "1".to_string();
        let mob_key = format!("{zone}:시험몹");
        let now_seconds = chrono::Utc::now().timestamp();

        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "시험몹".to_string();
            data.zone = zone.clone();
            data.corpse_time = 1;
            data.regen = 60;
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mut mob =
                crate::world::MobInstance::new(mob_key.clone(), zone.clone(), &room, &data);
            mob.kill();
            mob.death_time = now_seconds - 1;
            let item = std::sync::Arc::new(std::sync::Mutex::new(crate::object::Object::new()));
            item.lock().unwrap().set("이름", "시체유품");
            mob.inventory.push(item);
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                &name,
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
        }

        let broadcaster = Broadcaster::new();
        let (client, _rx) = active_client(41051, &name, ActState::Stand);
        broadcaster.add_client(client);
        let mut loop_ = GameLoop::new(GameLoopConfig::default());
        loop_.tick_at(&broadcaster, Instant::now());
        assert_eq!(loop_.take_combat_render(), vec![addr(41051)]);

        {
            let world = get_world_state().read().unwrap();
            let mob = world.mob_cache.get_all_mobs_in_room(&zone, &room)[0];
            assert!(!mob.alive);
            assert_eq!(mob.act, 3, "corpse must advance to Python ACT_REGEN");
            assert!(mob.inventory.is_empty());
            assert_eq!(world.get_room_objs(&zone, &room).len(), 1);
        }
        let mut clients = broadcaster.clients.lock();
        let body = &mut clients
            .get_mut(&addr(41051))
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .body;
        let events = crate::script::combat_commands::take_combat_presentation_events(body);
        assert!(events.into_iter().any(|event| {
            event.try_cast::<rhai::Map>().is_some_and(|event| {
                event["kind"].clone().into_string().ok().as_deref() == Some("room_corpse_update")
            })
        }));
        drop(clients);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&name);
        world.mob_cache.remove_instance(&zone, &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
        world.room_objs.remove(&format!("{zone}:{room}"));
    }

    #[test]
    fn recovery_rates_match_python_action_states() {
        for (offset, act, expected) in [
            (1, ActState::Stand, 20),
            (2, ActState::Rest, 30),
            (3, ActState::Fight, 15),
        ] {
            let broadcaster = Broadcaster::new();
            let (client, _rx) = active_client(41100 + offset, "회복시험", act);
            broadcaster.add_client(client);
            let config = GameLoopConfig {
                recovery_interval: 1,
                ..GameLoopConfig::default()
            };
            let mut game_loop = GameLoop::new(config);
            game_loop.tick_at(&broadcaster, Instant::now());

            let clients = broadcaster.clients.lock();
            let body = &clients[&addr(41100 + offset)].player.as_ref().unwrap().body;
            assert_eq!(body.get_hp(), expected);
            assert_eq!(body.get_mp(), expected);
        }
    }

    #[test]
    fn completed_combat_queues_python_after_fight_reentry() {
        let player_name = format!("전투후경로-{}", std::process::id());
        let zone = format!("전투후경로존-{}", std::process::id());
        let room = "1".to_string();
        let mob_key = format!("{zone}:표적");
        let mob_instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "표적".to_string();
            data.zone = zone.clone();
            data.hp = 1;
            data.max_hp = 1;
            data.arm = 0;
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mob = crate::world::MobInstance::new(mob_key.clone(), zone.clone(), &room, &data);
            mob_instance_id = mob.instance_id;
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                &player_name,
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
        }

        let broadcaster = Broadcaster::new();
        let (mut client, _rx) = active_client(41150, &player_name, ActState::Fight);
        let player = client.player.as_mut().unwrap();
        // Python fightMobNormal runs before the player's strike; keep this
        // after-fight test focused on command re-entry rather than death.
        player.body.set("체력", 10_000_i64);
        player.body.set("최고체력", 10_000_i64);
        player.body.set("레벨", 1_i64);
        player.body.set("힘", 100_i64);
        player.body.set("명중", 100_000_i64);
        player.body.temp_mut().insert(
            "_attack_target_key".to_string(),
            crate::object::Value::String(mob_key.clone()),
        );
        player.body.temp_mut().insert(
            "_combat_target_ids".to_string(),
            crate::object::Value::String(mob_key.clone()),
        );
        player.body.temp_mut().insert(
            "_combat_target_instance_ids".to_string(),
            crate::object::Value::String(mob_instance_id.to_string()),
        );
        player.auto_move_list.push("동".to_string());
        broadcaster.add_client(client);

        let mut game_loop = GameLoop::new(GameLoopConfig::default());
        game_loop.tick_at(&broadcaster, Instant::now());
        assert_eq!(game_loop.take_after_fight(), vec![addr(41150)]);
        {
            let clients = broadcaster.clients.lock();
            let body = &clients[&addr(41150)].player.as_ref().unwrap().body;
            assert_eq!(body.act, ActState::Stand);
            assert!(body.targets.is_empty());
            assert!(
                crate::script::combat_commands::combat_target_ids(body).is_empty(),
                "template combat targets must be cleared after the final target dies"
            );
            assert!(
                crate::script::combat_commands::combat_target_instance_ids(body).is_empty(),
                "instance combat targets must be cleared after the final target dies"
            );
        }

        let mut world = get_world_state().write().unwrap();
        let corpse = world
            .mob_cache
            .get_all_mobs_in_room(&zone, &room)
            .into_iter()
            .find(|mob| mob.mob_key == mob_key)
            .unwrap();
        assert!(!corpse.alive);
        assert_eq!(corpse.act, 2);
        assert!(corpse.death_time > 0);
        world.remove_player_position(&player_name);
        world.mob_cache.remove_instance(&zone, &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn lethal_mob_tick_cancels_python_automatic_route() {
        let player_name = format!("치명경로취소-{}", std::process::id());
        let zone = format!("치명경로존-{}", std::process::id());
        let room = "1".to_string();
        let mob_key = format!("{zone}:처형자");
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "처형자".to_string();
            data.zone = zone.clone();
            data.hp = 100_000;
            data.max_hp = 100_000;
            data.strength = 100_000;
            data.hit = 100_000;
            data.corpse_time = 60;
            data.regen = 60;
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mut mob =
                crate::world::MobInstance::new(mob_key.clone(), zone.clone(), &room, &data);
            mob.targets.push(player_name.clone());
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                &player_name,
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
        }
        let broadcaster = Broadcaster::new();
        let (mut client, _rx) = active_client(41155, &player_name, ActState::Fight);
        let player = client.player.as_mut().unwrap();
        player.body.set("체력", 1_i64);
        player.body.set("최고체력", 100_i64);
        player.body.set("레벨", 1_i64);
        player.body.temp_mut().insert(
            "_attack_target_key".to_string(),
            crate::object::Value::String(mob_key.clone()),
        );
        player.auto_move_list = vec!["동".to_string(), "남".to_string()];
        broadcaster.add_client(client);

        let mut loop_ = GameLoop::new(GameLoopConfig::default());
        loop_.tick_at(&broadcaster, Instant::now());
        let clients = broadcaster.clients.lock();
        let player = clients[&addr(41155)].player.as_ref().unwrap();
        assert_eq!(player.body.act, ActState::Death);
        assert!(player.auto_move_list.is_empty());
        drop(clients);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&player_name);
        world.mob_cache.remove_instance(&zone, &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn mob_death_distributes_rewards_to_every_same_room_damage_contributor() {
        let killer = format!("분배갑-{}", std::process::id());
        let helper = format!("분배을-{}", std::process::id());
        let zone = format!("분배존-{}", std::process::id());
        let room = "1".to_string();
        let mob_key = format!("{zone}:표적");
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "분배표적".to_string();
            data.zone = zone.clone();
            data.hp = 1;
            data.max_hp = 1;
            data.corpse_time = 60;
            data.regen = 60;
            data.strength = 0;
            data.events.insert(
                "이벤트 $%소멸이벤트%".to_string(),
                crate::world::mob::EventScript::Legacy(vec![
                    "소멸완료".to_string(),
                    "$엔터$ 계속".to_string(),
                ]),
            );
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            let mut mob =
                crate::world::MobInstance::new(mob_key.clone(), zone.clone(), &room, &data);
            mob.targets = vec![killer.clone(), helper.clone()];
            mob.record_player_damage(&helper, 1);
            mob.inventory
                .push(crate::script::object_from_item_json("1").unwrap().0);
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                &killer,
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
            world.set_player_position(
                &helper,
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
        }
        let broadcaster = Broadcaster::new();
        let (mut killer_client, _killer_rx) = active_client(41160, &killer, ActState::Fight);
        let killer_player = killer_client.player.as_mut().unwrap();
        killer_player.body.set("체력", 10_000_i64);
        killer_player.body.set("최고체력", 10_000_i64);
        killer_player.body.set("레벨", 1_i64);
        killer_player.body.set("힘", 100_i64);
        killer_player.body.set("명중", 100_000_i64);
        killer_player.body.set("설정상태", "자동습득 1");
        killer_player.body.temp_mut().insert(
            "_attack_target_key".to_string(),
            crate::object::Value::String(mob_key.clone()),
        );
        let (mut helper_client, mut helper_rx) = active_client(41161, &helper, ActState::Stand);
        helper_client
            .player
            .as_mut()
            .unwrap()
            .body
            .set("레벨", 1_i64);
        broadcaster.add_client(killer_client);
        broadcaster.add_client(helper_client);

        let mut loop_ = GameLoop::new(GameLoopConfig::default());
        loop_.tick_at(&broadcaster, Instant::now());
        assert!(helper_rx.try_recv().unwrap().contains("소멸완료"));
        let clients = broadcaster.clients.lock();
        assert!(matches!(
            clients
                .values()
                .find(|client| client
                    .player
                    .as_ref()
                    .is_some_and(|player| player.body.get_name() == helper))
                .and_then(|client| client.pending_input.as_ref()),
            Some(PendingInput::EventEnter { .. })
        ));
        for name in [&killer, &helper] {
            let body = &clients
                .values()
                .find_map(|client| {
                    client
                        .player
                        .as_ref()
                        .filter(|player| player.body.get_name() == *name)
                })
                .unwrap()
                .body;
            assert!(body.get_int("은전") > 0, "{name} did not receive gold");
            assert!(body
                .temp()
                .contains_key(crate::script::combat_commands::COMBAT_PRESENTATION_EVENTS));
        }
        let killer_body = &clients
            .values()
            .find_map(|client| {
                client
                    .player
                    .as_ref()
                    .filter(|player| player.body.get_name() == killer)
            })
            .unwrap()
            .body;
        assert_eq!(killer_body.get_item_count(), 1);
        drop(clients);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&killer);
        world.remove_player_position(&helper);
        world.mob_cache.remove_instance(&zone, &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn admin_mob_death_uses_the_normal_reward_and_auto_loot_tick() {
        let contributor = format!("관리죽임기여자-{}", std::process::id());
        let zone = format!("관리죽임보상존-{}", std::process::id());
        let room = "1".to_string();
        let mob_key = format!("{zone}:보상표적");
        let mut data = crate::world::RawMobData::new();
        data.name = "관리죽임보상표적".into();
        data.zone = zone.clone();
        data.level = 10;
        data.gold = 20;
        data.hp = 100;
        data.max_hp = 100;
        let mut mob = crate::world::MobInstance::new(mob_key.clone(), zone.clone(), &room, &data);
        mob.targets.push(contributor.clone());
        mob.record_player_damage(&contributor, 50);
        mob.inventory
            .push(crate::script::object_from_item_json("1").unwrap().0);
        let instance_id = mob.instance_id;
        {
            let mut world = get_world_state().write().unwrap();
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world.mob_cache.add_mob_instance(mob);
            world.set_player_position(
                &contributor,
                crate::world::PlayerPosition::new(zone.clone(), room.clone()),
            );
            let mobs = world
                .mob_cache
                .get_all_mobs_in_room_mut(&zone, &room)
                .unwrap();
            let selected = mobs
                .iter_mut()
                .find(|mob| mob.instance_id == instance_id)
                .unwrap();
            assert!(queue_admin_mob_death(selected, &data));
            assert!(!selected.alive);
        }

        let broadcaster = Broadcaster::new();
        let (mut client, _rx) = active_client(41169, &contributor, ActState::Stand);
        let player = client.player.as_mut().unwrap();
        player.body.set("레벨", 1_i64);
        player.body.set("은전", 0_i64);
        player.body.set("힘", 100_i64);
        player.body.set("설정상태", "자동습득 1");
        broadcaster.add_client(client);
        let mut loop_ = GameLoop::new(GameLoopConfig::default());
        loop_.tick_at(&broadcaster, Instant::now());
        let clients = broadcaster.clients.lock();
        let body = &clients[&addr(41169)].player.as_ref().unwrap().body;
        assert!(body.get_int("은전") > 0);
        assert_eq!(body.object.inv_stack.get("1"), Some(&1));
        assert!(body.object.objs.is_empty());
        drop(clients);
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&contributor);
        world.mob_cache.remove_instance(&zone, &room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn active_auto_skill_does_not_recharge_cost_each_heartbeat() {
        crate::world::skill::reload_skill_cache().unwrap();
        let name = format!("무공비용주기-{}", std::process::id());
        let zone = format!("무공비용주기존-{}", std::process::id());
        let room = "1";
        let mob_key = format!("{zone}:표적");
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "표적".to_string();
            data.zone = zone.clone();
            data.hp = 100_000;
            data.max_hp = 100_000;
            data.arm = 100_000;
            data.strength = 0;
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world
                .mob_cache
                .add_mob_instance(crate::world::MobInstance::new(
                    mob_key.clone(),
                    zone.clone(),
                    room,
                    &data,
                ));
            world.set_player_position(
                &name,
                crate::world::PlayerPosition::new(zone.clone(), room.to_string()),
            );
        }
        let mut player = Player::new();
        player.body.set("이름", name.clone());
        player.body.set("레벨", 1_i64);
        player.body.set("체력", 10_000_i64);
        player.body.set("최고체력", 10_000_i64);
        player.body.set("내공", 284_i64);
        player.body.set("최고내공", 500_i64);
        player.body.set("설정상태", "자동무공시전 1");
        player.body.set("자동무공", "가의신공");
        player.body.set_skill_training("가의신공", 11, 0);
        player.body.skill = Some("가의신공".to_string());
        player.body.act = ActState::Fight;
        player
            .body
            .temp_mut()
            .insert("_skill_turn".to_string(), crate::object::Value::Int(1));
        player.body.temp_mut().insert(
            "_attack_target_key".to_string(),
            crate::object::Value::String(mob_key.clone()),
        );

        let mut pending_rewards = Vec::new();
        process_combat_tick(&mut player, &mut pending_rewards);
        process_combat_tick(&mut player, &mut pending_rewards);
        assert_eq!(player.body.get_mp(), 284);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&name);
        world.mob_cache.remove_instance(&zone, room, &mob_key);
        world.mob_cache.remove_mob(&mob_key);
    }

    #[test]
    fn mob_attack_skill_selection_matches_python_threshold_probability_and_mp() {
        crate::world::skill::reload_skill_cache().unwrap();
        let mut data = crate::world::RawMobData::new();
        data.name = "무공몹".to_string();
        data.hp = 1_000;
        data.max_hp = 1_000;
        data.inner_power = 500;
        data.skills.push(("가의신공".to_string(), 50, 70));
        let mut mob = crate::world::MobInstance::new(
            "시험:무공몹".to_string(),
            "시험".to_string(),
            "1",
            &data,
        );

        assert!(!try_start_mob_attack_skill(&mut mob, &data, 0, 0));
        mob.hp = 500;
        assert!(!try_start_mob_attack_skill(&mut mob, &data, 0, 71));
        assert!(try_start_mob_attack_skill(&mut mob, &data, 0, 70));
        assert_eq!(mob.mp, 260); // 가의신공 내공소모 240
        assert_eq!(
            mob.active_attack_skill
                .as_ref()
                .map(|skill| skill.name.as_str()),
            Some("가의신공")
        );
    }

    #[test]
    fn runtime_taught_mob_skill_becomes_a_safe_combat_candidate() {
        crate::world::skill::reload_skill_cache().unwrap();
        let mut data = crate::world::RawMobData::new();
        data.name = "전수무공몹".to_string();
        data.hp = 1_000;
        data.max_hp = 1_000;
        data.inner_power = 1_000;
        let mut mob = crate::world::MobInstance::new(
            "시험:전수무공몹".to_string(),
            "시험".to_string(),
            "1",
            &data,
        );
        mob.learned_skills.push("강룡십팔장".to_string());
        assert_eq!(
            mob_skill_candidates(&mob, &data),
            vec![("강룡십팔장".to_string(), 100, 100)]
        );
        assert!(try_start_mob_attack_skill(&mut mob, &data, 0, 100));
        assert_eq!(
            mob.active_attack_skill
                .as_ref()
                .map(|skill| skill.name.as_str()),
            Some("강룡십팔장")
        );
        assert_eq!(mob.mp, 700);
    }

    #[test]
    fn mob_defense_skill_applies_once_and_expires_with_modifier_rollback() {
        crate::world::skill::reload_skill_cache().unwrap();
        let mut data = crate::world::RawMobData::new();
        data.name = "방어몹".to_string();
        data.hp = 1_000;
        data.max_hp = 1_000;
        data.inner_power = 500;
        data.skills.push(("금강불괴".to_string(), 100, 100));
        let mut mob = crate::world::MobInstance::new(
            "시험:방어몹".to_string(),
            "시험".to_string(),
            "1",
            &data,
        );
        mob.skill_map.insert(
            "금강불괴".to_string(),
            crate::player::SkillTraining::new(3, 199_999),
        );
        let mut player = Body::new();

        let started_at = chrono::Utc::now().timestamp();
        let script = try_start_mob_defense_skill(&mut mob, &data, &mut player, &mut || 100);
        assert!(script.unwrap().0.contains("금강불괴"));
        assert_eq!(mob.mp, 350);
        assert_eq!(mob.str_modifier, 15);
        assert_eq!(mob.arm_modifier, 100);
        assert_eq!(mob.skill_effects.len(), 1);
        assert!(
            (started_at + 40..=started_at + 41).contains(&mob.skill_effects[0].expires_at),
            "3성 금강불괴는 30 + 5*(3-1)초여야 합니다"
        );
        assert!(try_start_mob_defense_skill(&mut mob, &data, &mut player, &mut || 0).is_none());

        let expiry = mob.skill_effects[0].expires_at;
        expire_mob_skill_effects(&mut mob, expiry + 1);
        assert_eq!(mob.str_modifier, 0);
        assert_eq!(mob.arm_modifier, 0);
        assert!(mob.skill_effects.is_empty());
        assert!(mob.skills.is_empty());
    }

    #[test]
    fn mob_against_skill_preserves_python_runtime_modifier_absorption_bug() {
        crate::world::skill::reload_skill_cache().unwrap();
        let mut data = crate::world::RawMobData::new();
        data.name = "흡수몹".to_string();
        data.hp = 1_000;
        data.max_hp = 1_000;
        data.inner_power = 500;
        data.skills.push(("흡정대법".to_string(), 100, 100));
        let mut mob = crate::world::MobInstance::new(
            "시험:흡수몹".to_string(),
            "시험".to_string(),
            "1",
            &data,
        );
        mob.hp = 900;
        let mut player = Body::new();
        player.set("체력", 1_000_i64);
        player.set("최고체력", 1_000_i64);
        player._hp = 100;

        let (_, absorbed) =
            try_start_mob_defense_skill(&mut mob, &data, &mut player, &mut || 0).unwrap();
        // Python reads ob.hp (the runtime percentage modifier), not current HP:
        // 100 * -5 // 100 * -1 == 5.
        assert_eq!(absorbed, 5);
        assert_eq!(player._hp, 95);
        assert_eq!(mob.hp, 905);
    }

    #[test]
    fn player_against_effect_expires_and_rolls_back_modifiers() {
        let mut body = Body::new();
        let mut effect = crate::player::ActiveSkill::new("감소효과".to_string(), 0);
        effect.mp_bonus = -40;
        effect.max_mp_bonus = -100;
        body._mp = -40;
        body._maxmp = -100;
        body.active_skills.push(effect);

        expire_player_skill_effects(&mut body);
        assert_eq!(body._mp, 0);
        assert_eq!(body._maxmp, 0);
        assert!(body.active_skills.is_empty());
        assert_eq!(body.get_string("방어무공시전"), "");
    }

    #[test]
    fn inactive_and_active_idle_timeouts_use_python_messages() {
        let broadcaster = Broadcaster::new();
        let now = Instant::now();

        let (tx_inactive, mut rx_inactive) = mpsc::unbounded_channel();
        let mut inactive = Client::new(addr(41201), tx_inactive);
        inactive.last_input = now - Duration::from_secs(10);
        broadcaster.add_client(inactive);

        let (mut active, mut rx_active) = active_client(41202, "대기시험", ActState::Stand);
        active.last_input = now - Duration::from_secs(180);
        broadcaster.add_client(active);

        let mut game_loop = GameLoop::new(GameLoopConfig::default());
        game_loop.tick_at(&broadcaster, now);

        assert_eq!(
            rx_inactive.try_recv().unwrap(),
            "\r\n\r\n입력 제한시간을 초과하였습니다.\r\n\r\n"
        );
        assert_eq!(rx_inactive.try_recv().unwrap(), DISCONNECT_SENTINEL);
        assert_eq!(
            rx_active.try_recv().unwrap(),
            "\r\n\r\n3분 동안 입력이 없어 접속을 종료합니다.\r\n\r\n"
        );
        assert_eq!(rx_active.try_recv().unwrap(), DISCONNECT_SENTINEL);
        assert_eq!(
            broadcaster.clients.lock()[&addr(41202)]
                .player
                .as_ref()
                .unwrap()
                .body
                .tick,
            0,
            "Python skips Player.update after requesting an idle disconnect"
        );

        game_loop.tick_at(&broadcaster, now + Duration::from_secs(1));
        assert!(rx_inactive.try_recv().is_err());
        assert!(rx_active.try_recv().is_err());
    }

    #[test]
    fn periodic_save_writes_only_to_the_configured_temp_directory() {
        let root = unique_temp_dir("save");
        let broadcaster = Broadcaster::new();
        let (client, _rx) = active_client(41301, "임시저장시험", ActState::Stand);
        broadcaster.add_client(client);

        let config = GameLoopConfig {
            save_interval: 1,
            recovery_interval: 1,
            user_data_dir: root.clone(),
            ..GameLoopConfig::default()
        };
        let mut game_loop = GameLoop::new(config);
        game_loop.tick_at(&broadcaster, Instant::now());

        let path = root.join("임시저장시험.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // Python saves before the tick-30 recovery step. The live body is 20, persisted is 10.
        assert_eq!(json["사용자오브젝트"]["체력"], 10);
        assert_eq!(
            broadcaster.clients.lock()[&addr(41301)]
                .player
                .as_ref()
                .unwrap()
                .body
                .get_hp(),
            20
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
