//! Application state machine: menus, loading, gameplay loop, persistence.
//!
//! Owns the fixed-timestep loop: the sim ticks at [`TICK_HZ`] driven by a
//! monotonic-clock accumulator; rendering happens after ticks and never
//! mutates the sim, so a slow terminal drops frames, not rules (goal §5).
//! Long stalls (> ~6 ticks) drop wall time instead of fast-forwarding.

pub mod menu;
pub mod persist;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::map::Map;
use crate::render::{AppEvent, Key, LoadingScreen, Renderer};
use crate::replay::Replay;
use crate::rules::Rules;
use crate::sim::Game;
use crate::types::{Difficulty, InputFrame, TICK_HZ};

use menu::{MainMenu, MenuAction};
use persist::Store;

const USAGE: &str = "\
pacmantui - Pac-Man in the terminal with real pixel graphics (kitty protocol)

USAGE: pacmantui [OPTIONS]
  --map <FILE.pmtoml>   add an external map to the menu (repeatable)
  --replay <FILE>       play back a recorded replay, then open the menu
  --record <FILE>       write a replay of each finished game to FILE
  --version             print the version
  --help                show this help

Controls: arrows/WASD move, P pause, R (paused) restart, Esc menu, Q quit.
Requires a kitty-graphics terminal (Ghostty, kitty); see README.md.
";

#[derive(Debug, Default, PartialEq, Eq)]
struct Cli {
    maps: Vec<PathBuf>,
    replay: Option<PathBuf>,
    record: Option<PathBuf>,
    help: bool,
    version: bool,
}

fn parse_args<I: Iterator<Item = String>>(mut args: I) -> Result<Cli, String> {
    let mut cli = Cli::default();
    while let Some(a) = args.next() {
        let mut value = |flag: &str| {
            args.next()
                .ok_or_else(|| format!("{flag} needs a file argument"))
        };
        match a.as_str() {
            "--map" => cli.maps.push(PathBuf::from(value("--map")?)),
            "--replay" => cli.replay = Some(PathBuf::from(value("--replay")?)),
            "--record" => cli.record = Some(PathBuf::from(value("--record")?)),
            "--help" | "-h" => cli.help = true,
            "--version" | "-V" => cli.version = true,
            other => return Err(format!("unknown argument {other:?} (try --help)")),
        }
    }
    Ok(cli)
}

/// Entry point: full application flow (menu → game → menu, quit).
pub fn run() -> Result<(), Box<dyn Error>> {
    let cli = parse_args(std::env::args().skip(1)).map_err(io::Error::other)?;
    if cli.help {
        print!("{USAGE}");
        return Ok(());
    }
    if cli.version {
        println!("pacmantui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    // Parse the replay before touching the terminal so errors print cleanly.
    let replay = match &cli.replay {
        Some(p) => {
            let src = fs::read_to_string(p)
                .map_err(|e| io::Error::other(format!("cannot read {}: {e}", p.display())))?;
            Some(Replay::from_text(&src).map_err(io::Error::other)?)
        }
        None => None,
    };

    let mut r = Renderer::new()?;
    let (mut store, maps) = boot_load(&mut r, &cli.maps)?;

    // Replay playback (if any) runs before the menu; its map must be loaded.
    if let Some(rp) = &replay {
        let map = maps
            .iter()
            .find(|m| m.id() == rp.map_id)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "replay wants map {:?}, which is not loaded (use --map)",
                    rp.map_id
                ))
            })?
            .clone();
        if let SessionEnd::QuitApp =
            game_session(&mut r, &mut store, &map, rp.difficulty, Some(rp), None)?
        {
            return Ok(());
        }
    }

    let start_idx = maps
        .iter()
        .position(|m| m.id() == store.settings.map)
        .unwrap_or(0);
    let mut mm = MainMenu::new(start_idx, store.settings.difficulty());
    let map_names: Vec<String> = maps.iter().map(|m| m.name().to_string()).collect();

    loop {
        r.render_menu(&mm.screen(&map_names))?;
        for ev in r.poll_events(Duration::from_millis(250))? {
            let key = match ev {
                AppEvent::Key(k) => k,
                AppEvent::Resized => continue, // redrawn next loop
            };
            match mm.on_key(key, maps.len()) {
                MenuAction::None => {}
                MenuAction::Quit => {
                    // Persist menu selections even when quitting without playing.
                    store.settings.map = maps[mm.map_idx].id().to_string();
                    store.settings.set_difficulty(mm.difficulty);
                    let _ = store.save();
                    return Ok(());
                }
                MenuAction::Controls => controls_loop(&mut r)?,
                MenuAction::Scores => {
                    scores_loop(&mut r, &store, &maps, mm.map_idx, mm.difficulty)?
                }
                MenuAction::Play => {
                    let map = maps[mm.map_idx].clone();
                    store.settings.map = map.id().to_string();
                    store.settings.set_difficulty(mm.difficulty);
                    let _ = store.save();
                    let end = game_session(
                        &mut r,
                        &mut store,
                        &map,
                        mm.difficulty,
                        None,
                        cli.record.as_deref(),
                    )?;
                    if let SessionEnd::QuitApp = end {
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Loading screen doing the real startup work: persistence, embedded maps,
/// external map discovery (./maps and <config>/maps), CLI-passed maps.
fn boot_load(r: &mut Renderer, cli_maps: &[PathBuf]) -> io::Result<(Store, Vec<Map>)> {
    let step = |r: &mut Renderer, msg: &str, p: f32| -> io::Result<()> {
        r.render_loading(&LoadingScreen {
            message: msg.into(),
            progress: p,
        })
    };

    step(r, "LOADING SETTINGS", 0.1)?;
    let store = Store::load(Store::default_dir());

    step(r, "LOADING MAPS", 0.4)?;
    let mut maps = vec![Map::classic(), Map::custom()];

    step(r, "SCANNING EXTERNAL MAPS", 0.7)?;
    for dir in [PathBuf::from("maps"), Store::default_dir().join("maps")] {
        for m in scan_map_dir(&dir) {
            if !maps.iter().any(|e| e.id() == m.id()) {
                maps.push(m);
            }
        }
    }
    for p in cli_maps {
        let m = Map::load_file(p)
            .map_err(|e| io::Error::other(format!("invalid map {}: {e}", p.display())))?;
        if !maps.iter().any(|e| e.id() == m.id()) {
            maps.push(m);
        }
    }

    step(r, "READY", 1.0)?;
    Ok((store, maps))
}

/// Valid .pmtoml maps in `dir`; unreadable/invalid files are skipped (the
/// menu only ever offers maps that already passed validation — use --map to
/// surface a specific file's validation error).
fn scan_map_dir(dir: &Path) -> Vec<Map> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pmtoml"))
        .collect();
    paths.sort();
    paths
        .iter()
        .filter_map(|p| Map::load_file(p).ok())
        .collect()
}

fn controls_loop(r: &mut Renderer) -> io::Result<()> {
    loop {
        r.render_menu(&menu::controls_screen())?;
        for ev in r.poll_events(Duration::from_millis(250))? {
            if let AppEvent::Key(Key::Escape | Key::Enter | Key::Quit) = ev {
                return Ok(());
            }
        }
    }
}

fn scores_loop(
    r: &mut Renderer,
    store: &Store,
    maps: &[Map],
    start_idx: usize,
    difficulty: Difficulty,
) -> io::Result<()> {
    let mut idx = start_idx;
    loop {
        let map = &maps[idx];
        let rows: Vec<(u32, u32)> = store
            .top_scores(map.id(), difficulty)
            .iter()
            .map(|e| (e.score, e.level))
            .collect();
        r.render_menu(&menu::scores_screen(map.name(), difficulty, &rows))?;
        for ev in r.poll_events(Duration::from_millis(250))? {
            match ev {
                AppEvent::Key(Key::Escape | Key::Enter | Key::Quit) => return Ok(()),
                AppEvent::Key(Key::Left) => idx = (idx + maps.len() - 1) % maps.len(),
                AppEvent::Key(Key::Right) => idx = (idx + 1) % maps.len(),
                _ => {}
            }
        }
    }
}

enum SessionEnd {
    ToMenu,
    QuitApp,
}

enum PlayEnd {
    Restart,
    ToMenu,
    QuitApp,
}

/// One game plus its restarts. Records scores (never for replay playback,
/// which must not pollute the tables) and optionally writes a replay file.
fn game_session(
    r: &mut Renderer,
    store: &mut Store,
    map: &Map,
    difficulty: Difficulty,
    replay_src: Option<&Replay>,
    record_to: Option<&Path>,
) -> io::Result<SessionEnd> {
    loop {
        match play_once(r, store, map, difficulty, replay_src, record_to)? {
            PlayEnd::Restart => continue,
            PlayEnd::ToMenu => return Ok(SessionEnd::ToMenu),
            PlayEnd::QuitApp => return Ok(SessionEnd::QuitApp),
        }
    }
}

fn play_once(
    r: &mut Renderer,
    store: &mut Store,
    map: &Map,
    difficulty: Difficulty,
    replay_src: Option<&Replay>,
    record_to: Option<&Path>,
) -> io::Result<PlayEnd> {
    let tick_len = Duration::from_secs_f64(1.0 / TICK_HZ);
    let max_backlog = tick_len * 6;

    let seed = match replay_src {
        Some(rp) => rp.seed,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5EED),
    };
    let mut game = Game::new(map.clone(), Rules::classic(), difficulty, seed);
    let mut recording = Replay::new(map.id().to_string(), difficulty, seed);
    let persisted_high = store.high_score(map.id(), difficulty);

    let mut desired: Option<crate::types::Dir> = None;
    let mut replay_pos = 0usize;
    let mut paused = false;
    let mut game_over = false;
    let mut acc = Duration::ZERO;
    let mut last = Instant::now();
    r.set_paused(false);
    r.set_game_over(false);

    let finish = |r: &mut Renderer,
                  store: &mut Store,
                  game: &Game,
                  recording: &Replay,
                  end: PlayEnd|
     -> io::Result<PlayEnd> {
        r.set_paused(false);
        r.set_game_over(false);
        if replay_src.is_none() {
            if game.score() > 0 {
                store.record_score(map.id(), difficulty, game.score(), game.level());
                let _ = store.save();
            }
            if let Some(p) = record_to {
                fs::write(p, recording.to_text())?;
            }
        }
        Ok(end)
    };

    // First frame before any input.
    render_frame(r, &game, persisted_high)?;

    loop {
        let mut dirty = false;
        for ev in r.poll_events(Duration::from_millis(2))? {
            match ev {
                AppEvent::Resized => dirty = true,
                AppEvent::Key(key) => match key {
                    Key::Up | Key::Down | Key::Left | Key::Right => {
                        if replay_src.is_none() {
                            desired = Some(match key {
                                Key::Up => crate::types::Dir::Up,
                                Key::Down => crate::types::Dir::Down,
                                Key::Left => crate::types::Dir::Left,
                                _ => crate::types::Dir::Right,
                            });
                        }
                    }
                    Key::Pause => {
                        if !game_over {
                            paused = !paused;
                            r.set_paused(paused);
                            dirty = true;
                        }
                    }
                    Key::Escape => {
                        if game_over || paused {
                            return finish(r, store, &game, &recording, PlayEnd::ToMenu);
                        }
                        paused = true;
                        r.set_paused(true);
                        dirty = true;
                    }
                    Key::Enter => {
                        if game_over {
                            return finish(r, store, &game, &recording, PlayEnd::ToMenu);
                        }
                    }
                    Key::Char('r') => {
                        if paused || game_over {
                            return finish(r, store, &game, &recording, PlayEnd::Restart);
                        }
                    }
                    Key::Quit => {
                        return finish(r, store, &game, &recording, PlayEnd::QuitApp);
                    }
                    _ => {}
                },
            }
        }

        let now = Instant::now();
        let dt = now - last;
        last = now;
        let mut ticked = false;
        if !paused && !game_over {
            acc += dt;
            if acc > max_backlog {
                // A long stall (window drag, SIGSTOP…) drops wall time instead
                // of fast-forwarding the rules.
                acc = max_backlog;
            }
            while acc >= tick_len && !game_over {
                acc -= tick_len;
                let input = match replay_src {
                    Some(rp) => {
                        let f = rp.inputs.get(replay_pos).copied().unwrap_or_default();
                        replay_pos += 1;
                        f
                    }
                    None => InputFrame { dir: desired },
                };
                let events = game.tick(input);
                recording.push(input);
                ticked = true;
                for ev in &events {
                    if let crate::types::Event::GameOver = ev {
                        game_over = true;
                        r.set_game_over(true);
                    }
                }
            }
        }

        if ticked || dirty {
            render_frame(r, &game, persisted_high)?;
        }
    }
}

fn render_frame(r: &mut Renderer, game: &Game, persisted_high: u32) -> io::Result<()> {
    let mut rs = game.render_state();
    rs.high_score = rs.high_score.max(persisted_high).max(rs.score);
    r.render_game(game.map(), &rs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn cli_defaults_empty() {
        assert_eq!(parse(&[]).unwrap(), Cli::default());
    }

    #[test]
    fn cli_collects_repeated_maps_and_flags() {
        let cli = parse(&[
            "--map", "a.pmtoml", "--map", "b.pmtoml", "--record", "out.txt",
        ])
        .unwrap();
        assert_eq!(cli.maps.len(), 2);
        assert_eq!(cli.record.as_deref(), Some(Path::new("out.txt")));
        assert!(cli.replay.is_none());
    }

    #[test]
    fn cli_rejects_unknown_and_missing_value() {
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--map"]).is_err());
    }
}
