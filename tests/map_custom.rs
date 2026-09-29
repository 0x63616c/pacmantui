//! Shipped custom map ("Vertigo") parses, validates, and differs from classic
//! in the ways docs/map-format.md and maps/custom.pmtoml claim.

use pacmantui::map::{Cell, Map};
use pacmantui::types::{Dir, Fix8, GhostId, PxPos, TilePos};

fn px(x: i32, y: i32) -> PxPos {
    PxPos {
        x: Fix8::from_px(x),
        y: Fix8::from_px(y),
    }
}

#[test]
fn identity_and_dimensions() {
    let m = Map::custom();
    assert_eq!(m.name(), "Vertigo");
    assert!(m.id().starts_with("vertigo-"), "id = {}", m.id());
    assert!(!m.is_classic());
    assert_eq!(m.width(), 32);
    assert_eq!(m.height(), 26);
}

#[test]
fn pellet_counts_differ_from_classic() {
    let m = Map::custom();
    assert_eq!(m.dots_total(), 336);
    assert_eq!(m.pellets_total(), 340);
    assert_ne!(m.pellets_total(), 244);
    let mut energizers = 0;
    for y in 0..m.height() {
        for x in 0..m.width() {
            if m.cell(TilePos::new(x, y)) == Cell::Energizer {
                energizers += 1;
            }
        }
    }
    assert_eq!(energizers, 4);
}

#[test]
fn vertical_tunnels_warp_top_to_bottom() {
    let m = Map::custom();
    // Two vertical tunnel columns (3 and 28), open on both the top and
    // bottom edges.
    for x in [3, 28] {
        assert!(m.walkable(TilePos::new(x, 0)));
        assert!(m.walkable(TilePos::new(x, 25)));
        assert!(m.is_tunnel(TilePos::new(x, 0)));
        assert!(m.is_tunnel(TilePos::new(x, 1)));
        assert!(m.is_tunnel(TilePos::new(x, 24)));
        assert!(m.is_tunnel(TilePos::new(x, 25)));
    }
    // Pixel-level warp: leaving the top edge reappears at the bottom.
    let x = Fix8::from_px(3 * 8 + 4);
    let up_exit = PxPos {
        x,
        y: Fix8::from_px(-1),
    };
    assert_eq!(
        m.warp(up_exit),
        PxPos {
            x,
            y: Fix8::from_px(26 * 8 - 1)
        }
    );
    let down_exit = PxPos {
        x,
        y: Fix8::from_px(26 * 8),
    };
    assert_eq!(
        m.warp(down_exit),
        PxPos {
            x,
            y: Fix8::from_px(0)
        }
    );
    // Left/right edges are walls: nothing exits horizontally.
    for y in 0..26 {
        assert!(!m.walkable(TilePos::new(0, y)));
        assert!(!m.walkable(TilePos::new(31, y)));
    }
}

#[test]
fn house_door_and_special_points() {
    let m = Map::custom();
    assert_eq!(
        m.door_tiles(),
        &[TilePos::new(15, 14), TilePos::new(16, 14)]
    );
    assert_eq!(m.house_center(), px(128, 132));
    assert_eq!(m.eyes_target(), TilePos::new(15, 13));
    assert_eq!(m.fruit_pos(), px(128, 156));
    assert!(m.walkable(m.fruit_pos().tile()));
}

#[test]
fn spawns() {
    let m = Map::custom();
    // Pac-Man spawns ABOVE the house on this map.
    assert_eq!(m.pac_spawn(), px(128, 60));
    assert_eq!(m.pac_spawn_dir(), Dir::Left);
    assert_eq!(m.ghost_spawn(GhostId::Blinky), px(128, 108));
    assert_eq!(m.ghost_spawn(GhostId::Pinky), px(128, 132));
    assert_eq!(m.ghost_spawn(GhostId::Inky), px(112, 132));
    assert_eq!(m.ghost_spawn(GhostId::Clyde), px(144, 132));
    // In-house ghosts occupy house tiles; Blinky sits outside above the door.
    assert_eq!(m.cell(m.ghost_spawn(GhostId::Pinky).tile()), Cell::House);
    assert_eq!(m.cell(m.ghost_spawn(GhostId::Inky).tile()), Cell::House);
    assert_eq!(m.cell(m.ghost_spawn(GhostId::Clyde).tile()), Cell::House);
    assert!(m.walkable(m.ghost_spawn(GhostId::Blinky).tile()));
}

#[test]
fn scatter_and_no_up() {
    let m = Map::custom();
    assert_eq!(m.scatter_target(GhostId::Blinky), TilePos::new(29, -2));
    assert_eq!(m.scatter_target(GhostId::Pinky), TilePos::new(2, -2));
    assert_eq!(m.scatter_target(GhostId::Inky), TilePos::new(31, 27));
    assert_eq!(m.scatter_target(GhostId::Clyde), TilePos::new(0, 27));
    assert_eq!(
        m.no_up_tiles(),
        &[
            TilePos::new(14, 13),
            TilePos::new(17, 13),
            TilePos::new(12, 7),
            TilePos::new(19, 7)
        ]
    );
}

#[test]
fn threshold_scaling_uses_pellet_ratio() {
    let m = Map::custom();
    // round(v * 340 / 244), per docs/map-format.md "Rule adaptations".
    assert_eq!(m.scale_dot_threshold(0), 0);
    assert_eq!(m.scale_dot_threshold(7), 10); // 9.754 -> 10
    assert_eq!(m.scale_dot_threshold(30), 42); // 41.803 -> 42
    assert_eq!(m.scale_dot_threshold(70), 98); // 97.541 -> 98
    assert_eq!(m.scale_dot_threshold(170), 237); // 236.885 -> 237
}

#[test]
fn load_file_matches_embedded() {
    let m = Map::load_file(std::path::Path::new("maps/custom.pmtoml")).expect("load custom");
    assert_eq!(m.id(), Map::custom().id());
    assert!(!m.is_classic());
}
