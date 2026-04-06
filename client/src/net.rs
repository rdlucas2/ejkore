use matchbox_socket::{PeerId, WebRtcSocket};

/// Packet layout: [msg_type: u8][payload...]
/// Type 0x01 = InputBundle: [frame: u32 LE][input0: u16][input1: u16][input2: u16] = 11 bytes
///   input0 = latest_frame, input1 = latest_frame-1, input2 = latest_frame-2 (redundancy)
/// Type 0x02 = CharSelect: [char_id: u8][ready: u8] = 3 bytes
/// Type 0x03 = Checksum:   [frame: u32 LE][hash: u64 LE] = 13 bytes
/// Type 0x04 = StartGame:  [local_player: u8] = 2 bytes

pub const MSG_INPUT: u8 = 0x01;
pub const MSG_CHAR_SELECT: u8 = 0x02;
pub const MSG_CHECKSUM: u8 = 0x03;
pub const MSG_START_GAME: u8 = 0x04;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionState {
    Connecting,
    WaitingForPeer,
    Connected,
    Disconnected,
}

pub struct NetworkManager {
    socket: WebRtcSocket,
    pub peer: Option<PeerId>,
    pub connection_state: ConnectionState,
}

impl NetworkManager {
    pub fn new(signaling_url: &str) -> Self {
        let (socket, loop_fut) = WebRtcSocket::new_unreliable(signaling_url);
        wasm_bindgen_futures::spawn_local(async move {
            let _ = loop_fut.await;
        });
        Self {
            socket,
            peer: None,
            connection_state: ConnectionState::Connecting,
        }
    }

    /// Poll for connection changes and incoming messages.
    /// Call this once per frame.
    pub fn update(&mut self) {
        // Check for new/disconnected peers
        let changes = self.socket.update_peers();
        for (peer, state) in changes {
            match state {
                matchbox_socket::PeerState::Connected => {
                    self.peer = Some(peer);
                    self.connection_state = ConnectionState::Connected;
                }
                matchbox_socket::PeerState::Disconnected => {
                    if self.peer == Some(peer) {
                        self.peer = None;
                        self.connection_state = ConnectionState::Disconnected;
                    }
                }
            }
        }

        // Update state if no peer yet
        if self.peer.is_none() && self.connection_state == ConnectionState::Connecting {
            self.connection_state = ConnectionState::WaitingForPeer;
        }
    }

    /// Receive all pending messages. Returns (msg_type, payload) pairs.
    pub fn receive(&mut self) -> Vec<(u8, Vec<u8>)> {
        let mut messages = Vec::new();
        for (_peer, data) in self.socket.receive() {
            if let Some(&msg_type) = data.first() {
                messages.push((msg_type, data[1..].to_vec()));
            }
        }
        messages
    }

    /// Send raw bytes to the connected peer.
    pub fn send(&mut self, data: Vec<u8>) {
        if let Some(peer) = self.peer {
            self.socket.send(data.into(), peer);
        }
    }

    /// Send an input bundle (current frame + 2 previous for redundancy).
    pub fn send_input(&mut self, frame: u32, inputs: [u16; 3]) {
        let mut buf = Vec::with_capacity(11);
        buf.push(MSG_INPUT);
        buf.extend_from_slice(&frame.to_le_bytes());
        buf.extend_from_slice(&inputs[0].to_le_bytes());
        buf.extend_from_slice(&inputs[1].to_le_bytes());
        buf.extend_from_slice(&inputs[2].to_le_bytes());
        self.send(buf);
    }

    /// Send character selection state.
    pub fn send_char_select(&mut self, char_id: u8, ready: bool) {
        self.send(vec![MSG_CHAR_SELECT, char_id, ready as u8]);
    }

    /// Send a checksum for desync detection.
    pub fn send_checksum(&mut self, frame: u32, hash: u64) {
        let mut buf = Vec::with_capacity(13);
        buf.push(MSG_CHECKSUM);
        buf.extend_from_slice(&frame.to_le_bytes());
        buf.extend_from_slice(&hash.to_le_bytes());
        self.send(buf);
    }

    /// Send game start signal with player assignment.
    pub fn send_start_game(&mut self, remote_player_idx: u8) {
        self.send(vec![MSG_START_GAME, remote_player_idx]);
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
}

/// Parse an input bundle message payload.
/// Returns (frame, [input0, input1, input2]) where input0 is the latest frame.
pub fn parse_input_bundle(payload: &[u8]) -> Option<(u32, [u16; 3])> {
    if payload.len() < 10 {
        return None;
    }
    let frame = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let i0 = u16::from_le_bytes([payload[4], payload[5]]);
    let i1 = u16::from_le_bytes([payload[6], payload[7]]);
    let i2 = u16::from_le_bytes([payload[8], payload[9]]);
    Some((frame, [i0, i1, i2]))
}

/// Parse a checksum message payload.
pub fn parse_checksum(payload: &[u8]) -> Option<(u32, u64)> {
    if payload.len() < 12 {
        return None;
    }
    let frame = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let hash = u64::from_le_bytes([
        payload[4], payload[5], payload[6], payload[7],
        payload[8], payload[9], payload[10], payload[11],
    ]);
    Some((frame, hash))
}
