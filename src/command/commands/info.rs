//! Information commands for MUD engine
//!
//! 소지품, 능력치, 도움말: Rhai 스크립트. 봐: 봐.rhai.

use crate::command::registry::CommandRegistry;

/// Registers all info commands
pub fn register_info_commands(_registry: &mut CommandRegistry) {
    // 소지품, 점수, 도움말은 Rhai 스크립트로 처리하고 objs/alias.py의
    // 소/소지, 점/상태/상/정/정보/능력치, 도움 매핑만 사용한다.

    // 봐/보 역시 Rhai 스크립트와 objs/alias.py의 보→봐 매핑으로 처리한다.
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
