use super::client::{handle_pending_change_password, Client};
use crate::command::handler::PendingInput;
use crate::network::Broadcaster;
use crate::player::Player;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;

fn file_edit_client(
    port: u16,
    relative_path: String,
) -> (
    Arc<Broadcaster>,
    SocketAddr,
    mpsc::UnboundedReceiver<String>,
) {
    let broadcaster = Arc::new(Broadcaster::new());
    let (tx, rx) = mpsc::unbounded_channel();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    let mut player = Player::new();
    player.body.set("이름", format!("파일편집시험-{port}"));
    client.player = Some(player);
    client.pending_input = Some(PendingInput::FileEdit {
        relative_path,
        lines: Vec::new(),
    });
    broadcaster.add_client(client);
    (broadcaster, addr, rx)
}

#[tokio::test]
async fn file_edit_matches_python_empty_accumulator_and_exact_output() {
    let suffix = format!("{}_{}", std::process::id(), 18121);
    let dir = std::path::Path::new("data/mob").join(&suffix);
    std::fs::create_dir_all(&dir).unwrap();
    let relative = format!("mob/{suffix}/시험.json");
    let path = std::path::Path::new("data").join(&relative);
    let (server, addr, mut rx) = file_edit_client(18121, relative);

    handle_pending_change_password(&server, addr, "")
        .await
        .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "\r\n:");
    handle_pending_change_password(&server, addr, "첫줄")
        .await
        .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "첫줄\r\n:");
    handle_pending_change_password(&server, addr, "둘째줄")
        .await
        .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "둘째줄\r\n:");
    handle_pending_change_password(&server, addr, ".")
        .await
        .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "작성을 마칩니다.\r\n");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "첫줄\n둘째줄");
    assert!(server
        .clients
        .lock()
        .get(&addr)
        .unwrap()
        .pending_input
        .is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn file_edit_missing_parent_silently_keeps_python_input_callback() {
    let suffix = format!("없는편집존-{}-18122", std::process::id());
    let relative = format!("mob/{suffix}/시험.json");
    let path = std::path::Path::new("data").join(&relative);
    let _ = std::fs::remove_dir_all(std::path::Path::new("data/mob").join(&suffix));
    let (server, addr, mut rx) = file_edit_client(18122, relative.clone());

    handle_pending_change_password(&server, addr, "내용")
        .await
        .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "내용\r\n:");
    handle_pending_change_password(&server, addr, ".")
        .await
        .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "");
    assert!(!path.exists());
    assert!(matches!(
        server.clients.lock().get(&addr).unwrap().pending_input.as_ref(),
        Some(PendingInput::FileEdit { relative_path, lines })
            if relative_path == &relative && lines == &["내용".to_string()]
    ));
}
