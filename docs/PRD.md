# PRD: Ejkore — 2D Platform Fighter

## Problem Statement

There is no browser-native platform fighter that offers competitive-grade netcode with zero install friction. Existing platform fighters require native installs, and browser-based fighting games typically use server-authoritative architectures with poor latency characteristics. Players want a Smash-like experience they can share with a friend via a link and a room code, with rollback netcode that feels responsive.

## Solution

Build a 2D platform fighter inspired by Super Smash Bros, running entirely in the browser via Rust compiled to WebAssembly. The game uses peer-to-peer rollback netcode (GGRS + matchbox/WebRTC) for low-latency multiplayer. Players connect via room codes through a lightweight signaling server. The game features 3 distinct characters, a full competitive moveset, and Smash-style percent-based knockback with stock-based win conditions.

## User Stories

1. As a player, I want to open the game in my browser with no install, so that I can start playing immediately
2. As a player, I want to create a room code and share it with a friend, so that we can play together
3. As a player, I want to join a match by entering a room code, so that I can connect to a specific opponent
4. As a player, I want to select from 3 different characters before a match, so that I can pick a playstyle I enjoy
5. As a player, I want smooth, responsive controls despite network latency, so that the game feels fair and reactive
6. As a player, I want to move my character left/right, jump, double jump, and fast-fall, so that I have full platform movement control
7. As a player, I want to perform tilt attacks by pressing a direction and attack, so that I can use quick, low-commitment moves
8. As a player, I want to perform smash attacks by holding a modifier key plus direction and attack, so that I can land high-knockback KO moves
9. As a player, I want to perform aerial attacks in five directions (neutral, forward, back, up, down), so that I have a full air combat toolkit
10. As a player, I want to use four special moves (neutral-B, side-B, up-B, down-B), so that my character has unique abilities
11. As a player, I want to grab and throw my opponent, so that I can punish shielding
12. As a player, I want to shield to block attacks, so that I have a defensive option against strikes
13. As a player, I want to roll, spot dodge, and air dodge, so that I have invincible defensive movement options
14. As a player, I want my shield to degrade when hit and break if depleted, so that defense has a cost and can be punished
15. As a player, I want my opponent's knockback to increase as their damage percent rises, so that matches have escalating tension
16. As a player, I want to influence my knockback trajectory with directional input (DI), so that I have counterplay when launched
17. As a player, I want to grab the ledge when recovering, so that I can get back to the stage safely
18. As a player, I want ledge invincibility when grabbing the edge, so that I have a fair chance to recover
19. As a player, I want to respawn on a revival platform with temporary invincibility after losing a stock, so that I have a fair reset
20. As a player, I want to see the current damage percent and stock count for both players, so that I know the match state
21. As a player, I want to see a results screen showing the winner after a match, so that the outcome is clear
22. As a player, I want to rematch or return to lobby after a match ends, so that I can keep playing
23. As the balanced character, I want medium speed, weight, and straightforward attacks, so that I have a well-rounded playstyle
24. As the ranged character, I want projectile-based special moves, so that I can control space and fight from a distance
25. As the rushdown character, I want fast movement and quick attacks, so that I can pressure opponents with speed
26. As a player, I want projectiles to despawn after a set lifetime or on hit, so that the stage doesn't get cluttered
27. As a player, I want attacks to have visible startup and recovery, so that I can read and punish my opponent's commitments
28. As a player, I want to drop through soft platforms (on future stages), so that I can navigate multi-platform stages

## Implementation Decisions

### Architecture

- **Cargo workspace** with two crates:
  - `game/` — pure game logic library. Physics, hitboxes, knockback, state management, frame data. Zero platform dependencies. Fully deterministic and unit-testable.
  - `client/` — WASM binary. Canvas 2D rendering, keyboard input, GGRS integration, matchbox networking, browser bindings via `web-sys`/`wasm-bindgen`. Depends on `game/`.
- **Signaling server** — off-the-shelf matchbox signaling server in a Docker container. No custom server code.

### Networking

- **Peer-to-peer** via WebRTC data channels using the `matchbox` crate
- **Rollback netcode** via `GGRS` (Rust GGPO implementation)
- Players connect through a **matchbox signaling server** for WebRTC handshake, then communicate directly
- **Room codes** for peer discovery — Player A creates a room, shares code, Player B joins
- Signaling server runs in Docker for easy deployment

### Game Simulation

- **60 FPS fixed timestep** — game logic ticks at exactly 60 FPS, decoupled from render rate
- **Fixed-point math** for all physics calculations — guarantees cross-platform determinism required for rollback
- **Flat struct game state** with fixed-size arrays, no heap allocations — enables fast `memcpy` save/restore for GGRS rollback
- Game state includes: player positions/velocities/damage/stocks, animation states, active hitboxes, active projectiles, input history

### Input

- **Keyboard only** for v1
- Input encoded as a compact **`u16` bitfield**: left, right, up, down, attack, special, shield, grab, smash modifier
- **Smash modifier key** — hold modifier + direction + attack for smash attacks; without modifier = tilt attacks
- GGRS synchronizes inputs between peers each frame

### Combat System

- **Hitbox/hurtbox system** — frame-data driven, defined per move per frame
- **Attack anatomy**: startup frames (no hitbox) → active frames (hitbox out) → recovery frames (cooldown)
- **Knockback formula**: `knockback = base_kb + (damage_percent * kb_scaling * weight_factor)`, direction set per hitbox
- **Directional Influence (DI)**: player being launched can shift knockback angle by up to ~15 degrees by holding a direction
- **Hitstun**: proportional to knockback received, enables combos
- **Projectiles**: traveling hitboxes with velocity, lifetime, and owner. Max 2 active projectiles per player.

### Defensive Mechanics

- **Shield**: blocks attacks, has health meter that depletes on hit, regenerates when not active. Full depletion = shield break (stunned ~2 seconds)
- **Roll**: shield + direction on ground = invincible directional dodge
- **Spot dodge**: shield + no direction on ground = invincible in place
- **Air dodge**: shield in air = brief invincibility with directional momentum
- **Combat triangle**: attack beats grab, grab beats shield, shield beats attack

### Characters

- **3 characters** at launch, defined via **RON data files** (not hardcoded)
- Each character has ~18 moves: jab, 3 tilts, 3 smash attacks, 5 aerials, 4 specials, grab + throw
- Character archetypes:
  - **Balanced** — medium speed, medium weight, straightforward attacks (Mario archetype)
  - **Ranged** — projectile-focused specials, space control (Samus/Link archetype)
  - **Rushdown** — fast movement, quick attacks, light weight (Fox/Captain Falcon archetype)
- Character properties defined in RON: weight, walk/run/air speed, fall speed, jump heights
- Move properties defined in RON: frame data, hitbox geometry, damage, knockback values

### Stage

- **1 stage** at launch — Final Destination style (flat, no platforms)
- **Logical resolution**: 1280x720 (16:9), scaled to fit any screen via CSS/canvas scaling
- Main platform: ~800 units wide, positioned near bottom of viewport
- **Blast zones**: ~300 units beyond each visible edge on all four sides
- **Fixed camera** — no dynamic zoom or follow
- Letterboxed on non-16:9 displays

### Ledge Mechanics

- Auto-snap to ledge when in air and within grab range
- Ledge options: climb up, roll on, jump off, drop off
- ~30 frames of ledge invincibility on grab
- One ledge grab per airborne state (must land or get hit to regrab)
- One player per ledge — second grab forces first player off

### Respawn

- After losing a stock, respawn on revival platform at top-center
- ~120 frames (2 seconds) of invincibility
- Platform disappears after 3 seconds or when player acts (jumps/attacks)

### Match Flow

- **3 stocks** per player
- Last player with stocks remaining wins
- Game flow: Title screen → Character select → Room code (create/join) → Fight → Results screen → Rematch or lobby

### Rendering

- **Canvas 2D** via `web-sys` bindings
- **Prototype art**: colored rectangles for characters (hurtbox visible), different color for hitboxes
- **Target art style** (post-prototype): cel-shaded, inspired by Viewtiful Joe — bold outlines, flat color, dramatic shading
- Render decoupled from simulation — interpolate between game states for smooth display

### Audio

- **Skipped for v1**

### Build Workflow

- **Makefile-driven** with self-documenting help
- Key targets: `build` (wasm-pack), `serve` (local dev server), `signaling` (Docker matchbox server), `dev` (all together), `test` (Rust unit tests), `clean`
- `.env` file loading for configuration
- Phony targets declared

### Designed For Later (Out of Scope for v1)

- 4-player support (game state struct accommodates it, networking does not yet)
- Gamepad input
- Additional stages (including soft platforms with drop-through)
- Wavedashing, L-canceling, dash dancing, wall jumping
- Parry / perfect shield
- Teching
- Stale move negation
- Auto-matchmaking
- Audio (SFX + music)
- Cel-shaded art replacement
- Spectator mode
- Dynamic camera

## Testing Decisions

- **Determinism is the #1 testing priority.** The most critical test: run two identical game simulations with the same input sequence, assert byte-identical game state after N frames. If this ever fails, rollback will desync.
- **Game logic (`game/` crate) is thoroughly unit-tested.** It has zero platform dependencies, making tests straightforward.
- **Test behaviors through the public `GameState` interface**, not internal implementation. Tests call `advance_frame(inputs)` and assert on resulting state (positions, damage, stocks, etc.).
- **Key behaviors to test**:
  - Knockback scaling with damage percent
  - DI angle modification
  - Shield health depletion and break
  - Combat triangle (attack > grab > shield > attack)
  - Hitbox/hurtbox collision detection
  - Projectile lifetime and despawn
  - Ledge grab rules (one per airborne state, one player per ledge)
  - Respawn invincibility timing
  - Stock depletion and match end detection
  - Fixed-point math correctness
- **Client crate (`client/`) is not unit-tested in v1.** Rendering and browser bindings are tested manually.
- **Frame data loaded from RON files is validated** at load time — ensure all required fields present, frame counts are positive, hitbox dimensions are valid.

## Out of Scope

- Native builds (desktop, mobile) — browser only via WASM
- Online ranking, leaderboards, or player accounts
- Character unlocks or progression systems
- Stage builder or custom content
- Replays (though rollback state history could enable this later)
- Tournaments or bracket management
- Chat or voice communication between players
- Accessibility features (remappable controls, colorblind modes) — important but deferred
- Monetization

## Further Notes

- The name "Ejkore" is the working title for this project
- The fixed-point math library choice should be evaluated early — `fixed` crate or a custom implementation
- RON files for character/move data enable rapid iteration and community modding potential
- The flat game state struct design constrains some features (max players, max projectiles) but these limits are acceptable and can be raised by changing constants
- WebRTC via matchbox gives UDP-like characteristics in the browser, which is unusual and valuable for fighting games — this is a genuine technical differentiator over WebSocket-based browser games
