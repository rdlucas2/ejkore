use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent};

use ejkore_game::state::*;

const CANVAS_WIDTH: f64 = 1280.0;
const CANVAS_HEIGHT: f64 = 720.0;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    CharSelect,
    Fighting,
    GameOver,
}

struct AppState {
    screen: Screen,
    game: GameState,
    p1_char: CharacterId,
    p2_char: CharacterId,
    p1_char_idx: usize,
    p2_char_idx: usize,
    p1_ready: bool,
    p2_ready: bool,
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

impl AppState {
    fn new() -> Self {
        Self {
            screen: Screen::CharSelect,
            game: default_state(),
            p1_char: CharacterId::Balanced,
            p2_char: CharacterId::Balanced,
            p1_char_idx: 0,
            p2_char_idx: 0,
            p1_ready: false,
            p2_ready: false,
        }
    }

    fn start_game(&mut self) {
        self.game = default_state();
        self.game.players[0].character = self.p1_char;
        self.game.players[1].character = self.p2_char;
        self.screen = Screen::Fighting;
    }
}

struct InputState {
    keys: std::collections::HashSet<String>,
}

impl InputState {
    fn new() -> Self {
        Self { keys: std::collections::HashSet::new() }
    }

    // Player 1: WASD + J(attack) K(special) L(shield) ;(grab) H(smash)
    fn player1_input(&self) -> PlayerInput {
        let mut bits: u16 = 0;
        if self.keys.contains("a") || self.keys.contains("A") { bits |= PlayerInput::LEFT; }
        if self.keys.contains("d") || self.keys.contains("D") { bits |= PlayerInput::RIGHT; }
        if self.keys.contains("w") || self.keys.contains("W") { bits |= PlayerInput::UP; }
        if self.keys.contains("s") || self.keys.contains("S") { bits |= PlayerInput::DOWN; }
        if self.keys.contains("j") || self.keys.contains("J") { bits |= PlayerInput::ATTACK; }
        if self.keys.contains("k") || self.keys.contains("K") { bits |= PlayerInput::SPECIAL; }
        if self.keys.contains("l") || self.keys.contains("L") { bits |= PlayerInput::SHIELD; }
        if self.keys.contains(";") { bits |= PlayerInput::GRAB; }
        if self.keys.contains("h") || self.keys.contains("H") { bits |= PlayerInput::SMASH; }
        PlayerInput(bits)
    }

    // Player 2: Arrow keys + Numpad 1(attack) 2(special) 3(shield) 0(grab) 4(smash)
    fn player2_input(&self) -> PlayerInput {
        let mut bits: u16 = 0;
        if self.keys.contains("ArrowLeft") { bits |= PlayerInput::LEFT; }
        if self.keys.contains("ArrowRight") { bits |= PlayerInput::RIGHT; }
        if self.keys.contains("ArrowUp") { bits |= PlayerInput::UP; }
        if self.keys.contains("ArrowDown") { bits |= PlayerInput::DOWN; }
        if self.keys.contains("1") { bits |= PlayerInput::ATTACK; }
        if self.keys.contains("2") { bits |= PlayerInput::SPECIAL; }
        if self.keys.contains("3") { bits |= PlayerInput::SHIELD; }
        if self.keys.contains("0") { bits |= PlayerInput::GRAB; }
        if self.keys.contains("4") { bits |= PlayerInput::SMASH; }
        PlayerInput(bits)
    }
}

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

    let input_state = Rc::new(RefCell::new(InputState::new()));
    let app_state = Rc::new(RefCell::new(AppState::new()));

    // Keyboard event listeners
    {
        let input = input_state.clone();
        let keydown = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            e.prevent_default();
            input.borrow_mut().keys.insert(e.key());
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

    // Character select / game control keys
    {
        let app = app_state.clone();
        let handler = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            let mut app = app.borrow_mut();
            match app.screen {
                Screen::CharSelect => {
                    match e.key().as_str() {
                        // P1: A/D to cycle, J to confirm
                        "a" | "A" => {
                            if !app.p1_ready {
                                app.p1_char_idx = (app.p1_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                                app.p1_char = CHAR_LIST[app.p1_char_idx];
                            }
                        }
                        "d" | "D" => {
                            if !app.p1_ready {
                                app.p1_char_idx = (app.p1_char_idx + 1) % CHAR_LIST.len();
                                app.p1_char = CHAR_LIST[app.p1_char_idx];
                            }
                        }
                        "j" | "J" => { app.p1_ready = !app.p1_ready; }
                        // P2: Arrow left/right to cycle, 1 to confirm
                        "ArrowLeft" => {
                            if !app.p2_ready {
                                app.p2_char_idx = (app.p2_char_idx + CHAR_LIST.len() - 1) % CHAR_LIST.len();
                                app.p2_char = CHAR_LIST[app.p2_char_idx];
                            }
                        }
                        "ArrowRight" => {
                            if !app.p2_ready {
                                app.p2_char_idx = (app.p2_char_idx + 1) % CHAR_LIST.len();
                                app.p2_char = CHAR_LIST[app.p2_char_idx];
                            }
                        }
                        "1" => { app.p2_ready = !app.p2_ready; }
                        _ => {}
                    }
                    if app.p1_ready && app.p2_ready {
                        app.start_game();
                    }
                }
                Screen::GameOver => {
                    if e.key() == "r" || e.key() == "R" {
                        app.screen = Screen::CharSelect;
                        app.p1_ready = false;
                        app.p2_ready = false;
                    }
                }
                Screen::Fighting => {
                    if e.key() == "r" || e.key() == "R" {
                        app.start_game();
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

        if app.screen == Screen::Fighting {
            while *accumulator.borrow() >= FRAME_TIME {
                *accumulator.borrow_mut() -= FRAME_TIME;
                let input = input_state.borrow();
                let inputs = [input.player1_input(), input.player2_input()];
                drop(input);
                advance_frame(&mut app.game, inputs);
            }
            if app.game.match_over {
                app.screen = Screen::GameOver;
            }
        } else {
            *accumulator.borrow_mut() = 0.0;
        }

        match app.screen {
            Screen::CharSelect => draw_char_select(&ctx, &app),
            Screen::Fighting => draw_fight(&ctx, &app.game),
            Screen::GameOver => draw_game_over(&ctx, &app.game),
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
    let _ = ctx.fill_text("P1: A/D to pick, J to confirm  |  P2: Left/Right to pick, 1 to confirm", CANVAS_WIDTH / 2.0, CANVAS_HEIGHT - 20.0);
    ctx.set_text_align("start");
}

fn draw_fight(ctx: &CanvasRenderingContext2d, state: &GameState) {
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
            ctx.set_fill_style_str("rgba(255, 255, 0, 0.5)");
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

    // Controls
    ctx.set_fill_style_str("rgba(255,255,255,0.3)");
    ctx.set_font("11px monospace");
    let _ = ctx.fill_text("P1: WASD J(atk) K(spc) L(shld) ;(grab) H(smash) | P2: Arrows 1(atk) 2(spc) 3(shld) 0(grab) 4(smash) | R: restart", 10.0, CANVAS_HEIGHT - 3.0);
}

fn draw_game_over(ctx: &CanvasRenderingContext2d, state: &GameState) {
    draw_fight(ctx, state);

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
    let _ = ctx.fill_text("Press R to return to character select", CANVAS_WIDTH / 2.0, CANVAS_HEIGHT / 2.0 + 60.0);
    ctx.set_text_align("start");
}
