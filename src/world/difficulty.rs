//! Difficulty zone configuration
//!
//! This module provides difficulty-based stat multipliers and bonuses
//! for optimizing memory usage in difficulty zones.
//!
//! Instead of copying map/mob files for each difficulty level,
//! we use templates (static data) and instances (dynamic data with difficulty).

/// Difficulty level type (0 = base, 1-9 = difficulty zones)
pub type DifficultyLevel = u8;

/// Difficulty configuration for stat multipliers and bonuses
#[derive(Debug, Clone)]
pub struct DifficultyConfig {
    /// Level bonus added to base level
    pub level_bonus: i64,
    /// HP multiplier (e.g., 2.5 = 250% HP)
    pub hp_multiplier: f64,
    /// Strength multiplier
    pub str_multiplier: f64,
    /// Defense/Arm multiplier
    pub arm_multiplier: f64,
    /// Agility multiplier
    pub agi_multiplier: f64,
    /// Experience bonus multiplier
    pub exp_multiplier: f64,
    /// Gold bonus multiplier
    pub gold_multiplier: f64,
    /// Item drop chance bonus (0.1 = +10%)
    pub drop_bonus: f64,
}

impl DifficultyConfig {
    /// Get difficulty configuration for a given level
    ///
    /// # Arguments
    /// * `difficulty` - Difficulty level (0 = base, 1-7 = difficulty zones)
    ///
    /// # Returns
    /// Configuration with appropriate multipliers
    pub fn get(difficulty: DifficultyLevel) -> Self {
        match difficulty {
            0 => Self::base(),
            1 => Self {
                level_bonus: 2000,
                hp_multiplier: 2.5,
                str_multiplier: 2.0,
                arm_multiplier: 2.0,
                agi_multiplier: 2.0,
                exp_multiplier: 2.0,
                gold_multiplier: 2.0,
                drop_bonus: 0.1,
            },
            2 => Self {
                level_bonus: 4000,
                hp_multiplier: 5.0,
                str_multiplier: 3.5,
                arm_multiplier: 3.5,
                agi_multiplier: 3.5,
                exp_multiplier: 3.0,
                gold_multiplier: 3.0,
                drop_bonus: 0.2,
            },
            3 => Self {
                level_bonus: 6000,
                hp_multiplier: 7.5,
                str_multiplier: 5.0,
                arm_multiplier: 5.0,
                agi_multiplier: 5.0,
                exp_multiplier: 4.0,
                gold_multiplier: 4.0,
                drop_bonus: 0.3,
            },
            4 => Self {
                level_bonus: 8000,
                hp_multiplier: 10.0,
                str_multiplier: 7.0,
                arm_multiplier: 7.0,
                agi_multiplier: 7.0,
                exp_multiplier: 5.0,
                gold_multiplier: 5.0,
                drop_bonus: 0.4,
            },
            5 => Self {
                level_bonus: 10000,
                hp_multiplier: 15.0,
                str_multiplier: 10.0,
                arm_multiplier: 10.0,
                agi_multiplier: 10.0,
                exp_multiplier: 6.0,
                gold_multiplier: 6.0,
                drop_bonus: 0.5,
            },
            6 => Self {
                level_bonus: 15000,
                hp_multiplier: 20.0,
                str_multiplier: 15.0,
                arm_multiplier: 15.0,
                agi_multiplier: 15.0,
                exp_multiplier: 7.5,
                gold_multiplier: 7.5,
                drop_bonus: 0.6,
            },
            7 => Self {
                level_bonus: 20000,
                hp_multiplier: 30.0,
                str_multiplier: 20.0,
                arm_multiplier: 20.0,
                agi_multiplier: 20.0,
                exp_multiplier: 10.0,
                gold_multiplier: 10.0,
                drop_bonus: 0.8,
            },
            8 => Self {
                level_bonus: 25_000,
                hp_multiplier: 53.69,
                str_multiplier: 12.5,
                arm_multiplier: 12.5,
                agi_multiplier: 12.5,
                exp_multiplier: 13.5,
                gold_multiplier: 13.5,
                drop_bonus: 0.9,
            },
            9 => Self {
                level_bonus: 30_000,
                hp_multiplier: 85.9,
                str_multiplier: 16.0,
                arm_multiplier: 16.0,
                agi_multiplier: 16.0,
                exp_multiplier: 17.0,
                gold_multiplier: 17.0,
                drop_bonus: 1.0,
            },
            _ => Self::base(),
        }
    }

    /// Base (no difficulty) configuration
    fn base() -> Self {
        Self {
            level_bonus: 0,
            hp_multiplier: 1.0,
            str_multiplier: 1.0,
            arm_multiplier: 1.0,
            agi_multiplier: 1.0,
            exp_multiplier: 1.0,
            gold_multiplier: 1.0,
            drop_bonus: 0.0,
        }
    }

    /// Apply difficulty to a base level
    pub fn apply_level(&self, base_level: i64) -> i64 {
        base_level + self.level_bonus
    }

    /// Apply difficulty to HP
    pub fn apply_hp(&self, base_hp: i64) -> i64 {
        (base_hp as f64 * self.hp_multiplier) as i64
    }

    /// Apply difficulty to strength
    pub fn apply_str(&self, base_str: i64) -> i64 {
        (base_str as f64 * self.str_multiplier) as i64
    }

    /// Apply difficulty to defense/arm
    pub fn apply_arm(&self, base_arm: i64) -> i64 {
        (base_arm as f64 * self.arm_multiplier) as i64
    }

    /// Apply difficulty to agility
    pub fn apply_agi(&self, base_agi: i64) -> i64 {
        (base_agi as f64 * self.agi_multiplier) as i64
    }

    /// Calculate bonus exp
    pub fn bonus_exp(&self, base_exp: i64) -> i64 {
        ((base_exp as f64) * (self.exp_multiplier - 1.0)) as i64
    }

    /// Calculate bonus gold
    pub fn bonus_gold(&self, base_gold: i64) -> i64 {
        ((base_gold as f64) * (self.gold_multiplier - 1.0)) as i64
    }

    /// Get minimum level required for a difficulty zone
    pub fn min_level_for_difficulty(difficulty: DifficultyLevel) -> i64 {
        match difficulty {
            0 => 0,
            1 => 2000,
            2 => 4000,
            3 => 6000,
            4 => 8000,
            5 => 10000,
            6 => 15000,
            7 => 20000,
            _ => 0,
        }
    }
}

impl Default for DifficultyConfig {
    fn default() -> Self {
        Self::get(0)
    }
}

/// Get difficulty from zone name suffix
///
/// # Arguments
/// * `zone_name` - Zone name (e.g., "낙양성", "낙양성1", "낙양성2")
///
/// # Returns
/// Difficulty level (0 for base zone, 1-9 for difficulty zones)
pub fn difficulty_from_zone(zone_name: &str) -> DifficultyLevel {
    // Try to extract trailing number from zone name
    // "낙양성" -> 0, "낙양성1" -> 1, "낙양성2" -> 2, etc.

    let mut num_str = String::new();
    for c in zone_name.chars().rev() {
        if c.is_ascii_digit() {
            num_str.insert(0, c);
        } else {
            break;
        }
    }

    if num_str.is_empty() {
        0
    } else {
        num_str.parse::<u8>().unwrap_or(0).min(9)
    }
}

/// Get base zone name from any difficulty zone
///
/// # Arguments
/// * `zone_name` - Zone name (e.g., "낙양성", "낙양성1", "낙양성2")
///
/// # Returns
/// Base zone name without difficulty suffix (e.g., "낙양성")
pub fn base_zone_name(zone_name: &str) -> &str {
    // Find where the trailing digits start
    let mut digit_start = zone_name.len();
    for (i, c) in zone_name.char_indices().rev() {
        if c.is_ascii_digit() {
            digit_start = i;
        } else {
            break;
        }
    }
    &zone_name[..digit_start]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_config_base() {
        let config = DifficultyConfig::get(0);
        assert_eq!(config.level_bonus, 0);
        assert_eq!(config.hp_multiplier, 1.0);
    }

    #[test]
    fn test_difficulty_config_level_1() {
        let config = DifficultyConfig::get(1);
        assert_eq!(config.level_bonus, 2000);
        assert_eq!(config.hp_multiplier, 2.5);
    }

    #[test]
    fn test_difficulty_config_level_7() {
        let config = DifficultyConfig::get(7);
        assert_eq!(config.level_bonus, 20000);
        assert_eq!(config.hp_multiplier, 30.0);
    }

    #[test]
    fn reward_bonus_matches_python_body_difficulty_table() {
        let expected = [100, 200, 300, 400, 500, 650, 900, 1250, 1600];
        for (level, bonus) in (1_u8..=9).zip(expected) {
            assert_eq!(DifficultyConfig::get(level).bonus_exp(100), bonus);
            assert_eq!(DifficultyConfig::get(level).bonus_gold(100), bonus);
        }
    }

    #[test]
    fn test_apply_hp() {
        let config = DifficultyConfig::get(2);
        assert_eq!(config.apply_hp(100), 500); // 100 * 5.0
    }

    #[test]
    fn test_difficulty_from_zone() {
        assert_eq!(difficulty_from_zone("낙양성"), 0);
        assert_eq!(difficulty_from_zone("낙양성1"), 1);
        assert_eq!(difficulty_from_zone("낙양성2"), 2);
        assert_eq!(difficulty_from_zone("낙양성7"), 7);
        assert_eq!(difficulty_from_zone("낙양성8"), 8);
        assert_eq!(difficulty_from_zone("낙양성9"), 9);
        assert_eq!(difficulty_from_zone("낙양성10"), 9); // capped at 9
    }

    #[test]
    fn test_base_zone_name() {
        assert_eq!(base_zone_name("낙양성"), "낙양성");
        assert_eq!(base_zone_name("낙양성1"), "낙양성");
        assert_eq!(base_zone_name("낙양성7"), "낙양성");
    }
}
