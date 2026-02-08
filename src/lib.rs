//! # MUD Engine (Rust)
//!
//! Python 기반 MUD 서버를 Rust로 마이그레이션한 엔진입니다.
//!
//! ## 주요 구성 요소
//!
//! - **combat**: 전투 시스템 (데미지 계산, 명중률 등)
//! - **command**: 명령어 처리 및 레지스트리
//! - **script**: Rhai 스크립트 엔진 연동
//! - **world**: 게임 월드 (방, 몹, 아이템, 스킬)
//! - **player**: 플레이어 데이터 및 상태
//! - **network**: 네트워크 서버 및 클라이언트
//! - **server**: 메인 서버 및 게임 루프
//!
//! ## 아키텍처
//!
//! - 205개의 Rhai 스크립트 명령어 (`cmds/` 디렉토리)
//! - JSON 기반 데이터 로딩 (`data/` 디렉토리)
//! - 핫리로드 지원 (스크립트 수정 시 즉시 반영)
//! - 333개 단위 테스트 통과

pub mod combat;
pub mod command;
pub mod data;
pub mod doumi;
pub mod emotion;
pub mod hangul;
pub mod hotreload;
pub mod loader;
pub mod master;
pub mod network;
pub mod object;
pub mod oneitem;
pub mod player;
pub mod scheduler;
pub mod script;
pub mod server;
pub mod utils;
pub mod world;

// Re-export commonly used types
pub use command::{CommandHandler, CommandParser, CommandRegistry, CommandResult};
pub use data::{create_global_data, json_to_dynamic, GlobalData, SharedGlobalData};
pub use loader::{load_json, load_script, save_json, ScriptValue, ScriptValueInner};
pub use master::{MasterConfig, MasterObject};
pub use object::{Object, Value};
pub use player::{ActState, Body, Player, SkillLevel};
pub use script::{ScriptConfig, ScriptEngine, SharedScriptEngine};
pub use server::{GameLoop, GameLoopConfig, MudServer, ServerConfig};
pub use world::{
    get_item_cache, get_mob_cache, get_world_state, Direction, Exit, ItemCache, ItemInstance,
    MobCache, MobInstance, PlayerPosition, RawItemData, RawMobData, Room, RoomCache, RoomError,
    WorldState,
};
