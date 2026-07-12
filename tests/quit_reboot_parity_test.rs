use muc_engine::command::commands::{register_basic_commands, register_script_commands};
use muc_engine::command::{CommandRegistry, CommandResult};
use muc_engine::network::client::{Client, DISCONNECT_SENTINEL};
use muc_engine::network::Broadcaster;
use muc_engine::object::Object;
use muc_engine::player::{ActState, Body};
use muc_engine::script::{ScriptConfig, ScriptStorage};
use muc_engine::world::{RebootRoomUpdateBlock, RoomCache, WorldState};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};

fn repository_script_config() -> ScriptConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ScriptConfig {
        script_dir: root.join("cmds"),
        hot_reload: false,
        extension: ".rhai".to_string(),
        data_dir: root.join("data/config"),
        lib_dir: root.join("lib"),
    }
}

async fn repository_registry() -> CommandRegistry {
    let storage = Arc::new(RwLock::new(ScriptStorage::new(repository_script_config())));
    let mut registry = CommandRegistry::new();
    register_basic_commands(&mut registry);
    register_script_commands(&mut registry, storage, None, None, None).await;
    registry
}

#[tokio::test]
async fn quit_and_reboot_commands_are_python_backed_rhai_actions() {
    let registry = repository_registry().await;

    for invented in ["quit", "셧다운", "shutdown"] {
        assert!(registry.get(invented).is_none(), "{invented}");
    }

    let mut body = Body::new();
    body.act = ActState::Stand;
    for name in ["끝", "종료"] {
        let command = registry.get(name).expect("Rhai close command");
        assert!(matches!(
            (command.handler)(&mut body, &[]),
            CommandResult::Disconnect(ref message)
                if message == "\r\n다음에 또 만나요~!!!\r\n"
        ));
    }

    body.act = ActState::Fight;
    let close = registry.get("끝").unwrap();
    assert!(matches!(
        (close.handler)(&mut body, &[]),
        CommandResult::Output(ref message)
            if message == "☞ 지금은 무림을 떠나기에 좋은 상황이 아니네요. ^_^"
    ));
    body.act = ActState::Rest;
    assert!(matches!(
        (close.handler)(&mut body, &[]),
        CommandResult::Output(ref message)
            if message == "☞ 지금은 무림을 떠나기에 좋은 상황이 아니네요. ^_^"
    ));
    body.act = ActState::Stand;
    assert!(matches!(
        (close.handler)(&mut body, &["인수"]),
        CommandResult::Output(ref message)
            if message == "☞ 무슨 말인지 모르겠어요. *^_^*"
    ));

    let reboot = registry.get("리부팅").expect("Rhai reboot command");
    body.set("관리자등급", 999_i64);
    assert!(matches!(
        (reboot.handler)(&mut body, &[]),
        CommandResult::Output(ref message)
            if message == "☞ 무슨 말인지 모르겠어요. *^_^*"
    ));
    body.set("관리자등급", 1000_i64);
    assert!(matches!(
        (reboot.handler)(&mut body, &[]),
        CommandResult::Reboot
    ));
}

#[test]
fn connection_close_queue_preserves_rhai_text_before_transport_close() {
    let broadcaster = Broadcaster::new();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let addr: SocketAddr = "127.0.0.1:43101".parse().unwrap();
    broadcaster.add_client(Client::new(addr, tx));

    let text = "\r\n다음에 또 만나요~!!!\r\n";
    broadcaster.send_to(addr, text).unwrap();
    broadcaster.request_disconnect(addr).unwrap();

    assert_eq!(rx.try_recv().unwrap(), text);
    assert_eq!(rx.try_recv().unwrap(), DISCONNECT_SENTINEL);
}

fn unique_temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "muc_reboot_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn empty_room_world(label: &str) -> (WorldState, PathBuf) {
    let root = unique_temp_dir(label);
    let zone_dir = root.join("시험존");
    std::fs::create_dir_all(&zone_dir).unwrap();
    std::fs::write(
        zone_dir.join("1.json"),
        r#"{"맵정보":{"맵속성":[],"몹":[],"설명":[],"이름":"시험방","존이름":"시험존","출구":[]}}"#,
    )
    .unwrap();

    let mut world = WorldState::new();
    world.room_cache = RoomCache::with_data_dir(&root);
    world.room_cache.get_room("시험존", "1").unwrap();
    (world, root)
}

#[test]
fn reboot_updates_a_representable_loaded_room_without_stopping_a_server() {
    let (mut world, root) = empty_room_world("safe");
    let room = world.room_cache.get_room_cached("시험존", "1").unwrap();
    assert_eq!(room.read().unwrap().last_update_millis, 0);

    world.update_loaded_rooms_before_reboot().unwrap();
    assert!(room.read().unwrap().last_update_millis > 0);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reboot_preflight_blocks_unrepresented_floor_item_lifetimes_without_partial_update() {
    let (mut world, root) = empty_room_world("floor_item");
    world
        .get_room_objs_mut("시험존", "1")
        .push(Arc::new(Mutex::new(Object::new())));

    assert_eq!(
        world.update_loaded_rooms_before_reboot(),
        Err(RebootRoomUpdateBlock::FloorItems {
            zone: "시험존".to_string(),
            room: "1".to_string(),
        })
    );
    assert_eq!(
        world
            .room_cache
            .get_room_cached("시험존", "1")
            .unwrap()
            .read()
            .unwrap()
            .last_update_millis,
        0
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn implementation_is_anchored_to_the_python_runtime_sources() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let player = std::fs::read_to_string(root.join("objs/player.py")).unwrap();
    let reboot = std::fs::read_to_string(root.join("cmds/리부팅.py")).unwrap();

    assert!(player.contains("if cmd in ('끝', '종료') and argc == 1:"));
    assert!(player.contains("if self.isMovable() == False:"));
    assert!(player.contains("self.INTERACTIVE = 2"));
    assert!(player.contains("self.sendLine('\\r\\n다음에 또 만나요~!!!')"));
    assert!(player.contains("self.channel.transport.loseConnection()"));

    assert!(reboot.contains("if getInt(ob['관리자등급']) < 1000:"));
    assert!(reboot.contains("self.updateZones()"));
    assert!(reboot.contains("reactor.stop()"));
}
