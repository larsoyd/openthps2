# Roadmap

Where OpenTHPS2 is headed once the original game is complete. Finishing the original gameplay and content comes
first; everything below builds on it. Like [OpenRCT2](https://github.com/OpenRCT2/OpenRCT2) and
[OpenTTD](https://www.openttd.org/), OpenTHPS2 keeps the original experience as its foundation and makes room for
modern platforms and community creation. Current progress is in [PROGRESS.md](PROGRESS.md).

## The original game

- **Complete gameplay and career** - every original level, goal, competition, skater and unlock, with THPS2's physics,
  tricks and scoring.
- **Original audio** - sound effects, music and movies from your own game data, with independent volumes and playlists
  of your own music. Played on a modern audio backend, without the bugs of the PC version's DirectSound wrappers around
  sound code written for the PlayStation's sound chip.
- **Compatible saves and parks** - career saves and created parks load in both OpenTHPS2 and the original game. Parks
  that use OpenTHPS2's larger limits are saved in an extended format only OpenTHPS2 reads. Automatic backups
  (autosave) and portable profiles come with them.

## Platforms and presentation

- **Native Windows and Linux** - 64-bit builds with platform-appropriate installation, including native Wayland.
- **High resolutions and ultrawide** - 1440p, 4K and ultrawide with correct aspect ratios and readable, scalable menus.
- **High refresh rates** - interpolated movement and animation while the 30 Hz simulation keeps trick timing and
  game speed.
- **No hard limits** - the modern renderer drops the original's fixed caps on draw distance, models and memory, so
  larger custom assets and worlds fit wherever the file formats and game rules allow.
- **Classic and modern rendering** - the authentic software look or modern GPU graphics, sharing one simulation. The
  classic renderer keeps the original's limits, so content built beyond them is refused with a clear message.
- **Displays and cameras** - windowed, fullscreen or borderless play, widescreen-aware framing and adjustable field of
  view.
- **Keyboard and controllers** - detected modern controllers, remapping and dead zones.
- **Accessibility** - readable text options and a choice of classic or modern interface.
- **Installer and updates** - binary releases for Windows and Linux, a graphical installer that lets you pick your
  THPS2 install and keeps OpenTHPS2 up to date. Update checks can be turned off.
- **Offline first** - portable or installed builds that find and check your game data and never need an account.

## Playing together

- **Local multiplayer** - the original two player modes (Trick Attack, Graffiti, Tag, HORSE and Free Skate) and split
  screen, plus a new Versus mode where both play at the same time and a Sandbox mode where both skate without a time
  limit.
- **LAN and online play** - the original multiplayer modes over a network between OpenTHPS2 players, with version and
  content checks. Playing with the original game's DirectPlay network code is not planned.
- **Community servers** - dedicated servers for Windows and Linux with maps, administration and optional modes, and many
  players in one session.
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

Gameplay stays faithful by default. Cheats, assistance and rebalanced physics will always be opt-in, and fixes of the
original's bugs are listed in [BUGFIXES.md](BUGFIXES.md).
