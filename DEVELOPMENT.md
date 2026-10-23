# Development guide

Build prerequisites and the interactive Hangar command are in the [README](README.md#getting-started).
This guide describes the current diagnostic interfaces and the constraints on planned extensions.

## Diagnostic runs

```bash
cargo build --locked --release -j 4
cargo run --locked --release -j 4 -p thps2-rs -- --data-root "/path/to/SETUP/data"
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
| `--play` | With the `viewer` feature, skate a Hangar single session with Tony Hawk in a 640x480 window. Arrows move, Z ollies (Enter in the end menus), X flips, C grabs, V grinds, A and S spin. |
| `--frames N --capture PPM` | With `--play`, run N frames without input and save the last one without opening a window. |

The held-input options apply to the command-line simulation, not to `--play`. Unsupported simulation paths can
stop the run with a diagnostic; this is not evidence that every original behavior is implemented.

In `--play` the level, the skater and its shadow, the props, the camera, the clock, the score and special panels, the
trick text and the statistics screen and end menu are drawn like the original by a software renderer. Particles
(sparks, glass shards, blood) are not drawn yet, and the keys are stand-ins until the original bindings are rebuilt.

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
  the static GPU viewer and interactive software-frame presentation behind the `viewer` feature.

Units, coordinates and timing are documented at the boundaries in the source. Positions are 20.12 fixed point in the
original's coordinate system with Y pointing down, angles are 4096 per turn, and one design frame is two 60 Hz ticks.

## Build automation

The [build workflow](.github/workflows/build.yml) compiles the viewer-enabled application for Linux and Windows GNU
on Ubuntu 24.04 runners. The Windows job checks the cross-compilation route used by Ubuntu WSL; it does not boot WSL
or verify a Windows launch. Fedora and Arch package recipes are not separately exercised by this workflow.

Runs are restricted to branch pushes changing Rust/shader sources, Cargo manifests/lockfiles, Rust build/toolchain
configuration or the workflow itself. README, other Markdown and media edits do not trigger a build. There is no
pull-request, scheduled or manual duplicate trigger.

Only builds run: no tests, game launches, original assets or release uploads. The two target jobs run one at a time,
with two compiler jobs each, per-step deadlines and cancellation of superseded runs on the same branch. Rust is pinned
in the workflow to keep caches reusable. [rust-cache](https://github.com/Swatinem/rust-cache) restores downloaded
dependencies and compiled dependency artifacts, keyed by target, toolchain and build inputs. A cold cache or compiler
update still needs a full dependency build; a cache hit does not imply a measured duration guarantee. The Ubuntu
packages are cached as well, because the runner's package mirror has served the MinGW toolchain slowly enough to
time out the install.

## Current architecture and coverage

The [progress map](PROGRESS.md) separates implemented routines, recovered behavior awaiting a port, and open work.
A parser or a recovered native routine alone does not establish a playable feature.

| Area | Present implementation | Remaining work |
| --- | --- | --- |
| Simulation and input | `ground::Run`, `FrameClock`, collision queries, animation state and the trick matcher drive the Hangar session. | Broader state and content coverage, original input options and modern device integration. |
| Presentation | `hangar::Session` composes original-style geometry and HUD through `raster::Raster`; `play.rs` displays the result as a Bevy image. | Interpolation, scalable UI, display choices and a complete GPU gameplay renderer. `renderer.rs` is a static diagnostic. |
| Original scripting | `TrickScripts` reads `TRICKS.BIN`; `Skater::run_script` interprets supported operations. Trigger commands, `ScriptObject` and `Prop` execute reconstructed level behavior. | Remaining instructions and side effects, documented source formats, validation, compilation and reload support. Some prop operations freeze at an unported instruction; some audio/effect operations are skipped. |
| Career and replay data | Career records, save serialization, goal data and some goal awards exist. PC replay headers and state streams can be read. | Complete career integration, recording/playback UI, ghost sessions and broad save/replay compatibility. |
| Assets | Explicit external data root, required archive hash, format readers and a Hangar-specific loader. | Validated overlays, general content selection, mod profiles, additional supported editions and authoring pipelines. |
| Multiplayer and extensions | No integrated multiplayer session, dedicated-server runtime, external source-script compiler or higher-level mod VM. | Authority, synchronization, protocol, permissions, deployment and full integration. |

Animation interpolation within the original animation/camera algorithms is not high-refresh presentation interpolation.
`FixedUpdate` advances `--play` at 30 Hz; `Update` presents the current composed frame without interpolating simulation
snapshots. Preserve that separation when adding rendering at 144, 180 or 240 Hz. Presentation must never advance
physics, trick windows, collision, scoring, timers or authoritative random-number state.

## Planned design priorities

1. Complete original gameplay, career, content, saves, audio and the remaining unsupported paths. Keep comparisons
   against recorded original behavior, with deliberate differences recorded in [BUGFIXES.md](BUGFIXES.md).
2. Make platform, display, input and usability improvements normal defaults as their acceptance checks pass.
   Build GPU and classic presentation around shared simulation snapshots, not separate gameplay engines.
3. Establish validated asset overlays and readable original-script tooling before adding a larger extension runtime.
   Favor documented formats, clear diagnostics and predictable failure behavior over a broad API with unstable semantics.
4. Build multiplayer on an explicit authority and compatibility contract. Dedicated servers, discovery, custom rules
   and community events follow that foundation. No networking library or architecture is selected yet.

OpenRCT2 demonstrates that preservation can coexist with larger limits, new editing tools, modern interfaces,
interpolated display frames, multiplayer and headless servers. Its documented alternatives and cheats coexist with
ordinary improvements, rather than imposing one universal historical mode.
See its [changes to the original game](https://github.com/OpenRCT2/OpenRCT2/wiki/Changes-to-original-game) and
[project introduction](https://github.com/OpenRCT2/OpenRCT2#1-introduction).
Its [changelog](https://github.com/OpenRCT2/OpenRCT2/blob/develop/distribution/changelog.txt) records both modernized
defaults, such as window size and controller dead zones, and selective returns to original gameplay calculations.
These are precedents for individual decisions, not evidence that OpenRCT2 follows OpenTHPS2's proposed policy.
Its [plugin API](https://github.com/OpenRCT2/OpenRCT2/blob/develop/distribution/scripting/scripting.md) also shows the
value of documented external extensions. OpenTHPS2 must make its own decisions about which extensions may run.

## Planned scripting architecture

### Preserve the original VM

Keep the original THPS2 trick VM and user-supplied bytecode executable. First build a disassembler and assembler for
understood instructions, with a documented textual representation and named trick definitions mapped to stable IDs.
Expose names, scores, animation references and sequencing only as their operand semantics become understood.
Validate instructions, table entries, control-flow targets, string termination and referenced resources before execution.

Readable source → validated bytecode → original trace comparison is the first acceptance path. Require byte-exact
round trips for understood portions of the original format, retaining layout and unknown bytes where necessary.
Do not claim full format coverage, or turn an unknown instruction into a guessed high-level operation. Validate new
bytecode independently of the current parser: its bounds checks are not a complete untrusted-code validator, and
its interpreter loops have no general instruction budget.

The level-event path also matters. `ground.rs` dispatches trigger commands; `ground/objects.rs` owns moving props
and script objects with word cursors, labels and waits. Preserve their ordering and interactions rather than forcing
all level behavior into the trick VM.

### A separate extension layer

A higher-level runtime could expose challenges, NPC behaviors, level events and server-side rules beyond the original
instruction set. Luau through Rust's `mlua` is a candidate, not a dependency decision. Its documented
[sandbox, interrupt and memory-limit APIs](https://docs.rs/mlua/latest/mlua/struct.Lua.html) are useful starting points.
They do not establish deterministic execution or make host callbacks safe automatically. Evaluate diagnostic quality,
embedding/build costs, licensing, maintenance, serialization, budget enforcement and reproducibility across platforms
against simpler data-driven extensions and other embeddable runtimes before choosing.

Rust retains ownership of physics, collision, simulation time, scoring and core rules, entity lifetime, world mutation,
resource validation, deterministic RNG, networking authority and save integrity. Scripts request documented operations
through versioned host APIs. Do not move low-level physics into a new language just to expose more scripting surface.

The sandbox must deny ambient filesystem, network and native-code access, including indirect access through host
callbacks. Give mods bounded resources and explicit permissions, validate entity handles, and apply ordered world
commands at defined simulation boundaries. Budget interpreter work, allocations, host work and emitted events.
Deterministic RNG and event ordering need engine-defined semantics; wall-clock timers and unordered iteration cannot
decide authoritative outcomes. A runaway or invalid script must fail without partial world updates or damaged saves.

### Compile and reload safely

The intended authoring loop is:

1. Edit an external human-readable source file without rebuilding Rust.
2. Compile and validate a candidate generation, reporting source locations and unresolved symbols or resource IDs.
3. Keep the last valid generation active if compilation or validation fails.
4. Commit the complete validated replacement at a safe simulation boundary, atomically with its lookup tables.
5. Let existing executions finish on their retained generation, or defer activation until a restart or other proven
   safe point. Require explicit, validated migration for any state transferred between generations.

A frame boundary alone is insufficient. `Skater` stores `script`, `script_return`, `exit_script` and `ollie_script`
byte offsets; input key entries also point into the script buffer. Props and script objects retain their own cursors,
labels, waits and script data. Stable IDs identify definitions, while generation handles must keep every executing
or queued reference valid. Arbitrary in-progress replacement is not promised. The first implementation should prefer
restart/deferred activation when generation ownership is not yet proven.

Production multiplayer sessions should pin scripts, rules and content for the session. Live authoring belongs in an
explicit development session, not an undisclosed rule change on a public server. Any negotiated change must be visible
to participants, synchronized and represented in replay/competition metadata.

## Planned assets, networking and persistence

Use a layered virtual asset lookup: a validated, read-only original archive under explicitly selected mod overlays.
Define canonical paths, precedence, dependency versions and conflict diagnostics. Validate sizes, references and
archive paths, reject traversal and out-of-root access, and bound decompression and allocations. Never weaken the
original archive hash gate to enable mods or require ordinary mods to repack copyrighted assets. Additional original
editions need separate format recovery and supported identities.

A modern network protocol should carry engine/protocol versions, content and script hashes, rule selections and a
clear authority model. Evaluate authoritative simulation with prediction/reconciliation and deterministic approaches
against latency, bandwidth, reproducibility and abuse resistance before choosing. Fixed-point arithmetic alone does
not prove determinism: RNG consumption, update order, scripts and omitted side effects must also agree.

Keep headless servers on the shared simulation. Define authentication and permissions, bounded messages, mod download
consent and integrity checks, moderation, and compatible save/replay snapshots before promising persistent servers.
The Garry's Mod community-server model is inspiration for independently hosted modes, not a claim of equivalent scope.
Spectating is a useful goal; join-in-progress remains conditional on complete state transfer and compatible active
scripts. Original DirectPlay interoperability in terms of network/multiplayer is not necessary and will not be implemented.

Replays and ghosts need engine/rule versions, source-data identity, mod/script generations, input sampling and RNG
state sufficient to reproduce the run, or a documented state-recording format. Reject incompatible playback clearly
instead of silently producing another outcome. Original saves and parks retain their established round-trip goals.
Extensions that cannot fit those formats need separate, versioned storage and an explicit export boundary, not
truncation of modded data into an original save. Backups must not silently overwrite the last usable profile.

Original audio and movies come only from local user-supplied data. Community audio and custom playlists need their own
rights and attribution; local playback does not imply permission to redistribute a track.

## Compatibility decisions

[README highlights](README.md#highlights) govern the finished product. Record confirmed original defects
and their policy in [BUGFIXES.md](BUGFIXES.md), independently of incomplete ports and optional enhancements.

- Keep original movement, collision outcomes, trick timing, scoring and career rules authoritative by default.
  Apparent defects affecting speedruns, animation synchronization, replays, stock content or multiplayer need evidence
  before correction. Record the decision and any justified opt-in alternative.
- Correct non-disruptive defects by default. `Fixes::multiplier_text` is the current precedent: the display uses the
  landing multiplier without changing awarded points. `Fixes::ORIGINAL` is an internal comparison mechanism, not an
  advertised player preset or a requirement that every future fix retain old behavior.
- Treat rendering, filtering, UI appearance, camera preferences and music selection as presentation choices.
  Default to correct DPI/aspect handling and, once validated, smooth presentation; classic choices reuse gameplay.
- Make cheats, altered physics/scoring, easier balance and gameplay-changing mods explicit opt-ins. Keep their
  progression and competitive results distinct. Mod loading and the runtime itself need not be disabled by default.
- Remove 32-bit process restrictions, obsolete APIs, unsafe behavior and avoidable crashes without legacy toggles.
  Before raising an asset/entity limit, check deliberate game rules, reference widths, ordering, saves and protocols.
  Wider memory capacity does not justify changing fixed-point math or overflowing an original file field.

Reject a universal “100% original” engine mode, duplicate gameplay engines for renderer choices, unrestricted native
plugins, arbitrary live-bytecode replacement and mandatory centralized accounts. These add large compatibility or
security costs without being necessary to preserve the game. Keep targeted comparison mechanisms only where they
answer a concrete regression question; do not retain unsafe execution paths to reproduce a crash.
