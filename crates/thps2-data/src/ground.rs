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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Grounded(Hit),
    LeftGround(Option<Hit>),
    Airborne,
    Landed(Hit),
}

type V3 = [i32; 3];

fn trig_q12(angle: i32) -> (i32, i32) {
    let radians = f64::from(angle & 0xfff) * std::f64::consts::TAU / 4096.0;
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
        }
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
    ) -> Result<(), String> {
        self.position = world.move_retrying(self.old_position, self.position)?;
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

fn wall_probe(world: &CollisionWorld, old: V3, new: V3) -> Result<(), String> {
    if old == new {
        return Ok(());
    }
    let delta = std::array::from_fn(|a| new[a] - old[a]);
    let length = speed_of(delta)?;
    let mut end = new;
    if length < WALL_REACH * 4096 {
        let divisor = length.max(0x800);
        end = std::array::from_fn(|a| WALL_REACH * delta[a] / divisor * 4096 + old[a]);
    }
    match world.query_filtered(old, end, false)? {
        Some(hit) => Err(format!(
            "ground wall contact unsupported item={} face={} distance={}",
            hit.item_index, hit.face_index, hit.distance_units
        )),
        None => Ok(()),
    }
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

fn post_contact_probes(world: &CollisionWorld, s: &Skater) -> Result<(), String> {
    let shoulder = add(s.position, scaled(s.up, STANDING_HEIGHT)?)?;
    for direction in [-1, 1] {
        let end = add(shoulder, scaled(s.lateral, direction * BOUNCE_REACH)?)?;
        if let Some(hit) = world.query_filtered(shoulder, end, false)?
            && dot_q12(short(hit.normal.map(i32::from)), s.up).abs() < 0x400
        {
            return Err(format!(
                "ground bounce wall unsupported item={} face={}",
                hit.item_index, hit.face_index
            ));
        }
    }
    if s.up[1] > 1000 {
        return Err("ground upside down bail unsupported".into());
    }
    head_probe(world, s)
}

/// One native ground frame without pad input, bail, manual, triggers or moving objects.
pub fn ground_step(
    world: &CollisionWorld,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
) -> Result<Outcome, String> {
    s.begin_frame(world)?;
    let mut accel = [0; 3];
    s.position = add(s.position, frame_part(s.velocity, frame.scale)?)?;
    if dot_q12(s.forward, s.velocity) > FAKIE_DOT {
        s.forward = s.forward.map(|c| -c);
        s.lateral = s.lateral.map(|c| -c);
        s.switched = !s.switched;
    }
    wall_probe(world, s.old_position, s.position)?;
    s.fast = speed_of(s.velocity)? > stat_speed(rules.speed_stat, 0x186);
    let ignore_lips = s.velocity[1] > 0 || s.forward[1] < 1;
    let start = add(s.position, scaled(s.up, 0x46)?)?;
    let end = add(start, scaled(s.up, -0x100)?)?;
    let hit = world.query_filtered(start, end, ignore_lips)?;
    let mut grounded = false;
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
        post_contact_probes(world, s)?;
    } else {
        accel = add(accel, [0, GRAVITY, 0])?;
    }
    s.state = if grounded { State::Ground } else { State::Air };
    s.finish_frame(world, accel, rules, frame)?;
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
) -> Result<Outcome, String> {
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
    s.finish_frame(world, accel, rules, frame)?;
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
) -> Result<Outcome, String> {
    match s.state {
        State::Ground => ground_step(world, s, rules, frame),
        State::Air => air_step(world, s, rules, frame),
    }
}
