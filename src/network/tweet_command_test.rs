use super::client::{handle_game_command, Client};
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

fn online_client(
    port: u16,
    name: &str,
    arm: i64,
) -> (Client, mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    let mut player = Player::new();
    player.state = STATE_ACTIVE;
    player.interactive = 1;
    player.body.set("이름", name);
    player.body.set("체력", 80_i64);
    player.body.set("최고체력", 100_i64);
    player.body.set("맷집", arm);
    player.body.set("내공", 20_i64);
    player.body.set("최고내공", 30_i64);
    player.body.set("설정상태", "엘피출력 0\n외침거부 0");
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
async fn tweet_recipient_prompt_uses_runtime_max_hp_not_saved_base() {
    let suffix = std::process::id();
    let zone = format!("트윗실시간존-{suffix}");
    let sender = format!("트윗망발신-{suffix}");
    let recipient = format!("트윗망수신-{suffix}");
    let sender_addr: SocketAddr = "127.0.0.1:18161".parse().unwrap();
    let (sender_client, mut sender_rx) = online_client(18161, &sender, 0);
    let (recipient_client, mut recipient_rx) = online_client(18162, &recipient, 10);
    let broadcaster = Arc::new(Broadcaster::new());
    broadcaster.add_client(sender_client);
    broadcaster.add_client(recipient_client);
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&sender, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&recipient, PlayerPosition::new(zone, "2".into()));
    }

    handle_game_command(
        &broadcaster,
        sender_addr,
        "실시간최대치 트윗",
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let _ = drain(&mut sender_rx);
    let wire = drain(&mut recipient_rx);
    assert!(wire.contains("실시간최대치"));
    assert!(
        wire.ends_with("\r\n\r\n\x1b[0;37;40m[ 80/400, 20/30 ] "),
        "Python lpPrompt uses getMaxHp = 최고체력 100 + 맷집 10 * 30: {wire:?}"
    );
    assert!(!wire.contains("[ 80/100, 20/30 ]"));

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&sender);
    world.remove_player_position(&recipient);
}
