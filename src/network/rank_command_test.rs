use super::client::{handle_game_command, Client, ClientState};
use crate::command::commands::{register_basic_commands, script::register_script_commands};
use crate::command::registry::CommandRegistry;
use crate::network::Broadcaster;
use crate::player::{Player, STATE_ACTIVE};
use crate::script::{ScriptConfig, ScriptStorage};
use crate::world::RoomCache;
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

fn ranked_client(
    port: u16,
    name: &str,
    strength: i64,
    active: bool,
    transparent: bool,
    silver: i64,
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
    player.body.set("힘", strength);
    player.body.set("투명상태", i64::from(transparent));
    player.body.set("은전", silver);
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
async fn rank_network_snapshot_includes_inactive_and_transparent_channel_players() {
    let suffix = std::process::id();
    let viewer = format!("순위망조회자-{suffix}");
    let inactive = format!("순위망비활성-{suffix}");
    let hidden = format!("순위망투명-{suffix}");
    let ordinary = format!("순위망일반-{suffix}");
    let viewer_addr: SocketAddr = "127.0.0.1:18141".parse().unwrap();
    let (viewer_client, mut viewer_rx) =
        ranked_client(18141, &viewer, 0, true, false, 100_000);
    let (inactive_client, _inactive_rx) =
        ranked_client(18142, &inactive, 40, false, false, 0);
    let (hidden_client, _hidden_rx) = ranked_client(18143, &hidden, 30, true, true, 0);
    let (ordinary_client, _ordinary_rx) =
        ranked_client(18144, &ordinary, 20, true, false, 0);
    let broadcaster = Arc::new(Broadcaster::new());
    for client in [viewer_client, inactive_client, hidden_client, ordinary_client] {
        broadcaster.add_client(client);
    }

    handle_game_command(
        &broadcaster,
        viewer_addr,
        "힘 순위",
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let output = drain(&mut viewer_rx);
    let expected = format!(
        "[01] {:<10} [02] {:<10} [03] {:<10} ",
        inactive, hidden, ordinary
    );
    assert!(
        output.contains(&expected),
        "Python includes inactive and transparent channel Players in stable value order: {output:?}"
    );
    assert_eq!(
        broadcaster.clients.lock()[&viewer_addr]
            .player
            .as_ref()
            .unwrap()
            .body
            .get_int("은전"),
        0
    );

    // Python administrators can rank an arbitrary numeric Body attribute.
    // This must come from the requested-key snapshot rather than a fixed list.
    {
        let mut clients = broadcaster.clients.lock();
        let viewer_body = &mut clients
            .get_mut(&viewer_addr)
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .body;
        viewer_body.set("관리자등급", 1000_i64);
        viewer_body.set("은전", 100_000_i64);
        for client in clients.values_mut() {
            let Some(player) = client.player.as_mut() else {
                continue;
            };
            if player.body.get_name() == ordinary {
                player.body.set("비밀수치", 77_i64);
            }
        }
    }
    handle_game_command(
        &broadcaster,
        viewer_addr,
        "비밀수치 순위",
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let custom = drain(&mut viewer_rx);
    assert!(
        custom.contains(&ordinary) && custom.contains("77"),
        "Python admin rank reads arbitrary numeric attributes: {custom:?}"
    );
}
