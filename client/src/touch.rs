use ejkore_game::state::PlayerInput;
use web_sys::CanvasRenderingContext2d;

// Virtual button layout constants
const DPAD_CENTER_X: f64 = 150.0;
const DPAD_CENTER_Y: f64 = 520.0;
const DPAD_RADIUS: f64 = 55.0;
const DPAD_DEAD: f64 = 20.0; // dead zone in center of d-pad

const BTN_CENTER_X: f64 = 1130.0;
const BTN_CENTER_Y: f64 = 500.0;
const BTN_RADIUS: f64 = 38.0;
const BTN_SPACING: f64 = 90.0;

// Button positions relative to BTN_CENTER (diamond layout)
// Right = ATTACK, Top = SPECIAL, Left = SHIELD, Bottom = GRAB
const BTN_ATTACK: (f64, f64) = (BTN_SPACING, 0.0);
const BTN_SPECIAL: (f64, f64) = (0.0, -BTN_SPACING);
const BTN_SHIELD: (f64, f64) = (-BTN_SPACING, 0.0);
const BTN_GRAB: (f64, f64) = (0.0, BTN_SPACING);
// Smash button centered below the diamond
const BTN_SMASH_X: f64 = BTN_CENTER_X;
const BTN_SMASH_Y: f64 = BTN_CENTER_Y + BTN_SPACING + 60.0;

#[derive(Clone)]
pub struct ActiveTouch {
    pub id: i32,
    pub x: f64,
    pub y: f64,
}

pub struct TouchState {
    pub active_touches: Vec<ActiveTouch>,
}

impl TouchState {
    pub fn new() -> Self {
        Self {
            active_touches: Vec::new(),
        }
    }

    pub fn touch_start(&mut self, id: i32, x: f64, y: f64) {
        // Update existing or add new
        if let Some(t) = self.active_touches.iter_mut().find(|t| t.id == id) {
            t.x = x;
            t.y = y;
        } else {
            self.active_touches.push(ActiveTouch { id, x, y });
        }
    }

    pub fn touch_move(&mut self, id: i32, x: f64, y: f64) {
        if let Some(t) = self.active_touches.iter_mut().find(|t| t.id == id) {
            t.x = x;
            t.y = y;
        }
    }

    pub fn touch_end(&mut self, id: i32) {
        self.active_touches.retain(|t| t.id != id);
    }

    pub fn to_input(&self) -> u16 {
        let mut bits = 0u16;
        for touch in &self.active_touches {
            bits |= self.hit_test(touch.x, touch.y);
        }
        bits
    }

    fn hit_test(&self, x: f64, y: f64) -> u16 {
        let mut bits = 0u16;

        // D-pad check
        let dx = x - DPAD_CENTER_X;
        let dy = y - DPAD_CENTER_Y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > DPAD_DEAD && dist < DPAD_RADIUS * 2.0 {
            let angle = dy.atan2(dx);
            // Right: -45 to 45, Down: 45 to 135, Left: 135 to -135, Up: -135 to -45
            if angle.abs() < std::f64::consts::FRAC_PI_4 {
                bits |= PlayerInput::RIGHT;
            } else if angle > std::f64::consts::FRAC_PI_4 && angle < 3.0 * std::f64::consts::FRAC_PI_4 {
                bits |= PlayerInput::DOWN;
            } else if angle.abs() > 3.0 * std::f64::consts::FRAC_PI_4 {
                bits |= PlayerInput::LEFT;
            } else {
                bits |= PlayerInput::UP;
            }
            // Allow diagonals
            if angle.abs() > std::f64::consts::FRAC_PI_8 && angle.abs() < 7.0 * std::f64::consts::FRAC_PI_8 {
                if dy > 0.0 { bits |= PlayerInput::DOWN; }
                if dy < 0.0 { bits |= PlayerInput::UP; }
            }
            if angle.abs() < 3.0 * std::f64::consts::FRAC_PI_8 {
                bits |= PlayerInput::RIGHT;
            }
            if angle.abs() > 5.0 * std::f64::consts::FRAC_PI_8 {
                bits |= PlayerInput::LEFT;
            }
        }

        // Action buttons
        let check_btn = |bx: f64, by: f64| -> bool {
            let d = ((x - bx).powi(2) + (y - by).powi(2)).sqrt();
            d < BTN_RADIUS * 1.3 // slightly generous hit area
        };

        if check_btn(BTN_CENTER_X + BTN_ATTACK.0, BTN_CENTER_Y + BTN_ATTACK.1) {
            bits |= PlayerInput::ATTACK;
        }
        if check_btn(BTN_CENTER_X + BTN_SPECIAL.0, BTN_CENTER_Y + BTN_SPECIAL.1) {
            bits |= PlayerInput::SPECIAL;
        }
        if check_btn(BTN_CENTER_X + BTN_SHIELD.0, BTN_CENTER_Y + BTN_SHIELD.1) {
            bits |= PlayerInput::SHIELD;
        }
        if check_btn(BTN_CENTER_X + BTN_GRAB.0, BTN_CENTER_Y + BTN_GRAB.1) {
            bits |= PlayerInput::GRAB;
        }
        if check_btn(BTN_SMASH_X, BTN_SMASH_Y) {
            bits |= PlayerInput::SMASH;
        }

        bits
    }

    /// Returns which buttons are currently pressed (for visual feedback)
    fn pressed_set(&self) -> u16 {
        self.to_input()
    }
}

pub fn draw_touch_controls(ctx: &CanvasRenderingContext2d, touch: &TouchState) {
    let pressed = touch.pressed_set();

    // D-pad background
    ctx.set_fill_style_str("rgba(255, 255, 255, 0.08)");
    ctx.begin_path();
    let _ = ctx.arc(DPAD_CENTER_X, DPAD_CENTER_Y, DPAD_RADIUS * 1.8, 0.0, std::f64::consts::TAU);
    ctx.fill();

    // D-pad arrows
    draw_dpad_arrow(ctx, DPAD_CENTER_X, DPAD_CENTER_Y, 0.0, pressed & PlayerInput::RIGHT != 0);   // right
    draw_dpad_arrow(ctx, DPAD_CENTER_X, DPAD_CENTER_Y, 90.0, pressed & PlayerInput::DOWN != 0);    // down
    draw_dpad_arrow(ctx, DPAD_CENTER_X, DPAD_CENTER_Y, 180.0, pressed & PlayerInput::LEFT != 0);   // left
    draw_dpad_arrow(ctx, DPAD_CENTER_X, DPAD_CENTER_Y, 270.0, pressed & PlayerInput::UP != 0);     // up

    // Action buttons
    let buttons: [(f64, f64, &str, u16, &str); 5] = [
        (BTN_CENTER_X + BTN_ATTACK.0, BTN_CENTER_Y + BTN_ATTACK.1, "A", PlayerInput::ATTACK, "rgba(233, 69, 96,"),
        (BTN_CENTER_X + BTN_SPECIAL.0, BTN_CENTER_Y + BTN_SPECIAL.1, "B", PlayerInput::SPECIAL, "rgba(0, 210, 255,"),
        (BTN_CENTER_X + BTN_SHIELD.0, BTN_CENTER_Y + BTN_SHIELD.1, "L", PlayerInput::SHIELD, "rgba(100, 200, 100,"),
        (BTN_CENTER_X + BTN_GRAB.0, BTN_CENTER_Y + BTN_GRAB.1, "G", PlayerInput::GRAB, "rgba(200, 200, 100,"),
        (BTN_SMASH_X, BTN_SMASH_Y, "SM", PlayerInput::SMASH, "rgba(255, 150, 50,"),
    ];

    for (bx, by, label, bit, color_base) in &buttons {
        let is_pressed = pressed & bit != 0;
        let alpha = if is_pressed { "0.6)" } else { "0.25)" };
        ctx.set_fill_style_str(&format!("{}{}", color_base, alpha));
        ctx.begin_path();
        let _ = ctx.arc(*bx, *by, BTN_RADIUS, 0.0, std::f64::consts::TAU);
        ctx.fill();

        // Border
        let border_alpha = if is_pressed { "0.9)" } else { "0.4)" };
        ctx.set_stroke_style_str(&format!("{}{}", color_base, border_alpha));
        ctx.set_line_width(2.0);
        ctx.begin_path();
        let _ = ctx.arc(*bx, *by, BTN_RADIUS, 0.0, std::f64::consts::TAU);
        ctx.stroke();

        // Label
        let text_alpha = if is_pressed { "1.0" } else { "0.6" };
        ctx.set_fill_style_str(&format!("rgba(255, 255, 255, {})", text_alpha));
        ctx.set_font("bold 16px monospace");
        ctx.set_text_align("center");
        let _ = ctx.fill_text(label, *bx, *by + 6.0);
    }

    ctx.set_text_align("start");
}

fn draw_dpad_arrow(ctx: &CanvasRenderingContext2d, cx: f64, cy: f64, angle_deg: f64, is_pressed: bool) {
    let angle = angle_deg * std::f64::consts::PI / 180.0;
    let dist = DPAD_RADIUS * 0.9;
    let ax = cx + angle.cos() * dist;
    let ay = cy + angle.sin() * dist;

    let alpha = if is_pressed { "0.7)" } else { "0.3)" };
    ctx.set_fill_style_str(&format!("rgba(255, 255, 255, {}", alpha));
    ctx.begin_path();
    let _ = ctx.arc(ax, ay, 22.0, 0.0, std::f64::consts::TAU);
    ctx.fill();

    // Arrow symbol
    let arrow = match angle_deg as i32 {
        0 => ">",
        90 => "v",
        180 => "<",
        _ => "^",
    };
    let text_alpha = if is_pressed { "1.0" } else { "0.5" };
    ctx.set_fill_style_str(&format!("rgba(255, 255, 255, {})", text_alpha));
    ctx.set_font("bold 18px monospace");
    ctx.set_text_align("center");
    let _ = ctx.fill_text(arrow, ax, ay + 6.0);
}
