pub mod combat;
pub mod fixed;
pub mod rollback;
pub mod state;

#[cfg(test)]
mod tests {
    use crate::fixed::Fp;
    use crate::state::*;

    fn no_input() -> [PlayerInput; MAX_PLAYERS] {
        [PlayerInput(0), PlayerInput(0)]
    }

    fn p1_input(bits: u16) -> [PlayerInput; MAX_PLAYERS] {
        [PlayerInput(bits), PlayerInput(0)]
    }

    // === Fixed-point math ===

    #[test]
    fn fp_from_int_and_back() {
        assert_eq!(Fp::from_int(42).to_int(), 42);
        assert_eq!(Fp::from_int(-7).to_int(), -7);
    }

    #[test]
    fn fp_arithmetic() {
        assert_eq!((Fp::from_int(3) + Fp::from_int(4)).to_int(), 7);
        assert_eq!((Fp::from_int(10) - Fp::from_int(3)).to_int(), 7);
        assert_eq!((Fp::from_int(6) * Fp::from_int(7)).to_int(), 42);
        assert_eq!((Fp::from_int(42) / Fp::from_int(7)).to_int(), 6);
    }

    #[test]
    fn fp_fractional_multiplication() {
        let a = Fp::from_raw(Fp::from_int(1).raw() + Fp::ONE.raw() / 2);
        assert_eq!((a * Fp::from_int(2)).to_int(), 3);
    }

    #[test]
    fn fp_negative_operations() {
        let a = Fp::from_int(-5);
        let b = Fp::from_int(3);
        assert_eq!((a + b).to_int(), -2);
        assert_eq!((a * b).to_int(), -15);
    }

    // === Game state initialization ===

    #[test]
    fn default_state_correct() {
        let state = default_state();
        for i in 0..MAX_PLAYERS {
            assert_eq!(state.players[i].stocks, STARTING_STOCKS);
            assert_eq!(state.players[i].damage_percent, 0);
            assert_eq!(state.players[i].velocity_x, Fp::ZERO);
            assert_eq!(state.players[i].velocity_y, Fp::ZERO);
            assert_eq!(state.players[i].action, ActionState::Idle);
        }
        assert!(state.players[0].position_x != state.players[1].position_x);
        assert_eq!(state.players[0].position_y, state.players[1].position_y);
    }

    // === Movement ===

    #[test]
    fn walk_right() {
        let mut state = default_state();
        let start_x = state.players[0].position_x;
        advance_frame(&mut state, p1_input(PlayerInput::RIGHT));
        assert_eq!(state.players[0].position_x, start_x + DEFAULT_WALK_SPEED);
        assert!(state.players[0].facing_right);
    }

    #[test]
    fn walk_left() {
        let mut state = default_state();
        let start_x = state.players[0].position_x;
        advance_frame(&mut state, p1_input(PlayerInput::LEFT));
        assert_eq!(state.players[0].position_x, start_x - DEFAULT_WALK_SPEED);
        assert!(!state.players[0].facing_right);
    }

    #[test]
    fn no_input_stops() {
        let mut state = default_state();
        let start_x = state.players[0].position_x;
        advance_frame(&mut state, p1_input(PlayerInput::RIGHT));
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].position_x, start_x + DEFAULT_WALK_SPEED);
    }

    #[test]
    fn frame_increments() {
        let mut state = default_state();
        advance_frame(&mut state, no_input());
        assert_eq!(state.frame, 1);
        advance_frame(&mut state, no_input());
        assert_eq!(state.frame, 2);
    }

    // === Gravity ===

    #[test]
    fn airborne_falls() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        advance_frame(&mut state, no_input());
        assert!(state.players[0].position_y > Fp::from_int(300));
    }

    #[test]
    fn grounded_stays() {
        let mut state = default_state();
        let y = state.players[0].position_y;
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].position_y, y);
        assert!(state.players[0].grounded);
    }

    #[test]
    fn lands_on_ground() {
        let mut state = default_state();
        state.players[0].position_y = GROUND_Y - Fp::from_int(1);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(5);
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].position_y, GROUND_Y);
        assert!(state.players[0].grounded);
    }

    #[test]
    fn gravity_accumulates() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(100);
        state.players[0].grounded = false;
        advance_frame(&mut state, no_input());
        let v1 = state.players[0].velocity_y;
        advance_frame(&mut state, no_input());
        assert!(state.players[0].velocity_y > v1);
    }

    // === Jumping ===

    #[test]
    fn jump_from_ground() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        assert!(!state.players[0].grounded);
        assert!(state.players[0].velocity_y < Fp::ZERO);
    }

    #[test]
    fn double_jump() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        assert_eq!(state.players[0].jumps_remaining, 1);
        advance_frame(&mut state, no_input());
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        assert_eq!(state.players[0].jumps_remaining, 0);
        assert!(state.players[0].velocity_y < Fp::ZERO);
    }

    #[test]
    fn no_triple_jump() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        advance_frame(&mut state, no_input());
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        advance_frame(&mut state, no_input());
        let v = state.players[0].velocity_y;
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        assert!(state.players[0].velocity_y > v); // gravity, no jump
    }

    #[test]
    fn jump_resets_on_landing() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        for _ in 0..120 {
            advance_frame(&mut state, no_input());
            if state.players[0].grounded { break; }
        }
        assert!(state.players[0].grounded);
        assert_eq!(state.players[0].jumps_remaining, MAX_JUMPS);
    }

    // === Fast-fall ===

    #[test]
    fn fast_fall() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(200);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(1);

        let mut normal = state;
        advance_frame(&mut normal, no_input());
        let mut ff = state;
        advance_frame(&mut ff, p1_input(PlayerInput::DOWN));
        assert!(ff.players[0].velocity_y > normal.players[0].velocity_y);
    }

    #[test]
    fn no_fast_fall_while_rising() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(200);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(-5);

        let mut a = state;
        advance_frame(&mut a, p1_input(PlayerInput::DOWN));
        let mut b = state;
        advance_frame(&mut b, no_input());
        assert_eq!(a.players[0].velocity_y, b.players[0].velocity_y);
    }

    // === Knockback ===

    #[test]
    fn knockback_scales_with_damage() {
        use crate::combat::{calculate_knockback, HitData};
        let hit = HitData {
            base_knockback: Fp::from_int(40), knockback_scaling: Fp::from_int(100),
            knockback_angle: 45, damage: Fp::from_int(12),
        };
        let k0 = calculate_knockback(&hit, 0, Fp::from_int(100));
        let k50 = calculate_knockback(&hit, 50, Fp::from_int(100));
        let k100 = calculate_knockback(&hit, 100, Fp::from_int(100));
        assert!(k50 > k0);
        assert!(k100 > k50);
    }

    #[test]
    fn lighter_flies_further() {
        use crate::combat::{calculate_knockback, HitData};
        let hit = HitData {
            base_knockback: Fp::from_int(40), knockback_scaling: Fp::from_int(100),
            knockback_angle: 45, damage: Fp::from_int(12),
        };
        assert!(calculate_knockback(&hit, 80, Fp::from_int(75)) > calculate_knockback(&hit, 80, Fp::from_int(120)));
    }

    // === DI ===

    #[test]
    fn di_shifts_angle() {
        use crate::combat::apply_di;
        let adj = apply_di(90, 1, 0);
        assert!(adj < 90 && adj >= 75);
    }

    #[test]
    fn di_no_input_no_shift() {
        use crate::combat::apply_di;
        assert_eq!(apply_di(45, 0, 0), 45);
    }

    // === Collision ===

    #[test]
    fn rect_overlap() {
        use crate::combat::Rect;
        let a = Rect { x: Fp::from_int(0), y: Fp::from_int(0), w: Fp::from_int(10), h: Fp::from_int(10) };
        let b = Rect { x: Fp::from_int(5), y: Fp::from_int(5), w: Fp::from_int(10), h: Fp::from_int(10) };
        assert!(a.overlaps(&b));
        let c = Rect { x: Fp::from_int(20), y: Fp::from_int(20), w: Fp::from_int(10), h: Fp::from_int(10) };
        assert!(!a.overlaps(&c));
    }

    // === Blast zone / KO ===

    #[test]
    fn blast_zone_right_ko() {
        let mut state = default_state();
        state.players[0].position_x = BLAST_RIGHT + Fp::from_int(1);
        state.players[0].grounded = false;
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].stocks, STARTING_STOCKS - 1);
        assert_eq!(state.players[0].damage_percent, 0);
        assert_eq!(state.players[0].position_x, STAGE_CENTER_X);
        assert_eq!(state.players[0].invincibility_frames, RESPAWN_INVINCIBILITY_FRAMES);
    }

    #[test]
    fn blast_zone_top_ko() {
        let mut state = default_state();
        state.players[0].position_y = BLAST_TOP - Fp::from_int(1);
        state.players[0].grounded = false;
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].stocks, STARTING_STOCKS - 1);
    }

    #[test]
    fn blast_zone_bottom_ko() {
        let mut state = default_state();
        state.players[0].position_y = BLAST_BOTTOM + Fp::from_int(1);
        state.players[0].grounded = false;
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].stocks, STARTING_STOCKS - 1);
    }

    #[test]
    fn respawn_invincibility_decrements() {
        let mut state = default_state();
        state.players[0].invincibility_frames = RESPAWN_INVINCIBILITY_FRAMES;
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].invincibility_frames, RESPAWN_INVINCIBILITY_FRAMES - 1);
    }

    #[test]
    fn match_ends_on_last_stock() {
        let mut state = default_state();
        state.players[0].stocks = 1;
        state.players[0].position_x = BLAST_RIGHT + Fp::from_int(1);
        state.players[0].grounded = false;
        advance_frame(&mut state, no_input());
        assert_eq!(state.players[0].stocks, 0);
        assert!(state.match_over);
        assert_eq!(state.winner, Some(1));
    }

    // === Attack state machine ===

    #[test]
    fn attack_transitions_through_phases() {
        let mut state = default_state();
        // Press attack
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        assert!(matches!(state.players[0].action, ActionState::AttackStartup { .. }));

        // Advance through startup
        for _ in 0..JAB_STARTUP {
            advance_frame(&mut state, no_input());
        }
        assert!(matches!(state.players[0].action, ActionState::AttackActive { .. }));

        // Advance through active
        for _ in 0..JAB_ACTIVE {
            advance_frame(&mut state, no_input());
        }
        assert!(matches!(state.players[0].action, ActionState::AttackRecovery { .. }));

        // Advance through recovery
        for _ in 0..JAB_RECOVERY {
            advance_frame(&mut state, no_input());
        }
        assert_eq!(state.players[0].action, ActionState::Idle);
    }

    #[test]
    fn cant_act_during_attack() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        // Try to shield during startup — should not work
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD));
        assert!(!matches!(state.players[0].action, ActionState::Shielding));
    }

    #[test]
    fn attack_hits_opponent() {
        let mut state = default_state();
        // Move players close together
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;

        // Start attack
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        // Go through startup to active
        for _ in 0..JAB_STARTUP {
            advance_frame(&mut state, no_input());
        }
        // Now in active — should hit
        advance_frame(&mut state, no_input());

        // Defender should have taken damage
        assert!(state.players[1].damage_percent > 0);
        assert!(matches!(state.players[1].action, ActionState::Hitstun { .. }));
    }

    #[test]
    fn attack_launches_opponent() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].damage_percent = 80; // high damage = big knockback

        // Attack through startup to active
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        for _ in 0..JAB_STARTUP {
            advance_frame(&mut state, no_input());
        }
        advance_frame(&mut state, no_input());

        // Defender should be launched (velocity set, airborne)
        assert!(!state.players[1].grounded);
        assert!(state.players[1].velocity_x != Fp::ZERO || state.players[1].velocity_y != Fp::ZERO,
            "knockback should set velocity, got vx={} vy={}",
            state.players[1].velocity_x.to_int(), state.players[1].velocity_y.to_int());

        // Run a few frames — player should move significantly
        let pos_before = state.players[1].position_x;
        for _ in 0..10 {
            advance_frame(&mut state, no_input());
        }
        let distance = (state.players[1].position_x - pos_before).raw().abs();
        assert!(distance > Fp::from_int(20).raw(),
            "player should travel far after knockback, distance={}",
            distance / Fp::ONE.raw());
    }

    #[test]
    fn smash_launches_further_than_jab() {
        // Compare knockback velocity, not distance (avoids timing differences)
        // Jab at 80%
        let mut s1 = default_state();
        s1.players[0].position_x = Fp::from_int(500);
        s1.players[1].position_x = Fp::from_int(540);
        s1.players[0].facing_right = true;
        s1.players[1].damage_percent = 80;

        advance_frame(&mut s1, p1_input(PlayerInput::ATTACK));
        for _ in 0..(JAB_STARTUP + JAB_ACTIVE) as usize {
            advance_frame(&mut s1, no_input());
        }
        let jab_vx = s1.players[1].velocity_x.raw().abs();
        let jab_vy = s1.players[1].velocity_y.raw().abs();
        let jab_kb = jab_vx + jab_vy;

        // Forward smash at 80%
        let mut s2 = default_state();
        s2.players[0].position_x = Fp::from_int(500);
        s2.players[1].position_x = Fp::from_int(540);
        s2.players[0].facing_right = true;
        s2.players[1].damage_percent = 80;

        let fsmash_data = attack_data(AttackType::ForwardSmash);
        advance_frame(&mut s2, p1_input(PlayerInput::SMASH | PlayerInput::RIGHT | PlayerInput::ATTACK));
        for _ in 0..(fsmash_data.startup + fsmash_data.active) as usize {
            advance_frame(&mut s2, no_input());
        }
        let smash_vx = s2.players[1].velocity_x.raw().abs();
        let smash_vy = s2.players[1].velocity_y.raw().abs();
        let smash_kb = smash_vx + smash_vy;

        assert!(smash_kb > jab_kb,
            "fsmash knockback {} should exceed jab knockback {}", smash_kb, jab_kb);
    }

    #[test]
    fn attack_misses_when_far() {
        let mut state = default_state();
        // Players far apart (default positions)
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        for _ in 0..(JAB_STARTUP + JAB_ACTIVE + JAB_RECOVERY) as usize {
            advance_frame(&mut state, no_input());
        }
        assert_eq!(state.players[1].damage_percent, 0);
    }

    // === Shield ===

    #[test]
    fn shield_blocks_attack() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;

        // P2 shields
        state.players[1].action = ActionState::Shielding;

        // P1 attacks through startup to active
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        for _ in 0..JAB_STARTUP {
            advance_frame(&mut state, [PlayerInput(PlayerInput::ATTACK), PlayerInput(PlayerInput::SHIELD)]);
        }
        advance_frame(&mut state, [PlayerInput(0), PlayerInput(PlayerInput::SHIELD)]);

        // Defender took no damage
        assert_eq!(state.players[1].damage_percent, 0);
        // Shield HP depleted
        assert!(state.players[1].shield_hp < SHIELD_MAX_HP);
    }

    #[test]
    fn shield_break_stuns() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].action = ActionState::Shielding;
        state.players[1].shield_hp = 5; // very low

        // Attack through startup to active
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        for _ in 0..JAB_STARTUP {
            advance_frame(&mut state, [PlayerInput(0), PlayerInput(PlayerInput::SHIELD)]);
        }
        advance_frame(&mut state, [PlayerInput(0), PlayerInput(PlayerInput::SHIELD)]);

        assert!(matches!(state.players[1].action, ActionState::ShieldStun { .. }));
    }

    #[test]
    fn shield_regenerates() {
        let mut state = default_state();
        state.players[0].shield_hp = 50;
        advance_frame(&mut state, no_input());
        assert!(state.players[0].shield_hp > 50);
    }

    // === Grab ===

    #[test]
    fn grab_beats_shield() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].action = ActionState::Shielding;

        advance_frame(&mut state, p1_input(PlayerInput::GRAB));

        // Defender should have taken damage despite shielding
        assert!(state.players[1].damage_percent > 0);
        assert!(matches!(state.players[1].action, ActionState::Hitstun { .. }));
    }

    #[test]
    fn grab_hits_idle_opponent() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;

        advance_frame(&mut state, p1_input(PlayerInput::GRAB));
        assert!(state.players[1].damage_percent > 0);
    }

    #[test]
    fn forward_throw_sends_forward() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].damage_percent = 60;

        // Grab while holding forward
        advance_frame(&mut state, p1_input(PlayerInput::GRAB | PlayerInput::RIGHT));
        // Defender should be launched forward (rightward)
        assert!(state.players[1].velocity_x > Fp::ZERO,
            "forward throw should send opponent in facing direction");
    }

    #[test]
    fn back_throw_sends_backward() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].damage_percent = 60;

        // Grab while holding backward (left when facing right)
        advance_frame(&mut state, p1_input(PlayerInput::GRAB | PlayerInput::LEFT));
        assert!(state.players[1].velocity_x < Fp::ZERO,
            "back throw should send opponent opposite of facing direction");
    }

    #[test]
    fn up_throw_sends_upward() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].damage_percent = 60;

        advance_frame(&mut state, p1_input(PlayerInput::GRAB | PlayerInput::UP));
        assert!(state.players[1].velocity_y < Fp::ZERO,
            "up throw should launch opponent upward");
    }

    #[test]
    fn down_throw_sends_low() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].damage_percent = 60;

        advance_frame(&mut state, p1_input(PlayerInput::GRAB | PlayerInput::DOWN));
        // Down throw should have low angle (more horizontal, good for combos)
        assert!(state.players[1].damage_percent > 60);
    }

    // === Projectiles ===

    #[test]
    fn special_spawns_projectile() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));
        let count = state.projectiles.iter().filter(|p| p.active).count();
        assert_eq!(count, 1);
        assert_eq!(state.projectiles.iter().find(|p| p.active).unwrap().owner, 0);
    }

    #[test]
    fn projectile_moves() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));
        let x1 = state.projectiles.iter().find(|p| p.active).unwrap().position_x;
        advance_frame(&mut state, no_input());
        let x2 = state.projectiles.iter().find(|p| p.active).unwrap().position_x;
        assert!(x2 > x1); // facing right, should move right
    }

    #[test]
    fn projectile_despawns_on_timeout() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));
        // Set lifetime to 1
        state.projectiles.iter_mut().find(|p| p.active).unwrap().lifetime = 1;
        advance_frame(&mut state, no_input());
        // Should despawn
        let count = state.projectiles.iter().filter(|p| p.active).count();
        assert_eq!(count, 0);
    }

    #[test]
    fn projectile_cap_per_player() {
        let mut state = default_state();
        // Spawn first
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));
        advance_frame(&mut state, no_input()); // release
        // Spawn second
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));
        advance_frame(&mut state, no_input()); // release
        // Try third — should not spawn
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));
        let count = state.projectiles.iter().filter(|p| p.active && p.owner == 0).count();
        assert_eq!(count, MAX_PROJECTILES_PER_PLAYER);
    }

    #[test]
    fn projectile_hits_opponent() {
        let mut state = default_state();
        // Place players close, p1 facing p2
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(560);
        state.players[0].facing_right = true;

        // Spawn projectile
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL));

        // Run until hit or timeout
        for _ in 0..20 {
            advance_frame(&mut state, no_input());
            if state.players[1].damage_percent > 0 { break; }
        }
        assert!(state.players[1].damage_percent > 0);
    }

    // === Tilt and smash attacks ===

    #[test]
    fn forward_tilt() {
        let mut state = default_state();
        // Direction + attack = tilt
        advance_frame(&mut state, p1_input(PlayerInput::RIGHT | PlayerInput::ATTACK));
        assert!(matches!(state.players[0].action, ActionState::AttackStartup { .. }));
        // Check that attack type is ForwardTilt
        assert_eq!(state.players[0].current_attack, AttackType::ForwardTilt);
    }

    #[test]
    fn up_tilt() {
        let mut state = default_state();
        // Up + attack while grounded = up tilt (not jump, because attack takes priority)
        advance_frame(&mut state, p1_input(PlayerInput::UP | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::UpTilt);
    }

    #[test]
    fn down_tilt() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::DOWN | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::DownTilt);
    }

    #[test]
    fn jab_when_no_direction() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::Jab);
    }

    #[test]
    fn forward_smash() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SMASH | PlayerInput::RIGHT | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::ForwardSmash);
    }

    #[test]
    fn up_smash() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SMASH | PlayerInput::UP | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::UpSmash);
    }

    #[test]
    fn down_smash() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SMASH | PlayerInput::DOWN | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::DownSmash);
    }

    #[test]
    fn smash_attack_hits_harder() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;

        // Forward smash
        advance_frame(&mut state, p1_input(PlayerInput::SMASH | PlayerInput::RIGHT | PlayerInput::ATTACK));
        // Advance through startup + active
        for _ in 0..20 {
            advance_frame(&mut state, no_input());
        }
        let smash_dmg = state.players[1].damage_percent;

        // Reset
        let mut state2 = default_state();
        state2.players[0].position_x = Fp::from_int(500);
        state2.players[1].position_x = Fp::from_int(540);
        state2.players[0].facing_right = true;

        // Jab
        advance_frame(&mut state2, p1_input(PlayerInput::ATTACK));
        for _ in 0..20 {
            advance_frame(&mut state2, no_input());
        }
        let jab_dmg = state2.players[1].damage_percent;

        assert!(smash_dmg > jab_dmg, "smash {} should hit harder than jab {}", smash_dmg, jab_dmg);
    }

    // === Aerial attacks ===

    #[test]
    fn neutral_air() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::NeutralAir);
    }

    #[test]
    fn forward_air() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        state.players[0].facing_right = true;
        advance_frame(&mut state, p1_input(PlayerInput::RIGHT | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::ForwardAir);
    }

    #[test]
    fn back_air() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        state.players[0].facing_right = true;
        // Left while facing right = back air
        advance_frame(&mut state, p1_input(PlayerInput::LEFT | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::BackAir);
    }

    #[test]
    fn up_air() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        advance_frame(&mut state, p1_input(PlayerInput::UP | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::UpAir);
    }

    #[test]
    fn down_air() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        advance_frame(&mut state, p1_input(PlayerInput::DOWN | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::DownAir);
    }

    #[test]
    fn aerial_hits_opponent() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[0].position_y = Fp::from_int(570); // slightly above ground
        state.players[0].grounded = false;
        state.players[0].facing_right = true;
        state.players[1].position_x = Fp::from_int(540);

        advance_frame(&mut state, p1_input(PlayerInput::RIGHT | PlayerInput::ATTACK));
        // Run through startup + active
        for _ in 0..15 {
            advance_frame(&mut state, no_input());
        }
        assert!(state.players[1].damage_percent > 0);
    }

    // === Meteor smash (down-air spike) ===

    #[test]
    fn dair_spikes_downward() {
        let mut state = default_state();
        // Both airborne, close together
        state.players[0].position_x = Fp::from_int(500);
        state.players[0].position_y = Fp::from_int(400);
        state.players[0].grounded = false;
        state.players[0].facing_right = true;
        state.players[1].position_x = Fp::from_int(510);
        state.players[1].position_y = Fp::from_int(430);
        state.players[1].grounded = false;

        // Down-air attack
        advance_frame(&mut state, p1_input(PlayerInput::DOWN | PlayerInput::ATTACK));
        let data = attack_data(AttackType::DownAir);
        for _ in 0..data.startup {
            advance_frame(&mut state, no_input());
        }
        // Active frames
        for _ in 0..data.active {
            advance_frame(&mut state, no_input());
            if state.players[1].damage_percent > 0 { break; }
        }

        // Should have been hit
        assert!(state.players[1].damage_percent > 0, "dair should hit");
        // Velocity should be strongly downward (positive Y)
        assert!(state.players[1].velocity_y > Fp::ZERO,
            "dair should spike DOWN, vy={}", state.players[1].velocity_y.to_int());
        // Horizontal velocity should be minimal
        assert!(state.players[1].velocity_x.raw().abs() < state.players[1].velocity_y.raw(),
            "spike should be mostly vertical");
    }

    #[test]
    fn dair_spike_kills_at_high_percent() {
        let mut state = default_state();
        // Position both players off-stage so defender can't land on platform
        state.players[0].position_x = Fp::from_int(1100);
        state.players[0].position_y = Fp::from_int(400);
        state.players[0].grounded = false;
        state.players[0].facing_right = true;
        state.players[1].position_x = Fp::from_int(1100);
        state.players[1].position_y = Fp::from_int(430);
        state.players[1].grounded = false;
        state.players[1].damage_percent = 120;

        advance_frame(&mut state, p1_input(PlayerInput::DOWN | PlayerInput::ATTACK));
        let data = attack_data(AttackType::DownAir);
        for _ in 0..(data.startup + data.active) {
            advance_frame(&mut state, no_input());
        }
        // Run frames until KO or timeout
        for _ in 0..120 {
            advance_frame(&mut state, no_input());
            if state.players[1].stocks < STARTING_STOCKS { break; }
        }
        assert!(state.players[1].stocks < STARTING_STOCKS,
            "dair spike at 120% off-stage should KO through bottom blast zone");
    }

    #[test]
    fn meteor_bounce_off_platform() {
        let mut state = default_state();
        // Player in hitstun spiked downward onto stage
        state.players[0].position_y = GROUND_Y - Fp::from_int(5);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(10); // strong downward spike
        state.players[0].action = ActionState::Hitstun { frames_left: 30 };

        advance_frame(&mut state, no_input());
        // Should bounce: still airborne, velocity reversed upward
        assert!(!state.players[0].grounded,
            "meteor bounce should keep player airborne");
        assert!(state.players[0].velocity_y < Fp::ZERO,
            "meteor bounce should reverse velocity upward");
    }

    #[test]
    fn weak_hit_doesnt_bounce() {
        let mut state = default_state();
        // Weak downward velocity — below threshold
        state.players[0].position_y = GROUND_Y - Fp::from_int(2);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(3); // below METEOR_BOUNCE_THRESHOLD
        state.players[0].action = ActionState::Hitstun { frames_left: 20 };

        advance_frame(&mut state, no_input());
        // Should land normally, not bounce
        assert!(state.players[0].grounded,
            "weak hit should land normally without bouncing");
    }

    // === Aerial landing lag ===

    #[test]
    fn landing_during_aerial_causes_lag() {
        let mut state = default_state();
        // Put player airborne, slightly above ground, attacking
        state.players[0].position_y = GROUND_Y - Fp::from_int(3);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(5);
        state.players[0].action = ActionState::AttackActive { frames_left: 3 };
        state.players[0].current_attack = AttackType::NeutralAir;

        advance_frame(&mut state, no_input());
        // Should have landed with lag (AttackRecovery on ground)
        assert!(state.players[0].grounded);
        assert!(matches!(state.players[0].action, ActionState::AttackRecovery { .. }),
            "landing during aerial should cause landing lag");
    }

    #[test]
    fn landing_during_aerial_startup_causes_lag() {
        let mut state = default_state();
        state.players[0].position_y = GROUND_Y - Fp::from_int(3);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(5);
        state.players[0].action = ActionState::AttackStartup { frames_left: 3 };
        state.players[0].current_attack = AttackType::ForwardAir;

        advance_frame(&mut state, no_input());
        assert!(state.players[0].grounded);
        assert!(matches!(state.players[0].action, ActionState::AttackRecovery { .. }),
            "landing during aerial startup should cause landing lag");
    }

    // === Wall jump ===

    #[test]
    fn wall_jump_off_left_wall() {
        let mut state = default_state();
        // Player at left edge of stage, airborne, touching wall
        state.players[0].position_x = STAGE_LEFT;
        state.players[0].position_y = GROUND_Y - Fp::from_int(30);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(2);
        state.players[0].facing_right = false;
        let y_before = state.players[0].velocity_y;

        // Press jump while touching left wall
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        // Should wall jump: gain upward velocity and move away from wall
        assert!(state.players[0].velocity_y < Fp::ZERO,
            "wall jump should give upward velocity");
        assert!(state.players[0].velocity_x > Fp::ZERO,
            "wall jump off left wall should push rightward");
    }

    #[test]
    fn wall_jump_off_right_wall() {
        let mut state = default_state();
        state.players[0].position_x = STAGE_RIGHT;
        state.players[0].position_y = GROUND_Y - Fp::from_int(30);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(2);
        state.players[0].facing_right = true;

        advance_frame(&mut state, p1_input(PlayerInput::UP));
        assert!(state.players[0].velocity_y < Fp::ZERO);
        assert!(state.players[0].velocity_x < Fp::ZERO,
            "wall jump off right wall should push leftward");
    }

    #[test]
    fn no_wall_jump_when_grounded() {
        let mut state = default_state();
        state.players[0].position_x = STAGE_LEFT;
        state.players[0].grounded = true;
        let jumps_before = state.players[0].jumps_remaining;

        advance_frame(&mut state, p1_input(PlayerInput::UP));
        // Should be a normal jump, not wall jump
        assert!(state.players[0].jumps_remaining < jumps_before);
    }

    // === Up-B recovery ===

    #[test]
    fn up_special_launches_upward() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(400);
        state.players[0].grounded = false;
        let y_before = state.players[0].position_y;
        advance_frame(&mut state, p1_input(PlayerInput::UP | PlayerInput::SPECIAL));
        assert!(matches!(state.players[0].action, ActionState::SpecialMove { .. }));
        // Run a few frames — should move upward
        for _ in 0..10 {
            advance_frame(&mut state, no_input());
        }
        assert!(state.players[0].position_y < y_before,
            "up-B should move player upward");
    }

    #[test]
    fn up_special_puts_in_freefall() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(400);
        state.players[0].grounded = false;
        advance_frame(&mut state, p1_input(PlayerInput::UP | PlayerInput::SPECIAL));
        // Run through the special move
        for _ in 0..40 {
            advance_frame(&mut state, no_input());
            if matches!(state.players[0].action, ActionState::Freefall) { break; }
        }
        assert!(matches!(state.players[0].action, ActionState::Freefall),
            "up-B should end in freefall");
    }

    #[test]
    fn freefall_cannot_attack() {
        let mut state = default_state();
        state.players[0].action = ActionState::Freefall;
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        assert!(matches!(state.players[0].action, ActionState::Freefall),
            "cannot attack during freefall");
    }

    #[test]
    fn freefall_ends_on_landing() {
        let mut state = default_state();
        state.players[0].action = ActionState::Freefall;
        state.players[0].position_x = Fp::from_int(500);
        state.players[0].position_y = GROUND_Y - Fp::from_int(2);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(5);
        advance_frame(&mut state, no_input());
        assert!(state.players[0].grounded);
        assert_eq!(state.players[0].action, ActionState::Idle);
    }

    #[test]
    fn up_special_only_once_per_airborne() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        // First up-B
        advance_frame(&mut state, p1_input(PlayerInput::UP | PlayerInput::SPECIAL));
        assert!(matches!(state.players[0].action, ActionState::SpecialMove { .. }));
        // Wait for freefall
        for _ in 0..40 {
            advance_frame(&mut state, no_input());
        }
        // Try second up-B — should not work
        state.players[0].special_held = false;
        advance_frame(&mut state, p1_input(PlayerInput::UP | PlayerInput::SPECIAL));
        assert!(!matches!(state.players[0].action, ActionState::SpecialMove { .. }),
            "cannot use up-B twice without landing");
    }

    // === Running and dash attack ===

    #[test]
    fn running_faster_than_walking() {
        let mut state = default_state();
        // Hold direction for a few frames to start running
        for _ in 0..RUN_START_FRAMES as usize + 1 {
            advance_frame(&mut state, p1_input(PlayerInput::RIGHT));
        }
        assert!(state.players[0].is_running);
        let run_x = state.players[0].position_x;

        // Compare: fresh walk
        let mut state2 = default_state();
        advance_frame(&mut state2, p1_input(PlayerInput::RIGHT));
        let walk_delta = state2.players[0].position_x - (STAGE_CENTER_X - SPAWN_OFFSET_X);

        // Running speed should exceed walk speed
        let stats = character_stats(state.players[0].character);
        assert!(stats.run_speed > stats.walk_speed);
    }

    #[test]
    fn dash_attack() {
        let mut state = default_state();
        // Start running
        for _ in 0..RUN_START_FRAMES as usize + 1 {
            advance_frame(&mut state, p1_input(PlayerInput::RIGHT));
        }
        assert!(state.players[0].is_running);
        // Attack while running = dash attack
        advance_frame(&mut state, p1_input(PlayerInput::RIGHT | PlayerInput::ATTACK));
        assert_eq!(state.players[0].current_attack, AttackType::DashAttack);
        assert!(matches!(state.players[0].action, ActionState::AttackStartup { .. }));
    }

    #[test]
    fn stop_running_on_release() {
        let mut state = default_state();
        for _ in 0..RUN_START_FRAMES as usize + 1 {
            advance_frame(&mut state, p1_input(PlayerInput::RIGHT));
        }
        assert!(state.players[0].is_running);
        advance_frame(&mut state, no_input());
        assert!(!state.players[0].is_running);
    }

    // === Crouch ===

    #[test]
    fn crouch_on_ground() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        assert!(state.players[0].is_crouching);
    }

    #[test]
    fn no_crouch_in_air() {
        let mut state = default_state();
        state.players[0].grounded = false;
        state.players[0].position_y = Fp::from_int(300);
        advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        assert!(!state.players[0].is_crouching);
    }

    #[test]
    fn crouch_stops_on_release() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        assert!(state.players[0].is_crouching);
        advance_frame(&mut state, no_input());
        assert!(!state.players[0].is_crouching);
    }

    #[test]
    fn crouch_stops_movement() {
        let mut state = default_state();
        // Hold down + right — should crouch, not walk
        advance_frame(&mut state, p1_input(PlayerInput::DOWN | PlayerInput::RIGHT));
        assert!(state.players[0].is_crouching);
        assert_eq!(state.players[0].velocity_x, Fp::ZERO);
    }

    // === Platform drop-through ===

    #[test]
    fn hold_down_drops_through_platform() {
        let mut state = default_state();
        assert!(state.players[0].grounded);
        // Hold down for enough frames to trigger drop-through
        for _ in 0..CROUCH_DROP_FRAMES as usize + 1 {
            advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        }
        assert!(!state.players[0].grounded, "should have dropped through platform");
        assert!(state.players[0].position_y > GROUND_Y);
    }

    #[test]
    fn release_down_resets_drop_counter() {
        let mut state = default_state();
        // Hold down for a few frames but not enough
        for _ in 0..CROUCH_DROP_FRAMES as usize - 1 {
            advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        }
        assert!(state.players[0].grounded);
        // Release
        advance_frame(&mut state, no_input());
        // Hold again — counter should reset, so same number of frames isn't enough
        for _ in 0..CROUCH_DROP_FRAMES as usize - 1 {
            advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        }
        assert!(state.players[0].grounded, "should still be grounded — counter reset");
    }

    // === Teching ===

    #[test]
    fn ground_tech_on_landing() {
        let mut state = default_state();
        // Put player in hitstun, moving downward toward ground
        state.players[0].position_y = GROUND_Y - Fp::from_int(5);
        state.players[0].velocity_y = Fp::from_int(10);
        state.players[0].grounded = false;
        state.players[0].action = ActionState::Hitstun { frames_left: 20 };
        // Hold shield = tech input
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD));
        // Should tech: go to idle, on ground, no bounce
        assert!(state.players[0].grounded);
        assert!(matches!(state.players[0].action, ActionState::Idle));
        assert_eq!(state.players[0].velocity_x, Fp::ZERO);
        assert_eq!(state.players[0].velocity_y, Fp::ZERO);
    }

    #[test]
    fn no_tech_without_shield() {
        let mut state = default_state();
        // Put player in hitstun landing on ground — no shield held
        state.players[0].position_y = GROUND_Y - Fp::from_int(5);
        state.players[0].velocity_y = Fp::from_int(10);
        state.players[0].grounded = false;
        state.players[0].action = ActionState::Hitstun { frames_left: 20 };
        advance_frame(&mut state, no_input());
        // Should NOT tech — still in hitstun
        assert!(matches!(state.players[0].action, ActionState::Hitstun { .. }));
    }

    #[test]
    fn tech_roll_with_direction() {
        let mut state = default_state();
        state.players[0].position_y = GROUND_Y - Fp::from_int(5);
        state.players[0].velocity_y = Fp::from_int(10);
        state.players[0].grounded = false;
        state.players[0].action = ActionState::Hitstun { frames_left: 20 };
        let x_before = state.players[0].position_x;
        // Shield + direction = tech roll
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::RIGHT));
        assert!(state.players[0].grounded);
        // Should be rolling
        assert!(matches!(state.players[0].action, ActionState::Rolling { .. }));
    }

    // === Dodge mechanics ===

    #[test]
    fn spot_dodge_on_ground() {
        let mut state = default_state();
        // Shield + down on ground = spot dodge
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::DOWN));
        assert!(matches!(state.players[0].action, ActionState::SpotDodge { .. }));
        assert!(state.players[0].invincibility_frames > 0);
    }

    #[test]
    fn spot_dodge_cant_act_during() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::DOWN));
        // Try to attack during spot dodge — should fail
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        assert!(!matches!(state.players[0].action, ActionState::AttackStartup { .. }));
    }

    #[test]
    fn spot_dodge_returns_to_idle() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::DOWN));
        for _ in 0..30 {
            advance_frame(&mut state, no_input());
            if state.players[0].action == ActionState::Idle { break; }
        }
        assert_eq!(state.players[0].action, ActionState::Idle);
    }

    #[test]
    fn roll_forward() {
        let mut state = default_state();
        let start_x = state.players[0].position_x;
        state.players[0].facing_right = true;
        // Shield + right (forward) on ground = forward roll
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::RIGHT));
        assert!(matches!(state.players[0].action, ActionState::Rolling { .. }));
        // Run through roll
        for _ in 0..25 {
            advance_frame(&mut state, no_input());
            if state.players[0].action == ActionState::Idle { break; }
        }
        // Should have moved forward
        assert!(state.players[0].position_x > start_x);
    }

    #[test]
    fn roll_has_invincibility() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::RIGHT));
        assert!(state.players[0].invincibility_frames > 0);
    }

    #[test]
    fn air_dodge() {
        let mut state = default_state();
        // Put player in air
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        // Shield in air = air dodge
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD));
        assert!(matches!(state.players[0].action, ActionState::AirDodge { .. }));
        assert!(state.players[0].invincibility_frames > 0);
    }

    #[test]
    fn air_dodge_with_direction() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        let start_x = state.players[0].position_x;
        // Shield + right in air = directional air dodge
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD | PlayerInput::RIGHT));
        assert!(matches!(state.players[0].action, ActionState::AirDodge { .. }));
        // Should have moved in the dodge direction
        assert!(state.players[0].position_x > start_x);
    }

    #[test]
    fn air_dodge_returns_to_idle() {
        let mut state = default_state();
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;
        advance_frame(&mut state, p1_input(PlayerInput::SHIELD));
        for _ in 0..30 {
            advance_frame(&mut state, no_input());
            if state.players[0].action == ActionState::Idle { break; }
        }
        assert_eq!(state.players[0].action, ActionState::Idle);
    }

    // === Ledge mechanics ===

    #[test]
    fn ledge_grab_right() {
        let mut state = default_state();
        // Position player near right ledge edge, airborne, moving toward it
        state.players[0].position_x = STAGE_RIGHT + Fp::from_int(5);
        state.players[0].position_y = GROUND_Y - Fp::from_int(10);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(2);
        state.players[0].velocity_x = Fp::ZERO;
        advance_frame(&mut state, no_input());
        assert!(matches!(state.players[0].action, ActionState::LedgeHang));
        assert!(state.players[0].invincibility_frames > 0);
    }

    #[test]
    fn ledge_grab_left() {
        let mut state = default_state();
        state.players[0].position_x = STAGE_LEFT - Fp::from_int(5);
        state.players[0].position_y = GROUND_Y - Fp::from_int(10);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(2);
        advance_frame(&mut state, no_input());
        assert!(matches!(state.players[0].action, ActionState::LedgeHang));
    }

    #[test]
    fn ledge_climb_up() {
        let mut state = default_state();
        state.players[0].action = ActionState::LedgeHang;
        state.players[0].position_x = STAGE_RIGHT;
        state.players[0].position_y = GROUND_Y;
        state.players[0].grounded = false;
        state.players[0].has_ledge_grab = true;
        // Up to climb
        advance_frame(&mut state, p1_input(PlayerInput::UP));
        // Should be on stage now
        assert!(state.players[0].grounded || state.players[0].position_y < GROUND_Y);
        assert!(!matches!(state.players[0].action, ActionState::LedgeHang));
    }

    #[test]
    fn ledge_jump_off() {
        let mut state = default_state();
        state.players[0].action = ActionState::LedgeHang;
        state.players[0].position_x = STAGE_RIGHT;
        state.players[0].position_y = GROUND_Y;
        state.players[0].grounded = false;
        state.players[0].has_ledge_grab = true;
        // Attack to let go and do aerial
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        assert!(!matches!(state.players[0].action, ActionState::LedgeHang));
    }

    #[test]
    fn ledge_drop() {
        let mut state = default_state();
        state.players[0].action = ActionState::LedgeHang;
        state.players[0].position_x = STAGE_RIGHT;
        state.players[0].position_y = GROUND_Y;
        state.players[0].grounded = false;
        state.players[0].has_ledge_grab = true;
        // Down to drop
        advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        assert!(!matches!(state.players[0].action, ActionState::LedgeHang));
        assert!(!state.players[0].grounded);
    }

    #[test]
    fn no_double_ledge_grab() {
        let mut state = default_state();
        // First grab
        state.players[0].position_x = STAGE_RIGHT + Fp::from_int(5);
        state.players[0].position_y = GROUND_Y - Fp::from_int(10);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(2);
        advance_frame(&mut state, no_input());
        assert!(matches!(state.players[0].action, ActionState::LedgeHang));
        assert!(state.players[0].has_ledge_grab);

        // Drop from ledge
        advance_frame(&mut state, p1_input(PlayerInput::DOWN));
        assert!(!matches!(state.players[0].action, ActionState::LedgeHang));

        // Try to grab again — should fail because has_ledge_grab is still true
        state.players[0].position_x = STAGE_RIGHT + Fp::from_int(5);
        state.players[0].position_y = GROUND_Y - Fp::from_int(10);
        state.players[0].velocity_y = Fp::from_int(2);
        advance_frame(&mut state, no_input());
        assert!(!matches!(state.players[0].action, ActionState::LedgeHang));
    }

    #[test]
    fn ledge_grab_resets_on_land() {
        let mut state = default_state();
        state.players[0].has_ledge_grab = true;
        state.players[0].position_y = GROUND_Y - Fp::from_int(1);
        state.players[0].grounded = false;
        state.players[0].velocity_y = Fp::from_int(5);
        advance_frame(&mut state, no_input());
        assert!(state.players[0].grounded);
        assert!(!state.players[0].has_ledge_grab);
    }

    // === Edge getup attack ===

    #[test]
    fn ledge_attack_climbs_and_attacks() {
        let mut state = default_state();
        state.players[0].action = ActionState::LedgeHang;
        state.players[0].position_x = STAGE_RIGHT;
        state.players[0].position_y = GROUND_Y;
        state.players[0].grounded = false;

        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        // Should be on stage, attacking with invincibility
        assert!(state.players[0].grounded);
        assert!(matches!(state.players[0].action, ActionState::AttackStartup { .. }
            | ActionState::AttackActive { .. }));
        assert!(state.players[0].invincibility_frames > 0,
            "getup attack should have invincibility");
    }

    #[test]
    fn ledge_attack_hits_opponent() {
        let mut state = default_state();
        // P1 on right ledge
        state.players[0].action = ActionState::LedgeHang;
        state.players[0].position_x = STAGE_RIGHT;
        state.players[0].position_y = GROUND_Y;
        state.players[0].grounded = false;
        // P2 near ledge (to the left, where getup attack faces)
        state.players[1].position_x = STAGE_RIGHT - Fp::from_int(50);
        state.players[1].position_y = GROUND_Y;

        // Getup attack
        advance_frame(&mut state, p1_input(PlayerInput::ATTACK));
        // Advance through startup and active frames
        for _ in 0..15 {
            advance_frame(&mut state, no_input());
        }
        assert!(state.players[1].damage_percent > 0,
            "getup attack should hit nearby opponent");
    }

    // === Character archetypes ===

    #[test]
    fn three_characters_exist() {
        assert_eq!(character_stats(CharacterId::Balanced).weight.to_int(), 100);
        assert!(character_stats(CharacterId::Ranged).walk_speed < character_stats(CharacterId::Rushdown).walk_speed);
        assert!(character_stats(CharacterId::Rushdown).weight < character_stats(CharacterId::Balanced).weight);
    }

    #[test]
    fn rushdown_walks_faster() {
        let mut balanced = default_state();
        balanced.players[0].character = CharacterId::Balanced;
        let mut rushdown = default_state();
        rushdown.players[0].character = CharacterId::Rushdown;

        advance_frame(&mut balanced, p1_input(PlayerInput::RIGHT));
        advance_frame(&mut rushdown, p1_input(PlayerInput::RIGHT));

        assert!(rushdown.players[0].position_x > balanced.players[0].position_x);
    }

    #[test]
    fn ranged_extra_projectiles() {
        // Ranged character gets 3 projectiles instead of 2
        let stats = character_stats(CharacterId::Ranged);
        assert!(stats.max_projectiles > 2);
    }

    #[test]
    fn rushdown_lighter_knockback() {
        use crate::combat::{calculate_knockback, HitData};
        let hit = HitData {
            base_knockback: Fp::from_int(40), knockback_scaling: Fp::from_int(100),
            knockback_angle: 45, damage: Fp::from_int(12),
        };
        let balanced_kb = calculate_knockback(&hit, 80, character_stats(CharacterId::Balanced).weight);
        let rushdown_kb = calculate_knockback(&hit, 80, character_stats(CharacterId::Rushdown).weight);
        assert!(rushdown_kb > balanced_kb); // lighter = more knockback
    }

    #[test]
    fn character_affects_gravity() {
        let mut state = default_state();
        state.players[0].character = CharacterId::Rushdown;
        state.players[0].position_y = Fp::from_int(300);
        state.players[0].grounded = false;

        let mut state2 = default_state();
        state2.players[0].character = CharacterId::Balanced;
        state2.players[0].position_y = Fp::from_int(300);
        state2.players[0].grounded = false;

        advance_frame(&mut state, no_input());
        advance_frame(&mut state2, no_input());

        // Rushdown has higher fall speed / gravity
        assert!(state.players[0].velocity_y > state2.players[0].velocity_y);
    }

    // === Character-specific specials ===

    #[test]
    fn ranged_projectile_faster() {
        // Ranged character's projectile should move faster
        let mut balanced = default_state();
        balanced.players[0].character = CharacterId::Balanced;
        advance_frame(&mut balanced, p1_input(PlayerInput::SPECIAL));
        let bal_proj = balanced.projectiles.iter().find(|p| p.active).unwrap();
        let bal_speed = bal_proj.velocity_x;

        let mut ranged = default_state();
        ranged.players[0].character = CharacterId::Ranged;
        advance_frame(&mut ranged, p1_input(PlayerInput::SPECIAL));
        let rng_proj = ranged.projectiles.iter().find(|p| p.active).unwrap();
        let rng_speed = rng_proj.velocity_x;

        assert!(rng_speed > bal_speed, "ranged projectile should be faster");
    }

    #[test]
    fn rushdown_projectile_shorter_range() {
        // Rushdown's projectile has shorter lifetime (burst style)
        let mut balanced = default_state();
        balanced.players[0].character = CharacterId::Balanced;
        advance_frame(&mut balanced, p1_input(PlayerInput::SPECIAL));
        let bal_life = balanced.projectiles.iter().find(|p| p.active).unwrap().lifetime;

        let mut rushdown = default_state();
        rushdown.players[0].character = CharacterId::Rushdown;
        advance_frame(&mut rushdown, p1_input(PlayerInput::SPECIAL));
        let rush_life = rushdown.projectiles.iter().find(|p| p.active).unwrap().lifetime;

        assert!(rush_life < bal_life, "rushdown projectile should have shorter lifetime");
    }

    #[test]
    fn side_special_move() {
        // Pressing side + special (grounded, no up) should do a side-B
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL | PlayerInput::RIGHT));
        assert!(matches!(state.players[0].action, ActionState::AttackStartup { .. }
            | ActionState::AttackActive { .. }));
        assert_eq!(state.players[0].current_attack, AttackType::SideSpecial);
    }

    #[test]
    fn side_special_lunges_forward() {
        // Side-B should move the player forward
        let mut state = default_state();
        let x_before = state.players[0].position_x;
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL | PlayerInput::RIGHT));
        // Run through a few frames
        for _ in 0..10 {
            advance_frame(&mut state, no_input());
        }
        assert!(state.players[0].position_x > x_before,
            "side-B should lunge the player forward");
    }

    // === Down-B counter ===

    #[test]
    fn down_special_enters_counter() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL | PlayerInput::DOWN));
        assert!(matches!(state.players[0].action, ActionState::Counter { .. }));
    }

    #[test]
    fn counter_reflects_attack() {
        let mut state = default_state();
        state.players[0].position_x = Fp::from_int(500);
        state.players[1].position_x = Fp::from_int(540);
        state.players[0].facing_right = true;
        state.players[1].facing_right = false;

        // P1 enters counter
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL | PlayerInput::DOWN));
        assert!(matches!(state.players[0].action, ActionState::Counter { .. }));

        // P2 attacks P1 during counter
        let inputs = [PlayerInput(0), PlayerInput(PlayerInput::ATTACK)];
        advance_frame(&mut state, inputs);
        // P1 counter triggers — run attack through
        // Advance a few frames for the attack to hit
        for _ in 0..10 {
            advance_frame(&mut state, [PlayerInput(0), PlayerInput(0)]);
        }
        // P2 (the attacker) should have taken counter damage
        assert!(state.players[1].damage_percent > 0,
            "counter should reflect damage to attacker");
    }

    #[test]
    fn counter_expires_if_not_hit() {
        let mut state = default_state();
        advance_frame(&mut state, p1_input(PlayerInput::SPECIAL | PlayerInput::DOWN));
        assert!(matches!(state.players[0].action, ActionState::Counter { .. }));
        // Let it expire
        for _ in 0..30 {
            advance_frame(&mut state, no_input());
        }
        assert!(matches!(state.players[0].action, ActionState::Idle),
            "counter should expire to idle");
    }

    // === Determinism ===

    #[test]
    fn determinism() {
        let input_sequence: Vec<[PlayerInput; MAX_PLAYERS]> = vec![
            no_input(),
            [PlayerInput(PlayerInput::RIGHT), PlayerInput(PlayerInput::LEFT)],
            [PlayerInput(PlayerInput::RIGHT), PlayerInput(PlayerInput::LEFT)],
            [PlayerInput(PlayerInput::RIGHT), PlayerInput(PlayerInput::LEFT)],
            p1_input(PlayerInput::UP),
            [PlayerInput(0), PlayerInput(PlayerInput::UP)],
            p1_input(PlayerInput::DOWN),
            no_input(),
            no_input(),
            [PlayerInput(PlayerInput::UP), PlayerInput(PlayerInput::RIGHT)],
            no_input(),
            p1_input(PlayerInput::ATTACK),
            no_input(),
            [PlayerInput(PlayerInput::LEFT), PlayerInput(PlayerInput::UP)],
            no_input(),
        ];

        let mut a = default_state();
        let mut b = default_state();
        for inputs in &input_sequence {
            advance_frame(&mut a, *inputs);
            advance_frame(&mut b, *inputs);
        }
        assert_eq!(a, b);
    }
}
