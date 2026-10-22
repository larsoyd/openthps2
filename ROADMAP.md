# Roadmap

Where OpenTHPS2 is headed once the original game is complete. Finishing the original gameplay and content comes
first; everything below builds on it. Like [OpenRCT2](https://github.com/OpenRCT2/OpenRCT2) and
[OpenTTD](https://www.openttd.org/), OpenTHPS2 keeps the original experience as its foundation and makes room for
modern platforms and community creation. Current progress is in [PROGRESS.md](PROGRESS.md).

## The original game

- **Complete gameplay and career** - every original level, goal, competition, skater and unlock, with THPS2's physics,
  tricks and scoring.
- **Original audio** - sound effects, music and movies from your own game data, with independent volumes and playlists
  of your own music. Improved from the original which was plagued with bugs due to the DirectSound wrappers for the audio routines that were originally written around the PlayStation’s SPU (Sound Processing Unit)
- **Compatible saves and parks** - career saves and created parks that load in both OpenTHPS2 and the original game (as long as both are built for the Classic Renderer),
  plus automatic backups and portable profiles.

## Platforms and presentation

- **Native Windows and Linux** - 64-bit builds with platform-appropriate installation, including native Wayland.
- **High resolutions and ultrawide** - 1440p, 4K and ultrawide with correct aspect ratios and readable, scalable menus.
- **High refresh rates** - interpolated movement and animation while the 30 Hz simulation keeps trick timing and
  game speed.
- **No Hard Limits** - No hard set limits from vanilla like on draw distance, 3d models, and in memory for the default improved rendering.
- **Classic and modern rendering** - the authentic software look or modern GPU graphics, sharing one simulation. - NB: Classic rendering may fail with a message when loading any skate parks/content relying on its improvements
- **Displays and cameras** - windowed, fullscreen or borderless play, widescreen-aware framing and adjustable field of
  view.
- **Keyboard and controllers** - detected modern controllers, remapping and dead zones.
- **Accessibility** - readable text options and a choice of classic or modern interface.
- **Offline first** - portable or installed builds that find and check your game data and never need an account.

## Playing together

- **Local multiplayer** - the original alternating-player modes and split screen.
- **LAN and online play** - the original multiplayer modes over a network, with version and content checks.
- **Community servers** - dedicated servers for Windows and Linux with maps, administration and optional modes.
- **Server browser and private sessions** - find servers by ping, players and map, or join a private session.
- **Spectating** - watch matches without taking a player slot.
- **Events and leaderboards** - with declared rules, keeping modded results apart from original-rules play.
- **Versus bots** - skate against a computer opponent.

## Creating and modding

- **Custom skaters and equipment** - skaters, clothing and boards shared as independent content.
- **Level creation** - modern editing tools and documented asset formats, alongside the original park editor files.
- **Editable trick scripts** - the original tricks in a readable text format that compiles back to THPS2's scripts.
- **Gameplay scripting** - custom tricks, challenges, props, moving platforms and level events without rebuilding
  the engine.
- **Live script reloading** - edit, check and reload scripts while the game runs.
- **Sandboxed extensions** - new game modes through engine interfaces with clear limits and server authority online.
- **Mod profiles** - load order, dependencies and overlays on top of the original data without changing it.

## Extras

- **Replays, ghosts and photo mode** - revisit runs, race your own ghost and frame the moment.
- **Practice and challenges** - practice sessions, time trials and custom trick challenges, kept apart from career
  progress.
- **Modernized defaults** - interface bugs fixed, helpful loading messages and quiet performance improvements.
- **Expanded limits** - larger custom assets and worlds where the formats and rules allow.

Gameplay stays faithful by default. Cheats, assistance and rebalanced physics will always be opt-in, and fixes of the
original's bugs are listed in [BUGFIXES.md](BUGFIXES.md).
