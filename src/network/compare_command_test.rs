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

fn combat_client(port: u16, name: &str, active: bool) -> (Client, mpsc::UnboundedReceiver<String>) {
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
    player.body.set("레벨", 10_i64);
    player.body.set("힘", 50_i64);
    player.body.set("최고체력", 500_i64);
    player.body.set("체력", 500_i64);
    player.body.set("최고내공", 100_i64);
    player.body.set("내공", 100_i64);
    player.body.set("맷집", 5_i64);
    player.body.set("설정상태", "비교거부 0");
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
async fn compare_network_snapshot_keeps_same_room_inactive_player_target() {
    let suffix = std::process::id();
    let zone = format!("비교비활성존-{suffix}");
    let viewer = format!("비교망조회자-{suffix}");
    let inactive = format!("비교망비활성-{suffix}");
    let viewer_addr: SocketAddr = "127.0.0.1:18151".parse().unwrap();
    let (viewer_client, mut viewer_rx) = combat_client(18151, &viewer, true);
    let (inactive_client, _inactive_rx) = combat_client(18152, &inactive, false);
    let broadcaster = Arc::new(Broadcaster::new());
    broadcaster.add_client(viewer_client);
    broadcaster.add_client(inactive_client);
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&viewer, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&inactive, PlayerPosition::new(zone.clone(), "1".into()));
    }

    handle_game_command(
        &broadcaster,
        viewer_addr,
        &format!("{inactive} 비교"),
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let output = drain(&mut viewer_rx);
    assert!(
        output.contains(&format!("▶ \x1b[1m{inactive}\x1b[0;37m"))
            && output.contains("의 상대비교"),
        "Python Room.findObjName does not apply an ACTIVE guard: {output:?}"
    );
    assert!(output.contains("☞ 당신의 승률 오차ː"));
    assert!(output.contains("☞ 상대의 승률 오차ː"));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&viewer);
    world.remove_player_position(&inactive);
}
