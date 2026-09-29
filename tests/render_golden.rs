//! Golden-image tests of composed framebuffers (pure CPU, no tty), per
//! docs/plan/ARCHITECTURE.md. One FNV-1a-64 hash per (map, scene) pins every
//! pixel of the composed frame: any refactor of the render implementation
//! must keep these bit-identical.
//!
//! Regenerating deliberately: when a VISUAL change is intended, run
//! `cargo test --test render_golden -- --nocapture` and copy the printed
//! `("name", 0x...)` table over `GOLDEN`, noting the visual change in the
//! commit message. Never regenerate to silence an unexpected mismatch — an
//! unexpected mismatch is a rendering regression.

use pacmantui::map::Map;
use pacmantui::render::compose::{Compositor, Frame, Overlay, TileKind, cell_kind};
use pacmantui::render::{LoadingScreen, MenuItem, MenuScreen};
use pacmantui::sim::timings;
use pacmantui::types::{
    Dir, GhostId, GhostRender, GhostState, PxPos, RenderState, Sequence, TilePos,
};

/// FNV-1a 64 over the frame dimensions and every RGB byte.
fn fnv1a(fb: &Frame) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(&(fb.width() as u64).to_le_bytes());
    eat(&(fb.height() as u64).to_le_bytes());
    eat(fb.data());
    h
}

fn ghost(
    id: GhostId,
    tile: (i32, i32),
    state: GhostState,
    frightened: bool,
    anim: u8,
) -> GhostRender {
    GhostRender {
        id,
        pos: PxPos::tile_center(TilePos::new(tile.0, tile.1)),
        dir: Dir::Right,
        state,
        frightened,
        anim,
        visible: true,
    }
}

/// A deterministic, feature-rich gameplay state: pellets from the map,
/// all four ghosts visible (one frightened, one as eyes), fruit on the
/// board, a score popup, HUD score/lives/fruit history.
fn rich_state(m: &Map) -> RenderState {
    let (tw, th) = (m.width(), m.height());
    let mut pellets = vec![false; (tw * th) as usize];
    for y in 0..th {
        for x in 0..tw {
            if matches!(cell_kind(m, x, y), TileKind::Dot | TileKind::Energizer) {
                pellets[(y * tw + x) as usize] = true;
            }
        }
    }
    RenderState {
        pac_pos: PxPos::tile_center(TilePos::new(tw / 2, th / 2)),
        pac_dir: Dir::Left,
        pac_anim: 2,
        ghosts: [
            ghost(GhostId::Blinky, (6, 6), GhostState::Active, false, 0),
            ghost(GhostId::Pinky, (8, 6), GhostState::Active, true, 1),
            ghost(GhostId::Inky, (6, 8), GhostState::Eyes, false, 0),
            ghost(GhostId::Clyde, (8, 8), GhostState::Active, false, 1),
        ],
        fright_flash: Some(false),
        fruit: Some(m.fruit_pos().tile()),
        popups: vec![(TilePos::new(10, 10), 200, 5)],
        score: 1234,
        high_score: 5000,
        lives: 3,
        level: 1,
        fruit_history: vec![0, 1],
        sequence: Sequence::Playing,
        pellets,
        energizer_blink_on: true,
        pac_visible: true,
    }
}

/// Compose one gameplay frame through the compositor's public interface —
/// the same seam `Renderer::render_game` adapts to the tty.
fn game_frame(m: &Map, st: &RenderState, paused: bool, game_over: bool) -> Frame {
    let mut c = Compositor::new();
    c.game(m, st, Overlay { paused, game_over }).clone()
}

fn menu_frame() -> Frame {
    let mut c = Compositor::new();
    c.menu(&MenuScreen {
        title: "PACMANTUI".into(),
        items: vec![
            MenuItem {
                label: "PLAY".into(),
                value: None,
                enabled: true,
            },
            MenuItem {
                label: "DIFFICULTY".into(),
                value: Some("NORMAL".into()),
                enabled: true,
            },
            MenuItem {
                label: "QUIT".into(),
                value: None,
                enabled: false,
            },
        ],
        selected: 1,
        footer: "ARROWS MOVE - ENTER SELECTS".into(),
        decorated: true,
    })
    .clone()
}

fn loading_frame() -> Frame {
    let mut c = Compositor::new();
    c.loading(&LoadingScreen {
        message: "LOADING MAPS".into(),
        progress: 0.42,
    })
    .clone()
}

/// Every hashed (map, scene) pair, in a fixed order.
fn all_frames() -> Vec<(&'static str, u64)> {
    let mut out = Vec::new();
    for (map_name, m) in [("classic", Map::classic()), ("custom", Map::custom())] {
        let rich = rich_state(&m);
        out.push((
            match map_name {
                "classic" => "classic-playing",
                _ => "custom-playing",
            },
            fnv1a(&game_frame(&m, &rich, false, false)),
        ));
        let mut ready = rich_state(&m);
        ready.sequence = Sequence::Ready { tick: 0 };
        out.push((
            match map_name {
                "classic" => "classic-ready",
                _ => "custom-ready",
            },
            fnv1a(&game_frame(&m, &ready, false, false)),
        ));
        out.push((
            match map_name {
                "classic" => "classic-paused",
                _ => "custom-paused",
            },
            fnv1a(&game_frame(&m, &rich, true, false)),
        ));
        out.push((
            match map_name {
                "classic" => "classic-game-over",
                _ => "custom-game-over",
            },
            fnv1a(&game_frame(&m, &rich, false, true)),
        ));
        let mut flash = rich_state(&m);
        flash.sequence = Sequence::LevelFlash {
            tick: timings::LEVEL_CLEAR_FREEZE_TICKS,
        };
        flash.pellets.iter_mut().for_each(|p| *p = false);
        out.push((
            match map_name {
                "classic" => "classic-flash-white",
                _ => "custom-flash-white",
            },
            fnv1a(&game_frame(&m, &flash, false, false)),
        ));
    }
    out.push(("menu", fnv1a(&menu_frame())));
    out.push(("loading", fnv1a(&loading_frame())));
    out
}

/// The pinned hashes. See the module doc for how to regenerate DELIBERATELY.
const GOLDEN: [(&str, u64); 12] = [
    ("classic-playing", 0x5235e39005753615),
    ("classic-ready", 0x20dc43b3b9e2b0cf),
    ("classic-paused", 0xee583deba5b3c400),
    ("classic-game-over", 0x0efa5d907277195a),
    ("classic-flash-white", 0xd90996eb96bc0dc9),
    ("custom-playing", 0x46f5b9132215a321),
    ("custom-ready", 0x50b036ab261c8073),
    ("custom-paused", 0x644d2c724a8d2138),
    ("custom-game-over", 0x96809df32338c1e2),
    ("custom-flash-white", 0x9366d1fdfd349fa2),
    ("menu", 0xe7cd0fbb683ed6b2),
    ("loading", 0xfdc6a768d0916449),
];

#[test]
fn composed_frames_match_golden_hashes() {
    let got = all_frames();
    for (name, hash) in &got {
        eprintln!("    (\"{name}\", {hash:#018x}),");
    }
    for (name, hash) in &got {
        let (_, want) = GOLDEN
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no golden entry for scene {name}"));
        assert_eq!(
            hash, want,
            "composed frame hash changed for scene {name} — a pixel-level \
             rendering change. If unintended, this is a regression; if \
             intended, regenerate per the module doc."
        );
    }
    assert_eq!(got.len(), GOLDEN.len(), "scene list and GOLDEN differ");
}
