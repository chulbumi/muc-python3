use super::client::{handle_game_command, Client};
use crate::command::commands::{register_basic_commands, script::register_script_commands};
use crate::command::registry::CommandRegistry;
use crate::hangul::han_iga;
use crate::network::Broadcaster;
use crate::object::Object;
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

fn shop_client(
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
async fn shop_recipient_wire_matches_python_send_fight_script_room() {
    let suffix = std::process::id();
    let buyer = format!("구입망구매자-{suffix}");
    let listener = format!("구입망관찰자-{suffix}");
    let buyer_addr: SocketAddr = "127.0.0.1:18201".parse().unwrap();
    let (mut buyer_client, mut buyer_rx) = shop_client(18201, &buyer, 90);
    buyer_client.player.as_mut().unwrap().body.set("은전", 100_i64);
    buyer_client.player.as_mut().unwrap().body.set("힘", 100_i64);
    let (listener_client, mut listener_rx) = shop_client(18202, &listener, 70);
    let broadcaster = Arc::new(Broadcaster::new());
    broadcaster.add_client(buyer_client);
    broadcaster.add_client(listener_client);
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room("낙양성", "6").unwrap();
        world.spawn_mobs_for_room("낙양성", "6");
        world.set_player_position(
            &buyer,
            PlayerPosition::new("낙양성".into(), "6".into()),
        );
        world.set_player_position(
            &listener,
            PlayerPosition::new("낙양성".into(), "6".into()),
        );
    }

    handle_game_command(
        &broadcaster,
        buyer_addr,
        "수박모자 구입",
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let buyer_wire = drain(&mut buyer_rx);
    let wire = drain(&mut listener_rx);
    assert_eq!(
        wire,
        format!(
            "\r\n\x1b[1m{buyer}\x1b[0;37m{} \x1b[0;36m수박모자\x1b[37m 1개를 은전 9개에 구입합니다.\r\n\r\n\x1b[0;37;40m[ 70/100, 8/10 ] ",
            han_iga(&buyer),
        ),
        "buyer wire: {buyer_wire:?}",
    );

    {
        let mut clients = broadcaster.clients.lock();
        let buyer_body = &mut clients
            .get_mut(&buyer_addr)
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .body;
        let mut item = Object::new();
        item.set("이름", "자주판매품");
        item.set("안시", "\x1b[1;35m");
        item.set("판매가격", 100_i64);
        buyer_body
            .object
            .objs
            .push(Arc::new(Mutex::new(item)));
    }
    {
        let mut world = get_world_state().write().unwrap();
        world.room_cache.get_room("낙양성", "43").unwrap();
        world.spawn_mobs_for_room("낙양성", "43");
        world.set_player_position(
            &buyer,
            PlayerPosition::new("낙양성".into(), "43".into()),
        );
        world.set_player_position(
            &listener,
            PlayerPosition::new("낙양성".into(), "43".into()),
        );
    }
    handle_game_command(
        &broadcaster,
        buyer_addr,
        "자주판매품 판매",
        script_registry().await,
        Arc::new(Mutex::new(RoomCache::new())),
        None,
    )
    .await
    .unwrap();
    let buyer_wire = drain(&mut buyer_rx);
    let wire = drain(&mut listener_rx);
    assert_eq!(
        wire,
        format!(
            "\r\n\x1b[1m{buyer}\x1b[0;37m{} \x1b[1;35m자주판매품\x1b[0;37m 1개를 은전 40개에 판매합니다.\r\n\r\n\x1b[0;37;40m[ 70/100, 8/10 ] ",
            han_iga(&buyer),
        ),
        "seller wire: {buyer_wire:?}",
    );

    let mut world = get_world_state().write().unwrap();
    world.remove_player_position(&buyer);
    world.remove_player_position(&listener);
    let _ = std::fs::remove_file(format!("data/user/{buyer}.json"));
}
