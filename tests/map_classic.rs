//! Classic-map invariants, checked against docs/research/dossier-mechanics.md
//! (grid §1.2, start positions §1.3, scatter §3.5, red zones §3.7, tunnel §2.5).

use pacmantui::map::{Cell, Map};
use pacmantui::types::{Dir, Fix8, GhostId, PxPos, TilePos};

fn px(x: i32, y: i32) -> PxPos {
    PxPos {
        x: Fix8::from_px(x),
        y: Fix8::from_px(y),
    }
}

#[test]
fn identity() {
    let m = Map::classic();
    assert_eq!(m.name(), "Classic");
    assert!(m.id().starts_with("classic-"), "id = {}", m.id());
    assert_eq!(
        m.id().len(),
        "classic-".len() + 16,
        "id carries a 16-hex content hash"
    );
    assert!(m.is_classic());
}

#[test]
fn dimensions_are_full_28x36_screen() {
    let m = Map::classic();
    assert_eq!(m.width(), 28);
    assert_eq!(m.height(), 36);
}

#[test]
fn pellet_counts_match_dossier() {
    let m = Map::classic();
    assert_eq!(m.dots_total(), 240);
    assert_eq!(m.pellets_total(), 244);

    // Recount from the grid to cross-check the accessors.
    let mut dots = 0;
    let mut energizers = 0;
    let mut walkable_no_dot = 0;
    for y in 0..m.height() {
        for x in 0..m.width() {
            match m.cell(TilePos::new(x, y)) {
                Cell::Dot => dots += 1,
                Cell::Energizer => energizers += 1,
                Cell::Path | Cell::Tunnel => walkable_no_dot += 1,
                _ => {}
            }
        }
    }
    assert_eq!(dots, 240);
    assert_eq!(energizers, 4);
    // Dossier §1.1/§1.2: 300 legal tiles, 56 of them without a dot.
    assert_eq!(walkable_no_dot, 56);
}

#[test]
fn energizers_at_documented_tiles() {
    let m = Map::classic();
    for (x, y) in [(1, 6), (26, 6), (1, 26), (26, 26)] {
        assert_eq!(
            m.cell(TilePos::new(x, y)),
            Cell::Energizer,
            "expected energizer at ({x},{y})"
        );
    }
}

#[test]
fn door_and_house_geometry() {
    let m = Map::classic();
    assert_eq!(
        m.door_tiles(),
        &[TilePos::new(13, 15), TilePos::new(14, 15)]
    );
    // House interior: cols 11-16, rows 16-18 (dossier §1.2).
    for y in 16..=18 {
        for x in 11..=16 {
            assert_eq!(
                m.cell(TilePos::new(x, y)),
                Cell::House,
                "expected house at ({x},{y})"
            );
        }
    }
    assert_eq!(m.house_center(), px(112, 140));
    assert_eq!(m.eyes_target(), TilePos::new(13, 14));
    // Doors and house are not walkable for normal actors.
    assert!(!m.walkable(TilePos::new(13, 15)));
    assert!(!m.walkable(TilePos::new(13, 17)));
}

#[test]
fn spawns_are_pixel_exact() {
    let m = Map::classic();
    // Dossier §1.3: actors start centered on tile-column boundaries.
    assert_eq!(m.pac_spawn(), px(112, 212));
    assert_eq!(m.ghost_spawn(GhostId::Blinky), px(112, 116));
    assert_eq!(m.ghost_spawn(GhostId::Pinky), px(112, 140));
    assert_eq!(m.ghost_spawn(GhostId::Inky), px(96, 140));
    assert_eq!(m.ghost_spawn(GhostId::Clyde), px(128, 140));
    // Occupied tiles derived from those pixel centers (floor(px/8)).
    assert_eq!(m.pac_spawn().tile(), TilePos::new(14, 26));
    assert_eq!(m.ghost_spawn(GhostId::Blinky).tile(), TilePos::new(14, 14));
    assert_eq!(m.ghost_spawn(GhostId::Inky).tile(), TilePos::new(12, 17));
}

#[test]
fn spawn_facing_directions() {
    let m = Map::classic();
    assert_eq!(m.pac_spawn_dir(), Dir::Left);
    assert_eq!(m.ghost_spawn_dir(GhostId::Blinky), Dir::Left);
    assert_eq!(m.ghost_spawn_dir(GhostId::Pinky), Dir::Down);
    assert_eq!(m.ghost_spawn_dir(GhostId::Inky), Dir::Up);
    assert_eq!(m.ghost_spawn_dir(GhostId::Clyde), Dir::Up);
}

#[test]
fn scatter_targets_dossier_measured() {
    let m = Map::classic();
    assert_eq!(m.scatter_target(GhostId::Pinky), TilePos::new(2, -1));
    assert_eq!(m.scatter_target(GhostId::Blinky), TilePos::new(25, -1));
    assert_eq!(m.scatter_target(GhostId::Clyde), TilePos::new(0, 34));
    assert_eq!(m.scatter_target(GhostId::Inky), TilePos::new(27, 34));
}

#[test]
fn fruit_position() {
    let m = Map::classic();
    assert_eq!(m.fruit_pos(), px(112, 164));
    assert!(m.walkable(m.fruit_pos().tile()));
}

#[test]
fn no_up_tiles_are_the_four_red_zone_decisions() {
    let m = Map::classic();
    assert_eq!(
        m.no_up_tiles(),
        &[
            TilePos::new(12, 14),
            TilePos::new(15, 14),
            TilePos::new(12, 26),
            TilePos::new(15, 26)
        ]
    );
    for &t in m.no_up_tiles() {
        assert!(m.walkable(t));
    }
}

#[test]
fn tunnel_slowdown_zones() {
    let m = Map::classic();
    // Dossier §2.5: row 17 cols 0-4 and 23-27.
    for x in (0..=4).chain(23..=27) {
        assert!(
            m.is_tunnel(TilePos::new(x, 17)),
            "expected tunnel at ({x},17)"
        );
    }
    // The rest of the tunnel row is plain walkable path/dots.
    assert!(!m.is_tunnel(TilePos::new(5, 17)));
    assert!(m.walkable(TilePos::new(5, 17)));
    assert!(!m.is_tunnel(TilePos::new(7, 17)));
}

#[test]
fn warp_wraps_pixel_positions_both_directions() {
    let m = Map::classic();
    let y = Fix8::from_px(140); // tunnel row 17 center

    // Exiting the left edge reappears at the right edge.
    let left_exit = PxPos {
        x: Fix8::from_px(-1),
        y,
    };
    assert_eq!(
        m.warp(left_exit),
        PxPos {
            x: Fix8::from_px(223),
            y
        }
    );

    // Exiting the right edge (grid is 224 px wide) reappears at the left.
    let right_exit = PxPos {
        x: Fix8::from_px(224),
        y,
    };
    assert_eq!(
        m.warp(right_exit),
        PxPos {
            x: Fix8::from_px(0),
            y
        }
    );

    // Subpixel wrap: 1/256 px past the left edge.
    let sub = PxPos { x: Fix8(-1), y };
    assert_eq!(m.warp(sub).x.raw(), 224 * 256 - 1);

    // Identity for in-bounds positions.
    let inside = px(100, 212);
    assert_eq!(m.warp(inside), inside);
}

#[test]
fn tile_index_is_row_major() {
    let m = Map::classic();
    assert_eq!(m.tile_index(TilePos::new(0, 0)), 0);
    assert_eq!(m.tile_index(TilePos::new(27, 0)), 27);
    assert_eq!(m.tile_index(TilePos::new(0, 1)), 28);
    assert_eq!(m.tile_index(TilePos::new(13, 15)), 15 * 28 + 13);
    assert_eq!(m.tile_index(TilePos::new(27, 35)), 28 * 36 - 1);
    // Consistency across the whole grid.
    let mut expected = 0usize;
    for y in 0..m.height() {
        for x in 0..m.width() {
            assert_eq!(m.tile_index(TilePos::new(x, y)), expected);
            expected += 1;
        }
    }
}

#[test]
fn out_of_grid_reads_as_wall() {
    let m = Map::classic();
    assert_eq!(m.cell(TilePos::new(-1, 17)), Cell::Wall);
    assert_eq!(m.cell(TilePos::new(28, 17)), Cell::Wall);
    assert_eq!(m.cell(TilePos::new(2, -1)), Cell::Wall); // scatter-target dead space
    assert!(!m.walkable(TilePos::new(2, -1)));
}

#[test]
fn hud_margin_rows_are_dead_space() {
    let m = Map::classic();
    for y in [0, 1, 2, 34, 35] {
        for x in 0..28 {
            assert_eq!(
                m.cell(TilePos::new(x, y)),
                Cell::Wall,
                "HUD row {y} must be dead space"
            );
        }
    }
}

#[test]
fn classic_is_exempt_from_threshold_scaling() {
    let m = Map::classic();
    for v in [0, 10, 20, 30, 60, 70, 170, 244] {
        assert_eq!(m.scale_dot_threshold(v), v);
    }
}

#[test]
fn load_file_matches_embedded() {
    let m = Map::load_file(std::path::Path::new("maps/classic.pmtoml")).expect("load classic");
    assert_eq!(m.id(), Map::classic().id());
    assert!(m.is_classic());
}
