use crate::collision::{CollisionWorld, Hit};
use crate::skater::{STANDING_HEIGHT, dot_q12, mul_q12, normalize, sqrt_int};

pub const GRAVITY: i32 = 0x1964;
pub const AIR_GRAVITY: i32 = 13000;
pub const WALL_REACH: i32 = 0x1e;
pub const BOUNCE_REACH: i32 = 10;
pub const HEAD_REACH: i32 = 0x3c;
pub const UNSTICK_SPAN: i32 = 500;
pub const NORMAL_HOLD_LIMIT: i32 = 0x18000;
pub const FAKIE_DOT: i32 = 0x5000;
pub const BIG_DROP_BASE: i32 = 0x3c3;
pub const WORLD_UP: [i32; 3] = [0, -4096, 0];
pub const TURN_ACCEL: i32 = 0x3c;
pub const TURN_MAX: i32 = 0x2d000;
pub const TURN_MAX_CROUCH: i32 = 0x5a000;
pub const LEAN_FRAMES: i32 = 0x16;
pub const LEAN_PROBE_HEIGHT: i32 = 100;
pub const LEAN_PROBE_REACH: i32 = 0x5a;
pub const WALL_TURN: i32 = 0x19;
pub const WALL_PUSH: i32 = 0xcd;
pub const HEAD_ON_DOT: i32 = 0xb50;

/// Digital pad state for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub down: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub speed_stat: i32,
    pub drop_stat: i32,
    pub auto_kick: bool,
}

impl Rules {
    /// Big drop limit in units from the stat scaled base.
    pub fn big_drop_limit(self) -> i32 {
        BIG_DROP_BASE * (self.drop_stat * 0x46 + 10000) / 10000
    }
}

/// Frame scale in Q8 with 256 meaning two 60 Hz ticks, plus the normal hold increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameScale {
    pub scale: i32,
    pub scale_squared: i32,
    pub hold_delta: i32,
}

pub const DESIGN_FRAME: FrameScale = FrameScale {
    scale: 256,
    scale_squared: 256,
    hold_delta: 0x1000,
};

impl FrameScale {
    /// Sum of the last three tick deltas capped at 15, the newest delta drives the hold timer.
    pub fn from_ticks(deltas: [i32; 3]) -> Self {
        let scale = (deltas.iter().sum::<i32>().min(15) << 8) / 6;
        Self {
            scale,
            scale_squared: (scale * scale) >> 8,
            hold_delta: deltas[2] * 0x800,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ground,
    Air,
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
    pub anim: u16,
    pub anim_frame: i32,
    /// Accumulated turn angle in 4096 units per revolution.
    pub spin: i32,
    pub crouch_lean: bool,
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

pub fn clamp_speed(mut v: V3, rules: Rules, frame: FrameScale) -> Result<V3, String> {
    let mut speed = speed_of(v)?;
    let cap = stat_speed(rules.speed_stat, 500);
    if speed > cap && speed & !0xfff != 0 {
        let shifted = speed >> 12;
        for c in &mut v {
            *c = (cap >> 12).checked_mul(*c).ok_or("ground clamp overflow")? / shifted;
        }
        speed = sqrt_int(dot_q12(v, v)) << 6;
    }
    if speed > stat_speed(rules.speed_stat, 0x186) {
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
            anim: 0,
            anim_frame: 0,
            spin: 0,
            crouch_lean: false,
        }
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
    fn turn_input(&mut self, pad: Pad, frame: FrameScale) {
        let mut accel = (TURN_ACCEL * 0x100 * frame.scale) >> 8;
        let mut limit = TURN_MAX;
        self.crouch_lean = pad.down;
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
        self.lean_anim(pad);
    }

    fn lean_anim(&mut self, pad: Pad) {
        let limit = if pad.down { TURN_MAX_CROUCH } else { TURN_MAX };
        let lean = self.lean;
        let anim = self.anim;
        if lean != 0 && (anim == 0 || (6..=10).contains(&anim)) {
            let target = (lean.abs() * LEAN_FRAMES / limit).min(LEAN_FRAMES);
            let (id, bump) = match (self.switched, lean >= 0) {
                (false, true) | (true, false) => (7, false),
                (false, false) => (6, false),
                (true, true) => (6, true),
            };
            if anim != id {
                self.anim_frame = 0;
            }
            if bump && self.anim_frame < target {
                self.anim_frame += 1;
            }
            self.anim_frame = lerp_frame(self.anim_frame, target);
            self.anim = id;
        }
        if lean == 0 && (self.anim == 6 || self.anim == 7) {
            self.anim = 0;
            self.anim_frame = 0;
        }
    }

    /// Removes the into wall velocity, pushes off and snaps the heading almost parallel to the wall.
    fn wall_response(&mut self, normal: V3) -> Result<(), String> {
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
        let angle = if side > 0 {
            WALL_TURN - 0x400
        } else {
            0x400 - WALL_TURN
        };
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

    /// Head on contacts enter the wall hit animation, which is not reproduced yet.
    fn head_on(&self, normal: V3, needs_speed: bool) -> Result<bool, String> {
        if self.anim == 0xc {
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
            if self.head_on(normal, false)? {
                return Err(format!(
                    "ground head on wall unsupported item={} face={}",
                    hit.item_index, hit.face_index
                ));
            }
            return self.wall_response(normal);
        }
        let d = dot_q12(normal, self.last_normal);
        if (-0x7ff..=0x7ff).contains(&d) {
            let [nx, ny, nz] = normal;
            if -nx.abs() == ny || -ny < nx.abs() || -ny <= nz.abs() {
                self.position = self.old_position;
                if self.head_on(normal, true)? {
                    return Err(format!(
                        "ground head on wall unsupported item={} face={}",
                        hit.item_index, hit.face_index
                    ));
                }
                return self.wall_response(normal);
            }
            return self.settle_on(world, hit, normal, false, frame);
        }
        if hit.distance_units <= length >> 12 {
            return self.settle_on(world, hit, normal, true, frame);
        }
        Ok(())
    }

    /// Lateral shoulder probe that pushes the skater off a near vertical face.
    fn bounce(&mut self, world: &CollisionWorld, direction: i32) -> Result<bool, String> {
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
        if self.head_on(normal, true)? {
            return Err(format!(
                "ground head on wall unsupported item={} face={}",
                hit.item_index, hit.face_index
            ));
        }
        self.wall_response(normal)?;
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
        if self.lean == 0 || !(self.anim == 6 || self.anim == 7) {
            return Ok(());
        }
        let span = if self.lean >= 0 {
            LEAN_FRAMES
        } else {
            -LEAN_FRAMES
        };
        let reach = self.anim_frame * LEAN_PROBE_REACH / span;
        let start = add(self.position, scaled(self.up, LEAN_PROBE_HEIGHT)?)?;
        let end = add(start, scaled(self.lateral, -reach)?)?;
        let Some(hit) = world.query(start, end)? else {
            return Ok(());
        };
        self.anim_frame = (hit.distance_units * span / LEAN_PROBE_REACH).abs();
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
                "ground unstick fallback unsupported, nothing within 500 units below".into(),
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
        self.velocity = clamp_speed(self.velocity, rules, frame)?;
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
fn bounce_probes(world: &CollisionWorld, s: &mut Skater) -> Result<bool, String> {
    let entry = s.position;
    if s.bounce(world, -1)? {
        if s.bounce(world, 1)? {
            s.position = entry;
        }
        return Ok(true);
    }
    if !s.bounce(world, 1)? {
        return Ok(false);
    }
    if s.bounce(world, -1)? {
        s.position = entry;
    }
    Ok(true)
}

fn post_contact_probes(world: &CollisionWorld, s: &mut Skater) -> Result<bool, String> {
    let bounced = bounce_probes(world, s)?;
    if s.up[1] > 1000 {
        return Err("ground upside down bail unsupported".into());
    }
    head_probe(world, s)?;
    Ok(bounced)
}

/// One native ground frame with digital steering, without bail, manual, triggers or moving objects.
pub fn ground_step(
    world: &CollisionWorld,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    s.turn_input(pad, frame);
    if pad.down && !pad.left && !pad.right {
        return Err("ground brake unsupported".into());
    }
    let old_matrix = s.basis_matrix();
    s.begin_frame(world)?;
    let mut accel = [0; 3];
    s.position = add(s.position, frame_part(s.velocity, frame.scale)?)?;
    if dot_q12(s.forward, s.velocity) > FAKIE_DOT {
        s.forward = s.forward.map(|c| -c);
        s.lateral = s.lateral.map(|c| -c);
        s.switched = !s.switched;
    }
    let angle = (frame.scale * (s.turn >> 12)) >> 8;
    s.rotate(old_matrix, angle)?;
    s.spin += angle;
    let (contact, length) = wall_probe(world, s.old_position, s.position)?;
    if let Some(contact) = contact {
        s.wall_branch(world, &contact, length, frame)?;
    }
    s.fast = speed_of(s.velocity)? > stat_speed(rules.speed_stat, 0x186);
    let ignore_lips = s.velocity[1] > 0 || s.forward[1] < 1;
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
            grounded = !bit24 && !convex_edge;
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
        accel = add(accel, project_onto_plane([0, GRAVITY, 0], normal))?;
        let mut v = reproject(s.velocity, normal)?;
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
        s.velocity = v;
        free_move = post_contact_probes(world, s)?;
        s.lean_probe(world)?;
    } else {
        accel = add(accel, [0, GRAVITY, 0])?;
    }
    s.state = if grounded { State::Ground } else { State::Air };
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
    let flat = project_onto_plane(s.velocity, normal);
    if speed_of(flat)? > 0x1e000 {
        let heading = normalize(flat);
        let facing = normalize(project_onto_plane(s.forward, normal));
        if dot_q12(heading, facing).abs() < 0x666 {
            return Err("air sideways landing bail unsupported".into());
        }
    }
    let height = (s.clearance + s.fall) >> 12;
    if height > rules.big_drop_limit() {
        return Err(format!(
            "air big drop bail unsupported height={height} limit={}",
            rules.big_drop_limit()
        ));
    }
    let tilt = dot_q12(s.up, normal);
    if tilt < 2000 && s.up[1] > -2000 && (s.velocity[1] > 150000 || tilt < 200) {
        return Err("air landing bail unsupported".into());
    }
    s.velocity = flat;
    s.last_normal = normal;
    s.smoothed_up = normal;
    s.same_normal_time = 0;
    s.smooth_basis();
    s.state = State::Ground;
    head_probe(world, s)
}

/// One native air frame without pad input, stopping at wall bounces, lips, wallrides and bails.
pub fn air_step(
    world: &CollisionWorld,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    if pad != Pad::default() {
        return Err("air input unsupported".into());
    }
    s.begin_frame(world)?;
    let mut accel = [0; 3];
    s.position = add(s.position, frame_part(s.velocity, frame.scale)?)?;
    let roll_axis = cross_q12(short(s.forward), WORLD_UP);
    if dot_q12(s.up, roll_axis).abs() > 0x29 {
        return Err("air upright roll unsupported".into());
    }
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

/// Dispatches one frame by the skater state.
pub fn step(
    world: &CollisionWorld,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
    pad: Pad,
) -> Result<Outcome, String> {
    match s.state {
        State::Ground => ground_step(world, s, rules, frame, pad),
        State::Air => air_step(world, s, rules, frame, pad),
    }
}
