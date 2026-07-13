use super::*;

#[test]
fn note_command_matches_python_location_guards_view_layout_and_memory_clear() {
    use crate::player::MemoRecord;
    use crate::world::{get_world_state, PlayerPosition};

    let suffix = std::process::id();
    let name = format!("쪽지조회자-{suffix}");
    let mut body = Body::new();
    body.set("이름", name.as_str());
    let storage = ScriptStorage::default();
    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&name, PlayerPosition::new("낙양성".into(), "10".into()));
    assert_eq!(
        storage
            .execute("쪽지", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["정보수집소에서 할 수 있습니다."]
    );

    get_world_state()
        .write()
        .unwrap()
        .set_player_position(&name, PlayerPosition::new("낙양성".into(), "11".into()));
    assert_eq!(
        storage
            .execute("쪽지", &mut body, "", None, None, None)
            .unwrap()
            .0,
        vec!["도착한 쪽지가 없습니다."]
    );
    assert_eq!(
        storage
            .execute("쪽지", &mut body, "수신자", None, None, None)
            .unwrap()
            .0,
        vec!["☞ 사용법: [이름] [제목] 쪽지"]
    );

    set_precomputed_connected_names(vec![Dynamic::from("접속대상")]);
    assert_eq!(
        storage
            .execute(
                "쪽지",
                &mut body,
                "접속대상 여러 단어 제목",
                None,
                None,
                None
            )
            .unwrap()
            .0,
        vec!["접속중인 사용자에게는 보낼 수 없습니다."]
    );
    set_precomputed_connected_names(Vec::new());

    for (key, title, time, author, content) in [
        (
            "메모:나",
            "둘째",
            "2026-07-13 12:00:00",
            "나",
            "두 번째 내용",
        ),
        (
            "메모:가",
            "첫째",
            "2026-07-13 11:00:00",
            "가",
            "첫 번째 내용",
        ),
    ] {
        body.memos.insert(
            key.into(),
            MemoRecord {
                제목: title.into(),
                시간: time.into(),
                작성자: author.into(),
                내용: content.into(),
            },
        );
    }
    let viewed = storage
        .execute("쪽지", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(
        viewed.0,
        vec![concat!(
            "┌────────────────────────────────────┐\r\n",
            "│◁                    무           림           첩                    ▷│\r\n",
            "└────────────────────────────────────┘\r\n",
            "\x1b[33m보 낸 이\x1b[37m : 가\r\n",
            "\x1b[33m제    목\x1b[37m : 첫째\r\n",
            "\x1b[33m작성시각\x1b[37m : 2026-07-13 11:00:00\r\n\r\n",
            "첫 번째 내용\r\n",
            " ─────────────────────────────────────\r\n",
            "\x1b[33m보 낸 이\x1b[37m : 나\r\n",
            "\x1b[33m제    목\x1b[37m : 둘째\r\n",
            "\x1b[33m작성시각\x1b[37m : 2026-07-13 12:00:00\r\n\r\n",
            "두 번째 내용\r\n",
            " ─────────────────────────────────────"
        )]
    );
    assert!(body.memos.is_empty());
    get_world_state()
        .write()
        .unwrap()
        .remove_player_position(&name);
}
