//! HUD padding for maps whose grid is all maze.
//!
//! Regression (live play, Vertigo): the gameplay frame was sized exactly to
//! the map grid, so the frame-anchored HUD (score rows 0-1, lives/fruit on
//! the bottom two rows) overprinted the maze on maps that do not embed the
//! classic dead rows. The frame now adds black padding rows derived from the
//! map's open-tile bounding box; classic (which embeds its own dead rows)
//! must stay pixel-identical.

use pacmantui::map::{Cell, Map};
use pacmantui::render::compose::{Compositor, Frame, Overlay, TileKind};
use pacmantui::render::layout::FrameLayout;
use pacmantui::types::{
    Dir, GhostId, GhostRender, GhostState, PxPos, RenderState, Sequence, TilePos,
};

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE_RGB: [u8; 3] = [255, 255, 255];
const YELLOW_RGB: [u8; 3] = [255, 255, 0];
const RED_RGB: [u8; 3] = [255, 0, 0];
const MAZE_BLUE_RGB: [u8; 3] = [33, 33, 222];
const DOOR_PINK_RGB: [u8; 3] = [255, 184, 222];
const PEACH_RGB: [u8; 3] = [255, 184, 174];

fn ghost(id: GhostId, tile: (i32, i32), visible: bool) -> GhostRender {
    GhostRender {
        id,
        pos: PxPos::tile_center(TilePos::new(tile.0, tile.1)),
        dir: Dir::Right,
        state: GhostState::Active,
        frightened: false,
        anim: 0,
        visible,
    }
}

/// Bare state: HUD data present, but no pellets and no visible actors, so a
/// composed frame is exactly maze layer + HUD.
fn bare_state(m: &Map) -> RenderState {
    RenderState {
        pac_pos: PxPos::tile_center(TilePos::new(m.width() / 2, m.height() / 2)),
        pac_dir: Dir::Left,
        pac_anim: 0,
        ghosts: [
            ghost(GhostId::Blinky, (6, 6), false),
            ghost(GhostId::Pinky, (6, 6), false),
            ghost(GhostId::Inky, (6, 6), false),
            ghost(GhostId::Clyde, (6, 6), false),
        ],
        fright_flash: None,
        fruit: None,
        popups: Vec::new(),
        score: 1234,
        high_score: 5000,
        lives: 3,
        level: 1,
        fruit_history: vec![0, 1],
        sequence: Sequence::Playing,
        pellets: vec![false; (m.width() * m.height()) as usize],
        energizer_blink_on: true,
        pac_visible: false,
    }
}

/// Compose a full gameplay frame through the compositor's public interface
/// (the seam `Renderer::render_game` adapts to the tty).
fn compose_frame(m: &Map, st: &RenderState) -> Frame {
    compose_frame_with(m, st, false, false)
}

fn compose_frame_with(m: &Map, st: &RenderState, paused: bool, game_over: bool) -> Frame {
    let mut c = Compositor::new();
    c.game(m, st, Overlay { paused, game_over }).clone()
}

/// The maze band may hold only maze-layer pixels (wall contours, door bar,
/// black): with a bare state, any other color there means the HUD (or an
/// overlay) touched the maze.
fn assert_maze_band_pure(fb: &Frame, y0: i32, y1: i32) {
    for (x, y, px) in pixels(fb, y0, y1) {
        assert!(
            px == BLACK || px == MAZE_BLUE_RGB || px == DOOR_PINK_RGB,
            "non-maze pixel {px:?} at ({x},{y}) inside the maze band"
        );
    }
}

fn pixels(fb: &Frame, y0: i32, y1: i32) -> impl Iterator<Item = (i32, i32, [u8; 3])> + '_ {
    (y0..y1).flat_map(move |y| (0..fb.width() as i32).map(move |x| (x, y, fb.get(x, y))))
}

// --- padding math ------------------------------------------------------------

#[test]
fn frame_layout_pads_from_open_bounding_box() {
    // Classic-shaped grid: 3 dead rows, border, open, border, 2 dead rows.
    // No padding: the frame equals the map grid and the maze band starts at 0.
    let classicish = |_x: i32, y: i32| {
        if (4..=6).contains(&y) {
            TileKind::Open
        } else {
            TileKind::Wall
        }
    };
    let lay = FrameLayout::new(6, 10, &classicish);
    assert_eq!(lay.frame_tiles(), (6, 10));
    assert_eq!(lay.maze_origin_px(), (0, 0));

    // All-maze grid, open through both edges (Vertigo shape): full padding —
    // 3 tile rows above (maze band starts at pixel 24) and 2 below.
    let all_open = |_x: i32, _y: i32| TileKind::Open;
    let lay = FrameLayout::new(6, 10, &all_open);
    assert_eq!(lay.frame_tiles(), (6, 15));
    assert_eq!(lay.maze_origin_px(), (0, 24));

    // Partially embedded dead space: 1 dead row on top (border at row 1,
    // open from row 2), open through the bottom edge -> pads (2, 2).
    let partial = |_x: i32, y: i32| {
        if y >= 2 {
            TileKind::Open
        } else {
            TileKind::Wall
        }
    };
    let lay = FrameLayout::new(6, 10, &partial);
    assert_eq!(lay.frame_tiles(), (6, 14));
    assert_eq!(lay.maze_origin_px(), (0, 16));

    // Degenerate all-wall grid: no open box, no padding.
    let walls = |_x: i32, _y: i32| TileKind::Wall;
    let lay = FrameLayout::new(6, 10, &walls);
    assert_eq!(lay.frame_tiles(), (6, 10));
    assert_eq!(lay.maze_origin_px(), (0, 0));
}

// --- classic: zero padding, frame identical to the map grid ------------------

#[test]
fn classic_map_needs_no_padding() {
    let m = Map::classic();
    let lay = FrameLayout::of_map(&m);
    assert_eq!(lay.frame_tiles(), lay.map_tiles());
    assert_eq!(lay.maze_origin_px(), (0, 0));

    let fb = compose_frame(&m, &bare_state(&m));
    // Frame is exactly the 28x36 map grid: 224x288 px, as before the fix.
    assert_eq!((fb.width(), fb.height()), (224, 288));

    // Maze area (border row 3 through border row 33) holds only maze-layer
    // pixels: the HUD never touches it.
    assert_maze_band_pure(&fb, 24, 272);

    // HUD placement unchanged: "1UP" row 0, score row 8 ('1' of 1234 at
    // x=26), lives and fruit strip on tile rows 34-35 (ly=272).
    assert_eq!(fb.get(26, 0), WHITE_RGB);
    assert_eq!(fb.get(26, 8), WHITE_RGB);
    assert_eq!(fb.get(16 + 12, 272 + 7), YELLOW_RGB); // first spare-life icon
    assert_eq!(fb.get(192 + 4, 272 + 4), RED_RGB); // latest fruit (strawberry)
}

// --- Vertigo: the regression -------------------------------------------------

#[test]
fn custom_map_hud_gets_padded_rows() {
    let m = Map::custom();
    let (tw, th) = (m.width(), m.height());
    // 32x26 all-maze grid: 3 padding rows on top, 2 on the bottom.
    let lay = FrameLayout::of_map(&m);
    assert_eq!(lay.map_tiles(), (32, 26));
    assert_eq!(lay.frame_tiles(), (32, 31));
    assert_eq!(lay.maze_origin_px(), (0, 24));

    let fb = compose_frame(&m, &bare_state(&m));
    // Frame is 32x31 tiles = 256x248 px; the maze band sits at tile rows
    // 3..=28 and holds only maze-layer pixels.
    assert_eq!((fb.width(), fb.height()), (256, 248));
    assert_maze_band_pure(&fb, 24, 232);
    // Top border's outer contour line lands at tile row 3 (pixel row 26).
    assert!((0..256).any(|x| fb.get(x, 26) == MAZE_BLUE_RGB));

    // Tile rows 0-2: nothing but black and the white HUD text, and the
    // spacer row (tile 2) is fully black.
    let mut top_white = 0;
    for (x, y, px) in pixels(&fb, 0, 24) {
        assert!(
            px == BLACK || px == WHITE_RGB,
            "non-HUD pixel {px:?} at ({x},{y}) in the top padding band"
        );
        top_white += (px == WHITE_RGB) as usize;
    }
    assert!(top_white > 0, "HUD text missing from the top padding band");
    assert!(pixels(&fb, 16, 24).all(|(_, _, px)| px == BLACK));
    assert_eq!(fb.get(26, 0), WHITE_RGB); // "1UP"
    assert_eq!(fb.get(26, 8), WHITE_RGB); // score 1234

    // Bottom two tile rows (232..248): HUD only — lives + fruit strip at
    // ly=(31-2)*8=232, and no maze or pellet pixels.
    assert_eq!(fb.get(16 + 12, 232 + 7), YELLOW_RGB); // first spare-life icon
    assert_eq!(fb.get(224 + 4, 232 + 4), RED_RGB); // latest fruit (strawberry)
    for (x, y, px) in pixels(&fb, 232, 248) {
        assert!(
            px != MAZE_BLUE_RGB && px != PEACH_RGB,
            "maze/pellet pixel at ({x},{y}) in the bottom padding band"
        );
    }

    // With every pellet present and actors parked mid-maze, the padding
    // bands still hold no maze or pellet pixels (sprite bleed through the
    // vertical-warp tunnels is allowed; pellets/maze are not).
    let mut st = bare_state(&m);
    for y in 0..th {
        for x in 0..tw {
            if matches!(m.cell(TilePos::new(x, y)), Cell::Dot | Cell::Energizer) {
                st.pellets[(y * tw + x) as usize] = true;
            }
        }
    }
    st.pac_visible = true;
    let fb = compose_frame(&m, &st);
    for (x, y, px) in pixels(&fb, 0, 24).chain(pixels(&fb, 232, 248)) {
        assert!(
            px != MAZE_BLUE_RGB && px != PEACH_RGB,
            "maze/pellet pixel at ({x},{y}) in a padding band"
        );
    }
    // Pellets themselves shifted with the maze: Vertigo's map row 1 dot row
    // renders on frame tile row 4 (pixel y = 4*8+3), not on the HUD rows.
    let dot_row_px = 4 * 8 + 3;
    assert!((0..256).any(|x| fb.get(x, dot_row_px) == PEACH_RGB));
}

#[test]
fn custom_map_banners_shift_into_maze_band() {
    // READY!/GAME OVER/PAUSED anchor on the fruit spawn in MAP coordinates,
    // so on a padded map they must render shifted down by pad_top with the
    // maze (live captures kept missing the 2-second READY window; this is
    // the durable proof). Vertigo: fruit center (128,156), pad_top=3 ->
    // banner top row at y = 156 - 4 + 24 = 176, inside the maze band
    // (24..232), below the low-center house where the classic READY sits.
    let m = Map::custom();
    let mut st = bare_state(&m);
    st.sequence = Sequence::Ready { tick: 0 };
    let fb = compose_frame(&m, &st);
    // "READY!" yellow, centered on x=128: 'R' top-left pixel at (104,176).
    assert_eq!(fb.get(104, 176), YELLOW_RGB);
    // Nothing at the unshifted (map-coordinate) anchor row.
    assert_ne!(fb.get(104, 152), YELLOW_RGB);
    // HUD bands stay HUD-only: top band is black + white HUD text (no
    // yellow banner bleed), bottom band holds no maze/pellet pixels (its
    // yellow/red pixels are the lives + fruit HUD sprites).
    assert!(pixels(&fb, 0, 24).all(|(_, _, px)| px == BLACK || px == WHITE_RGB));
    assert!(pixels(&fb, 232, 248).all(|(_, _, px)| px != MAZE_BLUE_RGB && px != PEACH_RGB));

    // Same anchor for the GAME OVER and PAUSED overlays.
    let st2 = bare_state(&m);
    let fb2 = compose_frame_with(&m, &st2, false, true);
    // "GAME  OVER" red, centered on x=128: 'G' top-row pixel at (89,176).
    assert_eq!(fb2.get(89, 176), RED_RGB);
    assert!(pixels(&fb2, 0, 24).all(|(_, _, px)| px == BLACK || px == WHITE_RGB));

    let fb3 = compose_frame_with(&m, &st2, true, false);
    // "PAUSED" white, 32px above the banner row: 'P' at (104,144).
    assert_eq!(fb3.get(104, 144), WHITE_RGB);
}
