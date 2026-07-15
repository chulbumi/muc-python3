//! Recomputed equipment-set and inventory possession effects.
//!
//! Set bonuses count equipped, distinct set pieces only. Possession effects
//! are a separate mechanic for talismans and similar non-equipped items.

use crate::object::Value;
use crate::player::Body;
use rhai::{Array, Dynamic, Engine, Map};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const APPLIED_STATS_KEY: &str = "_applied_item_effect_stats";
const ACTIVE_EFFECTS_KEY: &str = "_active_item_effects";
const TIMED_EFFECTS_ATTR: &str = "소모품버프";
const APPLIED_TIMED_STATS_KEY: &str = "_applied_consumable_effect_stats";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct EffectStats {
    values: BTreeMap<String, i64>,
}

impl EffectStats {
    fn add(&mut self, name: &str, amount: i64) {
        let canonical = match name.trim() {
            "민첩" => "민첩성",
            "최고체력" | "최대체력" => "체력",
            "최고내공" | "최대내공" => "내공",
            other => other,
        };
        if matches!(
            canonical,
            "공격력"
                | "방어력"
                | "힘"
                | "민첩성"
                | "맷집"
                | "체력"
                | "내공"
                | "명중"
                | "회피"
                | "필살"
                | "운"
                | "경험치"
                | "마법발견"
        ) {
            *self.values.entry(canonical.to_string()).or_insert(0) += amount;
        }
    }

    fn merge(&mut self, other: &Self) {
        for (name, amount) in &other.values {
            self.add(name, *amount);
        }
    }

    fn is_empty(&self) -> bool {
        self.values.values().all(|amount| *amount == 0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveEffect {
    kind: String,
    name: String,
    count: i64,
    effects: EffectStats,
}

#[derive(Debug, Clone, Default)]
struct ItemEffectDefinition {
    name: String,
    group: String,
    minimum: i64,
    set_effects: BTreeMap<i64, EffectStats>,
    possession: EffectStats,
    consumable: Option<ConsumableEffectDefinition>,
    permanent: EffectStats,
    permanent_script: String,
}

#[derive(Debug, Clone)]
struct ConsumableEffectDefinition {
    name: String,
    duration_seconds: i64,
    effects: EffectStats,
    activation_script: String,
    expiration_script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimedEffect {
    id: String,
    name: String,
    expires_at: i64,
    effects: EffectStats,
    expiration_script: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConsumableEffectApplication {
    pub name: String,
    pub duration_seconds: i64,
    pub activation_script: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PermanentEffectApplication {
    pub effects: Vec<(String, i64)>,
    pub activation_script: String,
}

type DefinitionCache = HashMap<
    String,
    (
        Option<SystemTime>,
        Option<u64>,
        Option<ItemEffectDefinition>,
    ),
>;
static DEFINITION_CACHE: OnceLock<Mutex<DefinitionCache>> = OnceLock::new();

fn item_info(key: &str) -> Option<serde_json::Map<String, JsonValue>> {
    let path = format!("data/item/{key}.json");
    let root: JsonValue = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    root.get("아이템정보")?.as_object().cloned()
}

fn parse_effect_string(raw: &str, stats: &mut EffectStats) {
    for effect in raw.split([',', '|', '\n', '\r']) {
        let words = effect.split_whitespace().collect::<Vec<_>>();
        if words.len() < 2 {
            continue;
        }
        let amount = words.last().and_then(|value| value.parse::<i64>().ok());
        if let Some(amount) = amount {
            stats.add(&words[..words.len() - 1].join(" "), amount);
        }
    }
}

fn parse_effects(value: &JsonValue) -> EffectStats {
    let mut stats = EffectStats::default();
    match value {
        JsonValue::String(raw) => parse_effect_string(raw, &mut stats),
        JsonValue::Array(values) => {
            for value in values {
                stats.merge(&parse_effects(value));
            }
        }
        JsonValue::Object(values) => {
            for (name, value) in values {
                if let Some(amount) = value.as_i64() {
                    stats.add(name, amount);
                } else if let Some(raw) = value.as_str() {
                    if let Ok(amount) = raw.parse::<i64>() {
                        stats.add(name, amount);
                    }
                }
            }
        }
        _ => {}
    }
    stats
}

fn parse_minimum(value: Option<&JsonValue>) -> i64 {
    match value {
        Some(JsonValue::Number(value)) => value.as_i64().unwrap_or(0),
        Some(JsonValue::Array(values)) => values
            .iter()
            .filter_map(JsonValue::as_i64)
            .min()
            .unwrap_or(0),
        Some(JsonValue::String(value)) => value.parse().unwrap_or(0),
        _ => 0,
    }
}

fn load_definition(key: &str) -> Option<ItemEffectDefinition> {
    let info = item_info(key)?;
    let name = info
        .get("이름")
        .and_then(JsonValue::as_str)
        .unwrap_or(key)
        .to_string();
    let group = info
        .get("세트그룹")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let minimum = parse_minimum(info.get("세트조건")).max(0);
    let mut set_effects = BTreeMap::new();
    if let Some(value) = info.get("세트효과") {
        if let Some(tiers) = value.as_object() {
            for (tier, value) in tiers {
                if let Ok(tier) = tier.parse::<i64>() {
                    let effects = parse_effects(value);
                    if tier > 0 && !effects.is_empty() {
                        set_effects
                            .entry(tier)
                            .or_insert_with(EffectStats::default)
                            .merge(&effects);
                    }
                }
            }
        } else if minimum > 0 {
            let effects = parse_effects(value);
            if !effects.is_empty() {
                set_effects.insert(minimum, effects);
            }
        }
    }
    for (field, value) in &info {
        let Some(tier) = field
            .strip_prefix("세트효과 ")
            .and_then(|tier| tier.trim().parse::<i64>().ok())
        else {
            continue;
        };
        let effects = parse_effects(value);
        if tier > 0 && !effects.is_empty() {
            set_effects
                .entry(tier)
                .or_insert_with(EffectStats::default)
                .merge(&effects);
        }
    }
    let possession = info.get("소지효과").map(parse_effects).unwrap_or_default();
    let consumable = info.get("사용효과").and_then(|value| {
        let config = value.as_object()?;
        let duration_seconds = config
            .get("지속시간")
            .and_then(JsonValue::as_i64)
            .unwrap_or(0)
            .max(0);
        let effects = config
            .get("효과")
            .or_else(|| config.get("능력치"))
            .map(parse_effects)
            .unwrap_or_default();
        if duration_seconds == 0 || effects.is_empty() {
            return None;
        }
        Some(ConsumableEffectDefinition {
            name: config
                .get("이름")
                .and_then(JsonValue::as_str)
                .unwrap_or(&name)
                .to_string(),
            duration_seconds,
            effects,
            activation_script: config
                .get("발동스크립")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_string(),
            expiration_script: config
                .get("만료스크립")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_string(),
        })
    });
    let permanent_value = info.get("영구효과");
    let permanent = permanent_value
        .and_then(JsonValue::as_object)
        .and_then(|config| config.get("효과").or_else(|| config.get("능력치")))
        .or(permanent_value)
        .map(parse_effects)
        .unwrap_or_default();
    let permanent_script = permanent_value
        .and_then(JsonValue::as_object)
        .and_then(|config| config.get("발동스크립"))
        .and_then(JsonValue::as_str)
        .or_else(|| info.get("영구효과스크립").and_then(JsonValue::as_str))
        .unwrap_or("")
        .to_string();
    Some(ItemEffectDefinition {
        name,
        group,
        minimum,
        set_effects,
        possession,
        consumable,
        permanent,
        permanent_script,
    })
}

fn definition(key: &str) -> Option<ItemEffectDefinition> {
    let path = format!("data/item/{key}.json");
    let metadata = std::fs::metadata(&path).ok();
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok());
    let file_len = metadata.as_ref().map(|metadata| metadata.len());
    let cache = DEFINITION_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = cache.lock() {
        if let Some((cached_modified, cached_len, definition)) = cache.get(key) {
            if *cached_modified == modified && *cached_len == file_len {
                return definition.clone();
            }
        }
    }
    let loaded = load_definition(key);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key.to_string(), (modified, file_len, loaded.clone()));
    }
    loaded
}

fn compute(body: &Body) -> (EffectStats, Vec<ActiveEffect>) {
    let mut equipped_groups = HashMap::<String, HashSet<String>>::new();
    let mut equipped_definitions = HashMap::<String, Vec<(String, ItemEffectDefinition)>>::new();
    let mut possession_items = HashMap::<String, ItemEffectDefinition>::new();

    for item in &body.object.objs {
        let Ok(item) = item.lock() else { continue };
        let key = item.getString("인덱스");
        if key.is_empty() {
            continue;
        }
        let Some(definition) = definition(&key) else {
            continue;
        };
        if !definition.possession.is_empty() {
            possession_items
                .entry(key.clone())
                .or_insert_with(|| definition.clone());
        }
        if item.getBool("inUse") && !definition.group.is_empty() {
            equipped_groups
                .entry(definition.group.clone())
                .or_default()
                .insert(key.clone());
            if definition.minimum > 0 && !definition.set_effects.is_empty() {
                equipped_definitions
                    .entry(definition.group.clone())
                    .or_default()
                    .push((key, definition));
            }
        }
    }
    for (key, count) in &body.object.inv_stack {
        if *count <= 0 {
            continue;
        }
        if let Some(definition) = definition(key) {
            if !definition.possession.is_empty() {
                possession_items.entry(key.clone()).or_insert(definition);
            }
        }
    }

    let mut total = EffectStats::default();
    let mut active = Vec::new();
    for (key, definition) in possession_items {
        total.merge(&definition.possession);
        active.push(ActiveEffect {
            kind: "possession".into(),
            name: definition.name,
            count: 1,
            effects: definition.possession,
        });
        let _ = key;
    }

    for (group, pieces) in equipped_groups {
        let count = pieces.len() as i64;
        let Some(definitions) = equipped_definitions.get(&group) else {
            continue;
        };
        let mut definitions = definitions.clone();
        definitions.sort_by(|left, right| left.0.cmp(&right.0));
        let mut applied_tiers = BTreeMap::<i64, EffectStats>::new();
        for (_, definition) in &definitions {
            if count < definition.minimum {
                continue;
            }
            for (tier, effects) in &definition.set_effects {
                if *tier <= count && *tier >= definition.minimum {
                    // A second definition-bearing piece is not a second copy
                    // of the set bonus. The first item key deterministically
                    // owns a tier when malformed data defines it twice.
                    applied_tiers
                        .entry(*tier)
                        .or_insert_with(|| effects.clone());
                }
            }
        }
        for (tier, effects) in applied_tiers {
            if effects.is_empty() {
                continue;
            }
            total.merge(&effects);
            active.push(ActiveEffect {
                kind: "set".into(),
                name: group.clone(),
                count: tier,
                effects,
            });
        }
    }
    active.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.count.cmp(&right.count))
    });
    (total, active)
}

fn apply(body: &mut Body, stats: &EffectStats, sign: i64) {
    let value = |name: &str| {
        stats
            .values
            .get(name)
            .copied()
            .unwrap_or(0)
            .saturating_mul(sign)
    };
    let add = |current: i32, amount: i64| {
        (i64::from(current) + amount).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    };
    body.attpower = add(body.attpower, value("공격력"));
    body.armor = add(body.armor, value("방어력"));
    body._str = add(body._str, value("힘"));
    body._dex = add(body._dex, value("민첩성"));
    body._arm = add(body._arm, value("맷집"));
    body._maxhp = add(body._maxhp, value("체력"));
    body._maxmp = add(body._maxmp, value("내공"));
    body._hit = add(body._hit, value("명중"));
    body._miss = add(body._miss, value("회피"));
    body._critical = add(body._critical, value("필살"));
    body._critical_chance = add(body._critical_chance, value("운"));
    body._exp = add(body._exp, value("경험치"));
    body._magic_chance = add(body._magic_chance, value("마법발견"));
}

pub(crate) fn clear(body: &mut Body) {
    let previous = body
        .temp_mut()
        .remove(APPLIED_STATS_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str::<EffectStats>(&json).ok())
        .unwrap_or_default();
    apply(body, &previous, -1);
    body.temp_mut().remove(ACTIVE_EFFECTS_KEY);
}

pub(crate) fn refresh(body: &mut Body) {
    clear(body);
    let (stats, active) = compute(body);
    apply(body, &stats, 1);
    if let Ok(json) = serde_json::to_string(&stats) {
        body.temp_mut()
            .insert(APPLIED_STATS_KEY.into(), Value::String(json));
    }
    if let Ok(json) = serde_json::to_string(&active) {
        body.temp_mut()
            .insert(ACTIVE_EFFECTS_KEY.into(), Value::String(json));
    }
}

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn timed_effects(body: &Body) -> Vec<TimedEffect> {
    let raw = body.get_string(TIMED_EFFECTS_ATTR);
    if raw.is_empty() {
        return Vec::new();
    }
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_timed_effects(body: &mut Body, effects: &[TimedEffect]) {
    if effects.is_empty() {
        body.object.attr.remove(TIMED_EFFECTS_ATTR);
    } else if let Ok(json) = serde_json::to_string(effects) {
        body.set(TIMED_EFFECTS_ATTR, json);
    }
}

pub(crate) fn clear_timed(body: &mut Body) {
    let previous = body
        .temp_mut()
        .remove(APPLIED_TIMED_STATS_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|json| serde_json::from_str::<EffectStats>(&json).ok())
        .unwrap_or_default();
    apply(body, &previous, -1);
}

fn install_timed_effects(body: &mut Body, effects: &[TimedEffect]) {
    let mut total = EffectStats::default();
    for effect in effects {
        total.merge(&effect.effects);
    }
    apply(body, &total, 1);
    if total.is_empty() {
        body.temp_mut().remove(APPLIED_TIMED_STATS_KEY);
    } else if let Ok(json) = serde_json::to_string(&total) {
        body.temp_mut()
            .insert(APPLIED_TIMED_STATS_KEY.into(), Value::String(json));
    }
    write_timed_effects(body, effects);
}

fn refresh_timed_at(body: &mut Body, now: i64) -> Vec<String> {
    clear_timed(body);
    let mut active = Vec::new();
    let mut expired_scripts = Vec::new();
    for effect in timed_effects(body) {
        if effect.expires_at <= now {
            if !effect.expiration_script.is_empty() {
                expired_scripts.push(effect.expiration_script);
            }
        } else {
            active.push(effect);
        }
    }
    install_timed_effects(body, &active);
    expired_scripts
}

pub(crate) fn refresh_timed(body: &mut Body) {
    let _ = refresh_timed_at(body, now_timestamp());
}

pub(crate) fn expire_timed(body: &mut Body) -> Vec<String> {
    refresh_timed_at(body, now_timestamp())
}

pub(crate) fn apply_consumable_effect(
    body: &mut Body,
    item_key: &str,
) -> ConsumableEffectApplication {
    let Some(effect) = definition(item_key).and_then(|definition| definition.consumable) else {
        return ConsumableEffectApplication::default();
    };
    let now = now_timestamp();
    let _ = refresh_timed_at(body, now);
    clear_timed(body);
    let mut active = timed_effects(body);
    // The item index is the buff identity. Reuse replaces its old modifiers
    // and refreshes the timeout instead of allowing unbounded stat stacking.
    active.retain(|existing| existing.id != item_key);
    active.push(TimedEffect {
        id: item_key.to_string(),
        name: effect.name.clone(),
        expires_at: now.saturating_add(effect.duration_seconds),
        effects: effect.effects,
        expiration_script: effect.expiration_script,
    });
    install_timed_effects(body, &active);
    ConsumableEffectApplication {
        name: effect.name,
        duration_seconds: effect.duration_seconds,
        activation_script: effect.activation_script,
    }
}

pub(crate) fn apply_permanent_effect(
    body: &mut Body,
    item_key: &str,
) -> PermanentEffectApplication {
    let Some(definition) = definition(item_key) else {
        return PermanentEffectApplication::default();
    };
    let mut applied = Vec::new();
    for (name, amount) in definition.permanent.values {
        let target = match name.as_str() {
            "힘" | "민첩성" | "맷집" | "명중" | "회피" | "필살" | "운" => {
                name.as_str()
            }
            "내공" => "최고내공",
            "체력" => "최고체력",
            _ => continue,
        };
        if amount <= 0 {
            continue;
        }
        if matches!(name.as_str(), "명중" | "회피" | "필살" | "운") {
            let allocation_key = format!("{name}특성치");
            if !body.object.attr.contains_key(&allocation_key) {
                // Preserve the legacy allocation baseline before adding a
                // non-refundable permanent consumable increase.
                body.set(&allocation_key, body.get_int(&name));
            }
        }
        body.set(target, body.get_int(target).saturating_add(amount));
        applied.push((name, amount));
    }
    PermanentEffectApplication {
        effects: applied,
        activation_script: definition.permanent_script,
    }
}

pub(crate) fn permanent_effect_array(effects: Vec<(String, i64)>) -> Array {
    effects
        .into_iter()
        .map(|(name, amount)| {
            let mut effect = Map::new();
            effect.insert("name".into(), Dynamic::from(name));
            effect.insert("amount".into(), Dynamic::from(amount));
            Dynamic::from(effect)
        })
        .collect()
}

fn active_effects(body: &Body) -> Array {
    let active = body
        .temp()
        .get(ACTIVE_EFFECTS_KEY)
        .and_then(Value::as_str)
        .and_then(|json| serde_json::from_str::<Vec<ActiveEffect>>(json).ok())
        .unwrap_or_default();
    active
        .into_iter()
        .map(|effect| {
            let mut map = Map::new();
            map.insert("kind".into(), Dynamic::from(effect.kind));
            map.insert("name".into(), Dynamic::from(effect.name));
            map.insert("count".into(), Dynamic::from(effect.count));
            map.insert(
                "effects".into(),
                Dynamic::from(
                    effect
                        .effects
                        .values
                        .into_iter()
                        .map(|(name, amount)| {
                            let mut value = Map::new();
                            value.insert("name".into(), Dynamic::from(name));
                            value.insert("amount".into(), Dynamic::from(amount));
                            Dynamic::from(value)
                        })
                        .collect::<Array>(),
                ),
            );
            Dynamic::from(map)
        })
        .collect()
}

pub(crate) fn register_efuns(engine: &mut Engine, body_ptr: *mut Body) {
    let refresh_ptr = body_ptr;
    engine.register_fn("refresh_item_effects", move |_ob: &mut Map| -> Array {
        let body = unsafe { &mut *refresh_ptr };
        refresh(body);
        active_effects(body)
    });
    let active_ptr = body_ptr;
    engine.register_fn("get_active_item_effects", move |_ob: &mut Map| -> Array {
        active_effects(unsafe { &*active_ptr })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Object;
    use std::sync::Arc;

    struct TestItems(Vec<String>);

    impl TestItems {
        fn write(&mut self, key: &str, info: JsonValue) {
            std::fs::create_dir_all("data/item").unwrap();
            let path = format!("data/item/{key}.json");
            std::fs::write(
                &path,
                serde_json::to_string(&serde_json::json!({ "아이템정보": info })).unwrap(),
            )
            .unwrap();
            self.0.push(path);
        }
    }

    impl Drop for TestItems {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    fn equipped(body: &mut Body, key: &str) {
        let mut item = Object::new();
        item.set("인덱스", key);
        item.set("inUse", 1_i64);
        body.object.objs.push(Arc::new(Mutex::new(item)));
    }

    #[test]
    fn equipped_set_tiers_are_cumulative_and_possession_is_independent() {
        let suffix = format!("{}-set", std::process::id());
        let definition_key = format!("effect-definition-{suffix}");
        let second_key = format!("effect-second-{suffix}");
        let third_key = format!("effect-third-{suffix}");
        let fourth_key = format!("effect-fourth-{suffix}");
        let talisman_key = format!("effect-talisman-{suffix}");
        let group = format!("복면인세트-{suffix}");
        let mut files = TestItems(Vec::new());
        files.write(
            &definition_key,
            serde_json::json!({
                "이름": "복면인의 가면", "세트그룹": group,
                "세트조건": 3,
                "세트효과": { "3": ["민첩 +100", "힘 -20"], "4": { "명중": 50 } }
            }),
        );
        for key in [&second_key, &third_key, &fourth_key] {
            files.write(key, serde_json::json!({ "이름": key, "세트그룹": group }));
        }
        files.write(
            &talisman_key,
            serde_json::json!({
                "이름": "청룡부", "세트그룹": group, "소지효과": ["운 +7", "힘 +3"]
            }),
        );

        let mut body = Body::new();
        body._str = 50;
        body._dex = 10;
        equipped(&mut body, &definition_key);
        equipped(&mut body, &second_key);
        equipped(&mut body, &third_key);
        body.object.inv_stack.insert(talisman_key.clone(), 20);
        refresh(&mut body);
        assert_eq!(body._dex, 110);
        assert_eq!(body._str, 33); // set -20 plus possession +3
        assert_eq!(body._critical_chance, 7);
        assert_eq!(body._hit, 0);

        // The possessed talisman shares the group name but is not equipped,
        // so it never becomes the fourth set piece. Quantity does not stack.
        refresh(&mut body);
        assert_eq!(body._str, 33);
        assert_eq!(body._critical_chance, 7);
        equipped(&mut body, &fourth_key);
        refresh(&mut body);
        assert_eq!(body._hit, 50);

        body.object.inv_stack.remove(&talisman_key);
        refresh(&mut body);
        assert_eq!(body._critical_chance, 0);
        assert_eq!(body._str, 30);
    }

    #[test]
    fn set_requires_an_equipped_definition_item_and_counts_each_index_once() {
        let suffix = format!("{}-required", std::process::id());
        let definition_key = format!("effect-definition-{suffix}");
        let other_key = format!("effect-other-{suffix}");
        let group = format!("의천세트-{suffix}");
        let mut files = TestItems(Vec::new());
        files.write(
            &definition_key,
            serde_json::json!({
                "이름": "의천검", "세트그룹": group, "세트조건": 2,
                "세트효과": { "2": { "공격력": 40 } }
            }),
        );
        files.write(
            &other_key,
            serde_json::json!({ "이름": "검집", "세트그룹": group }),
        );

        let mut body = Body::new();
        equipped(&mut body, &other_key);
        equipped(&mut body, &other_key); // duplicate copies are one set piece
        refresh(&mut body);
        assert_eq!(body.attpower, 0);

        equipped(&mut body, &definition_key);
        refresh(&mut body);
        assert_eq!(body.attpower, 40);
        for item in &body.object.objs {
            if item.lock().unwrap().getString("인덱스") == definition_key {
                item.lock().unwrap().attr.remove("inUse");
            }
        }
        refresh(&mut body);
        assert_eq!(body.attpower, 0);
    }

    #[test]
    fn edited_item_effect_definition_reloads_without_restart() {
        let suffix = format!("{}-reload", std::process::id());
        let key = format!("effect-reload-{suffix}");
        let mut files = TestItems(Vec::new());
        files.write(
            &key,
            serde_json::json!({ "이름": "변화부", "소지효과": { "운": 4 } }),
        );
        let mut body = Body::new();
        body.object.inv_stack.insert(key.clone(), 1);
        refresh(&mut body);
        assert_eq!(body._critical_chance, 4);

        std::fs::write(
            format!("data/item/{key}.json"),
            serde_json::to_string(&serde_json::json!({
                "아이템정보": { "이름": "변화부", "소지효과": { "운": 123 } }
            }))
            .unwrap(),
        )
        .unwrap();
        refresh(&mut body);
        assert_eq!(body._critical_chance, 123);
    }

    #[test]
    fn timed_consumable_refreshes_without_stacking_and_expires_cleanly() {
        let suffix = format!("{}-timed", std::process::id());
        let key = format!("effect-timed-{suffix}");
        let mut files = TestItems(Vec::new());
        files.write(
            &key,
            serde_json::json!({
                "이름": "비호단",
                "사용효과": {
                    "이름": "비호강기", "지속시간": 60,
                    "효과": ["힘 +10", "방어력 +25"],
                    "발동스크립": "비호강기가 몸을 감쌉니다.",
                    "만료스크립": "비호강기가 흩어집니다."
                }
            }),
        );
        let mut body = Body::new();
        body._str = 5;
        body.armor = 3;
        let first = apply_consumable_effect(&mut body, &key);
        assert_eq!(first.name, "비호강기");
        assert_eq!(first.duration_seconds, 60);
        assert_eq!(body._str, 15);
        assert_eq!(body.armor, 28);

        let _ = apply_consumable_effect(&mut body, &key);
        assert_eq!(body._str, 15, "reusing must refresh, not stack");
        assert_eq!(timed_effects(&body).len(), 1);

        clear_timed(&mut body);
        let mut records = timed_effects(&body);
        records[0].expires_at = now_timestamp() - 1;
        install_timed_effects(&mut body, &records);
        let expired = expire_timed(&mut body);
        assert_eq!(expired, vec!["비호강기가 흩어집니다."]);
        assert_eq!(body._str, 5);
        assert_eq!(body.armor, 3);
        assert!(body.get_string(TIMED_EFFECTS_ATTR).is_empty());
    }

    #[test]
    fn rhai_use_and_eat_commands_consume_and_apply_timed_effect() {
        let suffix = format!("{}-command", std::process::id());
        let key = format!("effect-command-{suffix}");
        let player_name = format!("소모품버프시험-{suffix}");
        let save_path = format!("data/user/{player_name}.json");
        let mut files = TestItems(Vec::new());
        files.write(
            &key,
            serde_json::json!({
                "이름": "호심단", "반응이름": ["호심단"], "종류": "먹는것",
                "사용스크립": "$아이템$을 삼킵니다.",
                "사용효과": {
                    "이름": "호심진기", "지속시간": 30,
                    "효과": { "맷집": 12 },
                    "발동스크립": "$효과$가 $지속시간$초 동안 깃듭니다.",
                    "만료스크립": "호심진기가 사라집니다."
                },
                "영구효과": {
                    "효과": {
                        "힘": 1, "민첩성": 2, "맷집": 3,
                        "명중": 4, "회피": 5, "필살": 6, "운": 7,
                        "내공": 8, "체력": 90
                    },
                    "발동스크립": "$영구효과$의 공력이 몸에 새겨집니다."
                }
            }),
        );
        let mut body = Body::new();
        body.set("이름", player_name);
        body.object.inv_stack.insert(key.clone(), 2);
        let base_max_mp = body.get_int("최고내공");
        let base_max_hp = body.get_int("최고체력");
        let storage = crate::script::ScriptStorage::default();
        let output = storage
            .execute("사용", &mut body, "호심단", None, None, None)
            .unwrap()
            .0;
        assert_eq!(body._arm, 12);
        assert_eq!(body.object.inv_stack.get(&key), Some(&1));
        for (name, amount) in [
            ("힘", 1),
            ("민첩성", 2),
            ("맷집", 3),
            ("명중", 4),
            ("회피", 5),
            ("필살", 6),
            ("운", 7),
        ] {
            assert_eq!(body.get_int(name), amount);
        }
        assert_eq!(body.get_int("최고내공"), base_max_mp + 8);
        assert_eq!(body.get_int("최고체력"), base_max_hp + 90);
        assert!(output
            .iter()
            .any(|line| line.contains("호심진기가 30초 동안 깃듭니다.")));
        assert!(output.iter().any(|line| line.contains("힘 +1")));

        storage
            .execute("먹어", &mut body, "호심단", None, None, None)
            .unwrap();
        assert_eq!(body._arm, 12);
        assert!(!body.object.inv_stack.contains_key(&key));
        assert_eq!(timed_effects(&body).len(), 1);
        for (name, amount) in [
            ("힘", 2),
            ("민첩성", 4),
            ("맷집", 6),
            ("명중", 8),
            ("회피", 10),
            ("필살", 12),
            ("운", 14),
        ] {
            assert_eq!(body.get_int(name), amount);
        }
        assert_eq!(body.get_int("최고내공"), base_max_mp + 16);
        assert_eq!(body.get_int("최고체력"), base_max_hp + 180);
        let permanent_cannot_be_refunded = storage
            .execute("내려", &mut body, "명중", None, None, None)
            .unwrap();
        assert_eq!(
            permanent_cannot_be_refunded.0,
            vec!["☞ [명중] 더이상 내릴 수 없습니다."]
        );
        assert_eq!(body.get_int("명중"), 8);
        assert_eq!(body.get_int("특성치"), 0);
        let mut loaded = Body::new();
        assert!(crate::script::load_body_from_json(&mut loaded, &save_path));
        assert_eq!(loaded._arm, 12);
        assert_eq!(timed_effects(&loaded).len(), 1);
        assert_eq!(loaded.get_int("힘"), 2);
        assert_eq!(loaded.get_int("명중"), 8);
        assert_eq!(loaded.get_int("회피"), 10);
        assert_eq!(loaded.get_int("필살"), 12);
        assert_eq!(loaded.get_int("운"), 14);
        let _ = std::fs::remove_file(save_path);
    }
}
