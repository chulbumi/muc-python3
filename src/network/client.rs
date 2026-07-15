//! Client protocol handling for MUD connections
//!
//! Manages individual TCP client connections with line-based protocol
//! and login flow.

#![allow(clippy::type_complexity)]

use bytes::BytesMut;
use rhai::{Array, Dynamic, Map};
use std::cell::RefCell;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::command::commands::{advance_note_body, finish_note, NoteEditAdvance};
use crate::command::{CommandParser, CommandRegistry, CommandResult, ParsedCommand, PendingInput};
use crate::doumi::{run_doumi_to_result, DoumiRunResult};
use crate::emotion::{self, EmotionTarget};
use crate::hangul::han_wa;
use crate::network::social::{RelationState, SocialAction};
use crate::network::DelimiterCodec;
use crate::player::{Body, Player, STATE_ACTIVE};
use crate::script::{
    build_adult_channel_member_snapshot, build_box_observer_snapshot,
    build_party_nonplayer_snapshot, build_party_person_snapshot, build_room_lines,
    build_room_mugong_item_snapshot, build_room_mugong_mob_snapshot,
    build_room_mugong_player_snapshot, build_room_mugong_stack_item_snapshot,
    build_room_objs_grouped, build_room_player_inventory_snapshot,
    build_room_view_player_snapshot_with_interactive, clear_precomputed_all_online,
    clear_precomputed_box_context, clear_precomputed_room_admin_bodies,
    clear_precomputed_room_view_players, immediate_exit_destinations,
    installed_box_party_snapshot_by_pointer, installed_box_party_snapshots, load_body_from_json,
    missing_party_person, password_hash, password_verify, save_body_to_json, set_cast_room_players,
    set_precomputed_adult_channel, set_precomputed_all_online, set_precomputed_box_context,
    set_precomputed_connected_names, set_precomputed_online_names, set_precomputed_party_context,
    set_precomputed_room_admin_bodies, set_precomputed_room_inventories,
    set_precomputed_room_mugong_targets, set_precomputed_room_view_players,
    set_precomputed_tell_players, take_admin_set_player_value_request, take_adult_channel_requests,
    take_auto_move_request, take_box_deliveries, take_change_player_request,
    take_event_command_request, take_force_command_request, take_guild_accept_request,
    take_guild_apply_request, take_guild_kick_request, take_guild_nickname_request,
    take_guild_position_request, take_guild_reset_request, take_guild_transfer_request,
    take_party_requests, take_remove_skill_request, take_save_all_request,
    take_set_player_attr_request, take_set_skill_request, take_summon_player_request,
    take_teach_skill_request, try_fixture_event, try_item_event, verify_and_upgrade_user_password,
    visible_fixture_short_lines, AdultChannelDelivery, BoxDelivery, CastRoomPlayerRef,
    PartyDelivery, TellPlayerSnapshot, PARTY_DISCONNECT_REQUEST,
};
use crate::world::event::{
    run_script_chunk, run_script_chunk_rhai, try_mob_event, ScriptNext, EVENT_DEATH_FINISH_REQUEST,
    EVENT_LP_PROMPT_MARKER,
};
use crate::world::item::{get_item_display_name, get_item_weight_by_key};
use crate::world::{get_world_state, PlayerPosition, RoomCache};
use std::collections::{HashMap, VecDeque};

/// Client connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientState {
    /// Client is connected but not yet logged in
    #[default]
    Inactive,
    /// Client is fully authenticated and active
    Active,
}

/// Sentinel sent to a client's channel to request the send task to exit (kicks the connection).
#[doc(hidden)]
pub const DISCONNECT_SENTINEL: &str = "\x1b__DISCONNECT__\x1b";

/// Python의 Player 객체 identity에 대응하는 프로세스 내 연결 식별자.
/// SocketAddr는 연결 종료 후 재사용될 수 있으므로 `_talker` 관계에 쓰지 않는다.
static NEXT_CONNECTION_TOKEN: AtomicU64 = AtomicU64::new(1);

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
pub struct LoginSession {
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
    _waiting_for_data: Option<&'static str>,
    /// Accumulated delay during $출력시작 block (in ms)
    _accumulated_delay: u64,
    /// Delay to apply after next output (from $틱:N commands)
    delay_after_output: u64,
    /// Rhai 도우미 스크립트 경로 (예: "lib/doumi/빠른도우미"). 비어 있으면 JSON 방식(미사용).
    doumi_script_path: String,
    /// Rhai 도우미 ob (get_name/get_password/get_sex 결과 등). Send 요구로 HashMap<String,String>로 보관.
    doumi_ob: Option<HashMap<String, String>>,
    /// 현재 실행할 단계 함수명 (예: "step1_welcome", "step2_name"). None이면 처음 시작.
    doumi_step: Option<String>,
    /// suspend 시 대기 op (wait_enter, get_name, get_password, get_sex, get_key_input 등)
    doumi_resume_op: Option<String>,
    /// get_key_input용. suspend.expected와 일치하는 입력만 통과.
    doumi_resume_expected: Option<String>,
    /// True when we're actively waiting for user input (wait_enter, wait_input, wait_key_input)
    /// When false, inputs received during script output are discarded
    waiting_for_input: bool,
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
            _waiting_for_data: None,
            _accumulated_delay: 0,
            delay_after_output: 0,
            doumi_script_path: String::new(),
            doumi_ob: None,
            doumi_step: None,
            doumi_resume_op: None,
            doumi_resume_expected: None,
            waiting_for_input: false,
        }
    }

    /// Get the current number of password attempts
    #[allow(dead_code)]
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
    /// 이 Client 객체에만 유효한 opaque identity.
    pub(crate) connection_token: String,
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
    /// 대기 중인 다단계 입력 (암호변경: 이전→새→확인). None이면 일반 명령.
    pub pending_input: Option<PendingInput>,
    /// Python `Client.dataReceived` updates this on every received byte chunk.
    pub(crate) last_input: Instant,
    /// Prevent the one-second loop from enqueuing the same timeout repeatedly.
    pub(crate) disconnect_requested: bool,
}

impl Client {
    /// Create a new client metadata
    pub fn new(addr: SocketAddr, sender: mpsc::UnboundedSender<String>) -> Self {
        Self {
            addr,
            connection_token: format!(
                "client-{}",
                NEXT_CONNECTION_TOKEN.fetch_add(1, Ordering::Relaxed)
            ),
            buffer: BytesMut::with_capacity(1024),
            codec: DelimiterCodec::new(),
            state: ClientState::Inactive,
            sender,
            login_session: Some(LoginSession::new()),
            player: None,
            pending_input: None,
            last_input: Instant::now(),
            disconnect_requested: false,
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
        self.player
            .as_ref()
            .map(|p| p.body.get_string("이름"))
            .unwrap_or_else(|| "방문자".to_string())
    }

    /// Python uses 10 seconds only while state is `INACTIVE`; NOTICE, DOUMI and ACTIVE use
    /// the 180-second timeout. Rust represents NOTICE/DOUMI inside the login session.
    pub(crate) fn uses_inactive_timeout(&self) -> bool {
        if self.state == ClientState::Active {
            return false;
        }
        match self.login_session.as_ref().map(|session| session.state) {
            Some(LoginState::Notice | LoginState::Complete) => false,
            Some(LoginState::ScriptMode) => self
                .login_session
                .as_ref()
                .is_none_or(|session| session.script_mode == 1),
            _ => true,
        }
    }

    fn record_input(&mut self) {
        self.last_input = Instant::now();
        if let Some(player) = self.player.as_mut() {
            player.idle = 0;
        }
    }
}

/// Read a text file from the data/text directory
fn read_text_file(filename: &str) -> Result<String, std::io::Error> {
    let mut path = PathBuf::from("data/text");
    path.push(filename);
    std::fs::read_to_string(&path)
}

/// Send text content to client with proper formatting
#[allow(dead_code)]
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
#[allow(dead_code)]
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
/// `shutdown_notify`: `리부팅` 시 서버 stop 트리거용.
/// None이면 방 갱신까지만 수행하고 프로세스는 계속 실행된다.
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

    // Create channel for send_task to notify read loop when it exits (e.g., due to broken pipe)
    let (send_done_tx, mut send_done_rx) = mpsc::channel::<()>(1);

    // Create client and add through the single lifecycle path.  The player is
    // attached and name-bound only after login completes.
    broadcaster.add_client(Client::new(addr, tx.clone()));

    // Clone tx for use in the read loop
    let tx_clone = tx.clone();

    // Spawn task to handle sending messages to client
    let send_task = tokio::spawn(async move {
        let mut writer = writer;
        let _login_complete = false;

        while let Some(msg) = rx.recv().await {
            // Disconnect sentinel: break to close connection (e.g. kick on duplicate login)
            if msg == DISCONNECT_SENTINEL {
                info!("Send task for {} received disconnect sentinel", addr);
                break;
            }
            // Don't add extra \r\n - messages already have proper line endings
            if let Err(e) = writer.write_all(msg.as_bytes()).await {
                error!(
                    "Failed to send to {} (broken pipe/connection reset): {}",
                    addr, e
                );
                break;
            }
            if let Err(e) = writer.flush().await {
                error!(
                    "Failed to flush to {} (broken pipe/connection reset): {}",
                    addr, e
                );
                break;
            }
            // Send prompt only after login is complete
            // Note: For now, prompts are sent by the read loop after each command
        }
        // Notify read loop that send task is exiting
        let _ = send_done_tx.send(()).await;
        info!("Send task for {} exiting", addr);
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
        // Use tokio::select! to wait for either: read data OR send_task completion (broken pipe)
        tokio::select! {
            // Check if send_task has exited (broken pipe, etc.)
            _ = send_done_rx.recv() => {
                error!("Send task for {} exited (broken pipe), closing connection", addr);
                break;
            }

            // Normal read path
            read_result = reader.read(&mut read_buf) => {
                match read_result {
                    Ok(0) => {
                        // Connection closed by peer
                        debug!("Client {} closed connection", addr);
                        break;
                    }
                    Ok(n) => {
                        let data = &read_buf[..n];
                        // Python resets idle in `dataReceived`, before delimiter parsing.
                        if let Some(client) = broadcaster.clients.lock().get_mut(&addr) {
                            client.record_input();
                        }
                        debug!("Received {} bytes from {}: {:?}", n, addr, data);

                        // Parse lines from data
                        match codec.feed_data(data) {
                            Ok(lines) => {
                                for line in lines {
                                    // Only trim CR/LF, keep spaces for say command detection
                                    // Python MUD uses trailing space/punctuation to detect 'say' command
                                    let line = line.trim_end_matches('\r').trim_end_matches('\n');
                                    // Never log raw input: this path also
                                    // carries login/current/new passwords and
                                    // private command arguments.
                                    debug!("Line from {} ({} bytes)", addr, line.len());

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
            },
        }
    }

    // Python Player.logout() handles Party/follow relationships before the
    // adult channel and remaining connection cleanup.
    leave_party_on_disconnect(&broadcaster, addr, &command_registry);
    leave_adult_channel_on_disconnect(&broadcaster, addr, &command_registry);

    // Cleanup: 최종 월드 위치와 Body를 동기화한 뒤 저장한다.
    // 중복 로그인으로 이미 같은 이름의 새 세션이 있으면, 오래된 세션이
    // 새 세션의 위치를 삭제하거나 사용자 파일을 덮어쓰지 않는다.
    let mut removed_client = broadcaster.remove_client(addr);
    if let Some(client) = removed_client.as_mut() {
        if let Some(player) = client.player_mut() {
            let name = player.body.get_name();
            let replacement_exists =
                !name.is_empty() && broadcaster.find_addr_by_player_name(&name).is_some();

            if !replacement_exists && !name.is_empty() {
                let final_position = get_world_state()
                    .write()
                    .ok()
                    .and_then(|mut world| world.remove_player_position(&name));
                if let Some(position) = final_position {
                    let location = format!("{}:{}", position.zone, position.room);
                    player.body.set("위치", location.as_str());
                    player.body.set("현재방", location.as_str());
                }
                let path = format!("data/user/{}.json", name);
                let _ = save_body_to_json(&mut player.body, &path);
            }
        }
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
        clients
            .get(&addr)
            .map(|c| c.login_session.is_none())
            .unwrap_or(false)
    };
    // Lock is released here
    tracing::debug!(%addr, is_logged_in, "Processed client login state");

    if is_logged_in {
        // 다단계 입력 대기 중이면 (암호변경 등) 해당 플로우 처리
        let has_pending = {
            let clients = broadcaster.clients.lock();
            clients
                .get(&addr)
                .and_then(|c| c.pending_input.as_ref())
                .is_some()
        };
        if has_pending {
            handle_pending_change_password_with_registry(
                broadcaster,
                addr,
                input,
                Some(command_registry.as_ref()),
            )
            .await?;
            return Ok(false);
        }
        handle_game_command(
            broadcaster,
            addr,
            input,
            command_registry,
            room_cache,
            shutdown_notify,
        )
        .await?;
        return Ok(false);
    }

    // bcrypt is intentionally CPU-expensive. Never run it while holding the
    // client map or on a Tokio worker that also serves other connections.
    let password_ok = {
        let identity = {
            let clients = broadcaster.clients.lock();
            clients
                .get(&addr)
                .and_then(|client| client.login_session.as_ref())
                .and_then(|session| {
                    (session.state == LoginState::Password).then(|| session.name.clone())
                })
        };
        if let Some(name) = identity {
            let plain = input.to_string();
            Some(
                tokio::task::spawn_blocking(move || {
                    verify_and_upgrade_user_password(&name, &plain)
                })
                .await
                .unwrap_or(false),
            )
        } else {
            None
        }
    };

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
                // If input is received in Logo state, transition to Name and process the input
                session.state = LoginState::Name;
                // Fall through to Name state processing by returning a special action
                // that will re-process the input in the new state
                // For now, directly process the input here
                let input_name = input.to_string();
                session.name = input_name.clone();
                let is_korean = crate::hangul::is_han(&input_name);
                let is_special =
                    input_name == "손님" || input_name == "무명객" || input_name == "나만바라바";

                debug!(
                    "Name validation (from Logo): name='{}', is_korean={}, is_special={}",
                    input_name, is_korean, is_special
                );
                tracing::debug!(
                    "[NAME VALID] from Logo: name='{}', is_korean={}, is_special={}",
                    input_name,
                    is_korean,
                    is_special
                );

                if input_name.is_empty() {
                    LoginAction::AskName
                } else if !is_korean {
                    session.state = LoginState::Name;
                    LoginAction::NameError(input_name)
                } else if is_special {
                    session.state = LoginState::ScriptMode;
                    session.script_mode = if input_name == "나만바라바" {
                        1
                    } else {
                        2
                    };
                    session.script_position = 0;
                    session.doumi_script_path = if session.script_mode == 1 {
                        "lib/doumi/빠른도우미".to_string()
                    } else {
                        "lib/doumi/초기도우미".to_string()
                    };
                    session.doumi_ob = None;
                    session.doumi_step = None; // None = step1_welcome부터 시작
                    session.doumi_resume_op = None;
                    session.doumi_resume_expected = None;
                    LoginAction::StartScript
                } else {
                    session.state = LoginState::Password;
                    LoginAction::AskPassword(input_name)
                }
            }
            LoginState::Name => {
                let input_name = input.to_string();
                session.name = input_name.clone();
                let is_korean = crate::hangul::is_han(&input_name);
                let is_special =
                    input_name == "손님" || input_name == "무명객" || input_name == "나만바라바";

                info!(
                    "Name validation: name='{}', is_korean={}, is_special={}, bytes={:?}",
                    input_name,
                    is_korean,
                    is_special,
                    input_name.as_bytes()
                );

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
                    session.script_mode = if input_name == "나만바라바" {
                        1
                    } else {
                        2
                    };
                    session.script_position = 0;
                    session.doumi_script_path = if session.script_mode == 1 {
                        "lib/doumi/빠른도우미".to_string()
                    } else {
                        "lib/doumi/초기도우미".to_string()
                    };
                    session.doumi_ob = None;
                    session.doumi_step = None; // None = step1_welcome부터 시작
                    session.doumi_resume_op = None;
                    session.doumi_resume_expected = None;
                    LoginAction::StartScript
                } else {
                    session.state = LoginState::Password;
                    LoginAction::AskPassword(input_name)
                }
            }
            LoginState::Password => {
                session.attempts += 1;
                let ok = password_ok.unwrap_or(false);

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
                        LoginAction::AskKickExisting(())
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
                    LoginAction::AskKickExistingRetry(())
                }
            }
            LoginState::Notice => {
                session.state = LoginState::Complete;
                LoginAction::EnterGame(name)
            }
            LoginState::Complete => LoginAction::GameCommand(name),
            LoginState::ScriptMode => {
                // If script is outputting (not waiting for input), discard any received input
                // This prevents buffered input from being processed when wait_enter/wait_input is reached
                info!(
                    "[ScriptMode] waiting_for_input={}, input={:?}",
                    session.waiting_for_input, input
                );
                if !session.waiting_for_input {
                    LoginAction::None
                } else {
                    // We're actively waiting for input - process it
                    handle_script_state(session, input)
                }
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
                    clients
                        .get(&addr)
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
                            let (
                                new_mode,
                                new_pos,
                                waiting,
                                output_msg,
                                wait_for_input,
                                is_complete,
                                delay,
                            ) = process_script_line(session, "");
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
                    // Check if we should apply line-by-line delays (DOUMI scripts with set_tick)
                    let (delay_per_line, is_doumi) = {
                        let clients = broadcaster.clients.lock();
                        clients
                            .get(&addr)
                            .and_then(|c| c.login_session.as_ref())
                            .map(|s| (s.delay_after_output, !s.doumi_script_path.is_empty()))
                            .unwrap_or((0, false))
                    };

                    if delay_per_line > 0 && is_doumi {
                        // DOUMI script with tick delay: send each line separately with delay
                        for line in msg.split("\r\n") {
                            if !line.is_empty() {
                                broadcaster.send_to(addr, &format!("{}\r\n", line))?;
                                tracing::debug!("[ TICK] Line delay: {}ms", delay_per_line);
                                tokio::time::sleep(tokio::time::Duration::from_millis(
                                    delay_per_line,
                                ))
                                .await;
                            }
                        }
                    } else {
                        // Normal output: send all at once
                        broadcaster.send_to(addr, &msg)?;
                    }
                }

                // Apply tick delay if any (after sending output)
                let delay_to_apply = {
                    let clients = broadcaster.clients.lock();
                    clients
                        .get(&addr)
                        .and_then(|c| c.login_session.as_ref())
                        .map(|s| s.delay_after_output)
                        .unwrap_or(0)
                };
                if delay_to_apply > 0 {
                    tracing::debug!("[ TICK] Applying {}ms delay after output", delay_to_apply);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_to_apply)).await;
                    // Don't clear delay - it persists for all subsequent lines
                }

                // NOW we can accept input - output is fully sent
                if should_wait {
                    let mut clients = broadcaster.clients.lock();
                    if let Some(client) = clients.get_mut(&addr) {
                        if let Some(session) = client.login_session_mut() {
                            session.waiting_for_input = true;
                        }
                    }
                }

                if script_complete {
                    complete_char_creation_and_enter_game(
                        broadcaster,
                        addr,
                        &player_name,
                        command_registry,
                        room_cache,
                    )
                    .await?;
                    return Ok(false);
                }

                if should_wait {
                    // Waiting for user input, exit loop
                    return Ok(false);
                }

                // Apply tick delay if any
                if delay_ms > 0 {
                    tracing::debug!("[ TICK] Sleeping for {}ms", delay_ms);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
                // Small delay to let data be sent
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                // Otherwise continue processing script lines
            }
        }
        LoginAction::ScriptContinue => {
            // Get script output to send
            let (msg, should_wait, script_complete, player_name, _delay_ms, _has_step) = {
                let mut clients = broadcaster.clients.lock();
                let client = clients.get_mut(&addr);
                if let Some(client) = client {
                    if let Some(session) = client.login_session_mut() {
                        info!(
                            "[ScriptContinue] START: step={:?}, mode={}, input={:?}",
                            session.doumi_step, session.script_mode, input
                        );
                        let (
                            new_mode,
                            new_pos,
                            waiting,
                            output_msg,
                            wait_for_input,
                            is_complete,
                            delay,
                        ) = process_script_line(session, input);
                        session.script_mode = new_mode;
                        session.script_position = new_pos;
                        session.waiting_for_command = waiting;
                        let name = session.char_name.clone();
                        let has_step = session.doumi_step.is_some();
                        info!(
                            "[ScriptContinue] END: step={:?}, wait={}, complete={}, msg_len={}, delay={}",
                            session.doumi_step,
                            wait_for_input,
                            is_complete,
                            output_msg.as_ref().map_or(0, |m| m.len()),
                            delay
                        );
                        (
                            output_msg,
                            wait_for_input,
                            is_complete,
                            name,
                            delay,
                            has_step,
                        )
                    } else {
                        (None, false, false, String::new(), 0, false)
                    }
                } else {
                    (None, false, false, String::new(), 0, false)
                }
            };

            // Note: No screen-clear here - Python server doesn't clear screen between DOUMI steps
            // Screen-clear is only sent at StartScript (client.rs:618) for the initial step

            if let Some(msg) = msg {
                // Check if we should apply line-by-line delays (DOUMI scripts with set_tick)
                let (delay_per_line, is_doumi) = {
                    let clients = broadcaster.clients.lock();
                    clients
                        .get(&addr)
                        .and_then(|c| c.login_session.as_ref())
                        .map(|s| (s.delay_after_output, !s.doumi_script_path.is_empty()))
                        .unwrap_or((0, false))
                };

                if delay_per_line > 0 && is_doumi {
                    // DOUMI script with tick delay: send each line separately with delay
                    for line in msg.split("\r\n") {
                        if !line.is_empty() {
                            broadcaster.send_to(addr, &format!("{}\r\n", line))?;
                            tracing::debug!("[ TICK] Line delay: {}ms", delay_per_line);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_per_line))
                                .await;
                        }
                    }
                } else {
                    // Normal output: send all at once
                    broadcaster.send_to(addr, &msg)?;
                }
            }

            // Apply tick delay if any (after sending output)
            let delay_to_apply = {
                let clients = broadcaster.clients.lock();
                clients
                    .get(&addr)
                    .and_then(|c| c.login_session.as_ref())
                    .map(|s| s.delay_after_output)
                    .unwrap_or(0)
            };
            if delay_to_apply > 0 {
                tracing::debug!("[ TICK] Applying {}ms delay after output", delay_to_apply);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_to_apply)).await;
                // Don't clear delay - it persists for all subsequent lines
            }

            // NOW we can accept input - output is fully sent
            if should_wait {
                let mut clients = broadcaster.clients.lock();
                if let Some(client) = clients.get_mut(&addr) {
                    if let Some(session) = client.login_session_mut() {
                        session.waiting_for_input = true;
                    }
                }
            }

            if script_complete {
                // Script ended - complete character creation
                complete_char_creation_and_enter_game(
                    broadcaster,
                    addr,
                    &player_name,
                    command_registry,
                    room_cache,
                )
                .await?;
                Ok(false)
            } else if should_wait {
                // Already sent prompt in the message
                info!("[ScriptContinue] should_wait=true, exiting");
                Ok(false)
            } else {
                // No more immediate output - wait for next user input
                debug!("[ScriptContinue] No more output, waiting for next input");
                Ok(false)
            }
        }
        LoginAction::ScriptComplete(player_name) => {
            complete_char_creation_and_enter_game(
                broadcaster,
                addr,
                &player_name,
                command_registry,
                room_cache,
            )
            .await?;
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
            Ok(true)
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
            broadcaster.send_to(addr, "\r\n\x1b[1;37m접속을 취소합니다.\x1b[0;37m\r\n")?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            Ok(true)
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
            handle_game_command(
                broadcaster,
                addr,
                input,
                command_registry,
                room_cache,
                shutdown_notify,
            )
            .await?;
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
    #[allow(dead_code)]
    ScriptComplete(String),
    /// 동일 접속자 있음: "기존 접속 종료? (네/아니오)" 질의
    AskKickExisting(()),
    AskKickExistingRetry(()),
    /// 기존 접속자 kick 후 새 접속 진행
    KickExistingAndProceed(String),
    /// 새 접속자가 "아니오" 선택 → 현재(새) 접속 끊기
    DisconnectSelf,
    /// 암호 3회 오류 → 접속 끊기
    PasswordWrongDisconnect,
}

/// Send helper selection menu
#[allow(dead_code)]
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

    let option1 =
        "\x1b[1;37m│  \x1b[1;33m1.\x1b[0;37m 빠른도우미 - 빠른 케릭터 생성         │\x1b[0;37m\r\n";
    broadcaster.send_to(addr, option1)?;

    let option2 =
        "\x1b[1;37m│  \x1b[1;33m2.\x1b[0;37m 초기도우미 - 스토리 모드            │\x1b[0;37m\r\n";
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

/// HashMap<String,String> -> rhai::Map (doumi ob). Send 요구로 Map은 호출 시점에만 사용.
fn doumi_hashmap_to_ob(m: HashMap<String, String>) -> Map {
    m.into_iter()
        .map(|(k, v)| (k.into(), Dynamic::from(v)))
        .collect()
}

/// rhai::Map -> HashMap<String,String>. doumi ob 보관용.
fn doumi_ob_to_hashmap(m: Map) -> HashMap<String, String> {
    m.into_iter()
        .filter_map(|(k, v)| v.into_string().ok().map(|s| (k.to_string(), s)))
        .collect()
}

/// Process a single script line and return the result
/// Rhai 도우미(doumi_script_path) 사용. doumi.json 미사용.
/// Returns (new_script_mode, new_position, waiting_for_command, message, should_wait_for_input, script_complete, delay_ms)
fn process_script_line(
    session: &mut LoginSession,
    input: &str,
) -> (u8, usize, Option<String>, Option<String>, bool, bool, u64) {
    tracing::debug!(
        "[process_script_line] input_len={}, doumi_step={:?}, doumi_resume_op={:?}",
        input.len(),
        session.doumi_step,
        session.doumi_resume_op
    );
    tracing::debug!(
        "[process_script_line] doumi_script_path='{}'",
        session.doumi_script_path
    );
    if session.doumi_script_path.is_empty() {
        return (
            session.script_mode,
            0,
            None,
            Some("오류: 도우미 스크립트가 설정되지 않았습니다.\r\n".to_string()),
            true,
            false,
            0,
        );
    }

    let mut ob = session
        .doumi_ob
        .take()
        .map(doumi_hashmap_to_ob)
        .unwrap_or_default();

    tracing::debug!("[process_script_line] loaded ob with {} entries", ob.len());

    // get_name 전용 검증: doumi_resume_op == "get_name"일 때 입력 검사. 실패 시 ob 복원 후 return.
    if session.doumi_resume_op.as_deref() == Some("get_name") {
        let t = input.trim();
        if t.is_empty() || !crate::hangul::is_han(t) {
            session.doumi_ob = Some(doumi_ob_to_hashmap(ob));
            return (
                session.script_mode,
                0,
                None,
                Some("한글 1~6글자만 입력 가능합니다.\r\n무림존함ː\r\n".to_string()),
                true,
                false,
                0,
            );
        }
        if t.chars().count() > 6 {
            session.doumi_ob = Some(doumi_ob_to_hashmap(ob));
            return (
                session.script_mode,
                0,
                None,
                Some("한글 1~6글자만 입력 가능합니다.\r\n무림존함ː\r\n".to_string()),
                true,
                false,
                0,
            );
        }
        if PathBuf::from("data/user")
            .join(format!("{}.json", t))
            .exists()
        {
            session.doumi_ob = Some(doumi_ob_to_hashmap(ob));
            return (
                session.script_mode,
                0,
                None,
                Some("☞ 이미 존재하는 이름은 사용할 수 없습니다.\r\n무림존함ː\r\n".to_string()),
                true,
                false,
                0,
            );
        }
    }

    // get_key_input 검증: expected와 일치할 때만 통과
    if session.doumi_resume_op.as_deref() == Some("get_key_input") {
        if let Some(ref exp) = session.doumi_resume_expected {
            if input.trim() != exp.as_str() {
                session.doumi_ob = Some(doumi_ob_to_hashmap(ob));
                return (
                    session.script_mode,
                    0,
                    None,
                    Some(format!(
                        "잘못된 입력입니다. 『{}』를(을) 입력해 주세요.\r\n> ",
                        exp
                    )),
                    true,
                    false,
                    0,
                );
            }
        }
    }

    // 현재 단계와 resume 정보 가져오기
    let current_step = session.doumi_step.take();
    let resume_op = session.doumi_resume_op.take();
    // wait_enter의 경우 Enter 키를 입력으로 사용하지 않음 (빈 문자열 전달)
    let effective_input = if resume_op.as_deref() == Some("wait_enter") {
        ""
    } else {
        input
    };
    tracing::debug!(
        "[process_script_line] resume_op={:?}, effective_input_len={}",
        resume_op,
        effective_input.len()
    );
    let resume = resume_op.as_ref().map(|o| (o.as_str(), effective_input));

    tracing::debug!(
        "[process_script_line] Calling run_doumi_to_result: current_step={:?}, resume_op={:?}, initial_delay={}",
        current_step,
        resume_op,
        session.delay_after_output
    );

    // Clear waiting flag - we're about to process script output, not waiting for input yet
    // Will be set to true again when we reach the next suspend (wait_enter/wait_input/wait_key_input)
    session.waiting_for_input = false;

    let result = run_doumi_to_result(
        &session.doumi_script_path,
        &mut ob,
        current_step.as_deref(),
        resume,
        session.delay_after_output, // Pass existing delay to preserve tick value across steps
    );

    match result {
        DoumiRunResult::Suspend {
            lines,
            delay_ms,
            suspend,
        } => {
            session.doumi_ob = Some(doumi_ob_to_hashmap(ob.clone()));
            tracing::debug!(
                "[process_script_line] Suspend: saving ob with {} entries",
                ob.len()
            );
            let next_step = suspend.next_step.clone();
            session.doumi_step = next_step.clone();
            session.doumi_resume_op = Some(suspend.op.clone());
            session.doumi_resume_expected = suspend.expected.clone();

            tracing::debug!(
                "[process_script_line] Suspend: op={}, next_step={:?}, lines={}",
                suspend.op,
                next_step,
                lines.len()
            );

            // Store delay for line-by-line output
            session.delay_after_output = delay_ms;

            // Don't duplicate the prompt if it's already in the lines
            let joined = lines.join("");
            let has_prompt = joined.contains(&suspend.prompt)
                || joined.contains("엔터키를 누르세요")
                || joined.contains("엔터키를 누르세요】");
            let output = if has_prompt {
                format!("{}\r\n", joined)
            } else {
                format!("{}{}\r\n", joined, suspend.prompt)
            };
            (
                session.script_mode,
                0,
                None,
                Some(output),
                true,
                false,
                delay_ms,
            )
        }
        DoumiRunResult::Finished {
            name,
            password,
            gender,
        } => {
            let norm = |g: &str| -> String {
                match g.trim().to_lowercase().as_str() {
                    "남" | "남자" | "m" | "male" => "남".to_string(),
                    "여" | "여자" | "f" | "female" => "여".to_string(),
                    _ => "남".to_string(),
                }
            };
            session.char_name = name;
            session.char_password = password;
            session.char_gender = norm(&gender);
            session.doumi_script_path.clear();
            session.doumi_ob = None;
            session.doumi_step = None;
            session.doumi_resume_op = None;
            session.doumi_resume_expected = None;
            (0, 0, None, None, false, true, 0)
        }
    }
}

/// Complete character creation and enter game
async fn complete_char_creation_and_enter_game(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    player_name: &str,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!(
        "[DEBUG COMPLETE] complete_char_creation_and_enter_game called with player_name={}",
        player_name
    );
    // Get character creation data and complete login
    let (char_name, char_gender) = {
        let mut clients = broadcaster.clients.lock();
        let client = clients.get_mut(&addr);

        if let Some(client) = client {
            let name = if !player_name.is_empty() {
                player_name.to_string()
            } else {
                client
                    .login_session
                    .as_ref()
                    .map(|s| s.char_name.clone())
                    .unwrap_or_else(|| "방문자".to_string())
            };
            let gender = client
                .login_session
                .as_ref()
                .map(|s| s.char_gender.clone())
                .unwrap_or_else(|| "남".to_string());
            let pwd = client
                .login_session
                .as_ref()
                .map(|s| s.char_password.clone())
                .unwrap_or_default();

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
                w.spawn_mobs_for_room(&start_pos.zone, &start_pos.room);
            }

            info!("New character created: {} ({})", name, gender);

            (name, gender)
        } else {
            ("방문자".to_string(), "남".to_string())
        }
    };
    // Lock is dropped here
    broadcaster.bind_player_name(&char_name, addr);

    // Send creation complete message
    broadcaster.send_to(
        addr,
        "\r\n\x1b[1;37m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0;37m\r\n",
    )?;
    broadcaster.send_to(addr, "\x1b[1;37m케릭터가 생성되었습니다.\x1b[0;37m\r\n")?;
    broadcaster.send_to(
        addr,
        &format!("\x1b[1;37m이름: {}\x1b[0;37m\r\n", char_name),
    )?;
    broadcaster.send_to(
        addr,
        &format!(
            "\x1b[1;37m성별: {}\x1b[0;37m\r\n",
            if char_gender == "남" {
                "남자"
            } else {
                "여자"
            }
        ),
    )?;
    broadcaster.send_to(addr, "\x1b[1;37m은전: 10000\x1b[0;37m\r\n")?;
    broadcaster.send_to(
        addr,
        "\x1b[1;37m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0;37m\r\n",
    )?;

    // Send welcome message
    broadcaster.send_to(
        addr,
        "\r\n\x1b[1;37m=== 무림에 입장하셨습니다 ===\x1b[0;37m\r\n",
    )?;
    broadcaster.send_to(
        addr,
        "도움말을 보려면 \x1b[1m도움말\x1b[0;37m 또는 \x1b[1mhelp\x1b[0;37m을 입력하세요.\r\n",
    )?;
    broadcaster.send_to(addr, "\r\n")?;

    // Show the starting room
    show_room(broadcaster, addr, &room_cache)?;

    if !run_automatic_adult_channel_join(broadcaster, addr, command_registry, room_cache).await? {
        send_game_prompt(broadcaster, addr).await?;
    }

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
    command_registry: Arc<CommandRegistry>,
    room_cache: &Arc<std::sync::Mutex<RoomCache>>,
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

            // The client must become active before room/prompt delivery; the
            // network input path otherwise keeps treating the first gameplay
            // command as login input.
            client.complete_login();

            // Complete login
            client.complete_login();

            // Create player and initialize
            let mut player = Player::new();
            player.body.set("이름", name.as_str());
            player.body.init_body();
            let _ = load_body_from_json(&mut player.body, &format!("data/user/{}.json", name));
            player.load_aliases_from_body();
            if name == "밍밍" {
                player.body.set("관리자등급", 2000i64);
            }
            player.state = STATE_ACTIVE;
            player.interactive = 1;

            // Python Player.getStart(): 마지막 저장 위치/현재방은 로그인 위치로
            // 사용하지 않는다. 최초 입장과 재접속 모두 귀환지맵을 기준으로 하며,
            // 무림별호 이벤트가 개인 방을 만든 뒤 귀환지맵을 갱신한다.
            let start_pos = {
                let return_loc = player.body.get_string("귀환지맵");
                let destination = if return_loc.is_empty() {
                    "낙양성:42"
                } else {
                    return_loc.as_str()
                };
                if let Some((zone, room)) = destination.split_once(':') {
                    PlayerPosition::new(zone.to_string(), room.to_string())
                } else if let Some((zone, room)) = destination.split_once('/') {
                    PlayerPosition::new(zone.to_string(), room.to_string())
                } else {
                    PlayerPosition::start_fallback()
                }
            };

            // Store the player in the client
            client.set_player(player);

            {
                let mut w = get_world_state().write().unwrap();
                w.set_player_position(name.as_str(), start_pos.clone());
                w.spawn_mobs_for_room(&start_pos.zone, &start_pos.room);
            }

            info!("Player {} logged in from {}", name, addr);

            name
        } else {
            "방문자".to_string()
        }
    };
    // Lock is dropped here
    broadcaster.bind_player_name(&player_name, addr);

    // Send welcome message (no lock held)
    broadcaster.send_to(
        addr,
        "\r\n\x1b[1;37m=== 무림에 입장하셨습니다 ===\x1b[0;37m\r\n",
    )?;
    broadcaster.send_to(
        addr,
        "도움말을 보려면 \x1b[1m도움말\x1b[0;37m 또는 \x1b[1mhelp\x1b[0;37m을 입력하세요.\r\n",
    )?;
    broadcaster.send_to(addr, "\r\n")?;

    // Show the starting room
    show_room_to_player(broadcaster, addr, &player_name).await?;

    if !run_automatic_adult_channel_join(broadcaster, addr, command_registry, room_cache.clone())
        .await?
    {
        send_game_prompt(broadcaster, addr).await?;
    }

    Ok(())
}

/// Python `Player.getStart()` invokes the same adult-channel entry behavior
/// when `자동채널입장` is enabled. Calling the registered Rhai handler keeps
/// hot reload and every user-visible byte on the command side.
async fn run_automatic_adult_channel_join(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let enabled = {
        let clients = broadcaster.clients.lock();
        clients
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            .is_some_and(|player| {
                config_value_is_one(&player.body.get_string("설정상태"), "자동채널입장")
            })
    };
    if !enabled {
        return Ok(false);
    }

    {
        let mut clients = broadcaster.clients.lock();
        if let Some(player) = clients
            .get_mut(&addr)
            .and_then(|client| client.player_mut())
        {
            player.body.temp_mut().insert(
                crate::script::ADULT_CHANNEL_AUTO_JOIN_REQUEST.to_string(),
                crate::object::Value::Int(1),
            );
        }
    }

    handle_single_game_command(
        broadcaster,
        addr,
        "채널입장",
        false,
        command_registry,
        room_cache,
        None,
    )
    .await?;
    Ok(true)
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
        clients
            .get(&addr)
            .and_then(|c| c.player())
            .map(|p| {
                let hp = p.body.get_hp();
                let max_hp = p.body.get_max_hp();
                let mp = p.body.get_mp();
                let max_mp = p.body.get_max_mp();
                // Python Player.lpPrompt() calls prompt(True), and
                // prompt(True) writes CRLF before the HP/MP status. Keep the
                // boundary in the prompt itself so empty rooms cannot attach
                // it to the final compass line.
                format!("\r\n\x1b[0;37;40m[ {}/{}, {}/{} ] ", hp, max_hp, mp, max_mp)
            })
            .unwrap_or_else(|| ">> ".to_string())
    };
    broadcaster.send_to(addr, &prompt)?;
    Ok(())
}

/// Render event output with Python `Player.lpPrompt()` boundaries intact.
/// `$특성치변경` writes its prompt immediately, while normal event text uses
/// `sendLine`; a plain `join("\r\n")` would insert a line break between the
/// raw prompt and the following `sendLine` that Python does not emit.
fn render_event_output_lines(
    output_lines: &[String],
    body: &Body,
    interactive: i32,
) -> (String, bool) {
    let show_lp_prompt = interactive == 1
        && !crate::script::config_is_enabled(&body.get_string("설정상태"), "엘피출력");
    let mut output = String::new();
    let mut wrote_part = false;
    let mut previous_was_raw_prompt = false;

    for line in output_lines {
        if line == EVENT_LP_PROMPT_MARKER {
            if show_lp_prompt {
                // The preceding ordinary item represents sendLine(), whose
                // CRLF comes before lpPrompt()'s own leading CRLF.
                if wrote_part && !previous_was_raw_prompt {
                    output.push_str("\r\n");
                }
                output.push_str(&format!(
                    "\r\n\x1b[0;37;40m[ {}/{}, {}/{} ] ",
                    body.get_hp(),
                    body.get_max_hp(),
                    body.get_mp(),
                    body.get_max_mp()
                ));
                wrote_part = true;
                previous_was_raw_prompt = true;
            }
            continue;
        }

        if wrote_part && !previous_was_raw_prompt {
            output.push_str("\r\n");
        }
        output.push_str(line);
        wrote_part = true;
        previous_was_raw_prompt = false;
    }

    (output, previous_was_raw_prompt)
}

/// Python `$위치이동` reports a missing destination through `sendLine`.
/// Preserve its boundary: with no preceding event output there is no leading
/// blank line; after ordinary output there is exactly one CRLF; after a raw
/// LP prompt the error begins on that prompt line.
fn append_event_move_failure(output: &mut String, ends_with_raw_prompt: &mut bool) {
    if !output.is_empty() && !*ends_with_raw_prompt {
        output.push_str("\r\n");
    }
    output.push_str("어느곳으로도 위치이동 할 수 없습니다.");
    *ends_with_raw_prompt = false;
}

/// `$위치이동` calls Python `Player.enterRoom`, so a real destination can
/// still reject the actor.  The directive's preceding `sendLine('')` is only
/// reached after `getRoom` succeeds; preserve that same event-output boundary
/// for every `enterRoom` guard message.
fn append_event_summon_rejection(
    output: &mut String,
    ends_with_raw_prompt: &mut bool,
    reason: &str,
) {
    let text = match reason {
        "pressure" => "강한 무형의 기운이 당신을 압박합니다.",
        "room_full" => "☞ 알 수 없는 무형의 기운이 당신을 가로막습니다. ^_^",
        "evil_forbidden" => "☞ 사파는 출입할 수 없는 곳이라네!",
        "good_forbidden" => "☞ 정파는 출입할 수 없는 곳이라네!",
        "guild_forbidden" => "☞ 그곳은 타 방파의 지역이므로 출입하실 수 없습니다.",
        _ => return,
    };
    if output.is_empty() || *ends_with_raw_prompt {
        output.push_str("\r\n");
    } else {
        output.push_str("\r\n\r\n");
    }
    output.push_str(text);
    *ends_with_raw_prompt = false;
}

/// Python `$위치이동` first performs `sendLine('')`, then
/// `exitRoom(..., "소환")`.  The self-facing departure text therefore has a
/// blank-line boundary after ordinary event text (and no text at all while
/// invisible).  Destination observers are handled by the room transition;
/// this helper preserves the actor's otherwise easy-to-lose wire output.
fn append_event_summon_departure(
    output: &mut String,
    ends_with_raw_prompt: &mut bool,
    body: &Body,
) {
    if body.get_int("투명상태") == 1 {
        return;
    }
    if output.is_empty() || *ends_with_raw_prompt {
        output.push_str("\r\n");
    } else {
        output.push_str("\r\n\r\n");
    }
    output.push_str("당신이 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'");
    *ends_with_raw_prompt = false;
}

/// Consume a lethal `$체력감소/$체력소모` request through the same Rhai
/// scripts used by combat death: first the collapse presentation, then the
/// Python-compatible inventory drop/coma presentation.
fn finish_lethal_event_rhai(registry: &CommandRegistry, body: &mut Body) -> String {
    if body.temp_mut().remove(EVENT_DEATH_FINISH_REQUEST).is_none() {
        return String::new();
    }

    let mut output = String::new();
    for name in ["combat_tick", "death"] {
        let Some(handler) = registry.get_internal(name) else {
            continue;
        };
        let result = handler(body, &[]);
        let line = match result {
            CommandResult::Output(line) | CommandResult::OutputAndSendToUsers(line, _) => line,
            _ => String::new(),
        };
        if line.is_empty() {
            continue;
        }
        if !output.is_empty() {
            output.push_str("\r\n");
        }
        output.push_str(&line);
    }
    output
}

fn config_value_is_one(config: &str, key: &str) -> bool {
    crate::script::config_is_enabled(config, key)
}

fn apply_adult_channel_requests(
    broadcaster: &crate::network::Broadcaster,
    actor_addr: SocketAddr,
    action: Option<String>,
    deliveries: Vec<AdultChannelDelivery>,
) {
    match action.as_deref() {
        Some("join") => {
            broadcaster.join_adult_channel(actor_addr);
        }
        Some("leave") => {
            broadcaster.leave_adult_channel(actor_addr);
        }
        _ => {}
    }

    for delivery in deliveries {
        let Ok(member_addr) = delivery.member_id.parse::<SocketAddr>() else {
            continue;
        };
        if broadcaster
            .send_to(member_addr, &delivery.raw_text)
            .is_err()
        {
            broadcaster.leave_adult_channel(member_addr);
            continue;
        }
    }
}

/// Create visual compass string for room exits (방향만, 숨겨진 제외)
#[allow(dead_code)]
fn format_exit_compass(room: &crate::world::Room) -> String {
    use crate::world::Direction;

    let has = |d: Direction| {
        room.exits
            .values()
            .any(|e| e.direction == Some(d) && e.has_destination() && !e.hidden)
    };
    let has_north = has(Direction::North);
    let has_south = has(Direction::South);
    let has_east = has(Direction::East);
    let has_west = has(Direction::West);

    let mut directions = Vec::new();
    if has_north {
        directions.push("북");
    }
    if has_south {
        directions.push("남");
    }
    if has_east {
        directions.push("동");
    }
    if has_west {
        directions.push("서");
    }
    if has(Direction::Up) {
        directions.push("위");
    }
    if has(Direction::Down) {
        directions.push("아래");
    }

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
        compass.push(' ');
    }
    compass.push('○');
    if has_east {
        compass.push_str("\x1b[32m▷\x1b[37m");
    } else {
        compass.push(' ');
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
        clients
            .get(&addr)
            .map(|c| c.player_name())
            .unwrap_or_else(|| "방문자".to_string())
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
                    w.spawn_mobs_for_room(&start_pos.zone, &start_pos.room);
                }
                start_pos
            }
        }
    };

    // Get room from cache
    let world = get_world_state().read().unwrap();
    let _room_key = format!("{}:{}", pos.zone, pos.room);
    let room_output = if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room) {
        let room_ref = room
            .read()
            .map_err(|e| format!("Room read lock error: {}", e))?;

        // Room name format
        let room_name_formatted = format!(
            "\x1b[1;30m[\x1b[0;37m[[\x1b[1;37m[]\x1b[1m {} \x1b[1;37m[]\x1b[0;37m]]\x1b[1;30m]\x1b[0;37m",
            room_ref.display_name
        );

        // Get exits (방향은 korean_name, 고유명은 display_name. 숨겨진 제외)
        let exits: Vec<String> = room_ref
            .exits
            .values()
            .filter(|e| e.has_destination() && !e.hidden)
            .map(|e| {
                e.direction
                    .as_ref()
                    .map(|d| d.korean_name().to_string())
                    .unwrap_or_else(|| e.display_name.clone())
            })
            .collect();
        let exits_str = if exits.is_empty() {
            "출구가 없습니다.".to_string()
        } else {
            format!("◁○   〔{}〕쪽으로 이동할 수 있습니다.", exits.join(" "))
        };
        let fixture_str = visible_fixture_short_lines(&world, &pos.zone, &pos.room).join("\r\n");
        let fixture_str = if fixture_str.is_empty() {
            String::new()
        } else {
            format!("{fixture_str}\r\n")
        };

        // Get mobs in room
        let mobs = world.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
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
        let room_objs = world.get_room_objs(&pos.zone, &pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, &pos.room);
        let item_str = build_room_objs_grouped(&room_objs, &room_stack);

        // 같은 방의 다른 접속 유저. 봐/show_room_to_player_with_world와 동일.
        let other_descs =
            get_other_players_desc_in_room(broadcaster.as_ref(), &pos.zone, &pos.room, player_name);
        let other_str = if other_descs.is_empty() {
            String::new()
        } else {
            format!("\r\n{}", other_descs.join("\r\n"))
        };

        format!(
            "{}\r\n{}\r\n{}{}{}{}{}\r\n[ {}/{} , {}/{} ]\r\n",
            room_name_formatted,
            room_ref.description.join("\r\n"),
            fixture_str,
            exits_str,
            mob_str,
            item_str,
            other_str,
            100,
            900,
            18,
            18 // Default HP/MP display
        )
    } else {
        format!(
            "\x1b[1;37m[{}:{}]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n",
            pos.zone, pos.room
        )
    };

    broadcaster.send_to(addr, "\r\n")?;
    broadcaster.send_to(addr, &room_output)?;
    Ok(())
}

// handle_game_command가 이미 clients 락을 보유한 상태에서 봐 스크립트를 실행할 때
// get_other_players_desc → get_other_players_desc_in_room이 같은 락을 다시 잡으면 데드락이 난다.
// 이 스레드로컬이 Some이면 그 값을 그대로 반환하고, None이면 broadcaster.clients.lock() 후 수집.
thread_local! {
    static PRE_COMPUTED_OTHER_DESCS: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}
thread_local! {
    static PRE_COMPUTED_OTHER_MAP: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

/// 락 보유 중에 get_other가 재진입하지 않도록, 미리 수집해 두고 핸들러 호출 전/후로 set/clear.
struct PreComputedOtherDescsGuard;
impl Drop for PreComputedOtherDescsGuard {
    fn drop(&mut self) {
        PRE_COMPUTED_OTHER_DESCS.with(|c| *c.borrow_mut() = None);
        PRE_COMPUTED_OTHER_MAP.with(|c| *c.borrow_mut() = None);
        clear_precomputed_room_view_players();
        clear_precomputed_room_admin_bodies();
        clear_precomputed_all_online();
        clear_precomputed_box_context();
    }
}

fn collect_other_players_from_map(
    exclude_name: &str,
    room_player_bindings: &[(String, SocketAddr)],
    clients_map: &HashMap<SocketAddr, Client>,
) -> (Vec<String>, HashMap<String, String>) {
    let mut out = Vec::new();
    let mut map = HashMap::new();
    for (indexed_name, addr) in room_player_bindings {
        if indexed_name == exclude_name {
            continue;
        }
        let Some(player) = clients_map
            .get(addr)
            .and_then(|client| client.player.as_ref())
        else {
            continue;
        };
        let name = player.body.get_string("이름");
        if name.is_empty()
            || name != *indexed_name
            || player.body.object_ref().getInt("투명상태") == 1
        {
            continue;
        }
        let desc = player.body.get_desc_for_look(false);
        out.push(desc.clone());
        map.insert(name, desc);
    }
    (out, map)
}

fn install_adult_channel_snapshot(
    broadcaster: &crate::network::Broadcaster,
    clients: &HashMap<SocketAddr, Client>,
    self_addr: SocketAddr,
) {
    let members = broadcaster
        .adult_channel_members()
        .into_iter()
        .filter_map(|member_addr| {
            let client = clients.get(&member_addr)?;
            let player = client.player.as_ref()?;
            Some(build_adult_channel_member_snapshot(
                member_addr.to_string(),
                &player.body,
                player.state == STATE_ACTIVE,
                player.interactive,
            ))
        })
        .collect::<Array>();
    set_precomputed_adult_channel(
        members,
        self_addr.to_string(),
        broadcaster.is_adult_channel_member(self_addr),
    );
}

/// Install only the actor's direct follower/Party relations plus the ordered
/// players in its current room. Related objects are resolved by opaque
/// connection token; room candidates use WorldState's room index and the
/// player-name index, never a global client scan.
fn install_party_context(
    broadcaster: &crate::network::Broadcaster,
    clients: &HashMap<SocketAddr, Client>,
    world: &crate::world::WorldState,
    actor_addr: SocketAddr,
) -> Option<String> {
    let actor = clients.get(&actor_addr)?;
    let actor_player = actor.player.as_ref()?;
    let actor_id = actor.connection_token.clone();
    let social = broadcaster.social_snapshot(&actor_id);

    let mut related_ids = vec![actor_id.clone()];
    let mut push_related = |id: &str| {
        if !id.is_empty() && !related_ids.iter().any(|existing| existing == id) {
            related_ids.push(id.to_string());
        }
    };
    if let Some(follow) = social.follow.as_deref() {
        push_related(follow);
    }
    for follower in &social.followers {
        push_related(follower);
    }
    if let Some(leader) = social.party_leader.as_deref() {
        push_related(leader);
    }
    for member in &social.party_members {
        push_related(member);
    }

    let mut people = HashMap::<String, Dynamic>::new();
    let related_bindings = broadcaster.connection_bindings_for_tokens(&related_ids);
    for (connection_id, addr) in related_bindings {
        let relation = social
            .relations
            .get(&connection_id)
            .cloned()
            .unwrap_or_default();
        let Some(player) = clients.get(&addr).and_then(|client| client.player.as_ref()) else {
            continue;
        };
        let interactive = clients
            .get(&addr)
            .map(|client| {
                client
                    .player
                    .as_ref()
                    .map_or(0, |player| player.interactive)
            })
            .unwrap_or(0);
        people.insert(
            connection_id.clone(),
            build_party_person_snapshot(connection_id, &player.body, relation, interactive),
        );
    }
    for connection_id in &related_ids {
        people.entry(connection_id.clone()).or_insert_with(|| {
            missing_party_person(
                connection_id.clone(),
                social
                    .relations
                    .get(connection_id)
                    .cloned()
                    .unwrap_or_default(),
            )
        });
    }

    let actor_name = actor_player.body.get_name();
    let actor_position = world.get_player_position(&actor_name);
    let room_player_names = actor_position
        .map(|position| world.get_players_in_room(&position.zone, &position.room))
        .unwrap_or_default();
    let room_bindings = broadcaster.player_bindings_for_names(&room_player_names);
    let mut room_players = Array::new();
    let mut room_object_lookup_supported = room_bindings.len() == room_player_names.len();
    for (indexed_name, addr) in room_bindings {
        let Some(client) = clients.get(&addr) else {
            room_object_lookup_supported = false;
            continue;
        };
        let Some(player) = client.player.as_ref() else {
            room_object_lookup_supported = false;
            continue;
        };
        if player.body.get_name() != indexed_name {
            room_object_lookup_supported = false;
            continue;
        }
        let connection_id = client.connection_token.clone();
        let person = people.entry(connection_id.clone()).or_insert_with(|| {
            build_party_person_snapshot(
                connection_id,
                &player.body,
                social
                    .relations
                    .get(&client.connection_token)
                    .cloned()
                    .unwrap_or_else(RelationState::default),
                player.interactive,
            )
        });
        room_players.push(person.clone());
    }

    // Python Room.findObjName selects from one room.objs sequence before the
    // command checks is_player(). Keep non-players both as a legacy ambiguity
    // guard and in the unified Rust room-object sequence when reconstructable.
    let mut room_nonplayers = Array::new();
    let mut room_objects = Array::new();
    if let Some(position) = actor_position {
        for mob in world
            .mob_cache
            .get_all_mobs_in_room(&position.zone, &position.room)
        {
            let Some(data) = world.mob_cache.get_mob(&mob.mob_key) else {
                room_object_lookup_supported = false;
                continue;
            };
            let snapshot = build_room_mugong_mob_snapshot(mob, data);
            room_nonplayers.push(build_party_nonplayer_snapshot(&snapshot));
        }
        for item in world.get_room_objs(&position.zone, &position.room) {
            let Ok(item) = item.lock() else {
                room_object_lookup_supported = false;
                continue;
            };
            let snapshot = build_room_mugong_item_snapshot(&item);
            room_nonplayers.push(build_party_nonplayer_snapshot(&snapshot));
        }
        let mut compressed_items: Vec<_> = world
            .get_room_objs_stack(&position.zone, &position.room)
            .into_iter()
            .collect();
        compressed_items.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (key, count) in compressed_items {
            if count <= 0 {
                continue;
            }
            let Some(snapshot) = build_room_mugong_stack_item_snapshot(&key, count) else {
                room_object_lookup_supported = false;
                continue;
            };
            room_nonplayers.push(build_party_nonplayer_snapshot(&snapshot));
        }
        match installed_box_party_snapshots(&position.zone, &position.room) {
            Some(installed_boxes) => room_nonplayers.extend(installed_boxes),
            None => room_object_lookup_supported = false,
        }

        let mobs = world
            .mob_cache
            .get_all_mobs_in_room(&position.zone, &position.room);
        let floor = world.get_room_objs(&position.zone, &position.room);
        let installed =
            installed_box_party_snapshots(&position.zone, &position.room).unwrap_or_default();
        for object in world.get_room_object_order(&position.zone, &position.room) {
            let selected = match object {
                crate::world::RoomObjectRef::Player(name) => room_players
                    .iter()
                    .find(|person| {
                        (*person)
                            .clone()
                            .try_cast::<Map>()
                            .and_then(|person| person.get("name").cloned())
                            .and_then(|name_value| name_value.into_string().ok())
                            .is_some_and(|candidate| candidate == name)
                    })
                    .cloned(),
                crate::world::RoomObjectRef::Mob(id) => mobs
                    .iter()
                    .find(|mob| mob.instance_id == id)
                    .and_then(|mob| {
                        world
                            .mob_cache
                            .get_mob(&mob.mob_key)
                            .map(|data| (mob, data))
                    })
                    .map(|(mob, data)| {
                        build_party_nonplayer_snapshot(&build_room_mugong_mob_snapshot(mob, data))
                    }),
                crate::world::RoomObjectRef::FloorItem(pointer) => floor
                    .iter()
                    .find(|item| Arc::as_ptr(item) as usize == pointer)
                    .and_then(|item| item.lock().ok())
                    .map(|item| {
                        build_party_nonplayer_snapshot(&build_room_mugong_item_snapshot(&item))
                    }),
                crate::world::RoomObjectRef::InstalledBox(ordinal) => {
                    installed.get(ordinal).cloned()
                }
                crate::world::RoomObjectRef::SummonedUser(id) => world
                    .summoned_users()
                    .iter()
                    .find(|user| user.id == id)
                    .and_then(|user| {
                        build_party_person_snapshot(
                            String::new(),
                            &user.body,
                            RelationState::default(),
                            0,
                        )
                        .try_cast::<Map>()
                    })
                    .map(|mut snapshot| {
                        // It is a Python Player for lookup ordering, but it
                        // has no connection token that Rust can bind into the
                        // social graph. Exact selection must stop here rather
                        // than disabling every lookup in the room.
                        snapshot.insert("kind".into(), Dynamic::from("unbound_player"));
                        Dynamic::from(snapshot)
                    }),
                crate::world::RoomObjectRef::Box(pointer) => {
                    installed_box_party_snapshot_by_pointer(&position.zone, &position.room, pointer)
                }
                crate::world::RoomObjectRef::Fixture(_) => None,
            };
            if let Some(selected) = selected {
                room_objects.push(selected);
            }
        }
    }

    let person = |id: Option<&str>| -> Dynamic {
        id.and_then(|id| people.get(id).cloned())
            .unwrap_or_else(|| missing_party_person(String::new(), RelationState::default()))
    };
    let people_array = |ids: &[String]| -> Array {
        ids.iter()
            .filter_map(|id| people.get(id).cloned())
            .collect()
    };

    let mut context = Map::new();
    context.insert("self_id".into(), Dynamic::from(actor_id.clone()));
    context.insert("self".into(), person(Some(&actor_id)));
    context.insert("follow".into(), person(social.follow.as_deref()));
    context.insert(
        "followers".into(),
        Dynamic::from(people_array(&social.followers)),
    );
    context.insert(
        "party_leader".into(),
        person(social.party_leader.as_deref()),
    );
    context.insert(
        "party_members".into(),
        Dynamic::from(people_array(&social.party_members)),
    );
    context.insert("room_players".into(), Dynamic::from(room_players));
    context.insert("room_nonplayers".into(), Dynamic::from(room_nonplayers));
    context.insert("room_objects".into(), Dynamic::from(room_objects));
    context.insert(
        "room_object_lookup_supported".into(),
        Dynamic::from(room_object_lookup_supported),
    );
    set_precomputed_party_context(context);
    Some(actor_id)
}

fn apply_party_requests(
    broadcaster: &crate::network::Broadcaster,
    actor_id: &str,
    action: Option<SocialAction>,
    deliveries: Vec<PartyDelivery>,
) {
    if let Some(action) = action {
        let assignments = match &action {
            SocialAction::SetCombatTargets { owner, targets } => {
                vec![(owner.clone(), targets.clone())]
            }
            SocialAction::SetPartyCombatTargets { assignments, .. } => assignments.clone(),
            _ => Vec::new(),
        };
        let tanker = match &action {
            SocialAction::SetPartyCombatTargets { tanker, .. } => tanker.clone(),
            _ => None,
        };
        let prepend_all = matches!(
            &action,
            SocialAction::SetPartyCombatTargets {
                prepend_all: true,
                ..
            }
        );
        let target_instances = match &action {
            SocialAction::SetPartyCombatTargets {
                target_instances, ..
            } => target_instances.clone(),
            _ => Vec::new(),
        };
        let mut world_links = Vec::new();
        for (owner, targets) in assignments {
            if let Some(addr) = broadcaster.find_addr_by_connection_token(&owner) {
                if let Some(client) = broadcaster.clients.lock().get_mut(&addr) {
                    if let Some(player) = client.player_mut() {
                        player.body.temp_mut().insert(
                            "_combat_target_ids".to_string(),
                            crate::object::Value::String(targets.join("\n")),
                        );
                        if target_instances.is_empty() {
                            player.body.temp_mut().remove("_combat_target_instance_ids");
                        } else {
                            player.body.temp_mut().insert(
                                "_combat_target_instance_ids".to_string(),
                                crate::object::Value::String(
                                    target_instances
                                        .iter()
                                        .map(u64::to_string)
                                        .collect::<Vec<_>>()
                                        .join("\n"),
                                ),
                            );
                        }
                        player.body.act = crate::player::ActState::Fight;
                        world_links.push((
                            owner.clone(),
                            player.body.get_name(),
                            targets.clone(),
                            target_instances.clone(),
                            tanker.as_deref() == Some(owner.as_str()),
                        ));
                    }
                }
            }
        }
        if let Ok(mut world) = crate::world::get_world_state().write() {
            for (_, player_name, _targets, target_instances, is_tanker) in world_links {
                let Some(position) = world.get_player_position(&player_name).cloned() else {
                    continue;
                };
                let Some(mobs) = world
                    .mob_cache
                    .get_all_mobs_in_room_mut(&position.zone, &position.room)
                else {
                    continue;
                };
                link_party_player_to_mobs(
                    mobs,
                    &player_name,
                    &target_instances,
                    is_tanker,
                    prepend_all,
                );
            }
        }
        broadcaster.apply_social_action(actor_id, action);
    }
    for delivery in deliveries {
        let Some(addr) = broadcaster.find_addr_by_connection_token(&delivery.connection_id) else {
            continue;
        };
        if broadcaster.send_to(addr, &delivery.raw_text).is_err() {
            broadcaster.remove_client(addr);
        }
    }
}

fn link_party_player_to_mobs(
    mobs: &mut [crate::world::MobInstance],
    player_name: &str,
    target_instances: &[u64],
    is_tanker: bool,
    prepend_all: bool,
) {
    for mob in mobs {
        if !target_instances.contains(&mob.instance_id) {
            continue;
        }
        mob.act = 1;
        mob.targets.retain(|target| target != player_name);
        if is_tanker || prepend_all {
            mob.targets.insert(0, player_name.to_string());
        } else {
            mob.targets.push(player_name.to_string());
        }
    }
}

#[cfg(test)]
mod party_combat_instance_tests {
    use super::{collected_user_payload, link_party_player_to_mobs};
    use crate::world::{MobInstance, RawMobData};

    #[test]
    fn party_combat_links_only_the_exact_duplicate_mob_instance() {
        let mut data = RawMobData::new();
        data.name = "동종무리합대상".into();
        let first = MobInstance::new("동종:키".into(), "동종존".into(), "1", &data);
        let selected_id = first.instance_id;
        let second = MobInstance::new("동종:키".into(), "동종존".into(), "1", &data);
        let mut mobs = vec![first, second];

        link_party_player_to_mobs(&mut mobs, "합류자", &[selected_id], false, true);

        assert_eq!(mobs[0].act, 1);
        assert_eq!(mobs[0].targets, vec!["합류자"]);
        assert_eq!(mobs[1].act, 0);
        assert!(mobs[1].targets.is_empty());
    }

    #[test]
    fn raw_collected_user_payload_bypasses_the_send_line_wrapper() {
        let wire = "\r\n당신은 파문되었습니다.\r\n\r\n프롬프트";
        let tagged = format!("{}{wire}", crate::script::RAW_USER_MESSAGE_PREFIX);
        assert_eq!(collected_user_payload(&tagged), (wire, true));
        assert_eq!(collected_user_payload("일반문구"), ("일반문구", false));
    }
}

/// Deliver Rhai-authored Box room notifications by opaque connection token.
/// Each payload already includes Python `sendLine` and conditional `lpPrompt`
/// bytes, so the network layer must not wrap or format it.
fn apply_box_deliveries(broadcaster: &crate::network::Broadcaster, deliveries: Vec<BoxDelivery>) {
    for delivery in deliveries {
        let Some(addr) = broadcaster.find_addr_by_connection_token(&delivery.connection_id) else {
            continue;
        };
        if broadcaster.send_to(addr, &delivery.raw_text).is_err() {
            broadcaster.remove_client(addr);
        }
    }
}

/// Python `Player.logout()` removes an adult-channel member and runs the same
/// recipient notification/prompt sequence as `채널퇴장`, without the self
/// confirmation line. The Rhai command remains the sole owner of that text.
fn leave_adult_channel_on_disconnect(
    broadcaster: &crate::network::Broadcaster,
    addr: SocketAddr,
    command_registry: &CommandRegistry,
) {
    if !broadcaster.is_adult_channel_member(addr) {
        return;
    }

    let requests = {
        let mut clients = broadcaster.clients.lock();
        install_adult_channel_snapshot(broadcaster, &clients, addr);
        let Some(player) = clients
            .get_mut(&addr)
            .and_then(|client| client.player_mut())
        else {
            clear_precomputed_all_online();
            broadcaster.leave_adult_channel(addr);
            return;
        };
        player.body.temp_mut().insert(
            crate::script::ADULT_CHANNEL_DISCONNECT_REQUEST.to_string(),
            crate::object::Value::Int(1),
        );
        if let Some(command) = command_registry.get("채널퇴장") {
            let _ = (command.handler)(&mut player.body, &[]);
        }
        take_adult_channel_requests(&mut player.body)
    };
    clear_precomputed_all_online();

    // Even a missing/broken hot-reload script must not leave a dead Player
    // identity in the runtime membership list.
    if requests.0.as_deref() != Some("leave") {
        broadcaster.leave_adult_channel(addr);
    }
    apply_adult_channel_requests(broadcaster, addr, requests.0, requests.1);
}

/// Run Python `Player.logout()` follower/Party cleanup through the Rhai-owned
/// output plan before removing the connection identity.
fn leave_party_on_disconnect(
    broadcaster: &crate::network::Broadcaster,
    addr: SocketAddr,
    command_registry: &CommandRegistry,
) {
    let actor_id = broadcaster
        .clients
        .lock()
        .get(&addr)
        .map(|client| client.connection_token.clone());
    let Some(actor_id) = actor_id else {
        return;
    };
    if !broadcaster.has_social_relations(&actor_id) {
        return;
    }

    let requests = {
        let mut clients = broadcaster.clients.lock();
        let world = get_world_state().read().unwrap();
        if install_party_context(broadcaster, &clients, &world, addr).is_none() {
            drop(world);
            drop(clients);
            clear_precomputed_all_online();
            broadcaster.apply_social_action(&actor_id, SocialAction::Disconnect);
            return;
        }
        drop(world);
        let Some(player) = clients
            .get_mut(&addr)
            .and_then(|client| client.player_mut())
        else {
            drop(clients);
            clear_precomputed_all_online();
            broadcaster.apply_social_action(&actor_id, SocialAction::Disconnect);
            return;
        };
        player.body.temp_mut().insert(
            PARTY_DISCONNECT_REQUEST.to_string(),
            crate::object::Value::Int(1),
        );
        if let Some(command) = command_registry.get("무리") {
            let _ = (command.handler)(&mut player.body, &[]);
        }
        take_party_requests(&mut player.body)
    };
    clear_precomputed_all_online();

    // A missing/broken hot-reload script must still release object-identity
    // state; only its user-visible notifications may be absent.
    let _requested_action = requests.0;
    apply_party_requests(
        broadcaster,
        &actor_id,
        Some(SocialAction::Disconnect),
        requests.1,
    );
}

/// 같은 방(zone, room)에 있는 다른 접속 유저들의 getDesc. 파이썬 viewMapData의 for obj in room.objs: is_player, getDesc.
/// 투명상태==1인 유저는 제외. 파이썬: if obj['투명상태'] == 1: continue
pub(crate) fn get_other_players_desc_in_room(
    broadcaster: &crate::network::Broadcaster,
    zone: &str,
    room: &str,
    exclude_name: &str,
) -> Vec<String> {
    if let Some(taken) = PRE_COMPUTED_OTHER_DESCS.with(|c| c.borrow_mut().take()) {
        return taken;
    }
    let room_player_names = crate::world::get_world_state()
        .read()
        .unwrap()
        .get_players_in_room(zone, room);
    let room_player_bindings = broadcaster.player_bindings_for_names(&room_player_names);
    let clients = broadcaster.clients.lock();
    collect_other_players_from_map(exclude_name, &room_player_bindings, &clients).0
}

/// find_target(봐 [대상])에서 같은 방 다른 유저 매칭용. PRE가 설정돼 있으면 그걸 반환(데드락 회피).
pub(crate) fn get_other_players_map_for_look() -> HashMap<String, String> {
    PRE_COMPUTED_OTHER_MAP
        .with(|c| c.borrow_mut().take())
        .unwrap_or_default()
}

/// 감정표현 대상: 같은 방에서 name으로 플레이어 또는 몹 검색. self_name이면 None.
/// 파이썬 objs/player.doEmotion → env.findObjName(l[0]), objs/room.findObjName.
/// - 플레이어: 이름 일치. 접촉거부 시 kd[2] 사용·buf2 전송.
/// - 몹: 이름 일치, 반응이름 일치, 또는 반응이름 중 alias.find(name)==0(접두사).
fn find_emotion_target_in_room(
    zone: &str,
    room: &str,
    name: &str,
    self_name: &str,
    world: &crate::world::WorldState,
    room_player_bindings: &[(String, SocketAddr)],
    clients: &HashMap<SocketAddr, Client>,
) -> Option<EmotionTarget> {
    if name.is_empty() || name == self_name {
        return None;
    }
    // 플레이어: 이름 일치
    if let Some((_, addr)) = room_player_bindings
        .iter()
        .find(|(indexed_name, _)| indexed_name == name)
    {
        if let Some(player) = clients
            .get(addr)
            .and_then(|client| client.player.as_ref())
            .filter(|player| player.body.get_string("이름") == name)
        {
            let contact_refuse = player.body.get_string("설정상태").contains("접촉거부 1");
            return Some(EmotionTarget::Player {
                name: name.to_string(),
                contact_refuse,
            });
        }
    }
    // 몹: 이름, 반응이름 일치, 또는 반응이름 접두사(alias.find(name)==0)
    for mob in world.mob_cache.get_mobs_in_room(zone, room) {
        if !mob.alive {
            continue;
        }
        if mob.name == name {
            return Some(EmotionTarget::Mob {
                name: mob.name.clone(),
            });
        }
        if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
            let ok = data.reaction_names.iter().any(|r| r == name)
                || data.reaction_names.iter().any(|r| r.starts_with(name));
            if ok {
                return Some(EmotionTarget::Mob {
                    name: mob.name.clone(),
                });
            }
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
    room: &str,
    exclude: &[&str],
    msg: &str,
) {
    let room_player_names = get_world_state()
        .read()
        .unwrap()
        .get_players_in_room(zone, room);
    let room_player_bindings = broadcaster.player_bindings_for_names(&room_player_names);
    let clients = broadcaster.clients.lock();
    let line = format!("\r\n{}\r\n", msg);
    let mut dead_addrs = Vec::new();

    for (indexed_name, addr) in room_player_bindings {
        if exclude.iter().any(|excluded| *excluded == indexed_name) {
            continue;
        }
        let Some(client) = clients.get(&addr) else {
            continue;
        };
        let Some(player) = client.player.as_ref() else {
            continue;
        };
        if player.body.get_string("이름") != indexed_name
            || player.body.object_ref().getInt("투명상태") == 1
        {
            continue;
        }
        if let Err(_e) = client.sender.send(line.clone()) {
            // Send failed - client likely has broken pipe
            tracing::debug!("Failed to send to {} (connection dead)", addr);
            dead_addrs.push(addr);
        }
    }

    // Clean up dead clients
    drop(clients);
    for addr in dead_addrs {
        tracing::warn!(
            "Removing dead client {} due to send failure in send_to_others_in_room",
            addr
        );
        broadcaster.remove_client(addr);
    }
}

/// 외쳐(shout): 게임 접속 전체에 전송. Active이고 외침거부가 아닌 클라이언트에만.
/// clients 락을 잡은 채 send_to를 호출하면 데드락이 나므로, client.sender로 직접 전송.
pub(crate) fn broadcast_shout(broadcaster: &crate::network::Broadcaster, msg: &str) {
    use crate::network::ClientState;
    let clients = broadcaster.clients.lock();
    let line = format!("\r\n{}\r\n", msg);
    let mut dead_addrs = Vec::new();

    for (&addr, client) in clients.iter() {
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
        if let Err(_e) = client.sender.send(line.clone()) {
            tracing::debug!("Failed to send to {} (connection dead)", addr);
            dead_addrs.push(addr);
        }
    }

    // Clean up dead clients
    drop(clients);
    for addr in dead_addrs {
        tracing::warn!(
            "Removing dead client {} due to send failure in broadcast_shout",
            addr
        );
        broadcaster.remove_client(addr);
    }
}

/// 공지(notice): 게임 접속 전체에 전송. 외침거부와 무관하게 Active 클라이언트 전원에게.
pub(crate) fn broadcast_notice(
    broadcaster: &crate::network::Broadcaster,
    sender_addr: SocketAddr,
    msg: &str,
) {
    use crate::network::ClientState;
    let clients = broadcaster.clients.lock();
    let mut dead_addrs = Vec::new();

    for (&addr, client) in clients.iter() {
        if client.state != ClientState::Active {
            continue;
        }
        if let Some(player) = client.player.as_ref() {
            // Python: 발신자는 sendLine(buf), 다른 사용자는
            // sendLine("\r\n" + buf) 후 lpPrompt()를 받는다.
            let mut line = if addr == sender_addr {
                format!("{}\r\n", msg)
            } else {
                format!("\r\n{}\r\n", msg)
            };
            if addr != sender_addr && !player.check_config("엘피출력") {
                line.push_str(&format!(
                    "\r\n\x1b[0;37;40m[ {}/{}, {}/{} ] ",
                    player.body.get_hp(),
                    player.body.get_max_hp(),
                    player.body.get_mp(),
                    player.body.get_max_mp()
                ));
            }
            if let Err(_e) = client.sender.send(line) {
                tracing::debug!("Failed to send to {} (connection dead)", addr);
                dead_addrs.push(addr);
            }
        }
    }

    // Clean up dead clients
    drop(clients);
    for addr in dead_addrs {
        tracing::warn!(
            "Removing dead client {} due to send failure in broadcast_notice",
            addr
        );
        broadcaster.remove_client(addr);
    }
}

/// Python Event `$순위갱신` 공지: 실행자 이외의 모든 활성 접속자에게
/// 출력만 보내며, `noPrompt=True`처럼 수신자의 프롬프트는 다시 그리지 않는다.
fn broadcast_event_lines_except(
    broadcaster: &crate::network::Broadcaster,
    actor_name: &str,
    lines: &[String],
) {
    use crate::network::ClientState;
    let message = lines.join("\r\n");
    if message.is_empty() {
        return;
    }
    let clients = broadcaster.clients.lock();
    let mut dead_addrs = Vec::new();
    for (&target_addr, client) in clients.iter() {
        if client.state != ClientState::Active
            || client
                .player
                .as_ref()
                .is_none_or(|player| player.body.get_name() == actor_name)
        {
            continue;
        }
        if client.sender.send(format!("\r\n{}\r\n", message)).is_err() {
            dead_addrs.push(target_addr);
        }
    }
    drop(clients);
    for target_addr in dead_addrs {
        broadcaster.remove_client(target_addr);
    }
}

fn save_all_active_players(broadcaster: &crate::network::Broadcaster) {
    let ordered = broadcaster.client_addresses_in_order();
    let mut clients = broadcaster.clients.lock();
    for client_addr in ordered {
        let Some(player) = clients
            .get_mut(&client_addr)
            .and_then(|client| client.player.as_mut())
        else {
            continue;
        };
        if player.state != STATE_ACTIVE {
            continue;
        }
        let path = format!("data/user/{}.json", player.body.get_name());
        let _ = save_body_to_json(&mut player.body, &path);
    }
}

fn append_guild_application(body: &mut Body, applicant: &str) -> bool {
    let existing = body.get_string("입문신청자");
    let mut names: Vec<String> = existing
        .split(['\r', '\n', ',', '|'])
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    if names.iter().any(|name| name == applicant) {
        return false;
    }
    names.push(applicant.to_string());
    body.set("입문신청자", names.join("\r\n"));
    true
}

fn clear_live_guild_members(broadcaster: &crate::network::Broadcaster, guild: &str) {
    let mut clients = broadcaster.clients.lock();
    for client in clients.values_mut() {
        let Some(player) = client.player.as_mut() else {
            continue;
        };
        if player.body.get_string("소속") != guild {
            continue;
        }
        player.body.object.attr.remove("소속");
        player.body.object.attr.remove("직위");
        let path = format!("data/user/{}.json", player.body.get_name());
        let _ = save_body_to_json(&mut player.body, &path);
    }
}

fn apply_admin_player_value(body: &mut Body, key: &str, value: serde_json::Value) {
    if value.is_null() {
        body.attr_mut().remove(key);
        return;
    }
    let value = match value {
        serde_json::Value::Number(value) if value.is_i64() => {
            crate::object::Value::Int(value.as_i64().unwrap_or_default())
        }
        serde_json::Value::Number(value) => {
            crate::object::Value::Float(value.as_f64().unwrap_or_default())
        }
        serde_json::Value::String(value) => crate::object::Value::String(value),
        _ => crate::object::Value::String(String::new()),
    };
    body.set(key, value);
}

/// 특정 접속자(이름)에게만 메시지 전송. 스크립트 send_to_user에서 수집된 목록 처리용.
pub(crate) fn send_to_one_user(broadcaster: &crate::network::Broadcaster, name: &str, msg: &str) {
    use crate::network::ClientState;
    let Some(addr) = broadcaster.find_addr_by_player_name(name) else {
        return;
    };
    let clients = broadcaster.clients.lock();
    let line = format!("\r\n{}\r\n", msg);
    let send_failed = clients.get(&addr).is_some_and(|client| {
        client.state == ClientState::Active
            && client
                .player
                .as_ref()
                .is_some_and(|player| player.body.get_string("이름") == name)
            && client.sender.send(line).is_err()
    });
    drop(clients);
    if send_failed {
        tracing::warn!(
            "Removing dead client {} due to send failure in send_to_one_user",
            addr
        );
        broadcaster.remove_client(addr);
    }
}

pub(crate) fn send_collected_user_message(
    broadcaster: &crate::network::Broadcaster,
    name: &str,
    message: &str,
) {
    let (payload, raw) = collected_user_payload(message);
    if raw {
        let Some(addr) = broadcaster.find_addr_by_player_name(name) else {
            return;
        };
        if broadcaster.send_to(addr, payload).is_err() {
            broadcaster.remove_client(addr);
        }
    } else {
        send_to_one_user(broadcaster, name, payload);
    }
}

fn collected_user_payload(message: &str) -> (&str, bool) {
    message
        .strip_prefix(crate::script::RAW_USER_MESSAGE_PREFIX)
        .map_or((message, false), |payload| (payload, true))
}

fn summon_observer_payload(message: &str, body: &Body, interactive: i32) -> String {
    // Python writeRoom('\r\n' + message): sendLine appends CRLF, then
    // lpPrompt() writes another CRLF before the vitals prompt.
    let mut payload = format!("\r\n{message}\r\n");
    if interactive == 1
        && !crate::script::config_is_enabled(&body.get_string("설정상태"), "엘피출력")
    {
        payload.push_str(&format!(
            "\r\n\x1b[0;37;40m[ {}/{}, {}/{} ] ",
            body.get_hp(),
            body.get_max_hp(),
            body.get_mp(),
            body.get_max_mp()
        ));
    }
    payload
}

/// Python `exitRoom/enterRoom(..., "소환")` uses `writeRoom`, so every
/// observer receives the message and its own LP prompt.  Event `$위치이동`
/// takes the same path even though its state transition originates in the
/// event engine rather than the regular movement command.
fn send_event_summon_observers(
    broadcaster: &crate::network::Broadcaster,
    zone: &str,
    room: &str,
    actor_name: &str,
    message: &str,
) {
    let names = get_world_state()
        .read()
        .unwrap()
        .get_players_in_room(zone, room);
    let bindings = broadcaster.player_bindings_for_names(&names);
    let clients = broadcaster.clients.lock();
    let mut dead_addrs = Vec::new();
    for (name, addr) in bindings {
        if name == actor_name {
            continue;
        }
        let Some(client) = clients.get(&addr) else {
            continue;
        };
        let Some(player) = client.player.as_ref() else {
            continue;
        };
        if player.body.get_name() != name {
            continue;
        }
        let payload = summon_observer_payload(message, &player.body, player.interactive);
        if client.sender.send(payload).is_err() {
            dead_addrs.push(addr);
        }
    }
    drop(clients);
    for addr in dead_addrs {
        broadcaster.remove_client(addr);
    }
}

/// Rhai가 완성한 전음 수신 문자열을 opaque 접속 토큰의 사용자에게 그대로
/// 전달하고 Python의 `_talker`/`talkHistory` 런타임 상태를 갱신한다.
fn apply_tell_delivery(
    broadcaster: &crate::network::Broadcaster,
    target_token: &str,
    sender_token: &str,
    recipient_output: &str,
    history_line: &str,
) -> bool {
    let mut target_addr = None;
    {
        let mut clients = broadcaster.clients.lock();
        for (&candidate_addr, client) in clients.iter_mut() {
            if client.connection_token != target_token {
                continue;
            }
            let Some(target) = client.player_mut() else {
                break;
            };
            target.body.temp_mut().insert(
                crate::script::TELL_TALKER_TOKEN.to_string(),
                crate::object::Value::String(sender_token.to_string()),
            );
            target.body.talk_history.push(history_line.to_string());
            if target.body.talk_history.len() > 22 {
                target.body.talk_history.remove(0);
            }
            target_addr = Some(candidate_addr);
            break;
        }
    }
    let Some(target_addr) = target_addr else {
        return false;
    };
    broadcaster.send_to(target_addr, recipient_output).is_ok()
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
                    w.spawn_mobs_for_room(&start_pos.zone, &start_pos.room);
                }
                start_pos
            }
        }
    };

    // 방이 캐시에 없을 수 있으므로 get_room으로 로드 보장 (이동 후 복귀 시 등)
    {
        let mut w = get_world_state().write().unwrap();
        let _ = w.room_cache.get_room(&pos.zone, &pos.room);
    }

    let world = get_world_state().read().unwrap();

    if let Some(room) = world.room_cache.get_room_cached(&pos.zone, &pos.room) {
        let room_ref = room
            .read()
            .map_err(|e| format!("Room read lock error: {}", e))?;

        let room_name_formatted = format_room_header(&room_ref.display_name);
        let exits_str = format_exits_long(&room_ref);
        let fixture_lines = visible_fixture_short_lines(&world, &pos.zone, &pos.room);

        let mobs = world.mob_cache.get_mobs_in_room(&pos.zone, &pos.room);
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
        let room_objs = world.get_room_objs(&pos.zone, &pos.room);
        let room_stack = world.get_room_objs_stack(&pos.zone, &pos.room);
        let item_str = build_room_objs_grouped(&room_objs, &room_stack);

        // 같은 방의 다른 접속 유저. 파이썬 viewMapData: for obj in room.objs: is_player then getDesc.
        let other_descs =
            get_other_players_desc_in_room(broadcaster.as_ref(), &pos.zone, &pos.room, player_name);
        let other_str = if other_descs.is_empty() {
            String::new()
        } else {
            format!("\r\n{}", other_descs.join("\r\n"))
        };

        // 파이썬 viewMapData: 헤더 \r\n\r\n 설명 \r\n\r\n 출구 \r\n [몹] \r\n [바닥 아이템] \r\n [다른 유저]
        broadcaster.send_to(addr, "\r\n")?;
        broadcaster.send_to(addr, &room_name_formatted)?;
        broadcaster.send_to(addr, "\r\n\r\n")?;
        for description_line in &room_ref.description {
            broadcaster.send_to(addr, description_line)?;
            broadcaster.send_to(addr, "\r\n")?;
        }
        for fixture_line in fixture_lines {
            broadcaster.send_to(addr, &fixture_line)?;
            broadcaster.send_to(addr, "\r\n")?;
        }
        broadcaster.send_to(addr, "\r\n")?;
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
        broadcaster.send_to(
            addr,
            &format!(
                "\x1b[1;37m[{}:{}]\x1b[0;37m\r\n알 수 없는 곳입니다.\r\n",
                pos.zone, pos.room
            ),
        )?;
    }

    Ok(())
}

/// Python `Player.viewMapData()` appends the current `zone:room` only for
/// administrators.  Event-driven movement renders a room through
/// `build_room_lines()` after the command result, so keep that viewer-specific
/// suffix at the network boundary rather than baking it into the shared view.
fn append_admin_room_position(view: &mut String, zone: &str, room: &str, is_admin: bool) {
    if !is_admin {
        return;
    }
    if let Some(header_end) = view.get(2..).and_then(|tail| tail.find("\r\n")) {
        view.insert_str(header_end + 2, &format!(" ({zone}:{room})"));
    }
}

/// `$위치이동` calls `Player.lpPrompt()` after `enterRoom()` returns; the
/// outer `do_command()` then emits its normal prompt as well.  Keep that
/// first, event-local prompt separate so it still obeys Python's interactive
/// and `엘피출력` gates.
fn event_move_lp_prompt(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
) -> String {
    let clients = broadcaster.clients.lock();
    let Some(player) = clients.get(&addr).and_then(|client| client.player.as_ref()) else {
        return String::new();
    };
    if player.interactive != 1
        || crate::script::config_is_enabled(&player.body.get_string("설정상태"), "엘피출력")
    {
        return String::new();
    }
    format!(
        "\r\n\r\n\x1b[0;37;40m[ {}/{}, {}/{} ] ",
        player.body.get_hp(),
        player.body.get_max_hp(),
        player.body.get_mp(),
        player.body.get_max_mp()
    )
}

/// 암호변경 다단계 입력: 이전암호 → 새암호 → 확인. (명령줄에 암호 넣지 않음)
async fn handle_pending_change_password_with_registry(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    input: &str,
    command_registry: Option<&CommandRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let input = input.trim_end_matches('\n').trim_end_matches('\r');
    let mut room_append: Option<(String, String, String)> = None;
    let mut event_position_transition = false;
    let mut resumed_event_room_broadcast: Option<(String, String, String, Vec<String>)> = None;
    let mut resumed_event_broadcast_lines: Vec<String> = Vec::new();
    let mut resumed_event_summon_observers: Vec<(String, String, String, String)> = Vec::new();
    let mut suppress_done_prompt = false;
    let (next_state, mut msg, done) = {
        let mut clients = broadcaster.clients.lock();
        // Python `channel.players`는 활성 상태와 관계없이 현재 연결된
        // Player 객체를 모두 봤다. 쪽지 종료 시의 재접속 검사에 쓴다.
        let connected_player_names: Vec<String> = clients
            .values()
            .filter_map(|connected| connected.player.as_ref())
            .map(|player| player.body.get_name())
            .filter(|name| !name.is_empty())
            .collect();
        let client = match clients.get_mut(&addr) {
            Some(c) => c,
            None => return Ok(()),
        };
        let pending = match client.pending_input.take() {
            Some(p) => p,
            None => return Ok(()),
        };
        let player = match client.player.as_mut() {
            Some(p) => p,
            None => {
                let _ = broadcaster.send_to(addr, "\x1b[1;31m☞ 오류가 발생했어요.\x1b[0;37m\r\n");
                return Ok(());
            }
        };
        let player_interactive = player.interactive;
        let body = &mut player.body;
        let stored = body.get_string("암호");
        match pending {
            PendingInput::ChangePasswordOld { text } => {
                suppress_done_prompt = true;
                if !password_verify(&stored, input.trim()) {
                    (None, text.wrong_password, true)
                } else {
                    (
                        Some(PendingInput::ChangePasswordNew { text: text.clone() }),
                        text.new_password_prompt,
                        false,
                    )
                }
            }
            PendingInput::ChangePasswordNew { text } => {
                suppress_done_prompt = true;
                (
                    Some(PendingInput::ChangePasswordConfirm {
                        new_password: input.to_string(),
                        text: text.clone(),
                    }),
                    text.confirm_prompt,
                    false,
                )
            }
            PendingInput::ChangePasswordConfirm { new_password, text } => {
                suppress_done_prompt = true;
                if input != new_password {
                    (None, text.mismatch, true)
                } else {
                    // Python은 새 값을 그대로 Body 속성에 반영하고,
                    // 일반 저장/로그아웃 흐름에서 파일에 기록한다.
                    body.object.attr.insert(
                        "암호".to_string(),
                        crate::object::Value::String(password_hash(input)),
                    );
                    (None, text.success, true)
                }
            }
            PendingInput::EventEnter {
                mob_key,
                event_key,
                words,
                line_num,
                resume_func,
            } => {
                let (zone, room) = get_world_state()
                    .read()
                    .unwrap()
                    .get_player_position(&body.get_name())
                    .map(|p| (p.zone.clone(), p.room.clone()))
                    .unwrap_or((String::new(), "0".to_string()));
                let result = crate::world::event::try_mob_event_resume(
                    body,
                    &zone,
                    &room,
                    &mob_key,
                    &event_key,
                    words,
                    line_num,
                    resume_func,
                );
                let event_death_finish_output = command_registry
                    .map(|registry| finish_lethal_event_rhai(registry, body))
                    .unwrap_or_default();
                match result {
                    Some(CommandResult::MobEventEnter {
                        output_lines,
                        set_position,
                        broadcast_lines,
                        room_broadcast_lines,
                        mob_key,
                        event_key,
                        words,
                        line_num,
                        prompt,
                        resume_func,
                        ..
                    }) => {
                        // A resumed Python `$엔터$` sequence may hit another
                        // `$엔터$`.  Keep the next callback exactly as the
                        // initial event path does; otherwise the second
                        // pause is discarded and the next user input falls
                        // through to normal command parsing.
                        if !room_broadcast_lines.is_empty() {
                            resumed_event_room_broadcast = Some((
                                zone.clone(),
                                room.clone(),
                                body.get_name().to_string(),
                                room_broadcast_lines,
                            ));
                        }
                        resumed_event_broadcast_lines = broadcast_lines;
                        let (mut out, mut ends_with_event_lp_prompt) =
                            render_event_output_lines(&output_lines, body, player_interactive);
                        if !event_death_finish_output.is_empty() {
                            if !out.is_empty() && !ends_with_event_lp_prompt {
                                out.push_str("\r\n");
                            }
                            out.push_str(&event_death_finish_output);
                            ends_with_event_lp_prompt = false;
                        }
                        if let Some((z, r)) = set_position {
                            let allowed =
                                crate::script::check_event_summon_destination(body, &z, &r);
                            let mut w = get_world_state().write().unwrap();
                            if (allowed.is_empty() || allowed == "same_place")
                                && w.room_cache.get_room(&z, &r).is_ok()
                            {
                                drop(w);
                                crate::script::clear_summon_combat(body);
                                let mut w = get_world_state().write().unwrap();
                                let pname = body.get_name().to_string();
                                let old_position = w.get_player_position(&pname).cloned();
                                append_event_summon_departure(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    body,
                                );
                                w.set_player_position(
                                    &pname,
                                    PlayerPosition::new(z.clone(), r.clone()),
                                );
                                w.spawn_mobs_for_room(&z, &r);
                                room_append = Some((z, r, pname));
                                event_position_transition = true;
                                if body.get_int("투명상태") != 1 {
                                    let actor = format!(
                                        "\x1b[1m{}\x1b[0;37m{}",
                                        body.get_name(),
                                        crate::hangul::han_iga(&body.get_name())
                                    );
                                    if let Some(old) = old_position {
                                        resumed_event_summon_observers.push((
                                            old.zone,
                                            old.room,
                                            body.get_name().to_string(),
                                            format!("{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'", actor),
                                        ));
                                    }
                                    let current = room_append.as_ref().unwrap();
                                    resumed_event_summon_observers.push((
                                        current.0.clone(),
                                        current.1.clone(),
                                        body.get_name().to_string(),
                                        format!(
                                            "{} 알수 없는 기운에 감싸여 나타납니다. '고오오오~~~'",
                                            actor
                                        ),
                                    ));
                                }
                            } else if allowed == "fail" {
                                append_event_move_failure(&mut out, &mut ends_with_event_lp_prompt);
                            } else {
                                append_event_summon_rejection(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    &allowed,
                                );
                            }
                        }
                        if !out.is_empty() && !ends_with_event_lp_prompt {
                            out.push_str("\r\n");
                        }
                        out.push_str(&prompt);
                        out.push_str("\r\n");
                        (
                            Some(PendingInput::EventEnter {
                                mob_key,
                                event_key,
                                words,
                                line_num,
                                resume_func,
                            }),
                            out,
                            false,
                        )
                    }
                    Some(CommandResult::MobEvent {
                        output_lines,
                        set_position,
                        broadcast_lines,
                        room_broadcast_lines,
                        ..
                    }) => {
                        if !room_broadcast_lines.is_empty() {
                            resumed_event_room_broadcast = Some((
                                zone.clone(),
                                room.clone(),
                                body.get_name().to_string(),
                                room_broadcast_lines,
                            ));
                        }
                        resumed_event_broadcast_lines = broadcast_lines;
                        let (mut out, mut ends_with_event_lp_prompt) =
                            render_event_output_lines(&output_lines, body, player_interactive);
                        if !event_death_finish_output.is_empty() {
                            if !out.is_empty() && !ends_with_event_lp_prompt {
                                out.push_str("\r\n");
                            }
                            out.push_str(&event_death_finish_output);
                        }
                        if let Some((z, r)) = set_position {
                            let allowed =
                                crate::script::check_event_summon_destination(body, &z, &r);
                            let mut w = get_world_state().write().unwrap();
                            if (allowed.is_empty() || allowed == "same_place")
                                && w.room_cache.get_room(&z, &r).is_ok()
                            {
                                drop(w);
                                crate::script::clear_summon_combat(body);
                                let mut w = get_world_state().write().unwrap();
                                let pname = body.get_name().to_string();
                                let old_position = w.get_player_position(&pname).cloned();
                                append_event_summon_departure(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    body,
                                );
                                w.set_player_position(
                                    &pname,
                                    PlayerPosition::new(z.clone(), r.clone()),
                                );
                                w.spawn_mobs_for_room(&z, &r);
                                room_append = Some((z, r, pname));
                                event_position_transition = true;
                                if body.get_int("투명상태") != 1 {
                                    let actor = format!(
                                        "\x1b[1m{}\x1b[0;37m{}",
                                        body.get_name(),
                                        crate::hangul::han_iga(&body.get_name())
                                    );
                                    if let Some(old) = old_position {
                                        resumed_event_summon_observers.push((
                                            old.zone,
                                            old.room,
                                            body.get_name().to_string(),
                                            format!("{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'", actor),
                                        ));
                                    }
                                    let current = room_append.as_ref().unwrap();
                                    resumed_event_summon_observers.push((
                                        current.0.clone(),
                                        current.1.clone(),
                                        body.get_name().to_string(),
                                        format!(
                                            "{} 알수 없는 기운에 감싸여 나타납니다. '고오오오~~~'",
                                            actor
                                        ),
                                    ));
                                }
                            } else if allowed == "fail" {
                                append_event_move_failure(&mut out, &mut ends_with_event_lp_prompt);
                            } else {
                                append_event_summon_rejection(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    &allowed,
                                );
                            }
                        }
                        if !out.is_empty() && !ends_with_event_lp_prompt {
                            out.push_str("\r\n");
                        }
                        (None, out, true)
                    }
                    _ => (None, "\r\n".to_string(), true),
                }
            }
            PendingInput::Script {
                name,
                lines,
                line_num,
                temp_input,
                from_confirm,
                script_ob,
                script_resume_op,
            } => {
                if from_confirm && input.eq_ignore_ascii_case("취소") {
                    body.script_temp_item = None;
                    (None, "* 무기강화를 종료합니다.\r\n".to_string(), true)
                } else {
                    let (out_lines, next) = if script_resume_op.is_some() {
                        run_script_chunk_rhai(
                            body,
                            &name,
                            Some(input.to_string()),
                            temp_input.clone(),
                            script_ob.clone(),
                            script_resume_op.clone(),
                        )
                    } else {
                        let input_opt = if from_confirm {
                            None
                        } else {
                            Some(input.to_string())
                        };
                        run_script_chunk(body, &lines, line_num, input_opt, temp_input.clone())
                    };
                    let mut out = out_lines.join("\r\n");
                    if !out.is_empty() {
                        out.push_str("\r\n");
                    }
                    match next {
                        ScriptNext::Complete => {
                            body.script_temp_item = None;
                            (None, out, true)
                        }
                        ScriptNext::Wait {
                            line_num: ln,
                            prompt,
                            persist_temp,
                            from_confirm: fc,
                            script_ob: so,
                            script_resume_op: sro,
                        } => {
                            let next_temp = persist_temp.or(temp_input);
                            let next_state = Some(PendingInput::Script {
                                name,
                                lines,
                                line_num: ln,
                                temp_input: next_temp,
                                from_confirm: fc,
                                script_ob: so,
                                script_resume_op: sro,
                            });
                            out.push_str(&prompt);
                            out.push_str("\r\n");
                            (next_state, out, false)
                        }
                    }
                }
            }
            PendingInput::NoteEdit {
                recipient,
                body: mut memo_body,
                text,
            } => match advance_note_body(&mut memo_body, input) {
                NoteEditAdvance::Continue => {
                    let prompt = text.continue_prompt.clone();
                    let next = Some(PendingInput::NoteEdit {
                        recipient,
                        body: memo_body,
                        text,
                    });
                    (next, prompt, false)
                }
                NoteEditAdvance::Complete { capacity_exceeded } => {
                    let target_connected = connected_player_names
                        .iter()
                        .any(|name| name == &recipient.target_name);
                    let message = if target_connected {
                        text.target_connected
                    } else {
                        finish_note(&recipient, body.get_name().as_str(), &memo_body);
                        let mut message = String::new();
                        if capacity_exceeded {
                            message.push_str(&text.capacity_exceeded);
                        }
                        message.push_str(&text.complete);
                        message
                    };
                    (None, message, true)
                }
            },
            PendingInput::RoomDescription {
                zone,
                room,
                mut lines,
            } => {
                if input == "." {
                    let description = lines.join("\r\n");
                    if let Ok(mut world) = get_world_state().write() {
                        world
                            .get_room_attrs_mut(&zone, &room)
                            .insert("설명".to_string(), description.clone());
                        if let Some(cached) = world.room_cache.get_room_cached(&zone, &room) {
                            if let Ok(mut cached) = cached.write() {
                                cached.description = lines.clone();
                            }
                        }
                    }
                    let map_path = format!("data/map/{}/{}.json", zone, room);
                    if let Ok(raw) = std::fs::read_to_string(&map_path) {
                        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&raw) {
                            if let Some(info) =
                                json.get_mut("맵정보").and_then(|v| v.as_object_mut())
                            {
                                info.insert(
                                    "설명".to_string(),
                                    // Python write_lines assigns the joined
                                    // _lineData string directly to Room['설명'].
                                    serde_json::Value::String(description.clone()),
                                );
                                if let Ok(saved) = serde_json::to_string_pretty(&json) {
                                    let _ = std::fs::write(&map_path, saved);
                                }
                            }
                        }
                    }
                    (None, "작성을 마칩니다.\r\n".to_string(), true)
                } else {
                    let line = if input.is_empty() { " " } else { input };
                    lines.push(line.to_string());
                    (
                        Some(PendingInput::RoomDescription { zone, room, lines }),
                        format!("{}\r\n:", line),
                        false,
                    )
                }
            }
            PendingInput::FileEdit {
                relative_path,
                mut lines,
            } => {
                if input == "." {
                    let path = format!("data/{}", relative_path);
                    if std::fs::write(&path, lines.join("\n")).is_ok() {
                        (None, "작성을 마칩니다.\r\n".to_string(), true)
                    } else {
                        // Python `write_edit` silently returns False and keeps
                        // the same input callback when open() fails.
                        (
                            Some(PendingInput::FileEdit {
                                relative_path,
                                lines,
                            }),
                            String::new(),
                            false,
                        )
                    }
                } else {
                    let echoed = input.to_string();
                    // `_lineData == ''` is also true after one or more empty
                    // inputs, so Python replaces those with the next line.
                    if lines.join("\n").is_empty() {
                        lines.clear();
                    }
                    lines.push(echoed.clone());
                    (
                        Some(PendingInput::FileEdit {
                            relative_path,
                            lines,
                        }),
                        format!("{}\r\n:", echoed),
                        false,
                    )
                }
            }
        }
    };
    if let Some((z, r, pname)) = room_append {
        let others = get_other_players_desc_in_room(broadcaster.as_ref(), &z, &r, &pname);
        if let Ok(mut room_str) = build_room_lines(&pname, &others) {
            let is_admin = broadcaster
                .clients
                .lock()
                .get(&addr)
                .and_then(|client| client.player.as_ref())
                .is_some_and(|player| player.body.get_int("관리자등급") >= 1_000);
            append_admin_room_position(&mut room_str, &z, &r, is_admin);
            if !event_position_transition {
                msg.push_str("\r\n");
            }
            msg.push_str(&room_str);
            if event_position_transition {
                msg.push_str(&event_move_lp_prompt(broadcaster, addr));
            }
        }
    }
    if let Some(s) = next_state {
        let mut clients = broadcaster.clients.lock();
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_input = Some(s);
        }
    }
    broadcaster.send_to(addr, &msg)?;
    for (zone, room, actor_name, message) in resumed_event_summon_observers {
        send_event_summon_observers(broadcaster, &zone, &room, &actor_name, &message);
    }
    if !resumed_event_broadcast_lines.is_empty() {
        let actor_name = broadcaster
            .clients
            .lock()
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            .map(|player| player.body.get_name().to_string())
            .unwrap_or_default();
        broadcast_event_lines_except(broadcaster, &actor_name, &resumed_event_broadcast_lines);
    }
    if let Some((zone, room, actor_name, lines)) = resumed_event_room_broadcast {
        let message = lines.join("\r\n");
        if !message.is_empty() {
            let names = get_world_state()
                .read()
                .unwrap()
                .get_players_in_room(&zone, &room);
            let bindings = broadcaster.player_bindings_for_names(&names);
            let deliveries = {
                let clients = broadcaster.clients.lock();
                bindings
                    .into_iter()
                    .filter_map(|(name, observer_addr)| {
                        (name != actor_name)
                            .then(|| clients.get(&observer_addr))
                            .flatten()
                            .and_then(|client| client.player.as_ref())
                            .map(|player| {
                                (
                                    observer_addr,
                                    summon_observer_payload(
                                        &message,
                                        &player.body,
                                        player.interactive,
                                    ),
                                )
                            })
                    })
                    .collect::<Vec<_>>()
            };
            for (observer_addr, payload) in deliveries {
                broadcaster.send_to(observer_addr, &payload)?;
            }
        }
    }
    if done && !suppress_done_prompt {
        send_game_prompt(broadcaster, addr).await?;
    }
    Ok(())
}

/// Compatibility entry point used by focused pending-input tests. Production
/// command paths pass their registry through the private variant so a resumed
/// lethal event can finish the same Rhai death sequence immediately.
#[cfg(test)]
pub(crate) async fn handle_pending_change_password(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    handle_pending_change_password_with_registry(broadcaster, addr, input, None).await
}

/// Python `Player.parse_command`의 사용자 줄임말 한 번 확장 결과.
/// 첫 명령은 현재 호출에서 실행하고, 나머지는 세미콜론 순서대로 후속 입력이 된다.
fn expand_user_alias(line: &str, aliases: &HashMap<String, String>) -> Option<Vec<String>> {
    // Surrounding whitespace is ignored before tokenization. Sentence-ending
    // punctuation still selects the implicit speech form.
    let without_newline = line.trim_end_matches('\n').trim_end_matches('\r').trim();
    if without_newline.ends_with('.')
        || without_newline.ends_with('!')
        || without_newline.ends_with('?')
    {
        return None;
    }

    let line = line.trim();
    let words: Vec<&str> = line.split_whitespace().collect();
    let command = words.last().copied()?;
    let shortcut = aliases.get(command)?;
    let argument = if words.len() > 1 {
        let command_start = line.rfind(command).unwrap_or(line.len());
        Some(line[..command_start].trim())
    } else {
        None
    };

    Some(
        shortcut
            .split(';')
            .map(|part| match argument {
                Some(value) => part.replace('*', value),
                None => part.to_string(),
            })
            .collect(),
    )
}

/// 사용자 줄임말의 첫 명령은 Python처럼 말하기/`!` 전처리를 다시 거치지 않는다.
fn parse_expanded_user_alias(line: &str) -> ParsedCommand {
    let line = line.trim_end_matches('\n').trim_end_matches('\r').trim();
    let words: Vec<&str> = line.split_whitespace().collect();
    let Some(command) = words.last().copied() else {
        return ParsedCommand::empty();
    };
    let command_start = line.rfind(command).unwrap_or(0);
    let args = line[..command_start].trim().to_string();
    let resolved = CommandParser::resolve_alias(command);
    let is_direction = matches!(
        resolved.as_str(),
        "동" | "서" | "남" | "북" | "위" | "아래" | "북동" | "북서" | "남동" | "남서"
    );
    let pickup_keywords = ["주워", "집어", "집", "가져"];
    let tokens = if args.is_empty() {
        Vec::new()
    } else if is_direction {
        args.split_whitespace()
            .filter(|word| !pickup_keywords.contains(word))
            .map(str::to_string)
            .collect()
    } else {
        args.split_whitespace().map(str::to_string).collect()
    };
    ParsedCommand {
        raw: line.to_string(),
        command: resolved,
        args,
        tokens,
    }
}

/// Python's direction/exit branch is entered only for a one-word line.
/// A final direction token in `맵 동` is therefore an unknown command, not a
/// movement command and not a first-token command fallback.
fn direction_with_arguments_is_not_a_command(command: &str, word_count: usize) -> bool {
    word_count > 1
        && matches!(
            command,
            "북" | "남" | "동" | "서" | "위" | "아래" | "북서" | "북동" | "남서" | "남동"
        )
}

/// Python `Player.parse_command` 의 `line.rstrip(cmd).strip()` 결과.
/// `str.rstrip(chars)`는 접미 문자열이 아니라 `cmd`에 포함된
/// 문자 집합을 끝에서 제거한다.
fn python_command_parameter(line: &str, cmd: &str) -> String {
    line.trim_end_matches(|character| cmd.contains(character))
        .trim()
        .to_string()
}

fn python_auto_move_route(route: &str) -> Vec<String> {
    if route.is_empty() {
        Vec::new()
    } else {
        route.split(';').map(str::to_string).collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GlobalPlayerSnapshotNeeds {
    details: bool,
    online_names: bool,
    connected_names: bool,
    tell_players: bool,
}

fn global_player_snapshot_needs(resolved_command: &str) -> GlobalPlayerSnapshotNeeds {
    let requested = |name: &str| resolved_command == name;
    GlobalPlayerSnapshotNeeds {
        details: [
            "누구",
            "어디",
            "방파상태",
            "소켓",
            "정리",
            "순위",
            "비교",
            "트윗",
            "외쳐",
            "외쳐2",
            "직위임명",
            "방파말",
            "똥파말",
            "방파별호",
            "방파파문",
            "방주권한양도",
            "명칭설정",
            "모두소환",
            "무림별호",
        ]
        .iter()
        .any(|name| requested(name)),
        online_names: ["외쳐", "외쳐2"].iter().any(|name| requested(name)),
        connected_names: ["쪽지", "기연정리", "기연정리리", "정리"]
            .iter()
            .any(|name| requested(name)),
        tell_players: ["전음", "반전음"].iter().any(|name| requested(name)),
    }
}

fn global_snapshot_includes_transparent(resolved_command: &str) -> bool {
    let requested = |name: &str| resolved_command == name;
    [
        "트윗",
        "외쳐",
        "외쳐2",
        "모두소환",
        "방파말",
        "똥파말",
        "방파별호",
        "방파파문",
        "방주권한양도",
        "직위임명",
        "명칭설정",
        "도망",
        "귀환",
        // Python 순위 scans channel.players without a transparency guard.
        "순위",
        "무림별호",
    ]
    .iter()
    .any(|command| requested(command))
}

/// Handle one network input, including Python-compatible repeat/user-alias expansion.
pub(crate) async fn handle_game_command(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    command: &str,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
    shutdown_notify: Option<Arc<tokio::sync::Notify>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = VecDeque::from([command.to_string()]);
    while let Some(input) = pending.pop_front() {
        let has_pending_input = {
            let clients = broadcaster.clients.lock();
            clients
                .get(&addr)
                .and_then(|client| client.pending_input.as_ref())
                .is_some()
        };
        if has_pending_input {
            // Python은 `;` 후속 줄을 channel._buffer 앞에 넣으므로, input_to가
            // 바뀌었다면 다음 줄은 일반 명령이 아니라 그 대기 입력으로 소비된다.
            handle_pending_change_password_with_registry(
                broadcaster,
                addr,
                &input,
                Some(command_registry.as_ref()),
            )
            .await?;
            continue;
        }

        let rate_limited = {
            let mut clients = broadcaster.clients.lock();
            let Some(player) = clients
                .get_mut(&addr)
                .and_then(|client| client.player_mut())
            else {
                return Ok(());
            };
            if player.body.get_int("관리자등급") < 2_000 {
                player.cmd_cnt = player.cmd_cnt.saturating_add(1);
            }
            player.body.get_int("관리자등급") < 2_000
                && i64::from(player.cmd_cnt) > crate::script::get_murim_config_int("입력초과경고수")
        };
        if rate_limited {
            broadcaster.send_to(addr, "^^;\r\n")?;
            send_game_prompt(broadcaster, addr).await?;
            continue;
        }

        // Python Player.parse_command strips its ANSI/backspace form before
        // testing empty input, `!`, prevCmd, say syntax and user aliases.
        // Pending input above intentionally receives the original bytes, and
        // an already-expanded first alias command is not routed back here.
        let input = CommandParser::strip_python_ansi(&input);
        if input.is_empty() {
            // An ANSI-only line does not replace prevCmd. The network command
            // loop still emits the normal in-game prompt for the consumed line.
            send_game_prompt(broadcaster, addr).await?;
            continue;
        }

        let (line, aliases) = {
            let mut clients = broadcaster.clients.lock();
            let Some(player) = clients
                .get_mut(&addr)
                .and_then(|client| client.player_mut())
            else {
                return Ok(());
            };
            let line = if input == "!" {
                player.prev_cmd.clone()
            } else {
                player.prev_cmd = input.clone();
                input
            };
            (line, player.alias.clone())
        };

        if let Some(mut expanded) = expand_user_alias(&line, &aliases) {
            if expanded.is_empty() {
                continue;
            }
            let first = expanded.remove(0);
            handle_single_game_command(
                broadcaster,
                addr,
                &first,
                true,
                command_registry.clone(),
                room_cache.clone(),
                shutdown_notify.clone(),
            )
            .await?;
            for followup in expanded.into_iter().rev() {
                pending.push_front(followup);
            }
        } else {
            handle_single_game_command(
                broadcaster,
                addr,
                &line,
                false,
                command_registry.clone(),
                room_cache.clone(),
                shutdown_notify.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

/// Execute one already-expanded game command.
async fn handle_single_game_command(
    broadcaster: &Arc<crate::network::Broadcaster>,
    addr: SocketAddr,
    command: &str,
    expanded_user_alias: bool,
    command_registry: Arc<CommandRegistry>,
    room_cache: Arc<std::sync::Mutex<RoomCache>>,
    shutdown_notify: Option<Arc<tokio::sync::Notify>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if command.is_empty() {
        send_prompt_raw(broadcaster, addr, ">> ").await?;
        return Ok(());
    }

    debug!(
        "Game command received from {} ({} bytes)",
        addr,
        command.len()
    );
    // Parse the command
    let parsed = if expanded_user_alias {
        parse_expanded_user_alias(command)
    } else {
        CommandParser::parse(command)
    };
    let raw_command_token = parsed.raw.split_whitespace().last().unwrap_or("");
    let python_raw_parameter = python_command_parameter(&parsed.raw, raw_command_token);
    let python_say_syntax = CommandParser::is_say_command(&parsed.raw);

    // Handle empty input
    if parsed.is_empty() {
        send_game_prompt(broadcaster, addr).await?;
        return Ok(());
    }

    // Get the player
    let actor_name = {
        let clients = broadcaster.clients.lock();
        clients
            .get(&addr)
            .map(|c| c.player_name())
            .unwrap_or_else(|| "방문자".to_string())
    };
    // Lock released

    // 봐/보/look: 봐.rhai 스크립트로 처리 (registry 통해 호출).

    // Python resolves aliases before its one-word exit branch.  The actual
    // branch is a private hot-reloaded registry handler invoked below after
    // room command limits and mob events.
    let move_cmd = command_registry.resolve_alias(parsed.command.as_str());
    let command_requested = |name: &str| move_cmd == name;
    let inventory_command_requested = command_requested("소지품");
    let mugong_command_requested = [
        "무공",
        "무공상태",
        "무공전수",
        "무공전수2",
        "무공제거",
        "성올려",
    ]
    .iter()
    .any(|name| command_requested(name));
    let cast_command_requested = command_requested("시전");
    let single_word_command = parsed.raw.split_whitespace().count() == 1;
    let movement_observers_requested = single_word_command
        && (crate::command::DIRECTIONS
            .iter()
            .any(|(korean, english, _)| parsed.command == *korean || parsed.command == *english)
            || crate::command::DIRECTION_ALIASES
                .iter()
                .any(|(alias, _)| parsed.command == *alias));
    let room_combat_context_requested = cast_command_requested
        || command_requested("쳐")
        || command_requested("도망")
        || command_requested("말")
        || command_requested("먹어")
        || command_requested("구입")
        || command_requested("판매");
    let global_snapshot_needs = global_player_snapshot_needs(&move_cmd);
    let all_online_details_requested =
        global_snapshot_needs.details || movement_observers_requested;
    let live_rank_attribute = if command_requested("순위") {
        let requested = python_command_parameter(&parsed.raw, &move_cmd);
        Some(match requested.as_str() {
            "내공" => "최고내공".to_string(),
            "체력" => "최고체력".to_string(),
            "민첩" => "민첩성".to_string(),
            _ => requested,
        })
    } else {
        None
    };
    // Python `어디` is asymmetric: a named lookup requires ACTIVE, while the
    // no-argument same-zone listing does not check Player.state at all.
    // Preserve inactive Player entries only for this command and let Rhai
    // apply the branch-specific rule.
    let global_details_include_inactive =
        command_requested("어디") || command_requested("순위") || command_requested("비교");
    let socket_details_requested = command_requested("소켓");
    let online_names_requested = global_snapshot_needs.online_names;
    let connected_names_requested = global_snapshot_needs.connected_names;
    let tell_players_requested = global_snapshot_needs.tell_players;
    let adult_channel_requested = ["채널입장", "채널퇴장", "채널잡담", "채널누구"]
        .iter()
        .any(|name| command_requested(name));
    // 방파입문 also needs the same scoped room-player vitals/config
    // snapshot to reproduce target.lpPrompt() without scanning all clients.
    let party_command_requested = [
        "따라",
        "무리",
        "무리제외",
        "무리말",
        "방파입문",
        "부셔",
        "소각",
        "분해",
        "세트착용",
        "쉬어",
        "일어나",
        "입문신청",
        "입어",
    ]
    .iter()
    .any(|name| command_requested(name));
    let box_command_requested = ["넣어", "꺼내"].iter().any(|name| command_requested(name));
    // Unknown command - try command registry
    let mut response = String::new();
    let mut say_to_room: Option<(String, String, String, String)> = None;
    let mut emotion_to_room: Option<(String, String, String, String, Option<(String, String)>)> =
        None; // (pname, zone, room, to_room, to_target)
    let mut give_pending: Option<(
        std::net::SocketAddr,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        Option<(String, usize, usize)>,
        Option<(String, i64)>,
        bool,
        bool,
    )> = None; // (giver_addr, zone, room, target_name, giver_name, give_silver, give_gold, give_item, give_item_stack)
    let mut shout_to_broadcast: Option<String> = None;
    let mut notice_to_broadcast: Option<String> = None;
    let mut send_to_users: Option<Vec<(String, String)>> = None; // 스크립트 send_to_user 수집분
    let mut broadcast_to_players: Option<(Vec<String>, String)> = None; // (names, msg) 방파말 등
    let mut event_broadcast_lines: Vec<String> = Vec::new();
    let mut event_room_broadcast: Option<(String, String, Vec<String>)> = None;
    let mut event_summon_observers: Vec<(String, String, String, String)> = Vec::new();
    // (opaque target token, sender token, recipient wire output, history line)
    let mut tell_pending: Option<(String, String, String, String)> = None;
    let mut kick_pending: Option<(String, String)> = None; // (target_name, reason)
    let mut _ban_pending: Option<(String, i64, String)> = None; // (target_name, duration, reason)
    let mut set_pending: Option<PendingInput> = None;
    let mut skip_normal_prompt = false;
    let mut disconnect_after_response = false;
    let mut reboot_after_response = false;
    let mut room_append: Option<(String, String, String)> = None;
    let mut event_position_transition = false;
    let mut adult_channel_action: Option<String> = None;
    let mut adult_channel_deliveries: Vec<AdultChannelDelivery> = Vec::new();
    let mut party_actor_id: Option<String> = None;
    let mut party_action: Option<SocialAction> = None;
    let mut party_deliveries: Vec<PartyDelivery> = Vec::new();
    let mut box_deliveries: Vec<BoxDelivery> = Vec::new();
    let mut teach_skill_pending: Option<(String, String)> = None;
    let mut remove_skill_pending: Option<(String, String)> = None;
    let mut guild_kick_pending: Option<String> = None;
    let mut save_all_pending = false;
    let mut set_skill_pending: Option<(String, String, i64)> = None;
    let mut guild_transfer_pending: Option<String> = None;
    let mut guild_position_pending: Option<(String, String)> = None;
    let mut guild_nickname_pending: Option<(String, String)> = None;
    let mut guild_accept_pending: Option<(String, String)> = None;
    let mut guild_apply_pending: Option<(String, String)> = None;
    let mut guild_reset_pending: Option<String> = None;
    let mut admin_set_player_value_pending: Option<(String, String, serde_json::Value)> = None;
    let mut summon_player_pending: Vec<(String, String, String)> = Vec::new();
    let mut force_command_pending: Vec<(String, String)> = Vec::new();
    let mut event_command_pending: Option<String> = None;
    let mut change_player_pending: Option<String> = None;
    let mut set_player_attr_pending: Option<(String, String, i64)> = None;
    let mut movement_follower_candidates: Vec<(SocketAddr, String)> = Vec::new();
    let mut follower_move_pending: Vec<(SocketAddr, String)> = Vec::new();
    let mut auto_move_followup: Option<String> = None;
    let mut room_auto_move_pending: Option<(String, String, String)> = None;
    // Party/emotion/mugong snapshots below must observe Python's individual
    // Room.objs model on the very first command, not one command after the
    // script engine upgrades a legacy compact floor stack.
    let legacy_floor_player = broadcaster
        .clients
        .lock()
        .get(&addr)
        .and_then(|client| client.player.as_ref())
        .map(|player| player.body.get_name());
    if let Some(player_name) = legacy_floor_player.as_deref() {
        crate::script::materialize_legacy_room_stacks_for_player(player_name);
    }
    {
        let mut clients = broadcaster.clients.lock();
        let world = get_world_state().read().unwrap();
        // 봐/보 등 스크립트의 view_map_data → get_other_players_desc_in_room이 clients 락을 다시 잡으면 데드락.
        let player_name = clients
            .get(&addr)
            .and_then(|c| c.player.as_ref())
            .map(|p| p.body.get_string("이름"))
            .unwrap_or_default();
        let player_connection_token = clients
            .get(&addr)
            .map(|client| client.connection_token.clone())
            .unwrap_or_default();
        let (zone, room) = world
            .get_player_position(&player_name)
            .map(|p| (p.zone.clone(), p.room.clone()))
            .unwrap_or((String::new(), "0".to_string()));
        let room_player_names = world.get_players_in_room(&zone, &room);
        let room_player_bindings = broadcaster.player_bindings_for_names(&room_player_names);
        let follower_tokens = broadcaster.movement_follower_tokens(&player_connection_token);
        let follower_bindings = broadcaster.connection_bindings_for_tokens(&follower_tokens);
        for (follower_token, follower_addr) in follower_bindings {
            // Python snapshots `f.env == prev` after the leader has entered
            // the destination.  A legal self-follow therefore never queues
            // the leader a second time.
            if follower_token == player_connection_token {
                continue;
            }
            let Some(client) = clients.get(&follower_addr) else {
                continue;
            };
            if client.state != ClientState::Active || client.connection_token != follower_token {
                continue;
            }
            let Some(follower) = client.player.as_ref() else {
                continue;
            };
            let follower_name = follower.body.get_name();
            if follower_name.is_empty() {
                continue;
            }
            let same_source_room = world
                .get_player_position(&follower_name)
                .is_some_and(|position| position.zone == zone && position.room == room);
            if same_source_room {
                movement_follower_candidates.push((follower_addr, follower_name));
            }
        }
        let (mut other_descs, mut other_map) =
            collect_other_players_from_map(&player_name, &room_player_bindings, &clients);
        for summoned in world.summoned_users_in_room(&zone, &room) {
            if summoned.body.get_int("투명상태") == 1 {
                continue;
            }
            let name = summoned.body.get_name();
            let desc = summoned.body.get_desc_for_look(false);
            other_descs.push(desc.clone());
            other_map.entry(name).or_insert(desc);
        }
        PRE_COMPUTED_OTHER_DESCS.with(|c| *c.borrow_mut() = Some(other_descs));
        PRE_COMPUTED_OTHER_MAP.with(|c| *c.borrow_mut() = Some(other_map));

        // viewMapData after movement needs raw fields from the destination
        // room while this client map is already locked.  Snapshot only the
        // current room and its immediate exits through WorldState's room
        // index; never scan all connected players.
        let mut view_rooms = vec![(zone.clone(), room.clone())];
        if single_word_command {
            view_rooms.extend(immediate_exit_destinations(&zone, &room));
        }
        let mut seen_view_rooms = std::collections::HashSet::new();
        let mut room_view_players = HashMap::new();
        for (view_zone, view_room) in view_rooms {
            let room_key = format!("{view_zone}:{view_room}");
            if !seen_view_rooms.insert(room_key.clone()) {
                continue;
            }
            let names = world.get_players_in_room(&view_zone, &view_room);
            let bindings = broadcaster.player_bindings_for_names(&names);
            let snapshots = bindings
                .iter()
                .filter_map(|(indexed_name, client_addr)| {
                    let client = clients.get(client_addr)?;
                    if client.state != ClientState::Active {
                        return None;
                    }
                    let player = client.player.as_ref()?;
                    (player.body.get_string("이름") == *indexed_name).then(|| {
                        build_room_view_player_snapshot_with_interactive(
                            &player.body,
                            player.interactive,
                        )
                    })
                })
                .chain(
                    world
                        .summoned_users_in_room(&view_zone, &view_room)
                        .into_iter()
                        .map(|user| {
                            build_room_view_player_snapshot_with_interactive(&user.body, 1)
                        }),
                )
                .collect::<Array>();
            room_view_players.insert(room_key, snapshots);
        }
        set_precomputed_room_view_players(room_view_players);
        let mugong_target_lookup_requested = mugong_command_requested
            && !parsed.args.is_empty()
            && clients
                .get(&addr)
                .and_then(|client| client.player.as_ref())
                .is_some_and(|player| player.body.get_int("관리자등급") >= 1000);
        // 전 접속자 스냅샷은 실제로 전역 목록을 쓰는 Python 명령에만 만든다.
        // 같은 방 명령은 world의 room player 목록과 전용 snapshot을 사용한다.
        let mut all_online = Array::new();
        let mut online_names = Array::new();
        let mut connected_names = Array::new();
        let mut tell_players = Vec::new();
        let room_body_snapshots_requested =
            inventory_command_requested || mugong_target_lookup_requested;
        let mut room_inventories = if room_body_snapshots_requested {
            Vec::with_capacity(room_player_bindings.len())
        } else {
            Vec::new()
        };
        let mut room_mugong_targets = if room_body_snapshots_requested {
            Vec::with_capacity(room_player_bindings.len())
        } else {
            Vec::new()
        };
        let mut mugong_players = HashMap::new();
        if all_online_details_requested
            || online_names_requested
            || connected_names_requested
            || tell_players_requested
        {
            for client_addr in broadcaster.client_addresses_in_order() {
                let Some(client) = clients.get(&client_addr) else {
                    continue;
                };
                let p = match &client.player {
                    Some(player) => player,
                    None => continue,
                };
                let name = p.body.get_string("이름");
                if name.is_empty() {
                    continue;
                }
                if connected_names_requested {
                    connected_names.push(Dynamic::from(name.clone()));
                }
                if tell_players_requested {
                    tell_players.push(TellPlayerSnapshot::new(
                        client.connection_token.clone(),
                        name.clone(),
                        p.state == STATE_ACTIVE,
                        p.body.get_int("투명상태") == 1,
                        &p.body.get_string("설정상태"),
                        p.interactive,
                        p.body.get_hp(),
                        p.body.get_max_hp(),
                        p.body.get_mp(),
                        p.body.get_max_mp(),
                        client_addr == addr,
                    ));
                }
                if socket_details_requested {
                    let mut details = Map::new();
                    details.insert("이름".into(), Dynamic::from(name.clone()));
                    details.insert("host".into(), Dynamic::from(client_addr.ip().to_string()));
                    all_online.push(Dynamic::from(details));
                    continue;
                }
                if client.state != ClientState::Active && !global_details_include_inactive {
                    continue;
                }
                if online_names_requested {
                    online_names.push(Dynamic::from(name.clone()));
                }
                if !all_online_details_requested
                    || (p.body.get_int("투명상태") == 1
                        && !global_snapshot_includes_transparent(&move_cmd)
                        && !movement_observers_requested)
                {
                    continue;
                }
                let mut details = Map::new();
                details.insert("이름".into(), Dynamic::from(name.clone()));
                details.insert(
                    "active".into(),
                    Dynamic::from(i64::from(client.state == ClientState::Active)),
                );
                details.insert(
                    "무림별호".into(),
                    Dynamic::from(p.body.get_string("무림별호")),
                );
                details.insert("성격".into(), Dynamic::from(p.body.get_string("성격")));
                details.insert(
                    "레벨초기화".into(),
                    Dynamic::from(p.body.get_string("레벨초기화")),
                );
                details.insert("소속".into(), Dynamic::from(p.body.get_string("소속")));
                details.insert(
                    "반응이름".into(),
                    Dynamic::from(p.body.get_string("반응이름")),
                );
                details.insert("투명상태".into(), Dynamic::from(p.body.get_int("투명상태")));
                details.insert(
                    "관리자등급".into(),
                    Dynamic::from(p.body.get_int("관리자등급")),
                );
                for key in [
                    "힘",
                    "은전",
                    "레벨",
                    "최고체력",
                    "최고내공",
                    "민첩성",
                    "맷집",
                    "명중",
                    "회피",
                    "필살",
                    "운",
                    "나이",
                ] {
                    details.insert(key.into(), Dynamic::from(p.body.get_int(key)));
                }
                // Python administrators may rank any numeric Body attribute,
                // not only the public allow-list. Snapshot the requested key
                // dynamically so Rhai sees the same c[line] value.
                if let Some(attribute) = live_rank_attribute.as_deref() {
                    details.insert(attribute.into(), Dynamic::from(p.body.get_int(attribute)));
                }
                details.insert(
                    "공격력".into(),
                    Dynamic::from(i64::from(p.body.get_attack_power())),
                );
                details.insert(
                    "숙련도차이".into(),
                    Dynamic::from(p.body.get_mastery_diff()),
                );
                details.insert(
                    "방어력".into(),
                    Dynamic::from(i64::from(p.body.get_armor())),
                );
                details.insert("host".into(), Dynamic::from(client_addr.ip().to_string()));
                details.insert(
                    "설정상태".into(),
                    Dynamic::from(p.body.get_string("설정상태")),
                );
                details.insert("현재체력".into(), Dynamic::from(p.body.get_hp()));
                details.insert("현재내공".into(), Dynamic::from(p.body.get_mp()));
                details.insert("현재최고체력".into(), Dynamic::from(p.body.get_max_hp()));
                details.insert("현재최고내공".into(), Dynamic::from(p.body.get_max_mp()));
                details.insert(
                    "show_prompt".into(),
                    Dynamic::from(
                        p.interactive == 1
                            && !crate::script::config_is_enabled(
                                &p.body.get_string("설정상태"),
                                "엘피출력",
                            ),
                    ),
                );
                details.insert("직위".into(), Dynamic::from(p.body.get_string("직위")));
                details.insert(
                    "interactive".into(),
                    Dynamic::from(i64::from(p.interactive)),
                );
                if let Some(pos) = world.get_player_position(&name) {
                    details.insert("zone".into(), Dynamic::from(pos.zone.clone()));
                    details.insert("room".into(), Dynamic::from(pos.room.clone()));
                } else {
                    details.insert("zone".into(), Dynamic::from(""));
                    details.insert("room".into(), Dynamic::from("0"));
                }
                all_online.push(Dynamic::from(details));
            }
            for summoned in world.summoned_users() {
                let body = &summoned.body;
                let name = body.get_name();
                if connected_names_requested {
                    connected_names.push(Dynamic::from(name.clone()));
                }
                if online_names_requested {
                    online_names.push(Dynamic::from(name.clone()));
                }
                if !all_online_details_requested || body.get_int("투명상태") == 1 {
                    continue;
                }
                let mut details = Map::new();
                for key in [
                    "이름",
                    "무림별호",
                    "성격",
                    "레벨초기화",
                    "소속",
                    "반응이름",
                    "설정상태",
                    "직위",
                ] {
                    details.insert(key.into(), Dynamic::from(body.get_string(key)));
                }
                for key in [
                    "힘",
                    "은전",
                    "레벨",
                    "최고체력",
                    "최고내공",
                    "민첩성",
                    "맷집",
                    "명중",
                    "회피",
                    "필살",
                    "운",
                    "나이",
                    "투명상태",
                    "관리자등급",
                ] {
                    details.insert(key.into(), Dynamic::from(body.get_int(key)));
                }
                details.insert("현재체력".into(), Dynamic::from(body.get_hp()));
                details.insert("현재내공".into(), Dynamic::from(body.get_mp()));
                details.insert("zone".into(), Dynamic::from(summoned.position.zone.clone()));
                details.insert("room".into(), Dynamic::from(summoned.position.room.clone()));
                details.insert("host".into(), Dynamic::from(""));
                // Python 사용자몹소환 creates a Player with the normal
                // interactive default and appends it to channel.players.
                details.insert("interactive".into(), Dynamic::from(1_i64));
                all_online.push(Dynamic::from(details));
            }
        }
        if room_body_snapshots_requested {
            for (indexed_name, client_addr) in &room_player_bindings {
                let Some(client) = clients.get(client_addr) else {
                    continue;
                };
                if client.state != ClientState::Active {
                    continue;
                }
                let Some(player) = client.player.as_ref() else {
                    continue;
                };
                if player.body.get_string("이름") != *indexed_name {
                    continue;
                }
                if inventory_command_requested {
                    room_inventories.push(build_room_player_inventory_snapshot(&player.body));
                }
                if mugong_target_lookup_requested {
                    mugong_players.insert(
                        indexed_name.clone(),
                        build_room_mugong_player_snapshot(&player.body),
                    );
                }
            }
        }
        if mugong_target_lookup_requested {
            let mut mugong_mobs = HashMap::new();
            for mob in world.mob_cache.get_all_mobs_in_room(&zone, &room) {
                if let Some(data) = world.mob_cache.get_mob(&mob.mob_key) {
                    mugong_mobs.insert(mob.instance_id, build_room_mugong_mob_snapshot(mob, data));
                }
            }
            let mut mugong_items = HashMap::new();
            for item in world.get_room_objs(&zone, &room) {
                let pointer = std::sync::Arc::as_ptr(&item) as usize;
                if let Ok(item) = item.lock() {
                    mugong_items.insert(pointer, build_room_mugong_item_snapshot(&item));
                }
            }
            // Rebuild Python Room.objs order across types. Each selected
            // snapshot is removed so unindexed legacy objects can be appended
            // exactly once below.
            for object in world.get_room_object_order(&zone, &room) {
                let selected = match object {
                    crate::world::RoomObjectRef::Player(name) => mugong_players.remove(&name),
                    crate::world::RoomObjectRef::Mob(id) => mugong_mobs.remove(&id),
                    crate::world::RoomObjectRef::FloorItem(pointer) => {
                        mugong_items.remove(&pointer)
                    }
                    _ => None,
                };
                if let Some(selected) = selected {
                    room_mugong_targets.push(selected);
                }
            }
            for (name, _) in &room_player_bindings {
                if let Some(snapshot) = mugong_players.remove(name) {
                    room_mugong_targets.push(snapshot);
                }
            }
            for mob in world.mob_cache.get_all_mobs_in_room(&zone, &room) {
                if let Some(snapshot) = mugong_mobs.remove(&mob.instance_id) {
                    room_mugong_targets.push(snapshot);
                }
            }
            for item in world.get_room_objs(&zone, &room) {
                let pointer = std::sync::Arc::as_ptr(&item) as usize;
                if let Some(snapshot) = mugong_items.remove(&pointer) {
                    room_mugong_targets.push(snapshot);
                }
            }
            let mut stack_items: Vec<_> = world
                .get_room_objs_stack(&zone, &room)
                .into_iter()
                .collect();
            stack_items.sort_by(|(left, _), (right, _)| left.cmp(right));
            for (key, count) in stack_items {
                if count <= 0 {
                    continue;
                }
                if let Some(item) = build_room_mugong_stack_item_snapshot(&key, count) {
                    room_mugong_targets.push(item);
                }
            }
        }
        set_precomputed_all_online(all_online);
        set_precomputed_online_names(online_names);
        set_precomputed_connected_names(connected_names);
        if tell_players_requested {
            set_precomputed_tell_players(tell_players);
        }
        if adult_channel_requested {
            install_adult_channel_snapshot(broadcaster, &clients, addr);
        }
        if box_command_requested {
            let observers = room_player_bindings
                .iter()
                .filter_map(|(indexed_name, client_addr)| {
                    let client = clients.get(client_addr)?;
                    if client.state != ClientState::Active {
                        return None;
                    }
                    let player = client.player.as_ref()?;
                    (player.body.get_name() == *indexed_name).then(|| {
                        build_box_observer_snapshot(
                            client.connection_token.clone(),
                            &player.body,
                            player.interactive,
                        )
                    })
                })
                .collect::<Array>();
            set_precomputed_box_context(player_connection_token.clone(), observers);
        }
        if party_command_requested {
            party_actor_id = install_party_context(broadcaster, &clients, &world, addr);
        }
        if inventory_command_requested {
            set_precomputed_room_inventories(room_inventories);
        }
        if mugong_target_lookup_requested {
            set_precomputed_room_mugong_targets(room_mugong_targets);
        }
        let _tl = PreComputedOtherDescsGuard;

        // 한국어 어법: 명령어가 마지막. [대상] [인용구] [명령] (예: 밍밍 하하 웃음). parser가 이미 마지막 단어=command, 나머지=args.
        let is_emotion = emotion::is_emotion_command(parsed.command.as_str());
        let (emotion_param, emotion_target) = if is_emotion {
            let ep = parsed.args.as_str();
            let fip = ep.split_whitespace().next().unwrap_or("");
            let t = find_emotion_target_in_room(
                &zone,
                &room,
                fip,
                &player_name,
                &world,
                &room_player_bindings,
                &clients,
            );
            (ep.to_string(), t)
        } else {
            (String::new(), None)
        };

        // 데드락 방지: world.read()를 잡은 채로 (cmd.handler)를 호출하면,
        // 귀환/이동 등이 get_world_state().write()를 시도해 블로킹된다. 핸들러 호출 전에 해제.
        drop(world);

        if room_combat_context_requested {
            let mut cast_players = Vec::new();
            for (indexed_name, client_addr) in &room_player_bindings {
                if indexed_name == &player_name {
                    continue;
                }
                let Some(client) = clients.get_mut(client_addr) else {
                    continue;
                };
                if client.state != ClientState::Active {
                    continue;
                }
                let Some(other) = client.player_mut() else {
                    continue;
                };
                if other.body.get_name() != *indexed_name {
                    continue;
                }
                cast_players.push(CastRoomPlayerRef::new_with_interactive(
                    &mut other.body,
                    other.interactive,
                ));
            }
            set_cast_room_players(cast_players);
        }

        // The script engine only receives the actor Body. Keep cheap Body
        // snapshots for arbitrary event scripts, but defer detailed admin
        // calculations and JSON conversion until an admin efun is called.
        let room_admin_snapshot_started = Instant::now();
        let room_admin_bodies: Vec<(String, Body)> = room_player_bindings
            .iter()
            .filter_map(|(name, client_addr)| {
                clients
                    .get(client_addr)
                    .and_then(|client| client.player.as_ref())
                    .map(|player| (name.clone(), player.body.clone()))
            })
            .collect();
        let room_admin_body_count = room_admin_bodies.len();
        set_precomputed_room_admin_bodies(room_admin_bodies);
        debug!(
            target: "muc_perf",
            command = %parsed.command,
            room_admin_body_count,
            room_admin_snapshot_us = room_admin_snapshot_started.elapsed().as_micros(),
            "prepared lazy room admin snapshots"
        );
        if let Some(client) = clients.get_mut(&addr) {
            if let Some(player) = client.player_mut() {
                player.body.temp_mut().insert(
                    "_connection_token".to_string(),
                    crate::object::Value::String(player_connection_token.clone()),
                );
                player.body.temp_mut().remove("_online_room_admin");
                player.body.temp_mut().insert(
                    "_auto_move_count".to_string(),
                    crate::object::Value::Int(player.auto_move_list.len() as i64),
                );
                let words: Vec<&str> = parsed.raw.split_whitespace().collect();
                let internal_movement = command_registry.get_internal("movement").cloned();
                let internal_leave = command_registry.get_internal("leave").cloned();

                // Python checks env.limitCmds before mob events, global aliases,
                // one-word exits and ordinary commands.  The private Rhai hook
                // owns the denial text; InternalNotHandled means continue.
                let limit_result = (!python_say_syntax)
                    .then(|| {
                        internal_movement.as_ref().map(|handler| {
                            (handler)(&mut player.body, &["__limit", raw_command_token])
                        })
                    })
                    .flatten();
                let limit_claimed = limit_result
                    .as_ref()
                    .is_some_and(|result| !matches!(result, CommandResult::InternalNotHandled));

                let result = if limit_claimed {
                    limit_result
                } else if let Some(r) = (!python_say_syntax)
                    .then(|| try_item_event(&mut player.body, &zone, &parsed.raw))
                    .flatten()
                {
                    info!("[CMD] Item event: {}", parsed.raw);
                    Some(r)
                } else if let Some(r) = (!python_say_syntax)
                    .then(|| try_fixture_event(&mut player.body, &zone, &room, &parsed.raw))
                    .flatten()
                {
                    info!("[CMD] Fixture event: {}", parsed.raw);
                    Some(r)
                } else if let Some(r) = (!python_say_syntax)
                    .then(|| try_mob_event(&mut player.body, &zone, &room, &parsed.raw))
                    .flatten()
                {
                    // Python checkMobEvent runs before exits and cmdList for
                    // both one-word and multi-word inputs.
                    info!(
                        "[CMD] Mob event: {} -> {:?}",
                        parsed.raw,
                        std::mem::discriminant(&r)
                    );
                    Some(r)
                } else {
                    let movement_result = if !python_say_syntax && words.len() == 1 {
                        let route_count = player.auto_move_list.len() as i64;
                        player.body.temp_mut().insert(
                            "_movement_route_count".to_string(),
                            crate::object::Value::Int(route_count),
                        );
                        player.body.temp_mut().insert(
                            "_movement_follower_names".to_string(),
                            crate::object::Value::String(
                                movement_follower_candidates
                                    .iter()
                                    .map(|(_, name)| name.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            ),
                        );
                        internal_movement.as_ref().map(|handler| {
                            (handler)(&mut player.body, &["__move", move_cmd.as_str()])
                        })
                    } else {
                        None
                    };
                    let movement_claimed = movement_result
                        .as_ref()
                        .is_some_and(|result| !matches!(result, CommandResult::InternalNotHandled));
                    if movement_claimed {
                        movement_result
                    } else if !python_say_syntax
                        && words.len() == 1
                        && matches!(parsed.command.as_str(), "끝" | "종료")
                    {
                        internal_leave.map(|handler| (handler)(&mut player.body, &[]))
                    } else if is_emotion {
                        Some(emotion::do_emotion(
                            &player.body,
                            parsed.command.as_str(),
                            &emotion_param,
                            emotion_target,
                        ))
                    } else {
                        // Python uses only the last token as the command. A
                        // registered first token must not rescue an unknown
                        // final token. Python only treats a direction as movement when it
                        // is the complete one-word input.  For example,
                        // `맵 동` has final token `동`, but Python does not
                        // dispatch it as either movement or `맵`; it reaches
                        // the ordinary unknown-command message.  The silent
                        // direction must not become an ordinary command.
                        let final_token_is_direction = direction_with_arguments_is_not_a_command(
                            parsed.command.as_str(),
                            words.len(),
                        );
                        let (cmd_lookup, args): (
                            Option<&crate::command::registry::CommandInfo>,
                            Vec<&str>,
                        ) = if final_token_is_direction {
                            (None, vec![])
                        } else if let Some(command) = command_registry.get(parsed.command.as_str())
                        {
                            let preserve_raw = (command.name == "쪽지"
                                && !python_raw_parameter.is_empty())
                                || (command.name == "말" && !parsed.args.is_empty());
                            let args = if preserve_raw {
                                if command.name == "쪽지" {
                                    vec![python_raw_parameter.as_str()]
                                } else {
                                    vec![parsed.args.as_str()]
                                }
                            } else {
                                parsed.tokens.iter().map(String::as_str).collect()
                            };
                            (Some(command), args)
                        } else {
                            (None, vec![])
                        };

                        if let Some(command) = cmd_lookup {
                            debug!("[CMD] Executing: {}", command.name);
                            if command.name == "귀환" || command.name == "도망" {
                                let count = player.auto_move_list.len() as i64;
                                player.body.temp_mut().insert(
                                    "_return_auto_move_count".to_string(),
                                    crate::object::Value::Int(count),
                                );
                            }
                            Some((command.handler)(&mut player.body, &args))
                        } else {
                            // Python checks room['오브젝트:' + cmd] only
                            // after registered commands and emotions. Keep
                            // presentation in the private Rhai router.
                            internal_movement.as_ref().and_then(|handler| {
                                let room_object = (handler)(
                                    &mut player.body,
                                    &["__room_object", parsed.command.as_str()],
                                );
                                (!matches!(room_object, CommandResult::InternalNotHandled))
                                    .then_some(room_object)
                            })
                        }
                    }
                };

                if let Some(crate::object::Value::String(move_name)) =
                    player.body.temp_mut().remove("_movement_completed_move")
                {
                    follower_move_pending.extend(
                        movement_follower_candidates
                            .iter()
                            .map(|(follower_addr, _)| (*follower_addr, move_name.clone())),
                    );
                    // `enterRoom` does not call moveNext when the room has
                    // automatic movement.  It instead schedules the room's
                    // first auto command after one second.
                    if let Some(crate::object::Value::String(auto_move)) =
                        player.body.temp_mut().remove("_movement_room_auto_move")
                    {
                        if let Some(command) = auto_move.split_whitespace().next() {
                            if !command.is_empty() {
                                if let Some((zone, room)) =
                                    player.body.get_string("위치").split_once(':')
                                {
                                    room_auto_move_pending = Some((
                                        command.to_string(),
                                        zone.to_string(),
                                        room.to_string(),
                                    ));
                                }
                            }
                        }
                    } else if auto_move_followup.is_none() {
                        auto_move_followup = player.auto_move_list.first().cloned();
                        if auto_move_followup.is_some() {
                            player.auto_move_list.remove(0);
                            if player.auto_move_list.is_empty() {
                                player.body.temp_mut().insert(
                                    "_after_fight_route_finished".to_string(),
                                    crate::object::Value::Int(1),
                                );
                            }
                        }
                    }
                }

                // Rhai 줄임말 명령이 Body 속성을 바꾼 경우 다음 입력부터 즉시 사용한다.
                player.load_aliases_from_body();
                let event_death_finish_output =
                    finish_lethal_event_rhai(command_registry.as_ref(), &mut player.body);
                let (channel_action, channel_deliveries) =
                    take_adult_channel_requests(&mut player.body);
                adult_channel_action = channel_action;
                adult_channel_deliveries = channel_deliveries;
                let (requested_party_action, requested_party_deliveries) =
                    take_party_requests(&mut player.body);
                party_action = requested_party_action;
                party_deliveries = requested_party_deliveries;
                box_deliveries = take_box_deliveries(&mut player.body);
                teach_skill_pending = take_teach_skill_request(&mut player.body);
                remove_skill_pending = take_remove_skill_request(&mut player.body);
                guild_kick_pending = take_guild_kick_request(&mut player.body);
                save_all_pending = take_save_all_request(&mut player.body);
                set_skill_pending = take_set_skill_request(&mut player.body);
                set_player_attr_pending = take_set_player_attr_request(&mut player.body);
                guild_transfer_pending = take_guild_transfer_request(&mut player.body);
                guild_position_pending = take_guild_position_request(&mut player.body);
                guild_nickname_pending = take_guild_nickname_request(&mut player.body);
                guild_accept_pending = take_guild_accept_request(&mut player.body);
                guild_apply_pending = take_guild_apply_request(&mut player.body);
                guild_reset_pending = take_guild_reset_request(&mut player.body);
                admin_set_player_value_pending =
                    take_admin_set_player_value_request(&mut player.body);
                summon_player_pending = take_summon_player_request(&mut player.body);
                force_command_pending = take_force_command_request(&mut player.body);
                event_command_pending = take_event_command_request(&mut player.body);
                change_player_pending = take_change_player_request(&mut player.body);
                if let Some(route) = take_auto_move_request(&mut player.body) {
                    // Python alias.split(';') preserves empty components and
                    // whitespace inside the saved alias. Only the explicit
                    // empty request is the delete operation.
                    player.auto_move_list = python_auto_move_route(&route);
                    auto_move_followup = player.auto_move_list.first().cloned();
                    if auto_move_followup.is_some() {
                        player.auto_move_list.remove(0);
                    }
                }

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
                    Some(CommandResult::RequestInput { prompt, state }) => {
                        set_pending = Some(state);
                        skip_normal_prompt = true;
                        prompt
                    }
                    Some(CommandResult::Move(_direction)) => {
                        String::new() // Movement is handled elsewhere
                    }
                    // No registered Python/Rhai command returns this legacy
                    // variant. Keep the exhaustive arm output-free so Rust
                    // cannot reintroduce combat text behind the scripts.
                    Some(CommandResult::Combat) => String::new(),
                    Some(CommandResult::Ok) => String::new(),
                    Some(CommandResult::InternalNotHandled) => String::new(),
                    Some(CommandResult::NoPrompt) => String::new(),
                    Some(CommandResult::SayToRoom(to_self, to_room)) => {
                        say_to_room = Some((player_name.clone(), zone.clone(), room, to_room));
                        format!("{}\r\n", to_self)
                    }
                    Some(CommandResult::Shout(msg)) => {
                        shout_to_broadcast = Some(msg);
                        String::new()
                    }
                    Some(CommandResult::Notice(msg)) => {
                        notice_to_broadcast = Some(msg);
                        String::new()
                    }
                    Some(CommandResult::SendToUsers(list)) => {
                        send_to_users = Some(list);
                        String::new()
                    }
                    Some(CommandResult::OutputAndSendToUsers(output, list)) => {
                        send_to_users = Some(list);
                        format!("{}\r\n", output)
                    }
                    Some(CommandResult::Tell {
                        target_token,
                        sender_output,
                        recipient_output,
                        history_line,
                    }) => {
                        tell_pending = Some((
                            target_token,
                            player_connection_token.clone(),
                            recipient_output,
                            history_line,
                        ));
                        sender_output
                    }
                    Some(CommandResult::Disconnect(message)) => {
                        // Python sets INTERACTIVE=2 before writing the Rhai-
                        // supplied farewell and closing the transport.
                        player.interactive = 2;
                        skip_normal_prompt = true;
                        disconnect_after_response = true;
                        message
                    }
                    Some(CommandResult::Reboot) => {
                        // Python calls every loaded Room.update() before
                        // reactor.stop().  Rust preflights its representable
                        // room-update subset so it never leaves a partial
                        // world mutation. An unsupported transient heartbeat
                        // branch must not silently cancel the administrator's
                        // actual reboot request: the process is stopping and
                        // that transient room state is not persisted.
                        match get_world_state()
                            .write()
                            .unwrap()
                            .update_loaded_rooms_before_reboot()
                        {
                            Ok(()) => {}
                            Err(error) => {
                                warn!("Room update skipped before server stop: {}", error);
                            }
                        }
                        reboot_after_response = true;
                        String::new()
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
                        deduct_from_giver,
                        bypass_item_limits,
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
                            deduct_from_giver,
                            bypass_item_limits,
                        ));
                        String::new()
                    }
                    Some(CommandResult::BroadcastToPlayers(names, msg)) => {
                        broadcast_to_players = Some((names, msg));
                        String::new()
                    }
                    Some(CommandResult::MobEvent {
                        output_lines,
                        set_position,
                        broadcast_lines,
                        room_broadcast_lines,
                    }) => {
                        event_broadcast_lines = broadcast_lines;
                        if !room_broadcast_lines.is_empty() {
                            if let Some(position) = get_world_state()
                                .read()
                                .unwrap()
                                .get_player_position(&player_name)
                            {
                                event_room_broadcast = Some((
                                    position.zone.clone(),
                                    position.room.clone(),
                                    room_broadcast_lines,
                                ));
                            }
                        }
                        let (mut out, mut ends_with_event_lp_prompt) = render_event_output_lines(
                            &output_lines,
                            &player.body,
                            player.interactive,
                        );
                        if !event_death_finish_output.is_empty() {
                            if !out.is_empty() && !ends_with_event_lp_prompt {
                                out.push_str("\r\n");
                            }
                            out.push_str(&event_death_finish_output);
                            ends_with_event_lp_prompt = false;
                        }
                        if let Some((z, r)) = set_position {
                            let allowed =
                                crate::script::check_event_summon_destination(&player.body, &z, &r);
                            let mut w = get_world_state().write().unwrap();
                            if (allowed.is_empty() || allowed == "same_place")
                                && w.room_cache.get_room(&z, &r).is_ok()
                            {
                                drop(w);
                                crate::script::clear_summon_combat(&mut player.body);
                                let mut w = get_world_state().write().unwrap();
                                let old_position = w.get_player_position(&player_name).cloned();
                                append_event_summon_departure(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    &player.body,
                                );
                                w.set_player_position(
                                    &player_name,
                                    PlayerPosition::new(z.clone(), r.clone()),
                                );
                                w.spawn_mobs_for_room(&z, &r);
                                room_append = Some((z, r, player_name.clone()));
                                event_position_transition = true;
                                if player.body.get_int("투명상태") != 1 {
                                    let actor = format!(
                                        "\x1b[1m{}\x1b[0;37m{}",
                                        player_name,
                                        crate::hangul::han_iga(&player_name)
                                    );
                                    if let Some(old) = old_position {
                                        event_summon_observers.push((
                                            old.zone,
                                            old.room,
                                            player_name.clone(),
                                            format!(
                                                "{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'",
                                                actor
                                            ),
                                        ));
                                    }
                                    event_summon_observers.push((
                                        room_append.as_ref().unwrap().0.clone(),
                                        room_append.as_ref().unwrap().1.clone(),
                                        player_name.clone(),
                                        format!(
                                            "{} 알수 없는 기운에 감싸여 나타납니다. '고오오오~~~'",
                                            actor
                                        ),
                                    ));
                                }
                            } else if allowed == "fail" {
                                append_event_move_failure(&mut out, &mut ends_with_event_lp_prompt);
                            } else {
                                append_event_summon_rejection(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    &allowed,
                                );
                            }
                        }
                        if !out.is_empty() && !ends_with_event_lp_prompt {
                            out.push_str("\r\n");
                        }
                        out
                    }
                    Some(CommandResult::MobEventEnter {
                        output_lines,
                        set_position,
                        broadcast_lines,
                        room_broadcast_lines,
                        mob_key,
                        event_key,
                        words,
                        line_num,
                        prompt,
                        resume_func,
                    }) => {
                        event_broadcast_lines = broadcast_lines;
                        if !room_broadcast_lines.is_empty() {
                            if let Some(position) = get_world_state()
                                .read()
                                .unwrap()
                                .get_player_position(&player_name)
                            {
                                event_room_broadcast = Some((
                                    position.zone.clone(),
                                    position.room.clone(),
                                    room_broadcast_lines,
                                ));
                            }
                        }
                        let (mut out, mut ends_with_event_lp_prompt) = render_event_output_lines(
                            &output_lines,
                            &player.body,
                            player.interactive,
                        );
                        if !event_death_finish_output.is_empty() {
                            if !out.is_empty() && !ends_with_event_lp_prompt {
                                out.push_str("\r\n");
                            }
                            out.push_str(&event_death_finish_output);
                            ends_with_event_lp_prompt = false;
                        }
                        if let Some((z, r)) = set_position {
                            let allowed =
                                crate::script::check_event_summon_destination(&player.body, &z, &r);
                            let mut w = get_world_state().write().unwrap();
                            if (allowed.is_empty() || allowed == "same_place")
                                && w.room_cache.get_room(&z, &r).is_ok()
                            {
                                drop(w);
                                crate::script::clear_summon_combat(&mut player.body);
                                let mut w = get_world_state().write().unwrap();
                                let old_position = w.get_player_position(&player_name).cloned();
                                append_event_summon_departure(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    &player.body,
                                );
                                w.set_player_position(
                                    &player_name,
                                    PlayerPosition::new(z.clone(), r.clone()),
                                );
                                w.spawn_mobs_for_room(&z, &r);
                                room_append = Some((z, r, player_name.clone()));
                                event_position_transition = true;
                                if player.body.get_int("투명상태") != 1 {
                                    let actor = format!(
                                        "\x1b[1m{}\x1b[0;37m{}",
                                        player_name,
                                        crate::hangul::han_iga(&player_name)
                                    );
                                    if let Some(old) = old_position {
                                        event_summon_observers.push((
                                            old.zone,
                                            old.room,
                                            player_name.clone(),
                                            format!(
                                                "{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'",
                                                actor
                                            ),
                                        ));
                                    }
                                    event_summon_observers.push((
                                        room_append.as_ref().unwrap().0.clone(),
                                        room_append.as_ref().unwrap().1.clone(),
                                        player_name.clone(),
                                        format!(
                                            "{} 알수 없는 기운에 감싸여 나타납니다. '고오오오~~~'",
                                            actor
                                        ),
                                    ));
                                }
                            } else if allowed == "fail" {
                                append_event_move_failure(&mut out, &mut ends_with_event_lp_prompt);
                            } else {
                                append_event_summon_rejection(
                                    &mut out,
                                    &mut ends_with_event_lp_prompt,
                                    &allowed,
                                );
                            }
                        }
                        if !out.is_empty() && !ends_with_event_lp_prompt {
                            out.push_str("\r\n");
                        }
                        set_pending = Some(PendingInput::EventEnter {
                            mob_key,
                            event_key,
                            words,
                            line_num,
                            resume_func,
                        });
                        skip_normal_prompt = true;
                        out.push_str(&prompt);
                        out.push_str("\r\n");
                        out
                    }
                    Some(CommandResult::StartScript {
                        script_name,
                        lines,
                        use_rhai,
                    }) => {
                        let (out_lines, next) = if use_rhai {
                            run_script_chunk_rhai(
                                &mut player.body,
                                &script_name,
                                None,
                                None,
                                None,
                                None,
                            )
                        } else {
                            run_script_chunk(&mut player.body, &lines, 0, None, None)
                        };
                        let mut out = out_lines.join("\r\n");
                        if !out.is_empty() {
                            out.push_str("\r\n");
                        }
                        match next {
                            ScriptNext::Complete => {}
                            ScriptNext::Wait {
                                line_num,
                                prompt,
                                persist_temp,
                                from_confirm,
                                script_ob: so,
                                script_resume_op: sro,
                            } => {
                                set_pending = Some(PendingInput::Script {
                                    name: script_name,
                                    lines: if use_rhai { vec![] } else { lines },
                                    line_num,
                                    temp_input: persist_temp,
                                    from_confirm,
                                    script_ob: so,
                                    script_resume_op: sro,
                                });
                                skip_normal_prompt = true;
                                out.push_str(&prompt);
                                out.push_str("\r\n");
                            }
                        }
                        out
                    }
                    Some(CommandResult::Kick {
                        target_name,
                        reason,
                    }) => {
                        kick_pending = Some((target_name, reason));
                        String::new()
                    }
                    Some(CommandResult::Ban {
                        target_name,
                        duration,
                        reason,
                    }) => {
                        _ban_pending = Some((target_name, duration, reason));
                        String::new()
                    }
                    None => "\x1b[1;31m☞ 무슨 말인지 모르겠어요. *^_^*\x1b[0;37m\r\n".to_string(),
                };
            }
        }
    }

    if let Some(actor_id) = party_actor_id.as_deref() {
        apply_party_requests(broadcaster, actor_id, party_action, party_deliveries);
    }

    if let Some((student_name, skill_name)) = teach_skill_pending {
        let mut clients = broadcaster.clients.lock();
        for client in clients.values_mut() {
            let Some(student) = client.player.as_mut() else {
                continue;
            };
            if student.body.get_name() != student_name {
                continue;
            }
            student.body.skill_list.push(skill_name.clone());
            student
                .body
                .skill_map
                .entry(skill_name.clone())
                .or_insert_with(|| crate::player::SkillTraining::new(1, 0));
            student.body.sync_skill_state_to_attrs();
            let path = format!("data/user/{}.json", student.body.get_name());
            let _ = save_body_to_json(&mut student.body, &path);
            break;
        }
    }

    if let Some((student_name, skill_name)) = remove_skill_pending {
        let mut clients = broadcaster.clients.lock();
        for client in clients.values_mut() {
            let Some(student) = client.player.as_mut() else {
                continue;
            };
            if student.body.get_name() != student_name {
                continue;
            }
            student.body.remove_active_skill_by_name(&skill_name);
            let path = format!("data/user/{}.json", student.body.get_name());
            let _ = save_body_to_json(&mut student.body, &path);
            break;
        }
    }

    if let Some(member_name) = guild_kick_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == member_name)
        }) {
            target.body.object.attr.remove("소속");
            target.body.object.attr.remove("직위");
            target.body.object.attr.remove("방파별호");
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
        }
    }

    if let Some(target_name) = change_player_pending {
        let same_room = get_world_state()
            .read()
            .ok()
            .and_then(|world| {
                let actor = world.get_player_position(&actor_name)?;
                let target = world.get_player_position(&target_name)?;
                Some(actor.zone == target.zone && actor.room == target.room)
            })
            .unwrap_or(false);
        if same_room {
            let mut clients = broadcaster.clients.lock();
            let target_addr = clients.iter().find_map(|(candidate, client)| {
                (client
                    .player
                    .as_ref()
                    .is_some_and(|player| player.body.get_name() == target_name))
                .then_some(*candidate)
            });
            if let Some(target_addr) = target_addr {
                let actor_message =
                    format!("\r\n{}{} 몸을 교환합니다.", actor_name, han_wa(&actor_name));
                let target_message = format!(
                    "\r\n{}{} 몸을 교환합니다.",
                    target_name,
                    han_wa(&target_name)
                );
                let mut actor_player = clients
                    .get_mut(&addr)
                    .and_then(|client| client.player.take());
                if let Some(target_client) = clients.get_mut(&target_addr) {
                    std::mem::swap(&mut actor_player, &mut target_client.player);
                }
                if let Some(actor_client) = clients.get_mut(&addr) {
                    actor_client.player = actor_player;
                }
                if let Some(actor_client) = clients.get(&addr) {
                    let _ = actor_client.send(actor_message);
                }
                if let Some(target_client) = clients.get(&target_addr) {
                    let _ = target_client.send(target_message);
                }
                // 이름→접속 인덱스도 교환된 Player 상태에 맞춰 갱신한다.
                drop(clients);
                broadcaster.bind_player_name(&actor_name, target_addr);
                broadcaster.bind_player_name(&target_name, addr);
            }
        }
    }

    if let Some((target_name, reason)) = kick_pending {
        let (target_addr, cleanup_end) = {
            let mut clients = broadcaster.clients.lock();
            let target_addr = clients.iter().find_map(|(target_addr, client)| {
                client
                    .player
                    .as_ref()
                    .filter(|player| player.body.get_name() == target_name)
                    .map(|_| *target_addr)
            });
            let cleanup_end = if reason == "정리 명령" {
                target_addr.and_then(|target_addr| {
                    let end = command_registry.get("끝")?.handler.clone();
                    let player = clients.get_mut(&target_addr)?.player.as_mut()?;
                    Some((end)(&mut player.body, &[]))
                })
            } else {
                None
            };
            (target_addr, cleanup_end)
        };
        if let Some(target_addr) = target_addr {
            match cleanup_end {
                Some(CommandResult::Disconnect(message)) => {
                    broadcaster.send_to(target_addr, &message)?;
                }
                Some(CommandResult::Output(message)) => {
                    broadcaster.send_to(target_addr, &format!("{message}\r\n"))?;
                    send_game_prompt(broadcaster, target_addr).await?;
                }
                _ => {}
            }
            broadcaster.request_disconnect(target_addr)?;
        }
    }

    if save_all_pending {
        save_all_active_players(broadcaster);
    }

    if let Some((target_name, skill_name, level)) = set_skill_pending {
        let mut clients = broadcaster.clients.lock();
        for client in clients.values_mut() {
            let Some(target) = client.player.as_mut() else {
                continue;
            };
            if target.body.get_name() != target_name {
                continue;
            }
            target.body.skill_map.insert(
                skill_name.clone(),
                crate::player::SkillTraining::new(level, 199_999),
            );
            target.body.sync_skill_state_to_attrs();
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
            break;
        }
    }

    if let Some((target_name, key, value)) = set_player_attr_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|p| p.body.get_name() == target_name)
        }) {
            target.body.set(&key, value);
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
        }
    }

    if let Some(target_name) = guild_transfer_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == target_name)
        }) {
            target.body.set("직위", "방주".to_string());
            target.body.sync_skill_state_to_attrs();
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
        }
    }

    if let Some((target_name, position)) = guild_position_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == target_name)
        }) {
            target.body.set("직위", position);
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
        }
    }

    if let Some((target_name, nickname)) = guild_nickname_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == target_name)
        }) {
            target.body.set("방파별호", nickname);
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
        }
    }

    if let Some((target_name, guild)) = guild_accept_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == target_name)
        }) {
            target.body.set("소속", guild);
            target.body.set("직위", "방파인".to_string());
            let path = format!("data/user/{}.json", target.body.get_name());
            let _ = save_body_to_json(&mut target.body, &path);
        }
    }

    if let Some((target_name, applicant)) = guild_apply_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == target_name)
        }) {
            if append_guild_application(&mut target.body, &applicant) {
                let path = format!("data/user/{}.json", target.body.get_name());
                let _ = save_body_to_json(&mut target.body, &path);
            }
        }
    }

    if let Some(guild) = guild_reset_pending {
        clear_live_guild_members(broadcaster, &guild);
    }
    if let Some((target_name, key, value)) = admin_set_player_value_pending {
        let mut clients = broadcaster.clients.lock();
        if let Some(target) = clients.values_mut().find_map(|client| {
            client
                .player
                .as_mut()
                .filter(|player| player.body.get_name() == target_name)
        }) {
            apply_admin_player_value(&mut target.body, &key, value);
        }
    }

    for (target_name, destination_zone, destination_room) in summon_player_pending {
        let target_addr = broadcaster
            .clients
            .lock()
            .iter()
            .find_map(|(candidate, client)| {
                client
                    .player
                    .as_ref()
                    .is_some_and(|player| player.body.get_name() == target_name)
                    .then_some(*candidate)
            });
        if let (Some(target_addr), Some(handler)) = (
            target_addr,
            command_registry.get_internal("summon_move").cloned(),
        ) {
            let mut target_client = broadcaster.clients.lock().remove(&target_addr);
            if let Some(mut client) = target_client.take() {
                let mut target_output = String::new();
                let mut target_deliveries = Vec::new();
                if let Some(player) = client.player.as_mut() {
                    let result = (handler)(
                        &mut player.body,
                        &[destination_zone.as_str(), destination_room.as_str()],
                    );
                    match result {
                        CommandResult::OutputAndSendToUsers(output, deliveries) => {
                            target_output = output;
                            target_deliveries = deliveries;
                        }
                        CommandResult::SendToUsers(deliveries) => {
                            target_deliveries = deliveries;
                        }
                        CommandResult::Output(output) => target_output = output,
                        _ => {}
                    }
                    if player.interactive == 1
                        && !player
                            .body
                            .get_string("설정상태")
                            .split(['\n', '|'])
                            .any(|entry| entry.trim() == "엘피출력 1")
                    {
                        target_output.push_str(&format!(
                            "\r\n\x1b[0;37;40m[ {}/{}, {}/{} ] ",
                            player.body.get_hp(),
                            player.body.get_max_hp(),
                            player.body.get_mp(),
                            player.body.get_max_mp()
                        ));
                    }
                }
                broadcaster.clients.lock().insert(target_addr, client);
                if !target_output.is_empty() {
                    if let Some(client) = broadcaster.clients.lock().get(&target_addr) {
                        let _ = client.send(target_output);
                    }
                }
                for (recipient, text) in target_deliveries {
                    let recipient_addr =
                        broadcaster
                            .clients
                            .lock()
                            .iter()
                            .find_map(|(candidate, client)| {
                                client
                                    .player
                                    .as_ref()
                                    .is_some_and(|player| player.body.get_name() == recipient)
                                    .then_some(*candidate)
                            });
                    if let Some(recipient_addr) = recipient_addr {
                        if let Some(client) = broadcaster.clients.lock().get(&recipient_addr) {
                            let payload = client.player.as_ref().map_or_else(
                                || format!("\r\n{text}\r\n"),
                                |player| {
                                    summon_observer_payload(&text, &player.body, player.interactive)
                                },
                            );
                            let _ = client.send(payload);
                        }
                    }
                }
            }
        } else if target_addr.is_none() {
            if let Some(handler) = command_registry.get_internal("summon_move").cloned() {
                let extracted = get_world_state().write().ok().and_then(|mut world| {
                    let user = world.take_summoned_user_by_name(&target_name)?;
                    world.set_player_position(&target_name, user.position.clone());
                    Some(user)
                });
                if let Some(mut user) = extracted {
                    let result = (handler)(
                        &mut user.body,
                        &[destination_zone.as_str(), destination_room.as_str()],
                    );
                    let deliveries = match result {
                        CommandResult::OutputAndSendToUsers(_, deliveries)
                        | CommandResult::SendToUsers(deliveries) => deliveries,
                        _ => Vec::new(),
                    };
                    if let Ok(mut world) = get_world_state().write() {
                        let position = world
                            .get_player_position(&target_name)
                            .cloned()
                            .unwrap_or_else(|| user.position.clone());
                        world.remove_player_position(&target_name);
                        world.restore_summoned_user(user, position);
                    }
                    for (recipient, text) in deliveries {
                        let recipient_addr =
                            broadcaster
                                .clients
                                .lock()
                                .iter()
                                .find_map(|(candidate, client)| {
                                    client
                                        .player
                                        .as_ref()
                                        .is_some_and(|player| player.body.get_name() == recipient)
                                        .then_some(*candidate)
                                });
                        if let Some(recipient_addr) = recipient_addr {
                            if let Some(client) = broadcaster.clients.lock().get(&recipient_addr) {
                                let payload = client.player.as_ref().map_or_else(
                                    || format!("\r\n{text}\r\n"),
                                    |player| {
                                        summon_observer_payload(
                                            &text,
                                            &player.body,
                                            player.interactive,
                                        )
                                    },
                                );
                                let _ = client.send(payload);
                            }
                        }
                    }
                }
            }
        }
    }

    apply_adult_channel_requests(
        broadcaster,
        addr,
        adult_channel_action,
        adult_channel_deliveries,
    );

    if let Some((z, r, pname)) = room_append {
        let others = get_other_players_desc_in_room(broadcaster.as_ref(), &z, &r, &pname);
        if let Ok(mut room_str) = build_room_lines(&pname, &others) {
            let is_admin = broadcaster
                .clients
                .lock()
                .get(&addr)
                .and_then(|client| client.player.as_ref())
                .is_some_and(|player| player.body.get_int("관리자등급") >= 1_000);
            append_admin_room_position(&mut room_str, &z, &r, is_admin);
            if !event_position_transition {
                response.push_str("\r\n");
            }
            response.push_str(&room_str);
            if event_position_transition {
                response.push_str(&event_move_lp_prompt(broadcaster, addr));
            }
        }
    }

    if let Some((pname, z, r, msg)) = say_to_room {
        send_to_others_in_room(broadcaster, &z, &r, &[pname.as_str()], &msg);
    }

    if let Some((pname, z, r, to_room, to_target)) = emotion_to_room {
        let exclude: Vec<&str> = if let Some((ref tname, _)) = &to_target {
            vec![pname.as_str(), tname.as_str()]
        } else {
            vec![pname.as_str()]
        };
        send_to_others_in_room(broadcaster, &z, &r, &exclude, &to_room);
        // 플레이어 대상일 때만: 대상에게 buf2(to_target 전용) 전송. 같은 방인지 확인(파이썬 ex=obj 배제와 동일).
        if let Some((tname, tmsg)) = to_target {
            let is_same_room = get_world_state()
                .read()
                .unwrap()
                .get_player_position(&tname)
                .is_some_and(|pos| pos.zone == z && pos.room == r);
            if is_same_room {
                if let Some(target_addr) = broadcaster.find_addr_by_player_name(&tname) {
                    let line = format!("\r\n{}\r\n", tmsg);
                    let clients = broadcaster.clients.lock();
                    let send_failed = clients.get(&target_addr).is_some_and(|client| {
                        client
                            .player
                            .as_ref()
                            .is_some_and(|player| player.body.get_string("이름") == tname)
                            && client.sender.send(line).is_err()
                    });
                    drop(clients);
                    if send_failed {
                        tracing::warn!("Failed to send to emotion target {} (broken pipe)", tname);
                        broadcaster.remove_client(target_addr);
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
        deduct_from_giver,
        bypass_item_limits,
    )) = give_pending.take()
    {
        use std::sync::Mutex;
        let target_is_in_room = get_world_state()
            .read()
            .unwrap()
            .get_player_position(&target_name)
            .is_some_and(|pos| pos.zone == z && pos.room == r);
        let target_addr = target_is_in_room
            .then(|| broadcaster.find_addr_by_player_name(&target_name))
            .flatten();
        if let Some(taddr) = target_addr {
            let mut to_move: Vec<Arc<Mutex<crate::object::Object>>> = Vec::new();
            let mut moved_stack_count = 0usize;
            let mut give_item_error: Option<(String, Option<String>)> = None;
            {
                let mut clients = broadcaster.clients.lock();
                if let Some(giver) = clients.get_mut(&giver_addr).and_then(|c| c.player_mut()) {
                    if deduct_from_giver {
                        if let Some(amt) = give_silver {
                            let have = giver.body.get_int("은전");
                            giver.body.set("은전", (have - amt).max(0));
                        } else if let Some(amt) = give_gold {
                            let have = giver.body.get_int("금전");
                            giver.body.set("금전", (have - amt).max(0));
                        }
                    }
                    // give_item: 아래 별도 블록에서 giver+target 동시에 처리 (출력안함/줄수없음/무게/수량한계 검사)
                }
            }
            // 아이템 건네기: 대상의 무게/수량한계를 검사하려면 giver와 target을 동시에 보유해야 함.
            // 아래 raw remove/insert는 같은 clients 락 안에서 반드시 복귀하는 임시 transaction이다.
            // 접속 lifecycle이 아니므로 Broadcaster의 (name, addr) 인덱스는 그대로 유지한다.
            if give_item.is_some() {
                if let Some((ref name, order, count)) = give_item {
                    let max_items =
                        crate::script::get_murim_config_int("사용자아이템갯수").max(0) as usize;
                    let mut clients = broadcaster.clients.lock();
                    match (clients.remove(&giver_addr), clients.remove(&taddr)) {
                        (None, target_opt) => {
                            if let Some(t) = target_opt {
                                clients.insert(taddr, t);
                            }
                            response = "☞ 오류가 발생했어요.\r\n".to_string();
                            give_item_error = Some(("".to_string(), None));
                        }
                        (Some(giver), None) => {
                            clients.insert(giver_addr, giver);
                            give_item_error = Some((
                                "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^".to_string(),
                                None,
                            ));
                        }
                        (Some(mut giver), Some(mut target)) => {
                            match (giver.player.as_mut(), target.player.as_mut()) {
                                (Some(gp), Some(tp)) => {
                                    let giver_body = &mut gp.body;
                                    let target_body = &mut tp.body;
                                    // 관리자는 무게/수량 제한 없음
                                    let target_is_admin = bypass_item_limits;
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
                                        let ok = o.getName() == name.as_str()
                                            || crate::script::python_item_field_contains(
                                                &o,
                                                "반응이름",
                                                name.as_str(),
                                            );
                                        if !ok || o.getBool("inUse") {
                                            continue;
                                        }
                                        if !bypass_item_limits
                                            && crate::script::python_item_field_contains(
                                                &o,
                                                "아이템속성",
                                                "출력안함",
                                            )
                                        {
                                            continue;
                                        }
                                        n += 1;
                                        if n < order {
                                            continue;
                                        }
                                        if !bypass_item_limits
                                            && crate::script::python_item_field_contains(
                                                &o,
                                                "아이템속성",
                                                "줄수없음",
                                            )
                                        {
                                            if to_move.is_empty() {
                                                give_item_error = Some((
                                                    "☞ 그 물건은 줄 수 없어요. ^^".to_string(),
                                                    None,
                                                ));
                                                break;
                                            }
                                            continue; // 이번 건만 스킵, 다음 후보 계속
                                        }
                                        if !crate::script::inventory_compat::can_accept_object(
                                            &target_body.object,
                                            &o,
                                        ) {
                                            if to_move.is_empty() {
                                                give_item_error = Some((
                                                    "☞ 그 물건은 줄 수 없어요. ^^".to_string(),
                                                    None,
                                                ));
                                            }
                                            break;
                                        }
                                        let w = o.getInt("무게");
                                        // 관리자가 아니면 무게/수량 체크
                                        if !target_is_admin {
                                            if target_body.get_item_weight() + running_weight + w
                                                > target_body.get_str() * 10
                                            {
                                                if to_move.is_empty() {
                                                    let iga = crate::hangul::han_iga(&target_name);
                                                    let go = o.han_obj();
                                                    give_item_error = Some((
                                                        format!(
                                                            "\x1b[1m{}\x1b[0;37m{} 무거워서 받지 못합니다.",
                                                            target_name, iga
                                                        ),
                                                        Some(format!(
                                                            "\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 무거워서 받지 못합니다.",
                                                            giver_name,
                                                            crate::hangul::han_iga(&giver_name),
                                                            go
                                                        )),
                                                    ));
                                                }
                                                break;
                                            }
                                            if target_body.get_item_count() + to_move.len() + 1
                                                > max_items
                                            {
                                                if to_move.is_empty() {
                                                    let iga = crate::hangul::han_iga(&target_name);
                                                    let go = o.han_obj();
                                                    give_item_error = Some((
                                                        format!(
                                                            "\x1b[1m{}\x1b[0;37m{} 수량 한계로 받지 못합니다.",
                                                            target_name, iga
                                                        ),
                                                        Some(format!(
                                                            "\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 수량 한계로 받지 못합니다.",
                                                            giver_name,
                                                            crate::hangul::han_iga(&giver_name),
                                                            go
                                                        )),
                                                    ));
                                                }
                                                break;
                                            }
                                        }
                                        running_weight += w;
                                        to_move.push(obj.clone());
                                    }
                                    if give_item_error.is_none() {
                                        for arc in &to_move {
                                            giver_body.object.remove(arc);
                                            let accepted = crate::script::inventory_compat::store_acquired_object(
                                                &mut target_body.object,
                                                arc.clone(),
                                                true,
                                            );
                                            debug_assert!(accepted);
                                            if let Ok(item) = arc.lock() {
                                                if crate::script::python_item_field_contains(
                                                    &item,
                                                    "아이템속성",
                                                    "단일아이템",
                                                ) {
                                                    let index = item.getString("인덱스");
                                                    if !index.is_empty() {
                                                        let _ = crate::oneitem::oneitem_have(
                                                            &index,
                                                            &target_name,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    clients.insert(giver_addr, giver);
                                    clients.insert(taddr, target);
                                }
                                _ => {
                                    clients.insert(giver_addr, giver);
                                    clients.insert(taddr, target);
                                    give_item_error =
                                        Some(("☞ 오류가 발생했어요.".to_string(), None));
                                }
                            }
                        }
                    }
                }
            }
            if give_item_error.is_none() {
                if let Some((ref key, cnt)) = give_item_stack {
                    let max_items =
                        crate::script::get_murim_config_int("사용자아이템갯수").max(0) as usize;
                    let w = get_item_weight_by_key(key);
                    let mut clients = broadcaster.clients.lock();
                    match (clients.remove(&giver_addr), clients.remove(&taddr)) {
                        (None, target_opt) => {
                            if let Some(t) = target_opt {
                                clients.insert(taddr, t);
                            }
                            response = "☞ 오류가 발생했어요.\r\n".to_string();
                            give_item_error = Some(("".to_string(), None));
                        }
                        (Some(giver), None) => {
                            clients.insert(giver_addr, giver);
                            give_item_error = Some((
                                "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^".to_string(),
                                None,
                            ));
                        }
                        (Some(mut giver), Some(mut target)) => {
                            match (giver.player.as_mut(), target.player.as_mut()) {
                                (Some(gp), Some(tp)) => {
                                    let giver_body = &mut gp.body;
                                    let target_body = &mut tp.body;
                                    // 관리자는 무게/수량 제한 없음
                                    let target_is_admin = bypass_item_limits;
                                    let have = *giver_body.object.inv_stack.get(key).unwrap_or(&0);
                                    let stack_restriction = crate::script::object_from_item_json(
                                        key,
                                    )
                                    .and_then(|(item, _)| {
                                        item.lock().ok().map(|item| {
                                            (
                                                crate::script::python_item_field_contains(
                                                    &item,
                                                    "아이템속성",
                                                    "출력안함",
                                                ),
                                                crate::script::python_item_field_contains(
                                                    &item,
                                                    "아이템속성",
                                                    "줄수없음",
                                                ),
                                            )
                                        })
                                    });
                                    let prior_moved = !to_move.is_empty();
                                    if have <= 0
                                        || (!target_is_admin && stack_restriction.is_none())
                                        || (!target_is_admin
                                            && stack_restriction.is_some_and(|(hidden, _)| hidden))
                                    {
                                        if !prior_moved {
                                            give_item_error = Some((
                                                "☞ 그런 아이템이 소지품에 없어요.".to_string(),
                                                None,
                                            ));
                                        }
                                    } else if !target_is_admin
                                        && stack_restriction
                                            .is_some_and(|(_, cannot_give)| cannot_give)
                                    {
                                        if !prior_moved {
                                            give_item_error = Some((
                                                "☞ 그 물건은 줄 수 없어요. ^^".to_string(),
                                                None,
                                            ));
                                        }
                                    } else {
                                        let requested = cnt.max(0).min(have);
                                        let weight_room = if target_is_admin || w <= 0 {
                                            requested
                                        } else {
                                            (target_body.get_str() * 10
                                                - target_body.get_item_weight())
                                            .max(0)
                                                / w
                                        };
                                        let item_room = if target_is_admin {
                                            requested
                                        } else {
                                            max_items.saturating_sub(target_body.get_item_count())
                                                as i64
                                        };
                                        let movable =
                                            requested.min(weight_room).min(item_room).max(0);
                                        if movable > 0 {
                                            let should_remove = {
                                                let r = giver_body
                                                    .object
                                                    .inv_stack
                                                    .get_mut(key.as_str())
                                                    .unwrap();
                                                *r -= movable;
                                                *r <= 0
                                            };
                                            if should_remove {
                                                giver_body.object.inv_stack.remove(key.as_str());
                                            }
                                            *target_body
                                                .object
                                                .inv_stack
                                                .entry(key.clone())
                                                .or_insert(0) += movable;
                                            moved_stack_count = movable as usize;
                                        } else if !prior_moved
                                            && !target_is_admin
                                            && weight_room == 0
                                        {
                                            let iga = crate::hangul::han_iga(&target_name);
                                            let disp = get_item_display_name(key);
                                            let go = format!(
                                                "\x1b[33m{}\x1b[37m{}",
                                                disp,
                                                crate::hangul::han_obj(&disp)
                                            );
                                            give_item_error = Some((
                                        format!(
                                            "\x1b[1m{}\x1b[0;37m{} 무거워서 받지 못합니다.",
                                            target_name, iga
                                        ),
                                        Some(format!(
                                            "\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 무거워서 받지 못합니다.",
                                            giver_name, iga, go
                                        )),
                                    ));
                                        } else if !prior_moved && !target_is_admin && item_room == 0
                                        {
                                            let iga = crate::hangul::han_iga(&target_name);
                                            let disp = get_item_display_name(key);
                                            let go = format!(
                                                "\x1b[33m{}\x1b[37m{}",
                                                disp,
                                                crate::hangul::han_obj(&disp)
                                            );
                                            give_item_error = Some((
                                        format!(
                                            "\x1b[1m{}\x1b[0;37m{} 수량 한계로 받지 못합니다.",
                                            target_name, iga
                                        ),
                                        Some(format!(
                                            "\r\n\x1b[1m{}\x1b[0;37m{} 줄려는 \x1b[36m{}\x1b[37m 수량 한계로 받지 못합니다.",
                                            giver_name, iga, go
                                        )),
                                    ));
                                        }
                                    }
                                    clients.insert(giver_addr, giver);
                                    clients.insert(taddr, target);
                                }
                                _ => {
                                    clients.insert(giver_addr, giver);
                                    clients.insert(taddr, target);
                                    give_item_error =
                                        Some(("☞ 오류가 발생했어요.".to_string(), None));
                                }
                            }
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
                    if let Some(client) = clients.get(&taddr) {
                        let _ = client.sender.send(format!("\r\n{}\r\n", tm));
                    }
                }
            } else {
                let (c, post, name_multi) = if !to_move.is_empty() {
                    let c = to_move.len() + moved_stack_count;
                    if c == 1 {
                        let o = to_move[0].lock().unwrap();
                        (c, o.han_obj(), o.getName())
                    } else {
                        let o = to_move[0].lock().unwrap();
                        let n = o.getName();
                        (c, n.clone(), n)
                    }
                } else if let Some((ref key, _)) = give_item_stack {
                    let c = moved_stack_count;
                    let name_multi = get_item_display_name(key);
                    let post = format!(
                        "\x1b[33m{}\x1b[37m{}",
                        name_multi,
                        crate::hangul::han_obj(&name_multi)
                    );
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
                // Python 줘줘.py deliberately uses ESC[0m for item-transfer
                // player names, while ordinary 줘.py uses ESC[0;37m.
                let item_name_reset = if bypass_item_limits {
                    "\x1b[0m"
                } else {
                    "\x1b[0;37m"
                };
                let giver_a = format!("\x1b[1m{}{}", giver_name, item_name_reset);
                let target_a = format!("\x1b[1m{}{}", target_name, item_name_reset);
                let (to_self, to_target, to_room) = if let Some(amt) = give_silver {
                    (
                        format!("당신이 {}에게 은전 {}개를 줍니다.", target_a, amt),
                        format!("\r\n{}{} 당신에게 은전 {}개를 줍니다.", giver_a, iga, amt),
                        format!(
                            "{}{} {}에게 은전 {}개를 줍니다.",
                            giver_a, iga, target_a, amt
                        ),
                    )
                } else if let Some(amt) = give_gold {
                    (
                        format!("당신이 {}에게 금전 {}개를 줍니다.", target_a, amt),
                        format!("\r\n{}{} 당신에게 금전 {}개를 줍니다.", giver_a, iga, amt),
                        format!(
                            "{}{} {}에게 금전 {}개를 줍니다.",
                            giver_a, iga, target_a, amt
                        ),
                    )
                } else if c == 0 {
                    response = "☞ 그런 아이템이 소지품에 없어요.\r\n".to_string();
                    (String::new(), String::new(), String::new())
                } else {
                    (
                        if c == 1 {
                            format!("당신이 {}에게 \x1b[36m{}\x1b[37m 줍니다.", target_a, post)
                        } else {
                            format!(
                                "당신이 {}에게 \x1b[36m{}\x1b[37m {}개를 줍니다.",
                                target_a, name_multi, c
                            )
                        },
                        if c == 1 {
                            format!(
                                "\r\n{}{} 당신에게 \x1b[36m{}\x1b[37m 줍니다.",
                                giver_a, iga, post
                            )
                        } else {
                            format!(
                                "\r\n{}{} 당신에게 \x1b[36m{}\x1b[37m {}개를 줍니다.",
                                giver_a, iga, name_multi, c
                            )
                        },
                        if c == 1 {
                            format!(
                                "{}{} {}에게 \x1b[36m{}\x1b[37m 줍니다.",
                                giver_a, iga, target_a, post
                            )
                        } else {
                            format!(
                                "{}{} {}에게 \x1b[36m{}\x1b[37m {}개를 줍니다.",
                                giver_a, iga, target_a, name_multi, c
                            )
                        },
                    )
                };
                if !to_self.is_empty() {
                    response = format!("{}\r\n", to_self);
                    {
                        let clients = broadcaster.clients.lock();
                        if let Some(client) = clients.get(&taddr) {
                            let _ = client.sender.send(format!("\r\n{}\r\n", to_target));
                        }
                    }
                    // Administrator 줘줘 only informs giver and recipient;
                    // Python does not call sendRoom in that command.
                    if !bypass_item_limits {
                        send_to_others_in_room(
                            broadcaster,
                            &z,
                            &r,
                            &[giver_name.as_str(), target_name.as_str()],
                            &to_room,
                        );
                    }
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
            response = "☞ 물품을 건내줄 무림인을 찾을 수 없어요. ^^\r\n".to_string();
        }
    }
    if let Some(ref msg) = shout_to_broadcast {
        broadcast_shout(broadcaster, msg);
    }
    if let Some(ref msg) = notice_to_broadcast {
        broadcast_notice(broadcaster, addr, msg);
    }
    if let Some(list) = send_to_users.take() {
        for (name, msg) in list {
            send_collected_user_message(broadcaster, &name, &msg);
        }
    }
    if let Some((names, msg)) = broadcast_to_players.take() {
        let line = format!("\r\n{}\r\n", msg);
        let bindings = broadcaster.player_bindings_for_names(&names);
        let clients = broadcaster.clients.lock();
        let mut dead_addrs = Vec::new();
        for (name, target_addr) in bindings {
            let Some(client) = clients.get(&target_addr) else {
                continue;
            };
            if client
                .player
                .as_ref()
                .is_none_or(|player| player.body.get_string("이름") != name)
            {
                continue;
            }
            if let Err(_e) = client.sender.send(line.clone()) {
                tracing::warn!("Failed to broadcast to player {} (broken pipe)", name);
                dead_addrs.push(target_addr);
            }
        }
        // Clean up dead clients
        drop(clients);
        for addr in dead_addrs {
            tracing::warn!(
                "Removing dead client {} due to send failure in broadcast_to_players",
                addr
            );
            broadcaster.remove_client(addr);
        }
    }
    broadcaster.send_to(addr, &response)?;
    for (zone, room, actor_name, message) in event_summon_observers {
        send_event_summon_observers(broadcaster, &zone, &room, &actor_name, &message);
    }
    if !event_broadcast_lines.is_empty() {
        let actor_name = broadcaster
            .clients
            .lock()
            .get(&addr)
            .and_then(|client| client.player.as_ref())
            .map(|player| player.body.get_name().to_string())
            .unwrap_or_default();
        broadcast_event_lines_except(broadcaster, &actor_name, &event_broadcast_lines);
    }
    if let Some((zone, room, lines)) = event_room_broadcast {
        let message = lines.join("\r\n");
        if !message.is_empty() {
            let event_actor_name = broadcaster
                .clients
                .lock()
                .get(&addr)
                .and_then(|client| client.player.as_ref())
                .map(|player| player.body.get_name().to_string())
                .unwrap_or_default();
            let observer_deliveries = {
                let names = get_world_state()
                    .read()
                    .unwrap()
                    .get_players_in_room(&zone, &room);
                let bindings = broadcaster.player_bindings_for_names(&names);
                let clients = broadcaster.clients.lock();
                bindings
                    .into_iter()
                    .filter_map(|(name, observer_addr)| {
                        (name != event_actor_name)
                            .then(|| clients.get(&observer_addr))
                            .flatten()
                            .and_then(|client| client.player.as_ref())
                            .map(|player| {
                                (
                                    observer_addr,
                                    summon_observer_payload(
                                        &message,
                                        &player.body,
                                        player.interactive,
                                    ),
                                )
                            })
                    })
                    .collect::<Vec<_>>()
            };
            for (observer_addr, payload) in observer_deliveries {
                broadcaster.send_to(observer_addr, &payload)?;
            }
        }
    }
    // Python emits the actor's Box command lines first, then each same-room
    // observer's sendRoom line and lpPrompt, and only then the actor's normal
    // command prompt.
    apply_box_deliveries(broadcaster, box_deliveries);
    // Python은 먼저 ob.sendLine(msg1), 그 다음 수신자의 `_talker`/history를
    // 갱신하고 수신 문구와 lpPrompt를 보낸다. 같은 사용자에게 보내는
    // 경우에도 이 FIFO 순서를 보존해야 한다.
    if let Some((target_token, sender_token, recipient_output, history_line)) = tell_pending.take()
    {
        apply_tell_delivery(
            broadcaster,
            &target_token,
            &sender_token,
            &recipient_output,
            &history_line,
        );
    }
    if disconnect_after_response {
        // The unbounded sender preserves FIFO order: the Rhai farewell is
        // flushed before the transport-close sentinel.
        broadcaster.request_disconnect(addr)?;
        return Ok(());
    }
    if let Some(s) = set_pending.take() {
        let mut clients = broadcaster.clients.lock();
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_input = Some(s);
        }
    }

    // Python `self.do_command()` from an NPC event runs immediately after
    // the event's own sendLine output and supplies the one final prompt.  It
    // is intentionally distinct from admin force-command delivery: no blank
    // line is injected and the outer command must not add a second prompt.
    if let Some(event_command) = event_command_pending.take() {
        Box::pin(handle_game_command(
            broadcaster,
            addr,
            &event_command,
            command_registry.clone(),
            room_cache.clone(),
            shutdown_notify.clone(),
        ))
        .await?;
        return Ok(());
    }

    // Python `obj.do_command(...)` is synchronous: every forced target
    // command (including 모두끝's repeated `끝`) completes before the
    // issuer's lineReceived callback emits its final prompt.
    for (target_name, forced_command) in force_command_pending.drain(..) {
        let target_addr = broadcaster
            .clients
            .lock()
            .iter()
            .find_map(|(candidate, client)| {
                client
                    .player
                    .as_ref()
                    .is_some_and(|player| player.body.get_name() == target_name)
                    .then_some(*candidate)
            });
        if let Some(target_addr) = target_addr {
            // Python obj.sendLine('') precedes obj.do_command(...).
            broadcaster.send_to(target_addr, "\r\n")?;
            Box::pin(handle_game_command(
                broadcaster,
                target_addr,
                &forced_command,
                command_registry.clone(),
                room_cache.clone(),
                shutdown_notify.clone(),
            ))
            .await?;
        }
    }
    if !skip_normal_prompt {
        send_game_prompt(broadcaster, addr).await?;
    }
    if reboot_after_response {
        if let Some(ref notify) = shutdown_notify {
            // `reactor.stop()` takes effect after the current Twisted input
            // callback, so let this command's normal prompt finish first.
            notify.notify_one();
        }
    }

    // Python schedules each eligible follower with reactor.callLater(0,
    // f.do_command, move) in follower-list order, after the leader's command
    // and prompt have completed. Box the recursive async dispatch so nested
    // follower chains retain that FIFO behavior without a global client scan.
    for (follower_addr, move_name) in follower_move_pending {
        Box::pin(handle_game_command(
            broadcaster,
            follower_addr,
            &move_name,
            command_registry.clone(),
            room_cache.clone(),
            shutdown_notify.clone(),
        ))
        .await?;
    }

    if let Some(next_command) = auto_move_followup {
        Box::pin(handle_game_command(
            broadcaster,
            addr,
            &next_command,
            command_registry.clone(),
            room_cache.clone(),
            shutdown_notify.clone(),
        ))
        .await?;
        // A direction consumes this marker inside __movement. For a final
        // non-movement command, render Python moveNext's completion line now,
        // after do_command(next) and its prompt have returned.
        let route_end = {
            let mut clients = broadcaster.clients.lock();
            let handler = command_registry.get_internal("movement").cloned();
            clients
                .get_mut(&addr)
                .and_then(|client| client.player.as_mut())
                .and_then(|player| {
                    handler.map(|handler| (handler)(&mut player.body, &["__route_end", "_"]))
                })
        };
        if let Some(CommandResult::Output(message)) = route_end {
            broadcaster.send_to(addr, &format!("{message}\r\n"))?;
            send_game_prompt(broadcaster, addr).await?;
        }
    }

    if let Some((next_command, expected_zone, expected_room)) = room_auto_move_pending {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Python's delayed callback executes only if the player is still in
        // the same Room object. A move, return, death or disconnect in the
        // interval silently cancels this hop.
        let still_in_expected_room = {
            let clients = broadcaster.clients.lock();
            let Some(client) = clients.get(&addr) else {
                return Ok(());
            };
            if client.disconnect_requested || client.state != ClientState::Active {
                return Ok(());
            }
            let Some(player) = client.player.as_ref() else {
                return Ok(());
            };
            let name = player.body.get_name();
            drop(clients);
            get_world_state()
                .read()
                .ok()
                .and_then(|world| world.get_player_position(&name).cloned())
                .is_some_and(|position| {
                    position.zone == expected_zone && position.room == expected_room
                })
        };
        if still_in_expected_room {
            Box::pin(handle_game_command(
                broadcaster,
                addr,
                &next_command,
                command_registry,
                room_cache,
                shutdown_notify,
            ))
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_room_view_adds_python_position_suffix_only_for_admins() {
        let original = "\r\n\x1b[1m시험 방\x1b[0;37m\r\n설명";
        let mut admin_view = original.to_string();
        append_admin_room_position(&mut admin_view, "하북성", "3001", true);
        assert_eq!(
            admin_view,
            "\r\n\x1b[1m시험 방\x1b[0;37m (하북성:3001)\r\n설명"
        );

        let mut player_view = original.to_string();
        append_admin_room_position(&mut player_view, "하북성", "3001", false);
        assert_eq!(player_view, original);
    }

    #[test]
    fn event_move_keeps_python_inner_and_outer_prompt_boundaries() {
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let addr: SocketAddr = "127.0.0.1:18192".parse().unwrap();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut client = Client::new(addr, sender);
        client.complete_login();
        let mut player = Player::new();
        player.interactive = 1;
        player.body.set("체력", 90_i64);
        player.body.set("최고체력", 100_i64);
        player.body.set("내공", 8_i64);
        player.body.set("최고내공", 10_i64);
        client.player = Some(player);
        broadcaster.add_client(client);
        assert_eq!(
            event_move_lp_prompt(&broadcaster, addr),
            "\r\n\r\n\x1b[0;37;40m[ 90/100, 8/10 ] "
        );

        broadcaster
            .clients
            .lock()
            .get_mut(&addr)
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .body
            .set("설정상태", "엘피출력 1");
        assert!(event_move_lp_prompt(&broadcaster, addr).is_empty());
    }

    #[test]
    fn summon_observer_payload_matches_python_write_room_and_lp_prompt() {
        let mut body = Body::new();
        body.set("체력", 700_i64);
        body.set("최고체력", 800_i64);
        body.set("내공", 12_i64);
        body.set("최고내공", 14_i64);
        assert_eq!(
            summon_observer_payload("소환 목격", &body, 1),
            "\r\n소환 목격\r\n\r\n\x1b[0;37;40m[ 700/800, 12/14 ] "
        );
        // `$출력`의 같은 방 문구는 Python Player.printScript()가
        // getNameA()로 만든 굵은 ANSI 이름을 이미 포함한다. 전달 계층은
        // 이를 NPC 노란색 이름으로 바꾸거나 ANSI를 제거하지 않고, sendRoom
        // + lpPrompt 경계만 더해야 한다.
        assert_eq!(
            summon_observer_payload("\x1b[1m가람\x1b[0;37m이 절을 합니다.", &body, 1),
            "\r\n\x1b[1m가람\x1b[0;37m이 절을 합니다.\r\n\r\n\x1b[0;37;40m[ 700/800, 12/14 ] "
        );

        body.set("설정상태", "엘피출력 1");
        assert_eq!(
            summon_observer_payload("소환 목격", &body, 1),
            "\r\n소환 목격\r\n"
        );
        body.set("설정상태", "");
        assert_eq!(
            summon_observer_payload("소환 목격", &body, 0),
            "\r\n소환 목격\r\n"
        );
    }

    #[test]
    fn event_summon_observer_delivery_keeps_python_actor_ansi_and_prompt() {
        let broadcaster = crate::network::Broadcaster::new();
        let suffix = std::process::id();
        let zone = format!("소환전달회귀{suffix}");
        let room = "1".to_string();
        let actor_name = format!("소환행위자{suffix}");
        let observer_name = format!("소환목격자{suffix}");
        let actor_addr: SocketAddr = "127.0.0.1:18230".parse().unwrap();
        let observer_addr: SocketAddr = "127.0.0.1:18231".parse().unwrap();
        let (actor_tx, mut actor_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();

        let mut actor_client = Client::new(actor_addr, actor_tx);
        actor_client.complete_login();
        let mut actor_player = Player::new();
        actor_player.state = STATE_ACTIVE;
        actor_player.interactive = 1;
        actor_player.body.set("이름", actor_name.clone());
        actor_client.player = Some(actor_player);
        broadcaster.add_client(actor_client);

        let mut observer = Client::new(observer_addr, observer_tx);
        observer.complete_login();
        let mut player = Player::new();
        player.state = STATE_ACTIVE;
        player.interactive = 1;
        player.body.set("이름", observer_name.clone());
        player.body.set("체력", 70_i64);
        player.body.set("최고체력", 100_i64);
        player.body.set("내공", 8_i64);
        player.body.set("최고내공", 10_i64);
        player.body.set("설정상태", "엘피출력 0");
        observer.player = Some(player);
        broadcaster.add_client(observer);

        {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(&actor_name, PlayerPosition::new(zone.clone(), room.clone()));
            world.set_player_position(
                &observer_name,
                PlayerPosition::new(zone.clone(), room.clone()),
            );
        }
        let actor = format!(
            "\x1b[1m{}\x1b[0;37m{}",
            actor_name,
            crate::hangul::han_iga(&actor_name)
        );
        let message = format!(
            "{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'",
            actor
        );
        send_event_summon_observers(&broadcaster, &zone, &room, &actor_name, &message);
        assert_eq!(
            observer_rx
                .try_recv()
                .expect("observer event summon delivery"),
            format!("\r\n{}\r\n\r\n\x1b[0;37;40m[ 70/100, 8/10 ] ", message)
        );
        assert!(
            actor_rx.try_recv().is_err(),
            "the actor must not receive their own summon observer message"
        );

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(&actor_name);
        world.remove_player_position(&observer_name);
    }

    #[tokio::test]
    async fn resumed_event_script_output_keeps_python_get_name_a_for_same_room_observer() {
        // Python `Player.pressEnter2()` resumes doEvent(), and a later
        // `$출력` still calls printScript(): only the actor sees `당신`, while
        // room observers receive getNameA() plus sendRoom/lpPrompt.  Exercise
        // that full command -> Enter -> Enter delivery path with the original
        // 안휘성 꼬마 event rather than only testing the payload helper.
        let storage = Arc::new(tokio::sync::RwLock::new(crate::script::ScriptStorage::new(
            crate::script::ScriptConfig::default(),
        )));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        crate::command::commands::script::register_script_commands(
            &mut registry,
            storage,
            None,
            None,
            None,
        )
        .await;
        let registry = Arc::new(registry);
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let suffix = std::process::id();
        let zone = "안휘성".to_string();
        let room = format!("출력이벤트회귀-{suffix}");
        let actor_name = format!("출력이벤트행위자-{suffix}");
        let observer_name = format!("출력이벤트목격자-{suffix}");
        let actor_addr: SocketAddr = "127.0.0.1:18191".parse().unwrap();
        let observer_addr: SocketAddr = "127.0.0.1:18192".parse().unwrap();
        let (actor_tx, mut actor_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();
        let drain = |receiver: &mut mpsc::UnboundedReceiver<String>| {
            let mut output = String::new();
            while let Ok(chunk) = receiver.try_recv() {
                output.push_str(&chunk);
            }
            output
        };

        let make_client = |addr, sender, name: &str, hp: i64| {
            let mut client = Client::new(addr, sender);
            client.complete_login();
            let mut player = Player::new();
            player.state = STATE_ACTIVE;
            player.interactive = 1;
            player.body.set("이름", name);
            player.body.set("체력", hp);
            player.body.set("최고체력", 100_i64);
            player.body.set("내공", 8_i64);
            player.body.set("최고내공", 10_i64);
            player.body.set("설정상태", "엘피출력 0");
            client.player = Some(player);
            client
        };
        let mut actor = make_client(actor_addr, actor_tx, &actor_name, 90);
        actor
            .player
            .as_mut()
            .expect("active actor player")
            .body
            .set("이벤트설정리스트", "황소개구리끝");
        broadcaster.add_client(actor);
        broadcaster.add_client(make_client(observer_addr, observer_tx, &observer_name, 70));

        let mob_name = format!("출력이벤트꼬마-{suffix}");
        let mob_key = format!("{zone}:{mob_name}");
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = mob_name.clone();
            data.zone = zone.clone();
            data.reaction_names.push("꼬마".to_string());
            data.events.insert(
                "이벤트 $대화".to_string(),
                crate::world::EventScript::Rhai("40_대화_대.rhai".to_string()),
            );
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world
                .mob_cache
                .add_mob_instance(crate::world::MobInstance::new(
                    mob_key.clone(),
                    zone.clone(),
                    room.clone(),
                    &data,
                ));
            for name in [&actor_name, &observer_name] {
                world.set_player_position(name, PlayerPosition::new(zone.clone(), room.clone()));
            }
        }

        let room_cache = Arc::new(std::sync::Mutex::new(RoomCache::new()));
        handle_game_command(
            &broadcaster,
            actor_addr,
            "꼬마 대화",
            registry.clone(),
            room_cache.clone(),
            None,
        )
        .await
        .unwrap();
        let initial_wire = drain(&mut actor_rx);
        assert!(
            initial_wire.contains("[엔터키를 누르세요]"),
            "original event did not enter step1: {initial_wire:?}"
        );
        assert!(observer_rx.try_recv().is_err());

        handle_pending_change_password_with_registry(
            &broadcaster,
            actor_addr,
            "",
            Some(registry.as_ref()),
        )
        .await
        .unwrap();
        let step1_wire = drain(&mut actor_rx);
        assert!(
            step1_wire.contains("[엔터키를 누르세요]"),
            "first Enter did not enter step2: {step1_wire:?}"
        );
        assert!(observer_rx.try_recv().is_err());

        handle_pending_change_password_with_registry(
            &broadcaster,
            actor_addr,
            "",
            Some(registry.as_ref()),
        )
        .await
        .unwrap();
        let step2_wire = drain(&mut actor_rx);
        assert!(
            step2_wire.contains("[엔터키를 누르세요]"),
            "second Enter did not preserve the source step3 wait: {step2_wire:?}"
        );
        assert_eq!(
            observer_rx.try_recv().unwrap_or_else(|_| {
                panic!("second Enter did not broadcast the source $출력; actor={step2_wire:?}")
            }),
            format!(
                "\r\n\x1b[33m꼬마\x1b[37;40m가 \x1b[1m{actor_name}\x1b[0;37m에게 철사를 선물로 줍니다.\r\n\r\n\x1b[0;37;40m[ 70/100, 8/10 ] "
            )
        );
        assert!(observer_rx.try_recv().is_err());

        handle_pending_change_password_with_registry(
            &broadcaster,
            actor_addr,
            "",
            Some(registry.as_ref()),
        )
        .await
        .unwrap();
        let step3_wire = drain(&mut actor_rx);
        assert!(
            step3_wire.ends_with("\x1b[0;37;40m[ 90/100, 8/10 ] "),
            "third Enter must return to the ordinary prompt: {step3_wire:?}"
        );
        let clients = broadcaster.clients.lock();
        let actor_body = &clients[&actor_addr]
            .player
            .as_ref()
            .expect("active actor player")
            .body;
        assert_eq!(actor_body.object.inv_stack.get("철사"), Some(&1));
        assert!(actor_body.object.objs.is_empty());
        assert!(clients[&actor_addr].pending_input.is_none());
        drop(clients);

        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        world.remove_player_position(&actor_name);
        world.remove_player_position(&observer_name);
    }

    #[tokio::test]
    async fn resumed_event_global_output_excludes_actor_and_has_no_observer_prompt() {
        // A resumed Rhai event can emit `broadcast_output()` as well as a
        // same-room `$출력`.  Python's `$순위갱신` path sends this as a
        // sendToAll1/noPrompt notice after the actor's response.  Keep that
        // ordering when the result comes from PendingInput::EventEnter.
        let suffix = std::process::id();
        let zone = "안휘성".to_string();
        let room = format!("재개전역공지회귀-{suffix}");
        let actor_name = format!("재개전역행위자-{suffix}");
        let observer_name = format!("재개전역목격자-{suffix}");
        let mob_name = format!("재개전역몹-{suffix}");
        let mob_key = format!("{zone}:{mob_name}");
        let script_name = format!("__resumed_global_output_{suffix}.rhai");
        let script_path = std::path::Path::new("data/script")
            .join(&zone)
            .join(&script_name);
        std::fs::write(
            &script_path,
            r#"
fn event() { wait_enter("step1", "[엔터키를 누르세요]"); }
fn step1() {
    output("재개 본인 문구");
    broadcast_output("재개 전역 공지");
    set_position("안휘성", "1");
    end_event();
}
"#,
        )
        .expect("write resumed global-output event script");

        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let actor_addr: SocketAddr = "127.0.0.1:18193".parse().unwrap();
        let observer_addr: SocketAddr = "127.0.0.1:18194".parse().unwrap();
        let (actor_tx, mut actor_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();
        let make_client = |addr, sender, name: &str| {
            let mut client = Client::new(addr, sender);
            client.complete_login();
            let mut player = Player::new();
            player.state = STATE_ACTIVE;
            player.interactive = 1;
            player.body.set("이름", name);
            player.body.set("체력", 90_i64);
            player.body.set("최고체력", 100_i64);
            player.body.set("내공", 8_i64);
            player.body.set("최고내공", 10_i64);
            client.player = Some(player);
            client
        };
        let mut actor = make_client(actor_addr, actor_tx, &actor_name);
        actor.pending_input = Some(PendingInput::EventEnter {
            mob_key: mob_key.clone(),
            event_key: "이벤트 $대화".to_string(),
            words: vec!["재개전역몹".to_string(), "대화".to_string()],
            line_num: 0,
            resume_func: Some("step1".to_string()),
        });
        broadcaster.add_client(actor);
        broadcaster.add_client(make_client(observer_addr, observer_tx, &observer_name));

        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = mob_name.clone();
            data.zone = zone.clone();
            data.events.insert(
                "이벤트 $대화".to_string(),
                crate::world::EventScript::Rhai(script_name),
            );
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world
                .mob_cache
                .add_mob_instance(crate::world::MobInstance::new(
                    mob_key.clone(),
                    zone.clone(),
                    room.clone(),
                    &data,
                ));
            for name in [&actor_name, &observer_name] {
                world.set_player_position(name, PlayerPosition::new(zone.clone(), room.clone()));
            }
        }

        handle_pending_change_password(&broadcaster, actor_addr, "")
            .await
            .unwrap();
        let actor_wire = actor_rx.try_recv().expect("resumed actor response");
        assert!(
            actor_wire.starts_with("재개 본인 문구\r\n"),
            "{actor_wire:?}"
        );
        let actor_prompt = actor_rx.try_recv().expect("ordinary actor prompt");
        assert!(
            actor_prompt.ends_with("\x1b[0;37;40m[ 90/100, 8/10 ] "),
            "{actor_prompt:?}"
        );
        assert_eq!(
            observer_rx
                .try_recv()
                .expect("resumed event move must notify the source observer"),
            format!(
                "\r\n\x1b[1m{actor_name}\x1b[0;37m{} 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'\r\n\r\n\x1b[0;37;40m[ 90/100, 8/10 ] ",
                crate::hangul::han_iga(&actor_name)
            )
        );
        assert_eq!(
            observer_rx.try_recv().expect("resumed global notice"),
            "\r\n재개 전역 공지\r\n"
        );
        assert!(
            observer_rx.try_recv().is_err(),
            "each resumed delivery is sent once"
        );
        assert!(
            actor_rx.try_recv().is_err(),
            "actor must not receive own global notice"
        );

        let mut world = get_world_state().write().unwrap();
        world.mob_cache.remove_mob(&mob_key);
        world.remove_player_position(&actor_name);
        world.remove_player_position(&observer_name);
        drop(world);
        let _ = std::fs::remove_file(script_path);
    }

    #[test]
    fn event_stat_change_lp_prompt_stays_on_the_same_wire_line_as_later_send_line() {
        let mut body = Body::new();
        body.set("체력", 100_i64);
        body.set("최고체력", 100_i64);
        body.set("내공", 20_i64);
        body.set("최고내공", 30_i64);
        let lines = vec![
            "앞문장".to_string(),
            EVENT_LP_PROMPT_MARKER.to_string(),
            "뒤문장".to_string(),
        ];

        let (wire, ends_with_raw_prompt) = render_event_output_lines(&lines, &body, 1);
        assert_eq!(wire, "앞문장\r\n\r\n\x1b[0;37;40m[ 100/100, 20/30 ] 뒤문장");
        assert!(!ends_with_raw_prompt);

        let (wire, ends_with_raw_prompt) =
            render_event_output_lines(&[EVENT_LP_PROMPT_MARKER.to_string()], &body, 1);
        assert_eq!(wire, "\r\n\x1b[0;37;40m[ 100/100, 20/30 ] ");
        assert!(ends_with_raw_prompt);

        body.set("설정상태", "엘피출력 1");
        let (wire, ends_with_raw_prompt) = render_event_output_lines(&lines, &body, 1);
        assert_eq!(wire, "앞문장\r\n뒤문장");
        assert!(!ends_with_raw_prompt);
    }

    #[test]
    fn event_missing_move_uses_python_send_line_boundaries() {
        let mut wire = String::new();
        let mut ends_with_raw_prompt = false;
        append_event_move_failure(&mut wire, &mut ends_with_raw_prompt);
        assert_eq!(wire, "어느곳으로도 위치이동 할 수 없습니다.");
        assert!(!ends_with_raw_prompt);

        let mut wire = "앞선 이벤트 문장".to_string();
        append_event_move_failure(&mut wire, &mut ends_with_raw_prompt);
        assert_eq!(
            wire,
            "앞선 이벤트 문장\r\n어느곳으로도 위치이동 할 수 없습니다."
        );

        let mut wire = "\r\n\x1b[0;37;40m[ 100/100, 20/30 ] ".to_string();
        ends_with_raw_prompt = true;
        append_event_move_failure(&mut wire, &mut ends_with_raw_prompt);
        assert_eq!(
            wire,
            "\r\n\x1b[0;37;40m[ 100/100, 20/30 ] 어느곳으로도 위치이동 할 수 없습니다."
        );
        assert!(!ends_with_raw_prompt);
    }

    #[test]
    fn event_summon_rejection_uses_python_enter_room_messages_and_boundaries() {
        let mut wire = "앞선 이벤트 문장".to_string();
        let mut ends_with_raw_prompt = false;
        append_event_summon_rejection(&mut wire, &mut ends_with_raw_prompt, "pressure");
        assert_eq!(
            wire,
            "앞선 이벤트 문장\r\n\r\n강한 무형의 기운이 당신을 압박합니다."
        );

        let mut wire = "\r\n\x1b[0;37;40m[ 100/100, 20/30 ] ".to_string();
        ends_with_raw_prompt = true;
        append_event_summon_rejection(&mut wire, &mut ends_with_raw_prompt, "room_full");
        assert_eq!(
            wire,
            "\r\n\x1b[0;37;40m[ 100/100, 20/30 ] \r\n☞ 알 수 없는 무형의 기운이 당신을 가로막습니다. ^_^"
        );
        assert!(!ends_with_raw_prompt);

        let mut wire = String::new();
        append_event_summon_rejection(&mut wire, &mut ends_with_raw_prompt, "guild_forbidden");
        assert_eq!(
            wire,
            "\r\n☞ 그곳은 타 방파의 지역이므로 출입하실 수 없습니다."
        );
    }

    #[test]
    fn event_position_move_keeps_python_summon_departure_wire_boundary() {
        let mut body = Body::new();
        let mut wire = "왕대협이 말합니다. \"그럼 열심히 수련 하거라~\"".to_string();
        let mut ends_with_raw_prompt = false;
        append_event_summon_departure(&mut wire, &mut ends_with_raw_prompt, &body);
        assert_eq!(
            wire,
            "왕대협이 말합니다. \"그럼 열심히 수련 하거라~\"\r\n\r\n당신이 알수 없는 기운에 휘말려 사라집니다. '고오오오~~~'"
        );
        assert!(!ends_with_raw_prompt);

        body.set("투명상태", 1_i64);
        let mut hidden = "앞선 문장".to_string();
        append_event_summon_departure(&mut hidden, &mut ends_with_raw_prompt, &body);
        assert_eq!(hidden, "앞선 문장");
    }

    #[tokio::test]
    async fn lethal_mob_event_runs_python_death_presentation_and_drop_in_the_same_response() {
        let storage = Arc::new(tokio::sync::RwLock::new(crate::script::ScriptStorage::new(
            crate::script::ScriptConfig::default(),
        )));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        crate::command::commands::script::register_script_commands(
            &mut registry,
            storage,
            None,
            None,
            None,
        )
        .await;
        let registry = Arc::new(registry);
        let room_cache = Arc::new(std::sync::Mutex::new(RoomCache::new()));
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let addr: SocketAddr = "127.0.0.1:18181".parse().unwrap();
        let name = format!("이벤트즉사회귀-{}", std::process::id());
        let zone = "사천성".to_string();
        let room = "1";
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let mut client = Client::new(addr, sender);
        client.complete_login();
        let mut player = Player::new();
        player.state = STATE_ACTIVE;
        player.interactive = 1;
        player.body.set("이름", name.as_str());
        player.body.set("체력", 100_i64);
        player.body.set("최고체력", 100_i64);
        player.body.set("내공", 10_i64);
        player.body.set("최고내공", 10_i64);
        let mut item = crate::object::Object::new();
        item.set("이름", "즉사시험검");
        item.set("인덱스", "즉사시험검");
        item.set("반응이름", "검");
        player
            .body
            .object
            .objs
            .push(Arc::new(std::sync::Mutex::new(item)));
        client.player = Some(player);
        broadcaster.add_client(client);

        let mob_key = format!("{zone}:이벤트즉사무면옹-{}", std::process::id());
        {
            let mut world = get_world_state().write().unwrap();
            let mut data = crate::world::RawMobData::new();
            data.name = "즉사무면옹".to_string();
            data.zone = zone.clone();
            data.events.insert(
                "이벤트 $대화".to_string(),
                crate::world::EventScript::Rhai("무면옹_대화_대.rhai".to_string()),
            );
            world
                .mob_cache
                .insert_mob_data(mob_key.clone(), data.clone());
            world
                .mob_cache
                .add_mob_instance(crate::world::MobInstance::new(
                    mob_key.clone(),
                    zone.clone(),
                    room,
                    &data,
                ));
            world.set_player_position(&name, PlayerPosition::new(zone.clone(), room.to_string()));
        }

        handle_game_command(
            &broadcaster,
            addr,
            "즉사무면옹 대화",
            registry,
            room_cache,
            None,
        )
        .await
        .unwrap();
        let mut output = String::new();
        while let Ok(chunk) = receiver.try_recv() {
            output.push_str(&chunk);
        }
        assert!(
            output.contains("당신이 쓰러집니다. '쿠웅~~ 철퍼덕~~'"),
            "{output:?}"
        );
        assert!(output.contains("즉사시험검"), "{output:?}");
        assert!(output.contains("당신은 정신이 혼미합니다."), "{output:?}");
        let clients = broadcaster.clients.lock();
        let body = &clients[&addr].player.as_ref().unwrap().body;
        assert_eq!(body.act, crate::player::ActState::Death);
        assert!(body.object.objs.is_empty());
        drop(clients);
        let mut world = get_world_state().write().unwrap();
        assert!(world
            .room_objs
            .get(&format!("{zone}:{room}"))
            .is_some_and(|items| items.iter().any(|item| {
                item.lock().is_ok_and(|item| item.getName() == "즉사시험검")
            })));
        world.mob_cache.remove_mob(&mob_key);
        world.remove_player_position(&name);
        world.room_objs.remove(&format!("{zone}:{room}"));
    }

    #[test]
    fn party_snapshot_keeps_runtime_box_and_socketless_player_in_integrated_order() {
        let broadcaster = crate::network::Broadcaster::new();
        let actor_addr: SocketAddr = "127.0.0.1:18131".parse().unwrap();
        let target_addr: SocketAddr = "127.0.0.1:18132".parse().unwrap();
        let (actor_tx, _actor_rx) = mpsc::unbounded_channel();
        let (target_tx, _target_rx) = mpsc::unbounded_channel();
        for (addr, sender, name) in [
            (actor_addr, actor_tx, "혼합스냅행위자"),
            (target_addr, target_tx, "혼합스냅대상"),
        ] {
            let mut client = Client::new(addr, sender);
            client.complete_login();
            let mut player = Player::new();
            player.state = STATE_ACTIVE;
            player.body.set("이름", name);
            client.player = Some(player);
            broadcaster.add_client(client);
        }
        let zone = "파티통합스냅존";
        let room = "1";
        let runtime_box = Arc::new(std::sync::Mutex::new(crate::object::Object::new()));
        runtime_box.lock().unwrap().set("이름", "혼합스냅대상");
        crate::script::register_installed_box(zone, room, runtime_box.clone());
        let summoned_id = {
            let mut world = get_world_state().write().unwrap();
            world.set_player_position(
                "혼합스냅행위자",
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            world.set_player_position(
                "혼합스냅대상",
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
            // Re-record the runtime box after both connected Players, then a
            // socket-less Player, matching Python's successive insert(0).
            world.record_box(zone, room, &runtime_box);
            let mut summoned = Body::new();
            summoned.set("이름", "혼합스냅대상");
            world.add_summoned_user(
                summoned,
                PlayerPosition::new(zone.to_string(), room.to_string()),
            )
        };

        {
            let clients = broadcaster.clients.lock();
            let world = get_world_state().read().unwrap();
            assert_eq!(
                install_party_context(&broadcaster, &clients, &world, actor_addr),
                clients
                    .get(&actor_addr)
                    .map(|client| client.connection_token.clone())
            );
        }
        let context = crate::script::precomputed_party_context_for_test().unwrap();
        assert_eq!(
            context["room_object_lookup_supported"].as_bool().unwrap(),
            true
        );
        let objects = context["room_objects"].clone().cast::<Array>();
        let kinds = objects
            .iter()
            .map(|object| {
                object.clone().cast::<Map>()["kind"]
                    .clone()
                    .into_string()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(kinds, ["unbound_player", "box", "player", "player"]);
        let selected = crate::script::find_follow_player_for_test("혼합스냅대상").cast::<Map>();
        assert_eq!(selected["lookup_supported"].as_bool().unwrap(), false);

        // Removing only the socket-less Player leaves the matching runtime
        // Box first; Python selects it and `따라` must still stop there.
        {
            let mut world = get_world_state().write().unwrap();
            world.remove_summoned_user_by_id(summoned_id);
        }
        {
            let clients = broadcaster.clients.lock();
            let world = get_world_state().read().unwrap();
            install_party_context(&broadcaster, &clients, &world, actor_addr);
        }
        let selected = crate::script::find_follow_player_for_test("혼합스냅대상").cast::<Map>();
        assert_eq!(selected["lookup_supported"].as_bool().unwrap(), false);

        // Re-entering the connected target prepends that Player ahead of the
        // Box, so the exact same query now resolves to the bindable Player.
        {
            let mut world = get_world_state().write().unwrap();
            world.remove_player_position("혼합스냅대상");
            world.set_player_position(
                "혼합스냅대상",
                PlayerPosition::new(zone.to_string(), room.to_string()),
            );
        }
        {
            let clients = broadcaster.clients.lock();
            let world = get_world_state().read().unwrap();
            install_party_context(&broadcaster, &clients, &world, actor_addr);
        }
        let selected = crate::script::find_follow_player_for_test("혼합스냅대상").cast::<Map>();
        assert_eq!(selected["kind"].clone().into_string().unwrap(), "player");
        assert_eq!(
            selected["name"].clone().into_string().unwrap(),
            "혼합스냅대상"
        );

        crate::script::clear_precomputed_party_context();
        let mut world = get_world_state().write().unwrap();
        world.remove_player_position("혼합스냅행위자");
        world.remove_player_position("혼합스냅대상");
    }

    #[test]
    fn guild_application_appends_to_live_target_without_overwriting_or_substring_duplicates() {
        let mut body = Body::new();
        body.set("입문신청자", "홍길동|길동이");
        assert!(append_guild_application(&mut body, "길동"));
        assert_eq!(body.get_string("입문신청자"), "홍길동\r\n길동이\r\n길동");
        assert!(!append_guild_application(&mut body, "길동"));
        assert_eq!(body.get_string("입문신청자"), "홍길동\r\n길동이\r\n길동");
    }

    #[test]
    fn admin_player_value_request_preserves_runtime_number_and_string_types() {
        let mut body = Body::new();
        apply_admin_player_value(&mut body, "레벨", serde_json::json!(33));
        apply_admin_player_value(&mut body, "배율", serde_json::json!(1.5));
        apply_admin_player_value(&mut body, "설명", serde_json::json!("새 설명"));
        assert_eq!(body.get_int("레벨"), 33);
        assert!(
            matches!(body.object.attr.get("배율"), Some(crate::object::Value::Float(value)) if (*value - 1.5).abs() < f64::EPSILON)
        );
        assert_eq!(body.get_string("설명"), "새 설명");
    }

    #[test]
    fn guild_reset_clears_only_matching_live_member_bodies() {
        let suffix = std::process::id();
        let guild = format!("실시간초기화-{suffix}");
        let other_guild = format!("다른방파-{suffix}");
        let member = format!("실시간방파원-{suffix}");
        let outsider = format!("실시간타방파원-{suffix}");
        let broadcaster = crate::network::Broadcaster::new();
        for (port, name, affiliation) in [
            (18331, member.as_str(), guild.as_str()),
            (18332, outsider.as_str(), other_guild.as_str()),
        ] {
            let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            let (tx, _) = mpsc::unbounded_channel();
            let mut client = Client::new(addr, tx);
            let mut player = Player::new();
            player.body.set("이름", name);
            player.body.set("소속", affiliation);
            player.body.set("직위", "방파인");
            client.player = Some(player);
            broadcaster.add_client(client);
        }

        clear_live_guild_members(&broadcaster, &guild);
        let clients = broadcaster.clients.lock();
        let by_name = |name: &str| {
            clients.values().find_map(|client| {
                client
                    .player
                    .as_ref()
                    .filter(|player| player.body.get_name() == name)
            })
        };
        let cleared = by_name(&member).unwrap();
        assert_eq!(cleared.body.get_string("소속"), "");
        assert_eq!(cleared.body.get_string("직위"), "");
        let preserved = by_name(&outsider).unwrap();
        assert_eq!(preserved.body.get_string("소속"), other_guild);
        assert_eq!(preserved.body.get_string("직위"), "방파인");
        drop(clients);
        let _ = std::fs::remove_file(format!("data/user/{member}.json"));
        let _ = std::fs::remove_file(format!("data/user/{outsider}.json"));
    }

    #[tokio::test]
    async fn room_description_editor_echoes_lines_preserves_blank_and_finishes_like_python() {
        let suffix = std::process::id();
        let zone = format!("방설명입력존-{suffix}");
        let room = "1".to_string();
        let dir = format!("data/map/{zone}");
        let path = format!("{dir}/{room}.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, r#"{"맵정보":{"설명":["옛설명"]}}"#).unwrap();
        get_world_state()
            .write()
            .unwrap()
            .room_cache
            .get_room(&zone, &room)
            .unwrap();

        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let addr: SocketAddr = "127.0.0.1:18341".parse().unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut client = Client::new(addr, tx);
        client.complete_login();
        let mut player = Player::new();
        player.body.set("이름", format!("방설명작성자-{suffix}"));
        client.player = Some(player);
        client.pending_input = Some(PendingInput::RoomDescription {
            zone: zone.clone(),
            room: room.clone(),
            lines: Vec::new(),
        });
        broadcaster.add_client(client);

        handle_pending_change_password(&broadcaster, addr, "첫줄")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "첫줄\r\n:");
        handle_pending_change_password(&broadcaster, addr, "")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), " \r\n:");
        handle_pending_change_password(&broadcaster, addr, ".")
            .await
            .unwrap();
        let finished = rx.try_recv().unwrap();
        assert!(finished.starts_with("작성을 마칩니다.\r\n"));

        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["맵정보"]["설명"], "첫줄\r\n ");
        assert_eq!(
            get_world_state()
                .read()
                .unwrap()
                .room_cache
                .get_room_cached(&zone, &room)
                .unwrap()
                .read()
                .unwrap()
                .description,
            vec!["첫줄".to_string(), " ".to_string()]
        );
        let mut reloaded = crate::world::RoomCache::with_data_dir("data/map");
        let room_data = reloaded.get_room(&zone, &room).unwrap();
        assert_eq!(
            room_data.read().unwrap().description,
            vec!["첫줄".to_string(), " ".to_string()]
        );
        assert!(broadcaster
            .clients
            .lock()
            .get(&addr)
            .is_some_and(|client| client.pending_input.is_none()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_all_matches_python_player_active_filter() {
        let suffix = std::process::id();
        let first_name = format!("모두저장첫째-{suffix}");
        let second_name = format!("모두저장둘째-{suffix}");
        let inactive_name = format!("모두저장비활성-{suffix}");
        let broadcaster = crate::network::Broadcaster::new();
        for (port, name, active, marker) in [
            (18201, first_name.as_str(), true, 11_i64),
            (18202, inactive_name.as_str(), false, 22_i64),
            (18203, second_name.as_str(), true, 33_i64),
        ] {
            let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            let (tx, _) = mpsc::unbounded_channel();
            let mut client = Client::new(addr, tx);
            let mut player = Player::new();
            player.state = if active { STATE_ACTIVE } else { 0 };
            player.body.set("이름", name);
            player.body.set("저장표식", marker);
            client.player = Some(player);
            broadcaster.add_client(client);
        }

        save_all_active_players(&broadcaster);
        for (name, marker) in [(&first_name, 11_i64), (&second_name, 33_i64)] {
            let path = format!("data/user/{name}.json");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(json["사용자오브젝트"]["저장표식"], marker);
            assert!(json["사용자오브젝트"]["마지막저장시간"]
                .as_i64()
                .is_some_and(|value| value > 0));
            let _ = std::fs::remove_file(path);
        }
        let inactive_path = format!("data/user/{inactive_name}.json");
        assert!(!std::path::Path::new(&inactive_path).exists());
    }

    #[test]
    fn notice_delivery_matches_python_sender_recipient_prompt_and_active_filter() {
        let broadcaster = crate::network::Broadcaster::new();
        let sender_addr: SocketAddr = "127.0.0.1:18055".parse().unwrap();
        let recipient_addr: SocketAddr = "127.0.0.1:18056".parse().unwrap();
        let inactive_addr: SocketAddr = "127.0.0.1:18057".parse().unwrap();
        let (sender_tx, mut sender_rx) = mpsc::unbounded_channel();
        let (recipient_tx, mut recipient_rx) = mpsc::unbounded_channel();
        let (inactive_tx, mut inactive_rx) = mpsc::unbounded_channel();

        let mut sender = Client::new(sender_addr, sender_tx);
        sender.complete_login();
        sender.player = Some(Player::new());
        broadcaster.add_client(sender);

        let mut recipient = Client::new(recipient_addr, recipient_tx);
        recipient.complete_login();
        let mut recipient_player = Player::new();
        recipient_player.body.set("체력", 11);
        recipient_player.body.set("최고체력", 22);
        recipient_player.body.set("내공", 3);
        recipient_player.body.set("최고내공", 44);
        recipient_player.body.set("설정상태", "엘피출력 0");
        recipient.player = Some(recipient_player);
        broadcaster.add_client(recipient);

        let mut inactive = Client::new(inactive_addr, inactive_tx);
        inactive.player = Some(Player::new());
        broadcaster.add_client(inactive);

        let message = "──\r\n\x1b[7m☞ 공지 : 점검\x1b[0m\r\n──";
        broadcast_notice(&broadcaster, sender_addr, message);

        assert_eq!(sender_rx.try_recv().unwrap(), format!("{message}\r\n"));
        assert_eq!(
            recipient_rx.try_recv().unwrap(),
            format!("\r\n{message}\r\n\r\n\x1b[0;37;40m[ 11/22, 3/44 ] ")
        );
        assert!(inactive_rx.try_recv().is_err());
    }

    #[test]
    fn rank_event_broadcast_skips_actor_and_preserves_recipient_prompt() {
        let broadcaster = crate::network::Broadcaster::new();
        let actor_addr: SocketAddr = "127.0.0.1:18058".parse().unwrap();
        let observer_addr: SocketAddr = "127.0.0.1:18059".parse().unwrap();
        let inactive_addr: SocketAddr = "127.0.0.1:18060".parse().unwrap();
        let (actor_tx, mut actor_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();
        let (inactive_tx, mut inactive_rx) = mpsc::unbounded_channel();

        let mut actor = Client::new(actor_addr, actor_tx);
        actor.complete_login();
        let mut actor_player = Player::new();
        actor_player.body.set("이름", "순위기록자");
        actor.player = Some(actor_player);
        broadcaster.add_client(actor);

        let mut observer = Client::new(observer_addr, observer_tx);
        observer.complete_login();
        let mut observer_player = Player::new();
        observer_player.body.set("이름", "순위관찰자");
        observer_player.body.set("설정상태", "엘피출력 0");
        observer.player = Some(observer_player);
        broadcaster.add_client(observer);

        let mut inactive = Client::new(inactive_addr, inactive_tx);
        let mut inactive_player = Player::new();
        inactive_player.body.set("이름", "비접속관찰자");
        inactive.player = Some(inactive_player);
        broadcaster.add_client(inactive);

        broadcast_event_lines_except(
            &broadcaster,
            "순위기록자",
            &["첫 공지".to_string(), "둘째 공지".to_string()],
        );

        assert!(actor_rx.try_recv().is_err());
        assert_eq!(
            observer_rx.try_recv().unwrap(),
            "\r\n첫 공지\r\n둘째 공지\r\n"
        );
        assert!(observer_rx.try_recv().is_err());
        assert!(inactive_rx.try_recv().is_err());
    }

    #[test]
    fn box_delivery_routes_rhai_authored_send_room_and_prompt_bytes_unchanged() {
        let broadcaster = crate::network::Broadcaster::new();
        let recipient_addr: SocketAddr = "127.0.0.1:18059".parse().unwrap();
        let (recipient_tx, mut recipient_rx) = mpsc::unbounded_channel();
        let mut recipient_client = Client::new(recipient_addr, recipient_tx);
        recipient_client.complete_login();
        let recipient_id = recipient_client.connection_token.clone();
        let mut recipient = Player::new();
        recipient.state = STATE_ACTIVE;
        recipient.interactive = 1;
        recipient.body.set("이름", "상자관찰자");
        recipient_client.player = Some(recipient);
        broadcaster.add_client(recipient_client);

        let raw = concat!(
            "\r\n\x1b[1m행위자\x1b[0;37m가 보관합니다.\r\n",
            "\r\n\x1b[0;37;40m[ 31/45, 7/9 ] "
        );
        apply_box_deliveries(
            &broadcaster,
            vec![BoxDelivery {
                connection_id: recipient_id,
                raw_text: raw.to_string(),
            }],
        );
        assert_eq!(recipient_rx.try_recv().unwrap(), raw);
        assert!(recipient_rx.try_recv().is_err());
    }

    #[test]
    fn adult_channel_delivery_routes_rhai_authored_bytes_unchanged() {
        let broadcaster = crate::network::Broadcaster::new();
        let actor_addr: SocketAddr = "127.0.0.1:18060".parse().unwrap();
        let recipient_addr: SocketAddr = "127.0.0.1:18061".parse().unwrap();
        let (recipient_tx, mut recipient_rx) = mpsc::unbounded_channel();
        let mut recipient_client = Client::new(recipient_addr, recipient_tx);
        recipient_client.complete_login();
        let mut recipient = Player::new();
        recipient.state = STATE_ACTIVE;
        recipient.interactive = 1;
        recipient.body.set("이름", "채널수신자");
        recipient.body.set("체력", 31_i64);
        recipient.body.set("최고체력", 45_i64);
        recipient.body.set("내공", 7_i64);
        recipient.body.set("최고내공", 9_i64);
        recipient.body.set("설정상태", "엘피출력 0");
        recipient_client.player = Some(recipient);
        broadcaster.add_client(recipient_client);

        let raw = concat!(
            "\r\n\x1b[1;31m①⑨\x1b[0;37m 알림\r\n",
            "\r\n\x1b[0;37;40m[ 31/45, 7/9 ] "
        );
        apply_adult_channel_requests(
            &broadcaster,
            actor_addr,
            Some("join".to_string()),
            vec![AdultChannelDelivery {
                member_id: recipient_addr.to_string(),
                raw_text: raw.to_string(),
            }],
        );
        assert!(broadcaster.is_adult_channel_member(actor_addr));
        assert_eq!(recipient_rx.try_recv().unwrap(), raw);
        assert!(recipient_rx.try_recv().is_err());
    }

    #[test]
    fn automatic_adult_channel_setting_uses_exact_config_pair() {
        assert!(config_value_is_one(
            "자동습득 0\n자동채널입장 1",
            "자동채널입장"
        ));
        assert!(config_value_is_one(
            "자동습득 0|자동채널입장 1",
            "자동채널입장"
        ));
        assert!(!config_value_is_one(
            "자동채널입장 0\n다른자동채널입장 1",
            "자동채널입장"
        ));
    }

    #[test]
    fn tell_delivery_updates_runtime_relation_caps_history_and_preserves_wire_output() {
        let broadcaster = crate::network::Broadcaster::new();
        let target_addr: SocketAddr = "127.0.0.1:18070".parse().unwrap();
        let (target_tx, mut target_rx) = mpsc::unbounded_channel();
        let mut target_client = Client::new(target_addr, target_tx);
        target_client.complete_login();
        let mut target = Player::new();
        target.body.set("이름", "전음전달수신자");
        target.body.talk_history = (0..22).map(|index| format!("old-{index}")).collect();
        target_client.player = Some(target);
        let target_token = target_client.connection_token.clone();
        broadcaster.add_client(target_client);

        let recipient_output = concat!(
            "\r\n[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] 발신자 : 내용 \r\n",
            "\r\n\x1b[0;37;40m[ 10/20, 3/4 ] "
        );
        let history_line = "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] 발신자 : 내용 ";
        assert!(apply_tell_delivery(
            &broadcaster,
            &target_token,
            "sender-object-1",
            recipient_output,
            history_line,
        ));
        assert_eq!(target_rx.try_recv().unwrap(), recipient_output);

        let clients = broadcaster.clients.lock();
        let target = clients
            .get(&target_addr)
            .and_then(|client| client.player.as_ref())
            .unwrap();
        assert_eq!(target.body.talk_history.len(), 22);
        assert_eq!(target.body.talk_history[0], "old-1");
        assert_eq!(target.body.talk_history[21], history_line);
        assert!(matches!(
            target.body.temp().get(crate::script::TELL_TALKER_TOKEN),
            Some(crate::object::Value::String(token)) if token == "sender-object-1"
        ));
    }

    #[tokio::test]
    async fn tell_network_flow_preserves_python_sender_recipient_and_prompt_order() {
        let script_storage = Arc::new(tokio::sync::RwLock::new(crate::script::ScriptStorage::new(
            crate::script::ScriptConfig::default(),
        )));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        crate::command::commands::script::register_script_commands(
            &mut registry,
            script_storage,
            None,
            None,
            None,
        )
        .await;
        let registry = Arc::new(registry);
        let room_cache = Arc::new(std::sync::Mutex::new(RoomCache::new()));
        let broadcaster = Arc::new(crate::network::Broadcaster::new());

        let sender_addr: SocketAddr = "127.0.0.1:18071".parse().unwrap();
        let target_addr: SocketAddr = "127.0.0.1:18072".parse().unwrap();
        let sender_name = "전음망발신자";
        let target_name = "전음망수신자";
        let (sender_tx, mut sender_rx) = mpsc::unbounded_channel();
        let (target_tx, mut target_rx) = mpsc::unbounded_channel();

        let mut sender_client = Client::new(sender_addr, sender_tx);
        sender_client.complete_login();
        let mut sender = Player::new();
        sender.state = STATE_ACTIVE;
        sender.interactive = 1;
        sender.body.set("이름", sender_name);
        sender.body.set("체력", 50);
        sender.body.set("최고체력", 60);
        sender.body.set("내공", 7);
        sender.body.set("최고내공", 8);
        sender_client.player = Some(sender);
        let sender_token = sender_client.connection_token.clone();
        broadcaster.add_client(sender_client);

        let mut target_client = Client::new(target_addr, target_tx);
        target_client.complete_login();
        let mut target = Player::new();
        target.state = STATE_ACTIVE;
        target.interactive = 1;
        target.body.set("이름", target_name);
        target.body.set("체력", 31);
        target.body.set("최고체력", 45);
        target.body.set("내공", 7);
        target.body.set("최고내공", 9);
        target.body.set("설정상태", "전음거부 0\n엘피출력 0");
        target_client.player = Some(target);
        broadcaster.add_client(target_client);

        {
            let mut world = get_world_state().write().unwrap();
            for name in [sender_name, target_name] {
                world.set_player_position(
                    name,
                    PlayerPosition::new("전음망검사존".to_string(), "1".to_string()),
                );
            }
        }

        handle_game_command(
            &broadcaster,
            sender_addr,
            &format!("{target_name} 여러 단어 전음"),
            registry.clone(),
            room_cache.clone(),
            None,
        )
        .await
        .unwrap();

        let tag = "[\x1b[1m\x1b[36m전음\x1b[0m\x1b[37m] ";
        assert_eq!(
            sender_rx.try_recv().unwrap(),
            format!("{tag}{target_name}에게 보냄 : 여러 단어 \r\n\r\n")
        );
        assert_eq!(
            sender_rx.try_recv().unwrap(),
            "\r\n\x1b[0;37;40m[ 50/60, 7/8 ] "
        );
        let history_line = format!("{tag}{sender_name} : 여러 단어 ");
        assert_eq!(
            target_rx.try_recv().unwrap(),
            format!("\r\n{history_line}\r\n\r\n\x1b[0;37;40m[ 31/45, 7/9 ] ")
        );
        assert!(target_rx.try_recv().is_err());

        handle_game_command(
            &broadcaster,
            sender_addr,
            &format!("{sender_name} 혼잣말 전음"),
            registry,
            room_cache,
            None,
        )
        .await
        .unwrap();
        let self_history = format!("{tag}{sender_name} : 혼잣말 ");
        assert_eq!(
            sender_rx.try_recv().unwrap(),
            format!("{tag}{sender_name}에게 보냄 : 혼잣말 \r\n")
        );
        assert_eq!(
            sender_rx.try_recv().unwrap(),
            format!("\r\n{self_history}\r\n\r\n\x1b[0;37;40m[ 50/60, 7/8 ] \r\n")
        );
        assert_eq!(
            sender_rx.try_recv().unwrap(),
            "\r\n\x1b[0;37;40m[ 50/60, 7/8 ] "
        );

        let clients = broadcaster.clients.lock();
        let target = clients
            .get(&target_addr)
            .and_then(|client| client.player.as_ref())
            .unwrap();
        assert_eq!(target.body.talk_history, vec![history_line]);
        assert!(matches!(
            target.body.temp().get(crate::script::TELL_TALKER_TOKEN),
            Some(crate::object::Value::String(token)) if token == &sender_token
        ));
        let sender = clients
            .get(&sender_addr)
            .and_then(|client| client.player.as_ref())
            .unwrap();
        assert_eq!(sender.body.talk_history, vec![self_history]);
        assert!(matches!(
            sender.body.temp().get(crate::script::TELL_TALKER_TOKEN),
            Some(crate::object::Value::String(token)) if token == &sender_token
        ));
        drop(clients);

        let mut world = get_world_state().write().unwrap();
        world.remove_player_position(sender_name);
        world.remove_player_position(target_name);
    }

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
    fn received_chunk_resets_the_authoritative_idle_clock() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let addr = "127.0.0.1:8081".parse().unwrap();
        let mut client = Client::new(addr, tx);
        client.last_input = Instant::now() - std::time::Duration::from_secs(20);
        let mut player = Player::new();
        player.idle = 20;
        client.player = Some(player);

        client.record_input();

        assert!(client.last_input.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(client.player.as_ref().unwrap().idle, 0);
    }

    #[test]
    fn login_states_use_the_same_idle_timeout_class_as_python() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let addr = "127.0.0.1:8082".parse().unwrap();
        let mut client = Client::new(addr, tx);
        assert!(client.uses_inactive_timeout());

        let session = client.login_session.as_mut().unwrap();
        session.state = LoginState::Notice;
        assert!(!client.uses_inactive_timeout());

        let session = client.login_session.as_mut().unwrap();
        session.state = LoginState::ScriptMode;
        session.script_mode = 2; // Python `무명객` sets sDOUMI.
        assert!(!client.uses_inactive_timeout());

        client.login_session.as_mut().unwrap().script_mode = 1;
        assert!(
            client.uses_inactive_timeout(),
            "Python `나만바라바` leaves state INACTIVE"
        );

        client.complete_login();
        assert!(!client.uses_inactive_timeout());
    }

    fn password_change_test_text() -> crate::command::handler::PasswordChangeText {
        crate::command::handler::PasswordChangeText {
            wrong_password: "wrong\r\n".to_string(),
            new_password_prompt: "new>".to_string(),
            confirm_prompt: "confirm>".to_string(),
            mismatch: "mismatch\r\n".to_string(),
            success: "success".to_string(),
        }
    }

    #[tokio::test]
    async fn password_change_preserves_python_raw_value_and_has_no_minimum_length() {
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let addr: SocketAddr = "127.0.0.1:18081".parse().unwrap();
        let mut client = Client::new(addr, tx);
        client.complete_login();
        let mut player = Player::new();
        player.body.set("이름", "암호흐름검사");
        player.body.set("암호", "old");
        client.player = Some(player);
        client.pending_input = Some(PendingInput::ChangePasswordOld {
            text: password_change_test_text(),
        });
        broadcaster.add_client(client);

        handle_pending_change_password(&broadcaster, addr, " old ")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "new>");
        assert!(matches!(
            broadcaster
                .clients
                .lock()
                .get(&addr)
                .unwrap()
                .pending_input
                .as_ref(),
            Some(PendingInput::ChangePasswordNew { .. })
        ));

        // Python change_password은 길이 제한을 두지 않고 입력값의 공백도 그대로 보존한다.
        handle_pending_change_password(&broadcaster, addr, "  ")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "confirm>");
        assert!(matches!(
            broadcaster
                .clients
                .lock()
                .get(&addr)
                .unwrap()
                .pending_input
                .as_ref(),
            Some(PendingInput::ChangePasswordConfirm {
                new_password,
                ..
            }) if new_password == "  "
        ));

        handle_pending_change_password(&broadcaster, addr, "  ")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "success");
        let clients = broadcaster.clients.lock();
        let client = clients.get(&addr).unwrap();
        assert!(client.pending_input.is_none());
        let stored = client.player.as_ref().unwrap().body.get_string("암호");
        assert!(stored.starts_with("$2"));
        assert!(password_verify(&stored, "  "));
        assert!(!password_verify(&stored, " "));
        assert!(
            rx.try_recv().is_err(),
            "Python flow does not append a game prompt"
        );
    }

    #[tokio::test]
    async fn password_change_rejects_wrong_old_password_and_mismatched_confirmation() {
        async fn setup(
            port: u16,
        ) -> (
            Arc<crate::network::Broadcaster>,
            SocketAddr,
            mpsc::UnboundedReceiver<String>,
        ) {
            let broadcaster = Arc::new(crate::network::Broadcaster::new());
            let (tx, rx) = mpsc::unbounded_channel();
            let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            let mut client = Client::new(addr, tx);
            client.complete_login();
            let mut player = Player::new();
            player.body.set("이름", "암호실패검사");
            player.body.set("암호", password_hash("old"));
            client.player = Some(player);
            client.pending_input = Some(PendingInput::ChangePasswordOld {
                text: password_change_test_text(),
            });
            broadcaster.add_client(client);
            (broadcaster, addr, rx)
        }

        let (wrong_server, wrong_addr, mut wrong_rx) = setup(18082).await;
        handle_pending_change_password(&wrong_server, wrong_addr, "bad")
            .await
            .unwrap();
        assert_eq!(wrong_rx.try_recv().unwrap(), "wrong\r\n");
        let wrong_clients = wrong_server.clients.lock();
        let wrong_client = wrong_clients.get(&wrong_addr).unwrap();
        assert!(wrong_client.pending_input.is_none());
        assert!(password_verify(
            &wrong_client
                .player
                .as_ref()
                .unwrap()
                .body
                .get_string("암호"),
            "old"
        ));
        drop(wrong_clients);

        let (mismatch_server, mismatch_addr, mut mismatch_rx) = setup(18083).await;
        handle_pending_change_password(&mismatch_server, mismatch_addr, " old ")
            .await
            .unwrap();
        assert_eq!(mismatch_rx.try_recv().unwrap(), "new>");
        handle_pending_change_password(&mismatch_server, mismatch_addr, "new value")
            .await
            .unwrap();
        assert_eq!(mismatch_rx.try_recv().unwrap(), "confirm>");
        handle_pending_change_password(&mismatch_server, mismatch_addr, "new value ")
            .await
            .unwrap();
        assert_eq!(mismatch_rx.try_recv().unwrap(), "mismatch\r\n");
        let mismatch_clients = mismatch_server.clients.lock();
        let mismatch_client = mismatch_clients.get(&mismatch_addr).unwrap();
        assert!(mismatch_client.pending_input.is_none());
        assert!(password_verify(
            &mismatch_client
                .player
                .as_ref()
                .unwrap()
                .body
                .get_string("암호"),
            "old"
        ));
    }

    fn note_edit_test_text() -> crate::command::handler::NoteEditText {
        crate::command::handler::NoteEditText {
            target_connected: "connected\r\n".to_string(),
            capacity_exceeded: "limit\r\n".to_string(),
            complete: "done\r\n".to_string(),
            continue_prompt: ":".to_string(),
        }
    }

    fn note_edit_test_state(
        path: &std::path::Path,
        target_name: &str,
        memo_body: &str,
    ) -> PendingInput {
        let mut target = crate::player::Body::new();
        target.set("이름", target_name);
        target.set("마지막저장시간", 77);
        target.memos.insert(
            "메모:보낸이".to_string(),
            crate::player::MemoRecord {
                제목: "제목".to_string(),
                시간: "2026-07-10 12:34:56".to_string(),
                작성자: "보낸이".to_string(),
                내용: String::new(),
            },
        );
        assert!(crate::script::save_body_to_json_without_timestamp(
            &mut target,
            &path.to_string_lossy()
        ));
        PendingInput::NoteEdit {
            recipient: crate::command::handler::NoteRecipientState {
                target_name: target_name.to_string(),
                save_path: path.to_string_lossy().into_owned(),
                body: Arc::new(std::sync::Mutex::new(target)),
            },
            body: memo_body.to_string(),
            text: note_edit_test_text(),
        }
    }

    fn note_test_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "muc_note_client_{label}_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[tokio::test]
    async fn note_edit_preserves_python_prompt_dot_and_body_save_flow() {
        let path = note_test_path("dot");
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let addr: SocketAddr = "127.0.0.1:18082".parse().unwrap();
        let mut client = Client::new(addr, tx);
        client.complete_login();
        let mut player = Player::new();
        player.body.set("이름", "보낸이");
        client.player = Some(player);
        client.pending_input = Some(note_edit_test_state(&path, "오프라인수신자", ""));
        broadcaster.add_client(client);

        handle_pending_change_password(&broadcaster, addr, "첫줄")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), ":");
        assert!(matches!(
            broadcaster
                .clients
                .lock()
                .get(&addr)
                .unwrap()
                .pending_input
                .as_ref(),
            Some(PendingInput::NoteEdit { body, .. }) if body == "첫줄"
        ));

        handle_pending_change_password(&broadcaster, addr, ".")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "done\r\n");
        assert!(broadcaster
            .clients
            .lock()
            .get(&addr)
            .unwrap()
            .pending_input
            .is_none());
        let mut saved = crate::player::Body::new();
        assert!(crate::script::load_body_from_json(
            &mut saved,
            &path.to_string_lossy()
        ));
        assert_eq!(saved.memos["메모:보낸이"].내용, "첫줄");
        assert_eq!(saved.get_int("마지막저장시간"), 77);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn note_edit_limit_discards_current_input_and_reconnect_keeps_empty_reservation() {
        let limit_path = note_test_path("limit");
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let addr: SocketAddr = "127.0.0.1:18083".parse().unwrap();
        let mut client = Client::new(addr, tx);
        client.complete_login();
        let mut player = Player::new();
        player.body.set("이름", "보낸이");
        client.player = Some(player);
        client.pending_input = Some(note_edit_test_state(
            &limit_path,
            "제한수신자",
            "가나다라마바사아자차",
        ));
        broadcaster.add_client(client);

        handle_pending_change_password(&broadcaster, addr, "이줄은버림")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "limit\r\ndone\r\n");
        let mut limited = crate::player::Body::new();
        assert!(crate::script::load_body_from_json(
            &mut limited,
            &limit_path.to_string_lossy()
        ));
        assert_eq!(limited.memos["메모:보낸이"].내용, "가나다라마바사아자차");

        let connected_path = note_test_path("connected");
        {
            let mut clients = broadcaster.clients.lock();
            let sender = clients.get_mut(&addr).unwrap();
            sender.pending_input = Some(note_edit_test_state(
                &connected_path,
                "재접속수신자",
                "본문",
            ));
        }
        let (target_tx, _target_rx) = mpsc::unbounded_channel();
        let target_addr: SocketAddr = "127.0.0.1:18084".parse().unwrap();
        let mut target_client = Client::new(target_addr, target_tx);
        // Python channel.players 호환성: Active가 아닌 연결도 이름이 같으면 중단.
        let mut target_player = Player::new();
        target_player.body.set("이름", "재접속수신자");
        target_client.player = Some(target_player);
        broadcaster.add_client(target_client);

        while rx.try_recv().is_ok() {}
        handle_pending_change_password(&broadcaster, addr, ".")
            .await
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "connected\r\n");
        let mut connected = crate::player::Body::new();
        assert!(crate::script::load_body_from_json(
            &mut connected,
            &connected_path.to_string_lossy()
        ));
        assert_eq!(connected.memos["메모:보낸이"].내용, "");

        let _ = std::fs::remove_file(limit_path);
        let _ = std::fs::remove_file(connected_path);
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

    #[test]
    fn user_alias_expands_last_command_token_and_star_like_python() {
        let aliases = HashMap::from([("공".to_string(), "* 쳐".to_string())]);
        assert_eq!(
            expand_user_alias("왕 대협 공", &aliases),
            Some(vec!["왕 대협 쳐".to_string()])
        );
        assert_eq!(
            expand_user_alias("공 왕 대협", &aliases),
            None,
            "첫 토큰은 사용자 줄임말 명령으로 보지 않는다"
        );
    }

    #[test]
    fn note_parameter_uses_python_rstrip_instead_of_first_command_substring() {
        assert_eq!(
            python_command_parameter("쪽지왕  여러 단어 제목 쪽지", "쪽지"),
            "쪽지왕  여러 단어 제목"
        );
        assert_eq!(
            python_command_parameter("수신자 제목 쪽지", "쪽지"),
            "수신자 제목"
        );
    }

    #[test]
    fn room_local_commands_do_not_request_global_player_snapshots() {
        for command in [
            "말",
            "봐",
            "소지품",
            "무공",
            "시전",
            "귀환",
            "동",
            "북서",
            "초보수련장",
        ] {
            assert_eq!(
                global_player_snapshot_needs(command),
                GlobalPlayerSnapshotNeeds::default(),
                "{command}"
            );
        }

        assert!(global_player_snapshot_needs("누구").details);
        assert!(global_player_snapshot_needs("무림별호").details);
        for command in ["순위", "비교", "트윗", "외쳐", "외쳐2"] {
            assert!(global_player_snapshot_needs(command).details, "{command}");
        }
        assert!(global_player_snapshot_needs("외쳐").online_names);
        for command in [
            "트윗",
            "외쳐",
            "외쳐2",
            "방파말",
            "똥파말",
            "방파별호",
            "방파파문",
            "방주권한양도",
            "직위임명",
            "명칭설정",
            "도망",
            "무림별호",
        ] {
            assert!(global_snapshot_includes_transparent(command), "{command}");
        }
        assert!(!global_snapshot_includes_transparent("누구"));
        assert!(global_player_snapshot_needs("쪽지").connected_names);
        for command in ["전음", "반전음"] {
            let needs = global_player_snapshot_needs(command);
            assert!(needs.tell_players, "{command}");
            assert!(
                !needs.details,
                "{command} must not build detailed user maps"
            );
            assert!(!needs.online_names, "{command}");
            assert!(!needs.connected_names, "{command}");
        }
        assert_eq!(
            global_player_snapshot_needs("알 수 없음"),
            GlobalPlayerSnapshotNeeds::default(),
            "Python never retries the first token as a command"
        );
    }

    #[test]
    fn user_alias_preserves_semicolon_followup_order() {
        let aliases = HashMap::from([("연속".to_string(), "* 쳐;봐;* 전음".to_string())]);
        assert_eq!(
            expand_user_alias("왕 연속", &aliases),
            Some(vec![
                "왕 쳐".to_string(),
                "봐".to_string(),
                "왕 전음".to_string(),
            ])
        );
    }

    #[test]
    fn automatic_route_preserves_python_split_empty_segments_and_whitespace() {
        assert!(python_auto_move_route("").is_empty());
        assert_eq!(
            python_auto_move_route("동; 서 ;;북 "),
            vec!["동", " 서 ", "", "북 "]
        );
        assert_eq!(python_auto_move_route(";"), vec!["", ""]);
    }

    #[test]
    fn user_alias_ignores_surrounding_space_but_not_sentence_punctuation() {
        let aliases = HashMap::from([
            ("n".to_string(), "봐".to_string()),
            ("말".to_string(), "북".to_string()),
            (",".to_string(), "외쳐".to_string()),
        ]);
        assert_eq!(
            expand_user_alias("n", &aliases),
            Some(vec!["봐".to_string()])
        );
        assert_eq!(
            expand_user_alias("말 ", &aliases),
            Some(vec!["북".to_string()])
        );
        assert_eq!(expand_user_alias("안녕!", &aliases), None);
        assert_eq!(
            expand_user_alias(",", &aliases),
            Some(vec!["외쳐".to_string()]),
            "Python 말하기 판정에는 쉼표가 포함되지 않는다"
        );
    }

    #[test]
    fn direction_with_arguments_is_not_reinterpreted_as_movement() {
        assert!(direction_with_arguments_is_not_a_command("동", 2));
        assert!(direction_with_arguments_is_not_a_command("북동", 3));
        assert!(!direction_with_arguments_is_not_a_command("동", 1));
        assert!(!direction_with_arguments_is_not_a_command("맵", 2));
    }

    #[test]
    fn expanded_alias_first_command_does_not_repeat_say_preprocessing() {
        let bang = parse_expanded_user_alias("!");
        assert_eq!(bang.command, "!");
        assert!(bang.args.is_empty());

        let sentence = parse_expanded_user_alias("안녕!");
        assert_eq!(sentence.command, "안녕!");
        assert_ne!(sentence.command, "말");
    }

    #[tokio::test]
    async fn movement_notifies_room_observers_and_enqueues_same_room_followers_fifo() {
        let script_storage = Arc::new(tokio::sync::RwLock::new(crate::script::ScriptStorage::new(
            crate::script::ScriptConfig::default(),
        )));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        crate::command::commands::script::register_script_commands(
            &mut registry,
            script_storage,
            None,
            None,
            None,
        )
        .await;
        let registry = Arc::new(registry);
        let room_cache = Arc::new(std::sync::Mutex::new(RoomCache::new()));
        let broadcaster = Arc::new(crate::network::Broadcaster::new());

        let leader_addr: SocketAddr = "127.0.0.1:18101".parse().unwrap();
        let follower_addr: SocketAddr = "127.0.0.1:18102".parse().unwrap();
        let source_observer_addr: SocketAddr = "127.0.0.1:18103".parse().unwrap();
        let destination_observer_addr: SocketAddr = "127.0.0.1:18104".parse().unwrap();
        let (leader_tx, mut leader_rx) = mpsc::unbounded_channel();
        let (follower_tx, mut follower_rx) = mpsc::unbounded_channel();
        let (source_tx, mut source_rx) = mpsc::unbounded_channel();
        let (destination_tx, mut destination_rx) = mpsc::unbounded_channel();

        let create_client =
            |addr: SocketAddr, sender: mpsc::UnboundedSender<String>, name: &str| {
                let mut client = Client::new(addr, sender);
                client.complete_login();
                let mut player = Player::new();
                player.state = STATE_ACTIVE;
                player.interactive = 1;
                player.body.set("이름", name);
                player.body.set("레벨", 1_i64);
                player.body.set("체력", 100_i64);
                player.body.set("최고체력", 100_i64);
                player.body.set("내공", 10_i64);
                player.body.set("최고내공", 10_i64);
                player.body.set("설정상태", "간략설명 1\n엘피출력 0");
                client.player = Some(player);
                client
            };

        let leader_name = "이동대장";
        let follower_name = "이동추종자";
        let source_observer_name = "출발방목격자";
        let destination_observer_name = "도착방목격자";
        let leader_client = create_client(leader_addr, leader_tx, leader_name);
        let leader_token = leader_client.connection_token.clone();
        let follower_client = create_client(follower_addr, follower_tx, follower_name);
        let follower_token = follower_client.connection_token.clone();
        broadcaster.add_client(leader_client);
        broadcaster.add_client(follower_client);
        broadcaster.add_client(create_client(
            source_observer_addr,
            source_tx,
            source_observer_name,
        ));
        broadcaster.add_client(create_client(
            destination_observer_addr,
            destination_tx,
            destination_observer_name,
        ));
        assert!(broadcaster.apply_social_action(
            &follower_token,
            SocialAction::Follow {
                target: leader_token,
            },
        ));

        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("산동성", "5").unwrap();
            world.room_cache.get_room("산동성", "6").unwrap();
            for name in [leader_name, follower_name, source_observer_name] {
                world.set_player_position(
                    name,
                    PlayerPosition::new("산동성".to_string(), "5".to_string()),
                );
            }
            world.set_player_position(
                destination_observer_name,
                PlayerPosition::new("산동성".to_string(), "6".to_string()),
            );
        }

        handle_game_command(&broadcaster, leader_addr, "동", registry, room_cache, None)
            .await
            .unwrap();

        {
            let world = get_world_state().read().unwrap();
            for name in [leader_name, follower_name] {
                let position = world.get_player_position(name).unwrap();
                assert_eq!(
                    (position.zone.as_str(), position.room.as_str()),
                    ("산동성", "6")
                );
            }
            assert_eq!(
                world.get_players_in_room("산동성", "5"),
                vec![source_observer_name.to_string()]
            );
        }

        let drain = |receiver: &mut mpsc::UnboundedReceiver<String>| {
            let mut output = String::new();
            while let Ok(message) = receiver.try_recv() {
                output.push_str(&message);
            }
            output
        };
        let leader_output = drain(&mut leader_rx);
        let follower_output = drain(&mut follower_rx);
        let source_output = drain(&mut source_rx);
        let destination_output = drain(&mut destination_rx);
        assert!(leader_output.contains("산동성 성도"));
        assert!(follower_output.contains("산동성 성도"));
        assert_eq!(source_output.matches("동쪽으로 갔습니다.").count(), 2);
        assert_eq!(destination_output.matches("왔습니다.").count(), 2);
        assert_eq!(source_output.matches("[ 100/100, 10/10 ] ").count(), 2);
        assert_eq!(destination_output.matches("[ 100/100, 10/10 ] ").count(), 2);
        assert!(source_output.ends_with("[ 100/100, 10/10 ] "));
        assert!(destination_output.ends_with("[ 100/100, 10/10 ] "));
        assert!(
            !follower_output.contains("동쪽으로 갔습니다."),
            "leader exit notification must exclude followers before their queued move"
        );

        let mut world = get_world_state().write().unwrap();
        for name in [
            leader_name,
            follower_name,
            source_observer_name,
            destination_observer_name,
        ] {
            world.remove_player_position(name);
        }
    }

    #[tokio::test]
    async fn four_clients_keep_follow_party_chat_and_leader_logout_object_order() {
        let script_storage = Arc::new(tokio::sync::RwLock::new(crate::script::ScriptStorage::new(
            crate::script::ScriptConfig::default(),
        )));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        crate::command::commands::script::register_script_commands(
            &mut registry,
            script_storage,
            None,
            None,
            None,
        )
        .await;
        let registry = Arc::new(registry);
        let room_cache = Arc::new(std::sync::Mutex::new(RoomCache::new()));
        let broadcaster = Arc::new(crate::network::Broadcaster::new());

        let leader_addr: SocketAddr = "127.0.0.1:18111".parse().unwrap();
        let first_addr: SocketAddr = "127.0.0.1:18112".parse().unwrap();
        let second_addr: SocketAddr = "127.0.0.1:18113".parse().unwrap();
        let observer_addr: SocketAddr = "127.0.0.1:18114".parse().unwrap();
        let (leader_tx, mut leader_rx) = mpsc::unbounded_channel();
        let (first_tx, mut first_rx) = mpsc::unbounded_channel();
        let (second_tx, mut second_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();

        let create_client =
            |addr: SocketAddr, sender: mpsc::UnboundedSender<String>, name: &str, hp: i64| {
                let mut client = Client::new(addr, sender);
                client.complete_login();
                let mut player = Player::new();
                player.state = STATE_ACTIVE;
                player.interactive = 1;
                player.body.set("이름", name);
                player.body.set("성격", "정파");
                player.body.set("체력", hp);
                player.body.set("최고체력", 100_i64);
                player.body.set("내공", 10_i64);
                player.body.set("최고내공", 20_i64);
                player.body.set("설정상태", "엘피출력 0\n동행거부 0");
                client.player = Some(player);
                client
            };

        let leader_name = "파티통합대장";
        let first_name = "파티통합첫째";
        let second_name = "파티통합둘째";
        let observer_name = "파티통합목격자";
        let leader_client = create_client(leader_addr, leader_tx, leader_name, 100);
        let leader_token = leader_client.connection_token.clone();
        let first_client = create_client(first_addr, first_tx, first_name, 80);
        let first_token = first_client.connection_token.clone();
        let second_client = create_client(second_addr, second_tx, second_name, 60);
        let second_token = second_client.connection_token.clone();
        broadcaster.add_client(leader_client);
        broadcaster.add_client(first_client);
        broadcaster.add_client(second_client);
        broadcaster.add_client(create_client(observer_addr, observer_tx, observer_name, 40));

        {
            let mut world = get_world_state().write().unwrap();
            for name in [leader_name, first_name, second_name, observer_name] {
                world.set_player_position(
                    name,
                    PlayerPosition::new("파티통합존".to_string(), "1".to_string()),
                );
            }
        }

        for (addr, name) in [(first_addr, first_name), (second_addr, second_name)] {
            handle_game_command(
                &broadcaster,
                addr,
                &format!("{leader_name} 따라"),
                registry.clone(),
                room_cache.clone(),
                None,
            )
            .await
            .unwrap_or_else(|error| panic!("{name} follow failed: {error}"));
        }
        assert_eq!(
            broadcaster.social_snapshot(&leader_token).followers,
            vec![first_token.clone(), second_token.clone()]
        );

        handle_game_command(
            &broadcaster,
            leader_addr,
            "모두 무리",
            registry.clone(),
            room_cache.clone(),
            None,
        )
        .await
        .unwrap();
        let party = broadcaster.social_snapshot(&leader_token);
        assert_eq!(party.party_leader.as_deref(), Some(leader_token.as_str()));
        assert_eq!(
            party.party_members,
            vec![first_token.clone(), second_token.clone()]
        );

        handle_game_command(
            &broadcaster,
            first_addr,
            "함께가자 무리말",
            registry.clone(),
            room_cache,
            None,
        )
        .await
        .unwrap();

        let drain = |receiver: &mut mpsc::UnboundedReceiver<String>| {
            let mut output = String::new();
            while let Ok(message) = receiver.try_recv() {
                output.push_str(&message);
            }
            output
        };
        let leader_before_logout = drain(&mut leader_rx);
        let first_before_logout = drain(&mut first_rx);
        let second_before_logout = drain(&mut second_rx);
        let observer_before_logout = drain(&mut observer_rx);
        assert!(leader_before_logout.contains("함께가자"));
        assert!(first_before_logout.contains("◁"));
        assert!(second_before_logout.contains("함께가자"));
        assert_eq!(
            observer_before_logout
                .matches("의 무리에 들어갑니다.")
                .count(),
            2
        );

        leave_party_on_disconnect(&broadcaster, leader_addr, &registry);
        let first_after = broadcaster.social_snapshot(&first_token);
        let second_after = broadcaster.social_snapshot(&second_token);
        assert_eq!(
            first_after.party_leader.as_deref(),
            Some(first_token.as_str())
        );
        assert_eq!(first_after.party_members, vec![second_token.clone()]);
        assert_eq!(first_after.follow, None);
        assert_eq!(first_after.followers, vec![second_token.clone()]);
        assert_eq!(
            second_after.party_leader.as_deref(),
            Some(first_token.as_str())
        );
        assert_eq!(second_after.follow, None);

        let leader_logout = drain(&mut leader_rx);
        let first_logout = drain(&mut first_rx);
        let second_logout = drain(&mut second_rx);
        let observer_logout = drain(&mut observer_rx);
        assert!(leader_logout.contains("당신과 따라다니는 것을 그만둡니다."));
        assert!(first_logout.contains("당신은 무리의 대장으로 변경 되었습니다."));
        assert!(first_logout.contains("따라다니는 것을 그만둡니다."));
        assert!(second_logout.contains("무리의 대장으로 변경 되었습니다."));
        assert!(second_logout.contains("따라다니는 것을 그만둡니다."));
        assert!(observer_logout.is_empty());

        let mut world = get_world_state().write().unwrap();
        for name in [leader_name, first_name, second_name, observer_name] {
            world.remove_player_position(name);
        }
    }

    #[tokio::test]
    async fn admin_give_item_uses_python_reset_and_does_not_notify_room() {
        use crate::object::Object;

        let script_storage = Arc::new(tokio::sync::RwLock::new(crate::script::ScriptStorage::new(
            crate::script::ScriptConfig::default(),
        )));
        let mut registry = CommandRegistry::new();
        crate::command::commands::register_basic_commands(&mut registry);
        crate::command::commands::script::register_script_commands(
            &mut registry,
            script_storage,
            None,
            None,
            None,
        )
        .await;
        let registry = Arc::new(registry);
        let room_cache = Arc::new(std::sync::Mutex::new(RoomCache::new()));
        let broadcaster = Arc::new(crate::network::Broadcaster::new());
        let suffix = std::process::id();
        let giver_name = format!("관리지급자{suffix}");
        let target_name = format!("관리수령자{suffix}");
        let observer_name = format!("관리목격자{suffix}");
        let giver_addr: SocketAddr = "127.0.0.1:18431".parse().unwrap();
        let target_addr: SocketAddr = "127.0.0.1:18432".parse().unwrap();
        let observer_addr: SocketAddr = "127.0.0.1:18433".parse().unwrap();
        let (giver_tx, mut giver_rx) = mpsc::unbounded_channel();
        let (target_tx, mut target_rx) = mpsc::unbounded_channel();
        let (observer_tx, mut observer_rx) = mpsc::unbounded_channel();

        let make_client = |addr, sender, name: &str, admin: i64| {
            let mut client = Client::new(addr, sender);
            client.complete_login();
            let mut player = Player::new();
            player.state = STATE_ACTIVE;
            player.interactive = 1;
            for (key, value) in [
                ("관리자등급", admin),
                ("힘", 100),
                ("체력", 100),
                ("최고체력", 100),
                ("내공", 10),
                ("최고내공", 10),
            ] {
                player.body.set(key, value);
            }
            player.body.set("이름", name);
            player.body.set("설정상태", "엘피출력 0");
            client.player = Some(player);
            client
        };
        let mut giver = make_client(giver_addr, giver_tx, &giver_name, 2000);
        let mut item = Object::new();
        item.set("이름", "청옥패");
        item.set("반응이름", "옥패");
        item.set("아이템속성", "줄수없음");
        giver
            .player
            .as_mut()
            .unwrap()
            .body
            .object
            .append(Arc::new(std::sync::Mutex::new(item)));
        let mut heavy = Object::new();
        heavy.set("이름", "무거운패");
        heavy.set("반응이름", "무거운패확장");
        heavy.temp.insert(
            "_python_json_array:반응이름".into(),
            crate::object::Value::Int(1),
        );
        heavy.set("아이템속성", "줄수없음확장");
        heavy.temp.insert(
            "_python_json_array:아이템속성".into(),
            crate::object::Value::Int(1),
        );
        heavy.set("무게", 2_000_i64);
        giver
            .player
            .as_mut()
            .unwrap()
            .body
            .object
            .append(Arc::new(std::sync::Mutex::new(heavy)));
        broadcaster.add_client(giver);
        broadcaster.add_client(make_client(target_addr, target_tx, &target_name, 1000));
        broadcaster.add_client(make_client(observer_addr, observer_tx, &observer_name, 0));
        {
            let mut world = get_world_state().write().unwrap();
            world.room_cache.get_room("낙양성", "42").unwrap();
            for name in [&giver_name, &target_name, &observer_name] {
                world.set_player_position(
                    name,
                    PlayerPosition::new("낙양성".to_string(), "42".to_string()),
                );
            }
        }

        handle_game_command(
            &broadcaster,
            giver_addr,
            &format!("{target_name} 무거운 줘"),
            registry.clone(),
            room_cache.clone(),
            None,
        )
        .await
        .unwrap();
        let array_membership_output = {
            let mut output = String::new();
            while let Ok(message) = giver_rx.try_recv() {
                output.push_str(&message);
            }
            output
        };
        assert!(array_membership_output.contains("그런 아이템이 소지품에 없어요."));

        handle_game_command(
            &broadcaster,
            giver_addr,
            &format!("{target_name} 무거운패 줘"),
            registry.clone(),
            room_cache.clone(),
            None,
        )
        .await
        .unwrap();
        let ordinary_output = {
            let mut output = String::new();
            while let Ok(message) = giver_rx.try_recv() {
                output.push_str(&message);
            }
            output
        };
        assert!(ordinary_output.contains("무거워서 받지 못합니다."));
        assert_eq!(
            broadcaster.clients.lock()[&giver_addr]
                .player
                .as_ref()
                .unwrap()
                .body
                .object
                .objs
                .len(),
            2,
            "ordinary 줘 must not exempt an administrator recipient"
        );

        {
            let mut clients = broadcaster.clients.lock();
            clients
                .get_mut(&giver_addr)
                .unwrap()
                .player
                .as_mut()
                .unwrap()
                .body
                .object
                .inv_stack
                .insert("비황석".to_string(), 2);
        }
        handle_game_command(
            &broadcaster,
            giver_addr,
            &format!("{target_name} 비황석 2 줘"),
            registry.clone(),
            room_cache.clone(),
            None,
        )
        .await
        .unwrap();
        let ordinary_stack_giver = {
            let mut output = String::new();
            while let Ok(message) = giver_rx.try_recv() {
                output.push_str(&message);
            }
            output
        };
        let ordinary_stack_target = {
            let mut output = String::new();
            while let Ok(message) = target_rx.try_recv() {
                output.push_str(&message);
            }
            output
        };
        let ordinary_stack_observer = {
            let mut output = String::new();
            while let Ok(message) = observer_rx.try_recv() {
                output.push_str(&message);
            }
            output
        };
        assert!(ordinary_stack_giver.contains("비황석\x1b[37m 2개를 줍니다."));
        assert!(ordinary_stack_target.contains("비황석\x1b[37m 2개를 줍니다."));
        assert!(ordinary_stack_observer.contains("비황석\x1b[37m 2개를 줍니다."));
        {
            let clients = broadcaster.clients.lock();
            let giver_body = &clients[&giver_addr].player.as_ref().unwrap().body;
            let target_body = &clients[&target_addr].player.as_ref().unwrap().body;
            assert_eq!(giver_body.object.inv_stack.get("비황석"), None);
            assert_eq!(target_body.object.inv_stack.get("비황석"), Some(&2));
        }

        handle_game_command(
            &broadcaster,
            giver_addr,
            &format!("{target_name} 옥패 줘줘"),
            registry,
            room_cache,
            None,
        )
        .await
        .unwrap();

        let drain = |receiver: &mut mpsc::UnboundedReceiver<String>| {
            let mut output = String::new();
            while let Ok(message) = receiver.try_recv() {
                output.push_str(&message);
            }
            output
        };
        let giver_output = drain(&mut giver_rx);
        let target_output = drain(&mut target_rx);
        let observer_output = drain(&mut observer_rx);
        assert!(
            giver_output.contains(&format!(
                "당신이 \x1b[1m{target_name}\x1b[0m에게 \x1b[36m\x1b[33m청옥패\x1b[37m를\x1b[37m 줍니다."
            )),
            "giver output: {giver_output:?}"
        );
        assert!(target_output.contains(&format!(
            "\x1b[1m{giver_name}\x1b[0m{} 당신에게 \x1b[36m\x1b[33m청옥패\x1b[37m를\x1b[37m 줍니다.",
            crate::hangul::han_iga(&giver_name)
        )));
        assert!(!observer_output.contains("청옥패"));
        let clients = broadcaster.clients.lock();
        assert_eq!(
            clients
                .get(&target_addr)
                .unwrap()
                .player
                .as_ref()
                .unwrap()
                .body
                .object
                .objs
                .iter()
                .filter(|item| item.lock().is_ok_and(|item| item.getName() == "청옥패"))
                .count(),
            1
        );
        drop(clients);
        let mut world = get_world_state().write().unwrap();
        for name in [&giver_name, &target_name, &observer_name] {
            world.remove_player_position(name);
            let _ = std::fs::remove_file(format!("data/user/{name}.json"));
        }
    }
}
