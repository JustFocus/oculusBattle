# Oculus Battle

Oculus Battle is a simplified real-time strategy game. You are the blue team and start on the left side of the map. Capture bases and eliminate the red team, which the computer controls. Neutral bases cannot attack and do not generate units.

The game is written in Rust and rendered with [macroquad](https://macroquad.rs). It runs natively and as WebAssembly in the browser.

## How to play

- You and the enemy each start with one base. Blue and red bases generate units over time, based on their starting size.
- Click a base to select it, then click another base to send units there.
- Sending units moves half of the selected base's units to the target. Use this to attack enemy or neutral bases, or to reinforce your own.
- A base can grow to twice its starting size.
- When a base drops to zero units, it flips to the attacking team and begins generating units for them.
- The computer's bases grow faster with each level.

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

## Project layout

- `crates/oculus-core/` - pure, deterministic game rules, with unit tests.
- `crates/oculus-game/` - macroquad frontend: rendering, particles, input, and screens.
- `web/` - HTML page shell and the vendored miniquad `gl.js` loader for the WebAssembly build.
- `scripts/build-web.sh` - builds the WebAssembly bundle and assembles `dist/`.
- `assets/` - map, images, and fonts.
- `docs/MIGRATION.md` - migration plan and notes from the original JavaScript version.

## Credits

The Cinzel font (`assets/fonts/Cinzel.ttf`) is licensed under the SIL Open Font License; see `assets/fonts/OFL.txt`.

`assets/map.jpg` and `assets/beholder.png` are placeholder art from the original project. Replace them with original art before any public distribution.
