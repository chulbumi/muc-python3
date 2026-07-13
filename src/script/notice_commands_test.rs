use super::*;

#[test]
fn notice_command_owns_python_usage_border_ansi_and_unbounded_width() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "공지회귀");

    let usage = storage
        .execute("공지말", &mut body, "", None, None, None)
        .unwrap();
    assert_eq!(usage.0, vec!["☞ 운영자 명령: [내용] 공지말"]);

    let notice = storage
        .execute("공지말", &mut body, "서버 점검", None, None, None)
        .unwrap();
    let expected = format!(
        "{}\r\n\x1b[7m☞ 공지 : {:<68}\x1b[0m\r\n{}",
        "─".repeat(39),
        "서버 점검",
        "─".repeat(39)
    );
    assert_eq!(notice.0, Vec::<String>::new());
    assert!(matches!(notice.1, Some(CommandResult::Notice(ref text)) if text == &expected));

    let long = "긴".repeat(201);
    let result = storage
        .execute("공지말", &mut body, &long, None, None, None)
        .unwrap();
    assert!(matches!(result.1, Some(CommandResult::Notice(ref text)) if text.contains(&long)));
}

#[test]
fn notice_board_preserves_python_cat_whitespace_ansi_and_crlf_conversion() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "공지사항회귀");
    let source = std::fs::read_to_string("data/text/notice.txt").unwrap();
    let result = storage
        .execute("공지사항", &mut body, "무시되는 인자", None, None, None)
        .unwrap();
    assert_eq!(result.0, vec![source.replace('\n', "\r\n")]);
    assert!(result.0[0].starts_with("\x1b[H\x1b[2J┌"));
    assert!(result.0[0].contains("                              공  지  사  항"));
}
