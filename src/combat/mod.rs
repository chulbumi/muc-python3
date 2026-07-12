//! Combat module for MUD engine
//!
//! Handles all combat-related functionality:
//! - PvM (Player vs Mob) combat
//! - PvP (Player vs Player) combat
//! - Damage calculations
//! - Combat state management

pub mod processor;

pub use processor::{
    apply_skill_effects, calculate_mob_damage, calculate_player_damage, calculate_skill_damage,
    calculate_skill_damage_against, check_hit, find_mob_in_room, process_mob_strike,
    process_player_attack, process_player_strike, start_combat, CombatAction, CombatRound,
    SkillDamageResult, SkillEffectResult, SkillEffectType,
};
