# NEON RANGE

A 3D keyboard-controlled shooting gallery, built entirely in Rust with Bevy. Track neon drones through an industrial training bay, lead your shots, and survive increasingly difficult waves.

## Run

```sh
cargo run
```

Rust 1.93 or newer and a graphics device supporting Bevy's renderer are required. Cargo downloads dependencies automatically on the first build; the initial compilation can take several minutes. Bevy 0.18.1 is used to support the installed Rust 1.93 toolchain. All geometry, materials, effects, and UI are created in Rust; no asset downloads are needed.

For an optimized build:

```sh
cargo run --release
```

## Controls

| Key | Action |
| --- | --- |
| Enter | Start, resume, or retry |
| WASD / arrow keys | Move the weapon's aim |
| Space (hold) | Fire |
| Shift (hold) | Precision aiming |
| P / Escape | Pause / resume |
| R | Restart |
| F11 | Toggle fullscreen |

The game pauses when the window loses focus. Close the window to quit.

## Rules

- Shoot drones before they cross the orange boundary markers. Five escapes end a run.
- Bullets take time to reach the targets: aim slightly ahead of moving drones.
- Clear the displayed quota to advance a level and restore one integrity point.
- Later levels increase speed, reduce target size, shorten spawn intervals, and add vertical weaving. Difficulty has limits to keep aiming possible.
- Hits earn 100 × level × multiplier points. Every five consecutive hits increases the multiplier, up to 4×. A missed shot or escaped drone breaks the streak.
- The best score is retained across restarts during the current app session.

## Validation

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --smoke
```

The smoke mode opens the actual game, starts a short simulation, saves `/tmp/neon-range-smoke.png`, and exits automatically. It requires a graphical desktop. Unit tests check swept projectile collision and difficulty progression.

On Linux, the system development packages for your platform's display backend and graphics drivers must be available (X11/Wayland and Vulkan). On macOS, install Apple's command line tools if Cargo reports a missing linker: `xcode-select --install`.
