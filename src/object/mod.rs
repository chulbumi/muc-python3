//! Object module for MUD engine
//!
//! Provides the base Object structure and related functionality for managing
//! game objects with attributes, child objects, and parent relationships.

pub mod base;

pub use base::Object;

// Re-export commonly used types
pub use crate::utils::Value;
