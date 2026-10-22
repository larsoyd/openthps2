use super::*;
use crate::rail::{Sweep, norm};

/// The ledge probes run across the rail forty units above and below the skater.
const LEDGE_PROBE: i32 = 0x28000;

const RAIL_STAND: i32 = 0x19000;
const RAIL_GRAVITY: i32 = 13000;
const HEAT_STEP: i32 = 0x28000;
const STAT_RAIL: usize = 7;
const STEPS_PER_FRAME: usize = 5;
const BALANCE_FALL: i32 = 4000;
const RAIL_POINT_SPAN: i32 = 0x1e;
const NO_STICK_FRAMES: i32 = 5;
const BLOCK_FRAMES: i32 = 15;
/// A rail ollie keeps rails off for as long as it keeps the skater from sticking.
const RAIL_OLLIE_BLOCK: i32 = 5;
const RAIL_OLLIE_STEER: i32 = 200;
const LIP_NO_STICK: i32 = 5;
const SPARKS: usize = 9;
const SPARK_DRAWS: usize = 5;
/// How long after its press a buffered grind move can still start, in frame halves.
const GRIND_TRICK_LIFE: i32 = 0x1d;
const WALL_LOOK: i32 = 0x78;

fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}

fn flat(v: V3) -> V3 {
    norm([v[0], 0, v[2]])
}

impl Skater {
    /// End of frame rail check: the search, the stick timers and the rail start.
    pub(super) fn try_grind(
        &mut self,
        level: &Level,
        rules: Rules,
        pad: Pad,
    ) -> Result<(), String> {
        self.search_rails(level, pad)?;
        if self.grind.block != 0 {
            self.grind.block -= 1;
        }
        if self.grind.no_stick > 0 {
            self.grind.on_rail = false;
            self.grind.no_stick -= 1;
            self.grind.found = None;
            return Ok(());
        }
        if !pad.grind || self.bailing || (self.state == State::Vert && self.velocity[1] > 0) {
            self.grind.found = None;
            return Ok(());
        }
        if self.grind.found.is_none() {
            return Ok(());
        }
        if self.grind.on_rail {
            self.grind.found = None;
            return Ok(());
        }
        if self.state == State::Ground {
            (self.spins, self.spin_angle) = (0, 0);
        }
        if self.trick_running {
            return Err("rail start while a trick script runs unsupported".into());
        }
        let lip = self.velocity[1] < 0
            && self.up[1].abs() < 3000
            && self.forward[1] > 0x800
            && self.last_normal[1].abs() < 0x5dc
            && self.lateral[1].abs() < 0x800;
        self.match_keys(level, rules, pad, self.frame_time)?;
        if !self.combo {
            self.grind.found = None;
            return Ok(());
        }
        self.start_grind(level, rules, pad, lip)
    }

    /// The handplant hold: the heading tips down a little until near level, then up and the
    /// lateral axis are rebuilt around it.
    pub(super) fn handplant(&mut self) {
        self.grind.no_stick = LIP_NO_STICK;
        self.frame_accel = [0; 3];
        if self.forward[1] > 0xda7 {
            self.forward[1] += 300;
        }
        self.forward = norm(self.forward);
        self.up = norm(cross_q12(short(self.lateral), short(self.forward)));
        self.lateral = cross_q12(short(self.forward), short(self.up));
    }

    /// A lip trick on a rail linked to a trick object scores the gap a command point ends there.
    fn check_lip_object(&self, level: &Level) -> Result<(), String> {
        let t = level.triggers;
        let start = self.grind.start;
        let mut node = start;
        let object = loop {
            let links = t.links(node);
            if let Some(&o) = links
                .iter()
                .take(2)
                .find(|&&l| matches!(t.kind(l), 0xc | 0xe))
            {
                break o;
            }
            match level.rails.next(node) {
                Some(n) if n != start => node = n,
                _ => return Ok(()),
            }
        };
        let Some(checksum) = t.checksum(object) else {
            return Ok(());
        };
        let ends_here = (0..t.nodes.len()).find(|&i| {
            t.is_command_point(i)
                && t.nodes[i]
                    .raw
                    .windows(4)
                    .any(|w| w == checksum.to_le_bytes())
        });
        match ends_here {
            Some(i) => Err(format!(
                "lip gap of command point={i} at trick object={object} unsupported"
            )),
            None => Ok(()),
        }
    }

    fn search_rails(&mut self, level: &Level, pad: Pad) -> Result<(), String> {
        if !pad.grind || self.grind.on_rail || self.bailing {
            return Ok(());
        }
        let sweep = Sweep {
            old: self.old_position,
            new: self.position,
            velocity: self.velocity,
            forward: self.forward,
            below: matches!(self.state, State::Ground | State::Vert),
            blocked: self.grind.last_rail.filter(|_| self.grind.block > 0),
        };
        level
            .rails
            .search(level.world, &sweep, &mut self.grind.found)
    }

    /// Onto the rail point: the grind script by approach and pad, the velocity along the rail plus
    /// a push from the speed stat.
    fn start_grind(
        &mut self,
        level: &Level,
        rules: Rules,
        pad: Pad,
        lip: bool,
    ) -> Result<(), String> {
        let hit = self.grind.found.take().unwrap();
        let travel = hit.direction.map(|c| -c);
        self.set_anim(level.table, ANIM_AIR)?;
        if lip {
            (self.turn_on_anim, self.switch_on_anim, self.armed) = (false, false, false);
        }
        self.grind.latch = false;
        self.position = [hit.point[0], hit.point[1] - RAIL_STAND, hit.point[2]];
        self.grind.on_rail = true;
        (self.grind.counted_from, self.grind.entry) = (self.position, self.position);
        if self.grind.last_balanced == Some(hit.start) {
            self.grind.heat += HEAT_STEP;
        } else {
            self.grind.heat = (self.grind.heat - HEAT_STEP).max(0);
            self.grind.last_balanced = Some(hit.start);
            self.grind.rate = 0;
        }
        let r = rules.stat(STAT_RAIL, self.stance());
        self.trick_running = true;
        self.grind.rate = (((300000 - 1800 * r) / 10000) * self.grind.rate / 100)
            * ((self.grind.heat >> 12) + 60)
            / (((r + 350) * 2000) / 10000);
        let t = sub(hit.point, self.ground_start);
        let perp = [t[2], 0, -t[0]];
        let across = dot_q12(travel, self.lateral);
        if self.grind.last_rail != Some(hit.start) {
            self.grind.outer = dot_q12(travel, perp) > 0;
        }
        let square = across.abs() < 0xb33;
        let ahead = if square {
            dot_q12(travel, self.forward) > 0
        } else {
            across < 0
        };
        self.grind.last_rail = Some(hit.start);
        let index = usize::from(self.grind.outer)
            + 2 * (usize::from(square)
                + 2 * (usize::from(ahead)
                    + 2 * (usize::from(self.switched)
                        + 2 * usize::from(crate::input::direction(pad)))));
        self.grind.board = 1;
        let side = i32::from(hit.side);
        self.grind.ledge_branch =
            side != 0 && (((1 - hit.sign) ^ side) >> 1 ^ i32::from(self.grind.outer)) != 0;
        (self.grind.start, self.grind.segment) = (hit.start, hit.segment);
        self.grind.side = hit.side;
        self.grind.contact = hit.point;
        self.grind.sparks = false;
        let queued = self
            .grind_trick
            .filter(|t| self.frame_time - t.pressed <= GRIND_TRICK_LIFE && (lip || !t.lip_only));
        if let Some(t) = queued {
            (self.trick_button, self.trick_pressed) = (manual::GRIND_BUTTON, t.pressed);
            self.start_script(t.script);
            self.grind_trick = None;
        } else {
            self.start_script(level.tricks.grind_script(index)?);
        }
        self.run_script(level.table, level.tricks)?;
        if self.state == State::Lip {
            return self.check_lip_object(level);
        }
        self.set_state(State::Grind);
        let along = dot_q12(travel, self.velocity);
        let k = ((rules.stat(STAT_SPEED, self.stance()) + 100) * 5000) / 10000;
        self.velocity = std::array::from_fn(|i| mul_q12(travel[i], along) + travel[i] * k / 10);
        (self.grind.kind, self.grind.direction) = (hit.kind, hit.direction);
        Ok(())
    }

    /// One rail frame: slide along up to five segments, then the board heading, balance and lean.
    pub(super) fn grind_frame(
        &mut self,
        level: &Level,
        rules: Rules,
        frame: FrameScale,
        pad: Pad,
    ) -> Result<(), String> {
        self.grind_time = self.now;
        if self.grind.sparks {
            // sparks are not drawn but their draws come first, the original skips them only with
            // 250 particles alive
            for _ in 0..SPARKS * SPARK_DRAWS {
                self.rng.next(1);
            }
        }
        let rails = level.rails;
        let mut accel = [0; 3];
        self.count_rail_points();
        let mut first = true;
        let mut changed = false;
        let mut left = false;
        let mut slide = 0;
        let mut d = [0; 3];
        for step in 0..=STEPS_PER_FRAME {
            if step == STEPS_PER_FRAME {
                left = true;
                break;
            }
            let seg = self.grind.segment;
            let next = rails
                .next(seg)
                .ok_or_else(|| format!("rail node={seg} without a next segment"))?;
            let (s0, s1) = (rails.position(seg), rails.position(next));
            d = norm(sub(s1, s0));
            let axis = if d[0].abs() < d[1].abs() {
                if d[1].abs() < d[2].abs() { 2 } else { 1 }
            } else if d[0].abs() < d[2].abs() {
                2
            } else {
                0
            };
            slide = dot_q12(d, self.velocity);
            if first {
                first = false;
                accel[1] += RAIL_GRAVITY;
                if slide == 0 {
                    slide = 1000;
                }
                if slide.abs() < 60000 {
                    slide = if d[1] == 0 {
                        slide * 5 / 4
                    } else {
                        slide + if d[1] >= 0 { 2000 } else { -2000 }
                    };
                }
            }
            let a = dot_q12(d, accel);
            accel = d.map(|c| mul_q12(c, a));
            self.velocity = d.map(|c| mul_q12(c, slide));
            let length = ((s1[axis] - s0[axis]) / d[axis]) << 12;
            let mut t = ((self.position[axis] - s0[axis]) / d[axis]) * 0x1000 + slide;
            if t >= 0 {
                if t <= length {
                    let half = frame_part(accel, frame.scale_squared)?.map(|c| c / 2);
                    self.position = add(
                        self.position,
                        add(frame_part(self.velocity, frame.scale)?, half)?,
                    )?;
                    break;
                }
                let turn_ok = rails.next(next).is_some_and(|after| {
                    dot_q12(flat(d), flat(sub(rails.position(after), s1))) > 0xa48
                });
                if !turn_ok {
                    left = true;
                    break;
                }
                self.grind.segment = next;
                self.grind.kind = rails.kind(next);
                self.position = [s1[0], s1[1] - RAIL_STAND, s1[2]];
                t -= length;
            } else {
                let Some(before) = rails.prev_in(seg, self.grind.start) else {
                    left = true;
                    break;
                };
                if dot_q12(d, norm(sub(s0, rails.position(before)))) < 0xa49 {
                    left = true;
                    break;
                }
                self.grind.segment = before;
                self.grind.kind = rails.kind(before);
                self.position = [s0[0], s0[1] - RAIL_STAND, s0[2]];
            }
            changed = true;
            if t == 0 {
                break;
            }
        }
        if left {
            self.leave_rail(level.table)?;
        }
        self.grind.direction = if slide < 0 { d } else { d.map(|c| -c) };
        let heading = self.grind.direction.map(|c| c * self.grind.board);
        if dot_q12(heading, self.forward) < 0 && self.old_position != self.grind.entry {
            self.grind.board = -self.grind.board;
            self.switch_on_anim = !self.switch_on_anim;
            self.turn_on_anim = !self.turn_on_anim;
        }
        self.forward = self.grind.direction.map(|c| c * self.grind.board);
        self.rail_basis();
        if self.balance(level, rules, frame, pad)? {
            self.frame_accel = accel;
            return Ok(());
        }
        let lean = (self.grind.angle / 10) & 0xfff;
        self.set_basis(mat_mul_cols(self.basis_matrix(), rot_z_matrix(lean)));
        (self.lateral, self.up, self.forward) =
            (short(self.lateral), short(self.up), short(self.forward));
        if self.grind.on_rail && self.grind.heat & !0xfff > 0x2000 && !changed {
            self.rail_wall(level)?;
        }
        self.frame_accel = accel;
        Ok(())
    }

    /// Thirty units of rail add the script's rail points to the trick on top of the stack.
    fn count_rail_points(&mut self) {
        let gap = sub(self.position, self.grind.counted_from).map(|c| c >> 12);
        let length = sqrt_int(gap.iter().map(|&c| c * c).sum());
        if length > RAIL_POINT_SPAN {
            if let Some(last) = self.panel.entries.last_mut() {
                last.points += self.points_rate;
                self.refresh_spins();
                self.text_events.push(TextEvent::Lines(self.shown()));
            }
            self.grind.counted_from = self.position;
        }
    }

    /// Up straight, the lateral axis across the rail heading, then up again from both.
    fn rail_basis(&mut self) {
        self.up = WORLD_UP;
        self.lateral = norm(cross_q12(short(self.forward), short(self.up)));
        self.up = cross_q12(short(self.lateral), short(self.forward));
        (self.lateral, self.up, self.forward) =
            (short(self.lateral), short(self.up), short(self.forward));
    }

    /// The balance angle swings with the rate, the rate drifts at random and follows left and right.
    /// Returns true when the skater fell off the rail onto a ledge, which ends the frame.
    fn balance(
        &mut self,
        level: &Level,
        rules: Rules,
        frame: FrameScale,
        pad: Pad,
    ) -> Result<bool, String> {
        let r = rules.stat(STAT_RAIL, self.stance());
        let g = &mut self.grind;
        let heat = (g.heat >> 12) + 60;
        let damp = ((r + 350) * 2000) / 10000;
        g.angle += (g.angle / ((r * 0x834 + 400000) / 10000) * heat / damp * frame.scale) >> 8;
        g.angle += (heat * g.rate / damp * frame.scale) >> 8;
        let u = g.rate.abs();
        g.rate += (self.rng.next(u) - u / 2) / 10;
        let g = &mut self.grind;
        if !pad.left && !pad.right {
            g.latch = true;
        }
        let nudge = (g.board * frame.scale * 20) >> 8;
        if pad.left && g.latch {
            g.rate -= nudge;
        } else if pad.right && g.latch {
            g.rate += nudge;
        } else if g.rate.abs() < ((200 - r) * 1000) / 10000 {
            g.rate = self.rng.next((800000 - 0xce4 * r) / 10000) + (0xce4 * r - 800000) / 20000;
        }
        if self.grind.angle.abs() <= BALANCE_FALL {
            return Ok(false);
        }
        let angle = self.grind.angle;
        let seg = self.grind.segment;
        let next = level
            .rails
            .next(seg)
            .ok_or_else(|| format!("rail node={seg} without a next segment"))?;
        let along = dot_q12(
            norm(sub(level.rails.position(next), level.rails.position(seg))),
            self.velocity,
        );
        if self.grind.side != 0 && (i32::from(self.grind.side) - 2) ^ angle ^ along < 0 {
            let across = self
                .lateral
                .map(|c| c * 20 * if angle >= 0 { -1 } else { 1 });
            let probe = |y: i32| {
                let p = self.position;
                (
                    [p[0] + across[0], p[1] + y, p[2] + across[2]],
                    [p[0] - across[0], p[1] + y, p[2] - across[2]],
                )
            };
            // the original probe also crosses the faces of created objects, which no query here sees
            let (start, end) = probe(-LEDGE_PROBE);
            if let Some(above) = level.world.query_filtered(start, end, false)? {
                let (start, end) = probe(LEDGE_PROBE);
                let hit = level
                    .world
                    .query_filtered(start, end, false)?
                    .unwrap_or(above);
                self.ledge_drop(level.table, angle, hit)?;
                return Ok(true);
            }
        }
        let kind = usize::try_from(self.rail_bail)
            .map_err(|_| format!("rail bail kind={} out of range", self.rail_bail))?;
        self.rail_bail = 0;
        self.start_bail(level.table, level.tricks, kind, false)?;
        Ok(false)
    }

    /// Turned an eighth toward the ledge, fitted to the face the probes found and sent back along
    /// the new heading at the same speed.
    fn ledge_drop(&mut self, table: &AnimationTable, angle: i32, hit: Hit) -> Result<(), String> {
        let yaw = if angle >= 0 { 0x200 } else { 0xe00 };
        self.set_basis(mat_mul_cols(self.basis_matrix(), rot_y_matrix(yaw)));
        let n = hit.normal.map(i32::from);
        (self.last_normal, self.smoothed_up, self.same_normal_time) = (n, n, 0);
        self.smooth_basis();
        let speed = speed_of(self.velocity)?;
        self.velocity = self.forward.map(|f| -mul_q12(f, speed));
        self.position = add(hit.position_fixed, self.up.map(|c| c * 30))?;
        self.exit_script = None;
        self.leave_rail(table)?;
        self.set_state(State::Ground);
        Ok(())
    }

    /// A wall ahead on the rail ends the grind where the frame began.
    fn rail_wall(&mut self, level: &Level) -> Result<(), String> {
        if self.position == self.old_position {
            return Ok(());
        }
        let ahead = self.grind.direction.map(|c| -c * WALL_LOOK);
        let mut start = self.old_position;
        let mut end = add(self.position, ahead)?;
        start[1] -= 0x32000;
        end[1] -= 0x32000;
        if level.world.query(start, end)?.is_some() {
            self.position = self.old_position;
            self.leave_rail(level.table)?;
        }
        Ok(())
    }

    /// Jumping off a rail keeps only the speed along it, faces down the rail and steers by left
    /// or right through the last kept frame basis.
    pub(super) fn ollie_off_rail(&mut self, pad: Pad) -> Result<(), String> {
        let g = &mut self.grind;
        (g.on_rail, g.found, g.no_stick, g.block) =
            (false, None, NO_STICK_FRAMES, RAIL_OLLIE_BLOCK);
        (self.trick_running, self.script) = (false, None);
        self.forward = self.grind.direction;
        self.rail_basis();
        let along = dot_q12(self.grind.direction, self.velocity);
        self.velocity = self.grind.direction.map(|c| mul_q12(c, along));
        let steer = if pad.right {
            RAIL_OLLIE_STEER
        } else if pad.left {
            -RAIL_OLLIE_STEER
        } else {
            0
        };
        self.rotate(self.frame_basis, steer)
    }

    /// Off the rail into the air, a blocked rail and the script's exit move or the air animation.
    pub(super) fn leave_rail(&mut self, table: &AnimationTable) -> Result<(), String> {
        self.set_state(State::Air);
        self.grind.no_stick = NO_STICK_FRAMES;
        self.grind.block = BLOCK_FRAMES;
        self.grind.on_rail = false;
        self.grind.found = None;
        self.jumped = true;
        self.trick_running = false;
        (self.spins, self.spin_angle) = (0, 0);
        match self.exit_script.take() {
            Some(pc) => self.start_script(pc),
            None => {
                self.set_anim(table, ANIM_AIR)?;
                self.script = None;
            }
        }
        Ok(())
    }
}
