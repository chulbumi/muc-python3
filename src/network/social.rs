//! Runtime follower/party relationships matching Python `Player` object identity.
//!
//! These relationships are deliberately not persisted as Body string fields.
//! A connection token represents one Python Player object lifetime, while the
//! vectors retain Python list insertion order.

use std::collections::HashMap;

pub(crate) type ConnectionId = String;
pub(crate) const MAX_PARTY_SIZE: usize = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RelationState {
    pub follow: Option<ConnectionId>,
    pub party_leader: Option<ConnectionId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SocialSnapshot {
    pub self_id: ConnectionId,
    pub follow: Option<ConnectionId>,
    pub followers: Vec<ConnectionId>,
    pub party_leader: Option<ConnectionId>,
    pub party_members: Vec<ConnectionId>,
    /// Direct relation lookups for the actor and only its related objects.
    pub relations: HashMap<ConnectionId, RelationState>,
    pub combat_targets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SocialAction {
    Follow {
        target: ConnectionId,
    },
    StopFollowing,
    LeaveParty,
    AddPartyMembers {
        members: Vec<ConnectionId>,
    },
    RemoveFollowers {
        members: Vec<ConnectionId>,
    },
    Disconnect,
    SetCombatTargets {
        owner: ConnectionId,
        targets: Vec<String>,
    },
    SetPartyCombatTargets {
        assignments: Vec<(ConnectionId, Vec<String>)>,
        target_instances: Vec<u64>,
        tanker: Option<ConnectionId>,
        prepend_all: bool,
    },
}

#[derive(Debug, Default)]
pub(crate) struct SocialState {
    /// Python `ob.follow`.
    follow_target: HashMap<ConnectionId, ConnectionId>,
    /// Python `ob.follower`, retaining append order.
    followers: HashMap<ConnectionId, Vec<ConnectionId>>,
    /// Python `ob.Party`; leaders map to themselves.
    party_leader: HashMap<ConnectionId, ConnectionId>,
    /// Python leader `PartyMember`, excluding the leader in stable state.
    party_members: HashMap<ConnectionId, Vec<ConnectionId>>,
    combat_targets: HashMap<ConnectionId, Vec<String>>,
}

impl SocialState {
    pub(crate) fn snapshot(&self, actor: &str) -> SocialSnapshot {
        let follow = self.follow_target.get(actor).cloned();
        let followers = self.followers.get(actor).cloned().unwrap_or_default();
        let party_leader = self.party_leader.get(actor).cloned();
        let party_members = party_leader
            .as_ref()
            .and_then(|leader| self.party_members.get(leader))
            .cloned()
            .unwrap_or_default();

        let mut related = vec![actor.to_string()];
        if let Some(target) = &follow {
            push_once(&mut related, target);
        }
        for follower in &followers {
            push_once(&mut related, follower);
        }
        if let Some(leader) = &party_leader {
            push_once(&mut related, leader);
        }
        for member in &party_members {
            push_once(&mut related, member);
        }

        let relations = related
            .into_iter()
            .map(|id| {
                let state = RelationState {
                    follow: self.follow_target.get(&id).cloned(),
                    party_leader: self.party_leader.get(&id).cloned(),
                };
                (id, state)
            })
            .collect();

        SocialSnapshot {
            self_id: actor.to_string(),
            follow,
            followers,
            party_leader,
            party_members,
            relations,
            combat_targets: self.combat_targets.get(actor).cloned().unwrap_or_default(),
        }
    }

    pub(crate) fn has_relations(&self, actor: &str) -> bool {
        self.follow_target.contains_key(actor)
            || self
                .followers
                .get(actor)
                .is_some_and(|followers| !followers.is_empty())
            || self.party_leader.contains_key(actor)
            || self
                .party_members
                .get(actor)
                .is_some_and(|members| !members.is_empty())
            || self
                .combat_targets
                .get(actor)
                .is_some_and(|targets| !targets.is_empty())
    }

    /// Python `enterRoom` iterates the leader object's follower list without
    /// rechecking each follower's reverse `follow` pointer.
    pub(crate) fn movement_followers(&self, leader: &str) -> Vec<ConnectionId> {
        self.followers.get(leader).cloned().unwrap_or_default()
    }

    pub(crate) fn apply(&mut self, actor: &str, action: SocialAction) -> bool {
        match action {
            SocialAction::Follow { target } => self.follow(actor, &target),
            SocialAction::StopFollowing => self.stop_following(actor).is_some(),
            SocialAction::LeaveParty => self.leave_party_member(actor),
            SocialAction::AddPartyMembers { members } => {
                self.add_party_members(actor, &members);
                true
            }
            SocialAction::RemoveFollowers { members } => self.remove_followers(actor, &members),
            SocialAction::Disconnect => {
                self.disconnect(actor);
                true
            }
            SocialAction::SetCombatTargets { owner, targets } => {
                if owner != actor
                    && !self
                        .party_members
                        .get(actor)
                        .is_some_and(|members| members.contains(&owner))
                {
                    return false;
                }
                self.combat_targets.insert(owner, targets);
                true
            }
            SocialAction::SetPartyCombatTargets { assignments, .. } => {
                if assignments.iter().any(|(owner, _)| {
                    owner != actor
                        && !self
                            .party_members
                            .get(actor)
                            .is_some_and(|members| members.contains(owner))
                }) {
                    return false;
                }
                for (owner, targets) in assignments {
                    self.combat_targets.insert(owner, targets);
                }
                true
            }
        }
    }

    fn follow(&mut self, actor: &str, target: &str) -> bool {
        if actor.is_empty() || target.is_empty() {
            return false;
        }
        self.stop_following(actor);
        self.follow_target
            .insert(actor.to_string(), target.to_string());
        let followers = self.followers.entry(target.to_string()).or_default();
        if !followers.iter().any(|id| id == actor) {
            followers.push(actor.to_string());
        }
        true
    }

    fn stop_following(&mut self, actor: &str) -> Option<ConnectionId> {
        let target = self.follow_target.remove(actor)?;
        remove_from_ordered(self.followers.get_mut(&target), actor);
        remove_empty(&mut self.followers, &target);
        Some(target)
    }

    /// `따라 나` while a normal party member assigns `follow = None` and
    /// removes the member specifically from the party leader's lists.  It does
    /// not call `delFollow`, so an already-inconsistent other reverse list is
    /// intentionally left untouched.
    fn leave_party_member(&mut self, actor: &str) -> bool {
        let Some(leader) = self.party_leader.get(actor).cloned() else {
            return false;
        };
        if leader == actor {
            return false;
        }

        self.follow_target.remove(actor);
        remove_from_ordered(self.followers.get_mut(&leader), actor);
        remove_empty(&mut self.followers, &leader);
        self.party_leader.remove(actor);
        self.combat_targets.remove(actor);
        remove_from_ordered(self.party_members.get_mut(&leader), actor);
        if self
            .party_members
            .get(&leader)
            .is_none_or(|members| members.is_empty())
        {
            self.party_members.remove(&leader);
            self.party_leader.remove(&leader);
        }
        true
    }

    fn add_party_members(&mut self, leader: &str, members: &[ConnectionId]) {
        self.party_leader
            .insert(leader.to_string(), leader.to_string());
        self.party_members.entry(leader.to_string()).or_default();

        for member in members {
            if self
                .party_members
                .get(leader)
                .map_or(1, |party| party.len().saturating_add(1))
                >= MAX_PARTY_SIZE
            {
                break;
            }
            if self.follow_target.get(member).map(String::as_str) != Some(leader)
                || self.party_leader.contains_key(member)
            {
                continue;
            }
            self.party_leader.insert(member.clone(), leader.to_string());
            let party = self.party_members.entry(leader.to_string()).or_default();
            if !party.contains(member) {
                party.push(member.clone());
            }
        }
    }

    fn remove_followers(&mut self, leader: &str, requested: &[ConnectionId]) -> bool {
        let mut removed = false;
        for member in requested {
            let is_follower = self
                .followers
                .get(leader)
                .is_some_and(|followers| followers.contains(member));
            if !is_follower {
                continue;
            }

            self.follow_target.remove(member);
            self.combat_targets.remove(member);
            remove_from_ordered(self.followers.get_mut(leader), member);
            let is_party_member = self
                .party_members
                .get(leader)
                .is_some_and(|members| members.contains(member));
            if is_party_member {
                self.party_leader.remove(member);
                remove_from_ordered(self.party_members.get_mut(leader), member);
            }
            removed = true;
        }
        remove_empty(&mut self.followers, leader);

        if self.party_leader.get(leader).map(String::as_str) == Some(leader)
            && self
                .party_members
                .get(leader)
                .is_none_or(|members| members.is_empty())
        {
            self.party_members.remove(leader);
            self.party_leader.remove(leader);
        }
        removed
    }

    /// Apply `Player.logout()` in its original order: party succession first,
    /// then `delFollow()`, then `delFollower()`.
    fn disconnect(&mut self, actor: &str) {
        if let Some(leader) = self.party_leader.get(actor).cloned() {
            if leader == actor {
                let old_members = self.party_members.get(actor).cloned().unwrap_or_default();
                if let Some(new_leader) = old_members.first().cloned() {
                    let mut new_members = old_members.clone();
                    let mut copied_followers =
                        self.followers.get(actor).cloned().unwrap_or_default();

                    for member in &new_members {
                        self.party_leader.insert(member.clone(), new_leader.clone());
                    }
                    remove_value(&mut new_members, &new_leader);
                    remove_value(&mut copied_followers, &new_leader);

                    if new_members.is_empty() {
                        self.party_members.remove(&new_leader);
                        self.party_leader.remove(&new_leader);
                    } else {
                        self.party_leader
                            .insert(new_leader.clone(), new_leader.clone());
                        self.party_members.insert(new_leader.clone(), new_members);
                    }
                    if copied_followers.is_empty() {
                        self.followers.remove(&new_leader);
                    } else {
                        // Python copies this list before the old leader calls
                        // delFollower(), leaving the same asymmetric state.
                        self.followers.insert(new_leader.clone(), copied_followers);
                    }
                }
            } else {
                remove_from_ordered(self.party_members.get_mut(&leader), actor);
                self.party_leader.remove(actor);
                self.combat_targets.remove(actor);
                if self
                    .party_members
                    .get(&leader)
                    .is_none_or(|members| members.is_empty())
                {
                    self.party_members.remove(&leader);
                    self.party_leader.remove(&leader);
                }
            }
        }

        self.stop_following(actor);

        let old_followers = self.followers.get(actor).cloned().unwrap_or_default();
        for follower in old_followers {
            // `f.delFollow(True)` runs only when f.follow is not None and uses
            // that actual target's delFollower method.
            self.stop_following(&follower);
        }

        self.follow_target.remove(actor);
        self.followers.remove(actor);
        self.party_leader.remove(actor);
        self.party_members.remove(actor);
        self.combat_targets.remove(actor);
    }
}

fn push_once(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn remove_value(values: &mut Vec<String>, value: &str) {
    if let Some(index) = values.iter().position(|entry| entry == value) {
        values.remove(index);
    }
}

fn remove_from_ordered(values: Option<&mut Vec<String>>, value: &str) {
    if let Some(values) = values {
        remove_value(values, value);
    }
}

fn remove_empty(map: &mut HashMap<String, Vec<String>>, key: &str) {
    if map.get(key).is_some_and(Vec::is_empty) {
        map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn following_uses_connection_identity_and_retains_follower_order() {
        let mut state = SocialState::default();
        assert!(state.apply(
            "첫째",
            SocialAction::Follow {
                target: "대장".into()
            }
        ));
        assert!(state.apply(
            "둘째",
            SocialAction::Follow {
                target: "대장".into()
            }
        ));
        assert_eq!(state.movement_followers("대장"), ids(&["첫째", "둘째"]));

        assert!(state.apply(
            "첫째",
            SocialAction::Follow {
                target: "새대장".into()
            }
        ));
        assert_eq!(state.movement_followers("대장"), ids(&["둘째"]));
        assert_eq!(state.movement_followers("새대장"), ids(&["첫째"]));
    }

    #[test]
    fn combat_targets_are_scoped_to_party_members() {
        let mut state = SocialState::default();
        assert!(state.apply(
            "무사",
            SocialAction::Follow {
                target: "대장".into()
            }
        ));
        assert!(state.apply(
            "대장",
            SocialAction::AddPartyMembers {
                members: ids(&["무사"])
            }
        ));
        assert!(state.apply(
            "대장",
            SocialAction::SetCombatTargets {
                owner: "무사".into(),
                targets: ids(&["낙양성:몹"])
            }
        ));
        assert_eq!(state.snapshot("무사").combat_targets, ids(&["낙양성:몹"]));
        assert!(!state.apply(
            "외부인",
            SocialAction::SetCombatTargets {
                owner: "무사".into(),
                targets: ids(&["다른몹"])
            }
        ));
    }

    #[test]
    fn python_allows_a_player_object_to_follow_itself() {
        let mut state = SocialState::default();
        assert!(state.apply(
            "자기",
            SocialAction::Follow {
                target: "자기".into(),
            },
        ));
        let snapshot = state.snapshot("자기");
        assert_eq!(snapshot.follow.as_deref(), Some("자기"));
        assert_eq!(snapshot.followers, ids(&["자기"]));

        assert!(state.apply("자기", SocialAction::StopFollowing));
        let snapshot = state.snapshot("자기");
        assert!(snapshot.follow.is_none());
        assert!(snapshot.followers.is_empty());
    }

    #[test]
    fn party_members_preserve_selected_follower_order() {
        let mut state = SocialState::default();
        for member in ["하나", "둘", "셋"] {
            state.apply(
                member,
                SocialAction::Follow {
                    target: "대장".into(),
                },
            );
        }
        state.apply(
            "대장",
            SocialAction::AddPartyMembers {
                members: ids(&["둘", "하나"]),
            },
        );

        let snapshot = state.snapshot("하나");
        assert_eq!(snapshot.party_leader.as_deref(), Some("대장"));
        assert_eq!(snapshot.party_members, ids(&["둘", "하나"]));

        state.apply(
            "대장",
            SocialAction::RemoveFollowers {
                members: ids(&["둘"]),
            },
        );
        assert_eq!(state.snapshot("대장").party_members, ids(&["하나"]));
        assert_eq!(state.snapshot("둘").party_leader, None);
        assert_eq!(state.snapshot("대장").followers, ids(&["하나", "셋"]));
    }

    #[test]
    fn empty_add_party_request_keeps_python_solo_leader_state() {
        let mut state = SocialState::default();
        assert!(state.apply(
            "혼자대장",
            SocialAction::AddPartyMembers { members: vec![] },
        ));
        let snapshot = state.snapshot("혼자대장");
        assert_eq!(snapshot.party_leader.as_deref(), Some("혼자대장"));
        assert!(snapshot.party_members.is_empty());
    }

    #[test]
    fn party_is_limited_to_leader_plus_three_members() {
        let mut state = SocialState::default();
        for member in ["일", "이", "삼", "사"] {
            state.apply(
                member,
                SocialAction::Follow {
                    target: "대장".into(),
                },
            );
        }
        state.apply(
            "대장",
            SocialAction::AddPartyMembers {
                members: ids(&["일", "이", "삼", "사"]),
            },
        );
        assert_eq!(
            state.snapshot("대장").party_members,
            ids(&["일", "이", "삼"])
        );
        assert_eq!(state.snapshot("사").party_leader, None);
    }

    #[test]
    fn leader_disconnect_promotes_first_member_before_python_follow_cleanup() {
        let mut state = SocialState::default();
        for member in ["첫후계", "둘째", "동행"] {
            state.apply(
                member,
                SocialAction::Follow {
                    target: "옛대장".into(),
                },
            );
        }
        state.apply(
            "옛대장",
            SocialAction::AddPartyMembers {
                members: ids(&["첫후계", "둘째"]),
            },
        );

        state.apply("옛대장", SocialAction::Disconnect);

        let successor = state.snapshot("첫후계");
        assert_eq!(successor.party_leader.as_deref(), Some("첫후계"));
        assert_eq!(successor.party_members, ids(&["둘째"]));
        assert_eq!(
            successor.followers,
            ids(&["둘째", "동행"]),
            "Python copies the old follower list before delFollower clears reverse pointers"
        );
        assert_eq!(state.snapshot("둘째").follow, None);
        assert_eq!(state.snapshot("동행").follow, None);
        assert!(!state.has_relations("옛대장"));
    }

    #[test]
    fn last_member_logout_dissolves_party_but_keeps_leader_followers_consistent() {
        let mut state = SocialState::default();
        state.apply(
            "구성원",
            SocialAction::Follow {
                target: "대장".into(),
            },
        );
        state.apply(
            "대장",
            SocialAction::AddPartyMembers {
                members: ids(&["구성원"]),
            },
        );

        state.apply("구성원", SocialAction::Disconnect);

        assert_eq!(state.snapshot("대장").party_leader, None);
        assert!(state.snapshot("대장").followers.is_empty());
    }
}
