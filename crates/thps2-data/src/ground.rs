use crate::anim::{AnimState, AnimationTable};
use crate::career::{CareerEvent, CareerRun, GoalKind};
use crate::collision::{CollisionWorld, Hit, shatter_draws};
use crate::input::{KeyTable, Presses};
use crate::rail::{RailHit, Rails};
use crate::rng::Rng;
use crate::score::{Entry, Panel, Shown, Style, TextEvent, hangar_gap};
use crate::script::TrickScripts;
use crate::skater::{STANDING_HEIGHT, dot_q12, mul_q12, normalize, sqrt_int};
use crate::trig::{Command, Regions, Restart, Triggers};

mod grind;
mod manual;
mod objects;
mod pickups;
mod session;
mod wall;

pub use manual::{Balance, QueuedTrick};
pub use objects::{Prop, PropModels, ScriptObject};
pub use pickups::{LETTERS, Pickup};
pub use session::{SESSION_TICKS, Session};

pub const GRAVITY: i32 = 0x1964;
pub const AIR_GRAVITY: i32 = 13000;
pub const WALL_REACH: i32 = 0x1e;
const WALL_REACH_BAILING: i32 = 100;
pub const BOUNCE_REACH: i32 = 10;
const BOUNCE_REACH_BAILING: i32 = 70;
pub const HEAD_REACH: i32 = 0x3c;
pub const UNSTICK_SPAN: i32 = 8000;
const SHADOW_REACH: i32 = 4000;
pub const NORMAL_HOLD_LIMIT: i32 = 0x18000;
pub const FAKIE_DOT: i32 = 0x5000;
pub const BIG_DROP_BASE: i32 = 0x3c3;
pub const WORLD_UP: [i32; 3] = [0, -4096, 0];
pub const TURN_MAX: i32 = 0x2d000;
pub const TURN_MAX_CROUCH: i32 = 0x5a000;
pub const AIR_TURN_MAX: i32 = 0xa0000;
pub const LEAN_FRAMES: i32 = 0x16;
pub const LEAN_PROBE_HEIGHT: i32 = 100;
pub const LEAN_PROBE_REACH: i32 = 0x5a;
pub const WALL_TURN: i32 = 0x19;
pub const WALL_PUSH: i32 = 0xcd;
pub const HEAD_ON_DOT: i32 = 0xb50;
pub const LATE_OLLIE_FRAMES: i32 = 0x14;
pub const LANDING_TIMER: i32 = 0xf;
pub const CROUCH_LEAN_LEFT_FRAMES: i32 = 0xf;
pub const CROUCH_LEAN_RIGHT_FRAMES: i32 = 0xc;
pub const ALIGN_EXTRA: i32 = 0x28;
pub const VERT_FACE: u32 = 0x200000;
const LIP_FACE: u32 = 0x400000;
/// Time after a vert pop during which faces catch the skater instead of landing it.
const POP_CATCH: i32 = 0x1e;
const POP_AIR: i32 = 0x19;
pub const WALL_HIT_TURN: i32 = 200;
pub const PUSH_COOLDOWN: i32 = 0x14;
pub const PUSH_TARGET_INIT: i32 = 0x2e9b6;
pub const PUSH_SPEED: i32 = 0x14000;
pub const BRAKE_STOP: i32 = 0xa000;

pub const STAT_AIR: usize = 0;
pub const STAT_HANGTIME: usize = 1;
pub const STAT_OLLIE: usize = 2;
pub const STAT_SPEED: usize = 3;
pub const STAT_SPIN: usize = 4;
pub const STAT_LANDING: usize = 5;
pub const STAT_SWITCH: usize = 6;
pub const TONY_HAWK_STATS: [i32; 11] = [7, 5, 2, 6, 8, 5, 3, 3, 6, 5, 0];
const SWITCH_WEIGHT: [i32; 11] = [50, 50, 100, 20, 50, 100, 100, 80, 100, 100, 30];
const STAT_CAP: [i32; 11] = [100, 100, 130, 110, 100, 130, 100, 130, 130, 130, 100];
const SPECIAL_BONUS: [i32; 11] = [30, 10, 100, 50, 20, 100, 100, 100, 100, 100, 100];

pub const ANIM_IDLE: u16 = 0;
pub const ANIM_PUSH_START: u16 = 1;
pub const ANIM_PUSH_ALT: u16 = 2;
pub const ANIM_PUSH: u16 = 3;
pub const ANIM_PUSH_SPECIAL: u16 = 0x5e;
pub const ANIM_BRAKE: u16 = 0x31;
pub const ANIM_BRAKE_FAST: u16 = 0x83;
pub const ANIM_OLLIE: u16 = 4;
pub const ANIM_LAND: u16 = 5;
pub const ANIM_CROUCH: u16 = 8;
pub const ANIM_WALL_HIT: u16 = 0xc;
pub const ANIM_AIR: u16 = 0xe;
pub const ANIM_CROUCH_LAND: u16 = 0x19;
pub const ANIM_CROUCH_FALL: u16 = 0x1a;
pub const ANIM_FALL: u16 = 0x1b;
pub const ANIM_NOLLIE: u16 = 0xaf;
const ANIM_BONELESS: u16 = 0x91;
const ANIM_NO_COMPLY: u16 = 0xd8;
const UP: u8 = 1;
const UP_FIRST_WINDOW: i32 = 20;
const UP_SECOND_WINDOW: i32 = 35;
const BONELESS_POP: i32 = 0x5000;
const BONELESS_POINTS: i32 = 250;
const NO_COMPLY_POINTS: i32 = 200;
const WALLIE_POINTS: i32 = 200;
const WALLIE_POP: i32 = 0xf000;
/// An animation during which riding backwards does not turn the board round.
const ANIM_NO_FAKIE: u16 = 0x22;
const CROUCH_END: i32 = 0x1a;
const CROUCH_PINGPONG: i32 = 0x13;
const BAIL_SLOW_SPEED: i32 = 0x2d000;
const BAIL_TILT: usize = 3;
const BAIL_END_COOLDOWN: i32 = 4;
const SCRIPT_LAST_FRAME: i16 = 0x2711;
const GAP_SLOTS: usize = 20;
const LEVEL_SEED: u32 = 0x12a6cc58;
const DEFAULT_TRICK_POINTS: i32 = 0x7b;
const MASH_SPAN: i32 = 20;
/// Points per half spin left over at a landing with no gap to take them.
const SPIN_OLLIE_POINTS: i32 = 50;
const GRIND_UP_FRAMES: i32 = 0x13;
const WALLRIDE_GAP: i32 = 0x29;
const BOOST_MAX: i32 = 20;
const JUMP_BLOCK_TICKS: i32 = 10;
const UNLIMITED_PULSES: u16 = 0xffff;
const PULSE_DEPTH: u32 = 8;
// objects a pulse creates are not part of the rewrite world, so pulsing or killing them changes nothing
const MOVING_OBJECT: i16 = 1;
const CREATED_OBJECTS: [i16; 3] = [MOVING_OBJECT, 5, SIGNALLED_OBJECT];
/// The other node kind a signal reaches besides moving objects.
const SIGNALLED_OBJECT: i16 = 7;
/// Env item nodes, a kill shatters their glass.
const ENV_ITEMS: [i16; 2] = [2, 9];
/// Env items and trick objects, the nodes a visibility script names by checksum.
const VISIBLE_ITEMS: [i16; 4] = [2, 9, 12, 14];

/// Digital pad state for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
    pub kick: bool,
    pub grab: bool,
    pub grind: bool,
    pub spin_left: bool,
    pub spin_right: bool,
}

const SPECIAL_FILL: i32 = 3000 << 12;
const SPECIAL_FULL: i32 = 750 << 12;

/// Special meter in Q12: landed points fill it until 3000 have gone in, a full meter then drains
/// and tops up by a fifth of each landing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Special {
    pub fill: i32,
    pub meter: i32,
}

impl Special {
    pub fn full(self) -> bool {
        self.meter & !0xfff != 0
    }

    fn drain(&mut self, hold_delta: i32) {
        for v in [&mut self.fill, &mut self.meter] {
            if *v & !0xfff != 0 {
                *v -= 2 * hold_delta;
                if *v & !0xfff <= 0 {
                    *v = 0;
                }
            }
        }
    }

    fn land(&mut self, pending: i32) {
        if self.full() {
            let meter = self.meter + ((pending as u32 / 5) << 12) as i32;
            self.meter = if meter & !0xfff > SPECIAL_FULL {
                SPECIAL_FULL
            } else {
                meter
            };
            return;
        }
        self.fill += pending << 12;
        if self.fill & !0xfff >= SPECIAL_FILL {
            (self.fill, self.meter) = (0, SPECIAL_FULL);
        }
    }
}

/// How the skater rides for stat reads: switch stance, and the special meter full.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stance {
    pub switched: bool,
    pub special: bool,
}

impl From<bool> for Stance {
    fn from(switched: bool) -> Self {
        Self {
            switched,
            special: false,
        }
    }
}

/// Skater stat bytes in native order: air, hangtime, ollie, speed, spin, landing, switch, rail, lip, manual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub stats: [i32; 11],
    /// Multiplayer handicap, single player reads the career record instead.
    pub handicap: Option<i32>,
    pub auto_kick: bool,
    /// Board turning setting, 1 unless changed in skater select.
    pub deck_turning: u8,
}

impl Rules {
    /// Stat as ten per point, multiplayer adds two plus the handicap and floors at 30, a full
    /// special meter adds its bonus, then the cap and the switch scaling apply.
    pub fn stat(self, index: usize, stance: impl Into<Stance>) -> i32 {
        let stance = stance.into();
        let base = match self.handicap {
            None => self.stats[index] * 10,
            Some(h) => ((self.stats[index] + 2 + h) * 10).max(0x1e),
        };
        let bonus = if stance.special {
            SPECIAL_BONUS[index] * 30 / 100
        } else {
            0
        };
        let mut value = (base + bonus).min(STAT_CAP[index]);
        if index == STAT_SWITCH {
            return value.min(100);
        }
        if stance.switched {
            let switch = self.stat(STAT_SWITCH, stance);
            value += SWITCH_WEIGHT[index] * ((switch + 100) * value * 0x32 / 10000 - value) / 100;
        }
        value
    }

    /// Big drop limit in units from the landing stat scaled base.
    pub fn big_drop_limit(self, stance: impl Into<Stance>) -> i32 {
        BIG_DROP_BASE * (self.stat(STAT_LANDING, stance) * 0x46 + 10000) / 10000
    }

    /// Frames an ollie can charge, from the ollie and air stats.
    pub fn max_charge(self, stance: impl Into<Stance>) -> i32 {
        let stance = stance.into();
        0xf - (self.stat(STAT_OLLIE, stance) + self.stat(STAT_AIR, stance)) / 0x14
    }
}

/// Vertical ollie impulse in Q12 for a flat takeoff, negative meaning up.
pub fn ollie_impulse(ollie_stat: i32, air_stat: i32, charge: i32) -> i32 {
    impulse(ollie_stat, 0x21c, 5000, ollie_stat + air_stat, charge)
}

/// Pop off a face steeper than about 52 degrees, from the air stat alone.
pub fn steep_ollie_impulse(ollie_stat: i32, air_stat: i32, charge: i32) -> i32 {
    impulse(air_stat, 900, 3000, ollie_stat + air_stat, charge)
}

fn impulse(stat: i32, offset: i32, weight: i32, both: i32, charge: i32) -> i32 {
    let max_charge = 0xf - both / 0x14;
    let base = (-offset - stat) * weight / 10000 * 0x400 / 10;
    let held =
        ((stat + offset) * weight / 10000 - (stat * 0xce4 + 0x166e30) / 10000) * charge * 0x400
            / 10;
    (base - held / max_charge) * 3
}

/// Frame scale in Q8 with 256 meaning two 60 Hz ticks, plus the normal hold increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameScale {
    pub scale: i32,
    pub scale_squared: i32,
    pub hold_delta: i32,
    /// Run time in design frame halves, two per frame at 30 Hz.
    pub time: i32,
}

pub const DESIGN_FRAME: FrameScale = FrameScale {
    scale: 256,
    scale_squared: 256,
    hold_delta: 0x1000,
    time: 0,
};

/// Ring of the last three VB tick deltas that sets the frame scale at the end of each frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameClock {
    ring: [i32; 3],
    index: usize,
    last: i32,
    time_acc: i32,
    current: FrameScale,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self {
            ring: [2; 3],
            index: 0,
            last: 0,
            time_acc: 0,
            current: DESIGN_FRAME,
        }
    }
}

impl FrameClock {
    pub fn current(&self) -> FrameScale {
        self.current
    }

    /// Run start refills the ring with two ticks and leaves the index where it was.
    pub fn reset(&mut self, vb: i32) {
        self.ring = [2; 3];
        self.last = vb - 2;
        self.time_acc = 0;
        self.current.time = 0;
    }

    pub fn end_frame(&mut self, vb: i32) {
        let delta = vb - self.last;
        self.ring[self.index] = delta;
        self.index = (self.index + 1) % 3;
        self.last = vb;
        let sum = self.ring.iter().sum::<i32>();
        let scale = (if sum == 0 { 1 } else { sum.min(15) } << 8) / 6;
        self.time_acc += scale * 2;
        self.current = FrameScale {
            scale,
            scale_squared: (scale * scale) >> 8,
            hold_delta: delta * 0x800,
            time: self.time_acc >> 8,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ground,
    Air,
    /// Braked to a halt, the ground handler runs but the position is restored each frame.
    Stopped,
    /// Riding up or down a lip face past the coping, the ground handler keeps the skater on its plane.
    Vert,
    /// Just popped off a vert face: airborne, but the face can still catch the skater.
    VertPop,
    /// Riding a wall face under its own gravity.
    Wall,
    /// Sliding along a rail chain.
    Grind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skater {
    pub state: State,
    pub position: [i32; 3],
    pub old_position: [i32; 3],
    pub velocity: [i32; 3],
    pub lateral: [i32; 3],
    pub up: [i32; 3],
    pub forward: [i32; 3],
    pub last_normal: [i32; 3],
    pub smoothed_up: [i32; 3],
    pub same_normal_time: i32,
    pub stick_slack: i32,
    pub fast: bool,
    pub terrain: u32,
    pub switched: bool,
    /// Downward travel accumulated while airborne, Q12.
    pub fall: i32,
    /// Ground distance below the origin minus the standing height at frame start, Q12.
    pub clearance: i32,
    /// Turn rate in Q12 angle units per frame times 4096.
    pub turn: i32,
    pub lean: i32,
    pub anim: AnimState,
    pub crouch_lean: bool,
    pub crouching: bool,
    pub charge: i32,
    pub armed: bool,
    pub jumped: bool,
    /// Frame counter value when the current air time began, zero on the ground.
    pub air_since: i32,
    pub vert: bool,
    pub vert_jump: bool,
    pub landing_timer: i32,
    pub previous_state: State,
    pub now: i32,
    pub ollies: i32,
    pub short_ollies: i32,
    pub airs: i32,
    /// Gravity projected onto the last ground plane, kept through the air.
    pub slope_gravity: [i32; 3],
    pub ground_normal: [i32; 3],
    pub align_dir: i32,
    pub align_time: i32,
    pub turning: bool,
    /// Push acceleration for the current frame, consumed by the ground handler.
    pub accel: [i32; 3],
    /// Everything the frame adds to the velocity once the handler has moved the skater.
    pub frame_accel: [i32; 3],
    pub push_cooldown: i32,
    /// Speed in Q12 units per frame that pushes stop accelerating toward.
    pub push_target: i32,
    pub brake_mode: i32,
    /// Air pitch per frame from up or down, 4096 units per turn.
    pub pitch: i32,
    /// Frame time of the last air frame without a direction, held input waits eight units after it.
    pub quiet_since: i32,
    /// Turn within the current half turn on the ground, a lip or in the air, and the half turns done.
    pub spin_angle: i32,
    pub spins: i32,
    /// Frame time since up has been held alone, zero while it is not.
    pub up_since: i32,
    /// Angle between the velocity and the lip face summed while riding it, 1024 per quarter turn.
    pub lip_angle: i32,
    pub bailing: bool,
    /// A bail that may still rise keeps upward velocity.
    pub bail_lift: bool,
    /// Divisor of the per frame velocity drag while bailing, zero for none.
    pub bail_drag: i32,
    pub gravity_percent: i32,
    /// Air gravity for this frame, fixed from the state the frame starts in.
    pub gravity: i32,
    /// Offset of the next trick script opcode, none while no script runs.
    pub script: Option<usize>,
    pub panel: Panel,
    /// A combo is open: tricks and gaps push onto the panel until the landing commits them.
    pub combo: bool,
    pub gaps: Vec<PendingGap>,
    /// An ended gap that only counts once the skater grinds.
    pub grind_gap: Option<i16>,
    /// The last landing's alignment, and whether a wall bonk excused it.
    pub style: Style,
    pub unjudged: bool,
    /// Frames of landing push left, each one about a unit along the velocity.
    pub boost: i32,
    pub special: Special,
    pub session: Session,
    /// Pop time, its state 3 entry time and speed, and the heights the steep pop bonus compares.
    pub pop_time: i32,
    pub pop_entered: i32,
    pub pop_speed: i32,
    pub upright_y: i32,
    pub ground_y: i32,
    /// Set by a frame on a lip, the air then turns with the spin stat and skips the upright roll.
    pub lip_spin: bool,
    /// A trick or gap went on the stack since the last landing, so the landing feeds the meter.
    pub trick_landed: bool,
    /// Trigger checksums the last sweep touched, they fire again only after a sweep without them.
    pub touching: Vec<u32>,
    /// Run clock reading just after the run started, a held jump arms nothing until ten ticks
    /// later or a release. Zero when clear.
    pub jump_block: i32,
    /// Kills and visibility changes the frame's scripts made, applied to the world after the frame.
    pub world_events: Vec<WorldEvent>,
    pub text_events: Vec<TextEvent>,
    /// Gaps, landings and takeoffs of the frame, for the career run to count after it.
    pub career_events: Vec<CareerEvent>,
    /// S-K-A-T-E letters taken this run, in panel order.
    pub letters: [u8; 5],
    /// Light level of the ground under the skater, easing toward the doubled ground brightness.
    pub brightness: i32,
    pub brightness_target: i32,
    /// A bail script took the board off the skater, it comes back when the bail ends.
    pub board_hidden: bool,
    /// Where the frame's probe down met the ground and that face's normal, for the shadow.
    pub shadow_point: [i32; 3],
    pub shadow_normal: [i16; 3],
    /// The shadow found steep enough ground under the skater after the frame.
    pub shadow: bool,
    /// Frame counter value of the last grind frame, the air shadow waits a little after a rail.
    pub grind_time: i32,
    /// Height the shadow was last probed from, small moves on the ground only slide it along.
    pub shadow_from: i32,
    /// Pulses left per command point node, set by the first run of its script.
    pub pulses: Vec<(usize, u16)>,
    pub grind: Grind,
    pub rng: Rng,
    /// Position at the start of the last frame the ground handler ran.
    pub ground_start: [i32; 3],
    /// Name and points the trick script will push, and the script queued for leaving a rail.
    pub trick_name: String,
    pub trick_points: i32,
    pub exit_script: Option<usize>,
    /// Points the trick on top of the stack gains per thirty units of rail or per manual frame.
    pub points_rate: i32,
    /// A trick script is running a move, a rail start then bails unless the script ended it.
    pub trick_running: bool,
    pub script_ended: bool,
    pub script_return: Option<usize>,
    /// The frame time this frame runs at, for time driven animation.
    pub frame_time: i32,
    /// Frame time of the last air landing, and the switch stance bonus judged after it.
    pub landed_at: i32,
    pub switch_bonus: bool,
    /// Frames since the grind button was let go, and the frame time of the last wall the board left.
    pub grind_up: i32,
    pub wall_time: i32,
    /// Bail kinds the script sets for falling off a rail and for landing mid move.
    pub rail_bail: i16,
    pub landing_bail: i16,
    pub presses: Presses,
    pub air_queue: std::collections::VecDeque<QueuedTrick>,
    /// Manual kind, 1 on the tail and 2 on the nose, the move waiting to start one, whether it
    /// started from rolling, its friction and its balance.
    pub manual: u8,
    pub manual_pending: Option<usize>,
    pub from_ground: bool,
    pub manual_friction: i32,
    pub balance: Balance,
    /// Move the script queues for an ollie out of a manual or off a rail.
    pub ollie_script: Option<usize>,
    /// The running air trick's button and its press time, and how long its points have been held.
    pub trick_button: u8,
    pub trick_pressed: i32,
    pub hold_time: i32,
    pub frame: FrameScale,
    /// Position at the start of this frame and of the one before.
    pub frame_start: V3,
    pub last_start: V3,
}

/// The skater's rail state: the candidate the search found, the rail ridden and its balance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grind {
    pub found: Option<RailHit>,
    pub on_rail: bool,
    pub start: usize,
    pub segment: usize,
    pub kind: u8,
    /// Unit vector against the travel along the segment.
    pub direction: [i32; 3],
    /// Which way the board faces along the rail, one or minus one.
    pub board: i32,
    pub last_rail: Option<usize>,
    pub last_balanced: Option<usize>,
    /// Frames the last rail stays blocked, and frames no rail can be taken.
    pub block: i32,
    pub no_stick: i32,
    /// Grows the longer the same rail is ridden, the balance swings harder with it.
    pub heat: i32,
    pub angle: i32,
    pub rate: i32,
    pub latch: bool,
    pub counted_from: [i32; 3],
    pub side: u8,
    /// The rail approached from the outside of the last ground position, kept while on one rail.
    pub outer: bool,
    /// Set at the grind start from the ledge side, the travel sign and the approach. Trick script
    /// op 0x34 branches on it once.
    pub ledge_branch: bool,
}

/// A gap started by one command point, ended by the command point with the end checksum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingGap {
    pub end: u32,
    pub id: i16,
}

/// What the skater meets in a run: collision, its animation bank, trick scripts and level nodes.
#[derive(Clone, Copy)]
pub struct Level<'a> {
    pub world: &'a CollisionWorld,
    pub table: &'a AnimationTable,
    pub tricks: &'a TrickScripts,
    pub triggers: &'a Triggers,
    pub rails: &'a Rails,
    pub keys: &'a KeyTable,
    pub props: &'a PropModels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Grinding,
    Grounded(Hit),
    LeftGround(Option<Hit>),
    Airborne,
    Landed(Hit),
    /// The run has ended and the world stands still.
    Ended,
    /// The end menu picked Retry, the caller starts a new run.
    Retry,
}

type V3 = [i32; 3];

/// The executable scales by the single precision constants 2^-12 and 6.2831854820251465.
const TWO_PI_F32: f64 = 6.283_185_482_025_146_5;

pub(crate) fn trig_q12(angle: i32) -> (i32, i32) {
    let radians = f64::from(angle & 0xfff) * (1.0 / 4096.0) * TWO_PI_F32;
    (
        (radians.cos() * 4096.0) as i32,
        (radians.sin() * 4096.0) as i32,
    )
}

/// Native rotation about Y with inputs pre-shifted by three bits.
pub fn rotate_y(v: V3, angle: i32) -> V3 {
    let (cos, sin) = trig_q12(angle);
    [
        (cos * (v[0] >> 3) + sin * (v[2] >> 3)) >> 9,
        v[1],
        (cos * (v[2] >> 3) - sin * (v[0] >> 3)) >> 9,
    ]
}

pub fn cross_q12(a: V3, b: V3) -> V3 {
    [
        (a[1] * b[2] - a[2] * b[1]) >> 12,
        (a[2] * b[0] - a[0] * b[2]) >> 12,
        (a[0] * b[1] - a[1] * b[0]) >> 12,
    ]
}

pub fn project_onto_plane(v: V3, normal: V3) -> V3 {
    let d = dot_q12(v, normal);
    std::array::from_fn(|a| v[a] - mul_q12(d, normal[a]))
}

fn dot_checked(a: V3, b: V3) -> Result<i32, String> {
    let sum: i128 = (0..3).map(|i| i128::from(a[i]) * i128::from(b[i])).sum();
    i32::try_from(sum / 4096).map_err(|_| "ground dot product overflow".into())
}

fn speed_of(v: V3) -> Result<i32, String> {
    Ok(sqrt_int(dot_checked(v, v)?) * 0x40)
}

/// Projects onto the plane and rescales to the previous speed, failing on native overflow.
pub fn reproject(v: V3, normal: V3) -> Result<V3, String> {
    let speed = speed_of(v)?;
    let flat = project_onto_plane(v, normal);
    let new_speed = (sqrt_int(dot_q12(flat, flat)) << 6) >> 8;
    if new_speed <= 0 {
        return Ok(flat);
    }
    let mut out = [0; 3];
    for a in 0..3 {
        out[a] = flat[a]
            .checked_mul(speed >> 8)
            .ok_or("ground reproject overflow")?
            / new_speed;
    }
    Ok(out)
}

fn short(v: V3) -> V3 {
    v.map(|c| i32::from(c as i16))
}

/// Native 3x3 short matrix times vector with arithmetic shifts.
fn mat_vec(m: [i32; 9], v: V3) -> V3 {
    [
        (m[0] * v[0] + m[1] * v[1] + m[2] * v[2]) >> 12,
        (m[3] * v[0] + m[4] * v[1] + m[5] * v[2]) >> 12,
        (m[6] * v[0] + m[7] * v[1] + m[8] * v[2]) >> 12,
    ]
}

fn mat_mul_cols(a: [i32; 9], b: [i32; 9]) -> [i32; 9] {
    let mut out = [0; 9];
    for j in 0..3 {
        let r = short(mat_vec(a, [b[j], b[3 + j], b[6 + j]]));
        out[j] = r[0];
        out[3 + j] = r[1];
        out[6 + j] = r[2];
    }
    out
}

fn transpose(m: [i32; 9]) -> [i32; 9] {
    [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]
}

/// Identity right multiplied by the native X rotation builder.
pub fn rot_x_matrix(angle: i32) -> [i32; 9] {
    let (c, s) = trig_q12(angle);
    let identity = [0x1000, 0, 0, 0, 0x1000, 0, 0, 0, 0x1000];
    mat_mul_cols(identity, [0x1000, 0, 0, 0, c, -s, 0, s, c])
}

/// Identity right multiplied by the native Z rotation builder.
pub fn rot_z_matrix(angle: i32) -> [i32; 9] {
    let (c, s) = trig_q12(angle);
    let identity = [0x1000, 0, 0, 0, 0x1000, 0, 0, 0, 0x1000];
    mat_mul_cols(identity, [c, -s, 0, s, c, 0, 0, 0, 0x1000])
}

/// Identity right multiplied by the native Y rotation builder.
pub fn rot_y_matrix(angle: i32) -> [i32; 9] {
    let (c, s) = trig_q12(angle);
    let identity = [0x1000, 0, 0, 0, 0x1000, 0, 0, 0, 0x1000];
    mat_mul_cols(identity, [c, 0, s, 0, 0x1000, 0, -s, 0, c])
}

/// Lean frame approach in steps of one, three or five frames.
pub fn lerp_frame(current: i32, target: i32) -> i32 {
    let diff = target - current;
    let step = match diff.abs() {
        0 => return target,
        d if d > 12 => 5,
        d if d > 3 => 3,
        _ => 1,
    };
    current + diff.signum() * step
}

fn add(a: V3, b: V3) -> Result<V3, String> {
    let mut out = [0; 3];
    for axis in 0..3 {
        out[axis] = a[axis]
            .checked_add(b[axis])
            .ok_or("ground vector overflow")?;
    }
    Ok(out)
}

fn scaled(v: V3, k: i32) -> Result<V3, String> {
    let mut out = [0; 3];
    for axis in 0..3 {
        out[axis] = v[axis].checked_mul(k).ok_or("ground vector overflow")?;
    }
    Ok(out)
}

fn frame_part(v: V3, scale: i32) -> Result<V3, String> {
    Ok(scaled(v, scale)?.map(|c| c >> 8))
}

fn stat_speed(stat: i32, bonus: i32) -> i32 {
    (stat + bonus) * 0x2d000 / 0x118
}

pub fn clamp_speed(
    mut v: V3,
    rules: Rules,
    stance: impl Into<Stance>,
    frame: FrameScale,
) -> Result<V3, String> {
    let mut speed = speed_of(v)?;
    let speed_stat = rules.stat(STAT_SPEED, stance);
    let cap = stat_speed(speed_stat, 500);
    if speed > cap && speed & !0xfff != 0 {
        let shifted = speed >> 12;
        for c in &mut v {
            *c = (cap >> 12).checked_mul(*c).ok_or("ground clamp overflow")? / shifted;
        }
        speed = sqrt_int(dot_q12(v, v)) << 6;
    }
    if speed > stat_speed(speed_stat, 0x186) {
        for c in &mut v {
            *c -= frame_part([mul_q12(100, *c), 0, 0], frame.scale)?[0];
        }
    }
    if !rules.auto_kick {
        let magnitude = speed.abs();
        if magnitude < 0x10000 {
            v = v.map(|c| c - c / 32);
        }
        if magnitude < 0x2000 {
            v = v
                .map(|c| c - c / 4)
                .map(|c| if c.abs() < 0x10 { 0 } else { c });
        }
    }
    Ok(v)
}

impl Skater {
    /// Restart placement rotates the native rest basis about Y by ry minus half a turn.
    pub fn at_restart(position_fixed: V3, angles: [i16; 3]) -> Self {
        let angle = (i32::from(angles[1]) - 0x800) & 0xfff;
        Self {
            state: State::Ground,
            position: position_fixed,
            old_position: position_fixed,
            velocity: [0; 3],
            lateral: rotate_y([-4096, 0, 0], angle),
            up: WORLD_UP,
            forward: rotate_y([0, 0, -4096], angle),
            last_normal: WORLD_UP,
            smoothed_up: WORLD_UP,
            same_normal_time: 0,
            stick_slack: 0,
            fast: false,
            terrain: 0,
            switched: false,
            fall: 0,
            clearance: 0,
            turn: 0,
            lean: 0,
            anim: AnimState::default(),
            crouch_lean: false,
            crouching: false,
            charge: 0,
            armed: false,
            jumped: false,
            air_since: 0,
            vert: false,
            vert_jump: false,
            landing_timer: 0,
            previous_state: State::Ground,
            now: 0,
            ollies: 0,
            short_ollies: 0,
            airs: 0,
            slope_gravity: [0; 3],
            ground_normal: WORLD_UP,
            align_dir: 0,
            align_time: 0,
            turning: false,
            accel: [0; 3],
            frame_accel: [0; 3],
            push_cooldown: 0,
            push_target: PUSH_TARGET_INIT,
            brake_mode: 0,
            pitch: 0,
            quiet_since: 0,
            spin_angle: 0,
            spins: 0,
            up_since: 0,
            lip_angle: 0,
            bailing: false,
            bail_lift: false,
            bail_drag: 0,
            gravity_percent: 100,
            gravity: AIR_GRAVITY,
            script: None,
            panel: Panel::default(),
            combo: false,
            gaps: Vec::new(),
            grind_gap: None,
            style: Style::default(),
            unjudged: false,
            boost: 0,
            special: Special::default(),
            session: Session::default(),
            pop_time: 0,
            pop_entered: 0,
            pop_speed: 0,
            upright_y: 0,
            ground_y: 0,
            lip_spin: false,
            trick_landed: false,
            touching: Vec::new(),
            world_events: Vec::new(),
            text_events: Vec::new(),
            career_events: Vec::new(),
            letters: [0; 5],
            brightness: 0,
            brightness_target: 0,
            board_hidden: false,
            shadow_point: [0; 3],
            shadow_normal: [0; 3],
            shadow: false,
            grind_time: 0,
            shadow_from: 0,
            jump_block: 0,
            pulses: Vec::new(),
            grind: Grind::default(),
            rng: Rng::seeded(LEVEL_SEED),
            ground_start: [0; 3],
            trick_name: String::new(),
            trick_points: 0,
            exit_script: None,
            points_rate: 0,
            trick_running: false,
            script_ended: false,
            script_return: None,
            frame_time: 0,
            landed_at: 0,
            switch_bonus: false,
            grind_up: 0,
            wall_time: 0,
            rail_bail: 0,
            landing_bail: 0,
            presses: Presses::default(),
            air_queue: Default::default(),
            manual: 0,
            manual_pending: None,
            from_ground: false,
            manual_friction: 0,
            balance: Balance::default(),
            ollie_script: None,
            trick_button: 0,
            trick_pressed: 0,
            hold_time: 0,
            frame: DESIGN_FRAME,
            frame_start: position_fixed,
            last_start: position_fixed,
        }
    }

    pub fn stance(&self) -> Stance {
        Stance {
            switched: self.switched,
            special: self.special.full(),
        }
    }

    fn on_ground(&self) -> bool {
        matches!(self.state, State::Ground | State::Stopped)
    }

    /// Air counts on entering the air or a lip from anything but the other, and a lip entry points the
    /// stored slope gravity along the heading.
    fn set_state(&mut self, state: State) {
        match (state, self.state) {
            (new, old) if new == old => {}
            (State::Air, State::Vert) | (State::Vert, State::Air) => {}
            (State::Air | State::Vert, _) => self.airs += 1,
            _ => {}
        }
        if state == State::Air && self.state == State::Wall {
            self.leave_wall_basis();
        }
        if state == State::Vert && self.state != State::Vert {
            let [x, y, z] = self.forward.map(|c| mul_q12(c, AIR_GRAVITY));
            self.slope_gravity = [-x, y, -z];
        }
        self.previous_state = self.state;
        self.state = state;
    }

    fn run_anim(
        &mut self,
        table: &AnimationTable,
        id: u16,
        start: i32,
        end: i32,
        pingpong: i32,
    ) -> Result<(), String> {
        self.anim.run(table, id, start, end, pingpong as i8)
    }

    fn set_anim(&mut self, table: &AnimationTable, id: u16) -> Result<(), String> {
        self.run_anim(table, id, 0, -1, -1)
    }

    /// Leaving a ground state enters the air with the fall animation and arms the landing timer.
    fn leave_ground(&mut self, table: &AnimationTable) -> Result<(), String> {
        if self.state == State::Vert {
            self.set_state(State::Air);
            return Ok(());
        }
        self.set_state(State::Air);
        self.open_combo();
        self.landing_timer = LANDING_TIMER;
        (self.spin_angle, self.spins) = (0, 0);
        self.jumped = true;
        self.trick_running = false;
        if self.bailing {
            self.manual = 0;
            return Ok(());
        }
        (self.script, self.manual) = (None, 0);
        let id = if self.crouching {
            ANIM_CROUCH_FALL
        } else {
            ANIM_FALL
        };
        self.set_anim(table, id)
    }

    /// Ollie button handling: crouch while held, jump on release, and the landing animation bookkeeping.
    pub fn jump_input(
        &mut self,
        table: &AnimationTable,
        scripts: &TrickScripts,
        rules: Rules,
        held: bool,
    ) -> Result<(), String> {
        let anim = self.anim;
        if !self.crouching {
            if anim.id == ANIM_CROUCH {
                if anim.pingpong == -1 {
                    if anim.frame == 0 {
                        self.set_anim(table, ANIM_IDLE)?;
                    }
                } else {
                    self.run_anim(table, ANIM_CROUCH, CROUCH_PINGPONG, 0, -1)?;
                }
            }
        } else if anim.id == ANIM_IDLE {
            self.run_anim(table, ANIM_CROUCH, 0, CROUCH_END, CROUCH_PINGPONG)?;
        }
        if anim.finished {
            self.finish_anim(table, anim)?;
        }
        if self.jumped && self.previous_state != State::Ground && self.on_ground() {
            self.jumped = false;
            self.vert_jump = false;
            if self.manual == 0 {
                self.script = None;
                if self.crouching {
                    self.run_anim(table, ANIM_CROUCH_LAND, 3, -1, -1)?;
                } else {
                    self.set_anim(table, ANIM_LAND)?;
                }
                self.landed_trick(table, scripts)?;
            }
            if !self.bailing {
                self.boost += (self.panel.total(self.landed_style()) / 100).min(BOOST_MAX);
                if self.manual == 0 {
                    self.commit_combo()?;
                }
            }
            self.trick_running = false;
        }
        if !held {
            self.jump_block = 0;
            self.crouching = false;
            if self.air_since > 0 && self.now - self.air_since > LATE_OLLIE_FRAMES {
                self.armed = false;
            }
            let held_off = self.session.time_up
                && !matches!(self.state, State::Grind | State::Wall)
                && self.manual == 0;
            if self.armed && held_off {
                self.armed = false;
                self.set_anim(table, ANIM_IDLE)?;
            } else if self.armed {
                self.ollie(table, rules)?;
            }
            self.charge = 0;
            return Ok(());
        }
        self.charge = (self.charge + 1).min(rules.max_charge(self.stance()));
        if self.jump_block != 0 {
            return Ok(());
        }
        if matches!(self.state, State::Wall | State::Grind) {
            self.armed = true;
        } else if self.on_ground() {
            self.armed = true;
            if !self.crouching && self.turn.abs() < 0xa000 {
                self.turn = 0;
                if self.manual == 0 {
                    self.run_anim(table, ANIM_CROUCH, 0, CROUCH_END, CROUCH_PINGPONG)?;
                }
            }
        }
        self.crouching = true;
        Ok(())
    }

    /// Follow up of a finished animation by id, with the trick and stance ids left unsupported.
    fn finish_anim(&mut self, table: &AnimationTable, anim: AnimState) -> Result<(), String> {
        match anim.id {
            ANIM_OLLIE | ANIM_NOLLIE => {
                if !self.crouching {
                    self.set_anim(table, ANIM_AIR)?;
                }
            }
            ANIM_LAND | ANIM_CROUCH_LAND | ANIM_IDLE => {
                if !self.crouching {
                    self.set_anim(table, ANIM_IDLE)?;
                } else {
                    self.run_anim(
                        table,
                        ANIM_CROUCH,
                        CROUCH_PINGPONG,
                        CROUCH_END,
                        CROUCH_PINGPONG,
                    )?;
                }
            }
            ANIM_CROUCH => {
                if anim.frame == 0 {
                    self.set_anim(table, ANIM_IDLE)?;
                } else {
                    self.run_anim(
                        table,
                        ANIM_CROUCH,
                        CROUCH_PINGPONG,
                        CROUCH_END,
                        CROUCH_PINGPONG,
                    )?;
                }
            }
            ANIM_AIR => self.set_anim(table, ANIM_AIR)?,
            0x22
            | 0x30
            | 0x7b..=0x7e
            | 0x93..=0x95
            | 0x97
            | 0x98
            | 0xa8
            | 0xb5
            | 0x82
            | 0xa2
            | 0xa3 => {
                return Err(format!("finished animation id={} unsupported", anim.id));
            }
            _ => {
                if matches!(self.state, State::Air | State::Vert) {
                    self.set_anim(table, ANIM_AIR)?;
                }
            }
        }
        Ok(())
    }

    fn ollie(&mut self, table: &AnimationTable, rules: Rules) -> Result<(), String> {
        if self.state == State::Grind {
            return Err("ollie off a rail unsupported".into());
        }
        if self.manual != 0 {
            if self.ollie_script.is_some() {
                return Err("ollie out of a manual into a queued move unsupported".into());
            }
            (self.trick_running, self.script, self.manual_pending) = (false, None, None);
            self.balance.kick = 0;
        }
        self.armed = false;
        self.jumped = true;
        self.vert_jump = self.vert;
        self.pop_speed = speed_of(self.velocity)?;
        let wallie = self.state == State::Wall;
        if wallie {
            self.wallie(rules)?;
        }
        let (ollie, air) = (
            rules.stat(STAT_OLLIE, self.stance()),
            rules.stat(STAT_AIR, self.stance()),
        );
        let impulse = if self.up[1].abs() < 0x9c4 {
            self.climb_bonus(steep_ollie_impulse(ollie, air, self.charge))
        } else {
            ollie_impulse(ollie, air, self.charge)
        };
        self.velocity[1] = self.velocity[1]
            .checked_add(impulse)
            .ok_or("ollie impulse overflow")?;
        self.ollies += 1;
        if self.charge < rules.max_charge(self.stance()) {
            self.short_ollies += 1;
        }
        (self.spin_angle, self.spins) = (0, 0);
        match self.state {
            State::Vert => {}
            State::Ground if self.vert_jump => {
                self.set_state(State::VertPop);
                self.pop_entered = self.frame_time;
            }
            _ => self.set_state(State::Air),
        }
        self.pop_time = self.frame_time;
        if self.script.is_some() {
            return Ok(());
        }
        self.open_combo();
        self.landing_timer = LANDING_TIMER;
        let ups = if self.manual == 0 && !wallie {
            self.up_presses()
        } else {
            0
        };
        match ups {
            0 => self.set_anim(table, ANIM_OLLIE),
            1 => {
                self.push_ollie_trick("NO COMPLY", NO_COMPLY_POINTS);
                self.anim.speed = 0x10000;
                self.set_anim(table, ANIM_NO_COMPLY)
            }
            _ => {
                self.velocity[1] -= BONELESS_POP;
                self.push_ollie_trick("BONELESS", BONELESS_POINTS);
                self.anim.speed = 0x10000;
                self.run_anim(table, ANIM_BONELESS, 5, -1, -1)?;
                self.presses.consume(UP);
                self.presses.consume(UP);
                Ok(())
            }
        }
    }

    /// Popping off a wall scores, takes the full charge and kicks away from the face.
    fn wallie(&mut self, rules: Rules) -> Result<(), String> {
        self.push_entry(Entry {
            trick: true,
            switched: self.switched,
            ..Entry::new("WALLIE", WALLIE_POINTS, 0, 1)
        });
        self.charge = rules.max_charge(self.stance());
        self.accel = add(self.accel, self.ground_normal.map(|c| c * 2))?;
        self.velocity[1] -= WALLIE_POP;
        Ok(())
    }

    /// A fast pop high up a steep face goes up to 2.4 times as far.
    fn climb_bonus(&self, impulse: i32) -> i32 {
        let climb = (self.upright_y - self.ground_y) >> 12;
        if self.pop_speed < 0x28000 || climb <= 500 {
            return impulse;
        }
        (climb.min(700) - 300) * impulse / 200 * 120 / 100
    }

    /// Up presses just before the pop, the newest within 20 time units and one more within 35.
    fn up_presses(&self) -> usize {
        let p = &self.presses;
        let newest = p.newest();
        let (mut window, mut ups) = (UP_FIRST_WINDOW, 0);
        let mut at = p.search(newest, 0, window, newest, false, self.frame_time);
        while let Some(i) = at {
            if p.code(i) == UP {
                ups += 1;
                if ups == 2 {
                    break;
                }
                window = UP_SECOND_WINDOW;
            }
            at = p.search(i, 1, window, newest, false, self.frame_time);
        }
        ups
    }

    fn push_ollie_trick(&mut self, name: &str, points: i32) {
        self.push_entry(Entry {
            trick: true,
            switched: self.switched,
            ..Entry::new(name, points, 0, 0)
        });
        self.trick_landed = true;
    }

    /// Riding backwards turns the board round to fakie.
    fn fakie_flip(&mut self) -> bool {
        if self.anim.id == ANIM_NO_FAKIE || dot_q12(self.forward, self.velocity) <= FAKIE_DOT {
            return false;
        }
        self.forward = self.forward.map(|c| -c);
        self.lateral = self.lateral.map(|c| -c);
        self.switched = !self.switched;
        true
    }

    /// Landing across the travel faces the board along last frame's motion and bails to the side
    /// the motion leans.
    fn sideways_bail(&mut self, level: &Level, flat: V3, normal: V3) -> Result<(), String> {
        self.fakie_flip();
        let mut side = dot_q12(flat, self.lateral);
        let mut travel: V3 = std::array::from_fn(|a| self.frame_start[a] - self.last_start[a]);
        if travel.iter().map(|c| c.abs()).sum::<i32>() > 0x8000 {
            travel = travel.map(|c| c >> 4);
        }
        self.forward = normalize(travel).map(|c| -c);
        self.up = normal;
        self.lateral = cross_q12(short(self.forward), short(self.up));
        self.forward = cross_q12(short(self.up), short(self.lateral));
        if self.switched {
            side = -side;
        }
        let kind = if side < 1 { 2 } else { 0 };
        self.start_bail(level.table, level.tricks, kind, false)
    }

    pub fn basis_matrix(&self) -> [i32; 9] {
        let (l, u, f) = (short(self.lateral), short(self.up), short(self.forward));
        [l[0], u[0], f[0], l[1], u[1], f[1], l[2], u[2], f[2]]
    }

    fn set_basis(&mut self, m: [i32; 9]) {
        self.lateral = [m[0], m[3], m[6]];
        self.up = [m[1], m[4], m[7]];
        self.forward = [m[2], m[5], m[8]];
    }

    /// Right multiplies the basis by a Y rotation and turns the velocity through the old basis.
    fn rotate(&mut self, old_matrix: [i32; 9], angle: i32) -> Result<(), String> {
        if angle == 0 || self.bailing {
            return Ok(());
        }
        let rotation = rot_y_matrix(angle);
        self.set_basis(mat_mul_cols(self.basis_matrix(), rotation));
        let v = self.velocity;
        let speed = speed_of(v)?;
        let turned = mat_mul_cols(transpose(old_matrix), rotation);
        let p = mat_mul_cols(old_matrix, turned);
        let mut v = [
            dot_q12([p[0], p[1], p[2]], v),
            dot_q12([p[3], p[4], p[5]], v),
            dot_q12([p[6], p[7], p[8]], v),
        ];
        let new_speed = (sqrt_int(dot_q12(v, v)) << 6) >> 8;
        if new_speed > 0 {
            for c in &mut v {
                *c = c.checked_mul(speed >> 8).ok_or("ground turn overflow")? / new_speed;
            }
        }
        self.velocity = v;
        Ok(())
    }

    /// Turn rate ramp from the pad and the lean animation it drives.
    fn turn_input(
        &mut self,
        table: &AnimationTable,
        rules: Rules,
        pad: Pad,
        frame: FrameScale,
    ) -> Result<(), String> {
        let base = match rules.deck_turning {
            0 => 0x3c,
            1 => 0x78,
            _ => 0xb4,
        };
        let mut accel = (base * 0x100 * frame.scale) >> 8;
        let mut limit = TURN_MAX;
        self.crouch_lean = pad.down;
        self.turning = pad.left || pad.right;
        if self.crouch_lean {
            accel <<= 1;
            limit = TURN_MAX_CROUCH;
        }
        let mut turn = self.turn;
        if pad.left {
            turn = (turn - accel).max(-limit);
        } else if pad.right {
            turn = (turn + accel).min(limit);
        } else {
            turn -= turn >> if self.crouch_lean { 1 } else { 2 };
            if (turn + 0x800) & !0xfff == 0 {
                turn = 0;
            }
        }
        self.turn = turn;
        self.lean = turn;
        self.lean_anim(table, pad)
    }

    /// Air turn from a direction held past a short delay or from the spin buttons at once, then the
    /// pitch from up or down while neither flip nor grab is held.
    fn air_turn(
        &mut self,
        table: &AnimationTable,
        pad: Pad,
        frame: FrameScale,
    ) -> Result<(), String> {
        let rate = if pad.spin_left || pad.spin_right {
            0x64000
        } else {
            0xa000
        };
        let accel = (frame.scale * rate) >> 8;
        let (mut left, mut right) = (pad.left || pad.spin_left, pad.right || pad.spin_right);
        if !left && !right {
            self.quiet_since = frame.time;
        } else if self.quiet_since > 0 && frame.time - self.quiet_since < 8 {
            (left, right) = (false, false);
        }
        left |= pad.spin_left;
        right |= pad.spin_right;
        self.crouch_lean = false;
        self.turning = left || right;
        self.turn = if left {
            (self.turn - accel).max(-AIR_TURN_MAX)
        } else if right {
            (self.turn + accel).min(AIR_TURN_MAX)
        } else {
            0
        };
        self.lean = self.turn;
        self.lean_anim(table, pad)?;
        self.pitch = 0;
        if self.state == State::Air && !pad.kick && !pad.grab {
            let step = (frame.scale * 20) >> 8;
            if pad.up {
                self.pitch = -step;
            }
            if pad.down {
                self.pitch = step;
            }
            if self.facing_from_camera() {
                self.pitch = -self.pitch;
            }
        }
        Ok(())
    }

    /// The original flips the pitch when the forward axis points away from the camera. Until the
    /// camera is ported it is taken to trail the skater along the level velocity.
    fn facing_from_camera(&self) -> bool {
        let [x, _, z] = self.velocity;
        dot_q12(self.forward, [x, 0, z]) > 0
    }

    /// Pitch about the lateral axis keeping the point 70 units up the old up axis in place, skipped
    /// when it would turn the up axis below level.
    fn air_pitch(&mut self) {
        if self.pitch == 0 {
            return;
        }
        let m = self.basis_matrix();
        let turned = mat_mul_cols(m, rot_x_matrix(self.pitch));
        if turned[4] >= 1 {
            return;
        }
        let pivot = |m: [i32; 9]| [m[1] * 70, m[4] * 70, m[7] * 70];
        let (before, after) = (pivot(m), pivot(turned));
        self.set_basis(turned);
        self.position = std::array::from_fn(|a| self.position[a] + before[a] - after[a]);
    }

    /// Rolls a step toward upright when the up axis leans across the heading. The original reads its
    /// vector copies here, which still hold the basis from the start of the frame.
    fn upright_roll(&mut self, up: V3, forward: V3) {
        let lean = dot_q12(up, cross_q12(short(forward), WORLD_UP));
        let angle = match lean {
            l if l > 0x29 => 0xb,
            l if l < -0x29 => 0xff5,
            _ => return,
        };
        self.set_basis(mat_mul_cols(self.basis_matrix(), rot_z_matrix(angle)));
    }

    /// Spin rate from the turn rate, scaled by the spin stat and cut nine percent per half turn in
    /// the top trick's spin group, or per loose half turn with nothing stacked.
    fn spin_amount(&self, rules: Rules) -> i32 {
        let stat = rules.stat(STAT_SPIN, self.stance());
        let n = match self.panel.entries.len() {
            0 => self.spins,
            len => self.panel.spin_group(len - 1),
        };
        (stat + 300) * (self.turn >> 12) * 25 / 10000 * (100 - 9 * n) / 100
    }

    fn count_spin(&mut self, angle: i32) {
        self.spin_angle += angle;
        if self.spin_angle > 0x7ff {
            self.spin_angle -= 0x800;
            self.spins += 1;
            self.refresh_spins();
            self.text_events.push(TextEvent::Lines(self.shown()));
        }
        if self.spin_angle < -0x7ff {
            self.spin_angle += 0x800;
            self.spins += 1;
            self.refresh_spins();
            self.text_events.push(TextEvent::Lines(self.shown()));
        }
    }

    fn open_combo(&mut self) {
        if !self.combo {
            self.career_events.push(CareerEvent::TakeOff);
        }
        self.combo = true;
        (self.style, self.unjudged) = (Style::default(), false);
        self.text_events.push(TextEvent::Opened);
    }

    pub(crate) fn shown(&self) -> Shown {
        Shown {
            entries: self.panel.entries.clone(),
            style: self.landed_style(),
        }
    }

    /// Landing style counts only while half spins are in hand.
    pub(crate) fn landed_style(&self) -> Style {
        if self.spins == 0 {
            Style::default()
        } else {
            self.style
        }
    }

    /// Half spins so far, a turn past 1500 units rounding up to the next outside a manual.
    fn round_spins(&mut self) -> i32 {
        if self.manual == 0 && self.spin_angle.abs() > 1500 {
            self.spin_angle = 0;
            return self.spins + 1;
        }
        self.spins
    }

    /// The last gap entry takes the current half spins, on a lip only once there are two.
    fn refresh_spins(&mut self) {
        let lip = self.state == State::Vert || self.previous_state == State::Vert;
        let stacked = self.panel.spins();
        let spins = self.spins;
        if let Some(last) = self.panel.entries.last_mut()
            && last.kind == 0
            && last.spins != spins
            && (!lip || spins > 1 || stacked > 0)
        {
            last.spins = spins;
        }
    }

    /// A new entry after a gap freezes the gap's half spins, riding switch adds a fifth.
    fn push_entry(&mut self, mut entry: Entry) {
        if !self.combo {
            return;
        }
        if let Some(id) = entry.gap {
            self.career_events.push(CareerEvent::Gap(id));
        }
        if self.manual != 0 {
            self.balance.kick = manual::MANUAL_KICK;
            self.manual_pending = None;
            self.presses.clear();
        }
        if self.switch_bonus {
            entry.points = entry.points * 120 / 100;
        }
        let after_gap = self.panel.entries.last().is_some_and(|e| e.kind == 0);
        let kind = entry.kind;
        self.panel.push(entry);
        let n = self.panel.entries.len();
        if after_gap {
            self.panel.entries[n - 2].spins = self.round_spins();
            self.spins = 0;
        } else if kind == 1 {
            self.spins = self.round_spins();
            self.panel.entries[n - 1].spins = self.spins;
        }
        self.refresh_spins();
        self.text_events.push(TextEvent::Restart(self.shown()));
    }

    /// Landing rounds the half spins into the last gap and moves the combo total to the panel.
    fn commit_combo(&mut self) -> Result<(), String> {
        self.spins = self.round_spins();
        self.refresh_spins();
        let stacked = self.panel.spins();
        if stacked == 0 || (self.previous_state == State::Vert && stacked == 1) {
            self.style = Style::default();
        }
        self.text_events.push(TextEvent::Lines(self.shown()));
        let spin_ollie = self.spins != 0
            && self.panel.entries.last().is_none_or(|e| e.kind != 0)
            && (self.previous_state != State::Vert || self.spins > 1);
        if spin_ollie {
            self.push_entry(Entry {
                trick: true,
                switched: self.switched,
                ..Entry::new("OLLIE", SPIN_OLLIE_POINTS * self.spins, 0, 0)
            });
            self.trick_landed = true;
        }
        if !self.panel.entries.is_empty() {
            let names = self.panel.entries.iter().map(|e| e.name.clone()).collect();
            self.career_events.push(CareerEvent::Land(names));
        }
        self.panel.commit(self.landed_style());
        if self.trick_landed {
            self.special.land(self.panel.pending);
        }
        self.trick_landed = false;
        (self.spin_angle, self.spins, self.combo) = (0, 0, false);
        self.text_events.push(TextEvent::Committed);
        Ok(())
    }

    /// The landing push adds about one unit along the velocity per boost frame.
    fn apply_boost(&mut self) -> Result<(), String> {
        if self.boost == 0 {
            return Ok(());
        }
        let speed = speed_of(self.velocity)?;
        if speed & !0xfff > 0 {
            let k = speed >> 12;
            self.accel = std::array::from_fn(|a| self.accel[a] + self.velocity[a] / k);
        }
        self.boost -= 1;
        Ok(())
    }

    /// Pending gaps whose state mask covers the current state are lost.
    fn cancel_gaps(&mut self) -> Result<(), String> {
        let mask = if self.bailing {
            0xffff
        } else {
            match self.state {
                State::Ground if self.manual != 0 => 0x10,
                State::Ground => 1,
                State::Air | State::Vert => 0x20,
                State::Grind => 2,
                State::Wall => 4,
                State::Stopped | State::VertPop => 0,
            }
        };
        let mut kept = Vec::with_capacity(self.gaps.len());
        for g in &self.gaps {
            if hangar_gap(g.id)?.mask & mask == 0 {
                kept.push(*g);
            }
        }
        self.gaps = kept;
        if self.state == State::Ground {
            self.grind_gap = None;
        }
        if self.state == State::Grind
            && let Some(id) = self.grind_gap.take()
        {
            let gap = hangar_gap(id)?;
            self.push_entry(Entry {
                gap: Some(id),
                ..Entry::new(gap.name, i32::from(gap.points), 0, 2)
            });
        }
        Ok(())
    }

    /// Sweeps the frame's travel for trigger planes and runs the command points newly touched.
    pub fn sweep_triggers(&mut self, level: &Level) -> Result<(), String> {
        let delta: V3 = std::array::from_fn(|a| self.position[a] - self.old_position[a]);
        if delta == [0; 3] {
            return Ok(());
        }
        let back = normalize(delta).map(|c| c * 16);
        let start = std::array::from_fn(|a| self.old_position[a] - back[a]);
        let fired = level.world.crossed_triggers(start, self.position)?;
        for &checksum in &fired {
            if !self.touching.contains(&checksum) {
                let node = level.triggers.command_point(checksum).ok_or_else(|| {
                    format!("trigger checksum={checksum:#x} has no command point")
                })?;
                self.run_command_point(level.triggers, node, 0)?;
            }
        }
        self.touching = fired;
        Ok(())
    }

    /// Runs a command point script, pulses run the scripts of the command points it links to.
    pub(super) fn run_command_point(
        &mut self,
        triggers: &Triggers,
        node: usize,
        depth: u32,
    ) -> Result<(), String> {
        if depth > PULSE_DEPTH {
            return Err(format!("command point node={node} pulse chain too deep"));
        }
        let own = triggers.checksum(node).unwrap_or(0);
        for command in triggers.commands(node)? {
            match command {
                Command::Gap { end, id } => self.gap_command(own, end, id)?,
                Command::InitialPulses(n) => {
                    if !self.pulses.iter().any(|p| p.0 == node) {
                        self.pulses.push((node, n));
                    }
                }
                Command::SendPulse => {
                    let Some(slot) = self.pulses.iter_mut().find(|p| p.0 == node) else {
                        continue;
                    };
                    if slot.1 == 0 {
                        continue;
                    }
                    if slot.1 != UNLIMITED_PULSES {
                        slot.1 -= 1;
                    }
                    self.pulse_links(triggers, node, depth)?;
                }
                Command::Kill => {
                    for link in triggers.links(node) {
                        match triggers.kind(link) {
                            kind if ENV_ITEMS.contains(&kind) => {
                                let checksum = triggers.checksum(link).ok_or_else(|| {
                                    format!("command point node={node} kills node={link} without checksum")
                                })?;
                                self.world_events.push(WorldEvent::Kill(checksum));
                            }
                            MOVING_OBJECT | SIGNALLED_OBJECT => self
                                .world_events
                                .push(WorldEvent::Touch(link, self.velocity)),
                            kind @ (10 | 11 | 500 | 501) => {
                                return Err(format!(
                                    "command point node={node} kills node={link} kind={kind}"
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                Command::Signal => {
                    for link in triggers.links(node) {
                        if triggers.kind(link) == MOVING_OBJECT {
                            self.world_events.push(WorldEvent::Signal(link));
                        }
                    }
                    if let Some(link) = triggers
                        .links(node)
                        .into_iter()
                        .find(|&l| triggers.kind(l) == SIGNALLED_OBJECT)
                    {
                        return Err(format!("command point node={node} signals node={link}"));
                    }
                }
                Command::Regions(regions) => self.world_events.push(WorldEvent::Regions(regions)),
                Command::Visible(show) => {
                    for link in triggers.links(node) {
                        match triggers.kind(link) {
                            kind if VISIBLE_ITEMS.contains(&kind) => {
                                let checksum = triggers.checksum(link).ok_or_else(|| {
                                    format!("command point node={node} shows node={link} without checksum")
                                })?;
                                self.world_events.push(WorldEvent::Visible(checksum, show));
                            }
                            kind => {
                                return Err(format!(
                                    "command point node={node} shows node={link} kind={kind}"
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Pulses the nodes a node links to.
    fn pulse_links(&mut self, triggers: &Triggers, node: usize, depth: u32) -> Result<(), String> {
        for link in triggers.links(node) {
            self.pulse(triggers, node, link, depth)?;
        }
        Ok(())
    }

    /// A pulse runs a command point or creates the object a node describes.
    fn pulse(
        &mut self,
        triggers: &Triggers,
        from: usize,
        node: usize,
        depth: u32,
    ) -> Result<(), String> {
        match triggers.kind(node) {
            MOVING_OBJECT | SIGNALLED_OBJECT
                if triggers.object_type(node)? == objects::SCRIPT_OBJECT =>
            {
                self.world_events.push(WorldEvent::Create(node))
            }
            kind if CREATED_OBJECTS.contains(&kind)
                && triggers.object_type(node)? == objects::PROP =>
            {
                self.world_events.push(WorldEvent::Prop(node))
            }
            pickups::PICKUP_NODE => self.world_events.push(WorldEvent::Pickup(node)),
            kind if CREATED_OBJECTS.contains(&kind) => {}
            _ if triggers.is_command_point(node) => {
                self.run_command_point(triggers, node, depth + 1)?
            }
            kind => return Err(format!("node={from} pulses node={node} kind={kind}")),
        }
        Ok(())
    }

    /// Ends the gaps waiting for this command point, then starts the one it names.
    fn gap_command(&mut self, own: u32, end: u32, id: i16) -> Result<(), String> {
        if self.bailing {
            return Ok(());
        }
        let (ended, kept): (Vec<_>, Vec<_>) = self.gaps.iter().partition(|g| g.end == own);
        self.gaps = kept;
        for g in ended {
            let gap = hangar_gap(g.id)?;
            if gap.mask & 0x40 != 0 {
                return Err(format!("gap id={} finishing in a manual unsupported", g.id));
            }
            if gap.mask & 8 != 0 {
                self.grind_gap = Some(g.id);
                continue;
            }
            let rolling = self.state == State::Ground && self.manual == 0;
            if rolling {
                self.open_combo();
            }
            self.push_entry(Entry {
                gap: Some(g.id),
                ..Entry::new(gap.name, i32::from(gap.points), 0, 0)
            });
            self.trick_landed = true;
            if rolling {
                self.commit_combo()?;
            }
        }
        if end != 0 {
            hangar_gap(id)?;
            if self.gaps.len() == GAP_SLOTS {
                self.gaps.clear();
            }
            self.gaps.push(PendingGap { end, id });
        }
        Ok(())
    }

    fn air_spin(&mut self, rules: Rules, frame: FrameScale) -> Result<(), String> {
        let amount = self.spin_amount(rules);
        if amount == 0 {
            return Ok(());
        }
        let angle = (amount * frame.scale) >> 8;
        self.set_basis(mat_mul_cols(self.basis_matrix(), rot_y_matrix(angle)));
        self.count_spin(angle);
        Ok(())
    }

    /// Turn on the ground, on a lip and in the air after a lip ride: the lip modes take the spin
    /// stat and leave the velocity alone, the others turn the travel with the board.
    fn vert_turn(
        &mut self,
        rules: Rules,
        pad: Pad,
        frame: FrameScale,
        old_matrix: [i32; 9],
    ) -> Result<(), String> {
        let spin_mode = self.state == State::Vert || (self.state == State::Air && self.lip_spin);
        let mut turn_units = if spin_mode {
            self.spin_amount(rules)
        } else {
            self.turn >> 12
        };
        if self.landing_timer != 0 {
            if turn_units == 0 {
                turn_units = self.fall_line_alignment(rules, ALIGN_EXTRA)?;
            } else {
                let step = if pad.kick || pad.grab { 1 } else { 3 };
                self.landing_timer = (self.landing_timer - step).max(0);
            }
        }
        let angle = (frame.scale * turn_units) >> 8;
        if spin_mode {
            self.set_basis(mat_mul_cols(self.basis_matrix(), rot_y_matrix(angle)));
        } else {
            self.rotate(old_matrix, angle)?;
        }
        self.count_spin(angle);
        Ok(())
    }

    /// Air gravity for the frame from the state it starts in, cut by the hangtime stat on a lip.
    fn air_gravity(&self, rules: Rules) -> i32 {
        if self.state == State::Vert {
            (500 - rules.stat(STAT_HANGTIME, self.stance())) * AIR_GRAVITY * 20 / 10000
        } else {
            AIR_GRAVITY * self.gravity_percent / 100
        }
    }

    /// A bail drops into the air on its script, slow plain bails taking the second script.
    fn start_bail(
        &mut self,
        table: &AnimationTable,
        scripts: &TrickScripts,
        kind: usize,
        lift: bool,
    ) -> Result<(), String> {
        self.bail_lift = lift;
        self.gravity_percent = 100;
        (self.combo, self.grind_gap) = (false, None);
        (self.spins, self.spin_angle) = (0, 0);
        self.panel.entries.clear();
        self.text_events.push(TextEvent::Bailed);
        (self.special, self.trick_landed) = (Special::default(), false);
        self.bailing = true;
        self.set_state(State::Air);
        let kind = if kind == 0 && speed_of(self.velocity)? < BAIL_SLOW_SPEED {
            1
        } else {
            kind
        };
        self.start_script(scripts.bail_script(kind)?);
        self.run_script(table, scripts)?;
        self.trick_running = false;
        Ok(())
    }

    /// A new trick script with the default name and points and nothing queued for leaving a rail.
    fn start_script(&mut self, pc: usize) {
        self.trick_running = true;
        self.script_ended = false;
        self.script = Some(pc);
        self.exit_script = None;
        self.trick_name = "Trick".into();
        self.trick_points = DEFAULT_TRICK_POINTS;
        self.anim.speed = 0x10000;
        self.script_return = None;
        (self.rail_bail, self.landing_bail) = (0, 0);
        (self.manual_pending, self.ollie_script) = (None, None);
        self.hold_time = 0;
    }

    /// Landing tipped over: the board turns onto its back and the skater slides off along the old up.
    fn tilt_bail(&mut self, table: &AnimationTable, scripts: &TrickScripts) -> Result<(), String> {
        self.start_bail(table, scripts, BAIL_TILT, false)?;
        let up = self.up;
        self.up = self.forward;
        self.forward = up.map(|c| -c);
        self.velocity = [up[0] * 10, 0, up[2] * 10];
        Ok(())
    }

    fn end_bail(&mut self, table: &AnimationTable) -> Result<(), String> {
        if self.crouching {
            return Err("bail end while crouching unsupported".into());
        }
        self.bailing = false;
        self.bail_drag = 0;
        self.push_cooldown = BAIL_END_COOLDOWN;
        self.set_anim(table, ANIM_IDLE)?;
        self.velocity = [0; 3];
        Ok(())
    }

    /// Bailing kills the horizontal push, upward motion unless the bail lifts, and drags velocity,
    /// and mashing buttons speeds up the bail animation.
    fn bail_drag(&mut self, frame: FrameScale) {
        self.frame_accel[0] = 0;
        self.frame_accel[2] = 0;
        (self.jumped, self.armed) = (false, false);
        if (!self.bail_lift || self.state == State::Vert) && self.velocity[1] < 0 {
            self.velocity[1] = 0;
        }
        if self.bail_drag != 0 {
            let d = self.bail_drag;
            self.velocity = self.velocity.map(|c| c - (((c / d) * frame.scale) >> 8));
        }
        self.anim.speed = self.presses.edges_since(frame.time, MASH_SPAN) * 0x5555 / 3 + 0x10000;
    }

    /// A wall in the air pushes the board off with a quarter of its normal and at times a random kick.
    fn air_bounce(&mut self, world: &CollisionWorld, hit: &Hit) -> Result<(), String> {
        if self.grind.no_stick != 0 {
            return Err("air wall bounce while rails are held off unsupported".into());
        }
        let normal = hit.normal.map(i32::from);
        (self.last_normal, self.smoothed_up, self.same_normal_time) = (WORLD_UP, WORLD_UP, 0);
        self.smooth_basis();
        self.position = world.move_retrying(self.position, add(hit.position_fixed, normal)?)?;
        self.jumped = true;
        self.wall_response(normal, WALL_TURN)?;
        self.velocity = add(self.velocity, normal.map(|c| c / 4))?;
        if self.rng.next(3) == 0 {
            self.velocity[0] += self.rng.next(0x4000) - 0x2000;
            self.velocity[2] += self.rng.next(0x4000) - 0x2000;
        }
        Ok(())
    }

    /// Landing in the middle of a trick move bails.
    fn landed_trick(
        &mut self,
        table: &AnimationTable,
        scripts: &TrickScripts,
    ) -> Result<(), String> {
        if !self.bailing && self.trick_running && !self.script_ended {
            let kind = usize::try_from(self.landing_bail)
                .map_err(|_| format!("landing bail kind={} out of range", self.landing_bail))?;
            self.start_bail(table, scripts, kind, false)?;
            self.landing_bail = 0;
        }
        Ok(())
    }

    /// Moves the frame along the play direction, held between the start and end frames.
    fn step_frame(&mut self, n: i16) {
        let a = &mut self.anim;
        a.frame += i16::from(a.direction) * n;
        let end = i16::from(a.end);
        a.frame = if a.direction > 0 {
            a.frame.max(a.start).min(end)
        } else {
            a.frame.min(a.start).max(end)
        };
    }

    /// The script's trick enters the combo, a rail trick closing its spin group.
    fn push_script_trick(&mut self) {
        let kind = if self.state == State::Grind {
            1
        } else {
            self.open_combo();
            u8::from(self.manual != 0)
        };
        self.push_entry(Entry {
            trick: true,
            switched: self.switched,
            ..Entry::new(&self.trick_name, self.trick_points, 0, kind)
        });
        self.trick_landed = true;
    }

    /// Frame a script wait targets, counted back from the end frame when negative.
    fn script_frame(&self, arg: i16) -> i32 {
        let (end, last) = (i32::from(self.anim.end), i32::from(self.anim.count) - 1);
        match arg {
            SCRIPT_LAST_FRAME => end,
            a if a < 0 && self.anim.direction < 1 => (end - i32::from(a)).min(last),
            a if a < 0 => (end + i32::from(a)).max(0),
            a => i32::from(a).min(last),
        }
    }

    /// Runs the trick script until it waits on a frame, yields or ends.
    fn run_script(&mut self, table: &AnimationTable, scripts: &TrickScripts) -> Result<(), String> {
        let Some(mut pc) = self.script else {
            return Ok(());
        };
        let short = |at: usize| {
            scripts
                .short(at)
                .ok_or_else(|| format!("trick script pc={at:#x} truncated"))
        };
        loop {
            let at = pc;
            let op = scripts
                .byte(at)
                .ok_or_else(|| format!("trick script pc={at:#x} past the end"))?;
            pc += 1;
            match op {
                0x01 => {
                    self.anim.id = short(pc)? as u16;
                    self.anim.frame = 0;
                    pc += 2;
                }
                0x02 => self.set_anim(table, self.anim.id)?,
                0x07 => {
                    self.script = None;
                    self.script_ended = true;
                    self.trick_running = false;
                    return Ok(());
                }
                0x0d => {
                    let target = self.script_frame(short(pc)?);
                    let dir = i32::from(self.anim.direction);
                    if i32::from(self.anim.frame) * dir < target * dir && !self.anim.finished {
                        self.script = Some(at);
                        return Ok(());
                    }
                    pc += 2;
                }
                0x03 => self.anim.reverse(),
                0x09 => {
                    self.step_frame(short(pc)?);
                    pc += 2;
                }
                0x0a => {
                    self.trick_points = i32::from(short(pc)?);
                    pc += 2;
                }
                0x0b => {
                    let end = (pc..)
                        .find(|&i| scripts.byte(i).is_none_or(|b| b == 0))
                        .unwrap();
                    self.trick_name = (pc..end)
                        .map(|i| char::from(scripts.byte(i).unwrap_or(0)))
                        .collect();
                    pc = end + 1;
                }
                0x0c => self.push_script_trick(),
                0x18 => {
                    let to = self.script_frame(short(pc)?);
                    let time = self.frame_time as i16;
                    let a = &mut self.anim;
                    a.swing = [a.frame, to as i16, time];
                    a.mode = crate::anim::SWING_MODE;
                    pc += 2;
                }
                0x19 => {
                    self.exit_script = Some(scripts.offset(pc)?);
                    pc += 2;
                }
                0x3d => {
                    if self.script_return.replace(pc + 2).is_some() {
                        return Err(format!("trick script gosub pc={at:#x} nested"));
                    }
                    pc = scripts.offset(pc)?;
                }
                0x3e => {
                    pc = self
                        .script_return
                        .take()
                        .ok_or_else(|| format!("trick script return pc={at:#x} without gosub"))?;
                }
                0x3f => {
                    self.rail_bail = short(pc)?;
                    pc += 2;
                }
                0x40 => {
                    self.landing_bail = short(pc)?;
                    pc += 2;
                }
                0x41 if self.grind.side != 0 => pc = scripts.offset(pc)?,
                0x41 => pc += 2,
                0x1a if self.grind.on_rail => {
                    self.script = Some(at);
                    return Ok(());
                }
                0x1a => {}
                0x1c => self.anim.mode = crate::anim::RAIL_MODE,
                0x1d => {
                    self.anim.speed = (i32::from(short(pc)?) << 17) / 60;
                    pc += 2;
                }
                0x20 => self.trick_running = false,
                0x23 => {
                    self.points_rate = i32::from(short(pc)?);
                    pc += 2;
                }
                // grind sound variant
                0x29 => pc += 2,
                0x2a => {
                    self.ollie_script = Some(scripts.offset(pc)?);
                    pc += 2;
                }
                0x52 => {
                    self.manual = scripts
                        .byte(pc)
                        .ok_or("trick script manual kind truncated")?;
                    pc += 1;
                }
                0x47 if speed_of(self.velocity)? >> 12 > i32::from(short(pc)?) => {
                    pc = scripts.offset(pc + 2)?
                }
                0x47 => pc += 4,
                0x53 if self.from_ground => pc = scripts.offset(pc)?,
                0x53 => pc += 2,
                0x54 => {
                    self.manual_friction = i32::from(short(pc)?);
                    pc += 2;
                }
                // shows the balance meter
                0x59 => pc += 1,
                0x0f => {
                    for a in 0..3 {
                        self.velocity[a] = i32::from(short(pc + a * 2)?) << 12;
                    }
                    pc += 6;
                }
                0x15 => pc = scripts.offset(pc)?,
                0x16 => {
                    self.script = Some(pc);
                    return Ok(());
                }
                0x2e => {
                    self.bail_drag = i32::from(short(pc)?);
                    pc += 2;
                }
                0x2f if matches!(self.state, State::Air | State::Vert) => {
                    pc = scripts.offset(pc)?
                }
                0x2f => pc += 2,
                0x31 => self.end_bail(table)?,
                0x34 if std::mem::take(&mut self.grind.ledge_branch) => pc = scripts.offset(pc)?,
                0x34 => pc += 2,
                0x32 => self.board_hidden = false,
                0x33 => self.board_hidden = true,
                // rumble only changes what is felt
                0x46 => {}
                0x36 => pc += 2,
                0x37 => pc += 4,
                0x38 | 0x45 => pc += 6,
                0x39 => {
                    let branches = short(pc)?;
                    if branches <= 0 {
                        return Err(format!("trick script random pc={at:#x} bad branch count"));
                    }
                    let pick = self.rng.next(i32::from(branches)) as usize;
                    pc = scripts.offset(pc + 2 + pick * 2)?;
                }
                0x10 => {
                    let target = self.script_frame(short(pc)?);
                    let dir = i32::from(self.anim.direction);
                    if i32::from(self.anim.frame) * dir < target * dir
                        && !self.anim.finished
                        && self.trick_held()
                    {
                        self.script = Some(at);
                        return Ok(());
                    }
                    pc += 2;
                }
                0x12 if self.trick_held() => pc = scripts.offset(pc)?,
                0x12 => pc += 2,
                0x14 => self.anim.mode = 0,
                0x17 => {
                    let (base, growth) = (i32::from(short(pc)?), i32::from(short(pc + 2)?));
                    let held = self.hold_time >> 12;
                    let rate = held * held * growth / 1000 + ((self.frame.scale * base) >> 8);
                    self.add_held_points(rate);
                    pc += 4;
                }
                // a trick database value, how near its end a trick may take a rail and the nollie
                // flag, none of which this port reads
                0x2d | 0x4e | 0x51 => pc += 2,
                // clears the spin lock, which no ported op sets
                0x3c => {}
                // asks for a blood splat, which is only drawn
                0x30 => {}
                0x4b if dot_q12(self.forward, self.velocity) < -0x800 => pc = scripts.offset(pc)?,
                0x4b => pc += 2,
                0x4c if scripts.byte(pc) == Some(native_state(self.state)) => {
                    pc = scripts.offset(pc + 1)?
                }
                0x4c => pc += 3,
                0x44 => {
                    self.gravity_percent = i32::from(short(pc)?);
                    pc += 2;
                }
                _ => return Err(format!("trick script op={op:#x} pc={at:#x} unsupported")),
            }
        }
    }

    /// Up held without a direction starts a timer that decides between riding a lip and popping off it.
    fn track_up_hold(&mut self, pad: Pad, frame: FrameScale) {
        if !pad.up || pad.left || pad.right {
            self.up_since = 0;
        } else if self.up_since == 0 {
            self.up_since = frame.time;
        }
    }

    /// Lip contact from the ground starts the lip ride with the air animation and a fresh landing
    /// timer, or pops off the lip when up has been held alone and the face allows it.
    fn enter_lip(
        &mut self,
        table: &AnimationTable,
        frame: FrameScale,
        hit: &Hit,
    ) -> Result<(), String> {
        if self.state == State::Stopped {
            return Err("stopped lip contact unsupported".into());
        }
        let held = self.up_since != 0 && frame.time - self.up_since >= 5;
        if held && hit.face_flags & 0x80 == 0 {
            let quarter = speed_of(self.velocity)? / 4;
            let [nx, _, nz] = hit.normal.map(i32::from);
            self.velocity[0] -= mul_q12(nx, quarter);
            self.velocity[2] -= mul_q12(nz, quarter);
            self.velocity[1] /= 2;
            (self.spin_angle, self.spins) = (0, 0);
            self.set_state(State::Air);
            self.open_combo();
            self.landing_timer = LANDING_TIMER;
            self.vert = false;
            self.set_anim(table, ANIM_AIR)?;
            self.jumped = true;
            return Ok(());
        }
        if !self.jumped {
            self.set_anim(table, ANIM_AIR)?;
            self.jumped = true;
        }
        self.open_combo();
        self.landing_timer = LANDING_TIMER;
        (self.spin_angle, self.spins) = (0, 0);
        self.set_state(State::Vert);
        Ok(())
    }

    /// Accumulates the angle between the velocity and the lip face, leaving the lip once it passes a
    /// quarter turn back.
    fn ride_lip(&mut self, normal: V3) {
        let n = normalize(normal);
        let v = normalize(self.velocity);
        let angle = crate::camera::racos(dot_q12(v, n)) - 0x3ff;
        if angle.abs() < 3 || (angle >= 0) == (self.lip_angle >= 0) {
            self.lip_angle += angle;
        } else {
            self.lip_angle = angle;
        }
        if self.lip_angle < -0x400 {
            self.set_state(State::Air);
        }
    }

    fn pushable(&self) -> bool {
        !self.crouching && (self.anim.id == ANIM_IDLE || (5..=7).contains(&self.anim.id))
    }

    fn start_push(
        &mut self,
        table: &AnimationTable,
        rules: Rules,
        bonus: i32,
    ) -> Result<(), String> {
        self.push_cooldown = PUSH_COOLDOWN;
        self.push_target = stat_speed(rules.stat(STAT_SPEED, self.stance()), bonus);
        self.run_anim(table, ANIM_PUSH_START, 0, -1, -1)?;
        self.anim.speed = PUSH_SPEED;
        Ok(())
    }

    /// Push animations accelerate along the heading on frames 11 to 15, doubled while the kick is held.
    /// The run start push gives eight times the heading on frames 16 to 19.
    fn push_burst(&mut self, speed: i32, kick: bool) -> bool {
        let (frames, k) = match self.anim.id {
            ANIM_PUSH | ANIM_PUSH_ALT => (11..=15, if kick { 8 } else { 4 }),
            ANIM_PUSH_SPECIAL => (16..=19, 8),
            _ => return false,
        };
        if frames.contains(&self.anim.frame) && speed < self.push_target {
            self.accel = self.forward.map(|c| -c * k);
        }
        true
    }

    /// Kick button and auto kick: the push animation chain, the glide toward the target and the bursts.
    fn push(&mut self, table: &AnimationTable, rules: Rules, pad: Pad) -> Result<(), String> {
        if self.manual != 0 || self.brake_mode != 0 {
            return Ok(());
        }
        if self.anim.finished {
            match self.anim.id {
                ANIM_PUSH_START => {
                    self.set_anim(table, ANIM_PUSH)?;
                    self.anim.speed = PUSH_SPEED;
                }
                ANIM_PUSH | ANIM_PUSH_SPECIAL => self.set_anim(table, ANIM_IDLE)?,
                _ => {}
            }
        }
        if self.state != State::Ground {
            return Ok(());
        }
        if self.push_cooldown > 0 {
            self.push_cooldown -= 1;
        }
        let speed = speed_of(self.velocity)?;
        let uphill = dot_q12(
            self.slope_gravity,
            project_onto_plane(self.velocity, self.ground_normal),
        );
        if pad.kick || rules.auto_kick {
            if !(self.crouch_lean && self.turning) {
                if self.push_cooldown == 0 {
                    let slow = uphill < -0x4cc || speed <= self.push_target / 2;
                    if slow && !self.crouching && self.anim.id != ANIM_PUSH_SPECIAL {
                        if self.pushable() {
                            self.start_push(table, rules, 0xaa)?;
                        }
                    } else {
                        self.push_cooldown = PUSH_COOLDOWN;
                    }
                }
                if !self.push_burst(speed, pad.kick)
                    && (speed > 20000 || self.forward[1] < 500)
                    && speed < self.push_target
                {
                    self.accel = self.forward.map(|c| -c);
                }
            }
            if rules.auto_kick {
                return Ok(());
            }
        }
        if pad.kick && uphill > -0x4cd && self.pushable() {
            self.start_push(table, rules, 0xdc)?;
        }
        if matches!(self.anim.id, ANIM_PUSH | ANIM_PUSH_ALT) {
            self.push_burst(speed, pad.kick);
        }
        Ok(())
    }

    /// Down alone brakes: a sixteenth of the velocity per frame, then a stop under the threshold.
    fn brake(&mut self, table: &AnimationTable, pad: Pad, frame: FrameScale) -> Result<(), String> {
        if self.script.is_some() || self.manual != 0 || !self.on_ground() {
            if self.state == State::Stopped {
                self.set_state(State::Ground);
            }
            self.brake_mode = 0;
            return Ok(());
        }
        let braking = pad.down && !pad.left && !pad.right;
        if self.state == State::Stopped && !braking {
            self.set_state(State::Ground);
        }
        let threshold = brake_threshold(self.ground_normal);
        let speed = speed_of(self.velocity)?;
        if self.state == State::Ground && braking {
            if threshold < speed {
                self.velocity = self.velocity.map(|c| c - (((c >> 4) * frame.scale) >> 8));
            } else if threshold <= BRAKE_STOP {
                self.set_state(State::Stopped);
                self.velocity = [0; 3];
            }
        }
        let anim = self.anim;
        let brake_anim = !matches!(anim.id, 0x5d | 0x5f);
        match self.brake_mode {
            0 => {
                if braking {
                    if speed <= threshold {
                        self.brake_mode = 0;
                    } else if !self.crouching && speed < 0x30000 {
                        if brake_anim {
                            self.set_anim(table, ANIM_BRAKE)?;
                        }
                        self.brake_mode = 5;
                    } else {
                        if brake_anim {
                            self.set_anim(table, ANIM_BRAKE_FAST)?;
                        }
                        self.brake_mode = 1;
                    }
                    self.push_cooldown = 2;
                }
            }
            1 | 5 => {
                if !braking || speed < (i32::from(anim.frame) << 12) + threshold {
                    self.anim.reverse();
                    self.brake_mode = if self.brake_mode == 1 { 3 } else { 6 };
                }
                self.push_cooldown = 2;
            }
            3 => {
                if anim.finished {
                    if self.crouching {
                        self.run_anim(
                            table,
                            ANIM_CROUCH,
                            CROUCH_PINGPONG,
                            CROUCH_END,
                            CROUCH_PINGPONG,
                        )?;
                    } else {
                        self.run_anim(table, ANIM_CROUCH, CROUCH_PINGPONG, 0, -1)?;
                    }
                    self.brake_mode = 4;
                }
                self.push_cooldown = 2;
            }
            6 => {
                if anim.finished {
                    self.set_anim(table, ANIM_IDLE)?;
                    self.brake_mode = 4;
                }
                self.push_cooldown = 2;
            }
            4 => {
                if !braking {
                    self.brake_mode = 0;
                }
            }
            mode => return Err(format!("brake mode={mode} unsupported")),
        }
        Ok(())
    }

    /// Finished wall hit animations return to idle or the crouch.
    fn wall_hit_recovery(&mut self, table: &AnimationTable) -> Result<(), String> {
        if !(self.anim.finished && matches!(self.anim.id, 0xb | ANIM_WALL_HIT)) {
            return Ok(());
        }
        if self.crouching {
            self.run_anim(
                table,
                ANIM_CROUCH,
                CROUCH_PINGPONG,
                CROUCH_END,
                CROUCH_PINGPONG,
            )
        } else {
            self.set_anim(table, ANIM_IDLE)
        }
    }

    /// The push target decays by one per frame and resets to the kick target while crouching.
    fn decay_push_target(&mut self, rules: Rules) {
        let stat = rules.stat(STAT_SPEED, self.stance());
        if self.crouching {
            self.push_target = stat_speed(stat, 0xdc);
        } else if stat_speed(stat, 0xaa) < self.push_target {
            self.push_target -= 1;
        }
    }

    /// Standing leans 6 and 7, crouch leans 9 and 10, and the return to idle or crouch when the lean ends.
    fn lean_anim(&mut self, table: &AnimationTable, pad: Pad) -> Result<(), String> {
        let limit = if pad.down { TURN_MAX_CROUCH } else { TURN_MAX };
        let lean = self.lean;
        let lean_id = |id: u16| id == 0 || (6..=10).contains(&id);
        let mut anim = self.anim.id;
        let mut frame = i32::from(self.anim.frame);
        if lean != 0 && !self.crouching && lean_id(anim) {
            let target = (lean.abs() * LEAN_FRAMES / limit).min(LEAN_FRAMES);
            let (id, bump) = match (self.switched, lean >= 0) {
                (false, true) | (true, false) => (7, false),
                (false, false) => (6, false),
                (true, true) => (6, true),
            };
            if anim != id {
                frame = 0;
            }
            if bump && frame < target {
                frame += 1;
            }
            frame = lerp_frame(frame, target);
            self.run_anim(table, id, frame, frame, -1)?;
            anim = id;
        }
        if lean != 0 && self.crouching && lean_id(anim) {
            let left = lean < 0;
            let (id, from, frames) = if left != self.switched {
                (9, 6, CROUCH_LEAN_LEFT_FRAMES)
            } else {
                (10, 7, CROUCH_LEAN_RIGHT_FRAMES)
            };
            let target = (lean.abs() * frames / limit).min(frames);
            if anim == from {
                anim = id;
                frame = target;
            }
            if anim != id {
                frame = 0;
            }
            frame = lerp_frame(frame, target);
            self.run_anim(table, id, frame, frame, -1)?;
        }
        if lean == 0 {
            match self.anim.id {
                6 | 7 => self.set_anim(table, ANIM_IDLE)?,
                9 | 10 => self.run_anim(
                    table,
                    ANIM_CROUCH,
                    CROUCH_PINGPONG,
                    CROUCH_END,
                    CROUCH_PINGPONG,
                )?,
                _ => {}
            }
        }
        Ok(())
    }

    /// Fall line alignment strength from the stored slope gravity and the speed below the stat limit.
    fn alignment_strength(&self, rules: Rules, extra: i32) -> Result<i32, String> {
        let g = self.slope_gravity;
        let magnitude = sqrt_int(dot_q12(g, g));
        let limit = stat_speed(rules.stat(STAT_SPEED, self.stance()), 0x186);
        let speed = speed_of(self.velocity)?.min(limit);
        let pull = magnitude * 0xc80 / 13000 * magnitude * 0x40 / 13000;
        Ok((limit + extra * 0x1000 - speed) * pull / limit)
    }

    /// Turn units toward the fall line of the last ground plane, latched per side for ten frames.
    fn fall_line_alignment(&mut self, rules: Rules, extra: i32) -> Result<i32, String> {
        let strength = self.alignment_strength(rules, extra)?;
        if strength == 0 {
            return Ok(0);
        }
        if self.align_time + 10 < self.now {
            self.align_dir = 0;
        }
        self.align_time = self.now;
        let down = normalize(self.slope_gravity);
        let across = cross_q12(short(self.ground_normal), short(down));
        let lean = dot_q12(self.forward, across);
        if lean < 0x2a {
            if lean < -0x29 {
                if self.align_dir != 1 {
                    self.align_dir = 2;
                    return Ok(strength);
                }
            } else {
                self.align_dir = 0;
            }
        } else if self.align_dir != 2 {
            self.align_dir = 1;
            return Ok(-strength);
        }
        Ok(0)
    }

    /// Head on hits run the wall hit animation and respond with the larger turn.
    fn wall_contact(
        &mut self,
        table: &AnimationTable,
        normal: V3,
        needs_speed: bool,
    ) -> Result<(), String> {
        if self.head_on(normal, needs_speed)? {
            self.set_anim(table, ANIM_WALL_HIT)?;
            self.unjudged = true;
            return self.wall_response(normal, WALL_HIT_TURN);
        }
        self.wall_response(normal, WALL_TURN)
    }

    /// Removes the into wall velocity, pushes off and snaps the heading almost parallel to the wall.
    fn wall_response(&mut self, normal: V3, turn: i32) -> Result<(), String> {
        let d = dot_q12(self.velocity, normal);
        if d < 0 {
            self.velocity = std::array::from_fn(|a| {
                self.velocity[a] - mul_q12(normal[a], d) + mul_q12(normal[a], WALL_PUSH)
            });
        }
        let away = normal.map(|c| -c);
        let f = dot_q12(self.forward, away);
        if f >= 0 {
            return Ok(());
        }
        let sine = sqrt_int(0x1000000 - f * f);
        let side = dot_q12(self.lateral, away);
        let signed = if side > 0 { -sine } else { sine };
        let snap = [f, 0, -signed, 0, 0x1000, 0, signed, 0, f].map(|c| i32::from(c as i16));
        let mut m = mat_mul_cols(self.basis_matrix(), snap);
        let angle = if side > 0 { turn - 0x400 } else { 0x400 - turn };
        m = mat_mul_cols(m, rot_y_matrix(angle));
        self.set_basis(m);
        let forward = normalize(self.forward);
        let up = normalize(self.up);
        let lateral = normalize(cross_q12(short(forward), short(up)));
        self.forward = short(cross_q12(short(up), short(lateral)));
        self.lateral = short(lateral);
        self.up = short(up);
        Ok(())
    }

    /// Heading within 45 degrees of the wall normal, with a speed condition on some paths.
    fn head_on(&self, normal: V3, needs_speed: bool) -> Result<bool, String> {
        if self.anim.id == ANIM_WALL_HIT {
            return Ok(false);
        }
        if needs_speed {
            let units = self.velocity.map(|c| c >> 12);
            let squares = units.iter().map(|c| c * c).sum::<i32>();
            if sqrt_int(squares) <= 6 {
                return Ok(false);
            }
        }
        Ok(dot_q12(normal, self.forward) > HEAD_ON_DOT)
    }

    fn settle_on(
        &mut self,
        world: &CollisionWorld,
        hit: &Hit,
        normal: V3,
        keep_hold: bool,
        frame: FrameScale,
    ) -> Result<(), String> {
        let target = add(hit.position_fixed, scaled(normal, STANDING_HEIGHT)?)?;
        self.position = world.move_retrying(self.position, target)?;
        if keep_hold && normal == self.last_normal {
            self.same_normal_time += frame.hold_delta;
        } else {
            self.same_normal_time = 0;
            self.last_normal = normal;
        }
        self.smooth_basis();
        Ok(())
    }

    /// Movement segment contact in the ground state: walls, perpendicular faces and steps.
    fn wall_branch(
        &mut self,
        world: &CollisionWorld,
        table: &AnimationTable,
        hit: &Hit,
        length: i32,
        frame: FrameScale,
    ) -> Result<(), String> {
        let normal = hit.normal.map(i32::from);
        let lip = hit.flags & 0x400000 != 0;
        let wall_face = hit.flags & 0x1800000 == 0x1800000;
        if lip && hit.face_flags & 0x80 == 0 {
            return Ok(());
        }
        if wall_face && !lip {
            self.position = self.old_position;
            return self.wall_contact(table, normal, false);
        }
        let d = dot_q12(normal, self.last_normal);
        if (-0x7ff..=0x7ff).contains(&d) {
            let [nx, ny, nz] = normal;
            if -nx.abs() == ny || -ny < nx.abs() || -ny <= nz.abs() {
                self.position = self.old_position;
                return self.wall_contact(table, normal, true);
            }
            return self.settle_on(world, hit, normal, false, frame);
        }
        if hit.distance_units <= length >> 12 {
            return self.settle_on(world, hit, normal, true, frame);
        }
        Ok(())
    }

    /// Lateral shoulder probe that pushes the skater off a near vertical face.
    fn bounce(
        &mut self,
        world: &CollisionWorld,
        table: &AnimationTable,
        direction: i32,
    ) -> Result<bool, String> {
        let reach = if self.bailing {
            BOUNCE_REACH_BAILING
        } else {
            BOUNCE_REACH
        };
        let shoulder = add(self.position, scaled(self.up, STANDING_HEIGHT)?)?;
        let end = add(shoulder, scaled(self.lateral, direction * reach)?)?;
        let Some(hit) = world.query_filtered(shoulder, end, false)? else {
            return Ok(false);
        };
        let normal = hit.normal.map(i32::from);
        if !(-0x3ff..=0x3ff).contains(&dot_q12(normal, self.up)) {
            return Ok(false);
        }
        self.position = self.old_position;
        self.wall_contact(table, normal, true)?;
        let away_start = add(self.position, normal)?;
        let away_end = add(self.position, scaled(normal, reach)?)?;
        if world.query_filtered(away_start, away_end, false)?.is_none() {
            self.position = away_end;
            return Ok(true);
        }
        Ok(false)
    }

    /// Lean wall probe from a hundred units above the origin along the lean side.
    fn lean_probe(&mut self, world: &CollisionWorld) -> Result<(), String> {
        let frames = match self.anim.id {
            6 | 7 => LEAN_FRAMES,
            9 | 10 => CROUCH_LEAN_RIGHT_FRAMES,
            _ => return Ok(()),
        };
        if self.lean == 0 {
            return Ok(());
        }
        let span = if self.lean >= 0 { frames } else { -frames };
        let reach = i32::from(self.anim.frame) * LEAN_PROBE_REACH / span;
        let start = add(self.position, scaled(self.up, LEAN_PROBE_HEIGHT)?)?;
        let end = add(start, scaled(self.lateral, -reach)?)?;
        let Some(hit) = world.query(start, end)? else {
            return Ok(());
        };
        self.anim.frame = (hit.distance_units * span / LEAN_PROBE_REACH).unsigned_abs() as i16;
        if hit.distance_units < 0x3c {
            self.position = add(self.position, scaled(hit.normal.map(i32::from), 2)?)?;
        }
        Ok(())
    }

    fn smooth_basis(&mut self) {
        if self.same_normal_time > NORMAL_HOLD_LIMIT {
            return;
        }
        let target = if self.same_normal_time == NORMAL_HOLD_LIMIT {
            self.last_normal
        } else {
            std::array::from_fn(|a| {
                ((self.last_normal[a] - self.smoothed_up[a]) >> 2) + self.smoothed_up[a]
            })
        };
        let up = normalize(target);
        let lateral = normalize(cross_q12(short(self.forward), short(up)));
        self.forward = short(cross_q12(short(up), short(lateral)));
        self.lateral = short(lateral);
        self.up = short(up);
        self.smoothed_up = up;
    }

    /// Frame setup shared by both states, probing the ground distance below the origin.
    fn begin_frame(&mut self, world: &CollisionWorld) -> Result<(), String> {
        self.stick_slack = (self.stick_slack - 5).max(0);
        self.old_position = self.position;
        let below = add(self.position, [0, UNSTICK_SPAN * 4096, 0])?;
        let Some(hit) = world.query_filtered(self.position, below, false)? else {
            return Err(
                "ground unstick fallback unsupported, nothing within 8000 units below".into(),
            );
        };
        self.clearance = hit.position_fixed[1] - self.position[1] - STANDING_HEIGHT * 4096;
        self.follow_light(world.shade(&hit)?);
        if !self.bailing {
            self.board_hidden = false;
        }
        Ok(())
    }

    /// The ground under the skater for the shadow, kept only on ground steep enough for the state:
    /// a flat floor everywhere, a little less flat in the air unless the slope just changed a lot.
    fn probe_shadow(&mut self, world: &CollisionWorld) -> Result<(), String> {
        if self.state == State::Ground && (self.position[1] - self.shadow_from).abs() <= 0x40 {
            (self.shadow_point[0], self.shadow_point[2]) = (self.position[0], self.position[2]);
            return Ok(());
        }
        self.shadow_from = self.position[1];
        let below = add(self.position, [0, SHADOW_REACH * 4096, 0])?;
        let hit = world.query_filtered(self.position, below, false)?;
        let threshold = |ny: i16| match self.state {
            State::Air if (ny - self.shadow_normal[1]).abs() <= 1000 => -1500,
            State::Air => -4096,
            State::Vert => -500,
            _ => -1000,
        };
        let recent_rail = self.state == State::Air && self.now - self.grind_time <= 6;
        self.shadow = false;
        if let Some(hit) = hit
            && i32::from(hit.normal[1]) < threshold(hit.normal[1])
            && self.state != State::Grind
            && !recent_rail
        {
            (self.shadow_point, self.shadow_normal) = (hit.position_fixed, hit.normal);
            self.shadow = true;
        }
        Ok(())
    }

    /// The light eases forty steps a frame toward twice the ground brightness, kept up on a vert.
    fn follow_light(&mut self, shade: i32) {
        let mut target = shade * 2;
        if self.state == State::Vert {
            target = target.max(0x100);
        }
        self.brightness_target = target;
        self.brightness = if self.brightness < target {
            (self.brightness + 0x28).min(target)
        } else {
            (self.brightness - 0x28).max(target)
        };
    }

    fn finish_frame(
        &mut self,
        world: &CollisionWorld,
        accel: V3,
        free_move: bool,
    ) -> Result<(), String> {
        if !free_move {
            self.position = world.move_retrying(self.old_position, self.position)?;
        }
        let descent = self.position[1] - self.old_position[1];
        if self.state == State::Air && !self.lip_spin && descent > 0 {
            self.fall += descent;
            self.clearance -= descent;
        } else {
            self.fall = 0;
        }
        self.frame_accel = accel;
        Ok(())
    }

    /// The frame's accel reaches the velocity only after the rail check and the trigger sweep.
    fn integrate(&mut self, rules: Rules, frame: FrameScale) -> Result<(), String> {
        self.velocity = add(self.velocity, frame_part(self.frame_accel, frame.scale)?)?;
        self.velocity = clamp_speed(self.velocity, rules, self.stance(), frame)?;
        Ok(())
    }
}

/// Probes the movement segment, extended to the wall reach when short, with lips included.
fn wall_probe(
    world: &CollisionWorld,
    old: V3,
    new: V3,
    bailing: bool,
) -> Result<(Option<Hit>, i32), String> {
    if old == new {
        return Ok((None, 0));
    }
    let reach = if bailing {
        WALL_REACH_BAILING
    } else {
        WALL_REACH
    };
    let delta = std::array::from_fn(|a| new[a] - old[a]);
    let length = speed_of(delta)?;
    let mut end = new;
    if length < reach * 4096 {
        let divisor = length.max(0x800);
        end = std::array::from_fn(|a| reach * delta[a] / divisor * 4096 + old[a]);
    }
    Ok((world.query_filtered(old, end, false)?, length))
}

fn head_probe(world: &CollisionWorld, s: &Skater) -> Result<(), String> {
    if (s.up[0] != 0 || s.up[2] != 0)
        && let Some(hit) = world.query(s.position, add(s.position, scaled(s.up, HEAD_REACH)?)?)?
    {
        return Err(format!(
            "ground head contact unsupported item={} face={}",
            hit.item_index, hit.face_index
        ));
    }
    Ok(())
}

/// Both shoulder bounces, restoring the entry position when both sides push back.
fn bounce_probes(
    world: &CollisionWorld,
    table: &AnimationTable,
    s: &mut Skater,
) -> Result<bool, String> {
    let entry = s.position;
    if s.bounce(world, table, -1)? {
        if s.bounce(world, table, 1)? {
            s.position = entry;
        }
        return Ok(true);
    }
    if !s.bounce(world, table, 1)? {
        return Ok(false);
    }
    if s.bounce(world, table, -1)? {
        s.position = entry;
    }
    Ok(true)
}

/// Shoulder bounces, the upside down check of the rolling state and the head probe.
fn post_contact_probes(
    world: &CollisionWorld,
    table: &AnimationTable,
    s: &mut Skater,
) -> Result<bool, String> {
    let bounced = bounce_probes(world, table, s)?;
    if s.state == State::Ground && s.up[1] > 1000 {
        return Err("ground upside down bail unsupported".into());
    }
    head_probe(world, s)?;
    Ok(bounced)
}

/// Speed under which braking stops the skater, raised on steep ground from the stored ground normal.
pub fn brake_threshold(ground_normal: V3) -> i32 {
    let steep = (-ground_normal[1]).max(0x300);
    ((0x1000 - steep) * 0x1800 / 0xd0).max(BRAKE_STOP)
}

/// The ground handler after the input phase, without bail, manual, triggers or moving objects.
pub fn ground_step(
    level: &Level,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    let (world, table) = (level.world, level.table);
    s.ground_start = s.position;
    let stopped = s.state == State::Stopped;
    let vert = s.state == State::Vert;
    if s.state == State::Ground {
        s.lip_angle = 0;
    }
    let old_matrix = s.basis_matrix();
    s.begin_frame(world)?;
    let mut accel = s.accel;
    let half_push = frame_part(accel, frame.scale_squared)?.map(|c| c / 2);
    let travel = add(frame_part(s.velocity, frame.scale)?, half_push)?;
    s.position = add(s.position, travel)?;
    if !vert
        && !s.bailing
        && s.fakie_flip()
        && s.state == State::Ground
        && frame.time - s.landed_at > (5 * frame.scale) >> 8
    {
        s.manual_fakie(table)?;
    }
    s.vert_turn(rules, pad, frame, old_matrix)?;
    let (contact, length) = wall_probe(world, s.old_position, s.position, s.bailing)?;
    if let Some(contact) = contact {
        s.wall_branch(world, table, &contact, length, frame)?;
    }
    s.fast = speed_of(s.velocity)? > stat_speed(rules.stat(STAT_SPEED, s.stance()), 0x186);
    let ignore_lips = s.state == State::Ground && (s.velocity[1] > 0 || s.forward[1] < 1);
    let start = add(s.position, scaled(s.up, 0x46)?)?;
    let end = add(start, scaled(s.up, -0x100)?)?;
    let hit = world.query_filtered(start, end, ignore_lips)?;
    let mut grounded = false;
    let mut free_move = false;
    if let Some(hit) = hit {
        let flags = hit.flags;
        let (lip, bit23, bit24) = (
            flags & 0x400000 != 0,
            flags & 0x800000 != 0,
            flags & 0x1000000 != 0,
        );
        s.terrain = (flags >> 25) & 0xf;
        if !(bit23 && bit24 && !lip) {
            s.vert = flags & VERT_FACE != 0;
        }
        let normal = hit.normal.map(i32::from);
        let mut reach = if vert {
            s.stick_slack = 20;
            0xa0
        } else {
            s.stick_slack + 0x74
        };
        if hit.distance_units > reach
            && pad.up
            && (1..0xed8).contains(&dot_q12(s.last_normal, normal))
        {
            reach *= 2;
        }
        if !(bit23 && bit24 && !lip) && hit.distance_units <= reach {
            s.position = world.stand(s.position, s.up, &hit)?;
            if lip {
                if vert {
                    s.ride_lip(normal);
                } else {
                    s.enter_lip(table, frame, &hit)?;
                }
                grounded = s.state == State::Vert;
            } else if bit24 {
                grounded = false;
            } else if vert {
                landing_bails(level, s, rules, normal)?;
                s.set_state(State::Ground);
                (s.manual_pending, s.landed_at) = (None, s.frame_time);
                s.presses.clear();
                grounded = true;
            } else {
                let convex_edge = dot_q12(s.forward, normal) < 0
                    && s.velocity[1] <= 0
                    && (1..3000).contains(&dot_q12(s.last_normal, normal));
                if stopped {
                    s.velocity = [0; 3];
                }
                grounded = stopped || !convex_edge;
            }
            if grounded {
                if normal == s.last_normal {
                    s.same_normal_time += frame.hold_delta;
                } else {
                    s.same_normal_time = 0;
                    s.last_normal = normal;
                }
                s.smooth_basis();
            }
        }
    }
    if grounded && s.state == State::Vert {
        let normal = hit.unwrap().normal.map(i32::from);
        accel = add(accel, project_onto_plane([0, s.gravity, 0], normal))?;
        s.ground_normal = normal;
        s.velocity = reproject(s.velocity, normal)?;
    } else if grounded {
        let hit = hit.unwrap();
        let normal = hit.normal.map(i32::from);
        s.slope_gravity = project_onto_plane([0, GRAVITY, 0], normal);
        s.ground_normal = normal;
        accel = add(accel, s.slope_gravity)?;
        let mut v = reproject(s.velocity, normal)?;
        if stopped {
            s.position = s.old_position;
        } else if !s.bailing {
            let sideways = dot_q12(v, s.lateral);
            v = std::array::from_fn(|a| v[a] - mul_q12(sideways, s.lateral[a]));
            let along = dot_q12(v, s.forward);
            let rolling: V3 = std::array::from_fn(|a| mul_q12(along, s.forward[a]));
            let mut frictions = Vec::new();
            if s.fast {
                frictions.push(0);
            } else if rules.auto_kick {
                frictions.push(8);
            }
            if s.terrain == 5 || s.terrain == 6 {
                frictions.push(0x78);
            }
            if s.terrain == 3 {
                frictions.push(0xb4);
            }
            if s.manual != 0 {
                frictions.push(s.manual_friction);
            }
            for k in frictions {
                let drag = frame_part(rolling.map(|c| mul_q12(k, c)), frame.scale)?;
                accel = std::array::from_fn(|a| accel[a] - drag[a]);
            }
        }
        s.velocity = v;
    } else {
        accel = add(accel, [0, GRAVITY, 0])?;
    }
    if grounded && !vert {
        free_move = post_contact_probes(world, table, s)?;
        s.lean_probe(world)?;
    }
    if !grounded && s.state != State::Air {
        s.leave_ground(table)?;
    }
    s.finish_frame(world, accel, free_move)?;
    Ok(if grounded {
        Outcome::Grounded(hit.unwrap())
    } else {
        Outcome::LeftGround(hit)
    })
}

fn land(level: &Level, s: &mut Skater, rules: Rules, pad: Pad, hit: &Hit) -> Result<(), String> {
    let (world, table, scripts) = (level.world, level.table, level.tricks);
    let normal = hit.normal.map(i32::from);
    let target = add(hit.position_fixed, scaled(normal, STANDING_HEIGHT)?)?;
    s.position = world.move_retrying(s.position, target)?;
    // the landing check runs with the no stick timer set, it only searches and counts down
    s.grind.no_stick = 1;
    s.try_grind(level, rules, pad)?;
    s.landed_at = s.frame_time;
    s.terrain = (hit.flags >> 25) & 0xf;
    s.turn = 0;
    s.set_state(State::Ground);
    s.vert = hit.flags & VERT_FACE != 0;
    if !s.bailing {
        landing_bails(level, s, rules, normal)?;
        let tilt = dot_q12(s.up, normal);
        let tipped = tilt < 2000 && s.up[1] > -2000 && (s.velocity[1] > 150000 || tilt < 200);
        if tipped && !s.bailing {
            return s.tilt_bail(table, scripts);
        }
        s.landed_trick(table, scripts)?;
        s.start_manual(level, rules, false)?;
    }
    if s.state != State::Air {
        s.velocity = project_onto_plane(s.velocity, normal);
        s.last_normal = normal;
        s.smoothed_up = normal;
        s.same_normal_time = 0;
        s.smooth_basis();
    }
    head_probe(world, s)
}

/// Fast landings are judged by how the board lines up with the travel, sideways ones and drops past
/// the landing stat limit bail.
fn landing_bails(level: &Level, s: &mut Skater, rules: Rules, normal: V3) -> Result<(), String> {
    s.style = Style::default();
    let flat = project_onto_plane(s.velocity, normal);
    if speed_of(flat)? > 0x1e000 {
        let heading = normalize(flat);
        let facing = normalize(project_onto_plane(s.forward, normal));
        let line = dot_q12(heading, facing).abs();
        if !s.unjudged {
            s.style = Style {
                perfect: line > 0xfc1,
                sloppy: line < 0xb50,
            };
        }
        if line < 0x666 {
            return s.sideways_bail(level, flat, normal);
        }
    }
    let height = (s.clearance + s.fall) >> 12;
    let base = rules.big_drop_limit(s.stance());
    let mut limit = base;
    if s.charge != 0 {
        limit += (0xb - s.charge) * 0x3c;
    }
    if height > limit {
        return Err(format!(
            "big drop bail unsupported height={height} limit={limit}"
        ));
    }
    if height > base && s.combo {
        return Err(format!(
            "big drop entry unsupported height={height} base={base}"
        ));
    }
    Ok(())
}

/// The air handler after the input phase, stopping at wall bounces, lips, wallrides and bails.
pub fn air_step(
    level: &Level,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    let (world, table) = (level.world, level.table);
    s.begin_frame(world)?;
    let (start_up, start_forward) = (s.up, s.forward);
    let mut accel = s.accel;
    let half_push = frame_part(s.accel, frame.scale_squared)?.map(|c| c / 2);
    let travel = add(frame_part(s.velocity, frame.scale)?, half_push)?;
    s.position = add(s.position, travel)?;
    s.air_pitch();
    if s.lip_spin {
        s.vert_turn(rules, pad, frame, s.basis_matrix())?;
    } else {
        s.air_spin(rules, frame)?;
        s.upright_roll(start_up, start_forward);
    }
    let popped = s.state == State::VertPop;
    let mut hit = world.query_filtered(s.old_position, s.position, false)?;
    if let Some(found) = hit
        && found.flags & LIP_FACE != 0
        && found.face_flags & 0x80 == 0
        && !popped
    {
        hit = world.query_filtered(s.old_position, s.position, true)?;
    }
    let skims = popped && s.frame_time - s.pop_time <= POP_CATCH;
    let rides = hit.as_ref().is_some_and(|h| {
        let wall =
            h.flags & 0x1000000 != 0 || (h.flags & LIP_FACE != 0 && h.face_flags & 0x80 != 0);
        let reach = pad.grind || s.grind_up <= GRIND_UP_FRAMES;
        wall && h.flags & 0x800000 == 0
            && !s.bailing
            && reach
            && s.frame_time - s.wall_time >= WALLRIDE_GAP
    });
    match hit {
        Some(hit) if hit.flags & 0x1000000 != 0 && !(popped && hit.flags & LIP_FACE != 0) => {
            if rides {
                s.enter_wall(level, &hit)?;
                s.finish_frame(world, accel, false)?;
                return Ok(Outcome::Airborne);
            }
            s.air_bounce(world, &hit)?;
        }
        Some(hit) if rides => {
            s.enter_wall(level, &hit)?;
            s.finish_frame(world, accel, false)?;
            return Ok(Outcome::Airborne);
        }
        Some(hit) if hit.flags & LIP_FACE != 0 => land_on_lip(world, table, s, pad, &hit)?,
        Some(hit) if skims => skim_face(world, s, &hit)?,
        Some(hit) => land(level, s, rules, pad, &hit)?,
        None => accel = add(accel, [0, s.gravity, 0])?,
    }
    if popped && s.state == State::VertPop && s.velocity[1] < 0 && skims && !s.bailing {
        let end = add(s.position, scaled(s.up, -0x80)?)?;
        if let Some(face) = world.query_filtered(s.position, end, false)?
            && face.flags & LIP_FACE != 0
        {
            lip_contact(table, s, pad, &face)?;
        }
    }
    s.finish_frame(world, accel, false)?;
    Ok(match hit {
        Some(hit) if !matches!(s.state, State::Air | State::VertPop) => Outcome::Landed(hit),
        _ => Outcome::Airborne,
    })
}

/// Touching a plain face just after a vert pop slides along it without landing.
fn skim_face(world: &CollisionWorld, s: &mut Skater, hit: &Hit) -> Result<(), String> {
    let normal = hit.normal.map(i32::from);
    let target = add(hit.position_fixed, scaled(normal, STANDING_HEIGHT)?)?;
    s.position = world.move_retrying(s.position, target)?;
    s.turn = 0;
    s.vert = hit.flags & VERT_FACE != 0;
    s.velocity = project_onto_plane(s.velocity, normal);
    s.last_normal = normal;
    s.smoothed_up = normal;
    s.same_normal_time = 0;
    s.smooth_basis();
    Ok(())
}

/// A lip hit from the air goes back onto the lip after a jump from a vert face, otherwise the skater
/// stays airborne against it for the frame without gravity.
fn land_on_lip(
    world: &CollisionWorld,
    table: &AnimationTable,
    s: &mut Skater,
    pad: Pad,
    hit: &Hit,
) -> Result<(), String> {
    let normal = hit.normal.map(i32::from);
    let target = add(hit.position_fixed, scaled(normal, STANDING_HEIGHT)?)?;
    s.position = world.move_retrying(s.position, target)?;
    if lip_contact(table, s, pad, hit)? {
        s.velocity = project_onto_plane(s.velocity, normal);
        s.last_normal = normal;
        s.same_normal_time = 0;
        s.smooth_basis();
    }
    Ok(())
}

/// After a jump from a vert face the lip takes the skater back unless up has been held alone
/// for a while, otherwise the skater stays airborne against it.
fn lip_contact(
    table: &AnimationTable,
    s: &mut Skater,
    pad: Pad,
    hit: &Hit,
) -> Result<bool, String> {
    let held = s.up_since != 0 && s.frame_time - s.up_since > 4;
    if s.vert_jump && (!held || hit.face_flags & 0x80 != 0) {
        s.set_state(State::Vert);
        return Ok(true);
    }
    s.set_state(State::Air);
    if pad.up {
        return Err(format!(
            "lip pop unsupported item={} face={}",
            hit.item_index, hit.face_index
        ));
    }
    s.vert = false;
    s.landing_timer = LANDING_TIMER;
    s.set_anim(table, ANIM_AIR)?;
    s.jumped = true;
    Ok(false)
}

/// The game's state number.
pub fn native_state(state: State) -> u8 {
    match state {
        State::Ground => 0,
        State::Air => 1,
        State::Vert => 2,
        State::VertPop => 3,
        State::Grind => 4,
        State::Wall => 5,
        State::Stopped => 7,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// Shatters the env item with this model checksum.
    Kill(u32),
    Regions(Regions),
    Visible(u32, bool),
    /// A pulse created the script object of this node.
    Create(usize),
    Prop(usize),
    /// A kill reached the objects of this node, with the skater's velocity when it was sent.
    Touch(usize, V3),
    Signal(usize),
    /// A pulse reached the powerup of this node.
    Pickup(usize),
}

/// One single session run: the skater, the frame clock and the unpaused VB counter.
#[derive(Clone, Debug)]
pub struct Run {
    pub skater: Skater,
    pub clock: FrameClock,
    pub vb: i32,
    /// VB ticks including the end screens, which keep running after the run's own clock stops.
    pub ticks: i32,
    /// The level world once a script has broken or hidden something in it.
    pub world: Option<CollisionWorld>,
    /// Script objects, the newest first like the original's object list.
    pub objects: Vec<ScriptObject>,
    /// Placed object models, the newest first like their draw list.
    pub props: Vec<Prop>,
    /// Powerups, the newest first like their list.
    pub pickups: Vec<Pickup>,
    pub career: Option<CareerRun>,
}

impl Run {
    /// Each run starts on the special push animation and a refilled clock ring, and the restart
    /// script pulses the command points the restart links to.
    pub fn start(level: &Level, restart: &Restart) -> Result<Self, String> {
        Self::start_with(level, restart, None)
    }

    /// A run with a career record: its pickups stay until taken.
    pub fn start_with(
        level: &Level,
        restart: &Restart,
        career: Option<CareerRun>,
    ) -> Result<Self, String> {
        let mut skater = Skater::at_restart(restart.position_fixed, restart.angles);
        skater.anim.run(level.table, ANIM_PUSH_SPECIAL, 0, -1, -1)?;
        skater.jump_block = 1;
        skater.pulse_links(level.triggers, restart.node_index, 0)?;
        let mut clock = FrameClock::default();
        clock.reset(0);
        let mut run = Self {
            skater,
            clock,
            vb: 0,
            ticks: 0,
            world: None,
            objects: Vec::new(),
            props: Vec::new(),
            pickups: Vec::new(),
            career,
        };
        run.apply_world_events(level)?;
        // the level start seeds the generator again after the powerups drew their bob speeds
        run.skater.rng = Rng::seeded(LEVEL_SEED);
        Ok(run)
    }

    pub fn frame(&mut self, level: &Level, rules: Rules, pad: Pad) -> Result<Outcome, String> {
        self.ticks += 2;
        self.skater.text_events.clear();
        if self.skater.session.ended {
            let retry = self
                .skater
                .session
                .end_screens(pad, self.skater.panel.score)?;
            return Ok(if retry {
                Outcome::Retry
            } else {
                Outcome::Ended
            });
        }
        self.skater.wind_down(level.table, pad)?;
        // the end menu pauses the game before the skaters update
        if self.skater.session.ended {
            return Ok(Outcome::Ended);
        }
        let pad = self.skater.time_up_pad(pad);
        self.vb += 2;
        let live = Level {
            world: self.world.as_ref().unwrap_or(level.world),
            ..*level
        };
        let frame = self.clock.current();
        let outcome = step(&live, &mut self.skater, rules, frame, pad)?;
        self.skater.probe_shadow(live.world)?;
        self.take_pickup()?;
        self.clock.end_frame(self.vb);
        self.apply_world_events(level)?;
        let at = self.skater.position;
        self.pickups.retain_mut(|p| p.update(at, frame.scale));
        // props draw their random numbers before the script objects
        let live = Level {
            world: self.world.as_ref().unwrap_or(level.world),
            ..*level
        };
        for prop in &mut self.props {
            prop.update(&mut self.skater, &live, self.vb)?;
        }
        for object in &mut self.objects {
            object.update(&mut self.skater, level.triggers, frame.hold_delta)?;
        }
        self.apply_world_events(level)?;
        let events = std::mem::take(&mut self.skater.career_events);
        if let Some(career) = &mut self.career {
            for event in events {
                career.apply(event)?;
            }
            career.check_score(self.skater.panel.score)?;
        }
        Ok(outcome)
    }

    /// The powerup the skater's travel this frame passes nearest, if it waits to be taken.
    fn take_pickup(&mut self) -> Result<(), String> {
        let s = &self.skater;
        let Some(i) = pickups::swept(s.old_position, s.position, &self.pickups) else {
            return Ok(());
        };
        if !self.pickups[i].active() {
            return Ok(());
        }
        let pickup = self.pickups.remove(i);
        match &mut self.career {
            Some(career) => self.skater.take_pickup(career, &pickup),
            None => Err(format!(
                "pickup node={} taken outside a career",
                pickup.node
            )),
        }
    }

    fn apply_world_events(&mut self, level: &Level) -> Result<(), String> {
        for event in std::mem::take(&mut self.skater.world_events) {
            let world = self.world.get_or_insert_with(|| level.world.clone());
            match event {
                WorldEvent::Kill(checksum) => {
                    if let Some(panes) = world.shatter(checksum) {
                        shatter_draws(&panes, &mut self.skater.rng);
                    }
                }
                WorldEvent::Regions(r) => {
                    for b in r.boxes {
                        world.show_items(b, r.inside, r.show);
                        for prop in &mut self.props {
                            if crate::collision::in_region(prop.position, b, r.inside) {
                                prop.hidden = !r.show;
                            }
                        }
                    }
                }
                WorldEvent::Visible(checksum, show) => world.show_item(checksum, show),
                WorldEvent::Create(node) => self.objects.insert(
                    0,
                    ScriptObject::new(node, level.triggers.object(node)?.script),
                ),
                WorldEvent::Prop(node) => self
                    .props
                    .insert(0, Prop::new(node, &level.triggers.object(node)?)),
                WorldEvent::Touch(node, push) => {
                    if self.objects.iter().any(|o| o.node == node) {
                        return Err(format!(
                            "kill or signal of script object node={node} unsupported"
                        ));
                    }
                    if !level.triggers.object(node)?.links.is_empty() {
                        return Err(format!("kill of prop node={node} with links unsupported"));
                    }
                    for prop in self.props.iter_mut().filter(|p| p.node == node) {
                        prop.knock(&mut self.skater, push)?;
                    }
                }
                WorldEvent::Pickup(node) => {
                    let career = self.career.as_ref();
                    let p = Pickup::new(node, level.triggers, &mut self.skater.rng, career)?;
                    self.pickups.insert(0, p);
                }
                WorldEvent::Signal(node) => {
                    if self.objects.iter().any(|o| o.node == node) {
                        return Err(format!("signal of script object node={node} unsupported"));
                    }
                    for prop in self.props.iter_mut().filter(|p| p.node == node) {
                        prop.signal();
                    }
                }
            }
        }
        Ok(())
    }
}

/// One frame: animation tick, pad input, ollie handling, the state handler and the airborne timer.
pub fn step(
    level: &Level,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    let table = level.table;
    s.panel.count_up();
    s.gravity = s.air_gravity(rules);
    s.special.drain(frame.hold_delta);
    s.track_up_hold(pad, frame);
    s.now += 1;
    s.frame_time = frame.time;
    if s.jump_block != 0 && frame.time - s.jump_block > JUMP_BLOCK_TICKS {
        s.jump_block = 0;
    }
    s.frame = frame;
    s.grind_up = if pad.grind { 0 } else { s.grind_up + 1 };
    if !matches!(s.state, State::Air | State::Vert)
        && frame.time - s.landed_at > (frame.scale * 15) >> 8
    {
        s.switch_bonus = s.switched;
    }
    s.anim.tick_at(frame.scale, frame.time)?;
    if s.on_ground() {
        s.turn_input(table, rules, pad, frame)?;
    } else if s.state != State::Grind {
        s.air_turn(table, pad, frame)?;
    }
    s.accel = [0; 3];
    s.push(table, rules, pad)?;
    s.check_time();
    if !s.bailing {
        s.jump_input(table, level.tricks, rules, pad.jump)?;
    }
    s.trick_input(level, rules, pad, frame.time)?;
    if !s.bailing {
        s.brake(table, pad, frame)?;
    }
    s.wall_hit_recovery(table)?;
    s.decay_push_target(rules);
    s.apply_boost()?;
    s.cancel_gaps()?;
    (s.last_start, s.frame_start) = (s.frame_start, s.position);
    let dispatched = s.state;
    if dispatched == State::Ground && frame.time - s.landed_at > (20 * frame.scale) >> 8 {
        s.start_manual(level, rules, true)?;
    }
    match s.state {
        State::Ground | State::Grind => s.lip_spin = false,
        State::Wall => (s.lip_spin, s.grind.last_rail) = (false, None),
        State::Vert => s.lip_spin = true,
        _ => {}
    }
    let outcome = match s.state {
        State::Ground | State::Stopped | State::Vert => ground_step(level, s, rules, frame, pad),
        State::Air => air_step(level, s, rules, frame, pad),
        State::Wall => wall::wall_step(level, s, rules, frame),
        State::VertPop => {
            let outcome = air_step(level, s, rules, frame, pad)?;
            if s.state == State::VertPop && s.frame_time - s.pop_entered > POP_AIR {
                s.set_state(State::Air);
            }
            Ok(outcome)
        }
        State::Grind => {
            s.old_position = s.position;
            s.grind.heat += frame.hold_delta;
            s.grind_frame(level, rules, frame, pad)?;
            Ok(Outcome::Grinding)
        }
    }?;
    if dispatched == State::Ground {
        s.balance_frame(level, rules, pad, frame)?;
    }
    if s.bailing {
        s.bail_drag(frame);
    }
    s.settle_manual();
    s.try_grind(level, rules, pad)?;
    s.sweep_triggers(level)?;
    s.integrate(rules, frame)?;
    if !matches!(s.state, State::Air | State::Vert | State::VertPop) {
        (s.air_since, s.gravity_percent) = (0, 100);
    } else if s.air_since == 0 {
        s.air_since = s.now;
    }
    if s.up[1] < -3800 {
        s.upright_y = s.position[1];
    }
    if s.state == State::Ground {
        s.ground_y = s.position[1];
    }
    Ok(outcome)
}
