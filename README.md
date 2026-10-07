# thps2-rs

Tony Hawk's Pro Skater 2 reimplementation in Rust and Bevy. Original game files are required and are not distributed here.

## Resource inspection

Use the `ALL.PKR` from the supplied Windows USA rerelease CD (`SETUP/data`). Keep the CD and extracted data outside this source directory.

```bash
cargo run --offline -p thps2-rs -- --data-root /path/to/SETUP/data
cargo run --offline -p thps2-rs -- --data-root /path/to/SETUP/data --manifest
cargo run --offline -p thps2-rs -- --data-root /path/to/SETUP/data --simulate-frames 300
cargo run --offline -p thps2-rs -- --data-root /path/to/SETUP/data --hold right
```

The first command validates the archive, decodes the Hangar and Tony Hawk resources through Bevy and steps the Start skater through 120 design frames of ground movement without input. The second emits tab-separated resource names, sizes and SHA-256 hashes. The third changes the stepped frame count and the fourth holds pad directions (`left`, `right`, `down`, comma separated) for every frame. The step halts with a reported reason when the skater leaves the ground or meets a contact the simulation does not cover yet. These commands do not launch a playable session.

The supported archive SHA-256 is `7e2ae173577c41598dc644824b52f7eaf9664ea478e177a8703f77c0185207da`. Other archives are rejected by the application. The format library reports unsupported compression explicitly.

## Development

Rust 1.89 or newer, edition 2024, Bevy 0.18.1. Dependencies are locked in Cargo.lock. Omit `--offline` if the dependencies are not cached. If a configured compiler wrapper cannot run in your sandbox, set `RUSTC_WRAPPER=`.

```bash
cargo fmt --all --check
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
```

## Captured-view projection

`--view /path/to/view.txt` projects the Hangar source instances through a captured original camera. It reports CPU projection counts and does not open a game window. The strict text format contains three records:

```text
camera X Y Z
matrix M00 M01 M02 M10 M11 M12 M20 M21 M22
viewport RIGHT BOTTOM LEFT TOP NEAR FAR UNKNOWN FOCAL CENTER_X CENTER_Y
```

Camera coordinates are signed integer source units, matrix entries are signed 16-bit Q12, and viewport fields are unsigned 16-bit values. The original viewport's far distance is multiplied by 1.5. This path supports unrotated source items and normal projection; runtime object changes, reflection, lighting and draw submission remain unimplemented. Invalid captures and unsupported item state fail explicitly.

## Static Hangar diagnostic

With an original camera capture in the format above:

```bash
cargo run --offline -p thps2-rs --features viewer -- --data-root /path/to/SETUP/data --view /path/to/view.txt --render
cargo run --offline -p thps2-rs --features viewer -- --data-root /path/to/SETUP/data --view /path/to/view.txt --render --capture /tmp/hangar.png
```

The optional `viewer` feature opens a fixed-camera Bevy window. Escape closes it; `--capture` saves an image after120 frames and exits. Audio is not enabled. Only centered, zero-origin viewports are currently supported.

This diagnostic renders original static geometry and textures with screen offsets, normal-mode clip rejection and clockwise culling. Unlit colors, nearest-repeat sampling, alpha masking and depth-buffer ordering are provisional; they do not reproduce all original lighting/blending/order settings. Dynamic objects, the skater model, animation and gameplay are absent.

Startup also parses Hangar's trigger/restart records and collision zones, explicitly selects `Start`, and creates a skater placement30 units above its accepted ground hit. Collision queries support segments confined to one zone cell; cross-cell queries fail explicitly. AUTOEXEC/restart scripts, trigger execution, moving-object collision, movement and session rules are not implemented.
