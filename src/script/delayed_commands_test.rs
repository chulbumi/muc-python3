use super::*;

#[test]
fn delayed_input_empty_command_matches_python_usage() {
    let mut body = Body::new();
    let result = ScriptStorage::default()
        .execute("지연입력", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec!["☞ 사용법: [입력글] 지연입력"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fishing_delayed_callbacks_clear_python_cooltime_and_send_exact_lines() {
    use crate::network::{Broadcaster, Client};
    use crate::player::{Player, STATE_ACTIVE};
    use tokio::sync::mpsc;

    let storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::default()));
    let broadcaster = Arc::new(Broadcaster::new());
    let scheduler = Arc::new(CallOutScheduler::default_resolution(broadcaster.clone()));
    let mut command_body = Body::new();
    command_body.set("이름", "낚시예약회귀");
    let command = storage
        .read()
        .await
        .execute(
            "낚시",
            &mut command_body,
            "무시",
            None,
            None,
            Some(scheduler.clone()),
        )
        .unwrap();
    assert_eq!(
        command.0,
        vec!["낚시바늘에 미끼를 끼우고 낚시대를 드리웁니다."]
    );
    let repeated = storage
        .read()
        .await
        .execute(
            "낚시",
            &mut command_body,
            "",
            None,
            None,
            Some(scheduler.clone()),
        )
        .unwrap();
    assert_eq!(repeated.0, command.0);
    assert_eq!(scheduler.pending_count(), 2);
    assert!(
        scheduler.process_due().is_empty(),
        "3-second callbacks must not run immediately"
    );
    assert!(scheduler.remove_call_out_by_name("낚시예약회귀", "fishing_2"));
    assert_eq!(scheduler.pending_count(), 0);

    let addr = "127.0.0.1:18054".parse().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    let old_connection_token = client.connection_token.clone();
    let mut player = Player::new();
    player.state = STATE_ACTIVE;
    player.body.set("이름", "낚시회귀");
    player.body.set("cooltime", 9_i64);
    player.body.temp_mut().insert(
        "_connection_token".to_string(),
        Value::String(old_connection_token.clone()),
    );
    client.player = Some(player);
    broadcaster.add_client(client);

    let runner = create_call_out_script_runner_with_scheduler(
        storage,
        broadcaster.clone(),
        Some(scheduler.clone()),
    );
    scheduler.set_script_runner(runner.clone());
    scheduler.call_out(
        &old_connection_token,
        "fishing_2",
        std::time::Duration::ZERO,
        vec![],
        Some("낚시".to_string()),
    );
    assert!(scheduler.process_due().iter().all(|result| result.success));
    assert_eq!(
        rx.try_recv().unwrap(),
        "낚시줄에 엄청난것이 걸린것 같다...\r\n"
    );
    assert_eq!(
        broadcaster.with_player_body_by_name("낚시회귀", |body| body.get_int("cooltime")),
        Some(0)
    );
    assert_eq!(scheduler.pending_count(), 1);
    assert!(scheduler.remove_call_out_by_name(&old_connection_token, "fishing_3"));

    broadcaster.with_player_body_by_name("낚시회귀", |body| {
        body.set("cooltime", 7_i64);
    });
    scheduler.call_out(
        &old_connection_token,
        "fishing_3",
        std::time::Duration::ZERO,
        vec![],
        Some("낚시".to_string()),
    );
    assert!(scheduler.process_due().iter().all(|result| result.success));
    assert_eq!(rx.try_recv().unwrap(), "젠장! 낚시줄이 끊어졌다.\r\n");
    assert_eq!(
        broadcaster.with_player_body_by_name("낚시회귀", |body| body.get_int("cooltime")),
        Some(0)
    );

    // Python callLater captures the old Player object. Reconnecting with
    // the same character name must not receive the old object's callback.
    broadcaster.remove_client(addr);
    let new_addr = "127.0.0.1:18057".parse().unwrap();
    let (new_tx, mut new_rx) = mpsc::unbounded_channel();
    let mut replacement = Client::new(new_addr, new_tx);
    replacement.complete_login();
    let mut replacement_player = Player::new();
    replacement_player.state = STATE_ACTIVE;
    replacement_player.body.set("이름", "낚시회귀");
    replacement_player.body.set("cooltime", 77_i64);
    replacement.player = Some(replacement_player);
    broadcaster.add_client(replacement);
    assert!(runner(&old_connection_token, Some("낚시"), "fishing_3", vec![]).is_err());
    assert!(new_rx.try_recv().is_err());
    assert_eq!(
        broadcaster.with_player_body_by_name("낚시회귀", |body| body.get_int("cooltime")),
        Some(77)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jump_cooldown_and_delayed_landing_match_python_exactly() {
    use crate::network::{Broadcaster, Client};
    use crate::player::{Player, STATE_ACTIVE};
    use tokio::sync::mpsc;

    let storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::default()));
    let broadcaster = Arc::new(Broadcaster::new());
    let scheduler = Arc::new(CallOutScheduler::default_resolution(broadcaster.clone()));
    let mut body = Body::new();
    body.set("이름", "점프예약검사");
    let denied = storage
        .read()
        .await
        .execute("점프", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);
    assert_eq!(body.get_int("cooltime"), 0);
    body.set("관리자등급", 1000_i64);
    let first = storage
        .read()
        .await
        .execute(
            "점프",
            &mut body,
            "무시",
            None,
            None,
            Some(scheduler.clone()),
        )
        .unwrap();
    assert_eq!(first.0, vec!["당신이 부웅~~ 날아 오릅니다"]);
    assert_eq!(body.get_int("cooltime"), 1);
    assert_eq!(scheduler.pending_count(), 1);
    let busy = storage
        .read()
        .await
        .execute("점프", &mut body, "", None, None, Some(scheduler))
        .unwrap();
    assert_eq!(busy.0, vec!["기술을 쓰기엔 너무도 바빠요~"]);

    let addr = "127.0.0.1:18055".parse().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    let mut player = Player::new();
    player.state = STATE_ACTIVE;
    player.body.set("이름", "점프착지검사");
    player.body.set("cooltime", 1_i64);
    client.player = Some(player);
    broadcaster.add_client(client);

    let runner = create_call_out_script_runner(storage, broadcaster.clone());
    runner("점프착지검사", Some("점프"), "jump_land", vec![]).unwrap();
    assert_eq!(
        rx.try_recv().unwrap(),
        "당신은 안전하게 착지합니다. ^^v\r\n"
    );
    assert_eq!(
        broadcaster.with_player_body_by_name("점프착지검사", |body| body.get_int("cooltime")),
        Some(0)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_input_keeps_each_argument_and_reenters_real_rhai_command() {
    use crate::network::{Broadcaster, Client};
    use crate::player::{Player, STATE_ACTIVE};
    use tokio::sync::mpsc;

    let storage = Arc::new(tokio::sync::RwLock::new(ScriptStorage::default()));
    let broadcaster = Arc::new(Broadcaster::new());
    let scheduler = Arc::new(CallOutScheduler::default_resolution(broadcaster.clone()));
    let mut command_body = Body::new();
    command_body.set("이름", "지연예약검사");
    for command in ["명중 올려", "회피 올려"] {
        let result = storage
            .read()
            .await
            .execute(
                "지연입력",
                &mut command_body,
                command,
                None,
                None,
                Some(scheduler.clone()),
            )
            .unwrap();
        assert!(result.0.is_empty());
    }
    assert_eq!(scheduler.pending_count(), 2);
    assert!(command_body.get_string("_지연입력").is_empty());

    let addr = "127.0.0.1:18056".parse().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut client = Client::new(addr, tx);
    client.complete_login();
    let mut player = Player::new();
    player.state = STATE_ACTIVE;
    player.body.set("이름", "지연실행검사");
    player.body.set("특성치", 2_i64);
    player.body.set("명중", 10_i64);
    player.body.set("회피", 20_i64);
    client.player = Some(player);
    broadcaster.add_client(client);
    let runner = create_call_out_script_runner(storage, broadcaster.clone());
    runner(
        "지연실행검사",
        Some("지연입력"),
        "delayed_execute",
        vec![serde_json::json!("명중 올려")],
    )
    .unwrap();
    runner(
        "지연실행검사",
        Some("지연입력"),
        "delayed_execute",
        vec![serde_json::json!("회피 올려")],
    )
    .unwrap();
    assert_eq!(rx.try_recv().unwrap(), "☞ [명중] 특성치를 올렸습니다.\r\n");
    assert_eq!(rx.try_recv().unwrap(), "☞ [회피] 특성치를 올렸습니다.\r\n");
    assert_eq!(
        broadcaster.with_player_body_by_name("지연실행검사", |body| (
            body.get_int("명중"),
            body.get_int("회피"),
            body.get_int("특성치")
        )),
        Some((11, 21, 0))
    );
    let _ = std::fs::remove_file("data/user/지연실행검사.json");
}
