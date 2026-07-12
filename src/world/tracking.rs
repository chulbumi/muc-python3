//! Read-only mob tracking support for the administrator `추적` command.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use serde_json::Value as JsonValue;

use super::{get_world_state, WorldState};

/// Result data for Rhai. User-facing text is intentionally left to the command script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobTrackingResult {
    pub zone_exists: bool,
    pub room: Option<String>,
}

/// Find the first room containing a placed mob whose `이름` exactly matches.
///
/// Python constructs `Room.Zones[zone]` while loading each mob's `위치` list. To preserve
/// that first-room order, initial placement rooms are traversed in mob-file/location order;
/// remaining map rooms are appended so the complete zone is still searched.
pub fn find_mob_room(zone: &str, mob_name: &str) -> MobTrackingResult {
    let world = match get_world_state().read() {
        Ok(world) => world,
        Err(_) => {
            let (rooms, _) = initial_mob_placements(Path::new("data/mob"), zone);
            return MobTrackingResult {
                zone_exists: !rooms.is_empty(),
                room: None,
            };
        }
    };
    find_mob_room_in(
        &world,
        Path::new("data/map"),
        Path::new("data/mob"),
        zone,
        mob_name,
    )
}

fn find_mob_room_in(
    world: &WorldState,
    map_root: &Path,
    mob_root: &Path,
    zone: &str,
    mob_name: &str,
) -> MobTrackingResult {
    let (initial_rooms, initial_mobs) = initial_mob_placements(mob_root, zone);
    let Some(rooms) = ordered_zone_rooms(world, map_root, zone, &initial_rooms) else {
        return MobTrackingResult {
            zone_exists: false,
            room: None,
        };
    };

    let room = rooms
        .into_iter()
        .find(|room| room_has_mob(world, zone, room, mob_name, &initial_mobs));
    MobTrackingResult {
        zone_exists: true,
        room,
    }
}

fn room_has_mob(
    world: &WorldState,
    zone: &str,
    room: &str,
    mob_name: &str,
    initial_mobs: &HashMap<String, Vec<String>>,
) -> bool {
    if world
        .mob_cache
        .get_all_mobs_in_room(zone, room)
        .into_iter()
        .any(|mob| mob.name == mob_name)
    {
        return true;
    }

    // Once a room has runtime instance state, moved/removed mobs must not be rediscovered from
    // static placement. Uninstantiated rooms use initial placement data because Rust loads them
    // on demand whereas Python places all mobs during startup.
    if world.mob_cache.has_room_instance_state(zone, room) {
        return false;
    }

    initial_mobs
        .get(room)
        .map(|names| names.iter().any(|name| name == mob_name))
        .unwrap_or(false)
}

fn ordered_zone_rooms(
    world: &WorldState,
    map_root: &Path,
    zone: &str,
    initial_rooms: &[String],
) -> Option<Vec<String>> {
    let loaded_rooms = world.room_cache.loaded_room_names_in_zone(zone);
    if initial_rooms.is_empty() && loaded_rooms.is_empty() {
        return None;
    }

    let map_zone = zone_dir(map_root, zone);
    let mut rooms = Vec::new();
    let mut seen = HashSet::new();

    // mob.place() creates the zone before attempting Room.create(), but only successfully
    // created rooms enter Room.Zones[zone].
    if let Some(map_zone) = map_zone {
        for room in initial_rooms {
            if room_file_loadable(&map_zone, room) && seen.insert(room.clone()) {
                rooms.push(room.clone());
            }
        }
    }
    for room in loaded_rooms {
        if seen.insert(room.clone()) {
            rooms.push(room);
        }
    }

    Some(rooms)
}

fn initial_mob_placements(
    mob_root: &Path,
    zone: &str,
) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let Some(zone_path) = zone_dir(mob_root, zone) else {
        return (Vec::new(), HashMap::new());
    };
    let Ok(entries) = std::fs::read_dir(zone_path) else {
        return (Vec::new(), HashMap::new());
    };
    let mut rooms = Vec::new();
    let mut seen = HashSet::new();
    let mut placements: HashMap<String, Vec<String>> = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(info) = read_json(&path)
            .and_then(|json| json.get("몹정보").cloned())
            .and_then(|info| info.as_object().cloned())
        else {
            continue;
        };
        let name = info
            .get("이름")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let locations = info
            .get("위치")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        for room in locations.into_iter().filter_map(json_key_string) {
            if seen.insert(room.clone()) {
                rooms.push(room.clone());
            }
            placements.entry(room).or_default().push(name.to_string());
        }
    }
    (rooms, placements)
}

fn room_file_loadable(zone_dir: &Path, room: &str) -> bool {
    json_file(zone_dir, room)
        .and_then(|path| read_json(&path))
        .and_then(|json| json.get("맵정보").cloned())
        .is_some()
}

fn read_json(path: &Path) -> Option<JsonValue> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn json_key_string(value: JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value),
        JsonValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn zone_dir(root: &Path, zone: &str) -> Option<PathBuf> {
    if !single_normal_component(zone) {
        return None;
    }
    let root = std::fs::canonicalize(root).ok()?;
    let child = std::fs::canonicalize(root.join(zone)).ok()?;
    (child.starts_with(&root) && child.is_dir()).then_some(child)
}

fn json_file(dir: &Path, stem: &str) -> Option<PathBuf> {
    if !single_normal_component(stem) {
        return None;
    }
    let path = dir.join(format!("{stem}.json"));
    let canonical = std::fs::canonicalize(path).ok()?;
    (canonical.starts_with(dir) && canonical.is_file()).then_some(canonical)
}

fn single_normal_component(value: &str) -> bool {
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{MobInstance, RawMobData};

    fn temp_roots() -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "muc_tracking_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let map_root = root.join("map");
        let mob_root = root.join("mob");
        std::fs::create_dir_all(map_root.join("시험존")).unwrap();
        std::fs::create_dir_all(mob_root.join("시험존")).unwrap();
        (root, map_root, mob_root)
    }

    fn write_room(map_root: &Path, room: &str, mobs: &[&str]) {
        let value = serde_json::json!({"맵정보": {"이름": room, "존이름": "시험존", "몹": mobs}});
        std::fs::write(
            map_root.join("시험존").join(format!("{room}.json")),
            serde_json::to_string(&value).unwrap(),
        )
        .unwrap();
    }

    fn write_mob(mob_root: &Path, id: &str, name: &str, rooms: &[&str]) {
        let value = serde_json::json!({"몹정보": {"이름": name, "위치": rooms}});
        std::fs::write(
            mob_root.join("시험존").join(format!("{id}.json")),
            serde_json::to_string(&value).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn follows_python_mob_placement_room_order() {
        let (root, map_root, mob_root) = temp_roots();
        write_room(&map_root, "1", &["대상"]);
        write_room(&map_root, "2", &["대상"]);
        write_room(&map_root, "3", &[]);
        write_mob(&mob_root, "대상", "추적대상", &["2", "1"]);

        let world = WorldState::new();
        let (initial_rooms, _) = initial_mob_placements(&mob_root, "시험존");
        assert_eq!(
            ordered_zone_rooms(&world, &map_root, "시험존", &initial_rooms)
                .unwrap()
                .into_iter()
                .take(2)
                .collect::<Vec<_>>(),
            vec!["2", "1"]
        );
        assert_eq!(
            find_mob_room_in(&world, &map_root, &mob_root, "시험존", "추적대상").room,
            Some("2".to_string())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn finds_dead_runtime_mob_that_remains_placed_in_the_room() {
        let (root, map_root, mob_root) = temp_roots();
        write_room(&map_root, "1", &["대상"]);
        write_mob(&mob_root, "대상", "추적대상", &["1"]);

        let mut world = WorldState::new();
        let mut data = RawMobData::new();
        data.name = "추적대상".to_string();
        let mut instance =
            MobInstance::new("시험존:대상".to_string(), "시험존".to_string(), "1", &data);
        instance.alive = false;
        world.mob_cache.add_mob_instance(instance);

        assert_eq!(
            find_mob_room_in(&world, &map_root, &mob_root, "시험존", "추적대상"),
            MobTrackingResult {
                zone_exists: true,
                room: Some("1".to_string())
            }
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn map_directory_alone_does_not_create_python_room_zone_state() {
        let (root, map_root, mob_root) = temp_roots();
        write_room(&map_root, "1", &[]);

        assert_eq!(
            find_mob_room_in(&WorldState::new(), &map_root, &mob_root, "없는존", "대상"),
            MobTrackingResult {
                zone_exists: false,
                room: None
            }
        );
        assert_eq!(
            find_mob_room_in(&WorldState::new(), &map_root, &mob_root, "시험존", "대상"),
            MobTrackingResult {
                zone_exists: false,
                room: None
            }
        );

        write_mob(&mob_root, "다른몹", "다른몹", &["1"]);
        assert_eq!(
            find_mob_room_in(&WorldState::new(), &map_root, &mob_root, "시험존", "대상"),
            MobTrackingResult {
                zone_exists: true,
                room: None
            }
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn successfully_loaded_room_creates_zone_without_initial_mob_placement() {
        let (root, map_root, mob_root) = temp_roots();
        write_room(&map_root, "1", &[]);

        let mut world = WorldState::new();
        world.room_cache = crate::world::RoomCache::with_data_dir(&map_root);
        world.room_cache.get_room("시험존", "1").unwrap();
        assert_eq!(
            find_mob_room_in(&world, &map_root, &mob_root, "시험존", "대상"),
            MobTrackingResult {
                zone_exists: true,
                room: None
            }
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
