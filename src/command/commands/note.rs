//! Python `cmds/쪽지.py`/`Player.write_memo` 이관을 위한 쪽지 데이터 로직.
//!
//! 사용자에게 보이는 문구와 레이아웃은 `cmds/쪽지.rhai`에만 두고,
//! 이 모듈은 오프라인 Body 로드·저장과 편집 상태 전이만 담당한다.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::command::handler::NoteRecipientState;
use crate::player::{Body, MemoRecord};
use crate::script::{load_body_from_json, save_body_to_json_without_timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeginNoteError {
    NotFound,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteEditAdvance {
    Continue,
    Complete { capacity_exceeded: bool },
}

/// Python `line.split(None, 1)`과 같은 수신자/제목 분리.
pub(crate) fn split_recipient_subject(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let boundary = line.find(char::is_whitespace)?;
    let recipient = &line[..boundary];
    let subject = line[boundary..].trim_start();
    if recipient.is_empty() || subject.is_empty() {
        return None;
    }
    Some((recipient.to_string(), subject.to_string()))
}

/// Python `Player.load(name)`의 성공 조건을 쪽지 수신자 로드에 적용한다.
/// `사용자오브젝트`가 없는 JSON은 Python에서도 로드 실패이다.
fn load_recipient(body: &mut Body, path: &Path) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(_) => return false,
    };
    let Some(root) = json.as_object() else {
        return false;
    };
    if !root
        .get("사용자오브젝트")
        .is_some_and(serde_json::Value::is_object)
    {
        return false;
    }
    if !load_body_from_json(body, &path.to_string_lossy()) {
        return false;
    }

    // Python Player.load는 `아이템` 키가 없으면 memo 로드 전에 반환한다.
    if !root.contains_key("아이템") {
        body.memos.clear();
    }
    true
}

/// 쪽지 작성을 시작하며 Python의 `_memoWho` Body를 반환한다.
/// `user_dir`은 실행 시 `data/user`, 테스트에서는 임시 경로다.
pub(crate) fn begin_note_in_dir(
    sender_name: &str,
    requested_name: &str,
    subject: &str,
    timestamp: &str,
    user_dir: &Path,
) -> Result<NoteRecipientState, BeginNoteError> {
    let load_path = user_dir.join(format!("{requested_name}.json"));
    let mut recipient = Body::new();
    if !load_recipient(&mut recipient, &load_path) {
        return Err(BeginNoteError::NotFound);
    }

    let memo_key = format!("메모:{sender_name}");
    if recipient.memos.contains_key(&memo_key) {
        return Err(BeginNoteError::Duplicate);
    }
    recipient.memos.insert(
        memo_key,
        MemoRecord {
            제목: subject.to_string(),
            시간: timestamp.to_string(),
            작성자: sender_name.to_string(),
            내용: String::new(),
        },
    );

    // Python `ply.save(False)`는 로드한 파일명이 아니라 Body의 `이름`으로 저장한다.
    let target_name = recipient.get_name();
    let save_path = user_dir.join(format!("{target_name}.json"));
    let _ = save_body_to_json_without_timestamp(&mut recipient, &save_path.to_string_lossy());

    Ok(NoteRecipientState {
        target_name,
        save_path: save_path.to_string_lossy().into_owned(),
        body: Arc::new(Mutex::new(recipient)),
    })
}

/// Python `write_memo` 입력 전이. 종료 검사는 현재 입력을
/// 본문에 더하기 전의 누적 문자열 길이를 사용한다.
pub(crate) fn advance_note_body(body: &mut String, input: &str) -> NoteEditAdvance {
    let capacity_exceeded = body.chars().count() >= 10;
    if input == "." || capacity_exceeded {
        return NoteEditAdvance::Complete { capacity_exceeded };
    }

    let line = if input.is_empty() { " " } else { input };
    if body.is_empty() {
        body.push_str(line);
    } else {
        body.push_str("\r\n");
        body.push_str(line);
    }
    NoteEditAdvance::Continue
}

/// 작성 시작 때 로드해 둔 수신자 Body에 본문을 넣고 `save(False)`한다.
pub(crate) fn finish_note(recipient: &NoteRecipientState, sender_name: &str, content: &str) {
    let Ok(mut target_body) = recipient.body.lock() else {
        return;
    };
    if let Some(memo) = target_body.memos.get_mut(&format!("메모:{sender_name}")) {
        memo.내용 = content.to_string();
    }
    let _ = save_body_to_json_without_timestamp(&mut target_body, &recipient.save_path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_user_dir(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("muc_note_{label}_{}_{}", std::process::id(), nonce));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_player(path: &Path, internal_name: &str, memo: Option<MemoRecord>) {
        let mut root = json!({
            "사용자오브젝트": {
                "이름": internal_name,
                "마지막저장시간": 1234
            },
            "아이템": []
        });
        if let Some(memo) = memo {
            root.as_object_mut().unwrap().insert(
                "메모:보낸이".to_string(),
                serde_json::to_value(memo).unwrap(),
            );
        }
        std::fs::write(path, serde_json::to_string_pretty(&root).unwrap()).unwrap();
    }

    #[test]
    fn split_header_matches_python_split_none_once() {
        assert_eq!(
            split_recipient_subject("  수신자\t여러 단어 제목  "),
            Some(("수신자".to_string(), "여러 단어 제목".to_string()))
        );
        assert_eq!(split_recipient_subject("수신자"), None);
    }

    #[test]
    fn editor_uses_character_length_before_current_input_and_exact_dot() {
        let mut body = String::new();
        assert_eq!(advance_note_body(&mut body, ""), NoteEditAdvance::Continue);
        assert_eq!(body, " ");
        assert_eq!(
            advance_note_body(&mut body, " . "),
            NoteEditAdvance::Continue
        );
        assert_eq!(body, " \r\n . ");
        assert_eq!(
            advance_note_body(&mut body, "."),
            NoteEditAdvance::Complete {
                capacity_exceeded: false
            }
        );

        let mut ten = "가나다라마바사아자차".to_string();
        assert_eq!(
            advance_note_body(&mut ten, "저장되지않음"),
            NoteEditAdvance::Complete {
                capacity_exceeded: true
            }
        );
        assert_eq!(ten, "가나다라마바사아자차");
    }

    #[test]
    fn begin_and_finish_hold_loaded_recipient_and_preserve_last_saved_time() {
        let dir = temp_user_dir("flow");
        let requested_path = dir.join("파일별칭.json");
        write_player(&requested_path, "실제수신자", None);

        let state =
            begin_note_in_dir("보낸이", "파일별칭", "제목", "2026-07-10 12:34:56", &dir).unwrap();
        assert_eq!(state.target_name, "실제수신자");
        assert_eq!(
            state.save_path,
            dir.join("실제수신자.json").to_string_lossy()
        );

        let mut saved = Body::new();
        assert!(load_body_from_json(&mut saved, &state.save_path));
        assert_eq!(saved.get_int("마지막저장시간"), 1234);
        assert_eq!(saved.memos["메모:보낸이"].내용, "");

        // 파일을 바꿔도 Python `_memoWho`처럼 시작 시점 Body를 저장한다.
        write_player(Path::new(&state.save_path), "실제수신자", None);
        finish_note(&state, "보낸이", "첫줄\r\n둘째줄");
        let mut finished = Body::new();
        assert!(load_body_from_json(&mut finished, &state.save_path));
        assert_eq!(finished.memos["메모:보낸이"].내용, "첫줄\r\n둘째줄");
        assert_eq!(finished.get_int("마지막저장시간"), 1234);

        assert_eq!(
            begin_note_in_dir("보낸이", "실제수신자", "다시", "now", &dir),
            Err(BeginNoteError::Duplicate)
        );
        assert_eq!(
            begin_note_in_dir("보낸이", "없는이", "제목", "now", &dir),
            Err(BeginNoteError::NotFound)
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
