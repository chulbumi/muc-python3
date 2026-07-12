//! 사용자 개인방 생성.
//!
//! Python `Player.makeHome()`이 만들던 방을 현재 Room loader가 읽는 JSON 형식으로
//! 기록한다. 화면 문구를 만들지는 않고 파일 생성 메커니즘과 결과 코드만 제공한다.

use serde_json::json;
use std::path::{Path, PathBuf};

const DEFAULT_MAP_ROOT: &str = "data/map";

#[derive(Debug, thiserror::Error)]
pub enum UserHomeError {
    #[error("invalid player name")]
    InvalidName,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// 기본 데이터 디렉토리에 `{이름}.json` 개인방을 만든다.
pub fn make_user_home(player_name: &str) -> Result<PathBuf, UserHomeError> {
    make_user_home_in(Path::new(DEFAULT_MAP_ROOT), player_name)
}

/// 테스트와 도구에서 데이터 루트를 바꿔 사용할 수 있는 개인방 생성 함수.
pub fn make_user_home_in(map_root: &Path, player_name: &str) -> Result<PathBuf, UserHomeError> {
    validate_player_name(player_name)?;

    let zone_dir = map_root.join("사용자맵");
    std::fs::create_dir_all(&zone_dir)?;
    let path = zone_dir.join(format!("{player_name}.json"));
    let room = json!({
        "맵정보": {
            "맵속성": ["사용자전투금지"],
            "설명": [format!("{player_name}의 방이다.")],
            "이름": format!("{player_name}의 방"),
            "존이름": "사용자맵",
            "주인": player_name,
            "출구": ["낙양성 낙양성:1"]
        }
    });
    let source = serde_json::to_string_pretty(&room)?;

    let temp_path = zone_dir.join(format!(".{player_name}.json.tmp"));
    std::fs::write(&temp_path, source)?;
    if let Err(error) = std::fs::rename(&temp_path, &path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(UserHomeError::Io(error));
    }
    Ok(path)
}

fn validate_player_name(player_name: &str) -> Result<(), UserHomeError> {
    let invalid = player_name.is_empty()
        || player_name == "."
        || player_name == ".."
        || player_name.contains(['/', '\\', '\0']);
    if invalid {
        Err(UserHomeError::InvalidName)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::RoomCache;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "muc_user_home_{}_{}_{}",
            label,
            std::process::id(),
            nonce
        ))
    }

    #[test]
    fn generated_home_is_loadable_by_room_cache() {
        let root = test_root("loadable");
        let path = make_user_home_in(&root, "홍길동").unwrap();
        assert_eq!(path, root.join("사용자맵/홍길동.json"));

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let info = &json["맵정보"];
        assert_eq!(info["이름"], "홍길동의 방");
        assert_eq!(info["주인"], "홍길동");
        assert_eq!(info["맵속성"][0], "사용자전투금지");

        let mut cache = RoomCache::with_data_dir(&root);
        let room = cache.get_room("사용자맵", "홍길동").unwrap();
        let room = room.read().unwrap();
        assert_eq!(room.display_name, "홍길동의 방");
        assert!(room.properties.iter().any(|p| p == "사용자전투금지"));
        assert_eq!(
            room.get_exit_by_name("낙양성")
                .and_then(|exit| exit.destination("사용자맵")),
            Some(("낙양성".to_string(), "1".to_string()))
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_path_components() {
        let root = test_root("invalid");
        assert!(matches!(
            make_user_home_in(&root, "../침범"),
            Err(UserHomeError::InvalidName)
        ));
        assert!(!root.join("침범.json").exists());
    }
}
