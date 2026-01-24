//! Client protocol handling for MUD connections
//!
//! Manages individual TCP client connections with line-based protocol
//! and login flow.

use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use bytes::BytesMut;
use tracing::{debug, error, info, warn};
use serde::Deserialize;
use once_cell::sync::Lazy;

use rhai::{Array, Dynamic, Map};

use crate::emotion::{self, EmotionTarget};
use crate::network::DelimiterCodec;
use crate::player::{Player, STATE_ACTIVE};
use crate::command::{CommandParser, CommandRegistry, CommandResult};
use crate::command::commands::try_move_by_exit_name;
use crate::world::item::{get_item_display_name, get_item_weight_by_key};
use crate::world::{get_world_state, RoomCache};
use std::collections::HashMap;
use crate::script::{build_room_objs_grouped, format_room_objs_display, set_precomputed_all_online, clear_precomputed_all_online, save_body_to_json, load_body_from_json, password_hash, password_verify, load_user_password_hash};

/// Client connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Client is connected but not yet logged in
    Inactive,
    /// Client is fully authenticated and active
    Active,
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Inactive
    }
}

/// Sentinel sent to a client's channel to request the send task to exit (kicks the connection).
pub(crate) const DISCONNECT_SENTINEL: &str = "\x1b__DISCONNECT__\x1b";

/// Login state machine for client connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginState {
    /// Initial state - show logo and ask for name
    Logo,
    /// Waiting for name input
    Name,
    /// Waiting for password input (for existing user)
    Password,
    /// Same name already connected: wait for "기존 접속 종료할까요? (네/아니오)"
    AskKickExisting,
    /// Showing notice after login
    Notice,
    /// Login complete, entering game
    Complete,
    /// Script-based character creation mode
    ScriptMode,
}

/// Login session data
struct LoginSession {
    /// Current login state
    state: LoginState,
    /// Player name (entered by user)
    name: String,
    /// Password attempts
    attempts: u32,
    /// Character creation data
    char_name: String,
    char_password: String,
    char_gender: String,
    /// Script mode: 0=none, 1=quick_helper, 2=initial_helper
    script_mode: u8,
    /// Script position (index in script lines)
    script_position: usize,
    /// Waiting for specific command (for $키입:<명령>)
    waiting_for_command: Option<String>,
    /// What data are we waiting for? (name, password, gender, or none)
    waiting_for_data: Option<&'static str>,
    /// Accumulated delay during $출력시작 block (in ms)
    accumulated_delay: u64,
    /// Delay to apply after next output (from $틱:N commands)
    delay_after_output: u64,
}

impl LoginSession {
    fn new() -> Self {
        Self {
            state: LoginState::Logo,
            name: String::new(),
            attempts: 0,
            char_name: String::new(),
            char_password: String::new(),
            char_gender: String::new(),
            script_mode: 0,
            script_position: 0,
            waiting_for_command: None,
            waiting_for_data: None,
            accumulated_delay: 0,
            delay_after_output: 0,
        }
    }

    /// Get the current number of password attempts
    fn get_attempts(&self) -> u32 {
        self.attempts
    }
}

/// Represents a connected client's metadata
///
/// The actual writer is managed in the handler task, and messages
/// are sent via the sender channel.
pub struct Client {
    /// Client's socket address
    pub addr: SocketAddr,
    /// Buffer for incoming data
    pub buffer: BytesMut,
    /// Codec for parsing delimiters
    pub codec: DelimiterCodec,
    /// Client state
    pub state: ClientState,
    /// Channel for sending messages to this client
    pub sender: mpsc::UnboundedSender<String>,
    /// Login session data (Some during login, None after complete)
    login_session: Option<LoginSession>,
    /// Player data (Some after login complete)
    pub player: Option<Player>,
}

impl Client {
    /// Create a new client metadata
    pub fn new(addr: SocketAddr, sender: mpsc::UnboundedSender<String>) -> Self {
        Self {
            addr,
            buffer: BytesMut::with_capacity(1024),
            codec: DelimiterCodec::new(),
            state: ClientState::Inactive,
            sender,
            login_session: Some(LoginSession::new()),
            player: None,
        }
    }

    /// Send a message to this client
    pub fn send(&self, message: String) -> Result<(), mpsc::error::SendError<String>> {
        self.sender.send(message)
    }

    /// Get the sender channel for this client
    pub fn get_sender(&self) -> mpsc::UnboundedSender<String> {
        self.sender.clone()
    }

    /// Check if client is still in login phase
    pub fn is_logging_in(&self) -> bool {
        self.login_session.is_some()
    }

    /// Get mutable reference to login session
    pub fn login_session_mut(&mut self) -> Option<&mut LoginSession> {
        self.login_session.as_mut()
    }

    /// Complete login and remove session
    pub fn complete_login(&mut self) {
        self.login_session = None;
        self.state = ClientState::Active;
    }

    /// Set the player for this client
    pub fn set_player(&mut self, player: Player) {
        self.player = Some(player);
    }

    /// Get mutable reference to player
    pub fn player_mut(&mut self) -> Option<&mut Player> {
        self.player.as_mut()
    }

    /// Get reference to player
    pub fn player(&self) -> Option<&Player> {
        self.player.as_ref()
    }

    /// Get the player's name
    pub fn player_name(&self) -> String {
        self.player.as_ref()
            .map(|p| p.body.get_string("이름"))
            .unwrap_or_else(|| "방문자".to_string())
    }
}

/// Read a text file from the data/text directory
fn read_text_file(filename: &str) -> Result<String, std::io::Error> {
    let mut path = PathBuf::from("data/text");
    path.push(filename);
    std::fs::read_to_string(&path)
}

/// Send text content to client with proper formatting
async fn send_text_file(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send the content as-is (files already have proper formatting)
    writer.write_all(content.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

/// Send a prompt to the client
async fn send_prompt(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    writer.write_all(prompt.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

/// Handle a client connection
///
/// This function runs in a task for each connected client,
/// handling incoming data and managing the client lifecycle.
/// `shutdown_notify`: 셧다운 명령 시 서버 종료 트리거용. None이면 셧다운 명령은 no-op.
pub async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    broadcaster: Arc<crate::network::Broadcaster>,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
    shutdown_notify: Option<Arc<tokio::sync::Notify>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Client connected from {}", addr);

    // Split the stream into owned reader and writer
    // Use into_split() to get owned halves that can be moved into spawned tasks
    let (reader, writer) = stream.into_split();

    let mut codec = DelimiterCodec::new();

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Create client and add to broadcaster
    {
        let mut clients = broadcaster.clients.lock();
        let client = Client::new(addr, tx.clone());
        clients.insert(addr, client);
    }

    // Clone tx for use in the read loop
    let tx_clone = tx.clone();

    // Spawn task to handle sending messages to client
    let send_task = tokio::spawn(async move {
        let mut writer = writer;
        let _login_complete = false;

        while let Some(msg) = rx.recv().await {
            // Disconnect sentinel: break to close connection (e.g. kick on duplicate login)
            if msg == DISCONNECT_SENTINEL {
                break;
            }
            // Don't add extra \r\n - messages already have proper line endings
            if let Err(e) = writer.write_all(msg.as_bytes()).await {
                error!("Failed to send to {}: {}", addr, e);
                break;
            }
            if let Err(e) = writer.flush().await {
                error!("Failed to flush to {}: {}", addr, e);
                break;
            }
            // Send prompt only after login is complete
            // Note: For now, prompts are sent by the read loop after each command
        }
    });

    // Main read loop - handle login flow
    let mut reader = reader;
    let mut read_buf = [0u8; 1024];

    // Start login flow by sending logo
    // Check if client exists before starting async work
    let client_exists = {
        let clients = broadcaster.clients.lock();
        clients.get(&addr).is_some()
    };
    // Lock is dropped here

    if client_exists {
        if let Err(e) = send_logo_and_name_prompt(&broadcaster, addr).await {
            error!("Failed to send logo to {}: {}", addr, e);
        }
    }

    'read_loop: loop {
        match reader.read(&mut read_buf).await {
            Ok(0) => {
                // Connection closed by peer
                debug!("Client {} closed connection", addr);
                break;
            }
            Ok(n) => {
                let data = &read_buf[..n];
                debug!("Received {} bytes from {}: {:?}", n, addr, data);

                // Parse lines from data
                match codec.feed_data(data) {
                    Ok(lines) => {
                        for line in lines {
                            let line = line.trim();
                            info!("Line from {}: '{}' (len={}, bytes={:?})", addr, line, line.len(), line.as_bytes());

                            // Check for quit command (works at any stage): quit, 끝, 종료
                            let is_quit = line.to_lowercase() == "quit" || line == "끝" || line == "종료";
                            if is_quit {
                                info!("Client {} requested quit: {}", addr, line);
                                let _ = tx_clone.send("Goodbye!\r\n".to_string());
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                break 'read_loop;
                            }

                            // Process login state machine
                            let should_disconnect = process_login_state(
                                &broadcaster,
                                addr,
                                line,
                                &tx_clone,
                                command_registry.clone(),
                                room_cache.clone(),
                                shutdown_notify.clone(),
                            ).await?;

                            if should_disconnect {
                                break 'read_loop;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Codec error for {}: {}", addr, e);
                        let _ = tx_clone.send(format!("\r\nError: {}\r\n", e));
                    }
                }
            }
            Err(e) => {
                error!("Error reading from {}: {}", addr, e);
                break;
            }
        }
    }

    // Cleanup: clients에서 제거, send_task 종료( TCP writer 정리) → 연결 종료
    {
        let mut clients = broadcaster.clients.lock();
        clients.remove(&addr);
    }
    send_task.abort();
    info!("Client {} disconnected", addr);

    Ok(())
}

/// Process the login state machine
///
/// Returns Ok(true) if client should disconnect
async fn process_login_state(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    input: &str,
    _tx: &mpsc::UnboundedSender<String>,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
    shutdown_notify: Option<Arc<tokio::sync::Notify>>,
) -> Result<bool, Box<dyn std::error::Error>> {
    // First, check if the client is still in login phase
    // Do this in a separate scope to ensure lock is released
    let is_logged_in = {
        let clients = broadcaster.clients.lock();
        clients.get(&addr)
            .map(|c| c.login_session.is_none())
            .unwrap_or(false)
    };
    // Lock is released here

    if is_logged_in {
        // Already logged in, handle as game command
        handle_game_command(broadcaster, addr, input, command_registry, room_cache, shutdown_notify).await?;
        return Ok(false);
    }

    // Now we're in login phase - get the next action to take
    // We must drop the lock before any await points
    let next_action: LoginAction = {
        let mut clients = broadcaster.clients.lock();
        let (name, state) = clients
            .get(&addr)
            .and_then(|c| c.login_session.as_ref())
            .map(|s| (s.name.clone(), s.state))
            .unwrap_or((String::new(), LoginState::Logo));
        let has_duplicate = state == LoginState::Password
            && clients.iter().any(|(a, c)| {
                *a != addr
                    && c.player
                        .as_ref()
                        .map(|p| p.body.get_string("이름"))
                        .as_deref()
                        == Some(name.as_str())
            });

        let client = clients.get_mut(&addr);
        if client.is_none() {
            return Ok(false);
        }

        let client = client.unwrap();
        let session = client.login_session_mut();

        if session.is_none() {
            // Should not reach here since we checked above
            return Ok(false);
        }

        let session = session.unwrap();

        // Update state based on current state and input
        // Return what action to take after releasing the lock
        match state {
            LoginState::Logo => {
                // Should not receive input in Logo state
                session.state = LoginState::Name;
                LoginAction::None
            }
            LoginState::Name => {
                let input_name = input.to_string();
                session.name = input_name.clone();
                let is_korean = crate::hangul::is_han(&input_name);
                let is_special = input_name == "손님" || input_name == "무명객" || input_name == "나만바라바";

                info!("Name validation: name='{}', is_korean={}, is_special={}, bytes={:?}",
                    input_name, is_korean, is_special, input_name.as_bytes());

                if input_name.is_empty() {
                    // Will ask again for name
                    LoginAction::AskName
                } else if !is_korean {
                    // Name must be Korean only - return error action
                    session.state = LoginState::Name; // Stay in Name state
                    LoginAction::NameError(input_name)
                } else if is_special {
                    // Directly start helper script mode (like production server)
                    session.state = LoginState::ScriptMode;
                    // "나만바라바" uses quick helper (script_mode=1), others use initial helper (script_mode=2)
                    session.script_mode = if input_name == "나만바라바" { 1 } else { 2 };
                    session.script_position = 0;
                    LoginAction::StartScript
                } else {
                    session.state = LoginState::Password;
                    LoginAction::AskPassword(input_name)
                }
            }
            LoginState::Password => {
                session.attempts += 1;
                let stored = load_user_password_hash(&name);
                let ok = stored.as_ref().map_or(false, |s| password_verify(s, input));

                if !ok {
                    // 암호 틀림: 3회면 접속 끊기, 아니면 재입력
                    if session.attempts >= 3 {
                        LoginAction::PasswordWrongDisconnect
                    } else {
                        LoginAction::AskPasswordRetry
                    }
                } else {
                    session.attempts = 0;
                    if has_duplicate {
                        session.state = LoginState::AskKickExisting;
                        LoginAction::AskKickExisting(name)
                    } else {
                        session.state = LoginState::Notice;
                        LoginAction::ShowNotice
                    }
                }
            }
            LoginState::AskKickExisting => {
                let t = input.trim();
                let yes = matches!(t, "네" | "예" | "y" | "Y" | "yes" | "YES");
                let no = matches!(t, "아니오" | "아니" | "n" | "N" | "no" | "NO");
                if yes {
                    LoginAction::KickExistingAndProceed(name)
                } else if no {
                    LoginAction::DisconnectSelf
                } else {
                    LoginAction::AskKickExistingRetry(name)
                }
            }
            LoginState::Notice => {
                session.state = LoginState::Complete;
                LoginAction::EnterGame(name)
            }
            LoginState::Complete => {
                LoginAction::GameCommand(name)
            }
            LoginState::ScriptMode => {
                // Handle script-based character creation
                handle_script_state(session, input)
            }
        }
    };
    // Lock is dropped here before any await points

    // Now handle the state transitions without holding the lock
    match next_action {
        LoginAction::None => Ok(false),
        LoginAction::AskName => {
            send_prompt_raw(broadcaster, addr, "\r\n무림존함ː").await?;
            Ok(false)
        }
        LoginAction::StartScript => {
            // Clear screen and start the script
            broadcaster.send_to(addr, "\x1b[0;37;40m\x1b[H\x1b[2J\r\n")?;
            // Give time for the clear to be received
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            // Now process script lines in a loop until we need user input
            loop {
                // Check if still in script mode
                let is_script_mode = {
                    let clients = broadcaster.clients.lock();
                    clients.get(&addr)
                        .and_then(|c| c.login_session.as_ref())
                        .map(|s| s.state == LoginState::ScriptMode)
                        .unwrap_or(false)
                };

                if !is_script_mode {
                    return Ok(false);
                }

                // Process next script line
                let (msg, should_wait, script_complete, player_name, delay_ms) = {
                    let mut clients = broadcaster.clients.lock();
                    let client = clients.get_mut(&addr);
                    if let Some(client) = client {
                        if let Some(session) = client.login_session_mut() {
                            let (new_mode, new_pos, waiting, output_msg, wait_for_input, is_complete, delay) =
                                process_script_line(session, "");
                            session.script_mode = new_mode;
                            session.script_position = new_pos;
                            session.waiting_for_command = waiting;
                            let name = session.char_name.clone();
                            (output_msg, wait_for_input, is_complete, name, delay)
                        } else {
                            (None, false, false, String::new(), 0)
                        }
                    } else {
                        (None, false, false, String::new(), 0)
                    }
                };

                if let Some(msg) = msg {
                    broadcaster.send_to(addr, &msg)?;
                }

                // Apply tick delay if any (after sending output)
                let delay_to_apply = {
                    let clients = broadcaster.clients.lock();
                    clients.get(&addr)
                        .and_then(|c| c.login_session.as_ref())
                        .map(|s| s.delay_after_output)
                        .unwrap_or(0)
                };
                if delay_to_apply > 0 {
                    eprintln!("[ TICK] Applying {}ms delay after output", delay_to_apply);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_to_apply)).await;
                    // Don't clear delay - it persists for all subsequent lines
                }

                if script_complete {
                    complete_char_creation_and_enter_game(broadcaster, addr, &player_name, command_registry, room_cache).await?;
                    return Ok(false);
                }

                if should_wait {
                    // Waiting for user input, exit loop
                    return Ok(false);
                }

                // Apply tick delay if any
                if delay_ms > 0 {
                    eprintln!("[ TICK] Sleeping for {}ms", delay_ms);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
                // Small delay to let data be sent
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                // Otherwise continue processing script lines
            }
        }
        LoginAction::ScriptContinue => {
            // Get script output to send
            let (msg, should_wait, script_complete, player_name, delay_ms) = {
                let mut clients = broadcaster.clients.lock();
                let client = clients.get_mut(&addr);
                if let Some(client) = client {
                    if let Some(session) = client.login_session_mut() {
                        info!("[ScriptContinue] START: pos={}, mode={}, input={:?}", session.script_position, session.script_mode, input);
                        let (new_mode, new_pos, waiting, output_msg, wait_for_input, is_complete, delay) =
                            process_script_line(session, input);
                        session.script_mode = new_mode;
                        session.script_position = new_pos;
                        session.waiting_for_command = waiting;
                        let name = session.char_name.clone();
                        info!("[ScriptContinue] END: new_pos={}, wait={}, complete={}, msg_len={}, delay={}",
                            new_pos, wait_for_input, is_complete, output_msg.as_ref().map_or(0, |m| m.len()), delay);
                        (output_msg, wait_for_input, is_complete, name, delay)
                    } else {
                        (None, false, false, String::new(), 0)
                    }
                } else {
                    (None, false, false, String::new(), 0)
                }
            };

            if let Some(msg) = msg {
                broadcaster.send_to(addr, &msg)?;
            }

            // Apply tick delay if any (after sending output)
            let delay_to_apply = {
                let clients = broadcaster.clients.lock();
                clients.get(&addr)
                    .and_then(|c| c.login_session.as_ref())
                    .map(|s| s.delay_after_output)
                    .unwrap_or(0)
            };
            if delay_to_apply > 0 {
                eprintln!("[ TICK] Applying {}ms delay after output", delay_to_apply);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_to_apply)).await;
                // Don't clear delay - it persists for all subsequent lines
            }

            if script_complete {
                // Script ended - complete character creation
                complete_char_creation_and_enter_game(broadcaster, addr, &player_name, command_registry, room_cache).await?;
                Ok(false)
            } else if should_wait {
                // Already sent prompt in the message
                info!("[ScriptContinue] should_wait=true, exiting");
                Ok(false)
            } else {
                // Continue processing script - use loop to avoid async recursion
                info!("[ScriptContinue] Continuing script processing, input={:?}", input);
                loop {
                    // Check if still in script mode
                    let is_script_mode = {
                        let clients = broadcaster.clients.lock();
                        clients.get(&addr)
                            .and_then(|c| c.login_session.as_ref())
                            .map(|s| s.state == LoginState::ScriptMode)
                            .unwrap_or(false)
                    };

                    if !is_script_mode {
                        return Ok(false);
                    }

                    // Process next script line
                    let (msg, should_wait, script_complete, player_name, delay_ms) = {
                        let mut clients = broadcaster.clients.lock();
                        let client = clients.get_mut(&addr);
                        if let Some(client) = client {
                            if let Some(session) = client.login_session_mut() {
                                let (new_mode, new_pos, waiting, output_msg, wait_for_input, is_complete, delay) =
                                    process_script_line(session, "");
                                session.script_mode = new_mode;
                                session.script_position = new_pos;
                                session.waiting_for_command = waiting;
                                let name = session.char_name.clone();
                                (output_msg, wait_for_input, is_complete, name, delay)
                            } else {
                                (None, false, false, String::new(), 0)
                            }
                        } else {
                            (None, false, false, String::new(), 0)
                        }
                    };

                    if let Some(msg) = msg {
                        broadcaster.send_to(addr, &msg)?;
                    }

                    // Apply tick delay if any (after sending output) - inner loop
                    let delay_to_apply = {
                        let clients = broadcaster.clients.lock();
                        clients.get(&addr)
                            .and_then(|c| c.login_session.as_ref())
                            .map(|s| s.delay_after_output)
                            .unwrap_or(0)
                    };
                    if delay_to_apply > 0 {
                        eprintln!("[ TICK] Applying {}ms delay after output (inner loop)", delay_to_apply);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_to_apply)).await;
                        // Don't clear delay - it persists for all subsequent lines
                    }

                    if script_complete {
                        complete_char_creation_and_enter_game(broadcaster, addr, &player_name, command_registry, room_cache).await?;
                        return Ok(false);
                    }

                    if should_wait {
                        // Waiting for user input, exit loop
                        return Ok(false);
                    }
                    // Otherwise continue processing script lines
                }
            }
        }
        LoginAction::ScriptComplete(player_name) => {
            complete_char_creation_and_enter_game(broadcaster, addr, &player_name, command_registry, room_cache).await?;
            Ok(false)
        }
        LoginAction::AskPassword(_player_name) => {
            send_password_prompt(broadcaster, addr).await?;
            Ok(false)
        }
        LoginAction::NameError(_invalid_name) => {
            // Send error message and ask for name again
            broadcaster.send_to(addr, "\r\n한글 입력만 가능합니다.\r\n무림존함ː")?;
            Ok(false)
        }
        LoginAction::AskPasswordRetry => {
            broadcaster.send_to(addr, "\r\n잘못된 암호 입니다.\r\n존함암호ː ")?;
            Ok(false)
        }
        LoginAction::PasswordWrongDisconnect => {
            broadcaster.send_to(addr, "\r\n")?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            return Ok(true);
        }
        LoginAction::AskKickExisting(_) | LoginAction::AskKickExistingRetry(_) => {
            broadcaster.send_to(
                addr,
                "\r\n\x1b[1;37m기존 접속을 종료하고 접속하시겠습니까? (네/아니오)\x1b[0;37m\r\n",
            )?;
            Ok(false)
        }
        LoginAction::KickExistingAndProceed(name) => {
            if let Some(existing_addr) = broadcaster.find_addr_by_player_name(&name) {
                let _ = broadcaster.send_to(
                    existing_addr,
                    "\r\n\x1b[1;33m다른 곳에서 접속하여 접속이 종료됩니다.\x1b[0;37m\r\n",
                );
                let _ = broadcaster.request_disconnect(existing_addr);
                {
                    let mut w = crate::world::get_world_state().write().unwrap();
                    w.remove_player_position(&name);
                }
            }
            send_notice_and_complete(broadcaster, addr).await?;
            Ok(false)
        }
        LoginAction::DisconnectSelf => {
            broadcaster.send_to(
                addr,
                "\r\n\x1b[1;37m접속을 취소합니다.\x1b[0;37m\r\n",
            )?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            return Ok(true);
        }
        LoginAction::ShowNotice => {
            send_notice_and_complete(broadcaster, addr).await?;
            Ok(false)
        }
        LoginAction::EnterGame(_player_name) => {
            complete_login_and_enter_game(broadcaster, addr, command_registry, &room_cache).await?;
            Ok(false)
        }
        LoginAction::GameCommand(_player_name) => {
            handle_game_command(broadcaster, addr, input, command_registry, room_cache, shutdown_notify).await?;
            Ok(false)
        }
    }
}

/// Actions to take after processing login state and releasing the lock
enum LoginAction {
    None,
    AskName,
    AskPassword(String),
    AskPasswordRetry,
    ShowNotice,
    EnterGame(String),
    GameCommand(String),
    /// Name validation error
    NameError(String),
    /// Script-based character creation actions
    StartScript,
    ScriptContinue,
    ScriptComplete(String),
    /// 동일 접속자 있음: "기존 접속 종료? (네/아니오)" 질의
    AskKickExisting(String),
    AskKickExistingRetry(String),
    /// 기존 접속자 kick 후 새 접속 진행
    KickExistingAndProceed(String),
    /// 새 접속자가 "아니오" 선택 → 현재(새) 접속 끊기
    DisconnectSelf,
    /// 암호 3회 오류 → 접속 끊기
    PasswordWrongDisconnect,
}

/// Send helper selection menu
async fn send_helper_selection(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    // Clear screen and show helper selection
    let clear = "\x1b[0;37;40m\x1b[H\x1b[2J";
    broadcaster.send_to(addr, clear)?;

    let header = "\x1b[1;37m┌─────────────────────────────────────────────┐\x1b[0;37m\r\n";
    broadcaster.send_to(addr, header)?;

    let title = "\x1b[1;37m│              케릭터 생성                       │\x1b[0;37m\r\n";
    broadcaster.send_to(addr, title)?;

    let divider = "\x1b[1;37m├─────────────────────────────────────────────┤\x1b[0;37m\r\n";
    broadcaster.send_to(addr, divider)?;

    let line1 = "\x1b[1;37m│  무림에 입문하시는 것을 환영합니다.        │\x1b[0;37m\r\n";
    broadcaster.send_to(addr, line1)?;

    let line2 = "\x1b[1;37m│  새로운 케릭터를 생성하겠습니다.            │\x1b[0;37m\r\n";
    broadcaster.send_to(addr, line2)?;

    let divider2 = "\x1b[1;37m├─────────────────────────────────────────────┤\x1b[0;37m\r\n";
    broadcaster.send_to(addr, divider2)?;

    let option1 = "\x1b[1;37m│  \x1b[1;33m1.\x1b[0;37m 빠른도우미 - 빠른 케릭터 생성         │\x1b[0;37m\r\n";
    broadcaster.send_to(addr, option1)?;

    let option2 = "\x1b[1;37m│  \x1b[1;33m2.\x1b[0;37m 초기도우미 - 스토리 모드            │\x1b[0;37m\r\n";
    broadcaster.send_to(addr, option2)?;

    let footer = "\x1b[1;37m└─────────────────────────────────────────────┘\x1b[0;37m\r\n";
    broadcaster.send_to(addr, footer)?;

    let prompt = "\r\n\x1b[1;37m선택 (1 또는 2):\x1b[0;37m ";
    broadcaster.send_to(addr, prompt)?;

    Ok(())
}

/// Handle script state and return appropriate LoginAction
fn handle_script_state(_session: &mut LoginSession, _input: &str) -> LoginAction {
    // Script mode is already set when entering ScriptMode
    // Just continue processing the script
    LoginAction::ScriptContinue
}

/// Substitute variables in script text
/// Check if a Korean syllable ends with a consonant (받침)
/// Returns true if the last character has 받침
fn has_batchilm(name: &str) -> bool {
    name.chars().last().map_or(false, |c| {
        let code = c as u32;
        // Korean syllables range from AC00 (가) to D7A3 (힣)
        // (code - 0xAC00) % 28 gives the 받침 index (0 = no 받침, 1-27 = 받침)
        (0xAC00..=0xD7A3).contains(&code) && ((code - 0xAC00) % 28) > 0
    })
}

/// Replaces:
/// - [공](이라/라) → "{name}이라" if ends with 받침, "{name}라" if no 받침 (template adds "고" after)
/// - [공](아/야) → "{name}아" if ends with 받침, "{name}야" if no 받침
/// - [공](이/가) → "{name}이" if ends with 받침, "{name}가" if no 받침
/// - [공] → character name
fn substitute_variables(text: &str, char_name: &str, _char_gender: &str) -> String {
    let mut result = text.to_string();

    // Handle [공](이라/라) → "{name}이라" or "{name}라" (template adds "고" after)
    if text.contains("[공](이라/라)") {
        let particle = if has_batchilm(char_name) { "이라" } else { "라" };
        result = result.replace("[공](이라/라)", &format!("{}{}", char_name, particle));
    }

    // Handle [공](아/야) → "{name}아" or "{name}야"
    if result.contains("[공](아/야)") {
        let particle = if has_batchilm(char_name) { "아" } else { "야" };
        result = result.replace("[공](아/야)", &format!("{}{}", char_name, particle));
    }

    // Handle [공](이/가) → "{name}이" or "{name}가"
    if result.contains("[공](이/가)") {
        let particle = if has_batchilm(char_name) { "이" } else { "가" };
        result = result.replace("[공](이/가)", &format!("{}{}", char_name, particle));
    }

    // Then replace [공] with the character name (standalone)
    result = result.replace("[공]", char_name);
    result
}

/// Process a single script line and return the result
/// Returns (new_script_mode, new_position, waiting_for_command, message, should_wait_for_input, script_complete, delay_ms)
fn process_script_line(
    session: &mut LoginSession,
    input: &str,
) -> (u8, usize, Option<String>, Option<String>, bool, bool, u64) {
    // Get the script from doumi.json
    let script = match get_script(session.script_mode) {
        Some(s) => s,
        None => return (0, 0, None, None, false, false, 0),
    };

    // Check if we're waiting for a specific command
    if let Some(ref expected_cmd) = session.waiting_for_command {
        let input_trimmed = input.trim().to_lowercase();
        let cleaned = input_trimmed.replace(" ", "").replace(".", "");
        let expected_cleaned = expected_cmd.to_lowercase().replace(" ", "");
        if cleaned == expected_cleaned {
            // Command matched, clear waiting and continue
            session.waiting_for_command = None;
            return (session.script_mode, session.script_position, None, None, false, false, 0);
        } else {
            // Wrong command - show prompt again
            return (session.script_mode, session.script_position, session.waiting_for_command.clone(),
                    Some(format!("\'{}\'를 입력 하세요\r\n>", expected_cmd)), true, false, 0);
        }
    }

    // Handle data input from user (when we were waiting for name/password/gender)
    if let Some(data_type) = session.waiting_for_data {
        let input_trimmed = input.trim();
        if data_type == "name" {
            // 케릭터 이름: 한글 1~6글자만 허용.
            if input_trimmed.is_empty() || !crate::hangul::is_han(input_trimmed) {
                return (
                    session.script_mode,
                    session.script_position,
                    None,
                    Some("한글 1~6글자만 입력 가능합니다.\r\n케릭터 이름:".to_string()),
                    true,
                    false,
                    0,
                );
            }
            let len = input_trimmed.chars().count();
            if len > 6 {
                return (
                    session.script_mode,
                    session.script_position,
                    None,
                    Some("한글 1~6글자만 입력 가능합니다.\r\n케릭터 이름:".to_string()),
                    true,
                    false,
                    0,
                );
            }
            session.char_name = input_trimmed.to_string();
            session.waiting_for_data = None;
        } else if !input_trimmed.is_empty() {
            match data_type {
                "password" => {
                    session.char_password = input_trimmed.to_string();
                    session.waiting_for_data = None;
                }
                "gender" => {
                    let gender = match input_trimmed.to_lowercase().as_str() {
                        "남" | "남자" | "m" | "male" => "남",
                        "여" | "여자" | "f" | "female" => "여",
                        _ => "남", // default
                    };
                    session.char_gender = gender.to_string();
                    session.waiting_for_data = None;
                }
                _ => {}
            }
        }
        // Continue processing script after storing data
    }

    // Process script lines
    let mut pos = session.script_position;
    while pos < script.len() {
        let line = &script[pos];
        pos += 1;

        // Debug: log script processing
        if line.starts_with('$') || line.contains("케릭터") || line.contains("엔터") {
            info!("[process_script_line] pos={}, line={}", pos, line.chars().take(30).collect::<String>());
        }

        // Process script commands
        if line.starts_with('$') {
            match line.as_str() {
                "$이름획득" => {
                    session.waiting_for_data = Some("name");
                    // Only return the input prompt - dialogue text comes from script
                    return (session.script_mode, pos, None,
                            Some("케릭터 이름:".to_string()), true, false, 0);
                }
                "$암호획득" => {
                    session.waiting_for_data = Some("password");
                    // Only return the input prompt - dialogue text comes from script
                    return (session.script_mode, pos, None,
                            Some("비밀번호:".to_string()), true, false, 0);
                }
                "$성별획득" => {
                    session.waiting_for_data = Some("gender");
                    // Only return the input prompt - dialogue text comes from script
                    return (session.script_mode, pos, None,
                            Some("성별(남/여):".to_string()), true, false, 0);
                }
                // Tick command - store delay to apply after next output
                _ if line.starts_with("$틱:") => {
                    let tick_str = &line[5..]; // "$틱:" = 5 bytes (1 + 3 + 1)
                    if let Ok(tick_value) = tick_str.parse::<u64>() {
                        // Each tick = 100ms, store to apply after next output
                        session.delay_after_output = tick_value * 100;
                        eprintln!("[ TICK] Stored {}ms delay for after next output, pos={}", session.delay_after_output, pos);
                    }
                    // Continue processing without immediate delay
                }
                // Check for command formats first (longer pattern before shorter)
                _ if line.starts_with("$키입력:") => {
                    // Wait for specific command (new format used in doumi.json)
                    // "$키입력:" = 1 + 3 + 3 + 3 + 1 = 11 bytes
                    let expected_cmd = &line[11..];
                    return (session.script_mode, pos, Some(expected_cmd.to_string()),
                            Some(">".to_string()), true, false, 0);
                }
                _ if line.starts_with("$키입:") => {
                    // Wait for specific command (old format)
                    // "$키입:" = 1 + 3 + 3 + 1 = 8 bytes
                    let expected_cmd = &line[8..];
                    return (session.script_mode, pos, Some(expected_cmd.to_string()),
                            Some(format!("『{}』를 입력 하세요\r\n>", expected_cmd)), true, false, 0);
                }
                "$키입" | "$키입력" => {
                    // The prompt text comes from script, just wait for input (no '>' prompt like original)
                    return (session.script_mode, pos, None, None, true, false, 0);
                }
                "$출력시작" => {
                    // Start of formatted output - collect until $출력끝 or $틱:N
                    session.accumulated_delay = 0;
                    let mut output = String::new();
                    while pos < script.len() {
                        let next_line = &script[pos];
                        pos += 1;
                        if next_line == "$출력끝" {
                            // End of output block
                            return (session.script_mode, pos, None, Some(output), false, false, 0);
                        }
                        // Check for $틱:N command - apply delay immediately
                        if next_line.starts_with("$틱:") {
                            let tick_str = &next_line[7..]; // "$틱:" = 7 bytes
                            if let Ok(tick_value) = tick_str.parse::<u64>() {
                                // Each tick = 100ms, apply immediately by returning output + delay
                                let delay_ms = tick_value * 100;
                                // Return current output with delay, will continue from current pos after delay
                                return (session.script_mode, pos, None, Some(output), false, false, delay_ms);
                            }
                        } else {
                            output.push_str(next_line);
                            output.push_str("\r\n");
                        }
                    }
                    return (session.script_mode, pos, None, Some(output), false, false, 0);
                }
                _ => {
                    // Unknown command - skip
                }
            }
        } else {
            // Regular text line - apply variable substitutions and send it
            // Skip screen clear sequences (already done in StartScript)
            if line.contains("[H") || line.contains("[2J") {
                continue;
            }
            // Empty lines create blank lines (send just \r\n)
            if line.is_empty() {
                return (session.script_mode, pos, None, Some("\r\n".to_string()), false, false, 0);
            }
            let text = substitute_variables(&line, &session.char_name, &session.char_gender);
            return (session.script_mode, pos, None, Some(format!("{}\r\n", text)), false, false, 0);
        }
    }

    // Script ended - signal completion
    (session.script_mode, pos, None, None, false, true, 0)
}

/// Complete character creation and enter game
async fn complete_char_creation_and_enter_game(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    player_name: &str,
    _command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get character creation data and complete login
    let (char_name, char_gender) = {
        let mut clients = broadcaster.clients.lock();
        let client = clients.get_mut(&addr);

        if let Some(client) = client {
            let name = if !player_name.is_empty() {
                player_name.to_string()
            } else {
                client.login_session.as_ref()
                    .map(|s| s.char_name.clone())
                    .unwrap_or_else(|| "방문자".to_string())
            };
            let gender = client.login_session.as_ref()
                .map(|s| s.char_gender.clone())
                .unwrap_or_else(|| "남".to_string());
            let pwd = client.login_session.as_ref()
                .map(|s| s.char_password.clone())
                .unwrap_or_default();

            // Complete login
            client.complete_login();

            // Create player and initialize
            let mut player = Player::new();
            player.body.set("이름", name.as_str());
            player.body.set("성별", gender.as_str());
            player.body.init_body();
            player.body.set("암호", password_hash(&pwd));
            player.state = STATE_ACTIVE;
            player.interactive = 1;

            // Set player's starting room
            player.body.set("위치", "시작/시작");

            // Give starting money (은전)
            player.body.set("은전", 10000i64);

            if name == "밍밍" {
                player.body.set("관리자등급", 2000i64);
            }

            let _ = save_body_to_json(&mut player.body, &format!("data/user/{}.json", name));

            // Store the player in the client
            client.set_player(player);

            // Set position in WorldState (same as complete_login_and_enter_game) so 봐/버려/이동 등이 동작
            let start_pos = crate::world::PlayerPosition::start();
            {
                let mut w = crate::world::get_world_state().write().unwrap();
                w.set_player_position(name.as_str(), start_pos.clone());
                w.spawn_mobs_for_room(&start_pos.zone, start_pos.room);
            }

            info!("New character created: {} ({})", name, gender);

            (name, gender)
        } else {
            ("방문자".to_string(), "남".to_string())
        }
    };
    // Lock is dropped here

    // Send creation complete message
    broadcaster.send_to(addr, "\r\n\x1b[1;37m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0;37m\r\n")?;
    broadcaster.send_to(addr, &format!("\x1b[1;37m케릭터가 생성되었습니다.\x1b[0;37m\r\n"))?;
    broadcaster.send_to(addr, &format!("\x1b[1;37m이름: {}\x1b[0;37m\r\n", char_name))?;
    broadcaster.send_to(addr, &format!("\x1b[1;37m성별: {}\x1b[0;37m\r\n", if char_gender == "남" { "남자" } else { "여자" }))?;
    broadcaster.send_to(addr, "\x1b[1;37m은전: 10000\x1b[0;37m\r\n")?;
    broadcaster.send_to(addr, "\x1b[1;37m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0;37m\r\n")?;

    // Send welcome message
    broadcaster.send_to(addr, "\r\n\x1b[1;37m=== 무림에 입장하셨습니다 ===\x1b[0;37m\r\n")?;
    broadcaster.send_to(addr, "도움말을 보려면 \x1b[1m도움말\x1b[0;37m 또는 \x1b[1mhelp\x1b[0;37m을 입력하세요.\r\n")?;
    broadcaster.send_to(addr, "\r\n")?;

    // Show the starting room
    show_room(broadcaster, addr, &room_cache)?;

    send_game_prompt(broadcaster, addr).await?;

    Ok(())
}

/// Send logo and name prompt to client
async fn send_logo_and_name_prompt(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    // Update state to Name first (before async operations)
    {
        let mut clients = broadcaster.clients.lock();
        if let Some(client) = clients.get_mut(&addr) {
            if let Some(session) = client.login_session_mut() {
                session.state = LoginState::Name;
            }
        }
    }
    // Lock is dropped here before the async file read

    // Read and send logo
    match read_text_file("logoMurim.txt") {
        Ok(logo) => {
            broadcaster.send_to(addr, &logo)?;
        }
        Err(e) => {
            warn!("Failed to read logoMurim.txt: {}", e);
            // Send simple greeting if logo not found
            broadcaster.send_to(addr, "\x1b[2J\x1b[H")?;
            broadcaster.send_to(addr, "무림 크래프트 트리 뉴얼에 오신 것을 환영합니다!\r\n")?;
        }
    }

    // Send name prompt (all on one line like original)
    // Add an extra blank line before the prompt to match 9900 output
    broadcaster.send_to(addr, "\r\n")?;
    let prompt = "\x1b[0;37m\x1b[40m무림에서 불리우는 존함을 알려주세요. (처음 오시는 분은 \x1b[1m무명객\x1b[0;40m이라고 하세요)\r\n";
    broadcaster.send_to(addr, prompt)?;

    send_prompt_raw(broadcaster, addr, "무림존함ː").await?;

    Ok(())
}

/// Send password prompt to client (존함암호ː)
async fn send_password_prompt(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let prompt = "\r\n존함암호ː ";
    broadcaster.send_to(addr, prompt)?;

    // Update state to Password
    let mut clients = broadcaster.clients.lock();
    if let Some(client) = clients.get_mut(&addr) {
        if let Some(session) = client.login_session_mut() {
            session.state = LoginState::Password;
        }
    }

    Ok(())
}

/// Send notice and prompt for Enter
async fn send_notice_and_complete(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read and send notice
    match read_text_file("notice.txt") {
        Ok(notice) => {
            broadcaster.send_to(addr, &notice)?;
        }
        Err(e) => {
            warn!("Failed to read notice.txt: {}", e);
            // Send simple notice if file not found
            broadcaster.send_to(addr, "\r\n환영합니다!\r\n")?;
        }
    }

    let prompt = "\r\x1b[0;37m계속하시려면 \x1b[1mEnter\x1b[0;37m키를 누르십시오.";
    broadcaster.send_to(addr, prompt)?;

    // Update state to Notice
    let mut clients = broadcaster.clients.lock();
    if let Some(client) = clients.get_mut(&addr) {
        if let Some(session) = client.login_session_mut() {
            session.state = LoginState::Notice;
        }
    }

    Ok(())
}

/// Complete login and move player to starting room
async fn complete_login_and_enter_game(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    _command_registry: Arc<CommandRegistry>,
    _room_cache: &Arc<std::sync::Mutex<RoomCache>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::world::{get_world_state, PlayerPosition};

    // Get player name and complete login (while holding lock)
    let player_name = {
        let mut clients = broadcaster.clients.lock();
        let client = clients.get_mut(&addr);

        if let Some(client) = client {
            // Get player name from session
            let name = if let Some(session) = &client.login_session {
                session.name.clone()
            } else {
                "방문자".to_string()
            };

            // Complete login
            client.complete_login();

            // Create player and initialize
            let mut player = Player::new();
            player.body.set("이름", name.as_str());
            player.body.init_body();
            let _ = load_body_from_json(&mut player.body, &format!("data/user/{}.json", name));
            if name == "밍밍" {
                player.body.set("관리자등급", 2000i64);
            }
            player.state = STATE_ACTIVE;
            player.interactive = 1;

            // Store the player in the client
            client.set_player(player);

            // Set player's starting position in WorldState and spawn mobs for start room
            let start_pos = PlayerPosition::start(); // 낙양성:1
            {
                let mut w = get_world_state().write().unwrap();
                w.set_player_position(name.as_str(), start_pos.clone());
                w.spawn_mobs_for_room(&start_pos.zone, start_pos.room);
            }

            info!("Player {} logged in from {}", name, addr);

            name
        } else {
            "방문자".to_string()
        }
    };
    // Lock is dropped here

    // Send welcome message (no lock held)
    broadcaster.send_to(addr, "\r\n\x1b[1;37m=== 무림에 입장하셨습니다 ===\x1b[0;37m\r\n")?;
    broadcaster.send_to(addr, "도움말을 보려면 \x1b[1m도움말\x1b[0;37m 또는 \x1b[1mhelp\x1b[0;37m을 입력하세요.\r\n")?;
    broadcaster.send_to(addr, "\r\n")?;

    // Show the starting room
    show_room_to_player(broadcaster, addr, &player_name).await?;

    send_game_prompt(broadcaster, addr).await?;

    Ok(())
}

/// Send a raw prompt without newline
async fn send_prompt_raw(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    broadcaster.send_to(addr, prompt)?;
    Ok(())
}

/// Send game prompt with HP/MP display
async fn send_game_prompt(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let prompt = {
        let clients = broadcaster.clients.lock();
        clients.get(&addr)
            .and_then(|c| c.player())
            .map(|p| {
                let hp = p.body.get_hp();
                let max_hp = p.body.get_max_hp();
                let mp = p.body.get_mp();
                let max_mp = p.body.get_max_mp();
                format!("\x1b[0;37;40m[ {}/{} , {}/{} ] ", hp, max_hp, mp, max_mp)
            })
            .unwrap_or_else(|| ">> ".to_string())
    };
    broadcaster.send_to(addr, &prompt)?;
    Ok(())
}

/// Create visual compass string for room exits (방향만, 숨겨진 제외)
fn format_exit_compass(room: &crate::world::Room) -> String {
    use crate::world::Direction;

    let has = |d: Direction| room.exits.values().any(|e| e.direction == Some(d) && e.has_destination() && !e.hidden);
    let has_north = has(Direction::North);
    let has_south = has(Direction::South);
    let has_east = has(Direction::East);
    let has_west = has(Direction::West);

    let mut directions = Vec::new();
    if has_north { directions.push("북"); }
    if has_south { directions.push("남"); }
    if has_east { directions.push("동"); }
    if has_west { directions.push("서"); }
    if has(Direction::Up) { directions.push("위"); }
    if has(Direction::Down) { directions.push("아래"); }

    if directions.is_empty() {
        return "  ○  어느 쪽으로도 이동할 수 없습니다.\r\n".to_string();
    }

    // Build visual compass (simplified version focusing on 4 directions)
    let mut compass = String::new();

    // Line 1: North
    if has_north {
        compass.push_str("\x1b[32m △\x1b[37m");
    } else {
        compass.push_str("   ");
    }
    compass.push_str("\r\n");

    // Line 2: West, Center, East
    if has_west {
        compass.push_str("\x1b[32m◁\x1b[37m");
    } else {
        compass.push_str(" ");
    }
    compass.push_str("○");
    if has_east {
        compass.push_str("\x1b[32m▷\x1b[37m");
    } else {
        compass.push_str(" ");
    }
    compass.push_str("\r\n");

    // Line 3: South
    if has_south {
        compass.push_str("\x1b[32m ▽\x1b[37m");
    } else {
        compass.push_str("   ");
    }

    // Add direction text
    let dir_str = directions.join("ː");
    compass.push_str(&format!(" 〔{}〕쪽으로 이동할 수 있습니다.\r\n", dir_str));

    compass
}

/// Show the current room to the player
fn show_room(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    _room_cache: &Arc<std::sync::Mutex<RoomCache>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get player name
    let player_name = {
        let clients = broadcaster.clients.lock();
        clients.get(&addr).map(|c| c.player_name()).unwrap_or_else(|| "방문자".to_string())
    };
    // Lock released

    // Use the WorldState-based function to show room
    show_room_to_player_with_world(broadcaster, addr, &player_name)
}

/// Show room to player using WorldState
async fn show_room_to_player(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    player_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::world::get_world_state;

    // Get player position (clone to avoid lock issues)
    let pos = {
        let world = get_world_state().read().unwrap();
        match world.get_player_position(player_name) {
            Some(p) => p.clone(),
            None => {
                // Set default position and spawn mobs for start room
                drop(world);
                let start_pos = crate::world::PlayerPosition::start();
                {
                    let mut w = get_world_state().write().unwrap();
                    w.set_player_position(player_name, start_pos.clone());
                    w.spawn_mobs_for_room(&start_pos.zone, start_pos.room);
                }
                start_pos
            }
        }
    };

    // Get room from cache
    let world = get_world_state().read().unwrap();
    let room_key = format!("{}:{}", pos.zone, pos.room);
    eprintln!("[show_room_to_player] Looking for room key: '{}', zone={}, room={}", room_key, pos.zone, pos.room);
    eprintln!("[show_room_to_player] Room cache size: {}", world.room_cache.len());
    let room_output = if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room.to_string()) {
        let room_ref = room.read()
            .map_err(|e| format!("Room read lock error: {}", e))?;

        // Room name format
        let room_name_formatted = format!(
            "\x1b[1;30m[\x1b[0;37m[[\x1b[1;37m[]\x1b[1m {} \x1b[1;37m[]\x1b[0;37m]]\x1b[1;30m]\x1b[0;37m",
            room_ref.display_name
        );

        // Get exits (방향은 korean_name, 고유명은 display_name. 숨겨진 제외)
        let exits: Vec<String> = room_ref.exits.values()
            .filter(|e| e.has_destination() && !e.hidden)
            .map(|e| e.direction.as_ref().map(|d| d.korean_name().to_string()).unwrap_or_else(|| e.display_name.clone()))
            .collect();
        let exits_str = if exits.is_empty() {
            "출구가 없습니다.".to_string()
        } else {
            format!("◁○   〔{}〕쪽으로 이동할 수 있습니다.", exits.join(" "))
        };

        // Get mobs in room
        let mobs = world.mob_cache.get_mobs_in_room(&pos.zone, pos.room);
        let mob_str = if mobs.is_empty() {
            String::new()
        } else {
            let mut mob_msgs = Vec::new();
            for mob in mobs {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    if !mob_data.desc1.is_empty() {
                        mob_msgs.push(mob_data.desc1.clone());
                    }
                }
            }
            if !mob_msgs.is_empty() {
                format!("\r\n{}", mob_msgs.join("\r\n"))
            } else {
                String::new()
            }
        };

        // 바닥에 떨어진 아이템(room_objs + room_inv_stack). 동일 이름은 N개로 묶어 표시.
        let room_objs = world.get_room_objs(&pos.zone, pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, pos.room);
        let item_str = build_room_objs_grouped(&room_objs, &room_stack);

        // 같은 방의 다른 접속 유저. 봐/show_room_to_player_with_world와 동일.
        let other_descs = get_other_players_desc_in_room(broadcaster.as_ref(), &pos.zone, pos.room, player_name);
        let other_str = if other_descs.is_empty() {
            String::new()
        } else {
            format!("\r\n{}", other_descs.join("\r\n"))
        };

        format!(
            "{}\r\n{}\r\n{}{}{}{}\r\n[ {}/{} , {}/{} ]\r\n",
            room_name_formatted,
            room_ref.description.join("\r\n"),
            exits_str,
            mob_str,
            item_str,
            other_str,
            100, 900, 18, 18  // Default HP/MP display
        )
    } else {
        format!("\x1b[1;37m[{}:{}]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n", pos.zone, pos.room)
    };

    broadcaster.send_to(addr, "\r\n")?;
    broadcaster.send_to(addr, &room_output)?;
    Ok(())
}

/// Handle movement commands
fn handle_movement(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    direction: &str,
    _room_cache: &Arc<std::sync::Mutex<RoomCache>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::world::{get_world_state, Direction};

    // Parse direction
    let dir = match direction {
        "북" => Direction::North,
        "남" => Direction::South,
        "동" => Direction::East,
        "서" => Direction::West,
        "위" => Direction::Up,
        "아래" => Direction::Down,
        "북서" => Direction::NorthWest,
        "북동" => Direction::NorthEast,
        "남서" => Direction::SouthWest,
        "남동" => Direction::SouthEast,
        _ => return Ok(()),
    };

    // Get player name
    let player_name = {
        let clients = broadcaster.clients.lock();
        clients.get(&addr).map(|c| c.player_name()).unwrap_or_else(|| "방문자".to_string())
    };
    // Lock released

    // Try to move player using WorldState
    let (new_zone, new_room) = {
        let mut world = get_world_state().write().unwrap();
        match world.move_player(&player_name, dir) {
            Ok(pos) => {
                // Spawn mobs for the new room
                world.spawn_mobs_for_room(&pos.0, pos.1);
                pos
            }
            Err(e) => {
                // Movement failed - show error and return
                broadcaster.send_to(addr, &format!("\x1b[1;31m☞ {}\x1b[0;37m\r\n", e))?;
                return Ok(());
            }
        }
    };

    // Show the new room
    show_room_to_player_with_world(broadcaster, addr, &player_name)?;

    Ok(())
}

// handle_game_command가 이미 clients 락을 보유한 상태에서 봐 스크립트를 실행할 때
// get_other_players_desc → get_other_players_desc_in_room이 같은 락을 다시 잡으면 데드락이 난다.
// 이 스레드로컬이 Some이면 그 값을 그대로 반환하고, None이면 broadcaster.clients.lock() 후 수집.
thread_local! {
    static PRE_COMPUTED_OTHER_DESCS: RefCell<Option<Vec<String>>> = RefCell::new(None);
}
thread_local! {
    static PRE_COMPUTED_OTHER_MAP: RefCell<Option<HashMap<String, String>>> = RefCell::new(None);
}

/// 락 보유 중에 get_other가 재진입하지 않도록, 미리 수집해 두고 핸들러 호출 전/후로 set/clear.
struct PreComputedOtherDescsGuard;
impl Drop for PreComputedOtherDescsGuard {
    fn drop(&mut self) {
        PRE_COMPUTED_OTHER_DESCS.with(|c| *c.borrow_mut() = None);
        PRE_COMPUTED_OTHER_MAP.with(|c| *c.borrow_mut() = None);
        clear_precomputed_all_online();
    }
}

fn collect_other_players_from_map(
    zone: &str,
    room: i64,
    exclude_name: &str,
    world: &crate::world::WorldState,
    clients_map: &HashMap<SocketAddr, Client>,
) -> (Vec<String>, HashMap<String, String>) {
    let mut out = Vec::new();
    let mut map = HashMap::new();
    for (_, client) in clients_map.iter() {
        if let Some(ref p) = client.player {
            if p.body.object_ref().getInt("투명상태") == 1 {
                continue;
            }
            let name = p.body.get_string("이름");
            if name.is_empty() || name == exclude_name {
                continue;
            }
            if let Some(pos) = world.get_player_position(&name) {
                if pos.zone == zone && pos.room == room {
                    let desc = p.body.get_desc_for_look(false);
                    out.push(desc.clone());
                    map.insert(name, desc);
                }
            }
        }
    }
    (out, map)
}

/// 같은 방(zone, room)에 있는 다른 접속 유저들의 getDesc. 파이썬 viewMapData의 for obj in room.objs: is_player, getDesc.
/// 투명상태==1인 유저는 제외. 파이썬: if obj['투명상태'] == 1: continue
pub(crate) fn get_other_players_desc_in_room(
    broadcaster: &crate::network::Broadcaster,
    zone: &str,
    room: i64,
    exclude_name: &str,
) -> Vec<String> {
    if let Some(taken) = PRE_COMPUTED_OTHER_DESCS.with(|c| c.borrow_mut().take()) {
        return taken;
    }
    let world = crate::world::get_world_state().read().unwrap();
    let clients = broadcaster.clients.lock();
    collect_other_players_from_map(zone, room, exclude_name, &world, &*clients).0
}

/// find_target(봐 [대상])에서 같은 방 다른 유저 매칭용. PRE가 설정돼 있으면 그걸 반환(데드락 회피).
pub(crate) fn get_other_players_map_for_look() -> HashMap<String, String> {
    PRE_COMPUTED_OTHER_MAP.with(|c| c.borrow_mut().take()).unwrap_or_default()
}

/// 감정표현 대상: 같은 방에서 name으로 플레이어 또는 몹 검색. self_name이면 None. 파이썬 env.findObjName.
fn find_emotion_target_in_room(
    zone: &str,
    room: i64,
    name: &str,
    self_name: &str,
    world: &crate::world::WorldState,
    clients: &HashMap<SocketAddr, Client>,
) -> Option<EmotionTarget> {
    if name.is_empty() || name == self_name {
        return None;
    }
    for (_addr, client) in clients.iter() {
        if let Some(ref p) = client.player {
            let n = p.body.get_string("이름");
            if n != name {
                continue;
            }
            if let Some(pos) = world.get_player_position(&n) {
                if pos.zone == zone && pos.room == room {
                    let contact_refuse = p.body.get_string("설정상태").contains("접촉거부 1");
                    return Some(EmotionTarget::Player {
                        name: n,
                        contact_refuse,
                    });
                }
            }
        }
    }
    for mob in world.mob_cache.get_mobs_in_room(zone, room) {
        if !mob.alive {
            continue;
        }
        if mob.name == name {
            return Some(EmotionTarget::Mob {
                name: mob.name.clone(),
            });
        }
    }
    None
}

/// 같은 방의 발언자(및 선택적 to_target)를 제외한 나머지 접속자에게만 msg 전송.
/// exclude: 제외할 플레이어 이름들 (발언자, 감정표현 시 대상 등).
/// clients 락을 잡은 채 broadcaster.send_to()를 호출하면 send_to가 다시 clients.lock()을 시도해 데드락이 나므로,
/// 이미 보유한 client 참조의 sender로 직접 전송.
pub(crate) fn send_to_others_in_room(
    broadcaster: &crate::network::Broadcaster,
    zone: &str,
    room: i64,
    exclude: &[&str],
    msg: &str,
) {
    let world = get_world_state().read().unwrap();
    let clients = broadcaster.clients.lock();
    let line = format!("\r\n{}\r\n", msg);
    for (_addr, client) in clients.iter() {
        if let Some(ref p) = client.player {
            if p.body.object_ref().getInt("투명상태") == 1 {
                continue;
            }
            let name = p.body.get_string("이름");
            if name.is_empty() || exclude.iter().any(|x| *x == name) {
                continue;
            }
            if let Some(pos) = world.get_player_position(&name) {
                if pos.zone == zone && pos.room == room {
                    let _ = client.sender.send(line.clone());
                }
            }
        }
    }
}

/// 외쳐(shout): 게임 접속 전체에 전송. Active이고 외침거부가 아닌 클라이언트에만.
/// clients 락을 잡은 채 send_to를 호출하면 데드락이 나므로, client.sender로 직접 전송.
pub(crate) fn broadcast_shout(broadcaster: &crate::network::Broadcaster, msg: &str) {
    use crate::network::ClientState;
    let clients = broadcaster.clients.lock();
    let line = format!("\r\n{}\r\n", msg);
    for (_addr, client) in clients.iter() {
        if client.state != ClientState::Active {
            continue;
        }
        if let Some(ref p) = client.player {
            let config = p.body.get_string("설정상태");
            if config.contains("외침거부 1") {
                continue;
            }
        } else {
            continue;
        }
        let _ = client.sender.send(line.clone());
    }
}

/// Show room to player using WorldState (synchronous version for use after movement)
fn show_room_to_player_with_world(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    player_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::world::{format_exits_long, format_room_header, get_world_state, PlayerPosition};

    // Get player position; if None (e.g. script char-creation path), set start and spawn mobs
    let pos = {
        let world = get_world_state().read().unwrap();
        match world.get_player_position(player_name) {
            Some(p) => p.clone(),
            None => {
                drop(world);
                let start_pos = PlayerPosition::start();
                {
                    let mut w = get_world_state().write().unwrap();
                    w.set_player_position(player_name, start_pos.clone());
                    w.spawn_mobs_for_room(&start_pos.zone, start_pos.room);
                }
                start_pos
            }
        }
    };

    // 방이 캐시에 없을 수 있으므로 get_room으로 로드 보장 (이동 후 복귀 시 등)
    {
        let mut w = get_world_state().write().unwrap();
        let _ = w.room_cache.get_room(&pos.zone, &pos.room.to_string());
    }

    let world = get_world_state().read().unwrap();

    if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room.to_string()) {
        let room_ref = room.read().map_err(|e| format!("Room read lock error: {}", e))?;

        let room_name_formatted = format_room_header(&room_ref.display_name);
        let exits_str = format_exits_long(&*room_ref);

        let mobs = world.mob_cache.get_mobs_in_room(&pos.zone, pos.room);
        let mob_str = if mobs.is_empty() {
            String::new()
        } else {
            let mut mob_msgs = Vec::new();
            for mob in mobs {
                if let Some(mob_data) = world.mob_cache.get_mob(&mob.mob_key) {
                    if !mob_data.desc1.is_empty() {
                        mob_msgs.push(mob_data.desc1.clone());
                    }
                }
            }
            if !mob_msgs.is_empty() {
                format!("\r\n{}", mob_msgs.join("\r\n"))
            } else {
                String::new()
            }
        };

        // 바닥에 떨어진 아이템(room_objs + room_inv_stack). 동일 이름은 N개로 묶어 표시.
        let room_objs = world.get_room_objs(&pos.zone, pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, pos.room);
        let item_str = build_room_objs_grouped(&room_objs, &room_stack);

        // 같은 방의 다른 접속 유저. 파이썬 viewMapData: for obj in room.objs: is_player then getDesc.
        let other_descs = get_other_players_desc_in_room(broadcaster.as_ref(), &pos.zone, pos.room, player_name);
        let other_str = if other_descs.is_empty() {
            String::new()
        } else {
            format!("\r\n{}", other_descs.join("\r\n"))
        };

        // 파이썬 viewMapData: 헤더 \r\n\r\n 설명 \r\n\r\n 출구 \r\n [몹] \r\n [바닥 아이템] \r\n [다른 유저]
        broadcaster.send_to(addr, "\r\n")?;
        broadcaster.send_to(addr, &room_name_formatted)?;
        broadcaster.send_to(addr, "\r\n\r\n")?;
        broadcaster.send_to(addr, &room_ref.description.join("\r\n"))?;
        broadcaster.send_to(addr, "\r\n\r\n")?;
        broadcaster.send_to(addr, &exits_str)?;
        broadcaster.send_to(addr, "\r\n")?;
        if !mob_str.is_empty() {
            broadcaster.send_to(addr, &mob_str)?;
            broadcaster.send_to(addr, "\r\n")?;
        }
        if !item_str.is_empty() {
            broadcaster.send_to(addr, &item_str)?;
            broadcaster.send_to(addr, "\r\n")?;
        }
        if !other_str.is_empty() {
            broadcaster.send_to(addr, &other_str)?;
            broadcaster.send_to(addr, "\r\n")?;
        }
    } else {
        broadcaster.send_to(addr, &format!("\x1b[1;37m[{}:{}]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n", pos.zone, pos.room))?;
    }

    Ok(())
}

/// Handle game command after login
async fn handle_game_command(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    command: &str,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
    shutdown_notify: Option<Arc<tokio::sync::Notify>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if command.is_empty() {
        send_prompt_raw(broadcaster, addr, ">> ").await?;
        return Ok(());
    }

    debug!("Game command from {}: {}", addr, command);

    // Parse the command
    let parsed = CommandParser::parse(command);

    // Handle empty input
    if parsed.is_empty() {
        send_game_prompt(broadcaster, addr).await?;
        return Ok(());
    }

    // Get the player
    let player_name = {
        let clients = broadcaster.clients.lock();
        clients.get(&addr).map(|c| c.player_name()).unwrap_or_else(|| "방문자".to_string())
    };
    // Lock released

    // 봐/보/look: 봐.rhai 스크립트로 처리 (registry 통해 호출).

    // Handle movement commands. n/e/s/w 등 alias 해석 후에도 방향이면 handle_movement 사용.
    // handle_movement → show_room_to_player_with_world 는 다른 유저(get_other_players_desc_in_room) 포함.
    // registry의 move_command → display_room 은 다른 유저 미포함이므로, 모든 이동을 여기서 처리.
    let move_cmd = command_registry.resolve_alias(parsed.command.as_str());
    if matches!(move_cmd.as_str(), "북" | "남" | "동" | "서" | "위" | "아래" | "북서" | "북동" | "남서" | "남동") {
        handle_movement(broadcaster, addr, &move_cmd, &room_cache)?;
        send_game_prompt(broadcaster, addr).await?;
        return Ok(());
    }

    // Handle "help" command
    if parsed.command == "help" || parsed.command == "도움말" {
        broadcaster.send_to(addr, "\x1b[1;37m=== 도움말 ===\x1b[0;37m\r\n")?;
        broadcaster.send_to(addr, "이동: 북(ㅂ) 남(ㄴ) 동(ㄷ) 서(ㅅ) 위(ㅇ) 아래(ㅁ) 북서(nw) 북동(ne) 남서(sw) 남동(se)\r\n")?;
        broadcaster.send_to(addr, "보기: look, 봐, 보\r\n")?;
        broadcaster.send_to(addr, "종료: quit, 끝, 종료\r\n")?;
        send_game_prompt(broadcaster, addr).await?;
        return Ok(());
    }

    // Handle quit/끝/종료 (접속 종료). 보통 앞단에서 처리되나, 이중 확인.
    if parsed.command.to_lowercase() == "quit" || parsed.command == "끝" || parsed.command == "종료" {
        broadcaster.send_to(addr, "Goodbye!\r\n")?;
        return Ok(());
    }

    // Unknown command - try command registry
    let mut response = String::new();
    let mut say_to_room: Option<(String, String, i64, String)> = None;
    let mut emotion_to_room: Option<(String, String, i64, String, Option<(String, String)>)> = None; // (pname, zone, room, to_room, to_target)
    let mut give_pending: Option<(
        std::net::SocketAddr,
        String,
        i64,
        String,
        String,
        Option<i64>,
        Option<i64>,
        Option<(String, usize, usize)>,
        Option<(String, i64)>,
    )> = None; // (giver_addr, zone, room, target_name, giver_name, give_silver, give_gold, give_item, give_item_stack)
    let mut shout_to_broadcast: Option<String> = None;
    let mut tell_pending: Option<(String, String, String)> = None; // (target, message, sender_name)
    {
        let mut clients = broadcaster.clients.lock();
        let world = get_world_state().read().unwrap();
        // 봐/보 등 스크립트의 view_map_data → get_other_players_desc_in_room이 clients 락을 다시 잡으면 데드락.
        let player_name = clients
            .get(&addr)
            .and_then(|c| c.player.as_ref())
            .map(|p| p.body.get_string("이름"))
            .unwrap_or_default();
        let (zone, room) = world
            .get_player_position(&player_name)
            .map(|p| (p.zone.clone(), p.room))
            .unwrap_or((String::new(), 0));
        let (other_descs, other_map) =
            collect_other_players_from_map(&zone, room, &player_name, &*world, &*clients);
        PRE_COMPUTED_OTHER_DESCS.with(|c| *c.borrow_mut() = Some(other_descs));
        PRE_COMPUTED_OTHER_MAP.with(|c| *c.borrow_mut() = Some(other_map));
        // 전 접속자 목록: 누구 스크립트용 get_all_online_players()
        let mut all_online = Array::new();
        for (_addr, client) in clients.iter() {
            if client.state != ClientState::Active {
                continue;
            }
            let p = match &client.player {
                Some(x) => x,
                None => continue,
            };
            if p.body.get_int("투명상태") == 1 {
                continue;
            }
            let name = p.body.get_string("이름");
            if name.is_empty() {
                continue;
            }
            let mut m = Map::new();
            m.insert("이름".into(), Dynamic::from(name.clone()));
            m.insert("무림별호".into(), Dynamic::from(p.body.get_string("무림별호")));
            m.insert("성격".into(), Dynamic::from(p.body.get_string("성격")));
            m.insert("레벨초기화".into(), Dynamic::from(p.body.get_string("레벨초기화")));
            m.insert("소속".into(), Dynamic::from(p.body.get_string("소속")));
            if let Some(pos) = world.get_player_position(&name) {
                m.insert("zone".into(), Dynamic::from(pos.zone.clone()));
                m.insert("room".into(), Dynamic::from(pos.room));
            } else {
                m.insert("zone".into(), Dynamic::from(""));
                m.insert("room".into(), Dynamic::from(0i64));
            }
            all_online.push(Dynamic::from(m));
        }
        set_precomputed_all_online(all_online);
        let _tl = PreComputedOtherDescsGuard;

        let first_word = command.split_whitespace().next().unwrap_or("");
        let is_emotion = emotion::is_emotion_command(first_word);
        let (emotion_param, emotion_target) = if is_emotion {
            let ep = command
                .strip_prefix(first_word)
                .map(|s| s.trim())
                .unwrap_or("");
            let fip = ep.split_whitespace().next().unwrap_or("");
            let t = find_emotion_target_in_room(
                &zone,
                room,
                fip,
                &player_name,
                &world,
                &*clients,
            );
            (ep.to_string(), t)
        } else {
            (String::new(), None)
        };

        // 데드락 방지: world.read()를 잡은 채로 (cmd.handler)를 호출하면,
        // 귀환/이동 등이 get_world_state().write()를 시도해 블로킹된다. 핸들러 호출 전에 해제.
        drop(world);

        if let Some(client) = clients.get_mut(&addr) {
            if let Some(player) = client.player_mut() {
                let result = if is_emotion {
                    Some(emotion::do_emotion(
                        &player.body,
                        first_word,
                        &emotion_param,
                        emotion_target,
                    ))
                } else {
                    let args: Vec<&str> = parsed.tokens.iter().map(|s| s.as_str()).collect();
                    let cmd_lookup = command_registry.get(parsed.command.as_str());
                    let result = if let Some(cmd) = cmd_lookup {
                        info!("[CMD] Executing command: {} (handler: {})", parsed.command, cmd.name);
                        Some((cmd.handler)(&mut player.body, &args))
                    } else {
                        info!("[CMD] Command not found, trying as exit name: {}", parsed.command);
                        try_move_by_exit_name(&mut player.body, &parsed.command)
                    };
                    result
                };

                response = match result {
                    Some(CommandResult::Output(msg)) => {
                        format!("{}\r\n", msg)
                    }
                    Some(CommandResult::Error(msg)) => {
                        format!("\x1b[1;31m{}\x1b[0;37m\r\n", msg)
                    }
                    Some(CommandResult::Usage(msg)) => {
                        format!("\x1b[1;33m{}\x1b[0;37m\r\n", msg)
                    }
                    Some(CommandResult::Move(_direction)) => {
                        String::new()  // Movement is handled elsewhere
                    }
                    Some(CommandResult::Combat) => {
                        String::new()  // Combat is handled elsewhere
                    }
                    Some(CommandResult::Ok) => {
                        String::new()
                    }
                    Some(CommandResult::NoPrompt) => {
                        String::new()
                    }
                    Some(CommandResult::SayToRoom(to_self, to_room)) => {
                        say_to_room = Some((player_name.clone(), zone.clone(), room, to_room));
                        format!("{}\r\n", to_self)
                    }
                    Some(CommandResult::Shout(msg)) => {
                        shout_to_broadcast = Some(msg);
                        String::new()
                    }
                    Some(CommandResult::Tell(target_name, message)) => {
                        tell_pending = Some((target_name, message, player_name.clone()));
                        String::new()
                    }
                    Some(CommandResult::Shutdown) => {
                        if let Some(ref n) = shutdown_notify {
                            n.notify_waiters();
                        }
                        "☞ 서버를 종료합니다.\r\n".to_string()
                    }
                    Some(CommandResult::EmotionToRoom(to_self, to_room, to_target)) => {
                        emotion_to_room =
                            Some((player_name.clone(), zone.clone(), room, to_room, to_target));
                        format!("{}\r\n", to_self)
                    }
                    Some(CommandResult::GiveToPlayer {
                        target_name,
                        giver_name,
                        give_silver,
                        give_gold,
                        give_item,
                        give_item_stack,
                    }) => {
                        give_pending = Some((
                            addr,
                            zone.clone(),
                            room,
                            target_name,
                            giver_name,
                            give_silver,
                            give_gold,
                            give_item,
                            give_item_stack,
                        ));
                        String::new()
                    }
                    None => {
                        "\x1b[1;31m☞ 무슨 말인지 모르겠어요. *^_^*\x1b[0;37m\r\n".to_string()
                    }
                };
            }
        }
    }

    if let Some((pname, z, r, msg)) = say_to_room {
        send_to_others_in_room(broadcaster, &z, r, &[pname.as_str()], &msg);
    }
    if let Some((pname, z, r, to_room, to_target)) = emotion_to_room {
        let exclude: Vec<&str> = if let Some((ref tname, _)) = &to_target {
            vec![pname.as_str(), tname.as_str()]
        } else {
            vec![pname.as_str()]
        };
        send_to_others_in_room(broadcaster, &z, r, &exclude, &to_room);
        if let Some((tname, tmsg)) = to_target {
            let line = format!("\r\n{}\r\n", tmsg);
            let clients = broadcaster.clients.lock();
            for (_a, c) in clients.iter() {
                if let Some(ref p) = c.player {
                    if p.body.get_string("이름") == tname {
                        let _ = c.sender.send(line);
                        break;
                    }
                }
            }
        }
    }
    if let Some((
        giver_addr,
        z,
        r,
        target_name,
        giver_name,
        give_silver,
        give_gold,
        give_item,
        give_item_stack,
    )) = give_pending.take()
    {
        use std::sync::Mutex;
        let world = get_world_state().read().unwrap();
        let mut target_addr: Option<SocketAddr> = None;
        {
            let clients = broadcaster.clients.lock();
            for (&a, c) in clients.iter() {
                if let Some(ref p) = c.player {
                    if p.body.get_string("이름") == target_name {
                        if let Some(pos) = world.get_player_position(&target_name) {
                            if pos.zone == z && pos.room == r {
                                target_addr = Some(a);
                                break;
                            }
                        }
                    }
                }
            }
        }
        if let Some(taddr) = target_addr {
            let mut to_move: Vec<Arc<Mutex<crate::object::Object>>> = Vec::new();
            let mut give_item_error: Option<(String, Option<String>)> = None;
            {
                let mut clients = broadcaster.clients.lock();
                if let Some(giver) = clients.get_mut(&giver_addr).and_then(|c| c.player_mut()) {
                    if let Some(amt) = give_silver {
                        let have = giver.body.get_int("은전");
                        giver.body.set("은전", (have - amt).max(0));
                    } else if let Some(amt) = give_gold {
                        let have = giver.body.get_int("금전");
                        giver.body.set("금전", (have - amt).max(0));
                    }
                    // give_item: 아래 별도 블록에서 giver+target 동시에 처리 (출력안함/줄수없음/무게/수량한계 검사)
                }
            }
            // 아이템 건네기: 대상의 무게/수량한계를 검사하려면 giver와 target을 동시에 보유해야 함. remove 후 로직 수행, insert로 복귀.
            if give_item.is_some() {
                if let Some((ref name, order, count)) = give_item {
                    const MAX_ITEMS: usize = 300;
                    let mut clients = broadcaster.clients.lock();
                    let giver_opt = clients.remove(&giver_addr);
                    let target_opt = clients.remove(&taddr);
                    if giver_opt.is_none() {
                        if let Some(t) = target_opt {
                            clients.insert(taddr, t);
                        }
                        response = "☞ 오류가 발생했어요.\r\n".to_string();
                        give_item_error = Some(("".to_string(), None));
                    } else if target_opt.is_none() {
                        clients.insert(giver_addr, giver_opt.unwrap());
                        give_item_error = Some(("☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^".to_string(), None));
                    } else {
                        let mut giver = giver_opt.unwrap();
                        let mut target = target_opt.unwrap();
                        match (giver.player.as_mut(), target.player.as_mut()) {
                            (Some(gp), Some(tp)) => {
                                let giver_body = &mut gp.body;
                                let target_body = &mut tp.body;
                                let mut n = 0usize;
                                let mut running_weight: i64 = 0;
                                for obj in &giver_body.object.objs {
                                    if to_move.len() >= count {
                                        break;
                                    }
                                    let o = match obj.lock() {
                                        Ok(x) => x,
                                        Err(_) => continue,
                                    };
                                    let rn = o.getString("반응이름");
                                    let ok = o.getName() == name.as_str()
                                        || (!rn.is_empty() && rn.contains(name.as_str()));
                                    if !ok || o.getBool("inUse") {
                                        continue;
                                    }
                                    if o.checkAttr("아이템속성", "출력안함") {
                                        continue;
                                    }
                                    n += 1;
                                    if n < order {
                                        continue;
                                    }
                                    if o.checkAttr("아이템속성", "줄수없음") {
                                        if to_move.is_empty() {
                                            give_item_error = Some((
                                                "☞ 그 물건은 줄 수 없어요. ^^".to_string(),
                                                None,
                                            ));
                                            break;
                                        }
                                        continue; // 이번 건만 스킵, 다음 후보 계속
                                    }
                                    let w = o.getInt("무게");
                                    if target_body.get_item_weight() + running_weight + w > target_body.get_str() * 10 {
                                        if to_move.is_empty() {
                                            let iga = crate::hangul::han_iga(&target_name);
                                            let go = o.han_obj();
                                            give_item_error = Some((
                                                format!("\x1b[1m{}\x1b[0;37m{} 무거워서 받지 못합니다.", target_name, iga),
                                                Some(format!("\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 무거워서 받지 못합니다.", giver_name, crate::hangul::han_iga(&giver_name), go)),
                                            ));
                                        }
                                        break;
                                    }
                                    if target_body.get_item_count() + to_move.len() + 1 > MAX_ITEMS {
                                        if to_move.is_empty() {
                                            let iga = crate::hangul::han_iga(&target_name);
                                            let go = o.han_obj();
                                            give_item_error = Some((
                                                format!("\x1b[1m{}\x1b[0;37m{} 수량 한계로 받지 못합니다.", target_name, iga),
                                                Some(format!("\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 수량 한계로 받지 못합니다.", giver_name, crate::hangul::han_iga(&giver_name), go)),
                                            ));
                                        }
                                        break;
                                    }
                                    running_weight += w;
                                    to_move.push(obj.clone());
                                }
                                if give_item_error.is_none() {
                                    for arc in &to_move {
                                        giver_body.object.remove(arc);
                                        target_body.object.append(arc.clone());
                                    }
                                }
                                clients.insert(giver_addr, giver);
                                clients.insert(taddr, target);
                            }
                            _ => {
                                clients.insert(giver_addr, giver);
                                clients.insert(taddr, target);
                                give_item_error = Some(("☞ 오류가 발생했어요.".to_string(), None));
                            }
                        }
                    }
                }
            } else if let Some((ref key, cnt)) = give_item_stack {
                const MAX_ITEMS: usize = 300;
                let cnt_u = cnt as usize;
                let w = get_item_weight_by_key(key);
                let mut clients = broadcaster.clients.lock();
                let giver_opt = clients.remove(&giver_addr);
                let target_opt = clients.remove(&taddr);
                if giver_opt.is_none() {
                    if let Some(t) = target_opt {
                        clients.insert(taddr, t);
                    }
                    response = "☞ 오류가 발생했어요.\r\n".to_string();
                    give_item_error = Some(("".to_string(), None));
                } else if target_opt.is_none() {
                    clients.insert(giver_addr, giver_opt.unwrap());
                    give_item_error = Some(("☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^".to_string(), None));
                } else {
                    let mut giver = giver_opt.unwrap();
                    let mut target = target_opt.unwrap();
                    match (giver.player.as_mut(), target.player.as_mut()) {
                        (Some(gp), Some(tp)) => {
                            let giver_body = &mut gp.body;
                            let target_body = &mut tp.body;
                            let have = *giver_body.object.inv_stack.get(key).unwrap_or(&0);
                            if have < cnt {
                                give_item_error = Some(("☞ 그런 아이템이 소지품에 없어요.".to_string(), None));
                            } else if target_body.get_item_weight() + w * cnt > target_body.get_str() * 10 {
                                let iga = crate::hangul::han_iga(&target_name);
                                let disp = get_item_display_name(key);
                                let go = format!("\x1b[33m{}\x1b[37m{}", disp, crate::hangul::han_obj(&disp));
                                give_item_error = Some((
                                    format!("\x1b[1m{}\x1b[0;37m{} 무거워서 받지 못합니다.", target_name, iga),
                                    Some(format!("\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 무거워서 받지 못합니다.", giver_name, iga, go)),
                                ));
                            } else if target_body.get_item_count() + cnt_u > MAX_ITEMS {
                                let iga = crate::hangul::han_iga(&target_name);
                                let disp = get_item_display_name(key);
                                let go = format!("\x1b[33m{}\x1b[37m{}", disp, crate::hangul::han_obj(&disp));
                                give_item_error = Some((
                                    format!("\x1b[1m{}\x1b[0;37m{} 수량 한계로 받지 못합니다.", target_name, iga),
                                    Some(format!("\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 수량 한계로 받지 못합니다.", giver_name, iga, go)),
                                ));
                            } else {
                                let should_remove = {
                                    let r = giver_body.object.inv_stack.get_mut(key.as_str()).unwrap();
                                    *r -= cnt;
                                    *r <= 0
                                };
                                if should_remove {
                                    giver_body.object.inv_stack.remove(key.as_str());
                                }
                                *target_body.object.inv_stack.entry(key.clone()).or_insert(0) += cnt;
                            }
                            clients.insert(giver_addr, giver);
                            clients.insert(taddr, target);
                        }
                        _ => {
                            clients.insert(giver_addr, giver);
                            clients.insert(taddr, target);
                            give_item_error = Some(("☞ 오류가 발생했어요.".to_string(), None));
                        }
                    }
                }
            }
            if let Some((gmsg, tmsg)) = give_item_error {
                if !gmsg.is_empty() {
                    response = format!("{}\r\n", gmsg);
                }
                if let Some(tm) = tmsg {
                    let clients = broadcaster.clients.lock();
                    for (_a, c) in clients.iter() {
                        if *_a == taddr {
                            let _ = c.sender.send(format!("\r\n{}\r\n", tm));
                            break;
                        }
                    }
                }
            } else {
            let (c, post, name_multi) = if give_item.is_some() {
                let c = to_move.len();
                if c == 0 {
                    (0, String::new(), String::new())
                } else if c == 1 {
                    let o = to_move[0].lock().unwrap();
                    (c, o.han_obj(), o.getName())
                } else {
                    let o = to_move[0].lock().unwrap();
                    let n = o.getName();
                    (c, n.clone(), n)
                }
            } else if let Some((ref key, cnt)) = give_item_stack {
                let c = cnt as usize;
                let name_multi = get_item_display_name(key);
                let post = format!("\x1b[33m{}\x1b[37m{}", name_multi, crate::hangul::han_obj(&name_multi));
                (c, post, name_multi)
            } else {
                (0, String::new(), String::new())
            };
            {
                let mut clients = broadcaster.clients.lock();
                if let Some(target) = clients.get_mut(&taddr).and_then(|c| c.player_mut()) {
                    if let Some(amt) = give_silver {
                        let have = target.body.get_int("은전");
                        target.body.set("은전", have + amt);
                    } else if let Some(amt) = give_gold {
                        let have = target.body.get_int("금전");
                        target.body.set("금전", have + amt);
                    }
                    // give_item: 위 아이템 블록에서 이미 giver→target 이전함
                }
            }
            let iga = crate::hangul::han_iga(&giver_name);
            let (to_self, to_target, to_room) = if let Some(amt) = give_silver {
                (
                    format!("당신이 {}에게 은전 {}개를 줍니다.", target_name, amt),
                    format!(
                        "\r\n{}{} 당신에게 은전 {}개를 줍니다.",
                        giver_name, iga, amt
                    ),
                    format!(
                        "{}{} {}에게 은전 {}개를 줍니다.",
                        giver_name, iga, target_name, amt
                    ),
                )
            } else if let Some(amt) = give_gold {
                (
                    format!("당신이 {}에게 금전 {}개를 줍니다.", target_name, amt),
                    format!(
                        "\r\n{}{} 당신에게 금전 {}개를 줍니다.",
                        giver_name, iga, amt
                    ),
                    format!(
                        "{}{} {}에게 금전 {}개를 줍니다.",
                        giver_name, iga, target_name, amt
                    ),
                )
            } else {
                if c == 0 {
                    response = "☞ 그런 아이템이 소지품에 없어요.\r\n".to_string();
                    (String::new(), String::new(), String::new())
                } else {
                    (
                        if c == 1 {
                            format!(
                                "당신이 \x1b[1m{}\x1b[0;37m에게 \x1b[36m{}\x1b[37m 줍니다.",
                                target_name, post
                            )
                        } else {
                            format!(
                                "당신이 \x1b[1m{}\x1b[0;37m에게 \x1b[36m{}\x1b[37m {}개를 줍니다.",
                                target_name, name_multi, c
                            )
                        },
                        if c == 1 {
                            format!(
                                "\r\n\x1b[1m{}\x1b[0;37m{} 당신에게 \x1b[36m{}\x1b[37m 줍니다.",
                                giver_name, iga, post
                            )
                        } else {
                            format!(
                                "\r\n\x1b[1m{}\x1b[0;37m{} 당신에게 \x1b[36m{}\x1b[37m {}개를 줍니다.",
                                giver_name, iga, name_multi, c
                            )
                        },
                        if c == 1 {
                            format!(
                                "{}{} {}에게 \x1b[36m{}\x1b[37m 줍니다.",
                                giver_name, iga, target_name, post
                            )
                        } else {
                            format!(
                                "{}{} {}에게 \x1b[36m{}\x1b[37m {}개를 줍니다.",
                                giver_name, iga, target_name, name_multi, c
                            )
                        },
                    )
                }
            };
            if !to_self.is_empty() {
                response = format!("{}\r\n", to_self);
                let clients = broadcaster.clients.lock();
                for (_a, c) in clients.iter() {
                    if *_a == taddr {
                        let _ = c.sender.send(format!("\r\n{}\r\n", to_target));
                        break;
                    }
                }
                send_to_others_in_room(
                    broadcaster,
                    &z,
                    r,
                    &[giver_name.as_str(), target_name.as_str()],
                    &to_room,
                );
            }
            {
                let mut clients = broadcaster.clients.lock();
                if let Some(p) = clients.get_mut(&giver_addr).and_then(|c| c.player_mut()) {
                    let path = format!("data/user/{}.json", p.body.get_name());
                    let _ = save_body_to_json(&mut p.body, &path);
                }
                if let Some(p) = clients.get_mut(&taddr).and_then(|c| c.player_mut()) {
                    let path = format!("data/user/{}.json", p.body.get_name());
                    let _ = save_body_to_json(&mut p.body, &path);
                }
            }
            } // else of give_item_error (c, post, name_multi, to_self/to_target/to_room)
        } else {
            response =
                "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^\r\n".to_string();
        }
    }
    if let Some(ref msg) = shout_to_broadcast {
        broadcast_shout(broadcaster, msg);
    }
    if let Some((target_name, message, sender_name)) = tell_pending.take() {
        use crate::network::ClientState;
        let clients = broadcaster.clients.lock();
        let mut tell_response = String::new();
        let mut tell_target: Option<(std::net::SocketAddr, String)> = None;
        for (&a, c) in clients.iter() {
            if c.state != ClientState::Active {
                continue;
            }
            if let Some(ref p) = c.player {
                if p.body.get_string("이름") != target_name {
                    continue;
                }
                if p.body.get_int("투명상태") == 1 {
                    continue;
                }
                if p.body.get_string("설정상태").contains("전음거부 1") {
                    tell_response = format!("\x1b[1;31m☞ 전음 거부중이에요. ^^\x1b[0;37m\r\n");
                    break;
                }
                let msg_to_sender = format!("[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] {}에게 보냄 : {}", target_name, message);
                let msg_to_target = format!("\r\n[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] {} : {}\r\n", sender_name, message);
                tell_response = format!("{}\r\n", msg_to_sender);
                tell_target = Some((a, msg_to_target));
                break;
            }
        }
        if tell_response.is_empty() {
            tell_response = "☞ 전음이 전달될만한 상대가 없어요. ^^\r\n".to_string();
        }
        response = tell_response;
        drop(clients);
        if let Some((a, m)) = tell_target {
            let _ = broadcaster.send_to(a, &m);
        }
    }

    broadcaster.send_to(addr, &response)?;
    send_game_prompt(broadcaster, addr).await?;

    Ok(())
}


/// Script data loaded from doumi.json
#[derive(Debug, Deserialize)]
struct DoumiJson {
    #[serde(rename = "도우미메인설정")]
    도우미메인설정: DoumiSettings,
}

#[derive(Debug, Deserialize)]
struct DoumiSettings {
    #[serde(rename = "빠른도우미")]
    빠른도우미: Vec<String>,
    #[serde(rename = "초기도우미")]
    초기도우미: Vec<String>,
}

/// Loaded script data from doumi.json. RwLock로 감싸 관리자 명령 '업데이트 도우미'에서 재로딩 가능.
static SCRIPT_DATA: Lazy<RwLock<Option<DoumiScriptData>>> = Lazy::new(|| {
    RwLock::new(load_doumi_json())
});

/// Holds the script vectors
#[derive(Debug, Clone)]
struct DoumiScriptData {
    quick_helper: Vec<String>,
    initial_helper: Vec<String>,
}

/// Load doumi.json file
fn load_doumi_json() -> Option<DoumiScriptData> {
    let path = PathBuf::from("data/config/doumi.json");
    info!("Loading doumi.json from {:?}", path);
    
    let content = std::fs::read(&path).ok()?;
    let json: DoumiJson = serde_json::from_slice(&content).ok()?;
    
    info!("Successfully loaded doumi.json: {} lines in 빠른도우미, {} lines in 초기도우미",
        json.도우미메인설정.빠른도우미.len(),
        json.도우미메인설정.초기도우미.len());
    
    Some(DoumiScriptData {
        quick_helper: json.도우미메인설정.빠른도우미,
        initial_helper: json.도우미메인설정.초기도우미,
    })
}

/// Get the script for the given mode
fn get_script(script_mode: u8) -> Option<Vec<String>> {
    let guard = SCRIPT_DATA.read().unwrap();
    let data = guard.as_ref()?;
    match script_mode {
        1 => Some(data.quick_helper.clone()),
        2 => Some(data.initial_helper.clone()),
        _ => None,
    }
}

/// doumi.json을 다시 로드. 관리자 명령 '업데이트 도우미'에서 호출.
pub fn reload_doumi_json() -> Result<(), String> {
    let v = load_doumi_json()
        .ok_or_else(|| "doumi.json 재로딩 실패 (파일 없음 또는 형식 오류)".to_string())?;
    *SCRIPT_DATA.write().unwrap() = Some(v);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_state_default() {
        let state = ClientState::default();
        assert_eq!(state, ClientState::Inactive);
    }

    #[test]
    fn test_client_new() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let addr = "127.0.0.1:8080".parse().unwrap();
        let client = Client::new(addr, tx);
        assert_eq!(client.addr, addr);
        assert_eq!(client.state, ClientState::Inactive);
        assert!(client.is_logging_in());
    }

    #[test]
    fn test_login_session_new() {
        let session = LoginSession::new();
        assert_eq!(session.state, LoginState::Logo);
        assert!(session.name.is_empty());
        assert_eq!(session.attempts, 0);
    }

    #[test]
    fn test_complete_login() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let addr = "127.0.0.1:8080".parse().unwrap();
        let mut client = Client::new(addr, tx);
        assert!(client.is_logging_in());
        assert_eq!(client.state, ClientState::Inactive);

        client.complete_login();
        assert!(!client.is_logging_in());
        assert_eq!(client.state, ClientState::Active);
    }

    #[test]
    fn test_read_text_file() {
        // Test reading existing files
        let logo = read_text_file("logoMurim.txt");
        assert!(logo.is_ok());
        // Check for "무림" (file has "무림크래프트뉴얼")
        assert!(logo.unwrap().contains("무림"));

        let notice = read_text_file("notice.txt");
        assert!(notice.is_ok());
    }

    #[test]
    fn test_read_text_file_not_found() {
        let result = read_text_file("nonexistent.txt");
        assert!(result.is_err());
    }
}
