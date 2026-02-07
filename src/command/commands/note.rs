//! 쪽지 명령. 파이썬 cmds/쪽지.py, objs/player.write_memo 참조.
//!
//! [이름] [제목] 쪽지: 편집 모드. 라인 단위 입력, '.' 또는 10줄이면 저장 후 종료.
//! 쪽지 (인자 없음): 도착 쪽지 보기.

use chrono::Local;
use std::sync::Arc;

use crate::command::registry::CommandRegistry;
use crate::command::CommandResult;
use crate::player::{Body, MemoRecord};
use crate::script::{load_body_from_json, save_body_to_json};
use crate::world::get_world_state;

const NOTE_POSITION: (&str, &str) = ("낙양성", "11");

fn note_command(body: &mut Body, args: &[&str]) -> CommandResult {
    if args.is_empty() || (args.len() == 1 && args[0].is_empty()) {
        return view_memo(body);
    }
    let line = args.join(" ");
    let words: Vec<&str> = line.splitn(2, |c: char| c.is_whitespace()).collect();
    if words.len() < 2 {
        return CommandResult::Usage("☞ 사용법: [이름] [제목] 쪽지".to_string());
    }
    let target_name = words[0].trim();
    let title = words[1].trim();
    if target_name.is_empty() || title.is_empty() {
        return CommandResult::Usage("☞ 사용법: [이름] [제목] 쪽지".to_string());
    }

    let (zone, room) = get_world_state()
        .read()
        .ok()
        .and_then(|w| w.get_player_position(&body.get_name()).cloned())
        .map(|p| (p.zone, p.room))
        .unwrap_or((String::new(), String::new()));

    if zone != NOTE_POSITION.0 || room != NOTE_POSITION.1 {
        return CommandResult::Output("정보수집소에서 할 수 있습니다.".to_string());
    }

    if get_world_state()
        .read()
        .ok()
        .map(|w| w.get_player_position(target_name).is_some())
        .unwrap_or(false)
    {
        return CommandResult::Output("접속중인 사용자에게는 보낼 수 없습니다.".to_string());
    }

    let target_path = format!("data/user/{}.json", target_name);
    let mut target_body = Body::new();
    if !load_body_from_json(&mut target_body, &target_path) {
        return CommandResult::Output("존재하지않는 사용자입니다.".to_string());
    }

    let memo_key = format!("메모:{}", body.get_name());
    if target_body.memos.contains_key(&memo_key) {
        return CommandResult::Output(
            "한번 보냈던 사용자에게는 다시 보낼 수 없습니다.".to_string(),
        );
    }

    let time_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let record = MemoRecord {
        제목: title.to_string(),
        시간: time_str,
        작성자: body.get_name(),
        내용: String::new(),
    };
    target_body.memos.insert(memo_key.clone(), record);
    let _ = save_body_to_json(&mut target_body, &target_path);

    CommandResult::StartNoteEdit {
        target_name: target_name.to_string(),
        title: title.to_string(),
    }
}

fn view_memo(body: &mut Body) -> CommandResult {
    if body.memos.is_empty() {
        return CommandResult::Output("도착한 쪽지가 없습니다.".to_string());
    }
    let mut msg = "┌────────────────────────────────────┐\r\n".to_string();
    msg.push_str("│◁                    무           림           첩                    ▷│\r\n");
    msg.push_str("└────────────────────────────────────┘\r\n");
    for (_, m) in &body.memos {
        msg.push_str(&format!("\x1b[33m보 낸 이\x1b[37m : {}\r\n", m.작성자));
        msg.push_str(&format!("\x1b[33m제    목\x1b[37m : {}\r\n", m.제목));
        msg.push_str(&format!("\x1b[33m작성시각\x1b[37m : {}\r\n\r\n", m.시간));
        msg.push_str(&format!("{}\r\n", m.내용));
        msg.push_str(" ─────────────────────────────────────\r\n");
    }
    if msg.ends_with("\r\n") {
        msg.truncate(msg.len() - 2);
    }
    body.memos.clear();
    let path = format!("data/user/{}.json", body.get_name());
    let _ = save_body_to_json(body, &path);
    CommandResult::Output(msg)
}

pub fn register_note_commands(registry: &mut CommandRegistry) {
    registry.register(crate::command::registry::CommandInfo {
        name: "쪽지".to_string(),
        aliases: vec!["메모보냄".to_string()],
        handler: Arc::new(note_command),
        level: 0,
        description: "쪽지 보내기/보기. 정보수집소(낙양성:11)에서만. [이름] [제목] 쪽지"
            .to_string(),
        usage: "[이름] [제목] 쪽지".to_string(),
    });
}
