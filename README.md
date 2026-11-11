# Oculus Battle

Oculus Battle is a simplified real-time strategy game. You are the blue team and start on the left side of the map. Capture bases and eliminate the red team, which the computer controls. Neutral bases cannot attack and do not generate units.

The game is written in Rust and rendered with [macroquad](https://macroquad.rs). It runs natively and as WebAssembly in the browser.

## How to play

- You and the enemy each start with one base. Blue and red bases generate units over time, based on their starting size.
- Click a base to select it, then click another base to send units there.
- Sending units moves half of the selected base's units to the target. Use this to attack enemy or neutral bases, or to reinforce your own.
- A base can grow to twice its starting size. A full base pulses gently: it stops producing until you spend some of its units.
- When a base drops to zero units, it flips to the attacking team and begins generating units for them.
- The computer's bases grow faster with each level.
- The bar above the board shows each side's total units, the level and how long the level has been running.

### Shortcuts

| Input | Action |
|---|---|
| Shift-click a base | Send from every one of your bases to it at once |
| Ctrl-click (Cmd-click on macOS) your base | Add it to, or remove it from, the selection; then click a target to send from all selected bases |
| A | Select all your bases |
| Esc | Clear the selection |

## Building and running

Prerequisites: [Rust](https://rustup.rs), installed through rustup. For the web build, also add the WebAssembly target:

```sh
rustup target add wasm32-unknown-unknown
```

Run the native game:

```sh
cargo run -p oculus-game --release
```

Build the web version and serve it locally:

```sh
scripts/build-web.sh --serve
```

Then open http://localhost:8000. Without `--serve`, the script only assembles the static site in `dist/`.

Checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Deployment

Every push to `master` that passes CI publishes `dist/` to GitHub Pages at https://www.colinfmcmahon.com/oculusBattle/ (see `.github/workflows/ci.yml`).

## Project layout

- `crates/oculus-core/` - pure, deterministic game rules, with unit tests.
- `crates/oculus-game/` - macroquad frontend: rendering, particles, input, and screens.
- `web/` - HTML page shell and the vendored miniquad `gl.js` loader for the WebAssembly build.
- `scripts/build-web.sh` - builds the WebAssembly bundle and assembles `dist/`.
- `assets/` - map, images, and fonts.
- `art/` - hand-authored SVG sprites for the bases and units.
- `tools/artgen/` - generator for the game art (`assets/base.png`, `assets/unit.png`, `assets/map.jpg`).
- `docs/MIGRATION.md` - migration plan and notes from the original JavaScript version.

## Credits

All art is original. The sprites are hand-authored SVG in `art/`, and the map is generated procedurally by `tools/artgen`. To regenerate `assets/base.png`, `assets/unit.png`, and `assets/map.jpg`, run:

```sh
cargo run -p artgen --release
```

The Cinzel font (`assets/fonts/Cinzel.ttf`) is licensed under the SIL Open Font License; see `assets/fonts/OFL.txt`.
