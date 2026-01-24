pub mod emotion;
pub mod hangul;
pub mod utils;
pub mod network;
pub mod loader;
pub mod object;
pub mod player;
pub mod command;
pub mod script;
pub mod server;
pub mod master;
pub mod scheduler;
pub mod data;
pub mod hotreload;
pub mod world;

// Re-export commonly used types
pub use loader::{load_json, load_script, save_json, ScriptValue, ScriptValueInner};
pub use object::{Object, Value};
pub use player::{Body, Player, ActState, SkillLevel};
pub use command::{CommandParser, CommandRegistry, CommandResult, CommandHandler};
pub use script::{ScriptEngine, ScriptConfig, SharedScriptEngine};
pub use server::{MudServer, ServerConfig, GameLoop, GameLoopConfig};
pub use master::{MasterObject, MasterConfig};
pub use data::{GlobalData, SharedGlobalData, create_global_data, json_to_dynamic};
pub use world::{
    Room, RoomCache, RoomError, Direction, Exit,
    MobInstance, MobCache, RawMobData, get_mob_cache,
    ItemInstance, ItemCache, RawItemData, get_item_cache,
    PlayerPosition, WorldState, get_world_state,
};