use crate::fixed::Fp;

/// Axis-aligned bounding box for hitboxes and hurtboxes.
/// (x, y) is the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: Fp,
    pub y: Fp,
    pub w: Fp,
    pub h: Fp,
}

impl Rect {
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

/// Data describing a single hit (from a hitbox or projectile).
#[derive(Debug, Clone, Copy)]
pub struct HitData {
    pub base_knockback: Fp,
    pub knockback_scaling: Fp,
    pub knockback_angle: i32, // degrees, 0 = right, 90 = up
    pub damage: Fp,
}

/// Calculate knockback magnitude.
///
/// Simplified Smash formula:
///   kb = base_kb + ((damage_percent + hit_damage) * kb_scaling / 100) * (200 / (weight + 100))
///
/// - `damage_percent`: victim's current damage (before this hit)
/// - `weight`: victim's weight (100 = standard)
pub fn calculate_knockback(hit: &HitData, damage_percent: u32, weight: Fp) -> Fp {
    let total_damage = Fp::from_int(damage_percent as i32) + hit.damage;
    let scaling = hit.knockback_scaling * total_damage / Fp::from_int(100);
    let weight_factor = Fp::from_int(200) / (weight + Fp::from_int(100));
    let base_kb = hit.base_knockback + scaling * weight_factor;

    // Rage-style multiplier: knockback ramps up faster past 100%
    if damage_percent > 100 {
        let excess = Fp::from_int((damage_percent - 100) as i32);
        // +1.5% per point over 100 (so at 200% it's 2.5x base)
        let bonus = Fp::ONE + excess * Fp::from_int(3) / Fp::from_int(200);
        base_kb * bonus
    } else {
        base_kb
    }
}

const MAX_DI_SHIFT: i32 = 9; // degrees

/// Apply Directional Influence to a knockback angle.
///
/// `di_x` and `di_y` are the held direction (-1, 0, or 1).
/// DI perpendicular to the launch angle has maximum effect.
/// Returns the adjusted angle in degrees.
pub fn apply_di(angle: i32, di_x: i32, di_y: i32) -> i32 {
    if di_x == 0 && di_y == 0 {
        return angle;
    }

    // DI is most effective perpendicular to launch angle.
    // Simplified: compute the perpendicular component of the DI direction
    // relative to the launch angle.
    //
    // Using integer approximation of sin/cos for determinism:
    // For a launch angle in degrees, the perpendicular direction is (angle + 90).
    // The DI effectiveness is the dot product of DI with the perpendicular.
    //
    // We use a lookup table for sin/cos to avoid floating point.
    let perp_angle = angle + 90;
    let perp_x = cos_deg(perp_angle);
    let perp_y = sin_deg(perp_angle);

    // Dot product of DI direction with perpendicular
    let dot = di_x * perp_x + di_y * perp_y;

    // Normalize: dot can range from roughly -100 to 100 (since our sin/cos return -100..100)
    // Scale to max shift
    let shift = dot * MAX_DI_SHIFT / 100;

    angle + shift
}

/// Integer sin approximation: returns sin(degrees) * 100.
/// Deterministic, no floating point.
pub fn sin_deg(deg: i32) -> i32 {
    // Normalize to 0-359
    let d = ((deg % 360) + 360) % 360;
    // Use symmetry and a small lookup for key angles
    match d {
        0 => 0,
        30 => 50,
        45 => 71,
        60 => 87,
        90 => 100,
        120 => 87,
        135 => 71,
        150 => 50,
        180 => 0,
        210 => -50,
        225 => -71,
        240 => -87,
        270 => -100,
        300 => -87,
        315 => -71,
        330 => -50,
        // Linear interpolation between known points for other angles
        _ => {
            // Rough approximation using quadrant math
            let quadrant = d / 90;
            let remainder = d % 90;
            let base = remainder * 100 / 90; // 0-100 linear ramp
            match quadrant {
                0 => base,
                1 => 100 - base + (base * 100 / 100 - base).min(0), // descend
                2 => -base,
                3 => -(100 - base),
                _ => 0,
            }
        }
    }
}

/// Integer cos approximation: returns cos(degrees) * 100.
pub fn cos_deg(deg: i32) -> i32 {
    sin_deg(deg + 90)
}
