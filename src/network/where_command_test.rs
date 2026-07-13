use super::client::{handle_game_command, Client, ClientState};
use crate::command::commands::{register_basic_commands, script::register_script_commands};
use crate::command::registry::CommandRegistry;
use crate::network::Broadcaster;
use crate::player::{Player, STATE_ACTIVE};
use crate::script::{ScriptConfig, ScriptStorage};
use crate::world::{get_world_state, PlayerPosition, RoomCache};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};

async fn script_registry() -> Arc<CommandRegistry> {
    let storage = Arc::new(RwLock::new(ScriptStorage::new(ScriptConfig::default())));
    let mut registry = CommandRegistry::new();
    register_basic_commands(&mut registry);
    register_script_commands(&mut registry, storage, None, None, None).await;
    Arc::new(registry)
}

fn player_client(
    port: u16,
    name: &str,
    active: bool,
) -> (Client, mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    if !active {
        client.state = ClientState::Inactive;
    }
    let mut player = Player::new();
    player.state = STATE_ACTIVE;
    player.interactive = 1;
    player.body.set("이름", name);
    player.body.set("체력", 10_i64);
    player.body.set("최고체력", 20_i64);
    player.body.set("내공", 3_i64);
    player.body.set("최고내공", 4_i64);
    client.player = Some(player);
    (client, rx)
}

fn drain(receiver: &mut mpsc::UnboundedReceiver<String>) -> String {
    let mut output = String::new();
    while let Ok(message) = receiver.try_recv() {
        output.push_str(&message);
    }
    output
}

#[tokio::test]
async fn where_network_snapshot_keeps_inactive_only_for_same_zone_listing() {
    let suffix = std::process::id();
    let zone = format!("어디비활성존-{suffix}");
    let directory = std::path::Path::new("data/map").join(&zone);
    std::fs::create_dir_all(&directory).unwrap();
    for (room, name) in [("1", "조회방"), ("2", "비활성방")] {
        std::fs::write(
            directory.join(format!("{room}.json")),
            serde_json::json!({"맵정보":{"이름":name,"존이름":zone,"설명":[],"출구":[]}})
                .to_string(),
        )
        .unwrap();
    }

    let viewer = format!("어디망조회자-{suffix}");
    let inactive = format!("어디망비활성-{suffix}");
    let viewer_addr: SocketAddr = "127.0.0.1:18131".parse().unwrap();
    let (viewer_client, mut viewer_rx) = player_client(18131, &viewer, true);
    let (inactive_client, _inactive_rx) = player_client(18132, &inactive, false);
    let broadcaster = Arc::new(Broadcaster::new());
    broadcaster.add_client(viewer_client);
    broadcaster.add_client(inactive_client);
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room(&zone, "1").unwrap();
        world.room_cache.get_room(&zone, "2").unwrap();
        world.set_player_position(&viewer, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&inactive, PlayerPosition::new(zone.clone(), "2".into()));
    }
    let registry = script_registry().await;
    let room_cache = Arc::new(Mutex::new(RoomCache::new()));

    handle_game_command(
        &broadcaster,
        viewer_addr,
        "어디",
        registry.clone(),
        room_cache.clone(),
        None,
    )
    .await
    .unwrap();
    let listed = drain(&mut viewer_rx);
    assert!(
        listed.contains(&format!("\x1b[1m{inactive}")),
        "Python's no-argument branch lists an env-bound inactive Player: {listed:?}"
    );
    assert!(listed.contains("▷ 비활성방"));

    handle_game_command(
        &broadcaster,
        viewer_addr,
        &format!("{inactive} 어디"),
        registry,
        room_cache,
        None,
    )
    .await
    .unwrap();
    let named = drain(&mut viewer_rx);
    assert!(named.contains("☞ 활동중인 그런 무림인이 없어요. ^^"));
    assert!(!named.contains("▷ 비활성방"));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&viewer);
    world.remove_player_position(&inactive);
    drop(world);
    let _ = std::fs::remove_dir_all(directory);
}
