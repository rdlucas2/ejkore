use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent, TouchEvent};

use ejkore_game::state::*;
use ejkore_game::rollback::RollbackManager;

mod net;
mod input;
mod touch;

use net::*;
use input::*;
use touch::draw_touch_controls;

#[wasm_bindgen(inline_js = "
export function start_music(src) {
    const audio = new Audio(src);
    audio.loop = true;
    audio.volume = 0.5;
    audio.play().catch(() => {});
    return audio;
}
export function try_play_music(audio) {
    if (audio.paused) {
        audio.play().catch(() => {});
    }
}
")]
extern "C" {
    fn start_music(src: &str) -> JsValue;
    fn try_play_music(audio: &JsValue);
}

const CANVAS_WIDTH: f64 = 1280.0;
const CANVAS_HEIGHT: f64 = 720.0;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    MainMenu,
    Settings,
    CharSelect,
    OnlineLobby,
    Fighting,
    GameOver,
}

#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    Local,
    Online,
}

struct AppState {
    screen: Screen,
    mode: GameMode,
    game: GameState,
    p1_char: CharacterId,
    p2_char: CharacterId,
    p1_char_idx: usize,
    p2_char_idx: usize,
    p1_ready: bool,
    p2_ready: bool,
    // Online-specific state
    net: Option<NetworkManager>,
    rollback: Option<RollbackManager>,
    local_player: usize,
    /// Frames since last checksum sent
    checksum_timer: u32,
    // Settings rebind state
    /// (player_index, action_index) currently being rebound, None if not rebinding
    rebind_target: Option<(usize, u8)>,
    // Menu cursor for gamepad/touch navigation
    menu_cursor: usize,
}

const CHAR_LIST: [CharacterId; 3] = [CharacterId::Balanced, CharacterId::Ranged, CharacterId::Rushdown];

fn char_name(id: CharacterId) -> &'static str {
    match id {
        CharacterId::Balanced => "BALANCED",
        CharacterId::Ranged => "RANGED",
        CharacterId::Rushdown => "RUSHDOWN",
    }
}

fn char_desc(id: CharacterId) -> &'static str {
    match id {
        CharacterId::Balanced => "Medium speed, medium weight. Well-rounded.",
        CharacterId::Ranged => "Slower, more projectiles. Controls space.",
        CharacterId::Rushdown => "Fast, light. Aggressive pressure.",
    }
}

const CHECKSUM_INTERVAL: u32 = 30; // send checksum every 30 frames
/// Build the signaling server URL from the current page's host.
/// Priority: ?signal=host:port query param > same-origin /ws path.
/// Detects HTTPS and uses wss:// accordingly.
fn signaling_url() -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();

    // Check query param: ?signal=192.168.1.5:3536
    if let Ok(search) = location.search() {
        for param in search.trim_start_matches('?').split('&') {
            if let Some(val) = param.strip_prefix("signal=") {
                if !val.is_empty() {
                    return format!("ws://{}", val);
                }
            }
        }
    }

    let protocol = location.protocol().unwrap_or_else(|_| "http:".into());
    let host = location.host().unwrap_or_else(|_| "localhost".into());

    if protocol == "https:" {
        // Production / Fly.io: signaling proxied at /ws on same origin
        format!("wss://{}/ws", host)
    } else {
        // Local dev: signaling on separate port
        let hostname = location.hostname().unwrap_or_else(|_| "localhost".into());
        format!("ws://{}:3536", hostname)
    }
}

impl AppState {
    fn new() -> Self {
        Self {
            screen: Screen::MainMenu,
            mode: GameMode::Local,
            game: default_state(),
            p1_char: CharacterId::Balanced,
            p2_char: CharacterId::Balanced,
            p1_char_idx: 0,
            p2_char_idx: 0,
            p1_ready: false,
            p2_ready: false,
            net: None,
            rollback: None,
            local_player: 0,
            checksum_timer: 0,
            rebind_target: None,
            menu_cursor: 0,
        }
    }

    fn start_game(&mut self) {
        self.game = default_state();
        self.game.players[0].character = self.p1_char;
        self.game.players[1].character = self.p2_char;
        if self.mode == GameMode::Online {
            self.rollback = Some(RollbackManager::new(self.game, self.local_player));
            self.checksum_timer = 0;
        }
        self.screen = Screen::Fighting;
    }

    fn start_online(&mut self) {
        self.mode = GameMode::Online;
        self.net = Some(NetworkManager::new(&signaling_url()));
        self.screen = Screen::OnlineLobby;
        self.p1_ready = false;
        self.p2_ready = false;
    }

    fn start_local(&mut self) {
        self.mode = GameMode::Local;
        self.net = None;
        self.rollback = None;
        self.screen = Screen::CharSelect;
        self.p1_ready = false;
        self.p2_ready = false;
    }
}

// InputState replaced by InputManager in input.rs

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("game")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;

    let ctx = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;

    let input_state = Rc::new(RefCell::new(InputManager::new()));
    let app_state = Rc::new(RefCell::new(AppState::new()));

    // Background music — loops forever, started on first user interaction
    let music = Rc::new(start_music("musinova-hyper-garden-jungle-breakbeat-drum-and-bass-loop-edit-356528.mp3"));

    // Disable touch-action on canvas for mobile
    {
        let html_el: web_sys::HtmlElement = canvas.clone().into();
        let _ = html_el.style().set_property("touch-action", "none");
    }

    // Keyboard event listeners
    {
        let input = input_state.clone();
        let music = music.clone();
        let keydown = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            e.prevent_default();
            input.borrow_mut().keys.insert(e.key());
            try_play_music(&music);
        });
        document.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())?;
        keydown.forget();
    }
    {
        let input = input_state.clone();
        let keyup = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            input.borrow_mut().keys.remove(&e.key());
        });
        document.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())?;
        keyup.forget();
    }

    // Touch event listeners
    {
        let input = input_state.clone();
        let music = music.clone();
        let handler = Closure::<dyn FnMut(TouchEvent)>::new(move |e: TouchEvent| {
            e.prevent_default();
            let mut input = input.borrow_mut();
            let touches = e.changed_touches();
            for i in 0..touches.length() {
                if let Some(t) = touches.get(i) {
                    let x = t.client_x() as f64;
                    let y = t.client_y() as f64;
                    input.touch.touch_start(t.identifier(), x, y);
                    input.pending_taps.push((x, y));
                }
            }
            try_play_music(&music);
        });
        canvas.add_event_listener_with_callback("touchstart", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }
    {
        let input = input_state.clone();
        let handler = Closure::<dyn FnMut(TouchEvent)>::new(move |e: TouchEvent| {
            e.prevent_default();
            let mut input = input.borrow_mut();
            let touches = e.changed_touches();
            for i in 0..touches.length() {
                if let Some(t) = touches.get(i) {
                    input.touch.touch_move(t.identifier(), t.client_x() as f64, t.client_y() as f64);
                }
            }
        });
        canvas.add_event_listener_with_callback("touchmove", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }
    {
        let input = input_state.clone();
        let handler = Closure::<dyn FnMut(TouchEvent)>::new(move |e: TouchEvent| {
            e.prevent_default();
            let mut input = input.borrow_mut();
            let touches = e.changed_touches();
            for i in 0..touches.length() {
                if let Some(t) = touches.get(i) {
                    input.touch.touch_end(t.identifier());
                }
            }
        });
        canvas.add_event_listener_with_callback("touchend", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }
    {
        let input = input_state.clone();
        let handler = Closure::<dyn FnMut(TouchEvent)>::new(move |e: TouchEvent| {
            e.prevent_default();
            let mut input = input.borrow_mut();
            let touches = e.changed_touches();
            for i in 0..touches.length() {
                if let Some(t) = touches.get(i) {
                    input.touch.touch_end(t.identifier());
                }
            }
        });
        canvas.add_event_listener_with_callback("touchcancel", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }

    // Gamepad connect/disconnect listeners
    {
        let input = input_state.clone();
        let handler = Closure::<dyn FnMut(web_sys::GamepadEvent)>::new(move |_e: web_sys::GamepadEvent| {
            input.borrow_mut().update_gamepad_assignments();
        });
        window.add_event_listener_with_callback("gamepadconnected", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }
    {
        let input = input_state.clone();
        let handler = Closure::<dyn FnMut(web_sys::GamepadEvent)>::new(move |_e: web_sys::GamepadEvent| {
            input.borrow_mut().update_gamepad_assignments();
        });
        window.add_event_listener_with_callback("gamepaddisconnected", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }

    // Screen control keys
    {
        let app = app_state.clone();
        let input = input_state.clone();
        let handler = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            let mut app = app.borrow_mut();
            let key = e.key();
            match app.screen {
                Screen::MainMenu => {
                    match key.as_str() {
                        "1" => app.start_local(),
                        "2" => app.start_online(),
                        "3" => { app.screen = Screen::Settings; app.rebind_target = None; }
                        _ => {}
                    }
                }
                Screen::Settings => {
                    if let Some((player, action)) = app.rebind_target {
                        // Capture this key as the new binding
                        if key != "Escape" {
                            input.borrow_mut().set_binding(player, action, key);
                        }
                        app.rebind_target = None;
                    } else {
                        match key.as_str() {
                            "Escape" => {
                                input.borrow().save_to_local_storage();
                                app.screen = Screen::MainMenu;
                            }
                            "r" | "R" => {
                                input.borrow_mut().reset_defaults();
                            }
                            // Navigate: 1-9 for P1 actions, shift+1-9 for P2
                            // Or use simpler scheme: a-i for P1, A-I (shift) for P2
                            // Actually use number keys: 1-9 = P1 action slots
                            "1" => { app.rebind_target = Some((0, 0)); }
                            "2" => { app.rebind_target = Some((0, 1)); }
                            "3" => { app.rebind_target = Some((0, 2)); }
                            "4" => { app.rebind_target = Some((0, 3)); }
                            "5" => { app.rebind_target = Some((0, 4)); }
                            "6" => { app.rebind_target = Some((0, 5)); }
                            "7" => { app.rebind_target = Some((0, 6)); }
                            "8" => { app.rebind_target = Some((0, 7)); }
                            "9" => { app.rebind_target = Some((0, 8)); }
                            // F1-F9 for P2
                            "F1" => { e.prevent_default(); app.rebind_target = Some((1, 0)); }
                            "F2" => { e.prevent_default(); app.rebind_target = Some((1, 1)); }
                            "F3" => { e.prevent_default(); app.rebind_target = Some((1, 2)); }
                            "F4" => { e.prevent_default(); app.rebind_target = Some((1, 3)); }
                            "F5" => { e.prevent_default(); app.rebind_target = Some((1, 4)); }
                            "F6" => { e.prevent_default(); app.rebind_target = Some((1, 5)); }
                            "F7" => { e.prevent_default(); app.rebind_target = Some((1, 6)); }
                            "F8" => { e.prevent_default(); app.rebind_target = Some((1, 7)); }
                            "F9" => { e.prevent_default(); app.rebind_target = Some((1, 8)); }
                            _ => {}
                        }
                    }
                }
                Screen::OnlineLobby => {
                    if key == "Escape" {
                        app.net = None;
                        app.screen = Screen::MainMenu;
                    }
                }
                Screen::CharSelect => {
                    let is_online = app.mode == GameMode::Online;
                    // Use P1 LEFT/RIGHT/ATTACK bindings for char select navigation
                    let p1_left = input.borrow().players[0].keyboard.keys[ACTION_LEFT as usize].clone();
                    let p1_right = input.borrow().players[0].keyboard.keys[ACTION_RIGHT as usize].clone();
                    let p1_attack = input.borrow().players[0].keyboard.keys[ACTION_ATTACK as usize].clone();
                    let p2_left = input.borrow().players[1].keyboard.keys[ACTION_LEFT as usize].clone();
                    let p2_right = input.borrow().players[1].keyboard.keys[ACTION_RIGHT as usize].clone();
                    let p2_attack = input.borrow().players[1].keyboard.keys[ACTION_ATTACK as usize].clone();

                    let key_lower = key.to_lowercase();
                    let p1_left_lower = p1_left.to_lowercase();
                    let p1_right_lower = p1_right.to_lowercase();
                    let p1_attack_lower = p1_attack.to_lowercase();

                    if key_lower == p1_left_lower {
                        if !app.p1_ready {
                            app.p1_char_idx = (app.p1_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                            app.p1_char = CHAR_LIST[app.p1_char_idx];
                            if is_online {
                                let idx = app.p1_char_idx as u8;
                                if let Some(ref mut net) = app.net {
                                    net.send_char_select(idx, false);
                                }
                            }
                        }
                    } else if key_lower == p1_right_lower {
                        if !app.p1_ready {
                            app.p1_char_idx = (app.p1_char_idx + 1) % CHAR_LIST.len();
                            app.p1_char = CHAR_LIST[app.p1_char_idx];
                            if is_online {
                                let idx = app.p1_char_idx as u8;
                                if let Some(ref mut net) = app.net {
                                    net.send_char_select(idx, false);
                                }
                            }
                        }
                    } else if key_lower == p1_attack_lower {
                        app.p1_ready = !app.p1_ready;
                        if is_online {
                            let idx = app.p1_char_idx as u8;
                            let ready = app.p1_ready;
                            if let Some(ref mut net) = app.net {
                                net.send_char_select(idx, ready);
                            }
                        }
                    } else if !is_online {
                        // P2 char select (local only)
                        if key == p2_left {
                            if !app.p2_ready {
                                app.p2_char_idx = (app.p2_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                                app.p2_char = CHAR_LIST[app.p2_char_idx];
                            }
                        } else if key == p2_right {
                            if !app.p2_ready {
                                app.p2_char_idx = (app.p2_char_idx + 1) % CHAR_LIST.len();
                                app.p2_char = CHAR_LIST[app.p2_char_idx];
                            }
                        } else if key == p2_attack {
                            app.p2_ready = !app.p2_ready;
                        }
                    }

                    if key == "Escape" {
                        if is_online {
                            app.screen = Screen::OnlineLobby;
                        } else {
                            app.screen = Screen::MainMenu;
                        }
                        app.p1_ready = false;
                        app.p2_ready = false;
                    }

                    if !is_online && app.p1_ready && app.p2_ready {
                        app.start_game();
                    }
                }
                Screen::GameOver => {
                    if key == "r" || key == "R" {
                        if app.mode == GameMode::Online {
                            app.screen = Screen::MainMenu;
                            app.net = None;
                            app.rollback = None;
                        } else {
                            app.screen = Screen::CharSelect;
                        }
                        app.p1_ready = false;
                        app.p2_ready = false;
                    }
                }
                Screen::Fighting => {
                    if key == "r" || key == "R" {
                        if app.mode == GameMode::Local {
                            app.start_game();
                        }
                    }
                }
            }
        });
        document.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref())?;
        handler.forget();
    }

    // Game loop
    let perf = window.performance().unwrap();
    let last_time = Rc::new(RefCell::new(perf.now()));
    let accumulator = Rc::new(RefCell::new(0.0_f64));
    const FRAME_TIME: f64 = 1000.0 / 60.0;

    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let ctx = Rc::new(ctx);

    *g.borrow_mut() = Some(Closure::new(move |now: f64| {
        let dt = (now - *last_time.borrow()).min(100.0);
        *last_time.borrow_mut() = now;
        *accumulator.borrow_mut() += dt;

        let mut app = app_state.borrow_mut();

        // Poll gamepad assignments and frame state
        let (menu, taps) = {
            let mut inp = input_state.borrow_mut();
            inp.update_gamepad_assignments();
            let m = inp.menu_input();
            let t = inp.drain_taps();
            (m, t)
        };

        // Gamepad + touch menu navigation (all non-Fighting screens)
        if app.screen != Screen::Fighting {
            handle_menu_nav(&mut app, &input_state, &menu, &taps);
        }

        // Update networking if active
        if let Some(ref mut net) = app.net {
            net.update();
        }

        // Handle online lobby state transitions
        if app.screen == Screen::OnlineLobby {
            if let Some(ref net) = app.net {
                if net.is_connected() {
                    // Connected to peer — determine player assignment
                    // Lower peer ID is player 0
                    if let Some(peer) = net.peer {
                        let we_are_lower = format!("{:?}", peer) > "local".to_string();
                        app.local_player = if we_are_lower { 0 } else { 1 };
                        app.screen = Screen::CharSelect;
                        app.p1_ready = false;
                        app.p2_ready = false;
                    }
                }
            }
        }

        // Handle online char select network messages
        if app.screen == Screen::CharSelect && app.mode == GameMode::Online {
            let messages = app.net.as_mut()
                .map(|net| net.receive())
                .unwrap_or_default();

            for (msg_type, payload) in &messages {
                match *msg_type {
                    MSG_CHAR_SELECT => {
                        if payload.len() >= 2 {
                            let char_idx = payload[0] as usize;
                            let ready = payload[1] != 0;
                            if char_idx < CHAR_LIST.len() {
                                app.p2_char_idx = char_idx;
                                app.p2_char = CHAR_LIST[char_idx];
                                app.p2_ready = ready;
                            }
                        }
                    }
                    MSG_START_GAME => {
                        if payload.len() >= 1 {
                            app.local_player = payload[0] as usize;
                            app.start_game();
                        }
                    }
                    _ => {}
                }
            }
            // If both ready in online mode, host (player 0) sends start
            if app.p1_ready && app.p2_ready && app.screen == Screen::CharSelect {
                if app.local_player == 1 {
                    let tmp = app.p1_char;
                    app.p1_char = app.p2_char;
                    app.p2_char = tmp;
                    let tmp = app.p1_char_idx;
                    app.p1_char_idx = app.p2_char_idx;
                    app.p2_char_idx = tmp;
                }
                let remote_player = 1 - app.local_player;
                if let Some(ref mut net) = app.net {
                    net.send_start_game(remote_player as u8);
                }
                app.start_game();
            }
        }

        if app.screen == Screen::Fighting {
            while *accumulator.borrow() >= FRAME_TIME {
                *accumulator.borrow_mut() -= FRAME_TIME;

                match app.mode {
                    GameMode::Local => {
                        let input = input_state.borrow();
                        let inputs = [input.player_input(0), input.player_input(1)];
                        drop(input);
                        advance_frame(&mut app.game, inputs);
                    }
                    GameMode::Online => {
                        // Take rollback + net out to avoid borrow conflicts
                        let mut rb = app.rollback.take();
                        let mut net = app.net.take();

                        // Collect incoming messages
                        let messages = net.as_mut()
                            .map(|n| n.receive())
                            .unwrap_or_default();

                        if let Some(ref mut rb) = rb {
                            // Process network messages
                            for (msg_type, payload) in &messages {
                                match *msg_type {
                                    MSG_INPUT => {
                                        if let Some((frame, inputs)) = parse_input_bundle(payload) {
                                            for i in (0..3).rev() {
                                                let f = frame.saturating_sub(i as u32);
                                                rb.add_remote_input(f, PlayerInput(inputs[i as usize]));
                                            }
                                        }
                                    }
                                    MSG_CHECKSUM => {
                                        if let Some((frame, hash)) = parse_checksum(payload) {
                                            rb.check_remote_checksum(frame, hash);
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            // Add local input
                            let input = input_state.borrow();
                            let local_input = input.player_input(0);
                            drop(input);
                            let target_frame = rb.add_local_input(local_input);

                            // Advance simulation
                            rb.advance();
                            app.game = rb.state;

                            // Send input + periodic checksum
                            if let Some(ref mut n) = net {
                                n.send_input(target_frame, [local_input.0; 3]);

                                app.checksum_timer += 1;
                                if app.checksum_timer >= CHECKSUM_INTERVAL && rb.last_confirmed_frame > 0 {
                                    app.checksum_timer = 0;
                                    let cf = rb.last_confirmed_frame;
                                    n.send_checksum(cf, rb.checksum_for_frame(cf));
                                }
                            }
                        }

                        // Put them back
                        app.rollback = rb;
                        app.net = net;
                    }
                }
            }
            if app.game.match_over {
                app.screen = Screen::GameOver;
            }
        } else {
            *accumulator.borrow_mut() = 0.0;
        }

        {
            let input = input_state.borrow();
            match app.screen {
                Screen::MainMenu => draw_main_menu(&ctx, &app, &input),
                Screen::Settings => draw_settings(&ctx, &app, &input),
                Screen::OnlineLobby => draw_online_lobby(&ctx, &app),
                Screen::CharSelect => draw_char_select(&ctx, &app),
                Screen::Fighting => {
                    draw_fight(&ctx, &app);
                    if input.is_touch_device {
                        draw_touch_controls(&ctx, &input.touch);
                    }
                }
                Screen::GameOver => draw_game_over(&ctx, &app.game),
            }
        }

        drop(app);

        web_sys::window()
            .unwrap()
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }));

    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}

fn draw_char_select(ctx: &CanvasRenderingContext2d, app: &AppState) {
    ctx.set_fill_style_str("#1a1a2e");
    ctx.fill_rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT);

    ctx.set_text_align("center");
    ctx.set_fill_style_str("#e94560");
    ctx.set_font("bold 48px monospace");
    let _ = ctx.fill_text("EJKORE", CANVAS_WIDTH / 2.0, 80.0);

    ctx.set_fill_style_str("rgba(255,255,255,0.6)");
    ctx.set_font("18px monospace");
    let _ = ctx.fill_text("SELECT YOUR FIGHTER", CANVAS_WIDTH / 2.0, 120.0);

    let colors = ["#e94560", "#00d2ff"];
    let panels = [CANVAS_WIDTH / 4.0, CANVAS_WIDTH * 3.0 / 4.0];
    let chars = [app.p1_char, app.p2_char];
    let ready = [app.p1_ready, app.p2_ready];

    for (i, &panel_x) in panels.iter().enumerate() {
        let color = colors[i];
        let ch = chars[i];

        // Panel background
        ctx.set_fill_style_str(if ready[i] { "rgba(0,255,0,0.1)" } else { "rgba(255,255,255,0.05)" });
        ctx.fill_rect(panel_x - 200.0, 160.0, 400.0, 420.0);

        // Border
        ctx.set_stroke_style_str(if ready[i] { "#00ff00" } else { color });
        ctx.set_line_width(3.0);
        ctx.stroke_rect(panel_x - 200.0, 160.0, 400.0, 420.0);

        // Player label
        ctx.set_fill_style_str(color);
        ctx.set_font("bold 24px monospace");
        let _ = ctx.fill_text(&format!("PLAYER {}", i + 1), panel_x, 200.0);

        // Character preview (colored rectangle)
        ctx.set_fill_style_str(color);
        let pw = PLAYER_WIDTH.to_int() as f64 * 2.0;
        let ph = PLAYER_HEIGHT.to_int() as f64 * 2.0;
        ctx.fill_rect(panel_x - pw / 2.0, 250.0, pw, ph);

        // Character name
        ctx.set_fill_style_str("#ffffff");
        ctx.set_font("bold 28px monospace");
        let _ = ctx.fill_text(char_name(ch), panel_x, 410.0);

        // Description
        ctx.set_fill_style_str("rgba(255,255,255,0.6)");
        ctx.set_font("14px monospace");
        let _ = ctx.fill_text(char_desc(ch), panel_x, 440.0);

        // Stats
        let s = character_stats(ch);
        ctx.set_font("13px monospace");
        ctx.set_fill_style_str("rgba(255,255,255,0.5)");
        let _ = ctx.fill_text(&format!("Weight: {}  Speed: {}  Gravity: {:.1}",
            s.weight.to_int(), s.walk_speed.to_int(),
            s.gravity.raw() as f64 / 65536.0), panel_x, 470.0);
        let _ = ctx.fill_text(&format!("Projectiles: {}", s.max_projectiles), panel_x, 490.0);

        // Arrows
        if !ready[i] {
            ctx.set_fill_style_str(color);
            ctx.set_font("bold 36px monospace");
            let _ = ctx.fill_text("<", panel_x - 170.0, 320.0);
            let _ = ctx.fill_text(">", panel_x + 150.0, 320.0);
        }

        // Ready status
        if ready[i] {
            ctx.set_fill_style_str("#00ff00");
            ctx.set_font("bold 22px monospace");
            let _ = ctx.fill_text("READY!", panel_x, 550.0);
        }
    }

    // Controls
    ctx.set_fill_style_str("rgba(255,255,255,0.4)");
    ctx.set_font("14px monospace");
    let _ = ctx.fill_text("P1: LEFT/RIGHT to pick, ATTACK to confirm  |  P2: same  |  ESC: back", CANVAS_WIDTH / 2.0, CANVAS_HEIGHT - 20.0);
    ctx.set_text_align("start");
}

/// Handle gamepad and touch navigation on non-fighting screens.
fn handle_menu_nav(
    app: &mut AppState,
    input_state: &Rc<RefCell<InputManager>>,
    menu: &MenuInput,
    taps: &[(f64, f64)],
) {
    fn tap_in(taps: &[(f64, f64)], x: f64, y: f64, w: f64, h: f64) -> bool {
        taps.iter().any(|&(tx, ty)| tx >= x && tx <= x + w && ty >= y && ty <= y + h)
    }

    match app.screen {
        Screen::MainMenu => {
            // 3 items: Local(0), Online(1), Settings(2)
            const ITEM_COUNT: usize = 3;
            if menu.up { app.menu_cursor = (app.menu_cursor + ITEM_COUNT - 1) % ITEM_COUNT; }
            if menu.down { app.menu_cursor = (app.menu_cursor + 1) % ITEM_COUNT; }
            if menu.confirm {
                match app.menu_cursor {
                    0 => app.start_local(),
                    1 => app.start_online(),
                    2 => { app.screen = Screen::Settings; app.rebind_target = None; }
                    _ => {}
                }
            }
            // Touch: tap on menu items (centered at CANVAS_WIDTH/2, y=380/430/480, ~400x50 each)
            let cx = CANVAS_WIDTH / 2.0 - 200.0;
            if tap_in(taps, cx, 355.0, 400.0, 50.0) { app.start_local(); }
            if tap_in(taps, cx, 405.0, 400.0, 50.0) { app.start_online(); }
            if tap_in(taps, cx, 455.0, 400.0, 50.0) { app.screen = Screen::Settings; app.rebind_target = None; }
        }
        Screen::Settings => {
            if menu.back {
                input_state.borrow().save_to_local_storage();
                app.screen = Screen::MainMenu;
                app.menu_cursor = 0;
            }
            // Touch: tap "Back to Menu" area at bottom
            if tap_in(taps, CANVAS_WIDTH / 2.0 - 200.0, CANVAS_HEIGHT - 40.0, 400.0, 40.0) {
                input_state.borrow().save_to_local_storage();
                app.screen = Screen::MainMenu;
                app.menu_cursor = 0;
            }
        }
        Screen::OnlineLobby => {
            if menu.back {
                app.net = None;
                app.screen = Screen::MainMenu;
                app.menu_cursor = 0;
            }
            if tap_in(taps, CANVAS_WIDTH / 2.0 - 200.0, 470.0, 400.0, 50.0) {
                app.net = None;
                app.screen = Screen::MainMenu;
                app.menu_cursor = 0;
            }
        }
        Screen::CharSelect => {
            let is_online = app.mode == GameMode::Online;
            // Gamepad: left/right to cycle char, confirm to ready
            if menu.left && !app.p1_ready {
                app.p1_char_idx = (app.p1_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                app.p1_char = CHAR_LIST[app.p1_char_idx];
                if is_online {
                    let idx = app.p1_char_idx as u8;
                    if let Some(ref mut net) = app.net {
                        net.send_char_select(idx, false);
                    }
                }
            }
            if menu.right && !app.p1_ready {
                app.p1_char_idx = (app.p1_char_idx + 1) % CHAR_LIST.len();
                app.p1_char = CHAR_LIST[app.p1_char_idx];
                if is_online {
                    let idx = app.p1_char_idx as u8;
                    if let Some(ref mut net) = app.net {
                        net.send_char_select(idx, false);
                    }
                }
            }
            if menu.confirm {
                app.p1_ready = !app.p1_ready;
                if is_online {
                    let idx = app.p1_char_idx as u8;
                    let ready = app.p1_ready;
                    if let Some(ref mut net) = app.net {
                        net.send_char_select(idx, ready);
                    }
                }
            }
            if menu.back {
                if is_online {
                    app.screen = Screen::OnlineLobby;
                } else {
                    app.screen = Screen::MainMenu;
                    app.menu_cursor = 0;
                }
                app.p1_ready = false;
                app.p2_ready = false;
            }

            // Touch: tap arrows and ready area
            // P1 panel centered at CANVAS_WIDTH/4 (~320), arrows at +-170, ready area at bottom
            let p1x = CANVAS_WIDTH / 4.0;
            // Left arrow
            if tap_in(taps, p1x - 200.0, 280.0, 60.0, 80.0) && !app.p1_ready {
                app.p1_char_idx = (app.p1_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                app.p1_char = CHAR_LIST[app.p1_char_idx];
                if is_online {
                    let idx = app.p1_char_idx as u8;
                    if let Some(ref mut net) = app.net { net.send_char_select(idx, false); }
                }
            }
            // Right arrow
            if tap_in(taps, p1x + 120.0, 280.0, 60.0, 80.0) && !app.p1_ready {
                app.p1_char_idx = (app.p1_char_idx + 1) % CHAR_LIST.len();
                app.p1_char = CHAR_LIST[app.p1_char_idx];
                if is_online {
                    let idx = app.p1_char_idx as u8;
                    if let Some(ref mut net) = app.net { net.send_char_select(idx, false); }
                }
            }
            // Ready/unready — tap the character panel center area
            if tap_in(taps, p1x - 150.0, 380.0, 300.0, 80.0) {
                app.p1_ready = !app.p1_ready;
                if is_online {
                    let idx = app.p1_char_idx as u8;
                    let ready = app.p1_ready;
                    if let Some(ref mut net) = app.net { net.send_char_select(idx, ready); }
                }
            }

            // P2 touch (local only)
            if !is_online {
                let p2x = CANVAS_WIDTH * 3.0 / 4.0;
                if tap_in(taps, p2x - 200.0, 280.0, 60.0, 80.0) && !app.p2_ready {
                    app.p2_char_idx = (app.p2_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                    app.p2_char = CHAR_LIST[app.p2_char_idx];
                }
                if tap_in(taps, p2x + 120.0, 280.0, 60.0, 80.0) && !app.p2_ready {
                    app.p2_char_idx = (app.p2_char_idx + 1) % CHAR_LIST.len();
                    app.p2_char = CHAR_LIST[app.p2_char_idx];
                }
                if tap_in(taps, p2x - 150.0, 380.0, 300.0, 80.0) {
                    app.p2_ready = !app.p2_ready;
                }
            }

            if !is_online && app.p1_ready && app.p2_ready {
                app.start_game();
            }
        }
        Screen::GameOver => {
            if menu.confirm {
                if app.mode == GameMode::Online {
                    app.screen = Screen::MainMenu;
                    app.net = None;
                    app.rollback = None;
                } else {
                    app.screen = Screen::CharSelect;
                }
                app.p1_ready = false;
                app.p2_ready = false;
                app.menu_cursor = 0;
            }
            // Touch: tap anywhere to continue
            if !taps.is_empty() {
                if app.mode == GameMode::Online {
                    app.screen = Screen::MainMenu;
                    app.net = None;
                    app.rollback = None;
                } else {
                    app.screen = Screen::CharSelect;
                }
                app.p1_ready = false;
                app.p2_ready = false;
                app.menu_cursor = 0;
            }
        }
        Screen::Fighting => {} // handled by game loop
    }
}

fn draw_main_menu(ctx: &CanvasRenderingContext2d, app: &AppState, input: &InputManager) {
    ctx.set_fill_style_str("#1a1a2e");
    ctx.fill_rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT);

    ctx.set_text_align("center");
    ctx.set_fill_style_str("#e94560");
    ctx.set_font("bold 72px monospace");
    let _ = ctx.fill_text("EJKORE", CANVAS_WIDTH / 2.0, 200.0);

    ctx.set_fill_style_str("rgba(255,255,255,0.5)");
    ctx.set_font("18px monospace");
    let _ = ctx.fill_text("PLATFORM FIGHTER", CANVAS_WIDTH / 2.0, 250.0);

    let items = ["1 - LOCAL VERSUS", "2 - ONLINE MATCH", "3 - SETTINGS"];
    let y_positions = [380.0, 430.0, 480.0];
    for (i, (label, &y)) in items.iter().zip(y_positions.iter()).enumerate() {
        // Cursor highlight
        if app.menu_cursor == i {
            ctx.set_fill_style_str("rgba(233, 69, 96, 0.15)");
            ctx.fill_rect(CANVAS_WIDTH / 2.0 - 200.0, y - 25.0, 400.0, 40.0);
            ctx.set_fill_style_str("#e94560");
            ctx.set_font("bold 28px monospace");
            let _ = ctx.fill_text(&format!("> {} <", label), CANVAS_WIDTH / 2.0, y);
        } else {
            ctx.set_fill_style_str("#ffffff");
            ctx.set_font("bold 28px monospace");
            let _ = ctx.fill_text(label, CANVAS_WIDTH / 2.0, y);
        }
    }

    // Gamepad status
    let gp_count = input.gamepads_connected.len();
    if gp_count > 0 {
        ctx.set_fill_style_str("#00ff88");
        ctx.set_font("14px monospace");
        let _ = ctx.fill_text(
            &format!("{} gamepad{} connected", gp_count, if gp_count > 1 { "s" } else { "" }),
            CANVAS_WIDTH / 2.0, 540.0,
        );
    }

    ctx.set_fill_style_str("rgba(255,255,255,0.3)");
    ctx.set_font("14px monospace");
    if input.is_touch_device {
        let _ = ctx.fill_text("Tap to select", CANVAS_WIDTH / 2.0, 580.0);
    } else {
        let _ = ctx.fill_text("Press 1/2/3 or use D-pad + A", CANVAS_WIDTH / 2.0, 580.0);
    }
    ctx.set_text_align("start");
}

fn draw_settings(ctx: &CanvasRenderingContext2d, app: &AppState, input: &InputManager) {
    ctx.set_fill_style_str("#1a1a2e");
    ctx.fill_rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT);

    ctx.set_text_align("center");
    ctx.set_fill_style_str("#e94560");
    ctx.set_font("bold 48px monospace");
    let _ = ctx.fill_text("CONTROLS", CANVAS_WIDTH / 2.0, 60.0);

    let panels = [CANVAS_WIDTH / 4.0, CANVAS_WIDTH * 3.0 / 4.0];
    let colors = ["#e94560", "#00d2ff"];
    let select_labels = ["Press 1-9 to rebind", "Press F1-F9 to rebind"];

    for (p, &panel_x) in panels.iter().enumerate() {
        // Panel header
        ctx.set_fill_style_str(colors[p]);
        ctx.set_font("bold 24px monospace");
        let _ = ctx.fill_text(&format!("PLAYER {}", p + 1), panel_x, 110.0);

        ctx.set_fill_style_str("rgba(255,255,255,0.3)");
        ctx.set_font("12px monospace");
        let _ = ctx.fill_text(select_labels[p], panel_x, 135.0);

        // Action rows
        for i in 0..ACTION_COUNT as usize {
            let y = 170.0 + i as f64 * 42.0;
            let is_rebinding = app.rebind_target == Some((p, i as u8));

            // Row background
            if is_rebinding {
                ctx.set_fill_style_str("rgba(233, 69, 96, 0.2)");
            } else {
                ctx.set_fill_style_str("rgba(255, 255, 255, 0.03)");
            }
            ctx.fill_rect(panel_x - 180.0, y - 15.0, 360.0, 36.0);

            // Slot number
            ctx.set_fill_style_str("rgba(255,255,255,0.3)");
            ctx.set_font("12px monospace");
            let slot = if p == 0 { format!("[{}]", i + 1) } else { format!("[F{}]", i + 1) };
            let _ = ctx.fill_text(&slot, panel_x - 160.0, y + 5.0);

            // Action name
            ctx.set_fill_style_str("rgba(255,255,255,0.7)");
            ctx.set_font("16px monospace");
            let _ = ctx.fill_text(ACTION_NAMES[i], panel_x - 60.0, y + 5.0);

            // Current key
            if is_rebinding {
                ctx.set_fill_style_str("#e94560");
                ctx.set_font("bold 16px monospace");
                let _ = ctx.fill_text("PRESS KEY...", panel_x + 80.0, y + 5.0);
            } else {
                ctx.set_fill_style_str("#ffffff");
                ctx.set_font("bold 16px monospace");
                let display = InputManager::key_display(&input.players[p].keyboard.keys[i]);
                let _ = ctx.fill_text(display, panel_x + 80.0, y + 5.0);
            }
        }

        // Gamepad status
        let gp_status = if let Some(idx) = input.players[p].gamepad_index {
            format!("Gamepad {} connected", idx)
        } else {
            "No gamepad".to_string()
        };
        ctx.set_fill_style_str(if input.players[p].gamepad_index.is_some() { "#00ff88" } else { "rgba(255,255,255,0.3)" });
        ctx.set_font("13px monospace");
        let _ = ctx.fill_text(&gp_status, panel_x, 570.0);
    }

    // Gamepad mapping reference
    ctx.set_fill_style_str("rgba(255,255,255,0.4)");
    ctx.set_font("12px monospace");
    let _ = ctx.fill_text("Gamepad: A=ATK  B=SPC  X=GRAB  Y=SMASH  RB=SHIELD  Stick/DPad=Move", CANVAS_WIDTH / 2.0, 620.0);

    // Bottom controls
    ctx.set_fill_style_str("rgba(255,255,255,0.5)");
    ctx.set_font("14px monospace");
    let _ = ctx.fill_text("R - Reset Defaults  |  ESC - Back to Menu", CANVAS_WIDTH / 2.0, CANVAS_HEIGHT - 20.0);
    ctx.set_text_align("start");
}

fn draw_online_lobby(ctx: &CanvasRenderingContext2d, app: &AppState) {
    ctx.set_fill_style_str("#1a1a2e");
    ctx.fill_rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT);

    ctx.set_text_align("center");
    ctx.set_fill_style_str("#e94560");
    ctx.set_font("bold 48px monospace");
    let _ = ctx.fill_text("ONLINE MATCH", CANVAS_WIDTH / 2.0, 200.0);

    let status = if let Some(ref net) = app.net {
        match net.connection_state {
            ConnectionState::Connecting => "Connecting to server...",
            ConnectionState::WaitingForPeer => "Waiting for opponent...",
            ConnectionState::Connected => "Opponent found!",
            ConnectionState::Disconnected => "Disconnected",
        }
    } else {
        "Not connected"
    };

    ctx.set_fill_style_str("#00d2ff");
    ctx.set_font("24px monospace");
    let _ = ctx.fill_text(status, CANVAS_WIDTH / 2.0, 350.0);

    // Animated dots for waiting states
    if let Some(ref net) = app.net {
        if matches!(net.connection_state, ConnectionState::Connecting | ConnectionState::WaitingForPeer) {
            ctx.set_fill_style_str("rgba(255,255,255,0.3)");
            ctx.set_font("24px monospace");
            let dots = ".".repeat((js_sys::Date::now() as u64 / 500 % 4) as usize);
            let _ = ctx.fill_text(&dots, CANVAS_WIDTH / 2.0, 390.0);
        }
    }

    ctx.set_fill_style_str("rgba(255,255,255,0.4)");
    ctx.set_font("14px monospace");
    let _ = ctx.fill_text("Press ESC to go back", CANVAS_WIDTH / 2.0, 500.0);
    ctx.set_text_align("start");
}

fn draw_fight_scene(ctx: &CanvasRenderingContext2d, state: &GameState) {
    // Clear
    ctx.set_fill_style_str("#16213e");
    ctx.fill_rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT);

    // Stage platform
    ctx.set_fill_style_str("#0f3460");
    let stage_x = STAGE_LEFT.to_int() as f64;
    let stage_w = (STAGE_RIGHT.to_int() - STAGE_LEFT.to_int()) as f64;
    let ground = GROUND_Y.to_int() as f64;
    ctx.fill_rect(stage_x, ground, stage_w, 20.0);

    // Ledge indicators
    ctx.set_fill_style_str("#e94560");
    ctx.fill_rect(stage_x - 4.0, ground - 8.0, 8.0, 28.0);
    ctx.fill_rect(stage_x + stage_w - 4.0, ground - 8.0, 8.0, 28.0);

    // Projectiles
    for proj in state.projectiles.iter() {
        if !proj.active { continue; }
        let px = proj.position_x.to_int() as f64;
        let py = proj.position_y.to_int() as f64;
        let size = PROJECTILE_SIZE.to_int() as f64;
        let color = if proj.owner == 0 { "#ff6b6b" } else { "#6bc5ff" };
        ctx.set_fill_style_str(color);
        ctx.begin_path();
        let _ = ctx.arc(px, py, size / 2.0, 0.0, std::f64::consts::TAU);
        ctx.fill();
    }

    // Players
    let colors = ["#e94560", "#00d2ff"];
    let shield_colors = ["rgba(233, 69, 96, 0.4)", "rgba(0, 210, 255, 0.4)"];

    for (i, player) in state.players.iter().enumerate() {
        if player.stocks == 0 { continue; }

        let px = player.position_x.to_int() as f64;
        let py = player.position_y.to_int() as f64;
        let pw = PLAYER_WIDTH.to_int() as f64;
        let ph = PLAYER_HEIGHT.to_int() as f64;

        // Invincibility flash
        if player.invincibility_frames > 0 && state.frame % 4 < 2 {
            ctx.set_global_alpha(0.3);
        }

        // Dodge visual — translucent during dodge
        if matches!(player.action, ActionState::SpotDodge { .. } | ActionState::Rolling { .. } | ActionState::AirDodge { .. }) {
            ctx.set_global_alpha(0.4);
        }

        // Player body
        ctx.set_fill_style_str(colors[i]);
        ctx.fill_rect(px - pw / 2.0, py - ph, pw, ph);

        // Ledge hang indicator
        if matches!(player.action, ActionState::LedgeHang) {
            ctx.set_fill_style_str("rgba(255,255,255,0.5)");
            ctx.set_font("12px monospace");
            ctx.set_text_align("center");
            let _ = ctx.fill_text("HANG", px, py - ph - 5.0);
            ctx.set_text_align("start");
        }

        // Freefall indicator
        if matches!(player.action, ActionState::Freefall) {
            ctx.set_fill_style_str("rgba(255,100,100,0.5)");
            ctx.set_font("10px monospace");
            ctx.set_text_align("center");
            let _ = ctx.fill_text("FALL", px, py - ph - 5.0);
            ctx.set_text_align("start");
        }

        // Special move indicator
        if matches!(player.action, ActionState::SpecialMove { .. }) {
            ctx.set_fill_style_str("rgba(255,200,0,0.5)");
            ctx.set_font("10px monospace");
            ctx.set_text_align("center");
            let _ = ctx.fill_text("UP-B", px, py - ph - 5.0);
            ctx.set_text_align("start");
        }

        // Crouch visual — squish the player rectangle
        if player.is_crouching {
            ctx.set_fill_style_str(colors[i]);
            ctx.fill_rect(px - pw / 2.0 - 3.0, py - ph * 0.6, pw + 6.0, ph * 0.6);
        }

        // Running trail
        if player.is_running {
            ctx.set_fill_style_str("rgba(255,255,255,0.15)");
            let trail_dir = if player.facing_right { -1.0 } else { 1.0 };
            for t in 1..4 {
                let alpha = 0.15 - t as f64 * 0.04;
                ctx.set_global_alpha(alpha);
                ctx.fill_rect(px - pw / 2.0 + trail_dir * t as f64 * 8.0, py - ph, pw, ph);
            }
            ctx.set_global_alpha(1.0);
        }

        // Face direction indicator
        ctx.set_fill_style_str("#ffffff");
        ctx.begin_path();
        if player.facing_right {
            ctx.move_to(px + pw / 2.0 - 2.0, py - ph + 10.0);
            ctx.line_to(px + pw / 2.0 + 6.0, py - ph + 17.0);
            ctx.line_to(px + pw / 2.0 - 2.0, py - ph + 24.0);
        } else {
            ctx.move_to(px - pw / 2.0 + 2.0, py - ph + 10.0);
            ctx.line_to(px - pw / 2.0 - 6.0, py - ph + 17.0);
            ctx.line_to(px - pw / 2.0 + 2.0, py - ph + 24.0);
        }
        ctx.fill();

        // Attack hitbox (using attack_data for current attack type)
        if matches!(player.action, ActionState::AttackActive { .. }) {
            let data = attack_data(player.current_attack);
            let facing = if player.current_attack == AttackType::BackAir {
                !player.facing_right
            } else {
                player.facing_right
            };
            let hb_w = data.hitbox_w.to_int() as f64;
            let hb_h = data.hitbox_h.to_int() as f64;
            let hb_offset_x = data.hitbox_offset_x.to_int() as f64;
            let hb_offset_y = data.hitbox_offset_y.to_int() as f64;
            let hb_x = if facing { px + hb_offset_x } else { px - hb_offset_x - hb_w };
            let hitbox_color = match player.current_attack {
                // Jab/tilts — yellow
                AttackType::Jab | AttackType::ForwardTilt | AttackType::UpTilt | AttackType::DownTilt
                    => "rgba(255, 255, 0, 0.5)",
                // Smash attacks — orange/red (heavy)
                AttackType::ForwardSmash | AttackType::UpSmash | AttackType::DownSmash
                    => "rgba(255, 100, 30, 0.6)",
                // Aerials — cyan
                AttackType::NeutralAir | AttackType::ForwardAir | AttackType::BackAir | AttackType::UpAir
                    => "rgba(0, 220, 255, 0.5)",
                // Meteor smash (dair) — purple
                AttackType::DownAir => "rgba(180, 50, 255, 0.6)",
                // Dash attack — green
                AttackType::DashAttack => "rgba(100, 255, 100, 0.5)",
                // Side special — magenta
                AttackType::SideSpecial => "rgba(255, 50, 200, 0.5)",
            };
            ctx.set_fill_style_str(hitbox_color);
            ctx.fill_rect(hb_x, py + hb_offset_y, hb_w, hb_h);
        }

        // Startup flash
        if matches!(player.action, ActionState::AttackStartup { .. }) {
            ctx.set_fill_style_str("rgba(255, 200, 0, 0.2)");
            ctx.fill_rect(px - pw / 2.0 - 2.0, py - ph - 2.0, pw + 4.0, ph + 4.0);
        }

        // Grab visualization
        if matches!(player.action, ActionState::Grabbing { .. }) {
            let data = attack_data(AttackType::Jab); // grab uses jab hitbox dimensions
            let hb_w = data.hitbox_w.to_int() as f64;
            let hb_h = data.hitbox_h.to_int() as f64;
            let hb_offset_x = data.hitbox_offset_x.to_int() as f64;
            let hb_offset_y = data.hitbox_offset_y.to_int() as f64;
            let hb_x = if player.facing_right { px + hb_offset_x } else { px - hb_offset_x - hb_w };
            ctx.set_fill_style_str("rgba(0, 255, 0, 0.5)");
            ctx.fill_rect(hb_x, py + hb_offset_y, hb_w, hb_h);
        }

        // Shield bubble
        if matches!(player.action, ActionState::Shielding) {
            ctx.set_fill_style_str(shield_colors[i]);
            let shield_ratio = player.shield_hp as f64 / SHIELD_MAX_HP as f64;
            let shield_radius = 35.0 * shield_ratio;
            ctx.begin_path();
            let _ = ctx.arc(px, py - ph / 2.0, shield_radius, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }

        // Shield stun stars
        if matches!(player.action, ActionState::ShieldStun { .. }) {
            ctx.set_fill_style_str("rgba(255, 255, 0, 0.6)");
            for s in 0..3 {
                let angle = (state.frame as f64 * 0.1) + (s as f64 * std::f64::consts::TAU / 3.0);
                let sx = px + angle.cos() * 25.0;
                let sy = py - ph - 10.0 + angle.sin() * 10.0;
                ctx.fill_rect(sx - 3.0, sy - 3.0, 6.0, 6.0);
            }
        }

        // Hitstun flash
        if matches!(player.action, ActionState::Hitstun { .. }) {
            ctx.set_fill_style_str("rgba(255, 255, 255, 0.3)");
            ctx.fill_rect(px - pw / 2.0, py - ph, pw, ph);
        }

        ctx.set_global_alpha(1.0);
    }

    // HUD
    let colors = ["#e94560", "#00d2ff"];
    for (i, player) in state.players.iter().enumerate() {
        let hud_x = if i == 0 { 160.0 } else { CANVAS_WIDTH - 320.0 };

        // Character name + player label
        ctx.set_fill_style_str(colors[i]);
        ctx.set_font("bold 16px monospace");
        let label = format!("P{} {}", i + 1, char_name(player.character));
        let _ = ctx.fill_text(&label, hud_x, CANVAS_HEIGHT - 65.0);

        // Damage percent
        let dmg = player.damage_percent;
        let r = 255.0_f64.min(180.0 + dmg as f64 * 0.75) as u8;
        let g_val = if dmg > 127 { 0u8 } else { 255.0_f64.min(255.0 - dmg as f64 * 2.0) as u8 };
        ctx.set_fill_style_str(&format!("rgb({r}, {g_val}, 80)"));
        ctx.set_font("bold 36px monospace");
        let _ = ctx.fill_text(&format!("{}%", dmg), hud_x + 30.0, CANVAS_HEIGHT - 30.0);

        // Stock icons
        for s in 0..player.stocks {
            ctx.set_fill_style_str(colors[i]);
            ctx.begin_path();
            let _ = ctx.arc(hud_x + 40.0 + s as f64 * 20.0, CANVAS_HEIGHT - 10.0, 6.0, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
    }

    // Title
    ctx.set_fill_style_str("#e94560");
    ctx.set_font("bold 18px monospace");
    ctx.set_text_align("center");
    let _ = ctx.fill_text("EJKORE", CANVAS_WIDTH / 2.0, 25.0);

    // Frame counter
    ctx.set_fill_style_str("rgba(255,255,255,0.3)");
    ctx.set_font("12px monospace");
    let _ = ctx.fill_text(&format!("F{}", state.frame), CANVAS_WIDTH / 2.0, 45.0);
    ctx.set_text_align("start");

    // Controls hint
    ctx.set_fill_style_str("rgba(255,255,255,0.3)");
    ctx.set_font("11px monospace");
    let _ = ctx.fill_text("P1: WASD J(atk) K(spc) L(shld) ;(grab) H(smash) | P2: Arrows 1(atk) 2(spc) 3(shld) 0(grab) 4(smash) | R: restart", 10.0, CANVAS_HEIGHT - 3.0);
}

fn draw_fight(ctx: &CanvasRenderingContext2d, app: &AppState) {
    draw_fight_scene(ctx, &app.game);

    // Online-specific overlays
    if app.mode == GameMode::Online {
        // Override controls text
        ctx.set_fill_style_str("#16213e");
        ctx.fill_rect(0.0, CANVAS_HEIGHT - 15.0, CANVAS_WIDTH, 15.0);
        ctx.set_fill_style_str("rgba(255,255,255,0.3)");
        ctx.set_font("11px monospace");
        let _ = ctx.fill_text("WASD J(atk) K(spc) L(shld) ;(grab) H(smash)", 10.0, CANVAS_HEIGHT - 3.0);

        // Rollback stats
        if let Some(ref rb) = app.rollback {
            ctx.set_fill_style_str("rgba(255,255,255,0.4)");
            ctx.set_font("11px monospace");
            ctx.set_text_align("right");
            let _ = ctx.fill_text(
                &format!("RB:{} CF:{}", rb.last_rollback_count, rb.last_confirmed_frame),
                CANVAS_WIDTH - 10.0, 25.0,
            );
            if rb.desync_detected {
                ctx.set_fill_style_str("#ff0000");
                ctx.set_font("bold 14px monospace");
                let _ = ctx.fill_text("DESYNC!", CANVAS_WIDTH - 10.0, 45.0);
            }
            ctx.set_text_align("start");
        }

        // Connection state
        if let Some(ref net) = app.net {
            if net.connection_state == ConnectionState::Disconnected {
                ctx.set_fill_style_str("rgba(255, 0, 0, 0.8)");
                ctx.set_font("bold 24px monospace");
                ctx.set_text_align("center");
                let _ = ctx.fill_text("OPPONENT DISCONNECTED", CANVAS_WIDTH / 2.0, 80.0);
                ctx.set_text_align("start");
            }
        }

        // Player indicator
        ctx.set_fill_style_str("rgba(255,255,255,0.3)");
        ctx.set_font("11px monospace");
        let _ = ctx.fill_text(&format!("You: P{}", app.local_player + 1), 10.0, 25.0);
    }
}

fn draw_game_over(ctx: &CanvasRenderingContext2d, state: &GameState) {
    // Draw the fight scene behind the overlay (create a minimal AppState view)
    draw_fight_scene(ctx, state);

    ctx.set_fill_style_str("rgba(0, 0, 0, 0.7)");
    ctx.fill_rect(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT);

    ctx.set_text_align("center");

    ctx.set_fill_style_str("#e94560");
    ctx.set_font("bold 48px monospace");
    let _ = ctx.fill_text("GAME!", CANVAS_WIDTH / 2.0, CANVAS_HEIGHT / 2.0 - 40.0);

    let colors = ["#e94560", "#00d2ff"];
    if let Some(winner) = state.winner {
        ctx.set_fill_style_str(colors[winner as usize]);
        ctx.set_font("bold 32px monospace");
        let text = format!("Player {} ({}) wins!",
            winner + 1,
            char_name(state.players[winner as usize].character));
        let _ = ctx.fill_text(&text, CANVAS_WIDTH / 2.0, CANVAS_HEIGHT / 2.0 + 10.0);
    }

    ctx.set_fill_style_str("rgba(255,255,255,0.6)");
    ctx.set_font("18px monospace");
    let _ = ctx.fill_text("Press R / Tap / Press A to continue", CANVAS_WIDTH / 2.0, CANVAS_HEIGHT / 2.0 + 60.0);
    ctx.set_text_align("start");
}
