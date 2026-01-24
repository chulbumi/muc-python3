//! Information commands for MUD engine
//!
//! 소지품, 능력치, 도움말: Rhai 스크립트. 봐: 봐.rhai.

use crate::command::registry::CommandRegistry;

/// Registers all info commands
pub fn register_info_commands(_registry: &mut CommandRegistry) {
    // 소지품, 능력치, 도움말: Rhai 스크립트로 처리. built_in_aliases에 소/소지/인벤토리/inventory→소지품, 점수/점/상태/상/정/정보/score/stat→능력치, 도움/help/?//h→도움말.

    // 봐/보/look: 봐.rhai 스크립트로 처리. built_in_aliases에 보→봐, look→봐.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_info_commands() {
        let mut registry = CommandRegistry::new();
        register_info_commands(&mut registry);
        // 소지품, 능력치, 도움말, 봐: Rhai 스크립트로 등록됨 (register_script_commands).
    }
}
