//! Broadcast functionality for MUD server
//!
//! Manages connected clients and broadcasts messages to them.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::debug;

use crate::network::{client::ClientState, Client};

/// Check if a Korean syllable ends with a consonant (받침)
fn has_batchilm(name: &str) -> bool {
    name.chars().last().map_or(false, |c| {
        let code = c as u32;
        // Korean syllables range from AC00 (가) to D7A3 (힣)
        // (code - 0xAC00) % 28 gives the 받침 index (0 = no 받침, 1-27 = 받침)
        (0xAC00..=0xD7A3).contains(&code) && ((code - 0xAC00) % 28) > 0
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
        let particle = if has_batchilm(player_name) { "이라" } else { "라" };
        result = result.replace("[공](이라/라)", &format!("{}{}", player_name, particle));
    }

    // Handle [공](아/야)
    if result.contains("[공](아/야)") {
        let particle = if has_batchilm(player_name) { "아" } else { "야" };
        result = result.replace("[공](아/야)", &format!("{}{}", player_name, particle));
    }

    // Handle [공](이/가)
    if result.contains("[공](이/가)") {
        let particle = if has_batchilm(player_name) { "이" } else { "가" };
        result = result.replace("[공](이/가)", &format!("{}{}", player_name, particle));
    }

    // Handle standalone [공]
    result = result.replace("[공]", player_name);
    result
}

/// Broadcaster manages all connected clients
///
/// Provides thread-safe access to the client list and broadcast functionality.
pub struct Broadcaster {
    /// Map of connected clients by their socket address
    pub clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
}

impl Broadcaster {
    /// Create a new broadcaster with an empty client list
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new broadcaster with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
        }
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
        self.clients.lock().insert(addr, client);
    }

    /// Remove a client from the broadcaster
    pub fn remove_client(&self, addr: SocketAddr) -> Option<Client> {
        self.clients.lock().remove(&addr)
    }

    /// Broadcast a message to all connected clients
    /// Replaces [공] templates with each player's name
    ///
    /// If `exclude` is Some, that client will not receive the message.
    pub fn broadcast(&self, message: &str, exclude: Option<SocketAddr>) {
        let clients = self.clients.lock();

        for (&addr, client) in clients.iter() {
            if Some(addr) != exclude {
                // Get player name and replace templates for each client
                let player_name = client.player.as_ref()
                    .map(|p| {
                        let name = p.body.get_string("이름");
                        if name.is_empty() { "방문자".to_string() } else { name }
                    })
                    .unwrap_or_else(|| "방문자".to_string());

                let processed_message = replace_player_templates(message, &player_name);

                if let Err(e) = client.sender.send(processed_message) {
                    debug!("Failed to send to {}: {}", addr, e);
                }
            }
        }
    }

    /// Broadcast a message to all active clients
    /// Replaces [공] templates with each player's name
    ///
    /// Only clients with `ClientState::Active` will receive the message.
    pub fn broadcast_active(&self, message: &str, exclude: Option<SocketAddr>) {
        let clients = self.clients.lock();

        for (&addr, client) in clients.iter() {
            if Some(addr) != exclude && client.state == ClientState::Active {
                // Get player name and replace templates for each client
                let player_name = client.player.as_ref()
                    .map(|p| {
                        let name = p.body.get_string("이름");
                        if name.is_empty() { "방문자".to_string() } else { name }
                    })
                    .unwrap_or_else(|| "방문자".to_string());

                let processed_message = replace_player_templates(message, &player_name);

                if let Err(e) = client.sender.send(processed_message) {
                    debug!("Failed to send to {}: {}", addr, e);
                }
            }
        }
    }

    /// Send a message to a specific client
    /// Replaces [공] templates with player's name
    pub fn send_to(&self, addr: SocketAddr, message: &str) -> Result<(), String> {
        let clients = self.clients.lock();

        if let Some(client) = clients.get(&addr) {
            // Get player name and replace templates
            let player_name = client.player.as_ref()
                .map(|p| {
                    let name = p.body.get_string("이름");
                    if name.is_empty() { "방문자".to_string() } else { name }
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
        let clients = self.clients.lock();
        for (&addr, client) in clients.iter() {
            if let Some(ref p) = client.player {
                if p.body.get_name() == player_name {
                    return Some(addr);
                }
            }
        }
        None
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
    /// This is a placeholder for future room-based messaging.
    pub fn tell_room(&self, _room_id: &str, message: &str, exclude: Option<SocketAddr>) {
        // TODO: Implement room-based filtering
        // For now, just broadcast to all
        self.broadcast(message, exclude);
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
    pub fn broadcast(
        broadcaster: &Broadcaster,
        message: &str,
        exclude: Option<SocketAddr>,
    ) {
        if message.is_empty() {
            return;
        }
        broadcaster.broadcast(message, exclude);
    }

    /// Send a message to all clients in a room (equivalent to Python's `tell_room()`)
    ///
    /// For now, this broadcasts to all clients since room management
    /// will be handled at a higher level.
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

    /// Send a message with "say" formatting (equivalent to Python's `say()`)
    pub fn say(
        broadcaster: &Broadcaster,
        speaker_addr: SocketAddr,
        speaker_name: &str,
        message: &str,
    ) {
        if message.is_empty() {
            return;
        }

        let clients = broadcaster.clients.lock();

        for (&addr, client) in clients.iter() {
            let msg = if addr == speaker_addr {
                format!("you say: {}", message)
            } else {
                format!("{} says: {}", speaker_name, message)
            };

            let _ = client.sender.send(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn test_empty_message_no_broadcast() {
        let broadcaster = Broadcaster::new();
        // Should not panic even with no clients
        Broadcast::broadcast(&broadcaster, "", None);
        Broadcast::broadcast(&broadcaster, "test", None);
    }
}
