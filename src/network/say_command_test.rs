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

fn room_client(port: u16, name: &str, hp: i64) -> (Client, mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    let mut player = Player::new();
    player.state = STATE_ACTIVE;
    player.interactive = 1;
    player.body.set("이름", name);
    player.body.set("체력", hp);
    player.body.set("최고체력", 100_i64);
    player.body.set("내공", 8_i64);
    player.body.set("최고내공", 10_i64);
    player.body.set("설정상태", "엘피출력 0");
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
async fn say_recipient_wire_is_send_line_then_lp_prompt_without_wrapper_crlf() {
    let suffix = std::process::id();
    let zone = format!("말망존-{suffix}");
    let speaker = format!("말망화자-{suffix}");
    let listener = format!("말망청자-{suffix}");
    let speaker_addr: SocketAddr = "127.0.0.1:18171".parse().unwrap();
    let (speaker_client, mut speaker_rx) = room_client(18171, &speaker, 90);
    let (listener_client, mut listener_rx) = room_client(18172, &listener, 70);
    let broadcaster = Arc::new(Broadcaster::new());
    broadcaster.add_client(speaker_client);
    broadcaster.add_client(listener_client);
    {
        let mut world = get_world_state().write().unwrap();
        world.set_player_position(&speaker, PlayerPosition::new(zone.clone(), "1".into()));
        world.set_player_position(&listener, PlayerPosition::new(zone, "1".into()));
    }

    handle_game_command(
        &broadcaster,
        speaker_addr,
        "안녕 말",
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let _ = drain(&mut speaker_rx);
    let wire = drain(&mut listener_rx);
    let room = format!("\x1b[33m{speaker}\x1b[37m가 말합니다 : '안녕\x1b[0;40;37m'");
    assert_eq!(
        wire,
        format!("{room}\r\n\r\n\x1b[0;37;40m[ 70/100, 8/10 ] ")
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&speaker);
    world.remove_player_position(&listener);
}
