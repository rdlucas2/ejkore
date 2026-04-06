use crate::combat::{calculate_knockback, cos_deg, sin_deg, apply_di, HitData, Rect};
use crate::fixed::Fp;

pub const MAX_PLAYERS: usize = 2;
pub const STARTING_STOCKS: u8 = 3;
pub const MAX_PROJECTILES: usize = 6; // max 3 per player (ranged)

// Stage dimensions (logical units, 1280x720 viewport)
pub const STAGE_WIDTH: Fp = Fp::from_int(800);
pub const GROUND_Y: Fp = Fp::from_int(600);
pub const STAGE_CENTER_X: Fp = Fp::from_int(640);
pub const STAGE_LEFT: Fp = Fp::from_int(240);
pub const STAGE_RIGHT: Fp = Fp::from_int(1040);
pub const SPAWN_OFFSET_X: Fp = Fp::from_int(200);
pub const RESPAWN_Y: Fp = Fp::from_int(100);

// Blast zones
pub const BLAST_LEFT: Fp = Fp::from_int(-300);
pub const BLAST_RIGHT: Fp = Fp::from_int(1580);
pub const BLAST_TOP: Fp = Fp::from_int(-300);
pub const BLAST_BOTTOM: Fp = Fp::from_int(1020);

// Movement
pub const DEFAULT_WALK_SPEED: Fp = Fp::from_int(4);
pub const DEFAULT_AIR_SPEED: Fp = Fp::from_int(3);
pub const DEFAULT_GRAVITY: Fp = Fp::from_raw(Fp::ONE.raw() / 2); // 0.5
pub const DEFAULT_JUMP_VELOCITY: Fp = Fp::from_int(-12);
pub const DEFAULT_DOUBLE_JUMP_VELOCITY: Fp = Fp::from_int(-10);
pub const DEFAULT_FAST_FALL_SPEED: Fp = Fp::from_int(8);
pub const MAX_JUMPS: u8 = 2;
pub const DEFAULT_WEIGHT: Fp = Fp::from_int(100);

// Respawn
pub const RESPAWN_INVINCIBILITY_FRAMES: u8 = 120;

// Shield
pub const SHIELD_MAX_HP: u16 = 100;
pub const SHIELD_REGEN_RATE: u16 = 1; // per frame when not shielding
pub const SHIELD_BREAK_STUN: u8 = 120; // 2 seconds
pub const SHIELD_DEPLETE_PER_HIT: u16 = 20;

// Dodge
pub const SPOT_DODGE_FRAMES: u8 = 20;
pub const SPOT_DODGE_INVINCIBLE: u8 = 14;
pub const ROLL_FRAMES: u8 = 22;
pub const ROLL_INVINCIBLE: u8 = 16;
pub const ROLL_DISTANCE: Fp = Fp::from_int(80);
pub const AIR_DODGE_FRAMES: u8 = 18;
pub const AIR_DODGE_INVINCIBLE: u8 = 12;
pub const AIR_DODGE_SPEED: Fp = Fp::from_int(6);

// Ledge
pub const LEDGE_GRAB_RANGE_X: Fp = Fp::from_int(20);
pub const LEDGE_GRAB_RANGE_Y: Fp = Fp::from_int(30);
pub const LEDGE_INVINCIBILITY: u8 = 30;

// Running
pub const RUN_START_FRAMES: u8 = 8; // frames of holding direction before run starts
pub const CROUCH_DROP_FRAMES: u8 = 6; // frames of crouching before dropping through
pub const COUNTER_FRAMES: u8 = 20; // counter active window
pub const LANDING_LAG_FRAMES: u8 = 8; // frames of lag when landing during aerial
pub const METEOR_BOUNCE_THRESHOLD: Fp = Fp::from_int(4); // minimum downward velocity for ground bounce

// Up-B recovery
pub const UP_SPECIAL_FRAMES: u8 = 20;
pub const UP_SPECIAL_VELOCITY: Fp = Fp::from_int(-10); // strong upward launch

// Knockback scaling — divides raw knockback to get velocity (higher = gentler launches)
pub const KB_VELOCITY_SCALE: Fp = Fp::from_int(8);

// Player hurtbox size
pub const PLAYER_WIDTH: Fp = Fp::from_int(40);
pub const PLAYER_HEIGHT: Fp = Fp::from_int(60);

// Default jab attack data
pub const JAB_STARTUP: u8 = 3;
pub const JAB_ACTIVE: u8 = 3;
pub const JAB_RECOVERY: u8 = 8;
pub const JAB_DAMAGE: Fp = Fp::from_int(5);
pub const JAB_BASE_KB: Fp = Fp::from_int(20);
pub const JAB_KB_SCALING: Fp = Fp::from_int(50);
pub const JAB_KB_ANGLE: i32 = 45;
pub const JAB_HITBOX_W: Fp = Fp::from_int(35);
pub const JAB_HITBOX_H: Fp = Fp::from_int(20);
pub const JAB_HITBOX_OFFSET_X: Fp = Fp::from_int(20);
pub const JAB_HITBOX_OFFSET_Y: Fp = Fp::from_int(-30);

// Projectile
pub const PROJECTILE_SPEED: Fp = Fp::from_int(8);
pub const PROJECTILE_LIFETIME: u8 = 90; // 1.5 seconds
pub const PROJECTILE_DAMAGE: Fp = Fp::from_int(8);
pub const PROJECTILE_BASE_KB: Fp = Fp::from_int(15);
pub const PROJECTILE_KB_SCALING: Fp = Fp::from_int(40);
pub const PROJECTILE_SIZE: Fp = Fp::from_int(12);
pub const MAX_PROJECTILES_PER_PLAYER: usize = 2;

/// Bitfield for player input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct PlayerInput(pub u16);

impl PlayerInput {
    pub const LEFT: u16 = 1 << 0;
    pub const RIGHT: u16 = 1 << 1;
    pub const UP: u16 = 1 << 2;
    pub const DOWN: u16 = 1 << 3;
    pub const ATTACK: u16 = 1 << 4;
    pub const SPECIAL: u16 = 1 << 5;
    pub const SHIELD: u16 = 1 << 6;
    pub const GRAB: u16 = 1 << 7;
    pub const SMASH: u16 = 1 << 8;

    pub fn pressed(self, button: u16) -> bool {
        self.0 & button != 0
    }

    pub fn di_x(self) -> i32 {
        if self.pressed(Self::RIGHT) { 1 }
        else if self.pressed(Self::LEFT) { -1 }
        else { 0 }
    }

    pub fn di_y(self) -> i32 {
        if self.pressed(Self::UP) { -1 }
        else if self.pressed(Self::DOWN) { 1 }
        else { 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CharacterId {
    #[default]
    Balanced,
    Ranged,
    Rushdown,
}

pub struct CharacterStats {
    pub weight: Fp,
    pub walk_speed: Fp,
    pub run_speed: Fp,
    pub air_speed: Fp,
    pub gravity: Fp,
    pub jump_velocity: Fp,
    pub double_jump_velocity: Fp,
    pub fast_fall_speed: Fp,
    pub max_projectiles: usize,
    pub projectile_speed: Fp,
    pub projectile_lifetime: u8,
}

pub fn character_stats(id: CharacterId) -> CharacterStats {
    match id {
        CharacterId::Balanced => CharacterStats {
            weight: Fp::from_int(100),
            walk_speed: Fp::from_int(4),
            run_speed: Fp::from_int(7),
            air_speed: Fp::from_int(3),
            gravity: Fp::from_raw(Fp::ONE.raw() / 2), // 0.5
            jump_velocity: Fp::from_int(-12),
            double_jump_velocity: Fp::from_int(-10),
            fast_fall_speed: Fp::from_int(8),
            max_projectiles: 2,
            projectile_speed: PROJECTILE_SPEED,
            projectile_lifetime: PROJECTILE_LIFETIME,
        },
        CharacterId::Ranged => CharacterStats {
            weight: Fp::from_int(95),
            walk_speed: Fp::from_int(3),
            run_speed: Fp::from_int(6),
            air_speed: Fp::from_int(2),
            gravity: Fp::from_raw(Fp::ONE.raw() * 45 / 100), // 0.45
            jump_velocity: Fp::from_int(-11),
            double_jump_velocity: Fp::from_int(-9),
            fast_fall_speed: Fp::from_int(7),
            max_projectiles: 3,
            projectile_speed: Fp::from_int(12), // faster projectile
            projectile_lifetime: PROJECTILE_LIFETIME,
        },
        CharacterId::Rushdown => CharacterStats {
            weight: Fp::from_int(82),
            walk_speed: Fp::from_int(6),
            run_speed: Fp::from_int(10),
            air_speed: Fp::from_int(4),
            gravity: Fp::from_raw(Fp::ONE.raw() * 60 / 100), // 0.6
            jump_velocity: Fp::from_int(-13),
            double_jump_velocity: Fp::from_int(-11),
            fast_fall_speed: Fp::from_int(10),
            max_projectiles: 1,
            projectile_speed: Fp::from_int(10), // fast but short range
            projectile_lifetime: 45, // half the normal lifetime
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum AttackType {
    #[default]
    Jab,
    ForwardTilt,
    UpTilt,
    DownTilt,
    ForwardSmash,
    UpSmash,
    DownSmash,
    DashAttack,
    NeutralAir,
    ForwardAir,
    BackAir,
    UpAir,
    DownAir,
    SideSpecial,
}

/// Frame data for an attack type
pub struct AttackData {
    pub startup: u8,
    pub active: u8,
    pub recovery: u8,
    pub damage: Fp,
    pub base_kb: Fp,
    pub kb_scaling: Fp,
    pub kb_angle: i32,
    pub hitbox_w: Fp,
    pub hitbox_h: Fp,
    pub hitbox_offset_x: Fp,
    pub hitbox_offset_y: Fp,
}

pub fn attack_data(attack_type: AttackType) -> AttackData {
    match attack_type {
        AttackType::Jab => AttackData {
            startup: JAB_STARTUP, active: JAB_ACTIVE, recovery: JAB_RECOVERY,
            damage: JAB_DAMAGE, base_kb: JAB_BASE_KB, kb_scaling: JAB_KB_SCALING,
            kb_angle: JAB_KB_ANGLE,
            hitbox_w: JAB_HITBOX_W, hitbox_h: JAB_HITBOX_H,
            hitbox_offset_x: JAB_HITBOX_OFFSET_X, hitbox_offset_y: JAB_HITBOX_OFFSET_Y,
        },
        AttackType::ForwardTilt => AttackData {
            startup: 5, active: 4, recovery: 12,
            damage: Fp::from_int(8), base_kb: Fp::from_int(25), kb_scaling: Fp::from_int(60),
            kb_angle: 40,
            hitbox_w: Fp::from_int(45), hitbox_h: Fp::from_int(20),
            hitbox_offset_x: Fp::from_int(20), hitbox_offset_y: Fp::from_int(-30),
        },
        AttackType::UpTilt => AttackData {
            startup: 4, active: 4, recovery: 10,
            damage: Fp::from_int(7), base_kb: Fp::from_int(25), kb_scaling: Fp::from_int(55),
            kb_angle: 85,
            hitbox_w: Fp::from_int(35), hitbox_h: Fp::from_int(30),
            hitbox_offset_x: Fp::from_int(0), hitbox_offset_y: Fp::from_int(-65),
        },
        AttackType::DownTilt => AttackData {
            startup: 4, active: 3, recovery: 10,
            damage: Fp::from_int(6), base_kb: Fp::from_int(20), kb_scaling: Fp::from_int(50),
            kb_angle: 70,
            hitbox_w: Fp::from_int(40), hitbox_h: Fp::from_int(15),
            hitbox_offset_x: Fp::from_int(15), hitbox_offset_y: Fp::from_int(-5),
        },
        AttackType::ForwardSmash => AttackData {
            startup: 16, active: 3, recovery: 32,
            damage: Fp::from_int(22), base_kb: Fp::from_int(50), kb_scaling: Fp::from_int(100),
            kb_angle: 35,
            hitbox_w: Fp::from_int(50), hitbox_h: Fp::from_int(25),
            hitbox_offset_x: Fp::from_int(25), hitbox_offset_y: Fp::from_int(-30),
        },
        AttackType::UpSmash => AttackData {
            startup: 12, active: 4, recovery: 28,
            damage: Fp::from_int(18), base_kb: Fp::from_int(45), kb_scaling: Fp::from_int(95),
            kb_angle: 90,
            hitbox_w: Fp::from_int(40), hitbox_h: Fp::from_int(40),
            hitbox_offset_x: Fp::from_int(0), hitbox_offset_y: Fp::from_int(-70),
        },
        AttackType::DownSmash => AttackData {
            startup: 10, active: 4, recovery: 28,
            damage: Fp::from_int(20), base_kb: Fp::from_int(42), kb_scaling: Fp::from_int(90),
            kb_angle: 30,
            hitbox_w: Fp::from_int(55), hitbox_h: Fp::from_int(15),
            hitbox_offset_x: Fp::from_int(10), hitbox_offset_y: Fp::from_int(-5),
        },
        AttackType::DashAttack => AttackData {
            startup: 5, active: 4, recovery: 14,
            damage: Fp::from_int(10), base_kb: Fp::from_int(30), kb_scaling: Fp::from_int(65),
            kb_angle: 50,
            hitbox_w: Fp::from_int(50), hitbox_h: Fp::from_int(25),
            hitbox_offset_x: Fp::from_int(20), hitbox_offset_y: Fp::from_int(-30),
        },
        AttackType::NeutralAir => AttackData {
            startup: 4, active: 6, recovery: 10,
            damage: Fp::from_int(8), base_kb: Fp::from_int(20), kb_scaling: Fp::from_int(50),
            kb_angle: 50,
            hitbox_w: Fp::from_int(40), hitbox_h: Fp::from_int(40),
            hitbox_offset_x: Fp::from_int(0), hitbox_offset_y: Fp::from_int(-30),
        },
        AttackType::ForwardAir => AttackData {
            startup: 5, active: 4, recovery: 12,
            damage: Fp::from_int(10), base_kb: Fp::from_int(25), kb_scaling: Fp::from_int(65),
            kb_angle: 40,
            hitbox_w: Fp::from_int(45), hitbox_h: Fp::from_int(20),
            hitbox_offset_x: Fp::from_int(20), hitbox_offset_y: Fp::from_int(-30),
        },
        AttackType::BackAir => AttackData {
            startup: 6, active: 3, recovery: 14,
            damage: Fp::from_int(12), base_kb: Fp::from_int(30), kb_scaling: Fp::from_int(70),
            kb_angle: 35,
            hitbox_w: Fp::from_int(40), hitbox_h: Fp::from_int(20),
            hitbox_offset_x: Fp::from_int(20), hitbox_offset_y: Fp::from_int(-30),
        },
        AttackType::UpAir => AttackData {
            startup: 4, active: 5, recovery: 10,
            damage: Fp::from_int(9), base_kb: Fp::from_int(22), kb_scaling: Fp::from_int(55),
            kb_angle: 85,
            hitbox_w: Fp::from_int(35), hitbox_h: Fp::from_int(30),
            hitbox_offset_x: Fp::from_int(0), hitbox_offset_y: Fp::from_int(-65),
        },
        AttackType::DownAir => AttackData {
            startup: 10, active: 3, recovery: 18,
            damage: Fp::from_int(14), base_kb: Fp::from_int(40), kb_scaling: Fp::from_int(90),
            kb_angle: 270, // meteor smash — straight down
            hitbox_w: Fp::from_int(30), hitbox_h: Fp::from_int(30),
            hitbox_offset_x: Fp::from_int(0), hitbox_offset_y: Fp::from_int(0),
        },
        AttackType::SideSpecial => AttackData {
            startup: 6, active: 5, recovery: 16,
            damage: Fp::from_int(12), base_kb: Fp::from_int(35), kb_scaling: Fp::from_int(80),
            kb_angle: 45,
            hitbox_w: Fp::from_int(40), hitbox_h: Fp::from_int(25),
            hitbox_offset_x: Fp::from_int(35), hitbox_offset_y: Fp::from_int(0),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionState {
    Idle,
    AttackStartup { frames_left: u8 },
    AttackActive { frames_left: u8 },
    AttackRecovery { frames_left: u8 },
    Shielding,
    ShieldStun { frames_left: u8 },
    Grabbing { frames_left: u8 },
    Hitstun { frames_left: u8 },
    SpotDodge { frames_left: u8 },
    Rolling { frames_left: u8, direction: i8 }, // direction: 1 = right, -1 = left
    AirDodge { frames_left: u8 },
    LedgeHang,
    SpecialMove { frames_left: u8 },
    Counter { frames_left: u8 },
    Freefall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Projectile {
    pub active: bool,
    pub owner: u8,
    pub position_x: Fp,
    pub position_y: Fp,
    pub velocity_x: Fp,
    pub lifetime: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerState {
    pub position_x: Fp,
    pub position_y: Fp,
    pub velocity_x: Fp,
    pub velocity_y: Fp,
    pub damage_percent: u32,
    pub stocks: u8,
    pub invincibility_frames: u8,
    pub facing_right: bool,
    pub grounded: bool,
    pub jumps_remaining: u8,
    pub jump_held: bool,
    pub attack_held: bool,
    pub special_held: bool,
    pub grab_held: bool,
    pub action: ActionState,
    pub shield_hp: u16,
    pub hit_this_attack: bool, // prevent multi-hit per attack
    pub shield_held: bool,
    pub current_attack: AttackType,
    pub has_ledge_grab: bool, // true = already used ledge grab this airborne state
    pub has_up_special: bool, // true = already used up-B this airborne state
    pub is_running: bool,
    pub run_frames: u8, // how many consecutive frames direction held
    pub is_crouching: bool,
    pub drop_through_frames: u8,
    pub drop_through_active: bool, // temporarily ignore ground collision
    pub character: CharacterId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameState {
    pub players: [PlayerState; MAX_PLAYERS],
    pub projectiles: [Projectile; MAX_PROJECTILES],
    pub frame: u32,
    pub match_over: bool,
    pub winner: Option<u8>,
}

fn new_player(position_x: Fp, facing_right: bool) -> PlayerState {
    PlayerState {
        position_x,
        position_y: GROUND_Y,
        velocity_x: Fp::ZERO,
        velocity_y: Fp::ZERO,
        damage_percent: 0,
        stocks: STARTING_STOCKS,
        invincibility_frames: 0,
        facing_right,
        grounded: true,
        jumps_remaining: MAX_JUMPS,
        jump_held: false,
        attack_held: false,
        special_held: false,
        grab_held: false,
        action: ActionState::Idle,
        shield_hp: SHIELD_MAX_HP,
        hit_this_attack: false,
        shield_held: false,
        current_attack: AttackType::Jab,
        has_ledge_grab: false,
        has_up_special: false,
        is_running: false,
        is_crouching: false,
        drop_through_frames: 0,
        drop_through_active: false,
        run_frames: 0,
        character: CharacterId::Balanced,
    }
}

fn new_projectile() -> Projectile {
    Projectile {
        active: false,
        owner: 0,
        position_x: Fp::ZERO,
        position_y: Fp::ZERO,
        velocity_x: Fp::ZERO,
        lifetime: 0,
    }
}

pub fn default_state() -> GameState {
    GameState {
        players: [
            new_player(STAGE_CENTER_X - SPAWN_OFFSET_X, true),
            new_player(STAGE_CENTER_X + SPAWN_OFFSET_X, false),
        ],
        projectiles: [new_projectile(); MAX_PROJECTILES],
        frame: 0,
        match_over: false,
        winner: None,
    }
}

/// Compute a checksum of the full game state for desync detection.
pub fn state_checksum(state: &GameState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

fn respawn_player(player: &mut PlayerState) {
    player.position_x = STAGE_CENTER_X;
    player.position_y = RESPAWN_Y;
    player.velocity_x = Fp::ZERO;
    player.velocity_y = Fp::ZERO;
    player.damage_percent = 0;
    player.invincibility_frames = RESPAWN_INVINCIBILITY_FRAMES;
    player.grounded = false;
    player.jumps_remaining = MAX_JUMPS;
    player.action = ActionState::Idle;
    player.shield_hp = SHIELD_MAX_HP;
    player.has_up_special = false;
    player.has_ledge_grab = false;
}

fn in_blast_zone(player: &PlayerState) -> bool {
    player.position_x < BLAST_LEFT
        || player.position_x > BLAST_RIGHT
        || player.position_y < BLAST_TOP
        || player.position_y > BLAST_BOTTOM
}

pub fn player_hurtbox(player: &PlayerState) -> Rect {
    Rect {
        x: player.position_x - PLAYER_WIDTH / Fp::from_int(2),
        y: player.position_y - PLAYER_HEIGHT,
        w: PLAYER_WIDTH,
        h: PLAYER_HEIGHT,
    }
}

fn attack_hitbox(player: &PlayerState) -> Rect {
    let data = attack_data(player.current_attack);
    // Back air hitbox appears behind the player
    let facing = if player.current_attack == AttackType::BackAir {
        !player.facing_right
    } else {
        player.facing_right
    };
    let dir = if facing { Fp::ONE } else { -Fp::ONE };
    Rect {
        x: player.position_x + data.hitbox_offset_x * dir
            - if facing { Fp::ZERO } else { data.hitbox_w },
        y: player.position_y + data.hitbox_offset_y,
        w: data.hitbox_w,
        h: data.hitbox_h,
    }
}

fn resolve_attack_type(input: PlayerInput, grounded: bool, facing_right: bool) -> AttackType {
    let has_left = input.pressed(PlayerInput::LEFT);
    let has_right = input.pressed(PlayerInput::RIGHT);
    let has_dir_lr = has_left || has_right;
    let has_up = input.pressed(PlayerInput::UP);
    let has_down = input.pressed(PlayerInput::DOWN);
    let smash = input.pressed(PlayerInput::SMASH);

    if !grounded {
        if has_up { return AttackType::UpAir; }
        if has_down { return AttackType::DownAir; }
        // Forward = same direction as facing, back = opposite
        if has_right && facing_right || has_left && !facing_right {
            return AttackType::ForwardAir;
        }
        if has_left && facing_right || has_right && !facing_right {
            return AttackType::BackAir;
        }
        return AttackType::NeutralAir;
    }

    if smash {
        if has_up { return AttackType::UpSmash; }
        if has_down { return AttackType::DownSmash; }
        if has_dir_lr { return AttackType::ForwardSmash; }
    }

    if has_up { return AttackType::UpTilt; }
    if has_down { return AttackType::DownTilt; }
    if has_dir_lr { return AttackType::ForwardTilt; }

    AttackType::Jab
}

fn can_act(player: &PlayerState) -> bool {
    matches!(player.action, ActionState::Idle)
}

fn count_player_projectiles(projectiles: &[Projectile; MAX_PROJECTILES], owner: u8) -> usize {
    projectiles.iter().filter(|p| p.active && p.owner == owner).count()
}

pub fn advance_frame(state: &mut GameState, inputs: [PlayerInput; MAX_PLAYERS]) {
    if state.match_over {
        return;
    }

    // Phase 1: process each player's input and state transitions
    for i in 0..MAX_PLAYERS {
        let input = inputs[i];
        let player = &mut state.players[i];

        if player.stocks == 0 {
            continue;
        }

        let stats = character_stats(player.character);

        // Decrement invincibility
        if player.invincibility_frames > 0 {
            player.invincibility_frames -= 1;
        }

        // Advance action state
        match player.action {
            ActionState::AttackStartup { frames_left } => {
                if frames_left <= 1 {
                    let data = attack_data(player.current_attack);
                    player.action = ActionState::AttackActive { frames_left: data.active };
                    player.hit_this_attack = false;
                } else {
                    player.action = ActionState::AttackStartup { frames_left: frames_left - 1 };
                }
            }
            ActionState::AttackActive { frames_left } => {
                if frames_left <= 1 {
                    let data = attack_data(player.current_attack);
                    player.action = ActionState::AttackRecovery { frames_left: data.recovery };
                } else {
                    player.action = ActionState::AttackActive { frames_left: frames_left - 1 };
                }
            }
            ActionState::AttackRecovery { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::AttackRecovery { frames_left: frames_left - 1 };
                }
            }
            ActionState::Hitstun { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::Hitstun { frames_left: frames_left - 1 };
                }
            }
            ActionState::ShieldStun { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::ShieldStun { frames_left: frames_left - 1 };
                }
            }
            ActionState::Grabbing { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::Grabbing { frames_left: frames_left - 1 };
                }
            }
            ActionState::Shielding => {
                if !input.pressed(PlayerInput::SHIELD) {
                    player.action = ActionState::Idle;
                }
            }
            ActionState::SpotDodge { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::SpotDodge { frames_left: frames_left - 1 };
                }
            }
            ActionState::Rolling { frames_left, direction } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    // Move during roll
                    let roll_speed = ROLL_DISTANCE / Fp::from_int(ROLL_FRAMES as i32);
                    player.position_x = player.position_x + roll_speed * Fp::from_int(direction as i32);
                    player.action = ActionState::Rolling { frames_left: frames_left - 1, direction };
                }
            }
            ActionState::AirDodge { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::AirDodge { frames_left: frames_left - 1 };
                }
            }
            ActionState::SpecialMove { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Freefall;
                    player.velocity_y = Fp::ZERO;
                } else {
                    player.action = ActionState::SpecialMove { frames_left: frames_left - 1 };
                }
            }
            ActionState::Freefall => {
                // Can only air drift, no attacks or specials
                // Freefall ends on landing (handled in ground collision)
            }
            ActionState::LedgeHang => {
                // Ledge options
                if input.pressed(PlayerInput::UP) {
                    // Climb up — put on stage
                    player.action = ActionState::Idle;
                    if player.position_x <= STAGE_LEFT {
                        player.position_x = STAGE_LEFT + Fp::from_int(30);
                    } else {
                        player.position_x = STAGE_RIGHT - Fp::from_int(30);
                    }
                    player.position_y = GROUND_Y;
                    player.grounded = true;
                    player.jumps_remaining = MAX_JUMPS;
                } else if input.pressed(PlayerInput::DOWN) {
                    // Drop — push slightly off-stage so ground collision doesn't catch
                    player.action = ActionState::Idle;
                    player.velocity_y = Fp::from_int(2);
                    if player.position_x <= STAGE_LEFT {
                        player.position_x = STAGE_LEFT - Fp::from_int(5);
                    } else {
                        player.position_x = STAGE_RIGHT + Fp::from_int(5);
                    }
                } else if input.pressed(PlayerInput::ATTACK) {
                    // Getup attack — climb onto stage and attack with i-frames
                    if player.position_x <= STAGE_LEFT {
                        player.position_x = STAGE_LEFT + Fp::from_int(30);
                        player.facing_right = true;
                    } else {
                        player.position_x = STAGE_RIGHT - Fp::from_int(30);
                        player.facing_right = false;
                    }
                    player.position_y = GROUND_Y;
                    player.grounded = true;
                    player.jumps_remaining = MAX_JUMPS;
                    let data = attack_data(AttackType::ForwardTilt); // getup attack uses ftilt data
                    player.current_attack = AttackType::ForwardTilt;
                    player.action = ActionState::AttackStartup { frames_left: data.startup };
                    player.hit_this_attack = false;
                    player.invincibility_frames = 10;
                } else {
                    // Stay hanging — no gravity, no movement
                    player.velocity_x = Fp::ZERO;
                    player.velocity_y = Fp::ZERO;
                }
            }
            ActionState::Counter { frames_left } => {
                if frames_left <= 1 {
                    player.action = ActionState::Idle;
                } else {
                    player.action = ActionState::Counter { frames_left: frames_left - 1 };
                }
            }
            ActionState::Idle => {}
        }

        // Input actions (only when idle)
        if can_act(player) {
            let shield_pressed = input.pressed(PlayerInput::SHIELD);
            let fresh_shield = shield_pressed && !player.shield_held;

            if fresh_shield {
                if !player.grounded {
                    // Air dodge
                    player.action = ActionState::AirDodge { frames_left: AIR_DODGE_FRAMES };
                    player.invincibility_frames = AIR_DODGE_INVINCIBLE;
                    // Directional air dodge momentum
                    if input.pressed(PlayerInput::RIGHT) {
                        player.velocity_x = AIR_DODGE_SPEED;
                    } else if input.pressed(PlayerInput::LEFT) {
                        player.velocity_x = -AIR_DODGE_SPEED;
                    }
                    if input.pressed(PlayerInput::UP) {
                        player.velocity_y = -AIR_DODGE_SPEED;
                    } else if input.pressed(PlayerInput::DOWN) {
                        player.velocity_y = AIR_DODGE_SPEED;
                    }
                } else if input.pressed(PlayerInput::DOWN) {
                    // Spot dodge
                    player.action = ActionState::SpotDodge { frames_left: SPOT_DODGE_FRAMES };
                    player.invincibility_frames = SPOT_DODGE_INVINCIBLE;
                } else if input.pressed(PlayerInput::LEFT) || input.pressed(PlayerInput::RIGHT) {
                    // Roll
                    let dir: i8 = if input.pressed(PlayerInput::RIGHT) { 1 } else { -1 };
                    player.action = ActionState::Rolling { frames_left: ROLL_FRAMES, direction: dir };
                    player.invincibility_frames = ROLL_INVINCIBLE;
                } else {
                    // Plain shield
                    player.action = ActionState::Shielding;
                }
            } else if shield_pressed {
                // Holding shield (not fresh press) — maintain shield
                player.action = ActionState::Shielding;
            }
            // Up-B recovery (fresh special + up, airborne, not used yet)
            else if input.pressed(PlayerInput::SPECIAL) && !player.special_held
                && input.pressed(PlayerInput::UP) && !player.grounded && !player.has_up_special
            {
                player.action = ActionState::SpecialMove { frames_left: UP_SPECIAL_FRAMES };
                player.velocity_y = UP_SPECIAL_VELOCITY;
                player.velocity_x = Fp::ZERO;
                player.has_up_special = true;
            }
            // Down-B counter (fresh special + down)
            else if input.pressed(PlayerInput::SPECIAL) && !player.special_held
                && input.pressed(PlayerInput::DOWN)
            {
                player.action = ActionState::Counter { frames_left: COUNTER_FRAMES };
            }
            // Attack (on fresh press)
            else if input.pressed(PlayerInput::ATTACK) && !player.attack_held {
                let atk_type = if player.is_running {
                    AttackType::DashAttack
                } else {
                    resolve_attack_type(input, player.grounded, player.facing_right)
                };
                let data = attack_data(atk_type);
                player.current_attack = atk_type;
                player.action = ActionState::AttackStartup { frames_left: data.startup };
                player.hit_this_attack = false;
                player.is_running = false;
                player.run_frames = 0;
            }
            // Grab (on fresh press)
            else if input.pressed(PlayerInput::GRAB) && !player.grab_held {
                player.action = ActionState::Grabbing { frames_left: 10 };
            }
        }
        player.attack_held = input.pressed(PlayerInput::ATTACK);
        player.grab_held = input.pressed(PlayerInput::GRAB);
        player.shield_held = input.pressed(PlayerInput::SHIELD);

        // Movement (only when actionable or in certain states)
        let can_move = matches!(player.action, ActionState::Idle | ActionState::Shielding);

        // Crouch: hold down while grounded and actionable
        player.is_crouching = can_move && player.grounded && input.pressed(PlayerInput::DOWN)
            && !matches!(player.action, ActionState::Shielding);
        if !player.is_crouching {
            player.drop_through_frames = 0;
        }

        if can_move {
            if player.grounded {
                if player.is_crouching {
                    player.velocity_x = Fp::ZERO;
                    player.is_running = false;
                    player.run_frames = 0;
                    player.drop_through_frames += 1;
                    if player.drop_through_frames > CROUCH_DROP_FRAMES {
                        // Drop through platform
                        player.grounded = false;
                        player.is_crouching = false;
                        player.drop_through_frames = 0;
                        player.drop_through_active = true;
                        player.position_y = player.position_y + Fp::ONE;
                    }
                } else if !matches!(player.action, ActionState::Shielding) {
                    let holding_left = input.pressed(PlayerInput::LEFT);
                    let holding_right = input.pressed(PlayerInput::RIGHT);
                    let holding_same_dir = (holding_left && !player.facing_right)
                        || (holding_right && player.facing_right);

                    if holding_left || holding_right {
                        if holding_same_dir {
                            player.run_frames = player.run_frames.saturating_add(1);
                            if player.run_frames >= RUN_START_FRAMES {
                                player.is_running = true;
                            }
                        } else {
                            // Changed direction — reset run
                            player.run_frames = 1;
                            player.is_running = false;
                        }

                        let speed = if player.is_running { stats.run_speed } else { stats.walk_speed };
                        if holding_left {
                            player.velocity_x = -speed;
                            player.facing_right = false;
                        } else {
                            player.velocity_x = speed;
                            player.facing_right = true;
                        }
                    } else {
                        player.velocity_x = Fp::ZERO;
                        player.is_running = false;
                        player.run_frames = 0;
                    }
                } else {
                    player.velocity_x = Fp::ZERO;
                }
            } else {
                // Air drift
                if input.pressed(PlayerInput::LEFT) {
                    player.velocity_x = -stats.air_speed;
                    player.facing_right = false;
                } else if input.pressed(PlayerInput::RIGHT) {
                    player.velocity_x = stats.air_speed;
                    player.facing_right = true;
                }
            }

            // Jumping (only on fresh press, not held)
            let jump_pressed = input.pressed(PlayerInput::UP);
            if jump_pressed && !player.jump_held {
                // Wall jump check: airborne, touching stage wall, below ground level
                let at_left_wall = !player.grounded
                    && player.position_x <= STAGE_LEFT + Fp::from_int(5)
                    && player.position_y < GROUND_Y
                    && player.position_y > GROUND_Y - Fp::from_int(200);
                let at_right_wall = !player.grounded
                    && player.position_x >= STAGE_RIGHT - Fp::from_int(5)
                    && player.position_y < GROUND_Y
                    && player.position_y > GROUND_Y - Fp::from_int(200);

                if at_left_wall {
                    player.velocity_y = stats.jump_velocity;
                    player.velocity_x = Fp::from_int(6); // push away from wall
                    player.facing_right = true;
                } else if at_right_wall {
                    player.velocity_y = stats.jump_velocity;
                    player.velocity_x = Fp::from_int(-6);
                    player.facing_right = false;
                } else if player.jumps_remaining > 0 {
                    if player.grounded {
                        player.velocity_y = stats.jump_velocity;
                        player.grounded = false;
                    } else {
                        player.velocity_y = stats.double_jump_velocity;
                    }
                    player.jumps_remaining -= 1;
                }
                if matches!(player.action, ActionState::Shielding) {
                    player.action = ActionState::Idle;
                }
            }
            player.jump_held = jump_pressed;
        } else {
            // Still track jump_held to prevent buffered jumps
            player.jump_held = input.pressed(PlayerInput::UP);
        }

        // Fast-fall
        if !player.grounded
            && input.pressed(PlayerInput::DOWN)
            && player.velocity_y > Fp::ZERO
            && !matches!(player.action, ActionState::Hitstun { .. })
        {
            player.velocity_y = stats.fast_fall_speed;
        }

        // Shield regen (when not shielding)
        if !matches!(player.action, ActionState::Shielding) && player.shield_hp < SHIELD_MAX_HP {
            player.shield_hp += SHIELD_REGEN_RATE;
        }

        // Skip physics when hanging on ledge
        if matches!(player.action, ActionState::LedgeHang) {
            continue;
        }

        // Gravity (skip during up-B — the move provides its own velocity)
        if !player.grounded && !matches!(player.action, ActionState::SpecialMove { .. }) {
            player.velocity_y = player.velocity_y + stats.gravity;
        }

        // Apply velocity
        player.position_x = player.position_x + player.velocity_x;
        player.position_y = player.position_y + player.velocity_y;

        // Ledge grab detection
        if !player.grounded
            && !player.has_ledge_grab
            && player.velocity_y >= Fp::ZERO
            && can_act(player)
        {
            let near_ground = player.position_y >= GROUND_Y - LEDGE_GRAB_RANGE_Y
                && player.position_y <= GROUND_Y + LEDGE_GRAB_RANGE_Y;

            let near_right_ledge = player.position_x >= STAGE_RIGHT - LEDGE_GRAB_RANGE_X
                && player.position_x <= STAGE_RIGHT + LEDGE_GRAB_RANGE_X;
            let near_left_ledge = player.position_x >= STAGE_LEFT - LEDGE_GRAB_RANGE_X
                && player.position_x <= STAGE_LEFT + LEDGE_GRAB_RANGE_X;

            if near_ground && (near_right_ledge || near_left_ledge) {
                player.action = ActionState::LedgeHang;
                player.velocity_x = Fp::ZERO;
                player.velocity_y = Fp::ZERO;
                player.invincibility_frames = LEDGE_INVINCIBILITY;
                player.has_ledge_grab = true;
                if near_right_ledge {
                    player.position_x = STAGE_RIGHT;
                } else {
                    player.position_x = STAGE_LEFT;
                }
                player.position_y = GROUND_Y;
                continue;
            }
        }

        // Blast zone
        if in_blast_zone(player) {
            player.stocks -= 1;
            if player.stocks > 0 {
                respawn_player(player);
            }
        } else {
            // Clear drop-through when below stage level
            if player.drop_through_active && player.position_y > GROUND_Y + Fp::from_int(20) {
                player.drop_through_active = false;
            }
            // Ground collision (skip during drop-through)
            if player.position_y >= GROUND_Y
                && player.position_x >= STAGE_LEFT
                && player.position_x <= STAGE_RIGHT
                && !player.drop_through_active
            {
                let just_landed = !player.grounded;
                let was_in_hitstun = just_landed && matches!(player.action, ActionState::Hitstun { .. });
                let was_aerial_attack = just_landed
                    && matches!(player.action,
                        ActionState::AttackStartup { .. } | ActionState::AttackActive { .. } | ActionState::AttackRecovery { .. })
                    && matches!(player.current_attack,
                        AttackType::NeutralAir | AttackType::ForwardAir | AttackType::BackAir | AttackType::UpAir | AttackType::DownAir);

                // Meteor bounce: if spiked downward hard enough, bounce off the ground
                // (but teching overrides the bounce)
                let can_tech = was_in_hitstun && input.pressed(PlayerInput::SHIELD);
                if was_in_hitstun && player.velocity_y > METEOR_BOUNCE_THRESHOLD && !can_tech {
                    player.position_y = GROUND_Y;
                    // Bounce: reverse and decay vertical velocity
                    player.velocity_y = -(player.velocity_y * Fp::from_raw(Fp::ONE.raw() * 60 / 100));
                    player.velocity_x = player.velocity_x * Fp::from_raw(Fp::ONE.raw() * 80 / 100);
                    player.grounded = false;
                    continue;
                }

                player.position_y = GROUND_Y;
                player.velocity_y = Fp::ZERO;
                player.grounded = true;
                player.jumps_remaining = MAX_JUMPS;
                player.has_ledge_grab = false;
                player.has_up_special = false;
                if was_aerial_attack {
                    // Landing lag — shorter than full recovery
                    player.action = ActionState::AttackRecovery { frames_left: LANDING_LAG_FRAMES };
                    player.velocity_x = Fp::ZERO;
                } else if can_tech {
                    // Tech: recover on landing
                    let holding_left = input.pressed(PlayerInput::LEFT);
                    let holding_right = input.pressed(PlayerInput::RIGHT);
                    if holding_left || holding_right {
                        let dir: i8 = if holding_right { 1 } else { -1 };
                        player.action = ActionState::Rolling { frames_left: ROLL_FRAMES, direction: dir };
                        player.invincibility_frames = ROLL_INVINCIBLE;
                    } else {
                        player.action = ActionState::Idle;
                    }
                    player.velocity_x = Fp::ZERO;
                } else if matches!(player.action, ActionState::Freefall) {
                    player.action = ActionState::Idle;
                }
            }
            // Stage edges (prevent walking off platform)
            if player.grounded {
                if player.position_x < STAGE_LEFT {
                    player.position_x = STAGE_LEFT;
                } else if player.position_x > STAGE_RIGHT {
                    player.position_x = STAGE_RIGHT;
                }
            }
        }
    }

    // Phase 2: Spawn projectiles (special button)
    for i in 0..MAX_PLAYERS {
        let input = inputs[i];
        let special_pressed = input.pressed(PlayerInput::SPECIAL);
        let fresh_press = special_pressed && !state.players[i].special_held;
        state.players[i].special_held = special_pressed;

        let holding_side = input.pressed(PlayerInput::LEFT) || input.pressed(PlayerInput::RIGHT);
        let holding_up = input.pressed(PlayerInput::UP);
        let stats = character_stats(state.players[i].character);

        if fresh_press && can_act(&state.players[i]) && state.players[i].stocks > 0 {
            if holding_side && !holding_up {
                // Side-B: lunging attack
                let data = attack_data(AttackType::SideSpecial);
                state.players[i].current_attack = AttackType::SideSpecial;
                state.players[i].action = ActionState::AttackStartup { frames_left: data.startup };
                state.players[i].hit_this_attack = false;
                // Set facing direction
                if input.pressed(PlayerInput::LEFT) {
                    state.players[i].facing_right = false;
                } else {
                    state.players[i].facing_right = true;
                }
                // Lunge velocity
                let dir = if state.players[i].facing_right { Fp::ONE } else { -Fp::ONE };
                state.players[i].velocity_x = Fp::from_int(6) * dir;
            } else if !holding_up
                && count_player_projectiles(&state.projectiles, i as u8) < stats.max_projectiles
            {
                // Neutral-B: spawn projectile
                if let Some(slot) = state.projectiles.iter_mut().find(|p| !p.active) {
                    let player = &state.players[i];
                    let dir = if player.facing_right { Fp::ONE } else { -Fp::ONE };
                    slot.active = true;
                    slot.owner = i as u8;
                    slot.position_x = player.position_x + Fp::from_int(30) * dir;
                    slot.position_y = player.position_y - Fp::from_int(30);
                    slot.velocity_x = stats.projectile_speed * dir;
                    slot.lifetime = stats.projectile_lifetime;
                }
            }
        }
    }

    // Phase 3: Update projectiles
    for proj in state.projectiles.iter_mut() {
        if !proj.active {
            continue;
        }
        proj.position_x = proj.position_x + proj.velocity_x;
        if proj.lifetime <= 1 {
            proj.active = false;
        } else {
            proj.lifetime -= 1;
        }
        // Despawn if off screen
        if proj.position_x < BLAST_LEFT || proj.position_x > BLAST_RIGHT {
            proj.active = false;
        }
    }

    // Phase 4: Hit detection (attacks and projectiles)
    // Copy positions for read-only access during hit checks
    let players_snapshot = state.players;

    for attacker_idx in 0..MAX_PLAYERS {
        let defender_idx = 1 - attacker_idx;
        let attacker = &players_snapshot[attacker_idx];
        let defender = &players_snapshot[defender_idx];

        if attacker.stocks == 0 || defender.stocks == 0 {
            continue;
        }
        if defender.invincibility_frames > 0 {
            continue;
        }

        let defender_hurtbox = player_hurtbox(defender);

        // Attack hit check
        if matches!(attacker.action, ActionState::AttackActive { .. }) && !attacker.hit_this_attack {
            let hitbox = attack_hitbox(attacker);
            if hitbox.overlaps(&defender_hurtbox) {
                let data = attack_data(attacker.current_attack);
                let hit = HitData {
                    base_knockback: data.base_kb,
                    knockback_scaling: data.kb_scaling,
                    knockback_angle: data.kb_angle,
                    damage: data.damage,
                };

                if matches!(defender.action, ActionState::Counter { .. }) {
                    // Counter! Reflect damage back to attacker with 1.3x multiplier
                    let counter_hit = HitData {
                        base_knockback: hit.base_knockback * Fp::from_raw(Fp::ONE.raw() * 13 / 10),
                        knockback_scaling: hit.knockback_scaling,
                        knockback_angle: hit.knockback_angle,
                        damage: hit.damage * Fp::from_raw(Fp::ONE.raw() * 13 / 10),
                    };
                    apply_hit(
                        &mut state.players[attacker_idx],
                        &counter_hit,
                        inputs[attacker_idx],
                        defender.facing_right,
                    );
                    state.players[defender_idx].action = ActionState::Idle;
                } else if matches!(defender.action, ActionState::Shielding) {
                    // Hit shield
                    let d = &mut state.players[defender_idx];
                    if d.shield_hp > SHIELD_DEPLETE_PER_HIT {
                        d.shield_hp -= SHIELD_DEPLETE_PER_HIT;
                    } else {
                        d.shield_hp = 0;
                        d.action = ActionState::ShieldStun { frames_left: SHIELD_BREAK_STUN };
                    }
                } else {
                    // Normal hit
                    apply_hit(
                        &mut state.players[defender_idx],
                        &hit,
                        inputs[defender_idx],
                        attacker.facing_right,
                    );
                }
                state.players[attacker_idx].hit_this_attack = true;
            }
        }

        // Grab hit check
        if matches!(attacker.action, ActionState::Grabbing { .. }) {
            let grab_box = attack_hitbox(attacker); // reuse attack hitbox for grab range
            if grab_box.overlaps(&defender_hurtbox)
                && (matches!(defender.action, ActionState::Shielding | ActionState::Idle))
            {
                // Determine throw direction from attacker's held input
                let atk_input = inputs[attacker_idx];
                let (kb_angle, base_kb, kb_scaling, dmg, throw_facing) =
                    if atk_input.pressed(PlayerInput::UP) {
                        (90, 30, 70, 7, attacker.facing_right) // up throw
                    } else if atk_input.pressed(PlayerInput::DOWN) {
                        (30, 20, 50, 5, attacker.facing_right) // down throw — low angle, combo starter
                    } else if (atk_input.pressed(PlayerInput::LEFT) && attacker.facing_right)
                        || (atk_input.pressed(PlayerInput::RIGHT) && !attacker.facing_right)
                    {
                        (135, 30, 65, 8, attacker.facing_right) // back throw — behind attacker
                    } else {
                        (45, 28, 60, 6, attacker.facing_right) // forward throw (default)
                    };
                let throw_hit = HitData {
                    base_knockback: Fp::from_int(base_kb),
                    knockback_scaling: Fp::from_int(kb_scaling),
                    knockback_angle: kb_angle,
                    damage: Fp::from_int(dmg),
                };
                state.players[defender_idx].action = ActionState::Idle;
                apply_hit(
                    &mut state.players[defender_idx],
                    &throw_hit,
                    inputs[defender_idx],
                    throw_facing,
                );
                state.players[attacker_idx].action = ActionState::Idle;
            }
        }
    }

    // Phase 5: Projectile hit detection
    for proj_idx in 0..MAX_PROJECTILES {
        if !state.projectiles[proj_idx].active {
            continue;
        }
        let proj = state.projectiles[proj_idx];

        for defender_idx in 0..MAX_PLAYERS {
            if defender_idx as u8 == proj.owner {
                continue;
            }
            if state.players[defender_idx].stocks == 0 {
                continue;
            }
            if state.players[defender_idx].invincibility_frames > 0 {
                continue;
            }

            let proj_rect = Rect {
                x: proj.position_x - PROJECTILE_SIZE / Fp::from_int(2),
                y: proj.position_y - PROJECTILE_SIZE / Fp::from_int(2),
                w: PROJECTILE_SIZE,
                h: PROJECTILE_SIZE,
            };
            let defender_hurtbox = player_hurtbox(&state.players[defender_idx]);

            if proj_rect.overlaps(&defender_hurtbox) {
                let hit = HitData {
                    base_knockback: PROJECTILE_BASE_KB,
                    knockback_scaling: PROJECTILE_KB_SCALING,
                    knockback_angle: 45,
                    damage: PROJECTILE_DAMAGE,
                };

                if matches!(state.players[defender_idx].action, ActionState::Shielding) {
                    let d = &mut state.players[defender_idx];
                    if d.shield_hp > SHIELD_DEPLETE_PER_HIT {
                        d.shield_hp -= SHIELD_DEPLETE_PER_HIT;
                    } else {
                        d.shield_hp = 0;
                        d.action = ActionState::ShieldStun { frames_left: SHIELD_BREAK_STUN };
                    }
                } else {
                    let facing = proj.velocity_x > Fp::ZERO;
                    apply_hit(
                        &mut state.players[defender_idx],
                        &hit,
                        inputs[defender_idx],
                        facing,
                    );
                }
                state.projectiles[proj_idx].active = false;
                break;
            }
        }
    }

    // Phase 6: Check for match end
    for i in 0..MAX_PLAYERS {
        if state.players[i].stocks == 0 {
            state.match_over = true;
            for j in 0..MAX_PLAYERS {
                if state.players[j].stocks > 0 {
                    state.winner = Some(j as u8);
                }
            }
        }
    }

    state.frame += 1;
}

fn apply_hit(defender: &mut PlayerState, hit: &HitData, defender_input: PlayerInput, attacker_facing_right: bool) {
    defender.damage_percent += hit.damage.to_int() as u32;

    let weight = character_stats(defender.character).weight;
    let kb_magnitude = calculate_knockback(hit, defender.damage_percent, weight);

    // Apply DI
    let base_angle = if attacker_facing_right {
        hit.knockback_angle
    } else {
        180 - hit.knockback_angle
    };
    let angle = apply_di(base_angle, defender_input.di_x(), defender_input.di_y());

    // Convert angle + magnitude to velocity
    // cos_deg/sin_deg return value * 100, so divide by 100 to normalize
    // Scale down by KB_VELOCITY_SCALE so knockback is a nudge at low %, a launch at high %
    let vx = kb_magnitude * Fp::from_int(cos_deg(angle)) / Fp::from_int(100) / KB_VELOCITY_SCALE;
    let vy = kb_magnitude * Fp::from_int(-sin_deg(angle)) / Fp::from_int(100) / KB_VELOCITY_SCALE;

    defender.velocity_x = vx;
    defender.velocity_y = vy;
    defender.grounded = false;

    // Hitstun proportional to knockback (scaled down too)
    let hitstun = (kb_magnitude.to_int() * 2 / 10).min(60).max(1) as u8;
    defender.action = ActionState::Hitstun { frames_left: hitstun };
}
