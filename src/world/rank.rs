//! 순위(RANK) 모듈. data/config/rank.json 로드/저장.
//! Python objs/rank.py: attr[type] = [(value, name), ...], value 내림차순.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const DEFAULT_PATH: &str = "data/config/rank.json";

/// (value, name) 튜플. JSON: [value, "name"]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RankEntry(i64, String);

/// type -> [(value, name), ...]
#[derive(Debug)]
pub struct Rank {
    path: PathBuf,
    pub attr: HashMap<String, Vec<(i64, String)>>,
}

impl Default for Rank {
    fn default() -> Self {
        Self::new()
    }
}

impl Rank {
    pub fn new() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_PATH),
            attr: HashMap::new(),
        }
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            attr: HashMap::new(),
        }
    }

    pub fn load(&mut self) {
        let Ok(s) = std::fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(root) = serde_json::from_str::<HashMap<String, Vec<RankEntry>>>(&s) else {
            return;
        };
        self.attr.clear();
        for (k, arr) in root {
            self.attr
                .insert(k, arr.into_iter().map(|e| (e.0, e.1)).collect());
        }
    }

    pub fn save(&self) -> bool {
        let root: HashMap<String, Vec<RankEntry>> = self
            .attr
            .iter()
            .map(|(k, arr)| {
                (
                    k.clone(),
                    arr.iter().map(|(a, b)| RankEntry(*a, b.clone())).collect(),
                )
            })
            .collect();
        let s = serde_json::to_string_pretty(&root).unwrap_or_default();
        std::fs::write(&self.path, s).is_ok()
    }

    /// rank_write(type, name, value, level). value==-1이면 0으로 맨 앞 삽입. 기존 동일 name 제거 후 추가, value 기준 내림차순 유지. 최대 200.
    pub fn write_rank(&mut self, ty: &str, name: &str, value: i64, _level: i64) -> i64 {
        let arr = self.attr.entry(ty.to_string()).or_default();
        arr.retain(|(_, n)| n != name);
        if value == -1 {
            arr.insert(0, (0, name.to_string()));
        } else {
            arr.push((value, name.to_string()));
            // Python `rank.sort(reverse=True)`는 (value, name) 튜플 전체를 내림차순 정렬한다.
            arr.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
        }
        if arr.len() > 200 {
            arr.truncate(200);
        }
        let _ = self.save();
        self.read_rank(ty, name)
    }

    /// rank_read(type, name) -> 순위(1부터). 없으면 0.
    pub fn read_rank(&self, ty: &str, name: &str) -> i64 {
        let arr = match self.attr.get(ty) {
            Some(a) => a,
            None => return 0,
        };
        for (i, (_, n)) in arr.iter().enumerate() {
            if n == name {
                return (i + 1) as i64;
            }
        }
        0
    }

    /// rank_get_num(type, rank) -> 그 순위의 이름. rank는 1부터. 없으면 None.
    pub fn get_rank_num(&self, ty: &str, rank: i64) -> Option<String> {
        let arr = self.attr.get(ty)?;
        let i = (rank - 1) as usize;
        arr.get(i).map(|(_, n)| n.clone())
    }

    /// rank_get_all(type) -> "━━...순  위...\r\n[ 1] xxx    [ 2] yyy ...
    pub fn get_rank_all(&self, ty: &str) -> String {
        let arr = self.attr.get(ty).map(Vec::as_slice).unwrap_or(&[]);
        let mut msg = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n".to_string();
        msg.push_str("\x1b[0m\x1b[47m\x1b[30m순  위 존      함    순  위 존      함    순  위 존      함 \x1b[0m\x1b[37m\x1b[40m\r\n");
        msg.push_str("──────────────────────────────\r\n");
        let mut c = 0i64;
        for (_, name) in arr {
            c += 1;
            msg.push_str(&format!("[{:4}] {:<10}    ", c, name));
            if c % 3 == 0 {
                msg.push_str("\r\n");
            }
        }
        msg.push_str("\r\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        msg
    }

    /// rank_clear(type). 해당 타입만 초기화.
    pub fn clear(&mut self, ty: &str) -> bool {
        if ty.is_empty() {
            self.attr.clear();
        } else if let Some(entries) = self.attr.get_mut(ty) {
            entries.clear();
        } else {
            return false;
        }
        let _ = self.save();
        true
    }
}

static RANK: std::sync::OnceLock<std::sync::RwLock<Rank>> = std::sync::OnceLock::new();

fn get_rank() -> &'static std::sync::RwLock<Rank> {
    RANK.get_or_init(|| {
        let mut r = Rank::new();
        r.load();
        std::sync::RwLock::new(r)
    })
}

pub fn rank_write(ty: &str, name: &str, value: i64, level: i64) -> i64 {
    get_rank()
        .write()
        .unwrap()
        .write_rank(ty, name, value, level)
}

pub fn rank_read(ty: &str, name: &str) -> i64 {
    get_rank().read().unwrap().read_rank(ty, name)
}

pub fn rank_get_num(ty: &str, rank: i64) -> Option<String> {
    get_rank().read().unwrap().get_rank_num(ty, rank)
}

pub fn rank_get_all(ty: &str) -> String {
    get_rank().read().unwrap().get_rank_all(ty)
}

pub fn rank_clear(ty: &str) -> bool {
    get_rank().write().unwrap().clear(ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("muc_rank_{}_{}.json", std::process::id(), nonce))
    }

    #[test]
    fn clear_keeps_named_rank_like_python() {
        let path = test_path();
        let mut rank = Rank::with_path(&path);
        rank.attr.insert(
            "경험".to_string(),
            vec![(10, "가".to_string()), (5, "나".to_string())],
        );

        assert!(rank.clear("경험"));
        assert_eq!(rank.attr.get("경험"), Some(&Vec::new()));

        let mut loaded = Rank::with_path(&path);
        loaded.load();
        assert_eq!(loaded.attr.get("경험"), Some(&Vec::new()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn equal_values_sort_by_name_descending_like_python_tuples() {
        let path = test_path();
        let mut rank = Rank::with_path(&path);
        rank.write_rank("경험", "가", 10, 1);
        rank.write_rank("경험", "나", 10, 1);
        assert_eq!(
            rank.attr.get("경험").unwrap(),
            &vec![(10, "나".to_string()), (10, "가".to_string())]
        );
        let _ = std::fs::remove_file(path);
    }
}
