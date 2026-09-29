//! Headless diagnostic: run the sim on a real map with a keyframed input
//! schedule and log events + positions. Not part of the game; used to
//! reproduce live-play findings and to author legal replays without the
//! terminal in the loop (no state cheats — inputs go through Game::tick).
//!
//! Usage:
//!   cargo run --example headless_probe -- <classic|custom> <ticks> \
//!       [schedule] [emit-replay-path]
//!
//! `schedule` is a comma list of `TICK:DIR` keyframes (dir = up/down/left/
//! right/none) meaning "from TICK on, hold DIR". Example:
//!   258:left,320:up,430:left,520:down

use std::collections::BTreeMap;

use pacmantui::map::Map;
use pacmantui::replay::Replay;
use pacmantui::rules::Rules;
use pacmantui::sim::Game;
use pacmantui::types::{Difficulty, Dir, InputFrame};

fn parse_dir(s: &str) -> Option<Dir> {
    match s {
        "up" => Some(Dir::Up),
        "down" => Some(Dir::Down),
        "left" => Some(Dir::Left),
        "right" => Some(Dir::Right),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let map_name = args.get(1).map(String::as_str).unwrap_or("classic");
    let map = match map_name {
        "custom" => Map::custom(),
        _ => Map::classic(),
    };
    let ticks: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3600);
    let mut schedule: BTreeMap<u32, Option<Dir>> = BTreeMap::new();
    if let Some(spec) = args.get(3) {
        for kf in spec.split(',').filter(|s| !s.is_empty()) {
            let (t, d) = kf.split_once(':').expect("keyframe TICK:DIR");
            schedule.insert(t.parse().expect("tick"), parse_dir(d));
        }
    }
    let seed = 1u64;
    // Replays are keyed by the content-hashed map id, so they only ever run
    // against the exact map content they were recorded on.
    let map_id = map.id().to_string();
    let mut g = Game::new(map, Rules::classic(), Difficulty::Normal, seed);
    let mut replay = Replay::new(map_id, Difficulty::Normal, seed);
    let mut held: Option<Dir> = None;
    for t in 0..ticks {
        if let Some(d) = schedule.get(&t) {
            held = *d;
        }
        let input = InputFrame { dir: held };
        let events = g.tick(input);
        replay.push(input);
        for e in &events {
            println!("t={t} {e:?}");
        }
        if t.is_multiple_of(60) {
            let rs = g.render_state();
            let pt = rs.pac_pos.tile();
            println!(
                "t={t} pac=({},{}) score={} lives={} fright={}",
                pt.x,
                pt.y,
                rs.score,
                rs.lives,
                rs.fright_flash.is_some()
            );
        }
        if g.is_game_over() {
            println!("t={t} (game over reached; stopping)");
            break;
        }
    }
    println!(
        "end: score={} lives={} level={} pellets_left={}",
        g.score(),
        g.lives(),
        g.level(),
        g.pellets_remaining()
    );
    if let Some(path) = args.get(4) {
        std::fs::write(path, replay.to_text()).expect("write replay");
        println!("replay written to {path} ({} ticks)", replay.inputs.len());
    }
}
