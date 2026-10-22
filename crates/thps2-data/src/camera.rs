use crate::{
    collision::CollisionWorld,
    ground::{Skater, native_state, trig_q12},
    skater::sqrt_int,
};

pub fn rsin(angle: i32) -> i32 {
    trig_q12(angle).1
}

pub fn rcos(angle: i32) -> i32 {
    trig_q12(angle).0
}

/// Arc cosine of a Q12 value in 4096 units per turn.
pub fn racos(x: i32) -> i32 {
    ((f64::from(x) * (1.0 / 4096.0)).acos() * 4096.0 * 0.159_154_943_091_895_35) as i32
}

type V3 = [i32; 3];
/// Quaternion x, y, z, w in Q12.
pub type Quat = [i32; 4];
/// PS1 rotation matrix, row major Q12.
pub type Matrix = [i16; 9];

const IDENTITY: Quat = [0, 0, 0, 0x1000];
/// Distance, pitch and height of the follow view. The first frame of a run still has the ones the
/// camera was built with.
const PRESET_DEFAULT: V3 = [386, 118, -140];
const PRESET_FOLLOW: V3 = [290, 150, -108];
const RATE_SLOW: i32 = 0x97;
const RATE_NORMAL: i32 = 0x137;
const RATE_SNAP: i32 = 0x2c7;
/// Run start settles the camera with this many updates before the first frame.
const SETTLE_CALLS: u32 = 10;
/// Heading and orientation snap to the skater for the first calls, feelers start after this one.
const SNAP_CALLS: u32 = 12;
const VERT_UP: i32 = 0xdf4;
const ORBIT_STEP: i32 = 0x40;
/// Side feeler gain and rear diagonal gain the camera is built with.
const SIDE_GAIN: i16 = 0x200;
const REAR_GAIN: i16 = 0x568;

/// The frame the camera updates in: the step the next physics frame uses, time up, the time up
/// orbit around an idle skater, and the pause the end screens hold the game in.
#[derive(Clone, Copy, Debug, Default)]
pub struct Moment {
    pub step: i32,
    /// World updates since the level start, the phase of a shake.
    pub frame: i32,
    pub time_up: bool,
    pub orbit: bool,
    pub paused: bool,
}

/// What the camera reads from the skater it follows. States use the original numbering.
#[derive(Clone, Copy, Debug)]
pub struct Tripod {
    pub position: V3,
    pub velocity: V3,
    pub state: u8,
    pub up: V3,
    pub forward: V3,
    pub bailing: bool,
    pub trick_running: bool,
    /// Left the ground by a jump, cleared on landing.
    pub jumped: bool,
    /// Set on every vert frame.
    pub vert: bool,
}

impl Tripod {
    pub fn of(s: &Skater) -> Self {
        Self {
            position: s.position,
            velocity: s.velocity,
            state: native_state(s.state),
            up: s.up,
            forward: s.forward,
            bailing: s.bailing,
            trick_running: s.trick_running,
            jumped: s.jumped,
            vert: s.lip_spin,
        }
    }
}

fn div_q12(sum: i128) -> i32 {
    (sum / 4096) as i32
}

pub fn dot3(a: V3, b: V3) -> i32 {
    let p = |i: usize| i128::from(a[i]) * i128::from(b[i]);
    div_q12(p(2) + p(1) + p(0))
}

pub fn mat_apply(m: &Matrix, v: V3) -> V3 {
    let m = m.map(i128::from);
    let v = v.map(i128::from);
    [
        div_q12(m[0] * v[0] + m[2] * v[2] + m[1] * v[1]),
        div_q12(m[5] * v[2] + m[3] * v[0] + m[4] * v[1]),
        div_q12(m[6] * v[0] + m[7] * v[1] + m[8] * v[2]),
    ]
}

/// Arc tangent of a Q12 ratio in 4096 units per turn.
fn ratan(x: i32) -> i32 {
    ((f64::from(x) * (1.0 / 4096.0)).atan() * 651.898_628_763_671_2) as i32
}

fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i].wrapping_add(b[i]))
}

fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i].wrapping_sub(b[i]))
}

/// Pitch and yaw from one point toward another, and the horizontal distance in whole units.
fn angles(from: V3, to: V3) -> ([i32; 2], i32) {
    let [dx, dy, dz] = sub(to, from).map(|c| c >> 12);
    let yaw = if dz == 0 {
        if dx > 0 { 0xfc00 } else { 0x400 }
    } else if dz > 0 {
        0x800 - ratan(-((dx << 12) / dz))
    } else {
        ratan((dx << 12) / dz)
    };
    let r = sqrt_int(dz.wrapping_mul(dz).wrapping_add(dx.wrapping_mul(dx)));
    let pitch = if r == 0 {
        if dy > 0 { 0x400 } else { -0x400 }
    } else if dy > 0 {
        ratan(((dy << 12) as u32 / r as u32) as i32)
    } else {
        -ratan(((-dy << 12) as u32 / r as u32) as i32)
    };
    ([pitch & 0xfff, yaw & 0xfff], r)
}

/// Q24 vector of length r along a pitch and yaw.
fn direction(r: i32, [pitch, yaw]: [i32; 2]) -> V3 {
    let c = (rcos(pitch) * r) >> 12;
    [-(c * rsin(yaw)), rsin(pitch) * r, -(c * rcos(yaw))]
}

/// Whole unit length of a Q12 vector, its components folded through shorts first.
pub(crate) fn length12(v: V3) -> i32 {
    let [x, y, z] = v.map(|c| i32::from((c >> 12) as i16));
    sqrt_int(
        x.wrapping_mul(x)
            .wrapping_add(y.wrapping_mul(y))
            .wrapping_add(z.wrapping_mul(z)),
    )
}

fn rot(axis: usize, angle: i32) -> Quat {
    let h = i32::from(angle as i16) >> 1;
    let mut q = [0, 0, 0, rcos(h)];
    q[axis] = rsin(h);
    q
}

pub fn quat_mul(a: Quat, b: Quat) -> Quat {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    let m = i32::wrapping_mul;
    let s = |terms: [i32; 4]| terms.iter().fold(0i32, |acc, t| acc.wrapping_add(*t)) >> 12;
    [
        s([m(ax, bw), -m(ay, bz), m(az, by), m(aw, bx)]),
        s([m(ax, bz), -m(az, bx), m(ay, bw), m(aw, by)]),
        s([m(az, bw), -m(ax, by), m(ay, bx), m(aw, bz)]),
        s([m(aw, bw), -m(ay, by), -m(ax, bx), -m(az, bz)]),
    ]
}

pub fn quat_to_matrix(q: Quat) -> Matrix {
    let [x, y, z, w] = q;
    let p = |a: i32, b: i32| a.wrapping_mul(b) >> 11;
    let (xx, yy, zz) = (p(x, x), p(y, y), p(z, z));
    let (xy, xz, yz) = (p(x, y), p(x, z), p(y, z));
    let (xw, yw, zw) = (p(x, w), p(y, w), p(z, w));
    [
        0x1000 - zz - yy,
        zw + xy,
        xz - yw,
        xy - zw,
        0x1000 - xx - zz,
        xw + yz,
        yw + xz,
        yz - xw,
        0x1000 - xx - yy,
    ]
    .map(|v| v as i16)
}

pub fn matrix_to_quat(m: &Matrix) -> Quat {
    let m = m.map(i32::from);
    let tr = m[0] + m[4] + m[8];
    let k = |s: i32| {
        if s == 0 {
            0
        } else {
            (0x80_0000i64 / i64::from(s)) as i32
        }
    };
    if tr > 0 {
        let s = sqrt_int((tr + 0x1000).wrapping_mul(0x1000));
        let k = k(s);
        return [
            (m[5] - m[7]).wrapping_mul(k) >> 12,
            (m[6] - m[2]).wrapping_mul(k) >> 12,
            (m[1] - m[3]).wrapping_mul(k) >> 12,
            s >> 1,
        ];
    }
    const NEXT: [usize; 3] = [1, 2, 0];
    let mut i = if m[0] < m[4] { 1 } else { 0 };
    if m[i * 4] < m[8] {
        i = 2;
    }
    let (j, l) = (NEXT[i], NEXT[NEXT[i]]);
    let s = sqrt_int((m[i * 4] - m[l * 4] - m[j * 4] + 0x1000).wrapping_mul(0x1000));
    let c = k(s);
    let mut q = [0; 4];
    q[i] = s >> 1;
    q[3] = (m[j * 3 + l] - m[l * 3 + j]).wrapping_mul(c) >> 12;
    q[j] = (m[i * 3 + j] + m[j * 3 + i]).wrapping_mul(c) >> 12;
    q[l] = (m[i * 3 + l] + m[l * 3 + i]).wrapping_mul(c) >> 12;
    q
}

fn normalised(q: Quat) -> Quat {
    let n = sqrt_int(
        q.iter()
            .rev()
            .fold(0i32, |acc, c| acc.wrapping_add(c.wrapping_mul(*c))),
    );
    let n = if n == 0 { 1 } else { n };
    q.map(|c| (c << 12) / n)
}

/// Spherical blend from one orientation toward another by t in Q12.
pub fn slerp(from: Quat, to: Quat, t: i32) -> Quat {
    let mut a = normalised(from);
    let b = normalised(to);
    let mut c = (b[3] * a[3] + b[2] * a[2] + b[1] * a[1] + b[0] * a[0]) >> 12;
    if c < 0 {
        c = -c;
        a = a.map(|v| -v);
    }
    let c = c.min(0x1000);
    let (wa, wb) = if 0x1000 - c <= 0x80 {
        (0x1000 - t, t)
    } else {
        let th = racos(c);
        let s = rsin(th);
        (
            (rsin(((0x1000 - t) * th) >> 12) << 12) / s,
            (rsin((th * t) >> 12) << 12) / s,
        )
    };
    normalised(std::array::from_fn(|i| (wb * b[i] + wa * a[i]) >> 12))
}

/// Cross product as the geometry coprocessor's outer product leaves it, before the shift.
fn outer(r: [i16; 3], ir: [i16; 3]) -> V3 {
    let [r1, r2, r3] = r.map(i32::from);
    let [i1, i2, i3] = ir.map(i32::from);
    [r2 * i3 - r3 * i2, r3 * i1 - r1 * i3, r1 * i2 - r2 * i1]
}

/// Feeler yaws and pitch with the values they held before their last probe, the averages of the
/// two pairs bend the desired orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Feelers {
    pub side_a: [i16; 2],
    pub side_b: [i16; 2],
    pub down: [i16; 2],
    pub rear: [i16; 2],
    pub index: u8,
    pub wraps: i32,
    /// Ground distance under the camera from the down probe, -1 when it found none.
    pub ground: i32,
}

/// The follow camera of one skater.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Camera {
    pub position: V3,
    pub velocity: V3,
    pub target: V3,
    pub previous_target: V3,
    pub look: V3,
    pub previous_look: V3,
    pub orientation: Quat,
    pub desired: Quat,
    pub previous: Quat,
    pub latched: Quat,
    pub latch: bool,
    pub settled: bool,
    /// 1 follows, 0x19 follows a skater upside down on vert.
    pub mode: i32,
    pub calls: u32,
    /// Q24 heading, its previous value and the one it moves toward.
    pub heading: V3,
    pub previous_heading: V3,
    pub heading_target: V3,
    pub distance: i32,
    pub speed: i32,
    pub history: [i32; 6],
    pub vert_latch: bool,
    pub tracking: bool,
    pub bump: [i32; 4],
    pub since_tracking: i32,
    pub tracking_frames: i32,
    pub ramp: i32,
    pub feelers: Feelers,
    pub yaw: i32,
    pub height_offset: V3,
    /// Height vector of this frame, raised by the vert trick bump.
    pub height: V3,
    /// Distance, pitch and height the next update uses.
    pub preset: V3,
    /// Orientation and eye offset kept for the wall state, which freezes the view.
    pub wall_orientation: Quat,
    pub wall_offset: V3,
    /// Render eye and look point in whole units, and the view matrix.
    pub eye: V3,
    pub look_at: V3,
    pub matrix: Matrix,
    /// Shake amplitude about x, y and z, wound down each frame by its decay.
    pub shake: [i16; 3],
    pub shake_decay: [u8; 3],
    pub shake_rate: [u16; 3],
}

struct Frame<'a> {
    tripod: &'a Tripod,
    world: &'a CollisionWorld,
    step: i32,
    distance: i32,
    pitch: i32,
    time_up: bool,
}

impl Camera {
    /// A new camera on the skater, settled like the run start does before its first frame.
    pub fn settled(tripod: &Tripod, world: &CollisionWorld, step: i32) -> Result<Self, String> {
        let mut camera = Self {
            position: tripod.position,
            velocity: [0; 3],
            target: [0; 3],
            previous_target: [0; 3],
            look: [0; 3],
            previous_look: [0; 3],
            orientation: IDENTITY,
            desired: IDENTITY,
            previous: IDENTITY,
            latched: IDENTITY,
            latch: false,
            settled: false,
            mode: 1,
            calls: 0,
            heading: [0; 3],
            previous_heading: [0; 3],
            heading_target: [0; 3],
            distance: 0,
            speed: 0,
            history: [0; 6],
            vert_latch: false,
            tracking: false,
            bump: [0; 4],
            since_tracking: 1000,
            tracking_frames: 0,
            ramp: 0,
            feelers: Feelers::default(),
            yaw: 0,
            height_offset: [0; 3],
            height: [0; 3],
            preset: PRESET_DEFAULT,
            wall_orientation: [0; 4],
            wall_offset: [0; 3],
            eye: [0; 3],
            look_at: [0; 3],
            matrix: [0; 9],
            shake: [0; 3],
            shake_decay: [0; 3],
            shake_rate: [0; 3],
        };
        let pushed = Tripod {
            velocity: tripod.forward.map(|c| c.wrapping_mul(20)),
            ..*tripod
        };
        for _ in 0..SETTLE_CALLS {
            camera.think(
                &pushed,
                world,
                Moment {
                    step,
                    ..Moment::default()
                },
            )?;
        }
        Ok(camera)
    }

    /// One frame's update after the skaters moved.
    pub fn update(
        &mut self,
        tripod: &Tripod,
        world: &CollisionWorld,
        moment: Moment,
    ) -> Result<(), String> {
        if moment.orbit {
            self.yaw += ORBIT_STEP;
        }
        self.think(tripod, world, moment)?;
        self.preset = PRESET_FOLLOW;
        Ok(())
    }

    /// Paused, only the target and look point keep closing in on the skater.
    fn think(
        &mut self,
        tripod: &Tripod,
        world: &CollisionWorld,
        moment: Moment,
    ) -> Result<(), String> {
        let smooth = |pos: V3, prev: V3| -> V3 {
            std::array::from_fn(|i| pos[i].wrapping_shl(1).wrapping_add(prev[i]) / 3)
        };
        self.previous_target = self.target;
        self.target = smooth(tripod.position, self.previous_target);
        self.previous_look = self.look;
        self.look = smooth(tripod.position, self.previous_look);
        let [distance, pitch, height] = self.preset;
        self.height = [0, height << 12, 0];
        if moment.paused {
            return Ok(());
        }
        let f = Frame {
            tripod,
            world,
            step: moment.step,
            distance,
            pitch,
            time_up: moment.time_up,
        };
        self.calls += 1;
        if self.mode == 0x19 {
            self.follow_heading(&f);
            self.follow(&f, RATE_SNAP)?;
            if tripod.state != 1 && !tripod.bailing && tripod.up[1] < 0 {
                self.mode = 1;
            }
        }
        if tripod.up[1] > 100 && tripod.state == 1 {
            self.mode = 0x19;
        }
        if self.settled {
            let f = tripod.forward.map(|c| -i32::from(c as i16));
            self.heading_target = f;
            self.heading = f.map(|c| c << 12);
            self.previous_heading = self.heading;
        }
        self.follow_heading(&f);
        let rate = if tripod.bailing {
            RATE_NORMAL
        } else if self.settled {
            self.settled = false;
            RATE_SNAP
        } else if self.target == self.previous_target
            || (self.speed as u32) < 4
            || (tripod.state == 1 && self.calls > 17)
        {
            RATE_SLOW
        } else {
            RATE_NORMAL
        };
        self.follow(&f, rate)?;
        self.shake_view(moment.frame);
        self.write_view(tripod);
        Ok(())
    }

    /// Starts a shake of a preset kind, replacing any shake still running.
    pub fn shake(&mut self, kind: u8) {
        self.shake = match kind {
            0 => [25, 0, 50],
            1 => [0, 0, 30],
            _ => [0, 0, 25],
        };
        (self.shake_decay, self.shake_rate) = ([3, 0, 3], [600, 0, 600]);
    }

    /// Turns the orientation by a sine of each amplitude, then winds the amplitudes toward zero.
    fn shake_view(&mut self, frame: i32) {
        if self.shake == [0; 3] {
            return;
        }
        let swing = |k: usize| {
            let phase = i32::from(self.shake_rate[k]).wrapping_mul(frame) & 0xfff;
            rsin(phase) * i32::from(self.shake[k]) / 4096
        };
        let (ax, ay, az) = (swing(0), swing(1), swing(2));
        let turned = quat_mul(rot(1, ay), self.orientation);
        self.orientation = quat_mul(quat_mul(turned, rot(0, ax)), rot(2, az));
        for (a, &d) in self.shake.iter_mut().zip(&self.shake_decay) {
            let next = if *a < 0 {
                a.wrapping_add(i16::from(d))
            } else {
                a.wrapping_sub(i16::from(d))
            };
            // a wind down that crosses zero stops the shake on that axis
            *a = if (next ^ *a) < 0 { 0 } else { next };
        }
    }

    fn follow_heading(&mut self, f: &Frame) {
        let t = f.tripod;
        let d = sub(self.target, self.previous_target);
        if t.bailing {
            (self.vert_latch, self.bump[0], self.tracking) = (false, 0, false);
        }
        let up = if self.mode == 0x19 {
            [0, -0x1000, 0]
        } else {
            t.up
        };
        let uy = dot3([0, 0x1000, 0], up).abs();
        let (ang, _) = angles([0; 3], d);
        let dir = direction(0x1000, ang);
        let du = dot3(dir, up).abs();
        self.since_tracking += 1;
        if !matches!(t.state, 1..=3) {
            self.vert_latch = false;
        }
        let vert_surface = uy < VERT_UP && (t.state == 2 || t.state == 8 || self.vert_latch);
        if vert_surface || (self.tracking && t.state == 1) {
            self.vert_latch = true;
            self.tracking_frames += 1;
            self.heading_target = if self.tracking_frames >= 4 {
                [0, 0x100_0000, 0]
            } else {
                dir
            };
            self.tracking = true;
            self.since_tracking = 0;
        } else {
            if (du as u32 & 0xffff_f000) <= 0x7d_0000 && length12(d) >= 5 {
                self.heading_target = dir;
            }
            self.tracking_frames = 0;
            if uy < VERT_UP && self.tracking {
                self.since_tracking = 0;
            }
            self.tracking = false;
        }
        self.previous_heading = self.heading;
        if self.target != self.previous_target && !self.settled {
            self.heading = std::array::from_fn(|i| {
                self.previous_heading[i].wrapping_add(self.heading_target[i]) >> 1
            });
        } else {
            self.heading = self.previous_heading;
        }
        if self.calls < SNAP_CALLS {
            let v = t.forward.map(|c| c.wrapping_neg() << 12);
            (self.heading, self.previous_heading, self.heading_target) = (v, v, v);
        }
        self.heading_target = self.heading;
        let h = self.heading_target.map(|c| (c >> 12) as i16);
        let u = up.map(|c| c.wrapping_neg() as i16);
        let s = outer(u, h).map(|c| (c >> 12) as i16);
        let fw = outer(s, u).map(|c| (c >> 12) as i16);
        let basis = [s[0], u[0], fw[0], s[1], u[1], fw[1], s[2], u[2], fw[2]];
        self.desired = quat_mul(matrix_to_quat(&basis), rot(1, self.yaw));
    }

    fn follow(&mut self, f: &Frame, rate: i32) -> Result<(), String> {
        let t = f.tripod;
        self.previous = self.orientation;
        let old = self.position;
        if t.state == 0 || t.state == 4 {
            let mut v = t.velocity;
            self.history.copy_within(0..5, 1);
            if length12(v) > 100 {
                v = [0x64000, 0, 0];
            }
            self.history[0] = sqrt_int(dot3(v, v)) << 6;
        }
        let sum = self.history.iter().fold(0i32, |a, h| a.wrapping_add(*h));
        self.speed = ((sum / 204 * 30 * 3600) / 5278) >> 12;
        self.distance = (self.distance + self.speed * 4 + f.distance) >> 1;
        let mut dz = -(self.distance << 12);
        if self.tracking_frames >= 4 {
            let q = quat_mul(self.desired, rot(0, -450));
            let o = mat_apply(&quat_to_matrix(q), [0, 0, -(f.distance << 12)]);
            if f.world
                .query_camera(self.target, add(self.target, o), true)?
                .is_none()
            {
                self.desired = q;
            }
        }
        if (!self.tracking || !t.trick_running) && self.bump[0] < 1 {
            let mut ramping = false;
            if self.ramp > 0 {
                self.ramp -= 1;
                ramping = true;
            }
            self.desired = quat_mul(self.desired, rot(0, f.pitch));
            self.feel(f)?;
            if t.jumped {
                if t.state == 3 {
                    ramping = true;
                    self.ramp = 20;
                } else if t.state == 1 {
                    self.desired = quat_mul(self.desired, rot(0, 200));
                }
            }
            let blend = if 1 < self.since_tracking && self.since_tracking < 20 {
                (f.step * 711) >> 8
            } else if ramping {
                (f.step * ((20 - self.ramp) * 260 / 20 + 51)) >> 8
            } else {
                ((f.step as u32).wrapping_mul(rate as u32) >> 8) as i32
            };
            self.orientation = slerp(self.previous, self.desired, blend);
        } else {
            self.feel(f)?;
            let n = self.bump[0];
            if n == 0 {
                self.bump[2] = self.speed;
            }
            self.bump[2] = self.bump[2].min(30);
            let scale = self.bump[2];
            let bump = |a: i32, b: i32, dz: &mut i32, height: &mut V3| {
                height[1] += scale * a / 30;
                *dz += scale * b / 30;
            };
            let height = &mut self.height;
            if self.tracking && !(self.vert_latch && !matches!(t.state, 1 | 2)) {
                self.bump[1] = 0;
                if n >= 7 {
                    bump(392_000, 882_000, &mut dz, height);
                    self.bump[3] += 1;
                } else if n >= 0 {
                    self.bump[3] = 0;
                    bump(8000 * n * n, 18000 * n * n, &mut dz, height);
                }
            } else {
                let k = if n < 1 {
                    0
                } else if self.bump[1] == 0 {
                    let k = n.min(7);
                    self.bump[1] = k + 1;
                    k
                } else {
                    self.bump[1] - 1
                };
                if k == 0 {
                    (self.bump[1], self.bump[0], self.bump[3]) = (1, -1, 0);
                }
                self.bump[1] -= 1;
                bump(8000 * k * k, 18000 * k * k, &mut dz, height);
            }
            self.orientation = slerp(self.previous, self.desired, (f.step * 222) >> 8);
            self.bump[0] += 1;
        }
        if self.calls < SNAP_CALLS {
            self.orientation = self.desired;
            self.previous = self.orientation;
        }
        let m = quat_to_matrix(self.orientation);
        self.height_offset = mat_apply(&m, self.height);
        self.position = add(self.target, mat_apply(&m, [0, 0, dz]));
        self.velocity = sub(self.position, old);
        Ok(())
    }

    fn write_view(&mut self, t: &Tripod) {
        if t.state != 5 {
            self.eye = add(self.height_offset, self.position).map(|c| c >> 12);
            self.wall_orientation = self.orientation;
            self.wall_offset = sub(self.eye.map(|c| c << 12), self.target);
        } else {
            self.eye = add(self.wall_offset, self.target).map(|c| c >> 12);
            self.orientation = self.wall_orientation;
        }
        self.look_at = add(self.look, self.height).map(|c| c >> 12);
        self.matrix = quat_to_matrix(self.orientation);
    }
}

/// Average of a feeler's value and the one before its last probe.
fn avg(pair: [i16; 2]) -> i16 {
    ((i32::from(pair[0]) + i32::from(pair[1])) >> 1) as i16
}

/// How deep into the segment a probe hit, squared, Q24.
fn closeness(hit: &crate::collision::Hit) -> i32 {
    let length = hit.length_units.max(1);
    let v = ((length - hit.distance_units) << 12) / length;
    v.wrapping_mul(v)
}

impl Camera {
    /// World probes around the camera bend the desired orientation away from walls and the floor,
    /// one side probe or the down probe per call in turn.
    fn feel(&mut self, f: &Frame) -> Result<(), String> {
        if self.calls <= SNAP_CALLS {
            return Ok(());
        }
        let t = f.tripod;
        let world = f.world;
        let m = quat_to_matrix(self.desired);
        let fl = &mut self.feelers;
        fl.index = fl.index.wrapping_add(1);
        if fl.index > 2 {
            fl.index = 0;
            fl.wraps += 1;
        }
        let reach = f.distance;
        if fl.index == 2 {
            fl.down[1] = fl.down[0];
            match world.query_camera(self.position, add(self.position, [0, 0xbb_8000, 0]), false)? {
                Some(hit) => {
                    fl.ground = hit.distance_units;
                    fl.down[0] = if hit.distance_units < 120 {
                        let v = ((120 - hit.distance_units) << 12) / 120;
                        ((((v * v) >> 12) << 9) >> 12) as i16
                    } else {
                        0
                    };
                }
                None => {
                    fl.down[0] = if t.state == 1 && !t.vert {
                        0x200
                    } else {
                        fl.down[1]
                    };
                    fl.ground = -1;
                }
            }
            if t.vert {
                let o = mat_apply(&m, [0, reach << 13, 0]);
                fl.down[0] = 0;
                match world.query_camera(self.position, add(self.position, o), true)? {
                    Some(hit) => {
                        fl.ground = hit.distance_units;
                        fl.down[0] = (((closeness(&hit) >> 12) * 300) >> 12) as i16;
                    }
                    None => fl.ground = reach,
                }
            }
        }
        let r = mat_apply(&m, [-(reach << 13), 0, 0]);
        let side_probe = |toward: V3| -> Result<Option<i32>, String> {
            let (start, end) = if fl_wraps_odd(self.feelers.wraps) {
                (self.position, add(self.position, toward))
            } else {
                let mid: V3 =
                    std::array::from_fn(|i| self.position[i].wrapping_add(self.target[i]) >> 1);
                (self.target, add(mid, toward))
            };
            let hit = world.query_camera(start, end, true)?;
            Ok(hit.map(|h| (i32::from(SIDE_GAIN) * (closeness(&h) >> 12)) >> 12))
        };
        if self.feelers.index == 0 {
            self.feelers.side_a[1] = self.feelers.side_a[0];
            self.feelers.side_a[0] = side_probe(r)?.unwrap_or(0) as i16;
        }
        if self.feelers.index == 1 {
            self.feelers.side_b[1] = self.feelers.side_b[0];
            self.feelers.side_b[0] =
                side_probe(r.map(|c| c.wrapping_neg()))?.map_or(0, |v| -v) as i16;
        }
        let back = mat_apply(&m, [0, 0, -(reach << 12)]);
        let side = mat_apply(&m, [0x64000, 0, 0]);
        let fl = &mut self.feelers;
        if self.bump[0] <= 0 && !t.vert {
            fl.rear[1] = fl.rear[0];
            let probe = |start: V3, end: V3| -> Result<u32, String> {
                Ok(world
                    .query_camera(start, end, true)?
                    .map_or(0, |h| closeness(&h) as u32 >> 12))
            };
            let half = side.map(|c| c >> 1);
            let twice_back = add(back, back);
            let mut ka = probe(
                add(self.target, half),
                sub(add(self.target, twice_back), side),
            )?;
            let mut kb = probe(
                sub(self.target, half),
                add(add(self.target, side), twice_back),
            )?;
            if t.state == 4 {
                (ka, kb) = (0, 0);
            }
            let mut gain = i32::from(REAR_GAIN);
            fl.rear[0] = 0;
            if kb != 0 || ka != 0 {
                let (left, right) = (i32::from(avg(fl.side_a)), i32::from(avg(fl.side_b)));
                if right != 0 && -right > left {
                    gain = (kb as i32).wrapping_mul(i32::from(REAR_GAIN)) >> 12;
                    fl.rear[0] = (-gain) as i16;
                }
                if left != 0 && left > -right {
                    fl.rear[0] = (gain.wrapping_mul(ka as i32) >> 12) as i16;
                }
            }
        }
        if self.bump[0] >= 1 || t.vert {
            fl.rear = [0, 0];
        }
        let (down, left, right, rear) =
            (avg(fl.down), avg(fl.side_a), avg(fl.side_b), avg(fl.rear));
        let mut desired = self.desired;
        if down != 0 {
            desired = quat_mul(desired, rot(0, i32::from(down)));
        }
        if left != 0 {
            desired = quat_mul(desired, rot(1, i32::from(left)));
        }
        if right != 0 {
            desired = quat_mul(desired, rot(1, i32::from(right)));
        }
        if rear != 0 {
            desired = quat_mul(rot(1, i32::from(rear)), desired);
        }
        if self.latch {
            let up = mat_apply(&m, [0, 0x1000, 0]);
            let forward = mat_apply(&quat_to_matrix(self.latched), [0, 0, 0x1000]);
            if fl.ground < 0 || dot3(up, forward).abs() > 0x76c {
                self.latch = false;
            }
        }
        self.settled = false;
        let moving = self.speed as u32 >= 4 && self.target != self.previous_target && !t.bailing;
        if moving {
            self.latch = false;
            self.desired = desired;
            return Ok(());
        }
        if !self.latch && self.feelers.ground >= 0 {
            self.latched = slerp(self.orientation, desired, ((f.step * 3) << 9) >> 8);
            self.latch = true;
        }
        if f.time_up || !self.latch {
            self.desired = desired;
            return Ok(());
        }
        if left == 0 && right == 0 && rear == 0 {
            self.settled = t.state != 8 && !t.bailing;
            self.latched = slerp(self.orientation, desired, (f.step * 0x5e7) >> 8);
        }
        desired = self.latched;
        if right != 0 && left == 0 && rear == 0 {
            desired = quat_mul(desired, rot(1, i32::from(right)));
        } else if right == 0 && left != 0 && rear == 0 {
            desired = quat_mul(desired, rot(1, i32::from(left)));
        }
        self.desired = desired;
        Ok(())
    }
}

fn fl_wraps_odd(wraps: i32) -> bool {
    wraps & 1 != 0
}
