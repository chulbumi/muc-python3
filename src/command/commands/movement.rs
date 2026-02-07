//! Movement commands for MUD engine
//!
//! Handles directional movement: 북, 남, 동, 서, 위, 아래
//! 방 표시 형식은 Python(objs/player.viewMapData, objs/room.longExitStr)을 따릅니다.

use crate::command::registry::CommandRegistry;
use crate::command::{CommandFn, CommandResult};
use crate::player::{ActState, Body};
use crate::script::build_room_objs_grouped;
use crate::world::{format_exits_long, format_room_header, get_world_state, Direction as WorldDir};
use std::sync::Arc;

/// Helper to convert movement Direction to world Direction
fn direction_to_world(dir: MovementDirection) -> WorldDir {
    match dir {
        MovementDirection::North => WorldDir::North,
        MovementDirection::South => WorldDir::South,
        MovementDirection::East => WorldDir::East,
        MovementDirection::West => WorldDir::West,
        MovementDirection::Up => WorldDir::Up,
        MovementDirection::Down => WorldDir::Down,
        MovementDirection::NorthWest => WorldDir::NorthWest,
        MovementDirection::NorthEast => WorldDir::NorthEast,
        MovementDirection::SouthWest => WorldDir::SouthWest,
        MovementDirection::SouthEast => WorldDir::SouthEast,
    }
}

/// Movement direction with its aliases and Korean name
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementDirection {
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

impl MovementDirection {
    /// Returns the Korean name of this direction
    pub fn korean_name(&self) -> &str {
        match self {
            MovementDirection::North => "북",
            MovementDirection::South => "남",
            MovementDirection::East => "동",
            MovementDirection::West => "서",
            MovementDirection::Up => "위",
            MovementDirection::Down => "아래",
            MovementDirection::NorthWest => "북서",
            MovementDirection::NorthEast => "북동",
            MovementDirection::SouthWest => "남서",
            MovementDirection::SouthEast => "남동",
        }
    }

    /// Returns the full direction description
    pub fn description(&self) -> &str {
        match self {
            MovementDirection::North => "북쪽",
            MovementDirection::South => "남쪽",
            MovementDirection::East => "동쪽",
            MovementDirection::West => "서쪽",
            MovementDirection::Up => "위로",
            MovementDirection::Down => "아래로",
            MovementDirection::NorthWest => "북서쪽",
            MovementDirection::NorthEast => "북동쪽",
            MovementDirection::SouthWest => "남서쪽",
            MovementDirection::SouthEast => "남동쪽",
        }
    }

    /// Returns the opposite direction (북서↔남동, 북동↔남서)
    pub fn opposite(&self) -> MovementDirection {
        match self {
            MovementDirection::North => MovementDirection::South,
            MovementDirection::South => MovementDirection::North,
            MovementDirection::East => MovementDirection::West,
            MovementDirection::West => MovementDirection::East,
            MovementDirection::Up => MovementDirection::Down,
            MovementDirection::Down => MovementDirection::Up,
            MovementDirection::NorthWest => MovementDirection::SouthEast,
            MovementDirection::NorthEast => MovementDirection::SouthWest,
            MovementDirection::SouthWest => MovementDirection::NorthEast,
            MovementDirection::SouthEast => MovementDirection::NorthWest,
        }
    }
}

/// Display room information to player (이동 시). 파이썬 viewMapData 레이아웃·ANSI 적용. room_id는 "1" 또는 사용자맵 "이름".
fn display_room(player: &mut Body, zone: &str, room_id: &str) -> CommandResult {
    let world = get_world_state().read().unwrap();

    let room_arc = world.room_cache.get_room_cached(zone, room_id);

    let (header, desc_lines, exits_str, mob_str) = if let Some(room) = room_arc {
        let room_read = room.read().unwrap();
        let header = format_room_header(&room_read.display_name);
        let desc_lines = room_read.description.join("\r\n");
        let exits_str = format_exits_long(&*room_read);

        let mobs = world.mob_cache.get_mobs_in_room(zone, room_id);
        let mob_str = if mobs.is_empty() {
            String::new()
        } else {
            let mut msgs = Vec::new();
            for mob in mobs {
                if let Some(m) = world.mob_cache.get_mob(&mob.mob_key) {
                    if !m.desc1.is_empty() {
                        msgs.push(m.desc1.clone());
                    }
                }
            }
            if msgs.is_empty() {
                String::new()
            } else {
                format!("\r\n{}", msgs.join("\r\n"))
            }
        };

        (header, desc_lines, exits_str, mob_str)
    } else {
        (
            format!("[{}:{}]", zone, room_id),
            "방을 찾을 수 없습니다.".to_string(),
            String::new(),
            String::new(),
        )
    };

    // 바닥에 떨어진 아이템(room_objs + room_inv_stack). 봐/build_room_lines·show_room_to_player와 동일.
    let room_objs = world.get_room_objs(zone, room_id);
    let room_stack = world.get_room_objs_stack(zone, room_id);
    let item_str = build_room_objs_grouped(&room_objs, &room_stack);

    let hp = player.get_hp();
    let max_hp = player.get_max_hp();
    let mp = player.get_mp();
    let max_mp = player.get_max_mp();
    let hpmp = format!("\r\n[ {}/{} , {}/{} ]", hp, max_hp, mp, max_mp);

    // 파이썬 viewMapData: 헤더 → 빈줄 → 설명 → 출구 → (박스) → 몹/아이템 → (다른 플레이어) → 프롬프트 전 [ hp , mp ]
    let out = format!(
        "\r\n{}\r\n\r\n{}\r\n\r\n{}\r\n{}{}{}",
        header, desc_lines, exits_str, mob_str, item_str, hpmp
    );

    CommandResult::Output(out)
}

/// 명령에 없는 경우, 현재 방의 출구 이름(고유명 "초보수련장", "출구" 또는 방향 "북" 등)으로 이동 시도.
/// 전투 중·플레이어명 없음·해당 출구 없음이면 None. 성공 시 Some(CommandResult::Output(방 설명)).
pub fn try_move_by_exit_name(player: &mut Body, exit_name: &str) -> Option<CommandResult> {
    if player.act == ActState::Fight {
        return None;
    }
    let name = player.get_name();
    if name.is_empty() {
        return None;
    }
    let mut world = get_world_state().write().unwrap();
    match world.move_player_by_name(&name, exit_name) {
        Ok((new_zone, new_room, _)) => {
            drop(world);
            Some(display_room(player, &new_zone, &new_room))
        }
        Err(_) => None,
    }
}

/// 귀환: 귀환지(귀환지맵 or 낙양성:42)로 이동. 파이썬 cmds/귀환.py
/// 귀환금지 방·전투/휴식 중 불가, isMovable=!(Fight|Rest)
fn return_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    use crate::world::PlayerPosition;

    if player.act == ActState::Fight {
        return CommandResult::Error("전투 중에는 이동 할 수 없습니다.".to_string());
    }
    if player.act == ActState::Rest {
        return CommandResult::Error("☞ 지금은 귀환할 상황이 아니에요. ^^".to_string());
    }

    let player_name = player.get_name();
    if player_name.is_empty() {
        return CommandResult::Error("플레이어 이름이 없습니다.".to_string());
    }

    let mut world = get_world_state().write().unwrap();
    let (cur_zone, cur_room) = match world.get_player_position(&player_name) {
        Some(p) => (p.zone.clone(), p.room.clone()),
        None => {
            world.set_player_position(&player_name, PlayerPosition::start());
            let p = world.get_player_position(&player_name).unwrap();
            (p.zone.clone(), p.room.clone())
        }
    };

    // 귀환금지: 현재 방 맵속성에 있으면 불가
    if let Ok(room_arc) = world.room_cache.get_room(&cur_zone, &cur_room) {
        if let Ok(r) = room_arc.read() {
            if r.properties.iter().any(|p| p == "귀환금지") {
                return CommandResult::Error("☞ 이곳에선 귀환하실 수 없어요. ^^".to_string());
            }
        }
    }

    let dest = player.get_string("귀환지맵");
    let (dest_zone, dest_room) = if dest.is_empty() {
        ("낙양성".to_string(), "42".to_string())
    } else {
        match dest.split_once(':') {
            Some((z, r)) => (z.to_string(), r.trim().to_string()),
            None => ("낙양성".to_string(), "42".to_string()),
        }
    };

    if world.room_cache.get_room(&dest_zone, &dest_room).is_err() {
        return CommandResult::Error("귀환지맵이 없습니다. 관리자에게 연락하세요.".to_string());
    }
    if dest_zone == cur_zone && dest_room == cur_room {
        return CommandResult::Error("☞ 같은 자리에요. ^^".to_string());
    }

    world.set_player_position(
        &player_name,
        PlayerPosition::new(dest_zone.clone(), dest_room.clone()),
    );
    world.spawn_mobs_for_room(&dest_zone, &dest_room);
    drop(world);
    display_room(player, &dest_zone, &dest_room)
}

/// Creates a movement command for a specific direction
fn move_command(direction: MovementDirection) -> CommandFn {
    Arc::new(move |player: &mut Body, _args: &[&str]| {
        // Check if player is in combat
        if player.act == ActState::Fight {
            return CommandResult::Error("전투 중에는 이동 할 수 없습니다.".to_string());
        }

        let player_name = player.get_name();
        if player_name.is_empty() {
            return CommandResult::Error("플레이어 이름이 없습니다.".to_string());
        }

        let world_dir = direction_to_world(direction);
        let mut world = get_world_state().write().unwrap();

        // Check if player is in world
        if world.get_player_position(&player_name).is_none() {
            // Initialize player at starting position
            world.set_player_position(&player_name, crate::world::PlayerPosition::start());
        }

        // Try to move player
        match world.move_player(&player_name, world_dir) {
            Ok((new_zone, new_room)) => {
                // Spawn mobs for the new room
                world.spawn_mobs_for_room(&new_zone, &new_room);

                // Display the new room
                drop(world); // Release lock before displaying
                display_room(player, &new_zone, &new_room)
            }
            Err(e) => CommandResult::Error(e),
        }
    })
}

/// Registers all movement commands
pub fn register_movement_commands(registry: &mut CommandRegistry) {
    // North movement
    registry.register(crate::command::registry::CommandInfo {
        name: "북".to_string(),
        aliases: vec!["n".to_string(), "north".to_string(), "ㅂ".to_string()],
        handler: move_command(MovementDirection::North),
        level: 0,
        description: "북쪽으로 이동합니다.".to_string(),
        usage: "북".to_string(),
    });

    // South movement
    registry.register(crate::command::registry::CommandInfo {
        name: "남".to_string(),
        aliases: vec!["s".to_string(), "south".to_string(), "ㄴ".to_string()],
        handler: move_command(MovementDirection::South),
        level: 0,
        description: "남쪽으로 이동합니다.".to_string(),
        usage: "남".to_string(),
    });

    // East movement
    registry.register(crate::command::registry::CommandInfo {
        name: "동".to_string(),
        aliases: vec!["e".to_string(), "east".to_string(), "ㄷ".to_string()],
        handler: move_command(MovementDirection::East),
        level: 0,
        description: "동쪽으로 이동합니다.".to_string(),
        usage: "동".to_string(),
    });

    // West movement
    registry.register(crate::command::registry::CommandInfo {
        name: "서".to_string(),
        aliases: vec!["w".to_string(), "west".to_string(), "ㅅ".to_string()],
        handler: move_command(MovementDirection::West),
        level: 0,
        description: "서쪽으로 이동합니다.".to_string(),
        usage: "서".to_string(),
    });

    // Up movement
    registry.register(crate::command::registry::CommandInfo {
        name: "위".to_string(),
        aliases: vec!["u".to_string(), "up".to_string(), "ㅇ".to_string()],
        handler: move_command(MovementDirection::Up),
        level: 0,
        description: "위로 이동합니다.".to_string(),
        usage: "위".to_string(),
    });

    // Down movement
    registry.register(crate::command::registry::CommandInfo {
        name: "아래".to_string(),
        aliases: vec!["d".to_string(), "down".to_string(), "ㅁ".to_string()],
        handler: move_command(MovementDirection::Down),
        level: 0,
        description: "아래로 이동합니다.".to_string(),
        usage: "아래".to_string(),
    });

    // Diagonal: 북서, 북동, 남서, 남동
    registry.register(crate::command::registry::CommandInfo {
        name: "북서".to_string(),
        aliases: vec!["nw".to_string()],
        handler: move_command(MovementDirection::NorthWest),
        level: 0,
        description: "북서쪽으로 이동합니다.".to_string(),
        usage: "북서".to_string(),
    });
    registry.register(crate::command::registry::CommandInfo {
        name: "북동".to_string(),
        aliases: vec!["ne".to_string()],
        handler: move_command(MovementDirection::NorthEast),
        level: 0,
        description: "북동쪽으로 이동합니다.".to_string(),
        usage: "북동".to_string(),
    });
    registry.register(crate::command::registry::CommandInfo {
        name: "남서".to_string(),
        aliases: vec!["sw".to_string()],
        handler: move_command(MovementDirection::SouthWest),
        level: 0,
        description: "남서쪽으로 이동합니다.".to_string(),
        usage: "남서".to_string(),
    });
    registry.register(crate::command::registry::CommandInfo {
        name: "남동".to_string(),
        aliases: vec!["se".to_string()],
        handler: move_command(MovementDirection::SouthEast),
        level: 0,
        description: "남동쪽으로 이동합니다.".to_string(),
        usage: "남동".to_string(),
    });

    // 귀환 (파이썬 cmds/귀환.py): 귀환지맵 or 낙양성:42, 귀환금지·isMovable 검사
    registry.register(crate::command::registry::CommandInfo {
        name: "귀환".to_string(),
        aliases: vec!["귀".to_string(), "귀가".to_string()],
        handler: Arc::new(move |p, a| return_command(p, a)),
        level: 0,
        description: "귀환지로 돌아갑니다.".to_string(),
        usage: "귀환".to_string(),
    });

    // 봐/보/look/바라보기: 봐.rhai 스크립트로 처리. built_in_aliases에 보→봐, look→봐, 바라보기→봐.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_korean_name() {
        assert_eq!(MovementDirection::North.korean_name(), "북");
        assert_eq!(MovementDirection::South.korean_name(), "남");
        assert_eq!(MovementDirection::East.korean_name(), "동");
        assert_eq!(MovementDirection::West.korean_name(), "서");
        assert_eq!(MovementDirection::Up.korean_name(), "위");
        assert_eq!(MovementDirection::Down.korean_name(), "아래");
        assert_eq!(MovementDirection::NorthWest.korean_name(), "북서");
        assert_eq!(MovementDirection::NorthEast.korean_name(), "북동");
        assert_eq!(MovementDirection::SouthWest.korean_name(), "남서");
        assert_eq!(MovementDirection::SouthEast.korean_name(), "남동");
    }

    #[test]
    fn test_direction_opposite() {
        assert_eq!(
            MovementDirection::North.opposite(),
            MovementDirection::South
        );
        assert_eq!(
            MovementDirection::South.opposite(),
            MovementDirection::North
        );
        assert_eq!(MovementDirection::East.opposite(), MovementDirection::West);
        assert_eq!(MovementDirection::Up.opposite(), MovementDirection::Down);
        assert_eq!(
            MovementDirection::NorthWest.opposite(),
            MovementDirection::SouthEast
        );
        assert_eq!(
            MovementDirection::NorthEast.opposite(),
            MovementDirection::SouthWest
        );
    }
}
