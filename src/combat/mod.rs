//! Combat module for MUD engine
//!
//! Handles all combat-related functionality:
//! - PvM (Player vs Mob) combat
//! - PvP (Player vs Player) combat
//! - Damage calculations
//! - Combat state management

pub mod processor;

pub use processor::{
    calculate_mob_damage, calculate_player_damage, check_hit, find_mob_in_room,
    process_player_attack, start_combat, CombatAction, CombatRound,
};

use crate::command::registry::CommandRegistry;
use crate::command::CommandResult;
use crate::player::{ActState, Body};
use crate::world::WorldState;

/// Player attacks a mob (쳐 command for PvM)
pub fn attack_mob_command(player: &mut Body, args: &[&str], world: &WorldState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Error("☞ 사용법: 쳐 [대상]".to_string());
    }

    let target_name = args[0];

    // Check if player can attack (not dead)
    if player.act == ActState::Death {
        return CommandResult::Error("☞ 죽은 사람은 공격할 수 없습니다.".to_string());
    }

    // Find mob in room
    let (mob_instance, mob_data) =
        match processor::find_mob_in_room(&player.get_name(), target_name, world) {
            Some(result) => result,
            None => {
                // Try to find player instead (PvP)
                let player_pos = match world.get_player_position(&player.get_name()) {
                    Some(pos) => pos,
                    None => {
                        return CommandResult::Error(
                            "☞ 당신의 위치를 찾을 수 없습니다.".to_string(),
                        )
                    }
                };

                let players_in_room = world.get_players_in_room(&player_pos.zone, &player_pos.room);
                if players_in_room.iter().any(|n| n == target_name) {
                    // PvP attack - return Combat for PvP handling
                    return CommandResult::Combat;
                }

                return CommandResult::Error("☞ 그런 상대가 없습니다.".to_string());
            }
        };

    // Process the attack
    let round = processor::process_player_attack(player, &mob_instance, &mob_data);

    // Build result messages
    if round.player_died || round.target_died {
        // For now, use Output with all messages
        let mut all_messages = round.player_messages;
        all_messages.extend(round.room_messages);
        CommandResult::Output(all_messages.join("\r\n"))
    } else {
        let mut all_messages = round.player_messages;
        all_messages.extend(round.room_messages);
        CommandResult::Output(all_messages.join("\r\n"))
    }
}

/// Register combat commands
pub fn register_combat_commands(_registry: &mut CommandRegistry) {
    // Note: This is handled by the combat.rs module
    // This is just a placeholder for any additional combat-related commands
}
