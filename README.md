# Ejkore

A browser-native 2D platform fighter inspired by Super Smash Bros, built in Rust and compiled to WebAssembly. No downloads, no installs — share a link and a room code, and you're fighting.

## The Game

Ejkore is a competitive platform fighter where players knock each other off the stage rather than depleting health bars. Damage accumulates as a percentage — the higher your percent, the further you fly when hit. Lose all three stocks and you're out.

**Three characters, three playstyles:**

- **The Balanced** — medium speed, medium weight, honest attacks. The fundamentals character.
- **The Ranged** — projectile-focused specials for space control and zoning. Keeps opponents at arm's length.
- **The Rushdown** — fast movement, quick attacks, light weight. Wins by staying in your face.

Each character has a full competitive moveset: tilts, smash attacks, five aerial attacks, four special moves, grabs, and throws. Shield, roll, spot dodge, and air dodge give you defensive options. Directional Influence (DI) lets you shift your knockback trajectory when launched, adding counterplay at every percent.

**Art direction:** Cel-shaded, inspired by Viewtiful Joe — bold outlines, flat color, dramatic shading. Currently in prototype phase with colored rectangles standing in for characters.

## How It Works

The game runs entirely in your browser. Under the hood:

- **Rust compiled to WebAssembly** — deterministic game logic at 60 FPS
- **Peer-to-peer rollback netcode** via [GGRS](https://github.com/gschup/ggrs) — competitive-grade networking with client-side prediction and rollback on misprediction
- **WebRTC data channels** via [matchbox](https://github.com/johanhelsing/matchbox) — UDP-like transport in the browser for low-latency input exchange
- **Fixed-point math** — guarantees identical simulation results on both peers, preventing desync
- **Canvas 2D rendering** — lightweight, no WebGL overhead
- **Flat struct game state** — save/restore via memcpy for fast rollback

A lightweight signaling server handles WebRTC handshakes. Once connected, players communicate directly — no game server in the middle.

## Project Structure

```
ejkore/
├── game/           # Pure game logic library (no platform deps)
│   └── src/
│       ├── lib.rs      # Module declarations + tests
│       ├── state.rs    # GameState, PlayerInput, advance_frame
│       ├── fixed.rs    # Deterministic fixed-point arithmetic
│       └── combat.rs   # Knockback, DI, collision detection
├── client/         # WASM binary (rendering, input, networking)
│   └── src/
├── dist/           # Static web files served to the browser
│   └── index.html
├── docs/
│   └── PRD.md      # Product requirements document
├── Dockerfile      # 3-stage build: base → test → artifact
├── docker-compose.yml
├── nginx.conf      # Serves WASM bundle with correct MIME types
├── Makefile        # All build/test/run commands
└── Cargo.toml      # Workspace root
```

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.94+)
- [Docker](https://docs.docker.com/get-docker/) (for signaling server and containerized builds)

### Local Development

```bash
# Install Rust tooling (wasm-pack, wasm32 target)
make setup

# Run game logic tests
make test

# Run tests on every file change
make test-watch

# Build WASM, start signaling server, and serve the game
make dev
```

The game will be available at `http://localhost:8080` with the signaling server on `ws://localhost:3536`.

### Docker Compose

Run everything in containers with no local tooling required (besides Docker):

```bash
# Build all images
make docker-build

# Start client + signaling server
make docker-up

# Run tests in Docker (output to ./coverage/)
make docker-test

# Tail logs
make docker-logs

# Stop everything
make docker-down
```

### All Make Targets

```
make help
```

| Target | Description |
|---|---|
| `help` | Show all targets |
| `setup` | Install wasm-pack and wasm32 target |
| `build` | Build game logic (native) |
| `build-wasm` | Compile client to WASM |
| `test` | Run all game logic tests |
| `test-watch` | Re-run tests on file change |
| `fmt` | Format all Rust code |
| `lint` | Run clippy lints |
| `check` | Format + lint + test |
| `serve` | Serve game at localhost:8080 |
| `signaling` | Start signaling server (standalone) |
| `dev` | Build + signaling + serve (all in one) |
| `docker-build` | Build all Docker images |
| `docker-test` | Run tests in Docker |
| `docker-up` | Start client + signaling |
| `docker-down` | Stop all containers |
| `docker-logs` | Tail service logs |
| `docker-debug` | Shell into client container |
| `clean` | Remove artifacts, stop containers |

## Architecture

```
┌─────────┐   room code   ┌───────────┐
│ Player A ├──────────────►│ Signaling │◄──────────────┤ Player B │
│ (Browser)│               │  Server   │               │ (Browser)│
└────┬─────┘               └───────────┘               └────┬─────┘
     │                                                      │
     │              WebRTC Data Channel (P2P)                │
     └──────────────────────────────────────────────────────┘
                        inputs ↔ inputs

     ┌──────────────────────────────────────────┐
     │              Each Browser                │
     │  ┌────────────┐    ┌──────────────────┐  │
     │  │ game crate │    │  client crate    │  │
     │  │ (WASM)     │    │  (WASM)          │  │
     │  │            │    │                  │  │
     │  │ GameState  │◄───│ GGRS + matchbox  │  │
     │  │ advance()  │    │ Canvas 2D render │  │
     │  │ save()     │    │ Keyboard input   │  │
     │  │ restore()  │    │                  │  │
     │  └────────────┘    └──────────────────┘  │
     └──────────────────────────────────────────┘
```

## Game Mechanics

| Mechanic | Description |
|---|---|
| **Stocks** | 3 lives per player. Cross a blast zone and you lose one. |
| **Damage %** | Starts at 0. Higher % = more knockback when hit. Resets on KO. |
| **Knockback** | `base + (damage% * scaling) * weight_factor`. Lighter characters fly further. |
| **DI** | Hold a direction when launched to shift your trajectory up to 15 degrees. |
| **Shield** | Blocks attacks. Degrades on hit. Breaks = 2 second stun. |
| **Grab** | Beats shield. Attack beats grab. Shield beats attack. |
| **Ledge** | Auto-grab near edge. 30 frames invincibility. One grab per airborne state. |
| **Respawn** | Top-center revival platform. 2 seconds invincibility. |

## Tech Stack

| Component | Technology |
|---|---|
| Language | Rust |
| Runtime | WebAssembly (browser) |
| Rendering | Canvas 2D via web-sys |
| Netcode | GGRS (rollback) |
| Transport | matchbox (WebRTC P2P) |
| Math | Custom fixed-point (i32, 16 fractional bits) |
| Character data | RON files |
| Signaling server | matchbox_server (Docker) |
| Web server | nginx |
| Build | Makefile + wasm-pack + Docker Compose |

## License

TBD
