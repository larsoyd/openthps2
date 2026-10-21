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
