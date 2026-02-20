//! Admin commands for MUD development and testing
//!
//! Includes: 아이템생성 (spawn item), etc.

use crate::command::{registry::CommandRegistry, CommandResult};
use crate::hangul;
use crate::object::Object;
use crate::player::Body;
use std::sync::{Arc, Mutex};

/// Creates a test item in player's inventory
fn spawn_item_command(player: &mut Body, args: &[&str]) -> CommandResult {
    // Check admin permission
    let admin_level = player.get_int("관리자등급");
    if admin_level < 1000 {
        return CommandResult::Error("☞ 관리자 권한이 필요합니다.".to_string());
    }

    if args.is_empty() {
        return CommandResult::Usage("아이템생성 [이름] [타입] [공격력] [방어력]".to_string());
    }

    let name = args[0];
    let item_type = if args.len() > 1 { args[1] } else { "무기" };
    let attack = if args.len() > 2 {
        args[2].parse::<i32>().unwrap_or(10)
    } else {
        10
    };
    let defense = if args.len() > 3 {
        args[3].parse::<i32>().unwrap_or(5)
    } else {
        5
    };

    // Create the item
    let item = Arc::new(Mutex::new(Object::new()));
    {
        let mut i = item.lock().unwrap();
        i.set("이름", name);
        i.set("타입", item_type);
        i.set("공격력", attack);
        i.set("방어력", defense);
        i.set("무게", 1);
        i.set("inUse", 0); // Not equipped
    }

    player.object.objs.push(item);

    let particle = hangul::han_obj(name);
    CommandResult::Output(format!("{} {} 생성했습니다.", name, particle))
}

/// Lists all admin commands
fn admin_help_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let admin_level = player.get_int("관리자등급");
    if admin_level < 1000 {
        return CommandResult::Error("☞ 관리자 권한이 필요합니다.".to_string());
    }

    let output = "\x1b[1;37m=== 관리자 명령어 ===\x1b[0;37m\r\n\
아이템생성 [이름] [타입] [공격력] [방어력] - 아이템을 생성합니다\r\n\
타입: 무기, 방어구, 투구, 신발, 장신구\r\n\
위치이동 [지역] [방번호] - 해당 위치로 이동합니다\r\n\
\r\n☞ 더 많은 관리자 명령어가 추가될 예정입니다.";

    CommandResult::Output(output.to_string())
}

/// Sets admin level (for testing - no permission check)
fn set_admin_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Usage("관리자설정 [등급]".to_string());
    }

    let level = match args[0].parse::<i64>() {
        Ok(l) => l,
        Err(_) => return CommandResult::Error("☞ 올바른 숫자를 입력하세요.".to_string()),
    };

    player.set("관리자등급", level);
    CommandResult::Output(format!("관리자 등급을 {}으로 설정했습니다.", level))
}

/// Warps to a specific zone/room
fn warp_command(player: &mut Body, args: &[&str]) -> CommandResult {
    let admin_level = player.get_int("관리자등급");
    if admin_level < 1000 {
        return CommandResult::Error("☞ 관리자 권한이 필요합니다.".to_string());
    }

    if args.len() < 2 {
        return CommandResult::Usage("위치이동 [지역] [방번호]".to_string());
    }

    let zone = args[0];
    let room = args[1];

    // Set position in player object
    player.set("위치", format!("{}:{}", zone, room));

    // Also update world state for PvP tracking and spawn mobs
    let player_name = player.get_name();
    if !player_name.is_empty() {
        if let Ok(mut world) = crate::world::get_world_state().write() {
            world.set_player_position(
                &player_name,
                crate::world::PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            // Spawn mobs for the room
            world.spawn_mobs_for_room(zone, room);
        }
    }

    let msg = format!("{} {}으로 이동했습니다.", zone, room);
    CommandResult::Output(msg)
}

/// Registers all admin commands
pub fn register_admin_commands(registry: &mut CommandRegistry) {
    // 관리자설정 (Set admin level - for testing)
    registry.register(crate::command::registry::CommandInfo {
        name: "관리자설정".to_string(),
        aliases: vec!["adminset".to_string(), "setadmin".to_string()],
        handler: Arc::new(set_admin_command),
        level: 0,
        description: "관리자 등급을 설정합니다 (테스트용).".to_string(),
        usage: "관리자설정 [등급]".to_string(),
    });

    // 아이템생성 (Spawn item)
    registry.register(crate::command::registry::CommandInfo {
        name: "아이템생성".to_string(),
        aliases: vec![
            "생성".to_string(),
            "spawn".to_string(),
            "create".to_string(),
        ],
        handler: Arc::new(spawn_item_command),
        level: 0,
        description: "아이템을 생성합니다 (관리자).".to_string(),
        usage: "아이템생성 [이름] [타입] [공격력] [방어력]".to_string(),
    });

    // 위치이동 (Warp)
    registry.register(crate::command::registry::CommandInfo {
        name: "위치이동".to_string(),
        aliases: vec![
            "warp".to_string(),
            "이동".to_string(),
            "teleport".to_string(),
        ],
        handler: Arc::new(warp_command),
        level: 0,
        description: "해당 위치로 이동합니다 (관리자).".to_string(),
        usage: "위치이동 [지역] [방번호]".to_string(),
    });

    // 관리자도움말
    registry.register(crate::command::registry::CommandInfo {
        name: "관리자도움말".to_string(),
        aliases: vec!["관리자정보".to_string(), "adminhelp".to_string()],
        handler: Arc::new(admin_help_command),
        level: 0,
        description: "관리자 명령어 도움말.".to_string(),
        usage: "관리자도움말".to_string(),
    });
}
