# Migration plan: JS canvas -> Rust + macroquad (WASM)

Branch: `rust-wasm-rewrite`. Goal: port the game faithfully, then extend it with
sprites (beholder bases, eye units, map background) and particle effects.

## Architecture

```
Cargo.toml                 workspace
crates/oculus-core/        pure game rules. No macroquad, no I/O, no global RNG.
                           Deterministic: Game::new(seed). Fully unit tested.
crates/oculus-game/        macroquad app: rendering, input, screens, particles.
                           Builds native (cargo run) and wasm32-unknown-unknown.
web/index.html             page shell around the macroquad canvas
web/gl.js                  vendored miniquad JS loader (matches miniquad version in Cargo.lock)
scripts/build-web.sh       builds wasm and assembles dist/
assets/                    game art (embedded with include_bytes!)
docs/                      plan, README screenshots
```

Boundary rules:
- `oculus-core` owns all rules and state, including selection/click logic.
- Core advances in fixed ticks (`TICK = 1/60 s`). The game crate runs a fixed-timestep
  accumulator (clamped so a backgrounded tab does not fast-forward minutes of play).
- Core reports what happened via `GameEvent`s (`Game::drain_events`). The renderer
  turns events into particles/animations; core never knows about visuals.
- World coordinates are 800x500 (as in the original). The renderer maps the world into the
  window with a letterboxed camera, so input must be converted screen -> world.

## Gameplay rules to preserve (from lib/*.js)

- Bases: home bases at (75,250) player and (725,250) cpu, value 30. Two columns of random
  neutral bases (1-3 per column, x = 225 and 400, mirrored at +350 for the first column);
  sizes drawn from `BASE_SIZES`; radius = value * 1.5; max value = 2 * start value.
- Growth per tick for non-neutral bases below max: `start_value / (1000 / mult)`,
  mult = 1 for player, `1 + 0.1 * (level - 1)` for cpu.
- Sending: half the units (floor) leave; each unit spawns at a random angle and distance in
  `[radius, radius + count]` from the base and flies straight at the target at 3 px/tick.
- Arrival (unit circle r=5 touches target circle): same team +1, else -1; if value <= 0 the base
  flips to the unit's team with value 1.
- Click logic: click your own base to select it; click another base to send; clicking the same
  base, empty space, or a non-owned base as a first click clears the selection.
- AI: every tick, aiTimer += dt; if aiTimer > rand*4000 + 8000/level (ms, re-rolled every tick),
  pick the cpu base with the largest value; if it has >= 15, scale = cpuUnits / playerUnits, and
  attack the first (shuffled) non-cpu base with value < selected.value * scale. Reset aiTimer.
- Win: player has no bases -> defeat (level resets to 1). Cpu has no bases -> victory (level+1).
  Between rounds an intermission screen waits for a click to begin the next level.

## Bugs in the original to fix (not preserve)

- Growth and AI timing ran per animation frame, so 144 Hz monitors played 2.4x faster -> fixed tick.
- Units removed while iterating (`splice` inside `forEach`) skipped collisions.
- `calcVec` used `atan(dy/dx)` (NaN at dx=0) -> normalized direction vector.
- AI `cpu/player` divides by zero when the player has 0 units.
- AI shuffle mutated the shared bases array (changed draw order and click priority).
- Click offsets were captured once at page load, so they broke on scroll/resize.
- The "Level N" HUD was drawn once per object, and a canvas font state leaked between draws.

## Visual direction

- `map.jpg` background, `beholder.png` bases, `eye.png` units (sprites scaled to radius).
- Team identity is a colored ring/glow (player blue, cpu red, neutral dark gray); value text
  stays readable over the sprite (outlined text).
- Particles: unit trails, hit sparks on arrival, a burst on capture, a pulse on the selected base.
- Screens: title ("Click to start"), HUD (level), intermission (victory/defeat + next level).

## Tasks

| # | Task | Owner | Depends on | Status |
|---|------|-------|------------|--------|
| 0 | Toolchain, workspace scaffold, core API contract, web build pipeline | lead | - | done |
| 1 | `oculus-core`: port rules + RNG + unit tests | Sonnet | 0 | done |
| 2 | Particle system module in `oculus-game` (pool, emitters, presets) | Sonnet | 0 | done |
| 3 | Repo hygiene: remove Jasmine boilerplate and stray files, move README screenshots to docs/, .gitignore, CI workflow | Haiku | 0 | done |
| 4 | `oculus-game`: renderer, camera, input, screens, events -> particles | Sonnet | 1, 2 | done |
| 5 | E2E verification in headless Chromium: original JS vs WASM, screenshots, scripted play | Sonnet | 4 | done |
| 6 | Remove legacy JS, README rewrite, final lint/test pass, commit | lead + Haiku | 5 | done |

Quality gates for every task: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, and `scripts/build-web.sh` succeeding.

## Open later

- Deployment: the old site was served from GitHub Pages. Pages deploy of `dist/` via Actions is
  a separate decision once the branch is merged.
