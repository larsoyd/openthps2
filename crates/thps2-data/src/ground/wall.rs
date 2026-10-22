use super::*;

const WALL_GRAVITY: i32 = 9000;
const WALL_POINTS: i32 = 500;
const WALL_RATE: i32 = 10;
const ANIM_WALL_LEFT: u16 = 0x23;
const ANIM_WALL_RIGHT: u16 = 0x24;
/// Faces pointing down past about 37 degrees cannot be ridden.
const OVERHANG: i32 = 0xccd;
/// Heading within 30 degrees of the wall normal hits it head on.
const HEAD_ON: i32 = 0xddb;
const RIDE_UPRIGHT: i32 = -0x5dc;
const WALL_ALIGN_EXTRA: i32 = 4;
const WALL_REACH: i32 = 0x74;

impl Skater {
    /// A wall hit with the grind button held rides the wall, unless the approach is head on,
    /// the face overhangs or the skater is tipped.
    pub(super) fn enter_wall(&mut self, level: &Level, hit: &Hit) -> Result<(), String> {
        let (world, table, scripts) = (level.world, level.table, level.tricks);
        let n = hit.normal.map(i32::from);
        let heading = normalize([self.velocity[0], 0, self.velocity[2]]);
        let flat_normal = normalize([n[0], 0, n[2]]);
        if n[1] > OVERHANG
            || dot_q12(heading, flat_normal).abs() >= HEAD_ON
            || self.up[1] >= RIDE_UPRIGHT
        {
            self.position = self.old_position;
            return self.start_bail(table, scripts, 0, false);
        }
        if self.trick_running && !self.script_ended {
            return Err("wallride during a trick move bail unsupported".into());
        }
        self.script_ended = true;
        self.set_state(State::Wall);
        self.push_entry(Entry {
            trick: true,
            switched: self.switched,
            ..Entry::new("WALL RIDE", WALL_POINTS, 0, 1)
        });
        self.wall_time = self.frame_time;
        let target = add(hit.position_fixed, scaled(n, STANDING_HEIGHT)?)?;
        self.position = world.move_retrying(self.position, target)?;
        self.fakie_flip();
        self.fit_basis(n);
        self.fakie_flip();
        (self.last_normal, self.smoothed_up, self.same_normal_time) = (n, n, 0);
        self.velocity = project_onto_plane(self.velocity, n);
        self.script = None;
        let along = cross_q12(WORLD_UP, n);
        let ahead = dot_q12(along, self.velocity) >= 0;
        let id = if ahead == self.switched {
            ANIM_WALL_LEFT
        } else {
            ANIM_WALL_RIGHT
        };
        self.set_anim(table, id)
    }

    fn land_from_wall(&mut self, n: V3) {
        self.set_state(State::Ground);
        self.fit_basis(n);
        (self.last_normal, self.smoothed_up, self.same_normal_time) = (n, n, 0);
        self.velocity = project_onto_plane(self.velocity, n);
    }

    /// Forward along the face, up out of it.
    pub(super) fn fit_basis(&mut self, n: V3) {
        self.forward = normalize(project_onto_plane(self.forward, n));
        self.up = n;
        self.lateral = short(cross_q12(short(self.forward), short(n)));
    }

    /// Leaving a wall for the air turns the board back upright about its forward axis.
    pub(super) fn leave_wall_basis(&mut self) {
        let (side, up) = (self.lateral, self.up);
        let neg = |v: V3| v.map(|c| -c);
        let flip = (self.anim.id == ANIM_WALL_LEFT) == self.switched;
        (self.lateral, self.up) = if flip {
            (neg(up), side)
        } else {
            (up, neg(side))
        };
    }

    fn snap_to(
        &mut self,
        world: &CollisionWorld,
        hit: &Hit,
        frame: FrameScale,
    ) -> Result<(), String> {
        let target = add(hit.position_fixed, scaled(self.up, STANDING_HEIGHT)?)?;
        self.position = world.move_retrying(self.position, target)?;
        let n = hit.normal.map(i32::from);
        if n == self.last_normal {
            self.same_normal_time += frame.hold_delta;
        } else {
            (self.same_normal_time, self.last_normal) = (0, n);
        }
        self.smooth_basis();
        Ok(())
    }
}

/// The wall handler: glide under wall gravity along the face, follow it with two probes and
/// drop off when it ends.
pub fn wall_step(
    level: &Level,
    s: &mut Skater,
    rules: Rules,
    frame: FrameScale,
) -> Result<Outcome, String> {
    let world = level.world;
    let gravity = [0, WALL_GRAVITY, 0];
    s.add_held_points(WALL_RATE);
    if s.anim.finished && matches!(s.anim.id, ANIM_WALL_LEFT | ANIM_WALL_RIGHT) {
        let m = (s.frame_time / 4) % 4;
        s.anim.frame = (if m < 3 { 7 + m } else { 8 }) as i16;
    }
    s.begin_frame(world)?;
    let old_matrix = s.basis_matrix();
    let half_push = frame_part(s.accel, frame.scale_squared)?.map(|c| c / 2);
    s.position = add(
        s.position,
        add(frame_part(s.velocity, frame.scale)?, half_push)?,
    )?;
    let align = s.fall_line_alignment(rules, WALL_ALIGN_EXTRA)?;
    s.rotate(old_matrix, (frame.scale * align) >> 8)?;
    let mut accel = s.accel;
    let (contact, _) = wall_probe(world, s.old_position, s.position, s.bailing)?;
    if let Some(hit) = contact
        && (hit.face_flags & 0x80 != 0 || hit.flags & LIP_FACE == 0)
    {
        s.snap_to(world, &hit, frame)?;
        if hit.flags & 0x1000000 == 0 {
            s.commit_combo()?;
            s.land_from_wall(hit.normal.map(i32::from));
            s.set_anim(level.table, ANIM_LAND)?;
            s.jumped = false;
        }
    }
    let start = add(s.position, scaled(s.up, 70)?)?;
    let probe = world.query_filtered(start, add(start, scaled(s.up, -0x80)?)?, false)?;
    let Some(hit) = probe.filter(|h| h.flags & 0x800000 == 0) else {
        s.set_state(State::Air);
        s.open_combo();
        (s.spin_angle, s.spins, s.trick_running, s.jumped, s.manual) = (0, 0, false, true, 0);
        if !s.bailing {
            let id = if s.crouching {
                ANIM_CROUCH_FALL
            } else {
                ANIM_FALL
            };
            s.set_anim(level.table, id)?;
        }
        s.finish_frame(world, add(accel, gravity)?, false)?;
        return Ok(Outcome::Airborne);
    };
    let n = hit.normal.map(i32::from);
    if hit.distance_units <= WALL_REACH {
        s.vert = hit.flags & VERT_FACE != 0;
        s.snap_to(world, &hit, frame)?;
        if hit.flags & 0x1000000 == 0 {
            s.land_from_wall(n);
        } else if n[1] > OVERHANG {
            return Err(format!(
                "wall overhang exit unsupported item={} face={}",
                hit.item_index, hit.face_index
            ));
        }
    }
    let along_face = project_onto_plane(gravity, n);
    accel = add(accel, along_face)?;
    s.slope_gravity = along_face;
    s.ground_normal = n;
    s.velocity = reproject(s.velocity, n)?;
    s.finish_frame(world, accel, false)?;
    Ok(if s.state == State::Ground {
        Outcome::Landed(hit)
    } else {
        Outcome::Airborne
    })
}
