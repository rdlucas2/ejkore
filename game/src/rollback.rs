use crate::state::*;

pub const INPUT_DELAY: u32 = 2;
pub const MAX_ROLLBACK_FRAMES: u32 = 8;
const BUFFER_SIZE: usize = 128;

fn idx(frame: u32) -> usize {
    frame as usize % BUFFER_SIZE
}

struct InputBuffer {
    inputs: [[PlayerInput; MAX_PLAYERS]; BUFFER_SIZE],
    confirmed: [[bool; MAX_PLAYERS]; BUFFER_SIZE],
}

impl InputBuffer {
    fn new() -> Self {
        Self {
            inputs: [[PlayerInput::default(); MAX_PLAYERS]; BUFFER_SIZE],
            confirmed: [[false; MAX_PLAYERS]; BUFFER_SIZE],
        }
    }

    fn set(&mut self, frame: u32, player: usize, input: PlayerInput, is_confirmed: bool) {
        let i = idx(frame);
        self.inputs[i][player] = input;
        self.confirmed[i][player] = is_confirmed;
    }

    fn get(&self, frame: u32) -> [PlayerInput; MAX_PLAYERS] {
        self.inputs[idx(frame)]
    }

    fn is_confirmed(&self, frame: u32, player: usize) -> bool {
        self.confirmed[idx(frame)][player]
    }

    fn predict(&mut self, frame: u32, player: usize) {
        if frame > 0 && !self.confirmed[idx(frame)][player] {
            self.inputs[idx(frame)][player] = self.inputs[idx(frame - 1)][player];
        }
    }
}

struct SnapshotBuffer {
    // snapshots[f] = game state at the START of frame f (before advance_frame)
    states: Vec<GameState>,
}

impl SnapshotBuffer {
    fn new(initial: GameState) -> Self {
        Self {
            states: vec![initial; BUFFER_SIZE],
        }
    }

    fn save(&mut self, frame: u32, state: &GameState) {
        self.states[idx(frame)] = *state;
    }

    fn load(&self, frame: u32) -> GameState {
        self.states[idx(frame)]
    }
}

pub struct RollbackManager {
    /// The next frame to simulate.
    pub current_frame: u32,
    /// The last frame where all players' inputs are confirmed.
    pub last_confirmed_frame: u32,
    /// Which player index we are (0 or 1).
    pub local_player: usize,
    /// Current game state.
    pub state: GameState,
    /// How many frames were re-simulated in the last update.
    pub last_rollback_count: u32,
    /// Whether a desync has been detected.
    pub desync_detected: bool,

    inputs: InputBuffer,
    snapshots: SnapshotBuffer,
}

impl RollbackManager {
    pub fn new(initial_state: GameState, local_player: usize) -> Self {
        let mut snapshots = SnapshotBuffer::new(initial_state);
        snapshots.save(0, &initial_state);
        Self {
            current_frame: 0,
            last_confirmed_frame: 0,
            local_player,
            state: initial_state,
            last_rollback_count: 0,
            desync_detected: false,
            inputs: InputBuffer::new(),
            snapshots,
        }
    }

    /// Add the local player's input. Scheduled INPUT_DELAY frames ahead.
    /// Returns the target frame number.
    pub fn add_local_input(&mut self, input: PlayerInput) -> u32 {
        let target = self.current_frame + INPUT_DELAY;
        self.inputs.set(target, self.local_player, input, true);
        target
    }

    /// Add a remote player's confirmed input for a specific frame.
    /// Triggers rollback if frame is in the past and prediction was wrong.
    pub fn add_remote_input(&mut self, frame: u32, input: PlayerInput) {
        let remote = 1 - self.local_player;
        let old_input = self.inputs.get(frame)[remote];
        let was_predicted = !self.inputs.is_confirmed(frame, remote);

        self.inputs.set(frame, remote, input, true);

        if frame < self.current_frame && was_predicted && old_input != input {
            self.rollback_to(frame);
        }

        self.update_confirmed_frame();
    }

    /// Check a remote checksum against local for desync detection.
    pub fn check_remote_checksum(&mut self, frame: u32, remote_checksum: u64) {
        if frame < self.current_frame {
            let local = state_checksum(&self.snapshots.load(frame));
            if local != remote_checksum {
                self.desync_detected = true;
            }
        }
    }

    /// Get the checksum for a confirmed frame (for sending to remote).
    pub fn checksum_for_frame(&self, frame: u32) -> u64 {
        state_checksum(&self.snapshots.load(frame))
    }

    /// Advance simulation by one frame. Predicts missing inputs.
    pub fn advance(&mut self) -> &GameState {
        self.last_rollback_count = 0;

        // Save state BEFORE this frame's simulation
        self.snapshots.save(self.current_frame, &self.state);

        // Predict any missing inputs
        self.ensure_inputs(self.current_frame);

        let inputs = self.inputs.get(self.current_frame);
        advance_frame(&mut self.state, inputs);
        self.current_frame += 1;

        &self.state
    }

    fn ensure_inputs(&mut self, frame: u32) {
        for p in 0..MAX_PLAYERS {
            if !self.inputs.is_confirmed(frame, p) {
                self.inputs.predict(frame, p);
            }
        }
    }

    fn rollback_to(&mut self, frame: u32) {
        let earliest = self.current_frame.saturating_sub(MAX_ROLLBACK_FRAMES);
        if frame < earliest {
            return;
        }

        let resim_count = self.current_frame - frame;
        self.last_rollback_count = resim_count;

        // Restore to the state at the START of the mispredicted frame
        self.state = self.snapshots.load(frame);

        // Re-simulate from frame to current_frame - 1
        for f in frame..self.current_frame {
            self.snapshots.save(f, &self.state);
            self.ensure_inputs(f);
            let inputs = self.inputs.get(f);
            advance_frame(&mut self.state, inputs);
        }
    }

    fn update_confirmed_frame(&mut self) {
        while self.last_confirmed_frame < self.current_frame
            && self.inputs.is_confirmed(self.last_confirmed_frame, 0)
            && self.inputs.is_confirmed(self.last_confirmed_frame, 1)
        {
            self.last_confirmed_frame += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_reference(frames: &[(PlayerInput, PlayerInput)]) -> GameState {
        let mut state = default_state();
        for (p1, p2) in frames {
            advance_frame(&mut state, [*p1, *p2]);
        }
        state
    }

    #[test]
    fn no_rollback_when_inputs_on_time() {
        let mut mgr = RollbackManager::new(default_state(), 0);
        let right = PlayerInput(PlayerInput::RIGHT);
        let left = PlayerInput(PlayerInput::LEFT);

        for _ in 0..10 {
            mgr.add_local_input(right);
            mgr.add_remote_input(mgr.current_frame + INPUT_DELAY, left);
            mgr.advance();
        }
        assert_eq!(mgr.last_rollback_count, 0);
    }

    #[test]
    fn rollback_on_late_remote_input() {
        let mut mgr = RollbackManager::new(default_state(), 0);
        let no = PlayerInput::default();
        let right = PlayerInput(PlayerInput::RIGHT);

        for _ in 0..5 {
            mgr.add_local_input(no);
            mgr.advance();
        }

        // Late remote input that differs from prediction
        mgr.add_remote_input(2, right);
        assert!(mgr.last_rollback_count > 0);
    }

    #[test]
    fn prediction_correct_no_rollback() {
        let mut mgr = RollbackManager::new(default_state(), 0);
        let no = PlayerInput::default();

        for _ in 0..5 {
            mgr.add_local_input(no);
            mgr.advance();
        }

        // Remote confirms default inputs — matches prediction
        for f in 0..5u32 {
            mgr.add_remote_input(f, no);
        }
        assert_eq!(mgr.last_rollback_count, 0);
    }

    #[test]
    fn input_delay_offsets_correctly() {
        let mut mgr = RollbackManager::new(default_state(), 0);
        let right = PlayerInput(PlayerInput::RIGHT);
        let target = mgr.add_local_input(right);
        assert_eq!(target, INPUT_DELAY);
    }

    #[test]
    fn determinism_through_rollback() {
        let right = PlayerInput(PlayerInput::RIGHT);
        let left = PlayerInput(PlayerInput::LEFT);
        let no = PlayerInput::default();

        let sequence = vec![
            (right, left),
            (right, left),
            (right, no),
            (no, left),
            (right, left),
            (no, no),
            (right, right),
            (no, left),
        ];
        let reference = run_reference(&sequence);

        let mut mgr = RollbackManager::new(default_state(), 0);

        // Pre-load all local (P1) inputs
        for (f, (p1, _)) in sequence.iter().enumerate() {
            mgr.inputs.set(f as u32, 0, *p1, true);
        }

        // Advance 4 frames with remote predicted (no remote input yet)
        for _ in 0..4 {
            mgr.advance();
        }

        // Deliver all 8 remote inputs (first 4 are late, triggers rollback)
        for (f, (_, p2)) in sequence.iter().enumerate() {
            mgr.add_remote_input(f as u32, *p2);
        }

        // Advance remaining 4 frames
        for _ in 4..8 {
            mgr.advance();
        }

        assert_eq!(mgr.state, reference, "rollback simulation must match reference");
    }

    #[test]
    fn checksum_detects_desync() {
        let mut mgr = RollbackManager::new(default_state(), 0);
        let no = PlayerInput::default();

        for _ in 0..5 {
            mgr.add_local_input(no);
            mgr.add_remote_input(mgr.current_frame, no);
            mgr.advance();
        }

        let correct = mgr.checksum_for_frame(3);
        mgr.check_remote_checksum(3, correct);
        assert!(!mgr.desync_detected);

        mgr.check_remote_checksum(3, correct.wrapping_add(1));
        assert!(mgr.desync_detected);
    }

    #[test]
    fn state_checksum_deterministic() {
        let a = default_state();
        let b = default_state();
        assert_eq!(state_checksum(&a), state_checksum(&b));

        let mut c = default_state();
        advance_frame(&mut c, [PlayerInput(PlayerInput::RIGHT), PlayerInput::default()]);
        assert_ne!(state_checksum(&a), state_checksum(&c));
    }
}
