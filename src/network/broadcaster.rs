//! Broadcast functionality for MUD server
//!
//! Manages connected clients and broadcasts messages to them.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::SocketAddr;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::network::social::{SocialAction, SocialSnapshot, SocialState};
use crate::network::{client::ClientState, Client};

/// Check if a Korean syllable ends with a consonant (받침)
fn has_batchilm(name: &str) -> bool {
    name.chars().last().is_some_and(|c| {
        let code = c as u32;
        // Korean syllables range from AC00 (가) to D7A3 (힣)
        // (code - 0xAC00) % 28 gives the 받침 index (0 = no 받침, 1-27 = 받침)
        (0xAC00..=0xD7A3).contains(&code) && !(code - 0xAC00).is_multiple_of(28)
    })
}

/// Replace player name templates in message
/// - [공](이라/라) → "{name}이라" or "{name}라"
/// - [공](아/야) → "{name}아" or "{name}야"
/// - [공](이/가) → "{name}이" or "{name}가"
/// - [공] → character name
fn replace_player_templates(message: &str, player_name: &str) -> String {
    let mut result = message.to_string();

    // Handle [공](이라/라)
    if result.contains("[공](이라/라)") {
        let particle = if has_batchilm(player_name) {
            "이라"
        } else {
            "라"
        };
        result = result.replace("[공](이라/라)", &format!("{}{}", player_name, particle));
    }

    // Handle [공](아/야)
    if result.contains("[공](아/야)") {
        let particle = if has_batchilm(player_name) {
            "아"
        } else {
            "야"
        };
        result = result.replace("[공](아/야)", &format!("{}{}", player_name, particle));
    }

    // Handle [공](이/가)
    if result.contains("[공](이/가)") {
        let particle = if has_batchilm(player_name) {
            "이"
        } else {
            "가"
        };
        result = result.replace("[공](이/가)", &format!("{}{}", player_name, particle));
    }

    // Handle standalone [공]
    result = result.replace("[공]", player_name);
    result
}

fn split_room_id(room_id: &str) -> Option<(&str, &str)> {
    room_id
        .rsplit_once(':')
        .or_else(|| room_id.rsplit_once('/'))
        .filter(|(zone, room)| !zone.is_empty() && !room.is_empty())
}

/// Broadcaster manages all connected clients
///
/// Provides thread-safe access to the client list and broadcast functionality.
pub struct Broadcaster {
    /// Map of connected clients by their socket address
    pub clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    /// Python `Client.players`/channel player iteration order.
    client_order: Arc<Mutex<Vec<SocketAddr>>>,
    /// Connected player identity lookup.
    ///
    /// Room-local paths first obtain the insertion-ordered player names from
    /// `WorldState`, then resolve only those names through this index.  A name
    /// can be rebound during duplicate login, so removal is always conditional
    /// on both the name and the old socket address.
    player_clients: Arc<Mutex<HashMap<String, SocketAddr>>>,
    /// Opaque Python Player-object identity to its current socket.
    connection_clients: Arc<Mutex<HashMap<String, SocketAddr>>>,
    /// Runtime-only follower and Party object relationships.
    social: Arc<Mutex<SocialState>>,
    /// Python `Player.adultCH` compatibility state.
    ///
    /// The original is a process-wide list of Player objects, so membership
    /// is keyed by the connection identity (not by a possibly duplicated
    /// character name) and retains join order.
    adult_channel: Arc<Mutex<Vec<SocketAddr>>>,
    #[cfg(test)]
    indexed_name_lookups: Arc<AtomicUsize>,
}

impl Broadcaster {
    /// Create a new broadcaster with an empty client list
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            client_order: Arc::new(Mutex::new(Vec::new())),
            player_clients: Arc::new(Mutex::new(HashMap::new())),
            connection_clients: Arc::new(Mutex::new(HashMap::new())),
            social: Arc::new(Mutex::new(SocialState::default())),
            adult_channel: Arc::new(Mutex::new(Vec::new())),
            #[cfg(test)]
            indexed_name_lookups: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a new broadcaster with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
            client_order: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
            player_clients: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
            connection_clients: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
            social: Arc::new(Mutex::new(SocialState::default())),
            adult_channel: Arc::new(Mutex::new(Vec::new())),
            #[cfg(test)]
            indexed_name_lookups: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Return adult-channel members in the same insertion order as
    /// Python's `Player.adultCH` list.
    pub(crate) fn adult_channel_members(&self) -> Vec<SocketAddr> {
        self.adult_channel.lock().clone()
    }

    pub(crate) fn is_adult_channel_member(&self, addr: SocketAddr) -> bool {
        self.adult_channel.lock().contains(&addr)
    }

    /// Append once, matching `adultCH.append(ob)` after the Python command's
    /// duplicate-membership check.
    pub(crate) fn join_adult_channel(&self, addr: SocketAddr) -> bool {
        let mut members = self.adult_channel.lock();
        if members.contains(&addr) {
            return false;
        }
        members.push(addr);
        true
    }

    /// Remove the matching connection identity, matching
    /// `adultCH.remove(ob)` and logout cleanup.
    pub(crate) fn leave_adult_channel(&self, addr: SocketAddr) -> bool {
        let mut members = self.adult_channel.lock();
        let Some(index) = members.iter().position(|member| *member == addr) else {
            return false;
        };
        members.remove(index);
        true
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.lock().len()
    }

    /// Check if a specific client is connected
    pub fn has_client(&self, addr: SocketAddr) -> bool {
        self.clients.lock().contains_key(&addr)
    }

    /// Add a client to the broadcaster
    pub fn add_client(&self, client: Client) {
        let addr = client.addr;
        let new_token = client.connection_token.clone();
        let new_name = client
            .player
            .as_ref()
            .map(|player| player.body.get_name())
            .filter(|name| !name.is_empty());
        let replaced = self.clients.lock().insert(addr, client);
        if replaced.is_none() {
            self.client_order.lock().push(addr);
        }
        let old_token = replaced.as_ref().map(|old| old.connection_token.clone());
        let old_name = replaced
            .and_then(|old| old.player.map(|player| player.body.get_name()))
            .filter(|name| !name.is_empty());

        if let Some(old_name) = old_name {
            self.unbind_player_name_if_matches(&old_name, addr);
        }
        if let Some(old_token) = old_token {
            self.unbind_connection_token_if_matches(&old_token, addr);
            self.social
                .lock()
                .apply(&old_token, SocialAction::Disconnect);
        }
        self.connection_clients.lock().insert(new_token, addr);
        if let Some(new_name) = new_name {
            self.bind_player_name(&new_name, addr);
        }
    }

    /// Remove a client from the broadcaster
    pub fn remove_client(&self, addr: SocketAddr) -> Option<Client> {
        let removed = self.clients.lock().remove(&addr);
        if removed.is_some() {
            self.client_order.lock().retain(|saved| *saved != addr);
        }
        if let Some(token) = removed
            .as_ref()
            .map(|client| client.connection_token.clone())
        {
            self.unbind_connection_token_if_matches(&token, addr);
            // Normal logout already ran the Rhai-authored notification plan;
            // broken transports still require state-only identity cleanup.
            self.social.lock().apply(&token, SocialAction::Disconnect);
        }
        if let Some(name) = removed
            .as_ref()
            .and_then(|client| client.player.as_ref())
            .map(|player| player.body.get_name())
            .filter(|name| !name.is_empty())
        {
            self.unbind_player_name_if_matches(&name, addr);
        }
        self.leave_adult_channel(addr);
        removed
    }

    pub(crate) fn client_addresses_in_order(&self) -> Vec<SocketAddr> {
        self.client_order.lock().clone()
    }

    fn unbind_connection_token_if_matches(&self, token: &str, addr: SocketAddr) -> bool {
        let mut index = self.connection_clients.lock();
        if index.get(token).copied() != Some(addr) {
            return false;
        }
        index.remove(token);
        true
    }

    pub(crate) fn find_addr_by_connection_token(&self, token: &str) -> Option<SocketAddr> {
        let addr = self.connection_clients.lock().get(token).copied()?;
        let valid = self
            .clients
            .lock()
            .get(&addr)
            .is_some_and(|client| client.connection_token == token);
        if valid {
            Some(addr)
        } else {
            self.unbind_connection_token_if_matches(token, addr);
            None
        }
    }

    /// Resolve only the requested connection identities, preserving their
    /// Python list order and never scanning unrelated clients.
    pub(crate) fn connection_bindings_for_tokens(
        &self,
        tokens: &[String],
    ) -> Vec<(String, SocketAddr)> {
        let index = self.connection_clients.lock();
        tokens
            .iter()
            .filter_map(|token| index.get(token).copied().map(|addr| (token.clone(), addr)))
            .collect()
    }

    pub(crate) fn social_snapshot(&self, token: &str) -> SocialSnapshot {
        self.social.lock().snapshot(token)
    }

    pub(crate) fn has_social_relations(&self, token: &str) -> bool {
        self.social.lock().has_relations(token)
    }

    pub(crate) fn apply_social_action(&self, token: &str, action: SocialAction) -> bool {
        self.social.lock().apply(token, action)
    }

    pub(crate) fn movement_follower_tokens(&self, token: &str) -> Vec<String> {
        self.social.lock().movement_followers(token)
    }

    /// Bind a player name after login attaches a Player to an existing client.
    /// Rebinding the same name intentionally replaces the old connection.
    pub(crate) fn bind_player_name(&self, player_name: &str, addr: SocketAddr) {
        if player_name.is_empty() {
            return;
        }
        let matches_client = self
            .clients
            .lock()
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            .is_some_and(|player| player.body.get_name() == player_name);
        if !matches_client {
            return;
        }
        self.player_clients
            .lock()
            .insert(player_name.to_string(), addr);
    }

    fn unbind_player_name_if_matches(&self, player_name: &str, addr: SocketAddr) -> bool {
        let mut index = self.player_clients.lock();
        if index.get(player_name).copied() != Some(addr) {
            return false;
        }
        index.remove(player_name);
        true
    }

    /// Resolve an insertion-ordered room name list without visiting unrelated
    /// connected clients.
    pub(crate) fn player_bindings_for_names(
        &self,
        player_names: &[String],
    ) -> Vec<(String, SocketAddr)> {
        #[cfg(test)]
        self.indexed_name_lookups
            .fetch_add(player_names.len(), Ordering::Relaxed);

        let index = self.player_clients.lock();
        player_names
            .iter()
            .filter_map(|name| index.get(name).copied().map(|addr| (name.clone(), addr)))
            .collect()
    }

    /// Broadcast a message to all connected clients
    /// Replaces [공] templates with each player's name
    ///
    /// If `exclude` is Some, that client will not receive the message.
    pub fn broadcast(&self, message: &str, exclude: Option<SocketAddr>) {
        let clients = self.clients.lock();
        let mut dead_addrs = Vec::new();

        for (&addr, client) in clients.iter() {
            if Some(addr) != exclude {
                // Get player name and replace templates for each client
                let player_name = client
                    .player
                    .as_ref()
                    .map(|p| {
                        let name = p.body.get_string("이름");
                        if name.is_empty() {
                            "방문자".to_string()
                        } else {
                            name
                        }
                    })
                    .unwrap_or_else(|| "방문자".to_string());

                let processed_message = replace_player_templates(message, &player_name);

                if let Err(_e) = client.sender.send(processed_message) {
                    // Send failed - client's send task likely exited (broken pipe)
                    debug!(
                        "Failed to send to {} (connection dead), marking for cleanup",
                        addr
                    );
                    dead_addrs.push(addr);
                }
            }
        }

        // Clean up dead clients to prevent memory leaks and cascading errors.
        // `remove_client` also conditionally clears the player-name index.
        drop(clients);
        for addr in dead_addrs {
            warn!(
                "Removing dead client {} due to send failure (broken pipe)",
                addr
            );
            self.remove_client(addr);
        }
    }

    /// Broadcast a message to all active clients
    /// Replaces [공] templates with each player's name
    ///
    /// Only clients with `ClientState::Active` will receive the message.
    pub fn broadcast_active(&self, message: &str, exclude: Option<SocketAddr>) {
        let clients = self.clients.lock();
        let mut dead_addrs = Vec::new();

        for (&addr, client) in clients.iter() {
            if Some(addr) != exclude && client.state == ClientState::Active {
                // Get player name and replace templates for each client
                let player_name = client
                    .player
                    .as_ref()
                    .map(|p| {
                        let name = p.body.get_string("이름");
                        if name.is_empty() {
                            "방문자".to_string()
                        } else {
                            name
                        }
                    })
                    .unwrap_or_else(|| "방문자".to_string());

                let processed_message = replace_player_templates(message, &player_name);

                if let Err(_e) = client.sender.send(processed_message) {
                    // Send failed - client's send task likely exited (broken pipe)
                    debug!(
                        "Failed to send to {} (connection dead), marking for cleanup",
                        addr
                    );
                    dead_addrs.push(addr);
                }
            }
        }

        // Clean up dead clients to prevent memory leaks and cascading errors.
        drop(clients);
        for addr in dead_addrs {
            warn!(
                "Removing dead client {} due to send failure (broken pipe)",
                addr
            );
            self.remove_client(addr);
        }
    }

    /// Send a message to a specific client
    /// Replaces [공] templates with player's name
    pub fn send_to(&self, addr: SocketAddr, message: &str) -> Result<(), String> {
        let clients = self.clients.lock();

        if let Some(client) = clients.get(&addr) {
            // Get player name and replace templates
            let player_name = client
                .player
                .as_ref()
                .map(|p| {
                    let name = p.body.get_string("이름");
                    if name.is_empty() {
                        "방문자".to_string()
                    } else {
                        name
                    }
                })
                .unwrap_or_else(|| "방문자".to_string());

            let processed_message = replace_player_templates(message, &player_name);

            client
                .sender
                .send(processed_message)
                .map_err(|e| format!("Failed to send: {}", e))
        } else {
            Err(format!("Client {} not found", addr))
        }
    }

    /// Get a list of all connected client addresses
    pub fn client_addresses(&self) -> Vec<SocketAddr> {
        self.clients.lock().keys().copied().collect()
    }

    /// Find SocketAddr for a connected player by name. 당신을 살펴봅니다 전송용.
    pub fn find_addr_by_player_name(&self, player_name: &str) -> Option<SocketAddr> {
        let addr = self.player_clients.lock().get(player_name).copied()?;
        let valid = self
            .clients
            .lock()
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            .is_some_and(|player| player.body.get_name() == player_name);
        if valid {
            Some(addr)
        } else {
            self.unbind_player_name_if_matches(player_name, addr);
            None
        }
    }

    /// Send a message to a player by name. call_out 점프_착지 등에서 사용.
    pub fn send_to_by_player_name(&self, player_name: &str, message: &str) -> Result<(), String> {
        if let Some(addr) = self.find_addr_by_player_name(player_name) {
            self.send_to(addr, message)
        } else {
            Err(format!("Player '{}' not found", player_name))
        }
    }

    /// Run a closure on a player's Body by name. call_out 점프_착지에서 cooltime 해제 등.
    pub fn with_player_body_by_name<F, R>(&self, player_name: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::player::Body) -> R,
    {
        let addr = self.find_addr_by_player_name(player_name)?;
        let mut clients = self.clients.lock();
        let client = clients.get_mut(&addr)?;
        let player = client.player.as_mut()?;
        Some(f(&mut player.body))
    }

    /// Request a client to disconnect (sends sentinel to their channel; e.g. kick on duplicate login).
    /// Caller should also send an informative message to the user before calling this.
    pub fn request_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        use crate::network::client::DISCONNECT_SENTINEL;
        let clients = self.clients.lock();
        if let Some(client) = clients.get(&addr) {
            client
                .sender
                .send(DISCONNECT_SENTINEL.to_string())
                .map_err(|e| format!("Failed to send disconnect: {}", e))
        } else {
            Err(format!("Client {} not found", addr))
        }
    }

    /// Send a message to all clients in a room (environment)
    ///
    /// Filters clients by their current location (위치 field in player data).
    /// room_id format: "area:room_number" (e.g., "낙양성:42", "시작/시작")
    pub fn tell_room(&self, room_id: &str, message: &str, exclude: Option<SocketAddr>) {
        if message.is_empty() {
            return;
        }
        let Some((zone, room)) = split_room_id(room_id) else {
            return;
        };
        let names = crate::world::get_world_state()
            .read()
            .unwrap()
            .get_players_in_room(zone, room);
        let bindings = self.player_bindings_for_names(&names);
        let clients = self.clients.lock();
        let mut dead_addrs = Vec::new();

        for (indexed_name, addr) in bindings {
            if Some(addr) == exclude {
                continue;
            }
            let Some(client) = clients.get(&addr) else {
                continue;
            };
            if client
                .player
                .as_ref()
                .is_none_or(|player| player.body.get_name() != indexed_name)
            {
                continue;
            }
            let processed_message = replace_player_templates(message, &indexed_name);
            if let Err(_e) = client.sender.send(processed_message) {
                // Send failed - client's send task likely exited (broken pipe)
                debug!(
                    "Failed to send to {} (connection dead), marking for cleanup",
                    addr
                );
                dead_addrs.push(addr);
            }
        }

        // Clean up dead clients to prevent memory leaks and cascading errors.
        drop(clients);
        for addr in dead_addrs {
            warn!(
                "Removing dead client {} due to send failure (broken pipe)",
                addr
            );
            self.remove_client(addr);
        }
    }

    /// Get all players in a room with their full data
    ///
    /// Returns a list of PlayerRoomData containing name, level, HP, guild, rank, etc.
    /// room_id format: "area:room_number" (e.g., "낙양성:42")
    pub fn get_players_in_room_with_data(
        &self,
        room_id: &str,
    ) -> Vec<crate::world::PlayerRoomData> {
        use crate::world::PlayerRoomData;

        let Some((zone, room)) = split_room_id(room_id) else {
            return Vec::new();
        };
        let names = crate::world::get_world_state()
            .read()
            .unwrap()
            .get_players_in_room(zone, room);
        let bindings = self.player_bindings_for_names(&names);
        let clients = self.clients.lock();
        let mut players = Vec::new();

        for (indexed_name, addr) in bindings {
            let Some(player) = clients
                .get(&addr)
                .and_then(|client| client.player.as_ref())
                .filter(|player| player.body.get_name() == indexed_name)
            else {
                continue;
            };
            let level = player.body.get_int("레벨");
            let hp = player.body.get_hp();
            let max_hp = player.body.get_max_hp();
            // Python-compatible player attribute key is `소속`; `길드` is
            // not used by the legacy Body object and would silently drop the
            // guild from live snapshots.
            let guild = player.body.get_string("소속");
            let rank = player.body.get_string("직위");
            let act_state = format!("{:?}", player.body.act);

            players.push(PlayerRoomData {
                name: indexed_name,
                level,
                hp,
                max_hp,
                guild: if guild.is_empty() { None } else { Some(guild) },
                rank: if rank.is_empty() { None } else { Some(rank) },
                act_state,
            });
        }

        players
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Clone implementation for Broadcaster
///
/// This creates a new Broadcaster sharing the same client list.
impl Clone for Broadcaster {
    fn clone(&self) -> Self {
        Self {
            clients: self.clients.clone(),
            client_order: self.client_order.clone(),
            player_clients: self.player_clients.clone(),
            connection_clients: self.connection_clients.clone(),
            social: self.social.clone(),
            adult_channel: self.adult_channel.clone(),
            #[cfg(test)]
            indexed_name_lookups: self.indexed_name_lookups.clone(),
        }
    }
}

/// Broadcast helper functions
///
/// These functions mirror the Python lib/comm.py functionality.
pub struct Broadcast;

impl Broadcast {
    /// Broadcast a message to all clients (equivalent to Python's `broadcast()`)
    ///
    /// If `exclude` is Some, that client will not receive the message.
    pub fn broadcast(broadcaster: &Broadcaster, message: &str, exclude: Option<SocketAddr>) {
        if message.is_empty() {
            return;
        }
        broadcaster.broadcast(message, exclude);
    }

    /// Send a message to all clients in a room (equivalent to Python's `tell_room()`)
    pub fn tell_room(
        broadcaster: &Broadcaster,
        room_id: &str,
        message: &str,
        exclude: Option<SocketAddr>,
    ) {
        if message.is_empty() {
            return;
        }
        broadcaster.tell_room(room_id, message, exclude);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_order_matches_python_append_remove_and_same_address_replace() {
        let broadcaster = Broadcaster::new();
        let first: SocketAddr = "127.0.0.1:18101".parse().unwrap();
        let second: SocketAddr = "127.0.0.1:18102".parse().unwrap();
        let (first_tx, _) = tokio::sync::mpsc::unbounded_channel();
        let (second_tx, _) = tokio::sync::mpsc::unbounded_channel();
        broadcaster.add_client(Client::new(first, first_tx));
        broadcaster.add_client(Client::new(second, second_tx));
        assert_eq!(broadcaster.client_addresses_in_order(), vec![first, second]);

        let (replacement_tx, _) = tokio::sync::mpsc::unbounded_channel();
        broadcaster.add_client(Client::new(first, replacement_tx));
        assert_eq!(broadcaster.client_addresses_in_order(), vec![first, second]);
        broadcaster.remove_client(first);
        assert_eq!(broadcaster.client_addresses_in_order(), vec![second]);
    }

    fn named_client(addr: SocketAddr, name: &str) -> Client {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut client = Client::new(addr, sender);
        let mut player = crate::player::Player::new();
        player.body.set("이름", name);
        client.player = Some(player);
        client
    }

    fn named_client_with_receiver(
        addr: SocketAddr,
        name: &str,
    ) -> (Client, tokio::sync::mpsc::UnboundedReceiver<String>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut client = Client::new(addr, sender);
        let mut player = crate::player::Player::new();
        player.body.set("이름", name);
        client.player = Some(player);
        (client, receiver)
    }

    #[test]
    fn test_broadcaster_new() {
        let broadcaster = Broadcaster::new();
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[test]
    fn test_broadcaster_with_capacity() {
        let broadcaster = Broadcaster::with_capacity(10);
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[test]
    fn test_broadcaster_default() {
        let broadcaster = Broadcaster::default();
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[test]
    fn test_broadcaster_clone() {
        let broadcaster1 = Broadcaster::new();
        let broadcaster2 = broadcaster1.clone();
        assert!(Arc::ptr_eq(&broadcaster1.clients, &broadcaster2.clients));
        assert!(Arc::ptr_eq(
            &broadcaster1.player_clients,
            &broadcaster2.player_clients
        ));
        assert!(Arc::ptr_eq(
            &broadcaster1.connection_clients,
            &broadcaster2.connection_clients
        ));
        assert!(Arc::ptr_eq(&broadcaster1.social, &broadcaster2.social));
        assert!(Arc::ptr_eq(
            &broadcaster1.adult_channel,
            &broadcaster2.adult_channel
        ));
    }

    #[test]
    fn add_bind_and_remove_keep_player_index_in_sync() {
        let broadcaster = Broadcaster::new();
        let addr: SocketAddr = "127.0.0.1:19111".parse().unwrap();

        broadcaster.add_client(named_client(addr, "인덱스검사"));
        assert_eq!(
            broadcaster.find_addr_by_player_name("인덱스검사"),
            Some(addr)
        );

        assert!(broadcaster.remove_client(addr).is_some());
        assert_eq!(broadcaster.find_addr_by_player_name("인덱스검사"), None);
    }

    #[test]
    fn login_can_bind_a_player_attached_after_add_client() {
        let broadcaster = Broadcaster::new();
        let addr: SocketAddr = "127.0.0.1:19112".parse().unwrap();
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        broadcaster.add_client(Client::new(addr, sender));

        let mut player = crate::player::Player::new();
        player.body.set("이름", "로그인인덱스");
        broadcaster.clients.lock().get_mut(&addr).unwrap().player = Some(player);
        broadcaster.bind_player_name("로그인인덱스", addr);

        assert_eq!(
            broadcaster.find_addr_by_player_name("로그인인덱스"),
            Some(addr)
        );
    }

    #[test]
    fn old_duplicate_disconnect_does_not_unbind_new_connection() {
        let broadcaster = Broadcaster::new();
        let old_addr: SocketAddr = "127.0.0.1:19113".parse().unwrap();
        let new_addr: SocketAddr = "127.0.0.1:19114".parse().unwrap();

        broadcaster.add_client(named_client(old_addr, "재접속검사"));
        broadcaster.add_client(named_client(new_addr, "재접속검사"));
        assert_eq!(
            broadcaster.find_addr_by_player_name("재접속검사"),
            Some(new_addr)
        );

        assert!(broadcaster.remove_client(old_addr).is_some());
        assert_eq!(
            broadcaster.find_addr_by_player_name("재접속검사"),
            Some(new_addr),
            "old-session cleanup must only unbind a matching (name, addr) pair"
        );
    }

    #[test]
    fn duplicate_names_keep_distinct_social_object_identities_on_old_disconnect() {
        let broadcaster = Broadcaster::new();
        let leader_addr: SocketAddr = "127.0.0.1:19131".parse().unwrap();
        let old_addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let new_addr: SocketAddr = "127.0.0.1:19133".parse().unwrap();

        let leader = named_client(leader_addr, "재접속대장");
        let leader_token = leader.connection_token.clone();
        let old = named_client(old_addr, "재접속동일이름");
        let old_token = old.connection_token.clone();
        let new = named_client(new_addr, "재접속동일이름");
        let new_token = new.connection_token.clone();
        broadcaster.add_client(leader);
        broadcaster.add_client(old);
        broadcaster.add_client(new);

        assert!(broadcaster.apply_social_action(
            &old_token,
            SocialAction::Follow {
                target: leader_token.clone(),
            },
        ));
        assert!(broadcaster.apply_social_action(
            &new_token,
            SocialAction::Follow {
                target: leader_token.clone(),
            },
        ));
        assert_eq!(
            broadcaster.social_snapshot(&leader_token).followers,
            vec![old_token.clone(), new_token.clone()]
        );

        assert!(broadcaster.remove_client(old_addr).is_some());
        assert_eq!(
            broadcaster.social_snapshot(&leader_token).followers,
            vec![new_token.clone()]
        );
        assert_eq!(
            broadcaster.find_addr_by_connection_token(&new_token),
            Some(new_addr)
        );
        assert_eq!(
            broadcaster.find_addr_by_player_name("재접속동일이름"),
            Some(new_addr)
        );
    }

    #[test]
    fn broken_connection_cleanup_uses_the_same_index_and_membership_lifecycle() {
        let broadcaster = Broadcaster::new();
        let addr: SocketAddr = "127.0.0.1:19115".parse().unwrap();
        // `named_client` drops its receiver, so the broadcast send fails.
        broadcaster.add_client(named_client(addr, "끊김검사"));
        assert!(broadcaster.join_adult_channel(addr));

        broadcaster.broadcast("probe", None);

        assert!(!broadcaster.has_client(addr));
        assert_eq!(broadcaster.find_addr_by_player_name("끊김검사"), None);
        assert!(!broadcaster.is_adult_channel_member(addr));
    }

    #[test]
    fn temporary_two_client_transaction_keeps_existing_name_bindings() {
        let broadcaster = Broadcaster::new();
        let first_addr: SocketAddr = "127.0.0.1:19116".parse().unwrap();
        let second_addr: SocketAddr = "127.0.0.1:19117".parse().unwrap();
        broadcaster.add_client(named_client(first_addr, "거래첫째"));
        broadcaster.add_client(named_client(second_addr, "거래둘째"));

        // The give path temporarily removes both entries while retaining the
        // one clients lock, then restores them before releasing it.  This is
        // not a connection lifecycle event, so the name index stays bound.
        let mut clients = broadcaster.clients.lock();
        let first = clients.remove(&first_addr).unwrap();
        let second = clients.remove(&second_addr).unwrap();
        clients.insert(first_addr, first);
        clients.insert(second_addr, second);
        drop(clients);

        assert_eq!(
            broadcaster.find_addr_by_player_name("거래첫째"),
            Some(first_addr)
        );
        assert_eq!(
            broadcaster.find_addr_by_player_name("거래둘째"),
            Some(second_addr)
        );
    }

    #[test]
    fn ordered_name_lookup_visits_only_requested_room_members() {
        let broadcaster = Broadcaster::new();
        for number in 0..64u16 {
            let addr: SocketAddr = format!("127.0.0.1:{}", 19200 + number).parse().unwrap();
            broadcaster.add_client(named_client(addr, &format!("다른방{number}")));
        }
        let first_addr: SocketAddr = "127.0.0.1:19301".parse().unwrap();
        let second_addr: SocketAddr = "127.0.0.1:19302".parse().unwrap();
        broadcaster.add_client(named_client(first_addr, "첫째"));
        broadcaster.add_client(named_client(second_addr, "둘째"));

        broadcaster
            .indexed_name_lookups
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let bindings =
            broadcaster.player_bindings_for_names(&["둘째".to_string(), "첫째".to_string()]);

        assert_eq!(
            bindings,
            vec![
                ("둘째".to_string(), second_addr),
                ("첫째".to_string(), first_addr)
            ]
        );
        assert_eq!(
            broadcaster
                .indexed_name_lookups
                .load(std::sync::atomic::Ordering::Relaxed),
            2,
            "unrelated connected clients must not be visited"
        );
    }

    #[test]
    fn room_delivery_and_data_visit_only_indexed_room_names_in_world_order() {
        let broadcaster = Broadcaster::new();
        let first_addr: SocketAddr = "127.0.0.1:19311".parse().unwrap();
        let second_addr: SocketAddr = "127.0.0.1:19312".parse().unwrap();
        let elsewhere_addr: SocketAddr = "127.0.0.1:19313".parse().unwrap();
        let (first, mut first_rx) = named_client_with_receiver(first_addr, "방조회첫째");
        let (second, mut second_rx) = named_client_with_receiver(second_addr, "방조회둘째");
        let (elsewhere, mut elsewhere_rx) =
            named_client_with_receiver(elsewhere_addr, "방조회다른방");
        broadcaster.add_client(first);
        broadcaster.add_client(second);
        broadcaster.add_client(elsewhere);
        {
            let mut world = crate::world::get_world_state().write().unwrap();
            world.set_player_position(
                "방조회첫째",
                crate::world::PlayerPosition::new("인덱스시험존".into(), "1".into()),
            );
            world.set_player_position(
                "방조회둘째",
                crate::world::PlayerPosition::new("인덱스시험존".into(), "1".into()),
            );
            world.set_player_position(
                "방조회다른방",
                crate::world::PlayerPosition::new("인덱스시험존".into(), "2".into()),
            );
        }

        broadcaster
            .indexed_name_lookups
            .store(0, std::sync::atomic::Ordering::Relaxed);
        broadcaster.tell_room("인덱스시험존:1", "방 전용", None);
        assert_eq!(first_rx.try_recv().unwrap(), "방 전용");
        assert_eq!(second_rx.try_recv().unwrap(), "방 전용");
        assert!(elsewhere_rx.try_recv().is_err());
        assert_eq!(
            broadcaster
                .indexed_name_lookups
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );

        broadcaster
            .indexed_name_lookups
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let data = broadcaster.get_players_in_room_with_data("인덱스시험존/1");
        assert_eq!(
            data.into_iter()
                .map(|player| player.name)
                .collect::<Vec<_>>(),
            ["방조회첫째", "방조회둘째"]
        );
        assert_eq!(
            broadcaster
                .indexed_name_lookups
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );

        let mut world = crate::world::get_world_state().write().unwrap();
        world.remove_player_position("방조회첫째");
        world.remove_player_position("방조회둘째");
        world.remove_player_position("방조회다른방");
    }

    #[test]
    fn adult_channel_uses_connection_identity_and_preserves_join_order() {
        let broadcaster = Broadcaster::new();
        let first: SocketAddr = "127.0.0.1:19101".parse().unwrap();
        let second: SocketAddr = "127.0.0.1:19102".parse().unwrap();

        assert!(broadcaster.join_adult_channel(first));
        assert!(broadcaster.join_adult_channel(second));
        assert!(!broadcaster.join_adult_channel(first));
        assert_eq!(broadcaster.adult_channel_members(), vec![first, second]);

        assert!(broadcaster.leave_adult_channel(first));
        assert!(!broadcaster.leave_adult_channel(first));
        assert_eq!(broadcaster.adult_channel_members(), vec![second]);
    }

    #[test]
    fn test_empty_message_no_broadcast() {
        let broadcaster = Broadcaster::new();
        // Should not panic even with no clients
        Broadcast::broadcast(&broadcaster, "", None);
        Broadcast::broadcast(&broadcaster, "test", None);
    }

    #[test]
    fn test_get_players_in_room_with_data_empty() {
        let broadcaster = Broadcaster::new();
        // Should return empty list when no clients
        let players = broadcaster.get_players_in_room_with_data("낙양성:42");
        assert!(players.is_empty());
    }
}
