use crate::anim::{AnimState, AnimationTable};
use crate::collision::{CollisionWorld, Hit};
use crate::skater::{STANDING_HEIGHT, dot_q12, mul_q12, normalize, sqrt_int};

pub const GRAVITY: i32 = 0x1964;
pub const AIR_GRAVITY: i32 = 13000;
pub const WALL_REACH: i32 = 0x1e;
pub const BOUNCE_REACH: i32 = 10;
pub const HEAD_REACH: i32 = 0x3c;
pub const UNSTICK_SPAN: i32 = 8000;
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
pub const WALL_HIT_TURN: i32 = 200;
pub const PUSH_COOLDOWN: i32 = 0x14;
pub const PUSH_TARGET_INIT: i32 = 0x2e9b6;
pub const PUSH_SPEED: i32 = 0x14000;
pub const BRAKE_STOP: i32 = 0xa000;

pub const STAT_AIR: usize = 0;
pub const STAT_OLLIE: usize = 2;
pub const STAT_SPEED: usize = 3;
pub const STAT_SPIN: usize = 4;
pub const STAT_LANDING: usize = 5;
pub const STAT_SWITCH: usize = 6;
pub const TONY_HAWK_STATS: [i32; 11] = [7, 5, 2, 6, 8, 5, 3, 3, 6, 5, 0];
const SWITCH_WEIGHT: [i32; 11] = [50, 50, 100, 20, 50, 100, 100, 80, 100, 100, 30];
const STAT_CAP: [i32; 11] = [100, 100, 130, 110, 100, 130, 100, 130, 130, 130, 100];

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
const CROUCH_END: i32 = 0x1a;
const CROUCH_PINGPONG: i32 = 0x13;

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
    pub spin_left: bool,
    pub spin_right: bool,
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
    /// Stat as ten per point, multiplayer adds two plus the handicap and floors at 30, then the cap
    /// and the switch scaling apply.
    pub fn stat(self, index: usize, switched: bool) -> i32 {
        let mut value = match self.handicap {
            None => self.stats[index] * 10,
            Some(h) => ((self.stats[index] + 2 + h) * 10).max(0x1e),
        }
        .min(STAT_CAP[index]);
        if index == STAT_SWITCH {
            return value.min(100);
        }
        if switched {
            let switch = self.stat(STAT_SWITCH, switched);
            value += SWITCH_WEIGHT[index] * ((switch + 100) * value * 0x32 / 10000 - value) / 100;
        }
        value
    }

    /// Big drop limit in units from the landing stat scaled base.
    pub fn big_drop_limit(self, switched: bool) -> i32 {
        BIG_DROP_BASE * (self.stat(STAT_LANDING, switched) * 0x46 + 10000) / 10000
    }

    /// Frames an ollie can charge, from the ollie and air stats.
    pub fn max_charge(self, switched: bool) -> i32 {
        0xf - (self.stat(STAT_OLLIE, switched) + self.stat(STAT_AIR, switched)) / 0x14
    }
}

/// Vertical ollie impulse in Q12 for a flat takeoff, negative meaning up.
pub fn ollie_impulse(ollie_stat: i32, air_stat: i32, charge: i32) -> i32 {
    let max_charge = 0xf - (ollie_stat + air_stat) / 0x14;
    let base = (-0x21c - ollie_stat) * 5000 / 10000 * 0x400 / 10;
    let held = ((ollie_stat + 0x21c) * 5000 / 10000 - (ollie_stat * 0xce4 + 0x166e30) / 10000)
        * charge
        * 0x400
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
    /// Accumulated turn angle in 4096 units per revolution.
    pub spin: i32,
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
    pub push_cooldown: i32,
    /// Speed in Q12 units per frame that pushes stop accelerating toward.
    pub push_target: i32,
    pub brake_mode: i32,
    /// Air pitch per frame from up or down, 4096 units per turn.
    pub pitch: i32,
    /// Frame time of the last air frame without a direction, held input waits eight units after it.
    pub quiet_since: i32,
    /// Air spin within the current half turn, and the half turns completed.
    pub spin_angle: i32,
    pub spins: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Grounded(Hit),
    LeftGround(Option<Hit>),
    Airborne,
    Landed(Hit),
}

type V3 = [i32; 3];

/// The executable scales by the single precision constants 2^-12 and 6.2831854820251465.
const TWO_PI_F32: f64 = 6.283_185_482_025_146_5;

fn trig_q12(angle: i32) -> (i32, i32) {
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
    switched: bool,
    frame: FrameScale,
) -> Result<V3, String> {
    let mut speed = speed_of(v)?;
    let speed_stat = rules.stat(STAT_SPEED, switched);
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
            spin: 0,
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
            push_cooldown: 0,
            push_target: PUSH_TARGET_INIT,
            brake_mode: 0,
            pitch: 0,
            quiet_since: 0,
            spin_angle: 0,
            spins: 0,
        }
    }

    fn on_ground(&self) -> bool {
        matches!(self.state, State::Ground | State::Stopped)
    }

    fn set_state(&mut self, state: State) {
        if state == State::Air {
            self.airs += 1;
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
        self.set_state(State::Air);
        self.landing_timer = LANDING_TIMER;
        self.spin = 0;
        self.jumped = true;
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
            if self.crouching {
                self.run_anim(table, ANIM_CROUCH_LAND, 3, -1, -1)?;
            } else {
                self.set_anim(table, ANIM_LAND)?;
            }
            self.spin = 0;
        }
        if !held {
            self.crouching = false;
            if self.air_since > 0 && self.now - self.air_since > LATE_OLLIE_FRAMES {
                self.armed = false;
            }
            if self.armed {
                self.ollie(table, rules)?;
            }
            self.charge = 0;
            return Ok(());
        }
        self.charge = (self.charge + 1).min(rules.max_charge(self.switched));
        if self.on_ground() {
            self.armed = true;
            if !self.crouching && self.turn.abs() < 0xa000 {
                self.turn = 0;
                self.run_anim(table, ANIM_CROUCH, 0, CROUCH_END, CROUCH_PINGPONG)?;
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
                if self.state == State::Air {
                    self.set_anim(table, ANIM_AIR)?;
                }
            }
        }
        Ok(())
    }

    fn ollie(&mut self, table: &AnimationTable, rules: Rules) -> Result<(), String> {
        self.armed = false;
        self.jumped = true;
        self.vert_jump = self.vert;
        if self.up[1].abs() < 0x9c4 {
            return Err("vert ollie impulse unsupported".into());
        }
        let impulse = ollie_impulse(
            rules.stat(STAT_OLLIE, self.switched),
            rules.stat(STAT_AIR, self.switched),
            self.charge,
        );
        self.velocity[1] = self.velocity[1]
            .checked_add(impulse)
            .ok_or("ollie impulse overflow")?;
        self.ollies += 1;
        if self.charge < rules.max_charge(self.switched) {
            self.short_ollies += 1;
        }
        self.spin = 0;
        if self.vert_jump && self.on_ground() {
            return Err("vert air state unsupported".into());
        }
        self.set_state(State::Air);
        self.landing_timer = LANDING_TIMER;
        self.set_anim(table, ANIM_OLLIE)
    }

    fn basis_matrix(&self) -> [i32; 9] {
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
        if angle == 0 {
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

    /// Air spin from the turn rate, scaled by the spin stat and cut nine percent per half turn done.
    fn air_spin(&mut self, rules: Rules, frame: FrameScale) -> Result<(), String> {
        if self.vert_jump {
            return Err("vert air spin unsupported".into());
        }
        let stat = rules.stat(STAT_SPIN, self.switched);
        let amount = (stat + 300) * (self.turn >> 12) * 25 / 10000 * (100 - 9 * self.spins) / 100;
        if amount == 0 {
            return Ok(());
        }
        let angle = (amount * frame.scale) >> 8;
        self.set_basis(mat_mul_cols(self.basis_matrix(), rot_y_matrix(angle)));
        self.spin_angle += angle;
        if self.spin_angle > 0x7ff {
            self.spin_angle -= 0x800;
            self.spins += 1;
        }
        if self.spin_angle < -0x7ff {
            self.spin_angle += 0x800;
            self.spins += 1;
        }
        Ok(())
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
        self.push_target = stat_speed(rules.stat(STAT_SPEED, self.switched), bonus);
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
        if self.brake_mode != 0 {
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
        if !self.on_ground() {
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
        let stat = rules.stat(STAT_SPEED, self.switched);
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
        let limit = stat_speed(rules.stat(STAT_SPEED, self.switched), 0x186);
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
        let shoulder = add(self.position, scaled(self.up, STANDING_HEIGHT)?)?;
        let end = add(shoulder, scaled(self.lateral, direction * BOUNCE_REACH)?)?;
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
        let away_end = add(self.position, scaled(normal, BOUNCE_REACH)?)?;
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
        Ok(())
    }

    fn finish_frame(
        &mut self,
        world: &CollisionWorld,
        accel: V3,
        rules: Rules,
        frame: FrameScale,
        free_move: bool,
    ) -> Result<(), String> {
        if !free_move {
            self.position = world.move_retrying(self.old_position, self.position)?;
        }
        let descent = self.position[1] - self.old_position[1];
        if self.state == State::Air && descent > 0 {
            self.fall += descent;
            self.clearance -= descent;
        } else {
            self.fall = 0;
        }
        self.velocity = add(self.velocity, frame_part(accel, frame.scale)?)?;
        self.velocity = clamp_speed(self.velocity, rules, self.switched, frame)?;
        Ok(())
    }
}

/// Probes the movement segment, extended to the wall reach when short, with lips included.
fn wall_probe(world: &CollisionWorld, old: V3, new: V3) -> Result<(Option<Hit>, i32), String> {
    if old == new {
        return Ok((None, 0));
    }
    let delta = std::array::from_fn(|a| new[a] - old[a]);
    let length = speed_of(delta)?;
    let mut end = new;
    if length < WALL_REACH * 4096 {
        let divisor = length.max(0x800);
        end = std::array::from_fn(|a| WALL_REACH * delta[a] / divisor * 4096 + old[a]);
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
    world: &CollisionWorld,
    table: &AnimationTable,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
) -> Result<Outcome, String> {
    let stopped = s.state == State::Stopped;
    let old_matrix = s.basis_matrix();
    s.begin_frame(world)?;
    let mut accel = s.accel;
    let half_push = frame_part(accel, frame.scale_squared)?.map(|c| c / 2);
    let travel = add(frame_part(s.velocity, frame.scale)?, half_push)?;
    s.position = add(s.position, travel)?;
    if dot_q12(s.forward, s.velocity) > FAKIE_DOT {
        s.forward = s.forward.map(|c| -c);
        s.lateral = s.lateral.map(|c| -c);
        s.switched = !s.switched;
    }
    let mut turn_units = s.turn >> 12;
    if s.landing_timer != 0 {
        if turn_units == 0 {
            turn_units = s.fall_line_alignment(rules, ALIGN_EXTRA)?;
        } else {
            s.landing_timer = (s.landing_timer - 3).max(0);
        }
    }
    let angle = (frame.scale * turn_units) >> 8;
    s.rotate(old_matrix, angle)?;
    s.spin += angle;
    let (contact, length) = wall_probe(world, s.old_position, s.position)?;
    if let Some(contact) = contact {
        s.wall_branch(world, table, &contact, length, frame)?;
    }
    s.fast = speed_of(s.velocity)? > stat_speed(rules.stat(STAT_SPEED, s.switched), 0x186);
    let ignore_lips = !stopped && (s.velocity[1] > 0 || s.forward[1] < 1);
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
        if !(bit23 && bit24 && !lip) && hit.distance_units <= s.stick_slack + 0x74 {
            s.position = world.stand(s.position, s.up, &hit)?;
            if lip {
                return Err(format!(
                    "ground lip surface unsupported item={} face={}",
                    hit.item_index, hit.face_index
                ));
            }
            let normal = hit.normal.map(i32::from);
            let convex_edge = dot_q12(s.forward, normal) < 0
                && s.velocity[1] <= 0
                && (1..3000).contains(&dot_q12(s.last_normal, normal));
            if stopped && !bit24 {
                s.velocity = [0; 3];
            }
            grounded = !bit24 && (stopped || !convex_edge);
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
    if grounded {
        let hit = hit.unwrap();
        let normal = hit.normal.map(i32::from);
        s.slope_gravity = project_onto_plane([0, GRAVITY, 0], normal);
        s.ground_normal = normal;
        accel = add(accel, s.slope_gravity)?;
        let mut v = reproject(s.velocity, normal)?;
        if stopped {
            s.position = s.old_position;
        } else {
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
            for k in frictions {
                let drag = frame_part(rolling.map(|c| mul_q12(k, c)), frame.scale)?;
                accel = std::array::from_fn(|a| accel[a] - drag[a]);
            }
        }
        s.velocity = v;
        free_move = post_contact_probes(world, table, s)?;
        s.lean_probe(world)?;
    } else {
        accel = add(accel, [0, GRAVITY, 0])?;
    }
    if !grounded {
        s.leave_ground(table)?;
    }
    s.finish_frame(world, accel, rules, frame, free_move)?;
    Ok(if grounded {
        Outcome::Grounded(hit.unwrap())
    } else {
        Outcome::LeftGround(hit)
    })
}

fn land(world: &CollisionWorld, s: &mut Skater, rules: Rules, hit: &Hit) -> Result<(), String> {
    let normal = hit.normal.map(i32::from);
    let target = add(hit.position_fixed, scaled(normal, STANDING_HEIGHT)?)?;
    s.position = world.move_retrying(s.position, target)?;
    if hit.flags & 0x400000 != 0 {
        return Err(format!(
            "air lip landing unsupported item={} face={}",
            hit.item_index, hit.face_index
        ));
    }
    s.terrain = (hit.flags >> 25) & 0xf;
    s.turn = 0;
    s.set_state(State::Ground);
    let flat = project_onto_plane(s.velocity, normal);
    if speed_of(flat)? > 0x1e000 {
        let heading = normalize(flat);
        let facing = normalize(project_onto_plane(s.forward, normal));
        if dot_q12(heading, facing).abs() < 0x666 {
            return Err("air sideways landing bail unsupported".into());
        }
    }
    let height = (s.clearance + s.fall) >> 12;
    let mut limit = rules.big_drop_limit(s.switched);
    if s.charge != 0 {
        limit += (0xb - s.charge) * 0x3c;
    }
    if height > limit {
        return Err(format!(
            "air big drop bail unsupported height={height} limit={limit}"
        ));
    }
    s.vert = hit.flags & VERT_FACE != 0;
    let tilt = dot_q12(s.up, normal);
    if tilt < 2000 && s.up[1] > -2000 && (s.velocity[1] > 150000 || tilt < 200) {
        return Err("air landing bail unsupported".into());
    }
    s.velocity = flat;
    s.last_normal = normal;
    s.smoothed_up = normal;
    s.same_normal_time = 0;
    s.smooth_basis();
    head_probe(world, s)
}

/// The air handler after the input phase, stopping at wall bounces, lips, wallrides and bails.
pub fn air_step(
    world: &CollisionWorld,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
) -> Result<Outcome, String> {
    s.begin_frame(world)?;
    let (start_up, start_forward) = (s.up, s.forward);
    let mut accel = [0; 3];
    let half_push = frame_part(s.accel, frame.scale_squared)?.map(|c| c / 2);
    let travel = add(frame_part(s.velocity, frame.scale)?, half_push)?;
    s.position = add(s.position, travel)?;
    s.air_pitch();
    s.air_spin(rules, frame)?;
    s.upright_roll(start_up, start_forward);
    let mut hit = world.query_filtered(s.old_position, s.position, false)?;
    if let Some(found) = hit
        && found.flags & 0x400000 != 0
        && found.face_flags & 0x80 == 0
    {
        hit = world.query_filtered(s.old_position, s.position, true)?;
    }
    match hit {
        Some(hit) if hit.flags & 0x1000000 != 0 => {
            return Err(format!(
                "air wall contact unsupported item={} face={}",
                hit.item_index, hit.face_index
            ));
        }
        Some(hit) => land(world, s, rules, &hit)?,
        None => accel = add(accel, [0, AIR_GRAVITY, 0])?,
    }
    s.finish_frame(world, accel, rules, frame, false)?;
    Ok(match hit {
        Some(hit) => Outcome::Landed(hit),
        None => Outcome::Airborne,
    })
}

/// One single session run: the skater, the frame clock and the unpaused VB counter.
#[derive(Clone, Debug)]
pub struct Run {
    pub skater: Skater,
    pub clock: FrameClock,
    pub vb: i32,
}

impl Run {
    /// Each run starts on the special push animation and a refilled clock ring.
    pub fn start(
        table: &AnimationTable,
        position_fixed: V3,
        angles: [i16; 3],
    ) -> Result<Self, String> {
        let mut skater = Skater::at_restart(position_fixed, angles);
        skater.anim.run(table, ANIM_PUSH_SPECIAL, 0, -1, -1)?;
        let mut clock = FrameClock::default();
        clock.reset(0);
        Ok(Self {
            skater,
            clock,
            vb: 0,
        })
    }

    pub fn frame(
        &mut self,
        world: &CollisionWorld,
        table: &AnimationTable,
        rules: Rules,
        pad: Pad,
    ) -> Result<Outcome, String> {
        self.vb += 2;
        let outcome = step(
            world,
            table,
            &mut self.skater,
            rules,
            self.clock.current(),
            pad,
        )?;
        self.clock.end_frame(self.vb);
        Ok(outcome)
    }
}

/// One frame: animation tick, pad input, ollie handling, the state handler and the airborne timer.
pub fn step(
    world: &CollisionWorld,
    table: &AnimationTable,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    s.now += 1;
    s.anim.tick(frame.scale)?;
    if s.on_ground() {
        s.turn_input(table, rules, pad, frame)?;
    } else {
        s.air_turn(table, pad, frame)?;
    }
    s.accel = [0; 3];
    s.push(table, rules, pad)?;
    s.jump_input(table, rules, pad.jump)?;
    s.brake(table, pad, frame)?;
    s.wall_hit_recovery(table)?;
    s.decay_push_target(rules);
    let outcome = match s.state {
        State::Ground | State::Stopped => ground_step(world, table, s, rules, frame),
        State::Air => air_step(world, s, rules, frame),
    }?;
    if s.on_ground() {
        s.air_since = 0;
    } else if s.air_since == 0 {
        s.air_since = s.now;
    }
    Ok(outcome)
}
