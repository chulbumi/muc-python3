//! Source-to-Rhai routing guard for Python `objs/event.py` directives.
//!
//! This is deliberately separate from `event.rs`: it checks the data
//! conversion inventory, while execution/state regressions remain beside the
//! event engine that they exercise.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn normalize_event_key(key: &str) -> String {
    key.trim()
        .trim_start_matches('#')
        .trim_start_matches("이벤트")
        .trim_start()
        .trim_start_matches(':')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_mob_paths(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
    {
        let path = entry.expect("mob directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "backup") {
                collect_mob_paths(&path, paths);
            }
        } else if path.extension().is_some_and(|extension| extension == "mob") {
            paths.push(path);
        }
    }
}

fn collect_rhai_paths(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
    {
        let path = entry.expect("Rhai directory entry").path();
        if path.is_dir() {
            collect_rhai_paths(&path, paths);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rhai")
        {
            paths.push(path);
        }
    }
}

/// At least one Rhai token that represents the Python directive's state or
/// branch.  Some source directives deliberately share one helper; the
/// alternatives preserve those intentional conversions.
fn rhai_fingerprints(directive: &str) -> Option<&'static [&'static str]> {
    Some(match directive {
        "$강화확인2000!" => &["item_attack_below("],
        "$기연존재확인" => &["one_item_exists_name("],
        // Python `$기연확인[!]` calls checkOneItemIndex(), unlike
        // `$기연존재확인`, which starts from a display name.
        "$기연확인" | "$기연확인!" => &["one_item_exists("],
        "$남자설정" => &["set_body_text(\"성별\", \"남\")"],
        "$여자설정" => &["set_body_text(\"성별\", \"여\")"],
        "$레벨상위확인" | "$레벨상위확인!" => &["get_stat(\"레벨\")"],
        "$몹상태설정" => &["set_selected_mob_", "respawn_selected_mob("],
        "$몹상태확인" | "$몹상태확인!" => &["selected_mob_"],
        "$무공개수확인" => &["skill_count("],
        "$무공리스트삭제" | "$무공회수" => &["remove_skill("],
        // Python checkMugongList() reads skillList.  A single-name inverse
        // check can therefore become has_skill(), but it must never be
        // satisfied merely by a neighbouring `$비전종류확인!` has_vision().
        "$무공리스트확인" | "$무공리스트확인!" => &["has_all_skills(", "has_skill("],
        "$무공시전" | "$무공시전2" => &["apply_defense_skill("],
        "$무공전수" => &["teach_skill("],
        "$무공확인" => &["has_skill("],
        "$무림별호조건" => &["get_tendency("],
        "$변수확인" => &["words("],
        "$별호변경" => &["nickname_"],
        "$비무관람시작" | "$비무관람끝" => &["output("],
        "$비전수련가능확인!" => &["vision_training_allowed("],
        "$비전수련설정" => &["set_vision_training_name("],
        "$비전수련설정확인" => &["vision =="],
        "$비전수련확인" => &["vision_training_is_empty("],
        "$비전종류확인" | "$비전종류확인!" => &["has_vision("],
        "$성별확인" => &["get_body_text(\"성별\")"],
        "$소오강호설정" => &["set_giin("],
        "$속성템주기" => &["give_lottery_attribute_item("],
        "$순위갱신" | "$순위기록" | "$순위확인" => &["rank_"],
        "$스크립트호출" => &["start_script("],
        "$아이템삭제" => &["delete_item(", "delete_item_named("],
        "$아이템속성확인!" => &["item_has_options("],
        "$아이템옵션삭제" => &["clear_item_options("],
        "$아이템종류확인" => &["item_kind_is("],
        "$아이템주기" => &["give_item("],
        "$아이템확인" => &["has_item("],
        "$아이템확인!" => &["has_item(", "item_exists_unworn_named("],
        "$아이템확장설정" => &["set_item_extension("],
        "$아이템확장설정지움" => &["clear_item_extension("],
        "$아이템확장확인" | "$아이템확장확인!" => &["item_has_extension("],
        "$엔터$" | "$입력대기출력끝$" => &["wait_enter("],
        "$올숙자격확인" => &["has_olsuk_qualification("],
        "$올숙확인" | "$올숙확인!" => &["is_olsuk_complete("],
        "$우화등선설정" => &["set_sunin("],
        "$위치이동" => &["set_position("],
        "$은둔칩거설정" => &["set_eundun("],
        "$이벤트삭제" => &["del_event("],
        "$이벤트설정" => &["set_event("],
        "$이벤트확인" | "$이벤트확인!" => &["check_event("],
        "$전투시작" => &["try_start_selected_mob_combat(", "start_event_combat("],
        "$정사전환" => &["tendency_switch("],
        "$중급수련" => &["apply_intermediate_training_to_selected_mob("],
        "$착용확인!" => &["item_is_equipped("],
        "$체력감소" | "$체력소모" => &["consume_hp("],
        "$출력" => &["output(", "literal_output(", "room_broadcast_output("],
        "$특성치변경" => &["change_stat("],
        "$특성치복사" | "$특성치복사고" | "$특성치복사저" => {
            &["copy_player_stats_to_selected_mob("]
        }
        "$특성치설정" => &["set_stat("],
        "$특성치확인" => &["get_stat("],
        _ => return None,
    })
}

#[test]
fn every_used_python_event_directive_keeps_a_rhai_conversion_fingerprint() {
    let python = std::fs::read_to_string("objs/event.py").expect("Python event source");
    let handlers = python
        .split("func == '")
        .skip(1)
        .filter_map(|tail| tail.split_once('\'').map(|(name, _)| name))
        .filter(|name| name.starts_with('$'))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    assert_eq!(handlers.len(), 87, "Python handler inventory changed");

    let mut paths = Vec::new();
    collect_mob_paths(Path::new("data/mob"), &mut paths);
    let mut used = BTreeSet::new();
    let mut checked_calls = 0_usize;
    let mut missing = Vec::new();

    for mob_path in paths {
        let json_path = mob_path.with_extension("json");
        if !json_path.exists() {
            continue;
        }
        let root: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&json_path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", json_path.display())),
        )
        .unwrap_or_else(|error| panic!("invalid {}: {error}", json_path.display()));
        let info = root["몹정보"]
            .as_object()
            .unwrap_or_else(|| panic!("missing 몹정보 in {}", json_path.display()));
        let scripts = info
            .iter()
            .filter_map(|(key, value)| {
                key.starts_with("이벤트")
                    .then(|| value.as_str().map(|path| (normalize_event_key(key), path)))
                    .flatten()
            })
            .collect::<BTreeMap<_, _>>();
        let zone = mob_path
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .expect("mob zone");

        let mut event_key: Option<String> = None;
        let mut directives = Vec::new();
        let mut block_depth = 0_i32;
        let mut top_level_terminated = false;
        let check_event = |event_key: Option<&String>, directives: &[String]| {
            let Some(event_key) = event_key else {
                return Vec::new();
            };
            let Some(script) = scripts.get(event_key) else {
                return vec![format!(
                    "{} has no JSON event mapping for {event_key}",
                    mob_path.display()
                )];
            };
            let path = Path::new("data/script").join(zone).join(script);
            let rhai = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            directives
                .iter()
                .filter_map(|directive| rhai_fingerprints(directive).map(|need| (directive, need)))
                .filter(|(_, need)| !need.iter().any(|fingerprint| rhai.contains(fingerprint)))
                .map(|(directive, need)| {
                    format!(
                        "{} {event_key}: {directive} missing one of {:?}",
                        path.display(),
                        need
                    )
                })
                .collect()
        };

        for line in std::fs::read_to_string(&mob_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", mob_path.display()))
            .lines()
        {
            if line.starts_with("#이벤트") {
                missing.extend(check_event(event_key.as_ref(), &directives));
                event_key = Some(normalize_event_key(line));
                directives.clear();
                block_depth = 0;
                top_level_terminated = false;
                continue;
            }
            let directive = line.trim().trim_start_matches(':').trim();
            if directive == "{" {
                block_depth += 1;
                continue;
            }
            if directive == "}" {
                block_depth = (block_depth - 1).max(0);
                continue;
            }
            // Keep the call inventory identical to the authoritative audit:
            // exactly one leading `:` denotes a legacy instruction.  A
            // double-colon text line must not become a synthetic directive.
            let token = line
                .trim()
                .strip_prefix(':')
                .filter(|directive| directive.starts_with('$'))
                .and_then(|directive| directive.split_whitespace().next());
            if token == Some("$종료") && block_depth == 0 {
                top_level_terminated = true;
            }
            if let Some(token) = token.filter(|token| handlers.contains(*token)) {
                used.insert(token.to_string());
                checked_calls += 1;
                if !top_level_terminated {
                    directives.push(token.to_string());
                }
            }
        }
        missing.extend(check_event(event_key.as_ref(), &directives));
    }

    assert_eq!(used.len(), 74, "actually-used Python handlers changed");
    let unused = handlers
        .difference(&used)
        .map(|directive| (*directive).to_string())
        .collect::<BTreeSet<_>>();
    let expected_unused = [
        "$강화확인4000!",
        "$난이도재진입확인",
        "$난이도재진입확인!",
        "$난이도진입기록",
        "$내공감소",
        "$무공확인!",
        "$비전설정",
        "$비전수련삭제",
        "$속성설정",
        "$순위확인!",
        "$아이템착용확인",
        "$아이템착용확인!",
        "$전투강제시작",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();
    assert_eq!(
        unused, expected_unused,
        "a Python handler changed between active Rhai data and the hot-reload-only audit"
    );
    assert_eq!(
        checked_calls, 5_355,
        "Python directive call inventory changed"
    );
    assert!(
        missing.is_empty(),
        "a Python directive lost its Rhai conversion fingerprint:\n{}",
        missing.join("\n")
    );
}

#[test]
fn event_output_never_leaves_python_particle_markers_literal() {
    // Python `doEvent()` routes lines through postPosition1() whenever a
    // substitution marker is present.  Rhai must either use one of its
    // presentation helpers for a Korean particle marker, or compose the
    // resolved particle explicitly (for example through one_item_owner()).
    let mut paths = Vec::new();
    collect_rhai_paths(Path::new("data/script"), &mut paths);
    let mut literal_markers = Vec::new();
    for path in paths {
        for (line_no, line) in std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
            .lines()
            .enumerate()
        {
            let trimmed = line.trim();
            let particle_marker = [
                "(이/가)",
                "(을/를)",
                "(은/는)",
                "(와/과)",
                "(이라/라)",
                "(으)로",
            ]
            .iter()
            .any(|marker| trimmed.contains(marker));
            if trimmed.starts_with("//") || !trimmed.contains("output(") || !particle_marker {
                continue;
            }
            let resolved = trimmed.contains("post_position_once(")
                || trimmed.contains("room_broadcast_output(")
                || trimmed.contains("self_output(")
                || trimmed.contains("broadcast_output(");
            if !resolved {
                literal_markers.push(format!("{}:{}: {trimmed}", path.display(), line_no + 1));
            }
        }
    }
    assert!(
        literal_markers.is_empty(),
        "Rhai output still contains Python particle markers:\n{}",
        literal_markers.join("\n")
    );
}

#[test]
fn every_active_python_directive_has_named_execution_regression_evidence() {
    // The conversion-fingerprint test above proves that every active legacy
    // directive reaches a Rhai helper.  Keep a second, deliberately explicit
    // ledger to prevent that structural proof from being mistaken for the
    // only coverage: every active type must continue to name the event-engine
    // regression that exercises its state/branch contract.  The names are
    // checked against event.rs so removing or renaming a regression forces an
    // audit of this ledger.
    let evidence = BTreeMap::from([
        (
            "$강화확인2000!",
            "blacksmith_legacy_event_arrays_are_rhai_with_python_olsuk_and_script_handoff",
        ),
        (
            "$기연존재확인",
            "fortune_teller_unique_owner_check_charges_only_when_python_condition_matches",
        ),
        (
            "$기연확인",
            "yuk_jahong_unique_gate_uses_the_python_item_index_not_its_display_name",
        ),
        (
            "$기연확인!",
            "yuk_jahong_unique_gate_uses_the_python_item_index_not_its_display_name",
        ),
        (
            "$남자설정",
            "legacy_item_option_and_gender_directives_change_only_the_selected_state",
        ),
        (
            "$레벨상위확인",
            "level_upper_checks_keep_python_tower_and_arena_boundary_selection",
        ),
        (
            "$레벨상위확인!",
            "level_upper_checks_keep_python_tower_and_arena_boundary_selection",
        ),
        (
            "$몹상태설정",
            "blood_tower_cremation_requires_a_corpse_and_immediately_respawns_it",
        ),
        (
            "$몹상태확인",
            "corpse_gate_keeps_live_combat_and_corpse_reward_paths_separate",
        ),
        (
            "$몹상태확인!",
            "corpse_gate_keeps_live_combat_and_corpse_reward_paths_separate",
        ),
        (
            "$무공개수확인",
            "mirror_and_turtle_keep_python_inner_power_and_skill_count_messages",
        ),
        (
            "$무공리스트삭제",
            "event_skill_removal_keeps_python_first_occurrence_and_training_record",
        ),
        (
            "$무공리스트확인",
            "legacy_skill_directives_keep_all_skill_gates_teaching_and_removal",
        ),
        (
            "$무공리스트확인!",
            "all_source_inverted_skill_list_checks_keep_their_rhai_branches",
        ),
        (
            "$무공시전",
            "event_defense_skill_directives_apply_once_and_keep_their_python_branches",
        ),
        (
            "$무공시전2",
            "event_defense_skill_directives_apply_once_and_keep_their_python_branches",
        ),
        (
            "$무공전수",
            "legacy_skill_directives_keep_all_skill_gates_teaching_and_removal",
        ),
        (
            "$무공확인",
            "skill_gated_events_keep_their_python_failure_output",
        ),
        (
            "$무공회수",
            "event_skill_removal_keeps_python_first_occurrence_and_training_record",
        ),
        (
            "$무림별호조건",
            "wang_daehyup_nickname_tendency_branches_match_all_three_source_conditions",
        ),
        (
            "$변수확인",
            "legacy_variable_check_uses_python_argument_index_including_the_mob_target",
        ),
        (
            "$별호변경",
            "nickname_change_event_releases_old_name_reserves_new_name_and_requests_return",
        ),
        (
            "$비무관람끝",
            "martial_arts_spectator_directives_keep_python_noop_state",
        ),
        (
            "$비무관람시작",
            "martial_arts_spectator_directives_keep_python_noop_state",
        ),
        (
            "$비전수련가능확인!",
            "vision_training_event_restores_python_allowlist_prerequisites_and_skill_consumption",
        ),
        (
            "$비전수련설정",
            "every_vision_trainer_prerequisite_list_is_consumed_on_its_success_path",
        ),
        (
            "$비전수련설정확인",
            "every_vision_trainer_prerequisite_list_is_consumed_on_its_success_path",
        ),
        (
            "$비전수련확인",
            "vision_training_event_restores_python_allowlist_prerequisites_and_skill_consumption",
        ),
        (
            "$비전종류확인",
            "vision_training_event_restores_python_allowlist_prerequisites_and_skill_consumption",
        ),
        (
            "$비전종류확인!",
            "jade_emperor_vision_prerequisites_skip_only_already_learned_blocks",
        ),
        (
            "$성별확인",
            "legacy_item_option_and_gender_directives_change_only_the_selected_state",
        ),
        (
            "$소오강호설정",
            "sogo_river_chain_starts_final_fight_awards_head_and_retires_player",
        ),
        (
            "$속성템주기",
            "lottery_event_uses_full_legacy_pool_and_marks_the_single_reward_untradeable",
        ),
        (
            "$순위갱신",
            "legacy_rank_record_broadcasts_only_when_the_player_becomes_first",
        ),
        (
            "$순위기록",
            "legacy_rank_record_broadcasts_only_when_the_player_becomes_first",
        ),
        (
            "$순위확인",
            "iron_bell_rank_view_resolves_python_rank_placeholders_in_rhai",
        ),
        (
            "$스크립트호출",
            "blacksmith_legacy_event_arrays_are_rhai_with_python_olsuk_and_script_handoff",
        ),
        (
            "$아이템삭제",
            "legacy_item_directives_share_python_add_and_delete_item_rules_with_rhai",
        ),
        (
            "$아이템속성확인!",
            "blacksmith_decomposition_keeps_python_dynamic_item_guard_order",
        ),
        (
            "$아이템옵션삭제",
            "legacy_item_option_and_gender_directives_change_only_the_selected_state",
        ),
        (
            "$아이템종류확인",
            "all_source_item_kind_directives_keep_their_rhai_rejection_predicates",
        ),
        (
            "$아이템주기",
            "event_item_grant_preserves_python_single_count_magic_and_money_subtraction",
        ),
        (
            "$아이템확인",
            "information_clerk_item_fee_gate_uses_python_inverted_item_check",
        ),
        (
            "$아이템확인!",
            "dynamic_unworn_name_check_keeps_python_quantity_and_currency_rules",
        ),
        (
            "$아이템확장설정",
            "craft_name_extension_events_keep_python_item_and_money_order",
        ),
        (
            "$아이템확장설정지움",
            "craft_name_extension_events_keep_python_item_and_money_order",
        ),
        (
            "$아이템확장확인",
            "craft_name_extension_events_keep_python_item_and_money_order",
        ),
        (
            "$아이템확장확인!",
            "craft_name_extension_events_keep_python_item_and_money_order",
        ),
        (
            "$엔터$",
            "frog_child_enter_sequence_finishes_after_the_legacy_interactive_end_marker",
        ),
        (
            "$여자설정",
            "legacy_item_option_and_gender_directives_change_only_the_selected_state",
        ),
        (
            "$올숙자격확인",
            "blacksmith_legacy_event_arrays_are_rhai_with_python_olsuk_and_script_handoff",
        ),
        (
            "$올숙확인",
            "blacksmith_legacy_event_arrays_are_rhai_with_python_olsuk_and_script_handoff",
        ),
        (
            "$올숙확인!",
            "blacksmith_legacy_event_arrays_are_rhai_with_python_olsuk_and_script_handoff",
        ),
        (
            "$우화등선설정",
            "ascension_scripts_restore_source_mp_status_rank_and_celestial_reward",
        ),
        (
            "$위치이동",
            "event_position_move_prefixes_the_zone_with_the_python_mob_difficulty",
        ),
        (
            "$은둔칩거설정",
            "eundun_event_keeps_python_reset_and_transfer_state_before_moving",
        ),
        (
            "$이벤트삭제",
            "event_flags_round_trip_as_python_arrays_and_keep_all_loaded_entries",
        ),
        (
            "$이벤트설정",
            "event_flags_round_trip_as_python_arrays_and_keep_all_loaded_entries",
        ),
        (
            "$이벤트확인",
            "legacy_event_checks_keep_their_rhai_predicates_and_negation",
        ),
        (
            "$이벤트확인!",
            "legacy_event_checks_keep_their_rhai_predicates_and_negation",
        ),
        (
            "$입력대기출력끝$",
            "frog_child_enter_sequence_finishes_after_the_legacy_interactive_end_marker",
        ),
        (
            "$전투시작",
            "legacy_combat_start_keeps_python_failure_order_and_only_starts_once",
        ),
        (
            "$정사전환",
            "wang_daehyup_tendency_switch_requires_the_source_head_and_toggles_once",
        ),
        (
            "$중급수련",
            "intermediate_training_applies_only_python_training_stats_after_combat_starts",
        ),
        (
            "$착용확인!",
            "blacksmith_decomposition_keeps_python_dynamic_item_guard_order",
        ),
        (
            "$체력감소",
            "migrated_hp_loss_directives_keep_the_python_fallback_branches",
        ),
        (
            "$체력소모",
            "migrated_hp_loss_directives_keep_the_python_fallback_branches",
        ),
        (
            "$출력",
            "legacy_script_output_keeps_python_self_and_same_room_renderings",
        ),
        (
            "$특성치변경",
            "stat_change_keeps_python_lp_prompt_boundary_before_later_event_text",
        ),
        (
            "$특성치복사",
            "legacy_stat_copy_changes_selected_mob_combat_attributes",
        ),
        (
            "$특성치복사고",
            "low_and_high_stat_copy_variants_match_python_multipliers",
        ),
        (
            "$특성치복사저",
            "low_and_high_stat_copy_variants_match_python_multipliers",
        ),
        (
            "$특성치설정",
            "legacy_stat_set_replaces_the_python_named_attribute",
        ),
        (
            "$특성치확인",
            "milestone_dialogues_keep_python_threshold_failure_messages",
        ),
    ]);

    let python = std::fs::read_to_string("objs/event.py").expect("Python event source");
    let handlers = python
        .split("func == '")
        .skip(1)
        .filter_map(|tail| tail.split_once('\'').map(|(name, _)| name))
        .filter(|name| name.starts_with('$'))
        .collect::<BTreeSet<_>>();
    let mut mob_paths = Vec::new();
    collect_mob_paths(Path::new("data/mob"), &mut mob_paths);
    let mut active = BTreeSet::new();
    for path in mob_paths {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        for line in source.lines() {
            if let Some(token) = line
                .trim()
                .strip_prefix(':')
                .filter(|directive| directive.starts_with('$'))
                .and_then(|directive| directive.split_whitespace().next())
                .filter(|token| handlers.contains(*token))
            {
                active.insert(token.to_owned());
            }
        }
    }
    let ledger = evidence
        .keys()
        .map(|directive| (*directive).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ledger, active,
        "active directive execution-evidence ledger changed"
    );

    let event_tests = std::fs::read_to_string("src/world/event.rs").expect("event regressions");
    for (directive, regression) in evidence {
        assert!(
            event_tests.contains(&format!("fn {regression}(")),
            "{directive} references missing execution regression {regression}"
        );
    }
}
