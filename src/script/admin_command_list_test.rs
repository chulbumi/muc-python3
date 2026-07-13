use super::*;

fn python_level_1000_names() -> Vec<String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir("cmds").unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("py") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        if source.lines().any(|line| {
            let code = line.trim_start();
            !code.starts_with('#') && (code.contains("level = 1000") || code.contains("level=1000"))
        }) {
            names.push(path.file_stem().unwrap().to_string_lossy().to_string());
        }
    }
    names
}

#[test]
fn admin_command_list_matches_python_level_filter_order_and_exact_columns() {
    let storage = ScriptStorage::default();
    let mut body = Body::new();
    body.set("이름", "명령어리스트검사");
    body.set("관리자등급", 999_i64);
    let denied = storage
        .execute("명령어리스트", &mut body, "무시", None, None, None)
        .unwrap();
    assert_eq!(denied.0, vec!["☞ 무슨 말인지 모르겠어요. *^_^*"]);

    let names = python_level_1000_names();
    assert!(!names.is_empty());
    let mut expected = String::new();
    for (index, name) in names.iter().enumerate() {
        expected.push_str(&format!("{name:>20}"));
        if (index + 1) % 3 == 0 {
            expected.push_str("\r\n");
        }
    }

    for (level, argument) in [(1000_i64, ""), (2000_i64, "어떤 인자도 무시")] {
        body.set("관리자등급", level);
        let listed = storage
            .execute("명령어리스트", &mut body, argument, None, None, None)
            .unwrap();
        assert_eq!(listed.0, vec![expected.clone()]);
    }

    let rows = expected
        .strip_suffix("\r\n")
        .unwrap_or(&expected)
        .split("\r\n");
    for (row_index, row) in rows.enumerate() {
        let expected_columns = if row_index * 3 + 3 <= names.len() {
            3
        } else {
            names.len() - row_index * 3
        };
        assert_eq!(row.chars().count(), expected_columns * 20, "{row:?}");
    }
    assert_eq!(
        expected.ends_with("\r\n"),
        names.len() % 3 == 0,
        "Python retains the final CRLF only for a complete three-column row"
    );
}
