# Original-game bug fixes

OpenTHPS2 preserves THPS2's gameplay identity while correcting defects that do not meaningfully change it.
This register lists implemented corrections to the original game, not incomplete reimplementation work or planned
features. An entry documents its own scope; it is not a claim that the whole game is complete.

## Confirmed corrections

### HUD-001: displayed trick multiplier

| Field | Decision and evidence |
| --- | --- |
| Affected edition | Windows USA rerelease, the currently supported original data set. |
| Original behavior | From eight combo entries, the pending trick display can show a smaller multiplier than the one used when the combo lands. The original display indexes the multiplier table at twice the entry count, while landing uses the entry count. |
| Corrected behavior | The pending multiplier and displayed lost-combo total use the landing factor. Awarded scores, trick execution and combo rules are unchanged. |
| Concrete comparison | An eight-entry combo with a base of 1,325 displays `1325 X7.5` in the original but banks 10,600 on landing. The correction displays the actual ×8 factor. |
| Intended default | Enabled. `Fixes::default()` selects `Fixes::ALL`, including `multiplier_text`. |
| Fidelity impact | Display and lost-combo text differ intentionally. The authoritative landing score remains the original's. No physics, progression, save-format or rule change is intended. |
| Validation | Original pending-display and landing observations distinguish the incorrect text from the correct awarded score; the implementation selects the existing landing multiplier calculation. Compare display and scoring together when changing this path. |
| Implementation | [Fix selection](crates/thps2-data/src/fixes.rs), [trick text](crates/thps2-data/src/tricktext.rs), [scoring](crates/thps2-data/src/score.rs). |
| Historical comparison | `Fixes::ORIGINAL` disables the correction for internal original-behavior comparisons. This is not a public gameplay option or a universal original-engine mode. |

### CFG-001: keyboard binding shared by two actions

| Field | Decision and evidence |
| --- | --- |
| Affected edition | Windows USA rerelease, the currently supported original data set. |
| Original behavior | When `TH2_OPT.CFG` is loaded, a key already bound to another action is rejected only when it sits in a different slot of that action's list. The same key in the same slot position is accepted, so two actions share one key. |
| Corrected behavior | A key bound to another action in any slot is rejected, and the controls fall back to their defaults as for every other rejected binding. Valid files load unchanged. |
| Concrete comparison | With `JUMP=PAD2,SPACE,RETURN`, the original keeps `GRIND=PAD2,V`, so both actions react to keypad 2, while `GRIND=V,PAD2` resets the controls. The correction resets both cases. |
| Intended default | Enabled. `Fixes::default()` selects `Fixes::ALL`, including `key_clash`. |
| Fidelity impact | Only options files that bind one key to two actions are affected. Gameplay, scoring, saves and the written file layout are unchanged. |
| Validation | Original runs with both files show the accepted and the rejected case. Loading and saving every recorded options file reproduces the original's output with `Fixes::ORIGINAL`, and the corrected loader resets the same-slot case. |
| Implementation | [Fix selection](crates/thps2-data/src/fixes.rs), [options loader](crates/thps2-data/src/options.rs). |
| Historical comparison | `Fixes::ORIGINAL` disables the correction for internal original-behavior comparisons. This is not a public gameplay option. |

## Policy for future fixes

Update this register in the same change that implements a confirmed original-game correction. Use a stable identifier
and a concise entry covering:

1. **Original defect and evidence.** Name the affected supported edition and a reproducible trigger. Record the original
   result and the relevant observation, source location or trace. Distinguish a confirmed defect from an inference.
2. **Corrected result and scope.** Give a concrete before/after example. Separate an intentional correction from missing
   behavior, a Rust regression, an optional feature or a change in game rules.
3. **Default and compatibility decision.** State whether it is enabled by default, an explicit opt-in, a presentation
   preference, or an internal-only comparison. Do not add a permanent toggle merely because the original differed.
4. **Fidelity assessment.** Check trick windows, movement, collision, scoring, progression, animation synchronization,
   speedrunning, stock content, replays, deterministic ordering, multiplayer parity and save/park compatibility as
   applicable. Mark unresolved effects and the evidence needed to settle them.
5. **Validation and implementation.** Link the affected public source and record which comparison demonstrates the
   correction and which unchanged outcomes were checked. Include regression coverage appropriate to the defect.
   Do not describe a proposed or model-only result as observed in the original.

Correct interface errors, safe loading defects and avoidable crashes by default when gameplay outcomes are preserved.
If a defect materially shapes gameplay or compatibility, retain the original result by default while investigating,
or justify a specific alternative with evidence. Cheats, assistance and rebalanced mechanics require opt-in and
clear competitive/progression treatment. Never preserve unsafe execution merely to recreate an old crash.

Keep frame-by-frame comparison available where useful. A targeted historical switch, recorded fixture or isolated
reference calculation may establish the cause of a difference; the cheapest adequate method is preferable to adding
another supported player configuration. Record the method and its limits, and remove obsolete comparison machinery
when it no longer answers a useful question.

This policy complements the [public defaults](README.md#defaults-and-compatibility) and
[development compatibility decisions](DEVELOPMENT.md#compatibility-decisions).
