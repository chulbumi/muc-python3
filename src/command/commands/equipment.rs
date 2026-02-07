//! Equipment and inventory commands for MUD engine
//!
//! Handles item management: 입고 (equip), 벗고 (unequip), 소지품 (inventory), 장비 (equipment)

use crate::command::{CommandResult, registry::CommandRegistry};
use crate::player::Body;
use crate::hangul;
use std::sync::Arc;

/// Equipment slot types
#[derive(Debug, Clone, PartialEq)]
enum EquipmentSlot {
    Weapon,
    Armor,
    Helmet,
    Boots,
    Accessory,
}

impl EquipmentSlot {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "무기" => Some(EquipmentSlot::Weapon),
            "방어구" => Some(EquipmentSlot::Armor),
            "투구" => Some(EquipmentSlot::Helmet),
            "신발" => Some(EquipmentSlot::Boots),
            "장신구" => Some(EquipmentSlot::Accessory),
            _ => None,
        }
    }

    fn display_name(&self) -> &str {
        match self {
            EquipmentSlot::Weapon => "무기",
            EquipmentSlot::Armor => "방어구",
            EquipmentSlot::Helmet => "투구",
            EquipmentSlot::Boots => "신발",
            EquipmentSlot::Accessory => "장신구",
        }
    }
}

/// Shows player's inventory
fn inventory_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let mut output = String::from("\x1b[1;37m=== 소지품 ===\x1b[0;37m\r\n");

    // Count stackable items
    let mut stack_count = 0;
    for (_key, count) in player.object.inv_stack.iter() {
        stack_count += *count;
    }

    // Count non-stackable items (excluding equipped)
    let mut item_list: Vec<String> = Vec::new();
    for obj in &player.object.objs {
        if let Ok(item) = obj.lock() {
            let in_use = item.getBool("inUse");
            if !in_use && !item.checkAttr("아이템속성", "출력안함") {
                let name = item.getString("이름");
                let weight = item.getInt("무게");
                let weight_str = if weight > 0 {
                    format!(" ({}kg)", weight)
                } else {
                    String::new()
                };
                item_list.push(format!("  \x1b[36m{}\x1b[0;37m{}", name, weight_str));
            }
        }
    }

    if stack_count > 0 {
        output.push_str(&format!("\x1b[33m소지품: {}개\x1b[0;37m\r\n", stack_count));
    }

    if !item_list.is_empty() {
        let items_str = item_list.join("\r\n");
        output.push_str(&items_str);
        output.push_str("\r\n");
    } else {
        output.push_str("(비어있음)\r\n");
    }

    let total_weight = player.get_item_weight();
    let max_weight = player.get_str() * 10;
    output.push_str(&format!("무게: {}/{}\r\n", total_weight, max_weight));

    CommandResult::Output(output)
}

/// Shows player's current equipment
fn equipment_command(player: &mut Body, _args: &[&str]) -> CommandResult {
    let mut output = String::from("\x1b[1;37m=== 장비 ===\x1b[0;37m\r\n");

    let mut has_equipment = false;

    // Check for equipped weapon
    if let Some(weapon_ref) = &player.weapon_item {
        if let Some(weapon) = weapon_ref.upgrade() {
            if let Ok(w) = weapon.lock() {
                let name = w.getString("이름");
                let attack = w.getInt("공격력");
                let attack_str = if attack > 0 {
                    format!(" (공격력 +{})", attack)
                } else {
                    String::new()
                };
                output.push_str(&format!("\x1b[33m무기\x1b[0;37m: {}{}\r\n", name, attack_str));
                has_equipment = true;
            }
        }
    }

    // Check for equipped armor/accessories in objs
    for obj in &player.object.objs {
        if let Ok(item) = obj.lock() {
            if item.getBool("inUse") {
                let name = item.getString("이름");
                let item_type = item.getString("타입");

                let bonus = match item_type.as_str() {
                    "방어구" => {
                        let armor = item.getInt("방어력");
                        if armor > 0 {
                            format!(" (방어력 +{})", armor)
                        } else {
                            String::new()
                        }
                    }
                    "투구" => {
                        let armor = item.getInt("방어력");
                        if armor > 0 {
                            format!(" (방어력 +{})", armor)
                        } else {
                            String::new()
                        }
                    }
                    "신발" => {
                        let dex = item.getInt("민첩");
                        if dex > 0 {
                            format!(" (민첩 +{})", dex)
                        } else {
                            String::new()
                        }
                    }
                    "장신구" => {
                        let hp = item.getInt("체력");
                        if hp > 0 {
                            format!(" (체력 +{})", hp)
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                };

                output.push_str(&format!("\x1b[33m{}\x1b[0;37m: {}{}\r\n", item_type, name, bonus));
                has_equipment = true;
            }
        }
    }

    if !has_equipment {
        output.push_str("(장비한 아이템이 없습니다)\r\n");
    }

    // Show total bonuses
    output.push_str("\r\n\x1b[1;37m=== 보너스 ===\x1b[0;37m\r\n");
    if player.attpower > 0 {
        output.push_str(&format!("공격력: +{}\r\n", player.attpower));
    }
    if player.armor > 0 {
        output.push_str(&format!("방어력: +{}\r\n", player.armor));
    }
    if player._str > 0 {
        output.push_str(&format!("힘: +{}\r\n", player._str));
    }
    if player._dex > 0 {
        output.push_str(&format!("민첩: +{}\r\n", player._dex));
    }
    if player._arm > 0 {
        output.push_str(&format!("맷집: +{}\r\n", player._arm));
    }
    if player._maxhp > 0 {
        output.push_str(&format!("최대체력: +{}\r\n", player._maxhp));
    }
    if player._maxmp > 0 {
        output.push_str(&format!("최대내공: +{}\r\n", player._maxmp));
    }

    CommandResult::Output(output)
}

/// Equips an item by name
fn equip_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Usage("입고 [아이템이름]".to_string());
    }

    let item_name = args[0];

    // Find the item in inventory and collect its data
    let mut found_item: Option<Arc<std::sync::Mutex<crate::object::Object>>> = None;
    let mut item_type = String::new();
    let mut display_name = String::new();
    let mut stat_attack = 0i32;
    let mut stat_defense = 0i32;
    let mut stat_strength = 0i32;
    let mut stat_dexterity = 0i32;
    let mut stat_armor = 0i32;
    let mut stat_hp = 0i32;
    let mut stat_mp = 0i32;

    for obj in player.object.objs.iter() {
        if let Ok(item) = obj.lock() {
            if item.getBool("inUse") {
                continue; // Skip already equipped items
            }

            let name = item.getString("이름");
            if name == item_name || name.contains(item_name) {
                let itype = item.getString("타입");
                if itype.is_empty() {
                    continue;
                }

                // Check if this slot is already occupied
                match itype.as_str() {
                    "무기" => {
                        if player.weapon_item.is_some() {
                            return CommandResult::Error("☞ 이미 무기를 장착하고 있습니다.".to_string());
                        }
                    }
                    "방어구" | "투구" | "신발" | "장신구" => {
                        for other_obj in &player.object.objs {
                            if let Ok(other) = other_obj.lock() {
                                if other.getBool("inUse") && other.getString("타입") == itype {
                                    return CommandResult::Error(format!("☞ 이미 {}를 장착하고 있습니다.", itype));
                                }
                            }
                        }
                    }
                    _ => {
                        return CommandResult::Error("☞ 장착할 수 없는 아이템입니다.".to_string());
                    }
                }

                // Collect item data
                item_type = itype;
                display_name = name;
                stat_attack = item.getInt("공격력") as i32;
                stat_defense = item.getInt("방어력") as i32;
                stat_strength = item.getInt("힘") as i32;
                stat_dexterity = item.getInt("민첩") as i32;
                stat_armor = item.getInt("맷집") as i32;
                stat_hp = item.getInt("체력") as i32;
                stat_mp = item.getInt("내공") as i32;
                found_item = Some(obj.clone());
                break;
            }
        }
    }

    let item_arc = match found_item {
        Some(i) => i,
        None => return CommandResult::Error("☞ 그런 아이템을 가지고 있지 않습니다.".to_string()),
    };

    // Apply stat bonuses
    player.attpower += stat_attack;
    player.armor += stat_defense;
    player._str += stat_strength;
    player._dex += stat_dexterity;
    player._arm += stat_armor;
    player._maxhp += stat_hp;
    player._maxmp += stat_mp;

    // Mark as equipped and store weapon reference
    if let Ok(mut item) = item_arc.lock() {
        item.set("inUse", 1);
    }

    if item_type == "무기" {
        let weak_ref = Arc::downgrade(&item_arc);
        player.weapon_item = Some(weak_ref);
    }

    // Show equip message
    let particle = hangul::han_obj(&display_name);
    CommandResult::Output(format!("{} {} 장착했습니다.", display_name, particle))
}

/// Unequips an item by name or slot
fn unequip_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Usage("벗고 [아이템이름 또는 무기/방어구/투구/신발/장신구]".to_string());
    }

    let target = args[0];

    // Check if it's a slot name
    if let Some(slot) = EquipmentSlot::from_str(target) {
        return unequip_slot(player, slot);
    }

    // Find equipped item by name and collect its data
    let mut found_item: Option<Arc<std::sync::Mutex<crate::object::Object>>> = None;
    let mut item_type = String::new();
    let mut display_name = String::new();
    let mut stat_attack = 0i32;
    let mut stat_defense = 0i32;
    let stat_strength = 0i32;
    let mut stat_dexterity = 0i32;
    let mut stat_armor = 0i32;
    let mut stat_hp = 0i32;
    let stat_mp = 0i32;

    // Check weapon by name
    if let Some(weapon_ref) = &player.weapon_item {
        if let Some(weapon) = weapon_ref.upgrade() {
            if let Ok(w) = weapon.lock() {
                let name = w.getString("이름");
                if name == target || name.contains(target) {
                    display_name = name;
                    stat_attack = w.getInt("공격력") as i32;
                    found_item = Some(weapon.clone());
                    item_type = "무기".to_string();
                }
            }
        }
    }

    // Check other equipped items
    if found_item.is_none() {
        for obj in &player.object.objs {
            if let Ok(item) = obj.lock() {
                if item.getBool("inUse") {
                    let name = item.getString("이름");
                    if name == target || name.contains(target) {
                        display_name = name;
                        item_type = item.getString("타입");
                        stat_defense = item.getInt("방어력") as i32;
                        stat_armor = item.getInt("맷집") as i32;
                        stat_dexterity = item.getInt("민첩") as i32;
                        stat_hp = item.getInt("체력") as i32;
                        found_item = Some(obj.clone());
                        break;
                    }
                }
            }
        }
    }

    let item_arc = match found_item {
        Some(i) => i,
        None => return CommandResult::Error("☞ 그런 아이템을 장착하고 있지 않습니다.".to_string()),
    };

    // Remove stat bonuses
    player.attpower -= stat_attack;
    player.armor -= stat_defense;
    player._str -= stat_strength;
    player._dex -= stat_dexterity;
    player._arm -= stat_armor;
    player._maxhp -= stat_hp;
    player._maxmp -= stat_mp;

    // Mark as unequipped
    if let Ok(mut item) = item_arc.lock() {
        item.set("inUse", 0);
    }

    // Clear weapon reference
    if item_type == "무기" {
        player.weapon_item = None;
    }

    // Show unequip message
    let particle = hangul::han_obj(&display_name);
    CommandResult::Output(format!("{} {} 벗었습니다.", display_name, particle))
}

/// Helper function to unequip by slot
fn unequip_slot(player: &mut Body, slot: EquipmentSlot) -> CommandResult {
    match slot {
        EquipmentSlot::Weapon => {
            if let Some(weapon_ref) = &player.weapon_item {
                if let Some(weapon) = weapon_ref.upgrade() {
                    if let Ok(w) = weapon.lock() {
                        let name = w.getString("이름");
                        let attack = w.getInt("공격력") as i32;

                        player.attpower -= attack;
                        drop(w);

                        // Mark as unequipped
                        if let Some(weapon) = weapon_ref.upgrade() {
                            if let Ok(mut w) = weapon.lock() {
                                w.set("inUse", 0);
                            }
                        }
                        player.weapon_item = None;

                        let particle = hangul::han_obj(&name);
                        return CommandResult::Output(format!("{} {} 벗었습니다.", name, particle));
                    }
                }
            }
            CommandResult::Error("☞ 무기를 장착하고 있지 않습니다.".to_string())
        }
        EquipmentSlot::Armor | EquipmentSlot::Helmet | EquipmentSlot::Boots | EquipmentSlot::Accessory => {
            let slot_name = slot.display_name();
            let mut found = false;
            let mut item_name = String::new();
            let mut stat_defense = 0i32;
            let mut stat_armor = 0i32;
            let mut stat_dexterity = 0i32;
            let mut stat_hp = 0i32;

            for obj in &player.object.objs {
                if let Ok(item) = obj.lock() {
                    if item.getBool("inUse") && item.getString("타입") == slot_name {
                        item_name = item.getString("이름");
                        stat_defense = item.getInt("방어력") as i32;
                        stat_armor = item.getInt("맷집") as i32;
                        stat_dexterity = item.getInt("민첩") as i32;
                        stat_hp = item.getInt("체력") as i32;
                        found = true;
                        break;
                    }
                }
            }

            if found {
                player.armor -= stat_defense;
                player._arm -= stat_armor;
                player._dex -= stat_dexterity;
                player._maxhp -= stat_hp;

                // Mark as unequipped
                for obj in &player.object.objs {
                    if let Ok(mut item) = obj.lock() {
                        if item.getString("이름") == item_name {
                            item.set("inUse", 0);
                            break;
                        }
                    }
                }

                let particle = hangul::han_obj(&item_name);
                CommandResult::Output(format!("{} {} 벗었습니다.", item_name, particle))
            } else {
                CommandResult::Error(format!("☞ {}를 장착하고 있지 않습니다.", slot_name))
            }
        }
    }
}

/// Shows detailed item information
fn item_info_command(player: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Usage("아이템정보 [아이템이름]".to_string());
    }

    let item_name = args[0];

    // Search in inventory
    for obj in &player.object.objs {
        if let Ok(item) = obj.lock() {
            let name = item.getString("이름");
            if name == item_name || name.contains(item_name) {
                let item_type = item.getString("타입");
                let attack = item.getInt("공격력");
                let defense = item.getInt("방어력");
                let strength = item.getInt("힘");
                let dexterity = item.getInt("민첩");
                let hp = item.getInt("체력");
                let mp = item.getInt("내공");
                let weight = item.getInt("무게");
                let desc = item.getString("설명");

                let mut output = format!("\x1b[1;36m{}\x1b[0;37m\r\n", name);
                if !item_type.is_empty() {
                    output.push_str(&format!("종류: {}\r\n", item_type));
                }
                if attack > 0 {
                    output.push_str(&format!("공격력: +{}\r\n", attack));
                }
                if defense > 0 {
                    output.push_str(&format!("방어력: +{}\r\n", defense));
                }
                if strength > 0 {
                    output.push_str(&format!("힘: +{}\r\n", strength));
                }
                if dexterity > 0 {
                    output.push_str(&format!("민첩: +{}\r\n", dexterity));
                }
                if hp > 0 {
                    output.push_str(&format!("체력: +{}\r\n", hp));
                }
                if mp > 0 {
                    output.push_str(&format!("내공: +{}\r\n", mp));
                }
                if weight > 0 {
                    output.push_str(&format!("무게: {}kg\r\n", weight));
                }
                if !desc.is_empty() {
                    output.push_str(&format!("설명: {}\r\n", desc));
                }

                let in_use = item.getBool("inUse");
                output.push_str(&format!("상태: {}\r\n", if in_use { "장착중" } else { "소지중" }));

                return CommandResult::Output(output);
            }
        }
    }

    // Also check stackable items
    for (key, count) in player.object.inv_stack.iter() {
        if key.contains(item_name) {
            return CommandResult::Output(format!("\x1b[1;36m{}\x1b[0;37m\r\n수량: {}개\r\n(소지품)", key, count));
        }
    }

    CommandResult::Error("☞ 그런 아이템을 가지고 있지 않습니다.".to_string())
}

/// Registers all equipment commands
pub fn register_equipment_commands(registry: &mut CommandRegistry) {
    // 소지품 (Inventory) - Rhai 스크립트로 처리 (cmds/소지품.rhai)
    // 네이티브 명령을 제거하여 Rhai 스크립트가 우선 적용되도록 함

    // 장비 (Equipment)
    registry.register(crate::command::registry::CommandInfo {
        name: "장비".to_string(),
        aliases: vec!["장착".to_string(), "입고".to_string(), "equipment".to_string(), "eq".to_string()],
        handler: Arc::new(equipment_command),
        level: 0,
        description: "장착한 장비를 보여줍니다.".to_string(),
        usage: "장비".to_string(),
    });

    // 입고 (Equip item)
    registry.register(crate::command::registry::CommandInfo {
        name: "입고".to_string(),
        aliases: vec!["장착".to_string(), "equip".to_string(), "wield".to_string()],
        handler: Arc::new(equip_command),
        level: 0,
        description: "아이템을 장착합니다.".to_string(),
        usage: "입고 [아이템이름]".to_string(),
    });

    // 벗고 (Unequip item)
    registry.register(crate::command::registry::CommandInfo {
        name: "벗고".to_string(),
        aliases: vec!["해제".to_string(), "unequip".to_string(), "remove".to_string()],
        handler: Arc::new(unequip_command),
        level: 0,
        description: "아이템을 해제합니다.".to_string(),
        usage: "벗고 [아이템이름 또는 무기/방어구/투구/신발/장신구]".to_string(),
    });

    // 아이템정보 (Item info)
    registry.register(crate::command::registry::CommandInfo {
        name: "아이템정보".to_string(),
        aliases: vec!["아이템".to_string(), "item".to_string(), "info".to_string()],
        handler: Arc::new(item_info_command),
        level: 0,
        description: "아이템 정보를 보여줍니다.".to_string(),
        usage: "아이템정보 [아이템이름]".to_string(),
    });
}
