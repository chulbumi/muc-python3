//! 주다(줘/give) 명령
//!
//! 플레이어에게 은전/금전/아이템을 건네줌. cmds/줘.py 기준.


use crate::command::parser::CommandParser;
use crate::command::registry::CommandRegistry;
use crate::command::CommandResult;
use crate::player::Body;

/// [대상] [물품] [개수] 주다. 물품=은전|금전|아이템이름. 개수 생략=1.
/// 대상이 self면 "장난" 에러. 은전/금전 부족 시 "돈이 모자라네요". 아이템 없으면 "그런 아이템이 소지품에 없어요".
fn give_command(body: &mut Body, args: &[&str]) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::Error("☞ 사용법: [대상] [물품] [개수] 주다".to_string());
    }
    let target_name = args[0].to_string();
    let thing = args[1];
    let count: i64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1)
        .min(50);

    let giver_name = body.get_name();

    if target_name == giver_name {
        if thing == "은전" || thing == "금전" {
            return CommandResult::Error("☞ 자기 자신에게 돈을 줄 수 없어요. ^^".to_string());
        }
        let (name, _) = CommandParser::parse_name_order(thing);
        if !name.is_empty() {
            if let Some(arc) = body.object_ref().findObjInven(&name, 1) {
                let post = arc
                    .lock()
                    .map(|o| o.han_obj())
                    .unwrap_or_else(|_| name.clone());
                return CommandResult::Error(format!(
                    "당신이 \x1b[36m{}\x1b[37m 가지고 장난합니다. '@_@'",
                    post
                ));
            }
        }
    }

    if thing == "은전" {
        let have = body.get_int("은전");
        let amt = count.min(have.max(0));
        if amt < 1 {
            return CommandResult::Error("☞ 돈이 모자라네요. ^^".to_string());
        }
        return CommandResult::GiveToPlayer {
            target_name,
            giver_name,
            give_silver: Some(amt),
            give_gold: None,
            give_item: None,
            give_item_stack: None,
        };
    }

    if thing == "금전" {
        let have = body.get_int("금전");
        let amt = count.min(have.max(0));
        if amt < 1 {
            return CommandResult::Error("☞ 돈이 모자라네요. ^^".to_string());
        }
        return CommandResult::GiveToPlayer {
            target_name,
            giver_name,
            give_silver: None,
            give_gold: Some(amt),
            give_item: None,
            give_item_stack: None,
        };
    }

    let (name, order) = CommandParser::parse_name_order(thing);
    if name.is_empty() {
        return CommandResult::Error("☞ 사용법: [대상] [물품] [개수] 주다".to_string());
    }
    let order = order.max(1);
    if body.object_ref().findObjInven(&name, order).is_none() {
        return CommandResult::Error("☞ 그런 아이템이 소지품에 없어요.".to_string());
    }
    // 파이썬: order != 1 이면 1개만 (2.검 5 → 1개만)
    let count = if order > 1 { 1 } else { count.min(50).max(1) as usize };

    CommandResult::GiveToPlayer {
        target_name,
        giver_name,
        give_silver: None,
        give_gold: None,
        give_item: Some((name, order, count)),
        give_item_stack: None,
    }
}

pub fn register_give_commands(_registry: &mut CommandRegistry) {
    // 주다: Rhai 전환 (cmds/주다.rhai). aliases는 register_script_commands에서 스크립트 등록 시 부여.
}
