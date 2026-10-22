<p align="center">
  <img src="media/logo.png" alt="OpenTHPS2" width="380">
</p>

<h1 align="center">OpenTHPS2</h1>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024_edition-bf3112.svg" alt="Rust 2024 edition"></a>
  <a href="https://bevyengine.org/"><img src="https://img.shields.io/badge/Engine-Bevy_0.18-b6963c.svg" alt="Bevy 0.18"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-c6c7c9.svg" alt="MIT license"></a>
  <a href="#status"><img src="https://img.shields.io/badge/Status-in_development-4e3938.svg" alt="In development"></a>
  <a href="PROGRESS.md"><img src="https://img.shields.io/badge/Reimplemented-19.7%25-e0a33a.svg" alt="Share of the executable reimplemented"></a>
</p>

<p align="center">
  <a href="PROGRESS.md"><img src="media/progress.svg" alt="Progress of the reimplementation" width="640"></a>
</p>

Tony Hawk's Pro Skater 2 for Windows, rebuilt in Rust on Bevy from the original game data. The goal is a faithful
reimplementation of the USA rerelease: the same levels, skaters, physics, tricks, scoring and modes, read from your own
copy of the game, with the freedom to fix bugs, easier modding and to add improvements.

> [!IMPORTANT]
> OpenTHPS2 is an independent project, not affiliated with or endorsed by Activision or Neversoft. It contains no game
> files. You need your own copy of the game.

## Status

**Incomplete, with a limited interactive Hangar session.** Tony Hawk can skate, perform tricks and combos, reach the
end of a two-minute single session and retry. Controlled recorded cases match the original frame by frame across skating,
ollies, flips, grabs, lips, grinds, wallrides, manuals, bails, specials, scoring and scripted level events. Selected
captures of the level, skater, camera, HUD, trick text and end screens match within a small per-pixel tolerance.
Unsupported paths can still stop a run, so these comparisons do not establish unrestricted play or a complete game.

The interactive session uses an original-style software renderer at 640×480, presented through Bevy, with a fixed
30 Hz simulation. The separate GPU viewer displays a static scene. Smooth visual interpolation and a modern GPU
gameplay renderer are planned. Career-save handling and goal logic are partially implemented, but a playable career,
the full front end, audio, particles and other playable levels remain unfinished.

<!-- progress --> 19.7% of the executable implemented by bytes (319 of 4720 functions), 9.0% more recovered. <!-- /progress -->
Every reimplemented routine is listed in [PROGRESS.md](PROGRESS.md), which also distinguishes recovered behavior from
code that has been ported.

## Features

These are plans for the finished game, not a list of capabilities available today. Completing the original gameplay
and content comes first. Like [OpenRCT2](https://github.com/OpenRCT2/OpenRCT2) and [OpenTTD](https://www.openttd.org/),
OpenTHPS2 takes the original experience as its foundation while making room for modern platforms and community creation.

- **Complete original gameplay and career (planned)** - Skate the original levels, complete goals and competitions, unlock skaters and equipment, and progress with THPS2's familiar physics, tricks and scoring.
- **Native Windows and Linux support (planned)** - Play on modern 64-bit Windows and Linux systems, including native Wayland desktops, with platform-appropriate installation and device handling.
- **High-resolution and ultrawide displays (planned)** - Play at 1440p, 4K and ultrawide resolutions with correct aspect ratios, per-monitor DPI scaling and readable interfaces.
- **High-refresh-rate rendering (planned)** - Enjoy interpolated movement and animation at modern refresh rates while the original 30 Hz simulation preserves trick timing and game speed.
- **Classic and modern rendering (planned)** - Choose authentic software-rendered presentation or modern GPU graphics, with selectable texture filtering and graphical effects sharing the same gameplay simulation.
- **Flexible displays and cameras (planned)** - Choose windowed, fullscreen or borderless play, widescreen-aware framing and adjustable field of view, with alternative camera behavior available as a preference.
- **Configurable keyboard and controller input (planned)** - Use detected modern controllers, remap buttons and adjust dead zones, with responsive input sampled consistently for the original gameplay timing.
- **Scalable menus and accessibility (planned)** - Navigate readable, scalable menus with clear focus indicators, accessible text options and a choice of classic or modern interface appearance.
- **Audio and soundtrack controls (planned)** - Hear original audio from your own game data, adjust independent volumes, select playback devices and manage playlists of locally supplied music.

- **Local multiplayer (planned)** - Share a machine for original alternating-player modes or additional split-screen play, with each mode's rules and progression clearly identified.
- **LAN and online multiplayer (planned)** - Compete in reconstructed original multiplayer modes over local networks or the internet, with synchronized gameplay and explicit version and content compatibility checks.
- **Dedicated community servers (planned)** - Host persistent headless servers on Windows or Linux, with configurable maps, administration, moderation and optional server-side game modes.
- **Server discovery and private sessions (planned)** - Browse community servers by ping, player count and map, or connect directly to private and password-protected sessions.
- **Spectating (planned)** - Watch multiplayer sessions through spectator cameras without taking a player slot or changing the outcome of the match.
- **Community rules and events (planned)** - Join optional events and leaderboards with declared rules and mod requirements, keeping modified results distinct from original-rules competition.

- **Custom skaters and equipment (planned)** - Create and share independently distributed skaters, models, clothing and boards using documented content interfaces and original-compatible options where supported.
- **Level creation and asset tools (planned)** - Build custom levels with modern editing tools, import and export documented asset formats, and preserve compatible original park files.
- **Editable original trick scripts (planned)** - Read and edit named trick definitions, scores, animations and sequences in a documented text format that compiles to validated THPS2 bytecode.
- **External gameplay scripting (planned)** - Define custom tricks, challenges, NPC behavior, interactive props, moving platforms and level events through documented external scripts without rebuilding the engine.
- **Live script development (planned)** - Validate and compile edited scripts, report useful errors and reload at safe simulation boundaries, retaining valid code until active executions can safely transition.
- **Sandboxed extension API (planned)** - Extend game modes through controlled engine APIs with resource and execution limits, explicit permissions and server authority over multiplayer rules.
- **Mod profiles and asset overlays (planned)** - Select mod profiles and load order, resolve dependencies and conflicts, and layer custom content over validated game data without rewriting the original archive.
- **Community addon discovery (planned)** - Find independently distributed addons with compatibility and permission information, while keeping manual installation available and gameplay-changing content explicitly opt-in.

- **Replays, ghosts and photo tools (planned)** - Record and revisit runs with compatible rules and script versions, compare ghost skaters, and capture moments with replay cameras and photo mode.
- **Practice and custom challenges (planned)** - Choose practice sessions, time trials or custom trick challenges, with assistance and altered rules explicitly enabled and separated from original career progression.
- **Portable profiles and save backups (planned)** - Keep automatic backups, move profiles between installations and exchange supported original saves, with modded progress clearly separated from original-compatible data.
- **Modernized defaults (planned)** - Benefit from corrected interface bugs, useful loading diagnostics, reliable error recovery and unobtrusive performance improvements as part of normal play.
- **Carefully expanded engine limits (planned)** - Use larger custom assets and worlds where validated formats and gameplay rules permit, without inheriting unnecessary limits from the original implementation.
- **Offline-first installation (planned)** - Install portable or platform-integrated builds, locate and validate supported local game data, and play offline without mandatory accounts or a proprietary launcher.

### Defaults and compatibility

THPS2's movement, trick windows, collision, scoring and progression will remain the foundation. Modern platform
behavior, readable interfaces and corrections that preserve meaningful gameplay outcomes will be normal defaults.
The existing [trick-multiplier display fix](BUGFIXES.md#hud-001-displayed-trick-multiplier) already follows this approach: it corrects the text without changing the
score. Changes affecting gameplay, speedrunning, animation timing, replays or determinism will need specific evidence
and a compatibility decision before becoming defaults.

Cheats, assistance, rebalanced physics and gameplay-changing mods will require explicit opt-in. Presentation choices
will share one gameplay simulation: classic visuals will not require historical resolution or hardware restrictions.
Safe mod loading and scripting infrastructure will be ordinary engine capabilities, independent of which mods are enabled.

Unsafe memory behavior, crashes and obsolete operating-system dependencies will not receive permanent toggles solely
for historical accuracy. Limits will be reviewed for gameplay, file-format and synchronization consequences before
removal. Frame-by-frame regression comparisons will remain engineering tools; their internal compatibility mechanisms
will not automatically become player settings. There will be no separate universal “100% original” engine mode.

## Requirements

- Your own copy of Tony Hawk's Pro Skater 2 for Windows (USA rerelease). The game data is not included. The application
  reads `ALL.PKR` from the installed `SETUP/data` directory and refuses any archive whose SHA-256 is not
  `7e2ae173577c41598dc644824b52f7eaf9664ea478e177a8703f77c0185207da`.
- Rust 1.89 or newer. Dependencies are pinned in `Cargo.lock`, Bevy at 0.18.1. Use a current stable toolchain through
  [rustup](https://rust-lang.org/tools/install/) for the builds below.
- For the windowed session, a working graphics driver supported by Bevy/wgpu. The current Linux build uses X11,
  including XWayland on a Wayland desktop. Native Wayland support is planned.

Keep the game files outside this source tree and point the application at them with `--data-root`. Original music and
movies must also come from your own installation when their playback is implemented. No licensed soundtrack or other
proprietary game assets are supplied. Additional editions need separately verified formats and validation rules.

## Build and run

The commands below build the current development version, including its limited Hangar session. Install the
prerequisites for your platform, install Rust through the link above, then open a new terminal and check `rustc --version`.
Linux builds and Windows cross-builds use different targets. The Windows recipe still needs end-to-end verification
on Ubuntu WSL and Windows; it is not a claim of a tested Windows release. The [build workflow](.github/workflows/build.yml)
checks Linux and Windows cross-compilation on qualifying source pushes, with cached dependencies and no test runs.

### Ubuntu LTS

On Ubuntu 24.04 LTS or newer, install the compiler and X11 dependencies:

```bash
sudo apt update
sudo apt install build-essential git curl pkg-config libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxkbcommon-x11-0 libvulkan1
```

Install the Vulkan driver appropriate for your GPU. Ubuntu's Mesa drivers are available as `mesa-vulkan-drivers`;
NVIDIA users need their distribution-supported NVIDIA driver. Continue with [Build on Linux](#build-on-linux).

### Fedora

```bash
sudo dnf install gcc-c++ git curl pkgconf-pkg-config libX11-devel libXcursor-devel libXrandr-devel libXi-devel libxkbcommon-x11 vulkan-loader
```

Use the graphics driver appropriate for your GPU (`mesa-vulkan-drivers` for supported Mesa hardware), then continue
with [Build on Linux](#build-on-linux).

### Arch Linux

On an up-to-date Arch installation:

```bash
sudo pacman -S --needed base-devel git curl libx11 libxcursor libxrandr libxi libxkbcommon-x11 vulkan-icd-loader
```

Install your GPU's Vulkan driver, such as `vulkan-radeon` for supported AMD hardware or `vulkan-intel` for supported
Intel hardware. NVIDIA installations need the matching NVIDIA driver userspace. Continue below.

### Build on Linux

These package lists follow [Bevy 0.18.1's Linux guidance](https://github.com/bevyengine/bevy/blob/v0.18.1/docs/linux_dependencies.md),
adapted to this workspace's enabled features. Audio and gamepad backends are not enabled yet.

```bash
git clone https://github.com/larsoyd/openthps2.git
cd openthps2
cargo build --locked --release -j 4 -p thps2-rs --features viewer
./target/release/thps2-rs --data-root "/path/to/SETUP/data" --play
```

The window is currently fixed at 640×480. Arrows move, Z ollies, X flips, C grabs, V grinds, and A/S spin. Enter confirms
end-menu choices. These bindings are temporary. Unsupported situations produce a diagnostic and can end the session.

For a command-line build and the default 120-frame simulation without a window:

```bash
cargo build --locked --release -j 4
cargo run --locked --release -j 4 -p thps2-rs -- --data-root "/path/to/SETUP/data"
```

### Build a Windows executable with Ubuntu WSL

Follow [Microsoft's official WSL installation tutorial](https://learn.microsoft.com/en-us/windows/wsl/install).
From an administrator PowerShell, install Ubuntu and restart when prompted:

```powershell
wsl --install -d Ubuntu
```

Open Ubuntu, finish its user setup and install Rust **inside Ubuntu** using [rustup](https://rust-lang.org/tools/install/).
For an x86-64 Windows executable, install the native build tools and MinGW cross-compiler in Ubuntu:

```bash
sudo apt update
sudo apt install build-essential git curl pkg-config gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
git clone https://github.com/larsoyd/openthps2.git
cd openthps2
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  cargo build --locked --release -j 4 -p thps2-rs --features viewer --target x86_64-pc-windows-gnu
```

This uses Rust's [cross-compilation workflow](https://rust-lang.github.io/rustup/cross-compilation.html) and an explicit
[Cargo target linker](https://doc.rust-lang.org/cargo/reference/config.html#targettriplelinker).
The output is `target/x86_64-pc-windows-gnu/release/thps2-rs.exe`. Copy it to a Windows folder, then run from PowerShell
with a **Windows** path to your data:

```powershell
.\thps2-rs.exe --data-root "C:\Games\THPS2\SETUP\data" --play
```

If your cross-toolchain links runtime DLLs dynamically, those matching DLLs must accompany the executable. Windows
launch, graphics behavior and redistribution dependencies still need verification before packaging a release.
An ordinary Linux build inside WSL produces a Linux executable; running that through WSLg is a separate route and
uses the Ubuntu/Linux instructions above.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for diagnostic commands, captured camera files, the source layout, and the planned
scripting, modding and compatibility architecture. [BUGFIXES.md](BUGFIXES.md) records confirmed original-game
corrections and the policy for adding new entries.

## License

MIT, see [LICENSE](LICENSE). The game, its data and its name belong to their rights holders; nothing from the game is distributed here.
