use crate::collision::CollisionWorld;
use crate::trig::Triggers;

const NOT_LINKED: u16 = 0x8000;
const INACTIVE: u16 = 0x40;

/// One rail chain with its bounding box, in world units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rail {
    pub active: bool,
    pub kind: u8,
    pub start: usize,
    pub min: [i32; 3],
    pub max: [i32; 3],
}

#[derive(Clone, Copy, Debug)]
struct RailNode {
    position: [i32; 3],
    flags: u16,
    next: Option<usize>,
}

/// The level's rails, newest first as the game searches them.
pub struct Rails {
    pub rails: Vec<Rail>,
    nodes: Vec<Option<RailNode>>,
}

impl Rails {
    /// Builds one rail per chain from the rail nodes the game mode bits allow.
    pub fn from_triggers(t: &Triggers, mode: u16) -> Result<Self, String> {
        let count = t.nodes.len();
        let point = |i: usize| {
            t.rail_point(i).filter(|&(_, flags)| {
                let bits = (flags >> 4) & 3;
                bits == 0 || bits & mode != 0
            })
        };
        let mut nodes: Vec<Option<RailNode>> = (0..count)
            .map(|i| {
                point(i).map(|(position, flags)| RailNode {
                    position,
                    flags,
                    next: None,
                })
            })
            .collect();
        if nodes.first().is_some_and(Option::is_some) {
            return Err("trigger node 0 is a rail node".into());
        }
        for i in 0..count {
            if nodes[i].is_none() {
                continue;
            }
            let next = t.links(i).into_iter().rev().find(|&l| {
                nodes
                    .get(l)
                    .copied()
                    .flatten()
                    .is_some_and(|n| n.flags & NOT_LINKED == 0)
            });
            nodes[i].as_mut().unwrap().next = next;
        }
        let mut previous = vec![None; count];
        for (i, node) in nodes.iter().enumerate() {
            if let Some(next) = node.and_then(|n| n.next) {
                previous[next] = Some(i);
            }
        }
        let mut visited = vec![false; count];
        let mut rails = Vec::new();
        for i in 0..count {
            let Some(node) = nodes[i] else { continue };
            if visited[i] {
                continue;
            }
            let mut start = i;
            if node.flags & NOT_LINKED == 0 {
                let mut p = previous[i];
                while let Some(prev) = p {
                    let before = previous[prev];
                    if before == Some(i) {
                        start = i;
                        break;
                    }
                    start = prev;
                    if nodes[prev].unwrap().flags & NOT_LINKED != 0 {
                        break;
                    }
                    p = before;
                }
            }
            rails.push(Self::chain(&nodes, &mut visited, start));
        }
        rails.reverse();
        Ok(Self { rails, nodes })
    }

    fn chain(nodes: &[Option<RailNode>], visited: &mut [bool], start: usize) -> Rail {
        let first = nodes[start].unwrap();
        let mut rail = Rail {
            active: first.flags & INACTIVE == 0,
            kind: (first.flags & 0xf) as u8,
            start,
            min: [i32::MAX; 3],
            max: [i32::MIN; 3],
        };
        visited[start] = true;
        let mut node = start;
        loop {
            let n = nodes[node].unwrap();
            for a in 0..3 {
                rail.min[a] = rail.min[a].min(n.position[a]);
                rail.max[a] = rail.max[a].max(n.position[a]);
            }
            match n.next {
                Some(next) if next != start => {
                    visited[next] = true;
                    node = next;
                }
                Some(next) => {
                    visited[next] = true;
                    return rail;
                }
                None => return rail,
            }
        }
    }

    pub fn next(&self, node: usize) -> Option<usize> {
        self.nodes.get(node).copied().flatten()?.next
    }

    /// The node before this one on the chain from start, none at the chain start.
    pub fn prev_in(&self, node: usize, start: usize) -> Option<usize> {
        let mut at = start;
        loop {
            let next = self.next(at)?;
            if next == node {
                return Some(at);
            }
            if next == start {
                return None;
            }
            at = next;
        }
    }

    pub fn position(&self, node: usize) -> [i32; 3] {
        self.nodes[node].map_or([0; 3], |n| n.position)
    }

    pub fn kind(&self, node: usize) -> u8 {
        self.nodes[node].map_or(0, |n| (n.flags & 0xf) as u8)
    }
}

const SHORT_LIMIT: i32 = 0x6883;
const BEST_START: i32 = 100_000_000;
const SWEEP_PAD: i32 = 0x64000;

/// Closest points of the motion segment and a rail segment, with their distance in file units.
pub struct Closest {
    pub q: [i32; 3],
    pub p: [i32; 3],
    pub dist: i32,
}

type V3 = [i32; 3];

fn shr(v: V3, k: u32) -> V3 {
    v.map(|c| c >> k)
}

fn units(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| (a[i].wrapping_sub(b[i])) >> 12)
}

/// Cross product on the sixteen bit registers of the geometry coprocessor.
fn cross(d: V3, ir: V3) -> V3 {
    let (d, ir) = (d.map(|c| c as i16 as i32), ir.map(|c| c as i16 as i32));
    [
        d[1] * ir[2] - d[2] * ir[1],
        d[2] * ir[0] - d[0] * ir[2],
        d[0] * ir[1] - d[1] * ir[0],
    ]
}

fn fit_short(mut v: V3) -> (V3, u32) {
    let mut k = 0;
    while v.iter().any(|c| c.abs() >= SHORT_LIMIT) {
        v = shr(v, 1);
        k += 1;
    }
    (v, k)
}

fn square(v: V3) -> i32 {
    v.iter()
        .fold(0i32, |sum, &c| sum.wrapping_add(c.wrapping_mul(c)))
}

/// Line parameter of one segment, 0 to 0x1000, and the point it gives on that segment.
fn along(n: V3, den0: i32, m: V3, origin: V3, end: V3, dir: V3) -> V3 {
    let (m, j) = fit_short(m);
    let n = shr(n, j);
    let num = (0..3).fold(0i32, |sum, i| {
        sum.wrapping_add((n[i] as i16 as i32).wrapping_mul(m[i] as i16 as i32))
    });
    let den = den0 >> ((2 * j) & 31);
    if num < 0 {
        return origin;
    }
    if num >= den {
        return end;
    }
    let t = if den >= 0x100000 {
        num / (den >> 12)
    } else {
        num.wrapping_shl(12) / den
    };
    std::array::from_fn(|i| {
        ((origin[i] >> 12) + ((dir[i] as i16 as i32 * t as i16 as i32) >> 12)) << 12
    })
}

pub fn closest(a: V3, b: V3, s0: V3, s1: V3) -> Closest {
    let (d, e, f) = (units(b, a), units(s1, s0), units(a, s0));
    let (n, k) = fit_short(cross(d, e));
    let den0 = square(n);
    let q = along(n, den0, shr(cross(e, f), k), a, b, d);
    let p = along(n, den0, shr(cross(d, f), k), s0, s1, e);
    let gap = units(q, p).map(|c| c as i16 as i32);
    Closest {
        q,
        p,
        dist: isqrt(square(gap)),
    }
}

fn isqrt(n: i32) -> i32 {
    if n <= 0 {
        0
    } else {
        f64::from(n).sqrt() as i32
    }
}

/// Unit vector the way the rail code takes it, a zero vector staying zero.
pub fn norm(v: V3) -> V3 {
    let (v, _) = fit_short(v);
    let len = isqrt(square(v));
    if len == 0 {
        v
    } else {
        v.map(|c| (c << 12) / len)
    }
}

fn level(v: V3) -> V3 {
    norm([v[0], 0, v[2]])
}

fn dot(a: V3, b: V3) -> i32 {
    ((0..3)
        .map(|i| i64::from(a[i]) * i64::from(b[i]))
        .sum::<i64>()
        / 4096) as i32
}

/// A rail point within reach of the frame's sweep and the direction the board will slide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RailHit {
    pub start: usize,
    pub segment: usize,
    pub point: V3,
    /// Unit vector against the travel along the rail.
    pub direction: V3,
    pub sign: i32,
    pub kind: u8,
    /// Ledge side from the probes beside the rail, 1 left, 2 right, 0 none or both.
    pub side: u8,
}

/// The skater's sweep this frame, for the rail search.
pub struct Sweep {
    pub old: V3,
    pub new: V3,
    pub velocity: V3,
    pub forward: V3,
    /// On the ground or a lip the rail point must lie below the skater.
    pub below: bool,
    /// The rail just left, blocked for a few frames.
    pub blocked: Option<usize>,
}

impl Rails {
    /// Ranks every rail segment near the sweep, a near miss with a better rank cancelling the find.
    pub fn search(
        &self,
        world: &CollisionWorld,
        sweep: &Sweep,
        found: &mut Option<RailHit>,
    ) -> Result<(), String> {
        let mut best = BEST_START;
        let lo: V3 = std::array::from_fn(|i| sweep.old[i].min(sweep.new[i]) - SWEEP_PAD);
        let hi: V3 = std::array::from_fn(|i| sweep.old[i].max(sweep.new[i]) + SWEEP_PAD);
        let outside = |min: V3, max: V3| (0..3).any(|i| hi[i] < min[i] || lo[i] > max[i]);
        for rail in &self.rails {
            if !rail.active || outside(rail.min, rail.max) || sweep.blocked == Some(rail.start) {
                continue;
            }
            let mut n = rail.start;
            let Some(mut nx) = self.next(n) else { continue };
            loop {
                let (s0, s1) = (self.position(n), self.position(nx));
                let min = std::array::from_fn(|i| s0[i].min(s1[i]));
                let max = std::array::from_fn(|i| s0[i].max(s1[i]));
                if !outside(min, max) {
                    let near = closest(sweep.old, sweep.new, s0, s1);
                    let seg: V3 = std::array::from_fn(|i| s1[i] - s0[i]);
                    let c = dot(level(seg), level(sweep.velocity)).abs();
                    let rank = (0x1032 - c).wrapping_mul(near.dist);
                    let reach = (0x2000 - c).wrapping_mul(near.dist) / 4096;
                    if rank < best && !(sweep.below && near.p[1] <= sweep.new[1]) {
                        if reach > 100 {
                            best = rank;
                            *found = None;
                        } else if let Some(hit) = self.accept(world, rail, n, nx, near.p, sweep)? {
                            best = rank;
                            *found = Some(hit);
                        }
                    }
                }
                if nx == rail.start {
                    break;
                }
                n = nx;
                let Some(next) = self.next(n) else { break };
                nx = next;
            }
        }
        Ok(())
    }

    fn accept(
        &self,
        world: &CollisionWorld,
        rail: &Rail,
        n: usize,
        nx: usize,
        point: V3,
        sweep: &Sweep,
    ) -> Result<Option<RailHit>, String> {
        let (s0, s1) = (self.position(n), self.position(nx));
        let u = norm(std::array::from_fn(|i| s1[i] - s0[i]));
        let mut d = dot(u, sweep.velocity);
        if d.abs() < 0x1000 {
            d = -dot(u, sweep.forward);
        }
        let (direction, sign) = if d <= 0 {
            if point != s1 && point == s0 && self.prev(n).is_none() {
                return Ok(None);
            }
            (u, 1)
        } else {
            if point != s0 && point == s1 && self.next(nx).is_none() {
                return Ok(None);
            }
            (u.map(|c| -c), -1)
        };
        Ok(Some(RailHit {
            start: rail.start,
            segment: n,
            point,
            direction,
            sign,
            kind: self.kind(n),
            side: Self::side(world, s0, s1)?,
        }))
    }

    /// Any rail node whose next is this one.
    fn prev(&self, node: usize) -> Option<usize> {
        (1..self.nodes.len()).find(|&m| self.next(m) == Some(node))
    }

    fn side(world: &CollisionWorld, s0: V3, s1: V3) -> Result<u8, String> {
        let mid: V3 = std::array::from_fn(|i| (s0[i] + s1[i]) / 2);
        let mut c: V3 = std::array::from_fn(|i| s0[i] + (mid[i] - s0[i]) / 2);
        c[1] -= 0xa000;
        let k = level(std::array::from_fn(|i| s1[i] - s0[i]));
        let (px, pz) = (10 * k[2], -10 * k[0]);
        let probe = |x: i32, z: i32, len: i32| -> Result<bool, String> {
            Ok(world.query([x, c[1], z], [x, c[1] + len, z])?.is_some())
        };
        let left = probe(c[0] - 2 * px, c[2] - 2 * pz, 0x1e000)?;
        let right = probe(c[0] + 2 * px, c[2] + 2 * pz, 0x1e000)?;
        if left || right {
            return Ok(if left { 1 } else { 2 });
        }
        let (qx, qz) = (2 * px / 4, 2 * pz / 4);
        let left = probe(c[0] - qx, c[2] - qz, 0xf000)?;
        let right = probe(c[0] + qx, c[2] + qz, 0xf000)?;
        Ok(match (left, right) {
            (false, true) => 2,
            (true, false) => 1,
            _ => 0,
        })
    }
}
