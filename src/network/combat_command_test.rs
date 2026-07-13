use super::client::{handle_game_command, Client};
use crate::command::commands::{register_basic_commands, script::register_script_commands};
use crate::command::registry::CommandRegistry;
use crate::hangul::han_obj;
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

fn combat_client(
    port: u16,
    name: &str,
    hp: i64,
) -> (Client, mpsc::UnboundedReceiver<String>) {
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
    player.body.set("설정상태", "엘피출력 0 타인전투출력거부 0");
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
async fn pvp_attack_target_and_observer_receive_exact_send_line_prompt_wire() {
    let suffix = std::process::id();
    let zone = format!("비무망존-{suffix}");
    let attacker = format!("비무망공격자-{suffix}");
    let target = format!("비무망대상-{suffix}");
    let observer = format!("비무망관전자-{suffix}");
    let attacker_addr: SocketAddr = "127.0.0.1:18211".parse().unwrap();
    let (attacker_client, mut attacker_rx) = combat_client(18211, &attacker, 90);
    let (target_client, mut target_rx) = combat_client(18212, &target, 70);
    let (observer_client, mut observer_rx) = combat_client(18213, &observer, 60);
    let broadcaster = Arc::new(Broadcaster::new());
    broadcaster.add_client(attacker_client);
    broadcaster.add_client(target_client);
    broadcaster.add_client(observer_client);
    {
        let mut world = get_world_state().write().unwrap();
        for name in [&attacker, &target, &observer] {
            world.set_player_position(name, PlayerPosition::new(zone.clone(), "1".into()));
        }
    }

    handle_game_command(
        &broadcaster,
        attacker_addr,
        &format!("{target} 쳐"),
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let attacker_wire = drain(&mut attacker_rx);
    assert!(attacker_wire.starts_with("당신이 주먹을 쥐며 공격 합니다.\r\n"));
    assert_eq!(
        drain(&mut target_rx),
        format!(
            "\r\n\x1b[1m{attacker}\x1b[0;37m가 당신을 공격하기 시작합니다.\r\n\r\n\x1b[0;37;40m[ 70/100, 8/10 ] "
        )
    );
    assert_eq!(
        drain(&mut observer_rx),
        format!(
            "\r\n\x1b[1m{attacker}\x1b[0;37m가 {target}{} 공격하기 시작합니다.\r\n\r\n\x1b[0;37;40m[ 60/100, 8/10 ] ",
            han_obj(&target),
        )
    );

    let mut world = get_world_state().write().unwrap();
    for name in [&attacker, &target, &observer] {
        world.remove_player_position(name);
    }
}
