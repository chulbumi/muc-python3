use super::*;
#[test]
fn test_fill_space_euc_kr_matches_python_fill_space() {
    assert_eq!(fill_space_euc_kr(20, "지르기(2성)"), "지르기(2성)         ");
    assert_eq!(
        fill_space_euc_kr(20, "\x1b[31m비전검법\x1b[0m"),
        "\x1b[31m비전검법\x1b[0m            "
    );
}

#[tokio::test]
async fn test_shared_storage() {
    let shared = SharedScriptStorage::new(ScriptConfig::default());
    let storage = shared.inner.read().await;
    assert!(storage.config.script_dir.ends_with("cmds"));
}
#[test]
fn test_han_iga() {
    assert_eq!(han_iga("사과"), "가");
    assert_eq!(han_iga("검"), "이");
}
#[test]
fn test_ansi_convert() {
    let result = ansi_convert("{밝}hello{어}", true);
    assert!(result.contains("\x1b[1m"));
    assert!(result.contains("\x1b[0m"));

    let result = ansi_convert("{밝}hello{어}", false);
    assert_eq!(result, "hello");
}
