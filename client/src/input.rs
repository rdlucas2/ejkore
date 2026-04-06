use std::collections::HashSet;
use wasm_bindgen::JsCast;
use ejkore_game::state::PlayerInput;
use crate::touch::TouchState;

const STICK_DEADZONE: f64 = 0.3;

// Action indices for rebinding
pub const ACTION_LEFT: u8 = 0;
pub const ACTION_RIGHT: u8 = 1;
pub const ACTION_UP: u8 = 2;
pub const ACTION_DOWN: u8 = 3;
pub const ACTION_ATTACK: u8 = 4;
pub const ACTION_SPECIAL: u8 = 5;
pub const ACTION_SHIELD: u8 = 6;
pub const ACTION_GRAB: u8 = 7;
pub const ACTION_SMASH: u8 = 8;
pub const ACTION_COUNT: u8 = 9;

pub const ACTION_NAMES: [&str; 9] = [
    "LEFT", "RIGHT", "UP", "DOWN",
    "ATTACK", "SPECIAL", "SHIELD", "GRAB", "SMASH",
];

const ACTION_BITS: [u16; 9] = [
    PlayerInput::LEFT, PlayerInput::RIGHT, PlayerInput::UP, PlayerInput::DOWN,
    PlayerInput::ATTACK, PlayerInput::SPECIAL, PlayerInput::SHIELD, PlayerInput::GRAB, PlayerInput::SMASH,
];

#[derive(Clone)]
pub struct KeyBindings {
    /// Keys for each action, indexed by ACTION_* constants
    pub keys: [String; 9],
}

impl KeyBindings {
    pub fn default_p1() -> Self {
        Self {
            keys: [
                "a".into(), "d".into(), "w".into(), "s".into(),
                "j".into(), "k".into(), "l".into(), ";".into(), "h".into(),
            ],
        }
    }

    pub fn default_p2() -> Self {
        Self {
            keys: [
                "ArrowLeft".into(), "ArrowRight".into(), "ArrowUp".into(), "ArrowDown".into(),
                "1".into(), "2".into(), "3".into(), "0".into(), "4".into(),
            ],
        }
    }

    pub fn to_json(&self) -> String {
        let entries: Vec<String> = self.keys.iter()
            .map(|k| format!("\"{}\"", k.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        format!("[{}]", entries.join(","))
    }

    pub fn from_json(s: &str) -> Option<Self> {
        // Minimal JSON array parser for ["key1","key2",...]
        let s = s.trim();
        if !s.starts_with('[') || !s.ends_with(']') {
            return None;
        }
        let inner = &s[1..s.len() - 1];
        let mut keys: Vec<String> = Vec::new();
        for part in inner.split(',') {
            let part = part.trim();
            if part.starts_with('"') && part.ends_with('"') && part.len() >= 2 {
                let key = part[1..part.len() - 1]
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                keys.push(key);
            } else {
                return None;
            }
        }
        if keys.len() != 9 {
            return None;
        }
        let mut arr: [String; 9] = Default::default();
        for (i, k) in keys.into_iter().enumerate() {
            arr[i] = k;
        }
        Some(Self { keys: arr })
    }
}

pub struct PlayerInputConfig {
    pub keyboard: KeyBindings,
    pub gamepad_index: Option<u32>,
}

/// Buttons that were just pressed this frame (not held from last frame).
/// Used for menu navigation where we want single-press, not repeat.
#[derive(Clone, Copy, Default)]
pub struct MenuInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub confirm: bool,  // A button / attack
    pub back: bool,     // B button / special
}

pub struct InputManager {
    pub keys: HashSet<String>,
    pub players: [PlayerInputConfig; 2],
    pub touch: TouchState,
    pub is_touch_device: bool,
    pub gamepads_connected: Vec<u32>,
    /// Previous frame's raw gamepad bits (for edge detection)
    prev_gamepad_bits: u16,
    /// Previous frame's key-derived bits (for edge detection)
    prev_key_bits: u16,
    /// Taps detected this frame (from touchstart), cleared each frame
    pub pending_taps: Vec<(f64, f64)>,
}

impl InputManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            keys: HashSet::new(),
            players: [
                PlayerInputConfig {
                    keyboard: KeyBindings::default_p1(),
                    gamepad_index: None,
                },
                PlayerInputConfig {
                    keyboard: KeyBindings::default_p2(),
                    gamepad_index: None,
                },
            ],
            touch: TouchState::new(),
            is_touch_device: false,
            gamepads_connected: Vec::new(),
            prev_gamepad_bits: 0,
            prev_key_bits: 0,
            pending_taps: Vec::new(),
        };
        mgr.load_from_local_storage();
        mgr.detect_touch();
        mgr
    }

    pub fn player_input(&self, player: usize) -> PlayerInput {
        let mut bits: u16 = 0;
        bits |= self.keyboard_input(player);
        bits |= self.gamepad_input(player);
        if player == 0 && self.is_touch_device {
            bits |= self.touch.to_input();
        }
        PlayerInput(bits)
    }

    /// Call once per frame BEFORE reading menu_input().
    /// Updates edge detection state and clears pending taps.
    pub fn poll_frame(&mut self) {
        let cur_gp = self.raw_gamepad_bits();
        let cur_kb = self.keyboard_input(0); // P1 keys for menu
        // Store for next frame
        self.prev_gamepad_bits = cur_gp;
        self.prev_key_bits = cur_kb;
    }

    /// Get menu navigation input with edge detection (just-pressed, not held).
    /// Combines gamepad + touch taps. Keyboard menus still use the keydown handler.
    pub fn menu_input(&mut self) -> MenuInput {
        let cur_gp = self.raw_gamepad_bits();
        let just_gp = cur_gp & !self.prev_gamepad_bits;

        let mut m = MenuInput::default();

        // Gamepad just-pressed
        m.left  |= just_gp & PlayerInput::LEFT != 0;
        m.right |= just_gp & PlayerInput::RIGHT != 0;
        m.up    |= just_gp & PlayerInput::UP != 0;
        m.down  |= just_gp & PlayerInput::DOWN != 0;
        m.confirm |= just_gp & PlayerInput::ATTACK != 0;
        m.back    |= just_gp & PlayerInput::SPECIAL != 0;

        // Update prev state
        self.prev_gamepad_bits = cur_gp;

        m
    }

    /// Drain pending taps (touch positions from touchstart events this frame).
    pub fn drain_taps(&mut self) -> Vec<(f64, f64)> {
        std::mem::take(&mut self.pending_taps)
    }

    /// Raw combined gamepad bits from first connected gamepad (for edge detection).
    fn raw_gamepad_bits(&self) -> u16 {
        // Check P1 gamepad first, then any connected
        if self.players[0].gamepad_index.is_some() {
            self.gamepad_input(0)
        } else if self.players[1].gamepad_index.is_some() {
            self.gamepad_input(1)
        } else {
            0
        }
    }

    fn keyboard_input(&self, player: usize) -> u16 {
        let binds = &self.players[player].keyboard;
        let mut bits = 0u16;
        for (i, key) in binds.keys.iter().enumerate() {
            // Case-insensitive match for single letter keys
            if self.keys.contains(key) {
                bits |= ACTION_BITS[i];
            } else if key.len() == 1 {
                // Check uppercase variant too
                let upper = key.to_uppercase();
                if self.keys.contains(&upper) {
                    bits |= ACTION_BITS[i];
                }
            }
        }
        bits
    }

    fn gamepad_input(&self, player: usize) -> u16 {
        let idx = match self.players[player].gamepad_index {
            Some(i) => i,
            None => return 0,
        };

        let window = match web_sys::window() {
            Some(w) => w,
            None => return 0,
        };
        let navigator = window.navigator();
        let gamepads = match navigator.get_gamepads() {
            Ok(g) => g,
            Err(_) => return 0,
        };

        let gp_val = gamepads.get(idx);
        if gp_val.is_null() || gp_val.is_undefined() {
            return 0;
        }
        let gp: web_sys::Gamepad = match gp_val.dyn_into() {
            Ok(g) => g,
            Err(_) => return 0,
        };

        let mut bits = 0u16;

        // Axes: left stick
        let axes = gp.axes();
        if axes.length() >= 2 {
            let x = axes.get(0).as_f64().unwrap_or(0.0);
            let y = axes.get(1).as_f64().unwrap_or(0.0);
            if x < -STICK_DEADZONE { bits |= PlayerInput::LEFT; }
            if x > STICK_DEADZONE { bits |= PlayerInput::RIGHT; }
            if y < -STICK_DEADZONE { bits |= PlayerInput::UP; }
            if y > STICK_DEADZONE { bits |= PlayerInput::DOWN; }
        }

        // Buttons
        let buttons = gp.buttons();
        let pressed = |idx: u32| -> bool {
            if idx >= buttons.length() { return false; }
            let btn_val = buttons.get(idx);
            if let Ok(btn) = btn_val.dyn_into::<web_sys::GamepadButton>() {
                btn.pressed()
            } else {
                false
            }
        };

        // Standard gamepad mapping
        if pressed(0) { bits |= PlayerInput::ATTACK; }   // A / Cross
        if pressed(1) { bits |= PlayerInput::SPECIAL; }   // B / Circle
        if pressed(2) { bits |= PlayerInput::GRAB; }      // X / Square
        if pressed(3) { bits |= PlayerInput::SMASH; }     // Y / Triangle
        if pressed(5) { bits |= PlayerInput::SHIELD; }    // RB / R1

        // D-pad
        if pressed(12) { bits |= PlayerInput::UP; }
        if pressed(13) { bits |= PlayerInput::DOWN; }
        if pressed(14) { bits |= PlayerInput::LEFT; }
        if pressed(15) { bits |= PlayerInput::RIGHT; }

        bits
    }

    pub fn update_gamepad_assignments(&mut self) {
        // Auto-assign gamepads: first connected = P1, second = P2
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let navigator = window.navigator();
        let gamepads = match navigator.get_gamepads() {
            Ok(g) => g,
            Err(_) => return,
        };

        let mut assigned = Vec::new();
        for i in 0..gamepads.length() {
            let gp_val = gamepads.get(i);
            if gp_val.is_null() || gp_val.is_undefined() {
                continue;
            }
            if let Ok(_gp) = gp_val.dyn_into::<web_sys::Gamepad>() {
                assigned.push(i);
            }
        }

        self.players[0].gamepad_index = assigned.first().copied();
        self.players[1].gamepad_index = assigned.get(1).copied();
        self.gamepads_connected = assigned;
    }

    fn detect_touch(&mut self) {
        if let Some(window) = web_sys::window() {
            let has_touch = js_sys::Reflect::has(&window, &"ontouchstart".into()).unwrap_or(false);
            self.is_touch_device = has_touch;
        }
    }

    pub fn save_to_local_storage(&self) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let storage = match window.local_storage() {
            Ok(Some(s)) => s,
            _ => return,
        };
        let _ = storage.set_item("ejkore_p1_keys", &self.players[0].keyboard.to_json());
        let _ = storage.set_item("ejkore_p2_keys", &self.players[1].keyboard.to_json());
    }

    pub fn load_from_local_storage(&mut self) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let storage = match window.local_storage() {
            Ok(Some(s)) => s,
            _ => return,
        };
        if let Ok(Some(json)) = storage.get_item("ejkore_p1_keys") {
            if let Some(binds) = KeyBindings::from_json(&json) {
                self.players[0].keyboard = binds;
            }
        }
        if let Ok(Some(json)) = storage.get_item("ejkore_p2_keys") {
            if let Some(binds) = KeyBindings::from_json(&json) {
                self.players[1].keyboard = binds;
            }
        }
    }

    pub fn reset_defaults(&mut self) {
        self.players[0].keyboard = KeyBindings::default_p1();
        self.players[1].keyboard = KeyBindings::default_p2();
    }

    /// Set a binding for a player's action. Returns the key that was replaced.
    pub fn set_binding(&mut self, player: usize, action: u8, key: String) -> String {
        let old = self.players[player].keyboard.keys[action as usize].clone();
        self.players[player].keyboard.keys[action as usize] = key;
        old
    }

    /// Get display name for a key
    pub fn key_display(key: &str) -> &str {
        match key {
            " " => "SPACE",
            "ArrowLeft" => "LEFT",
            "ArrowRight" => "RIGHT",
            "ArrowUp" => "UP",
            "ArrowDown" => "DOWN",
            "Escape" => "ESC",
            "Shift" => "SHIFT",
            "Control" => "CTRL",
            "Alt" => "ALT",
            "Enter" => "ENTER",
            "Backspace" => "BKSP",
            "Tab" => "TAB",
            _ => key,
        }
    }
}
