//! Scheduler module for delayed and periodic function calls
//!
//! Implements LPMUD-style call_out and heart_beat functionality.
//!
//! Based on MudOS/FluffOS:
//! - call_out(): Delayed function calls
//! - heart_beat: Per-object periodic updates

pub mod call_out;
pub mod heart_beat;

pub use call_out::{CallOutRegistry, CallOutScheduler, CallOutTask, ScriptRunnerFn};
pub use heart_beat::{HeartBeatConfig, HeartBeatRegistry};
