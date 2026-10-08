# thps2-rs

Tony Hawk's Pro Skater 2 for Windows, rebuilt in Rust on Bevy from the original game data. The goal is a faithful
reimplementation of the USA rerelease: the same levels, skaters, physics, tricks, scoring and modes, read from your own
copy of the game, with the freedom to fix bugs and to add widescreen and native 64-bit support along the way.

**Status: not playable yet.** The Hangar level, Tony Hawk's model and animation bank, the collision world and the
skater's ground, air, steering and ollie physics are reconstructed from the executable and checked frame by frame
against models of the recovered routines. Rendering is a static diagnostic. Tricks, scoring, the session timer, menus,
audio and the other levels are open. <!-- progress --> 7.1% of the executable implemented by bytes (92 of 4720 functions), 4.4% more recovered. <!-- /progress --> The full map is in [PROGRESS.md](PROGRESS.md).

## Requirements

- Your own copy of Tony Hawk's Pro Skater 2 for Windows (USA rerelease). The game data is not included. The application
  reads `ALL.PKR` from the installed `SETUP/data` directory and refuses any archive whose SHA-256 is not
  `7e2ae173577c41598dc644824b52f7eaf9664ea478e177a8703f77c0185207da`.
- Rust 1.89 or newer. Dependencies are pinned in `Cargo.lock`, Bevy at 0.18.1.

Keep the game files outside this source tree and point the application at them with `--data-root`.

## Build and run

```sh
cargo build --release
cargo run --release -p thps2-rs -- --data-root /path/to/SETUP/data
```

Without further options the application validates the archive, loads the Hangar and Tony Hawk's data, places the skater
at the Start restart and steps 120 frames of skating with no input. Each run ends with one `[ground]` line giving the
frame count, position, velocity, animation and the reason it stopped early, if any.

| Option | Effect |
|---|---|
| `--manifest` | Print every resource in the archive with its size and SHA-256 and exit. |
| `--simulate-frames N` | Step N frames at the original 30 Hz design rate instead of 120. |
| `--hold KEYS` | Hold pad keys for every frame: `left`, `right`, `down`, `jump`, `kick`, comma separated. Down alone brakes. |
| `--release-after N` | Release the held keys after frame N, for example `--hold jump --release-after 9` for a charged ollie. |
| `--view FILE` | Project the Hangar through a captured original camera (format below). |
| `--render` | With `--view` and the `viewer` feature, open a fixed-camera window of the static Hangar. |
| `--capture PNG` | With `--render`, save an image after 120 frames and exit. |

The simulation stops with an explicit reason at the first situation it does not cover yet, such as a lip, a wallride, a
head-on wall hit or a bail. It never guesses.

## Captured camera format

`--view` takes a text file with three records:

```text
camera X Y Z
matrix M00 M01 M02 M10 M11 M12 M20 M21 M22
viewport RIGHT BOTTOM LEFT TOP NEAR FAR UNKNOWN FOCAL CENTER_X CENTER_Y
```

Camera coordinates are signed integer game units, matrix entries are signed 16-bit Q12 and viewport fields are unsigned
16-bit values. The far distance is multiplied by 1.5 as the original does. Only centered, zero-origin viewports are
supported.

The static viewer draws the original geometry and textures with the original screen offsets, clip rejection and
clockwise culling. Lighting, blending and draw order are provisional. Dynamic objects and the skater model are not drawn.

## Layout

- `crates/thps2-data` reads the original formats and holds the game logic: the PKR archive, PSX scenes, textures and
  materials, the collision world, the animation bank and the skater physics. It does not depend on Bevy.
- `crates/thps2-rs` is the Bevy application: asset loading, the fixed-rate skater step, the captured-view projection and
  the static viewer behind the `viewer` feature.

Units, coordinates and timing are documented at the boundaries in the source. Positions are 20.12 fixed point in the
original's coordinate system with Y pointing down, angles are 4096 per turn, and one design frame is two 60 Hz ticks.

## License

MIT, see [LICENSE](LICENSE). The game, its data and its name belong to their rights holders; nothing from the game is distributed here.
