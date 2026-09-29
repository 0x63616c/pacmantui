//! Level-clearing input author (acceptance G11: "complete a level via normal
//! controls or replay of legal controls").
//!
//! HONESTY CONTRACT: the bot reads only player-observable state (the map and
//! `Game::render_state()`) and plays by submitting one `InputFrame` per tick
//! through the ordinary `Game::tick` — exactly what a human at the keyboard
//! provides. No sim internals, no state edits. The produced replay file is
//! bit-replayable through `pacmantui --replay`.
//!
//! Strategy: at every tick, BFS from Pac-Man's tile over walkable tiles
//! (with edge wrap) to the nearest remaining pellet, avoiding tiles near
//! hostile ghosts; chase nearby frightened ghosts; when boxed in, step
//! toward the neighbor that maximizes distance from the nearest hostile.
//!
//! Usage: cargo run --release --example bot_clear -- <classic|custom> \
//!            [seed] [danger-radius] [emit-replay-path]

use std::collections::VecDeque;

use pacmantui::map::Map;
use pacmantui::replay::Replay;
use pacmantui::rules::Rules;
use pacmantui::sim::Game;
use pacmantui::types::{Difficulty, Dir, GhostState, InputFrame, TilePos};

struct Grid {
    w: i32,
    h: i32,
}

impl Grid {
    fn wrap(&self, t: TilePos) -> TilePos {
        TilePos::new(t.x.rem_euclid(self.w), t.y.rem_euclid(self.h))
    }
    fn idx(&self, t: TilePos) -> usize {
        (t.y * self.w + t.x) as usize
    }
}

fn neighbors(map: &Map, grid: &Grid, t: TilePos) -> Vec<(Dir, TilePos)> {
    Dir::IN_PRIORITY_ORDER
        .iter()
        .filter_map(|&d| {
            let (dx, dy) = d.delta();
            let n = grid.wrap(TilePos::new(t.x + dx, t.y + dy));
            map.walkable(n).then_some((d, n))
        })
        .collect()
}

/// BFS to the closest goal tile; returns the first step direction.
fn bfs_first_step(
    map: &Map,
    grid: &Grid,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    blocked: impl Fn(TilePos) -> bool,
) -> Option<Dir> {
    let n = (grid.w * grid.h) as usize;
    let mut prev: Vec<Option<(usize, Dir)>> = vec![None; n];
    let mut seen = vec![false; n];
    let mut q = VecDeque::new();
    seen[grid.idx(from)] = true;
    q.push_back(from);
    while let Some(t) = q.pop_front() {
        if goal(t) && t != from {
            // Walk back to the step out of `from`.
            let (mut i, mut d) = prev[grid.idx(t)]?;
            let start = grid.idx(from);
            while i != start {
                let (pi, pd) = prev[i]?;
                i = pi;
                d = pd;
            }
            return Some(d);
        }
        for (d, nb) in neighbors(map, grid, t) {
            let bi = grid.idx(nb);
            if !seen[bi] && !blocked(nb) {
                seen[bi] = true;
                prev[bi] = Some((grid.idx(t), d));
                q.push_back(nb);
            }
        }
    }
    None
}

fn tile_dist(a: TilePos, b: TilePos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let map_name = args.get(1).map(String::as_str).unwrap_or("classic");
    let map = match map_name {
        "custom" => Map::custom(),
        _ => Map::classic(),
    };
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let radius: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    let grid = Grid {
        w: map.width(),
        h: map.height(),
    };
    let map_id = map.id().to_string();
    let mut g = Game::new(map.clone(), Rules::classic(), Difficulty::Normal, seed);
    let mut replay = Replay::new(map_id, Difficulty::Normal, seed);
    let mut held: Option<Dir> = None;
    let mut cleared = false;
    let max_ticks = 60 * 60 * 12; // 12 minutes of game time
    for t in 0..max_ticks {
        let rs = g.render_state();
        let pac = rs.pac_pos.tile();

        // Observable ghost threat/opportunity sets.
        let mut hostiles: Vec<TilePos> = Vec::new();
        let mut edibles: Vec<TilePos> = Vec::new();
        for gr in &rs.ghosts {
            let gt = gr.pos.tile();
            match gr.state {
                GhostState::Active | GhostState::Leaving if gr.frightened => edibles.push(gt),
                GhostState::Active | GhostState::Leaving => hostiles.push(gt),
                _ => {}
            }
        }
        let danger = |t: TilePos| -> bool { hostiles.iter().any(|&h| tile_dist(t, h) <= radius) };

        let pellet_at =
            |t: TilePos| -> bool { rs.pellets.get(map.tile_index(t)).copied().unwrap_or(false) };

        // 1) chase a close frightened ghost; 2) nearest pellet avoiding
        // danger; 3) any pellet ignoring danger but not through a ghost
        // tile; 4) flee to the neighbor farthest from the nearest hostile.
        let chase = edibles
            .iter()
            .any(|&e| tile_dist(pac, e) <= 8)
            .then(|| {
                bfs_first_step(
                    &map,
                    &grid,
                    pac,
                    |t| edibles.contains(&t),
                    |t| hostiles.iter().any(|&h| tile_dist(t, h) <= 1),
                )
            })
            .flatten();
        let step = chase
            .or_else(|| bfs_first_step(&map, &grid, pac, pellet_at, danger))
            .or_else(|| {
                bfs_first_step(&map, &grid, pac, pellet_at, |t| {
                    hostiles.iter().any(|&h| tile_dist(t, h) <= 1)
                })
            })
            .or_else(|| {
                neighbors(&map, &grid, pac)
                    .into_iter()
                    .max_by_key(|(_, n)| {
                        hostiles
                            .iter()
                            .map(|&h| tile_dist(*n, h))
                            .min()
                            .unwrap_or(i32::MAX)
                    })
                    .map(|(d, _)| d)
            });
        if let Some(d) = step {
            held = Some(d);
        }

        let input = InputFrame { dir: held };
        let events = g.tick(input);
        replay.push(input);
        for e in &events {
            use pacmantui::types::Event::*;
            match e {
                FruitSpawned { .. }
                | FruitEaten { .. }
                | ExtraLife
                | PacDying
                | LevelCleared { .. }
                | NextLevelStarted { .. }
                | GameOver => {
                    println!("t={t} {e:?}");
                }
                _ => {}
            }
            if matches!(e, LevelCleared { level: 1 }) {
                cleared = true;
            }
        }
        if cleared || g.is_game_over() {
            break;
        }
    }
    println!(
        "result: cleared={} score={} lives={} pellets_left={} ticks={}",
        cleared,
        g.score(),
        g.lives(),
        g.pellets_remaining(),
        replay.inputs.len()
    );
    if cleared && let Some(path) = args.get(4) {
        std::fs::write(path, replay.to_text()).expect("write replay");
        println!("replay written to {path}");
    }
    std::process::exit(if cleared { 0 } else { 1 });
}
