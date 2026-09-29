//! Visual validation driver for the renderer (not the game): draws the classic
//! map with actors at spawn positions and cycling animation phases.
//! Run in a kitty-graphics terminal: `cargo run --example render_demo [custom]`.
//! Keys: q/Esc quit, f toggle frightened look, e toggle energizer blink demo.

use std::time::Duration;

use pacmantui::map::Map;
use pacmantui::render::{AppEvent, Key, Overlay, Renderer};
use pacmantui::types::{Dir, GhostId, GhostRender, GhostState, RenderState, Sequence};

fn main() -> std::io::Result<()> {
    let map = if std::env::args().nth(1).as_deref() == Some("custom") {
        Map::custom()
    } else {
        Map::classic()
    };
    let mut r = Renderer::new()?;
    let mut tick: u32 = 0;
    let mut frightened = false;

    loop {
        for ev in r.poll_events(Duration::from_millis(33))? {
            if let AppEvent::Key(k) = ev {
                match k {
                    Key::Quit | Key::Escape => return Ok(()),
                    Key::Char('f') => frightened = !frightened,
                    _ => {}
                }
            }
        }
        tick = tick.wrapping_add(2);

        let pellets = (0..(map.width() * map.height()))
            .map(|_| true)
            .collect::<Vec<_>>();
        let ghosts = [
            (GhostId::Blinky, GhostState::Active),
            (GhostId::Pinky, GhostState::InHouse),
            (GhostId::Inky, GhostState::InHouse),
            (GhostId::Clyde, GhostState::InHouse),
        ]
        .map(|(id, state)| GhostRender {
            id,
            pos: map.ghost_spawn(id),
            dir: map.ghost_spawn_dir(id),
            state,
            frightened,
            anim: ((tick / 8) % 2) as u8,
            visible: true,
        });
        let state = RenderState {
            pac_pos: map.pac_spawn(),
            pac_dir: map.pac_spawn_dir(),
            pac_anim: ((tick / 4) % 4) as u8,
            ghosts,
            fright_flash: if frightened {
                Some((tick / 16).is_multiple_of(2))
            } else {
                None
            },
            fruit: Some(map.fruit_pos().tile()),
            popups: vec![],
            score: 1280,
            high_score: 10_000,
            lives: 3,
            level: 1,
            fruit_history: vec![0],
            sequence: Sequence::Playing,
            pellets,
            energizer_blink_on: (tick / 10).is_multiple_of(2),
            pac_visible: true,
        };
        r.render_game(&map, &state, Overlay::default())?;
        let _ = Dir::Up; // silence unused import when not otherwise referenced
    }
}
