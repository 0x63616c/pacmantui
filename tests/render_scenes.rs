//! Scene composition tests: maze auto-tiling, gameplay frame, HUD, menu,
//! loading — all CPU-only against the framebuffer, no tty, no `Map`.

use pacmantui::render::test_api::*;
use pacmantui::render::{LoadingScreen, MenuItem, MenuScreen};
use pacmantui::sim::timings;
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
const CYAN_RGB: [u8; 3] = [0, 255, 255];

fn layer_px(layer: &[u8], wpx: usize, x: i32, y: i32) -> [u8; 3] {
    let o = (y as usize * wpx + x as usize) * 3;
    [layer[o], layer[o + 1], layer[o + 2]]
}

// --- maze auto-tiling --------------------------------------------------------

#[test]
fn wall_block_outline_with_rounded_corners() {
    // 8x8 open grid with a 2x2 wall block at tiles (3,3)-(4,4).
    let kind = |x: i32, y: i32| {
        if (3..=4).contains(&x) && (3..=4).contains(&y) {
            TileKind::Wall
        } else {
            TileKind::Open
        }
    };
    let layer = compose_maze_layer(8, 8, &kind, false);
    let w = 64;
    // Top contour of tile (3,3): line at local y=2, x 4..7 (trimmed at the
    // rounded corner) -> global y=26, x=28..31.
    for x in 28..=31 {
        assert_eq!(layer_px(&layer, w, x, 26), MAZE_BLUE_RGB, "top line x={x}");
    }
    // Left contour: global x=26, y=28..31.
    for y in 28..=31 {
        assert_eq!(layer_px(&layer, w, 26, y), MAZE_BLUE_RGB, "left line y={y}");
    }
    // Rounded NW corner diagonal pixel at local (3,3) -> (27,27).
    assert_eq!(layer_px(&layer, w, 27, 27), MAZE_BLUE_RGB);
    // Line continues into tile (4,3): y=26, x=32..35, and its NE corner.
    assert_eq!(layer_px(&layer, w, 33, 26), MAZE_BLUE_RGB);
    assert_eq!(layer_px(&layer, w, 36, 27), MAZE_BLUE_RGB);
    // Block interior stays black (no fill, contour only).
    assert_eq!(layer_px(&layer, w, 31, 31), BLACK);
    assert_eq!(layer_px(&layer, w, 32, 32), BLACK);
    // Far away stays black.
    assert_eq!(layer_px(&layer, w, 4, 4), BLACK);
}

#[test]
fn concave_corner_connects_perpendicular_lines() {
    // All wall except a single open tile at (4,4): the walls around it get
    // concave arcs at the diagonal tiles.
    let kind = |x: i32, y: i32| {
        if (x, y) == (4, 4) {
            TileKind::Open
        } else {
            TileKind::Wall
        }
    };
    let layer = compose_maze_layer(9, 9, &kind, false);
    let w = 72;
    // Tile (3,3) is the NW diagonal neighbour: concave SE arc pixels at
    // local (5,7),(6,6),(7,5) -> global (29,31),(30,30),(31,29).
    assert_eq!(layer_px(&layer, w, 30, 30), MAZE_BLUE_RGB);
    assert_eq!(layer_px(&layer, w, 29, 31), MAZE_BLUE_RGB);
    assert_eq!(layer_px(&layer, w, 31, 29), MAZE_BLUE_RGB);
}

#[test]
fn door_renders_pink_bar() {
    let kind = |x: i32, y: i32| {
        if (x, y) == (1, 1) {
            TileKind::Door
        } else {
            TileKind::Open
        }
    };
    let layer = compose_maze_layer(3, 3, &kind, false);
    let w = 24;
    for x in 8..16 {
        assert_eq!(layer_px(&layer, w, x, 11), DOOR_PINK_RGB, "bar x={x}");
        assert_eq!(layer_px(&layer, w, x, 12), DOOR_PINK_RGB);
    }
    assert_eq!(layer_px(&layer, w, 8, 10), BLACK);
    assert_eq!(layer_px(&layer, w, 8, 13), BLACK);
}

#[test]
fn white_flash_variant_recolors_lines() {
    let kind = |x: i32, y: i32| {
        if (3..=4).contains(&x) && (3..=4).contains(&y) {
            TileKind::Wall
        } else {
            TileKind::Open
        }
    };
    let blue = compose_maze_layer(8, 8, &kind, false);
    let white = compose_maze_layer(8, 8, &kind, true);
    let w = 64;
    assert_eq!(layer_px(&blue, w, 28, 26), MAZE_BLUE_RGB);
    assert_eq!(layer_px(&white, w, 28, 26), WHITE_RGB);
    // Same geometry: every blue pixel is white and vice versa.
    let blue_set: Vec<bool> = blue.chunks(3).map(|c| c != [0, 0, 0]).collect();
    let white_set: Vec<bool> = white.chunks(3).map(|c| c != [0, 0, 0]).collect();
    assert_eq!(blue_set, white_set);
}

#[test]
fn dead_space_padding_stays_black_and_border_gets_outer_line() {
    // 6x8 grid shaped like the classic screen: HUD rows 0-1 and 6-7 all wall
    // (dead space), border walls rows 2/5 + cols 0/5, open interior.
    let kind = |x: i32, y: i32| {
        if y <= 1 || y >= 6 || y == 2 || y == 5 || x == 0 || x == 5 {
            TileKind::Wall
        } else {
            TileKind::Open
        }
    };
    let layer = compose_maze_layer(6, 8, &kind, false);
    let w = 48;
    // HUD dead space: nothing drawn in pixel rows 0..16 or 48..64.
    for y in (0..16).chain(48..64) {
        for x in 0..48 {
            assert_eq!(layer_px(&layer, w, x, y), BLACK, "dead space at ({x},{y})");
        }
    }
    // Top border tile (1,2): outer contour line at global y = 2*8+2 = 18.
    assert_eq!(layer_px(&layer, w, 12, 18), MAZE_BLUE_RGB);
    // And inner contour facing the open interior at y = 2*8+5 = 21.
    assert_eq!(layer_px(&layer, w, 12, 21), MAZE_BLUE_RGB);
}

#[test]
fn flash_phase_from_tick() {
    let f = timings::LEVEL_CLEAR_FREEZE_TICKS;
    let h = timings::LEVEL_CLEAR_FLASH_HALF_TICKS;
    let n = timings::LEVEL_CLEAR_FLASHES;
    assert!(!flash_is_white(0));
    assert!(!flash_is_white(f - 1));
    assert!(flash_is_white(f)); // first white half-period
    assert!(flash_is_white(f + h - 1));
    assert!(!flash_is_white(f + h)); // back to blue
    assert!(flash_is_white(f + 2 * h)); // second flash
    assert!(!flash_is_white(f + 2 * n * h)); // sequence over
    assert!(!flash_is_white(f + 2 * n * h + 100));
}

// --- gameplay frame ----------------------------------------------------------

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

fn base_state() -> RenderState {
    RenderState {
        pac_pos: PxPos::tile_center(TilePos::new(5, 5)),
        pac_dir: Dir::Left,
        pac_anim: 0,
        ghosts: [
            ghost(GhostId::Blinky, (7, 7), true),
            ghost(GhostId::Pinky, (1, 1), false),
            ghost(GhostId::Inky, (1, 1), false),
            ghost(GhostId::Clyde, (1, 1), false),
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
        pellets: vec![false; 100],
        energizer_blink_on: false,
    }
}

fn test_kind(x: i32, y: i32) -> TileKind {
    if x == 0 || y == 0 || x == 9 || y == 9 {
        TileKind::Wall
    } else if (x, y) == (2, 2) {
        TileKind::Dot
    } else if (x, y) == (3, 3) {
        TileKind::Energizer
    } else {
        TileKind::Open
    }
}

fn compose(state: &RenderState, paused: bool, game_over: bool) -> Frame {
    let layer = compose_maze_layer(10, 10, &test_kind, false);
    let mut fb = Frame::new(80, 80);
    let view = GameView {
        layer: &layer,
        tw: 10,
        th: 10,
        state,
        fruit_px: (40, 60), // overlay anchor inside the 80x80 test frame
        paused,
        game_over,
    };
    draw_game(&mut fb, &view, test_kind);
    fb
}

#[test]
fn game_frame_actors_pellets_and_walls() {
    let mut st = base_state();
    st.pellets[2 * 10 + 2] = true; // dot at (2,2)
    st.pellets[3 * 10 + 3] = true; // energizer at (3,3)
    let fb = compose(&st, false, false);

    // Dot: peach 2x2 at the tile center.
    assert_eq!(fb.get(19, 19), PEACH_RGB);
    assert_eq!(fb.get(20, 20), PEACH_RGB);
    assert_eq!(fb.get(18, 18), BLACK);
    // Energizer with blink off: not drawn.
    assert_eq!(fb.get(28, 28), BLACK);
    // Pac closed disc centered on his position (tile 5,5 center = 44,44).
    assert_eq!(fb.get(43, 43), YELLOW_RGB);
    // Blinky at tile (7,7): red body, white sclera, blue pupil (looking right).
    assert_eq!(fb.get(57, 53), RED_RGB);
    assert_eq!(fb.get(54, 56), WHITE_RGB);
    assert_eq!(fb.get(56, 56), MAZE_BLUE_RGB);
    // Border wall double line from the maze layer survives compositing.
    assert_eq!(fb.get(2, 44), MAZE_BLUE_RGB);
    assert_eq!(fb.get(5, 44), MAZE_BLUE_RGB);
    // Untouched interior is black.
    assert_eq!(fb.get(68, 68), BLACK);
}

#[test]
fn energizer_blinks_with_state_flag() {
    let mut st = base_state();
    st.pellets[3 * 10 + 3] = true;
    st.energizer_blink_on = true;
    let fb = compose(&st, false, false);
    assert_eq!(fb.get(28, 28), PEACH_RGB);
    // Missing pellet bit -> nothing, even when blinking.
    let mut st2 = base_state();
    st2.energizer_blink_on = true;
    let fb2 = compose(&st2, false, false);
    assert_eq!(fb2.get(28, 28), BLACK);
}

#[test]
fn popup_drawn_in_mini_font() {
    let mut st = base_state();
    st.popups.push((TilePos::new(7, 5), 200, 10));
    let fb = compose(&st, false, false);
    // Centered on tile center (60,44): "200" starts at x=55, top row y=42.
    assert_eq!(fb.get(55, 42), CYAN_RGB);
    assert_eq!(fb.get(56, 42), CYAN_RGB);
    assert_eq!(fb.get(57, 42), CYAN_RGB);
}

#[test]
fn fruit_on_board_uses_history_symbol() {
    let mut st = base_state();
    st.fruit = Some(TilePos::new(5, 7)); // center (44,60), sprite at (36,52)
    st.fruit_history = vec![1]; // strawberry: red body with green calyx
    let fb = compose(&st, false, false);
    // Strawberry body pixel local (4,4) -> (40,56).
    assert_eq!(fb.get(40, 56), RED_RGB);
}

#[test]
fn overlays_ready_game_over_paused() {
    let mut st = base_state();
    st.sequence = Sequence::Ready { tick: 0 };
    let fb = compose(&st, false, false);
    // "READY!" yellow, centered on fruit anchor x=40, top y=56: R at x=16.
    assert_eq!(fb.get(16, 56), YELLOW_RGB);

    let st2 = base_state();
    let fb2 = compose(&st2, false, true);
    // "GAME  OVER" red: G at x=0, top row bit1 -> (1,56).
    assert_eq!(fb2.get(1, 56), RED_RGB);

    let fb3 = compose(&st2, true, false);
    // "PAUSED" white 32px above the anchor row: P at (16,24).
    assert_eq!(fb3.get(16, 24), WHITE_RGB);
    // Without flags/sequence: nothing at those spots.
    let fb4 = compose(&st2, false, false);
    assert_eq!(fb4.get(16, 56), BLACK);
    assert_eq!(fb4.get(16, 24), BLACK);
}

#[test]
fn frightened_and_eyes_ghost_states() {
    let mut st = base_state();
    st.ghosts[0].frightened = true;
    let fb = compose(&st, false, false);
    // Frightened body blue at (57,53) instead of red.
    assert_eq!(fb.get(57, 53), [33, 33, 255]);

    st.fright_flash = Some(true);
    let fb2 = compose(&st, false, false);
    // Flash phase: body white.
    assert_eq!(fb2.get(57, 53), WHITE_RGB);

    let mut st3 = base_state();
    st3.ghosts[0].state = GhostState::Eyes;
    let fb3 = compose(&st3, false, false);
    // Eyes only: body pixel transparent (black), sclera white remains.
    assert_eq!(fb3.get(57, 53), BLACK);
    assert_eq!(fb3.get(54, 56), WHITE_RGB);
}

#[test]
fn death_animation_uses_shrinking_frames() {
    let mut st = base_state();
    st.sequence = Sequence::DeathAnim {
        tick: timings::DEATH_ANIM_TICKS - 1,
    };
    let fb = compose(&st, false, false);
    // Final frames are the white starburst: no yellow left at pac's center.
    assert_ne!(fb.get(43, 43), YELLOW_RGB);
}

#[test]
fn invisible_ghosts_are_not_drawn() {
    let st = base_state(); // Pinky/Inky/Clyde at (1,1) but invisible
    let fb = compose(&st, false, false);
    // Tile (1,1) center (12,12): sprite would cover (4..20, 4..20); probe
    // body pixels inside tile (1,1), clear of the border wall contour lines.
    assert_eq!(fb.get(12, 12), BLACK);
    assert_eq!(fb.get(14, 10), BLACK);
}

// --- HUD ---------------------------------------------------------------------

#[test]
fn hud_score_lives_and_fruit_history() {
    let mut fb = Frame::new(224, 288);
    let st = base_state();
    draw_hud(&mut fb, &st, 28, 36);
    // "1UP" at col 3: glyph '1' top row pixels at (26,0),(27,0).
    assert_eq!(fb.get(26, 0), WHITE_RGB);
    // Score 1234 right-aligned ending col 6: text starts x=24, row y=8.
    assert_eq!(fb.get(26, 8), WHITE_RGB); // '1' first digit
    // High score 5000 right-aligned ending col 16: '5' starts at x=104.
    assert_eq!(fb.get(104, 8), WHITE_RGB);
    // Spare lives (3 lives -> 2 icons) at x=16 and x=32, y=272.
    assert_eq!(fb.get(16 + 12, 272 + 7), YELLOW_RGB);
    assert_eq!(fb.get(32 + 12, 272 + 7), YELLOW_RGB);
    assert_eq!(fb.get(48 + 12, 272 + 7), BLACK); // no third icon
    // Fruit history [cherry, strawberry]: strawberry (latest) at x=192,
    // cherry at x=176.
    assert_eq!(fb.get(192 + 4, 272 + 4), RED_RGB); // strawberry body
    assert_eq!(fb.get(176 + 2, 272 + 7), RED_RGB); // cherry left fruit
}

#[test]
fn hud_score_display_rolls_at_a_million() {
    let mut fb = Frame::new(224, 288);
    let mut st = base_state();
    st.score = 1_000_005;
    st.lives = 1; // no spare icons
    st.fruit_history.clear();
    draw_hud(&mut fb, &st, 28, 36);
    // Displays "05": two glyphs ending at col 6 -> x=40..56.
    assert_eq!(fb.get(41, 8), WHITE_RGB); // '0' left edge pixel (0x78 bit1)
    // Nothing further left than the two digits on the score row... the high
    // score is drawn from x=104; between x=56 and 104 the row is empty.
    for x in 56..104 {
        assert_eq!(fb.get(x, 8), BLACK, "unexpected pixel at ({x},8)");
    }
}

// --- menu / loading ----------------------------------------------------------

#[test]
fn menu_title_selection_and_footer() {
    let mut fb = Frame::new(224, 288);
    let screen = MenuScreen {
        title: "PACMANTUI".into(),
        items: vec![
            MenuItem {
                label: "PLAY".into(),
                value: Some("ON".into()),
                enabled: true,
            },
            MenuItem {
                label: "SECOND".into(),
                value: None,
                enabled: true,
            },
            MenuItem {
                label: "LOCKED".into(),
                value: None,
                enabled: false,
            },
        ],
        selected: 0,
        footer: "ARROWS MOVE".into(),
        decorated: false,
    };
    compose_menu(&mut fb, &screen);
    // Title 2x, centered: 9 chars * 16 = 144 -> x=40; 'P' row0 -> (40,32).
    assert_eq!(fb.get(40, 32), YELLOW_RGB);
    // Selected marker '>' at x=24, first item row y=128.
    assert_eq!(fb.get(24, 128), YELLOW_RGB);
    // Selected label yellow at x=40.
    assert_eq!(fb.get(40, 128), YELLOW_RGB);
    // Value column right-aligned: "ON" at x=184: 'O' row0 bit1 -> (185,128).
    assert_eq!(fb.get(185, 128), YELLOW_RGB);
    // Unselected item white: 'S' row0 bit1 -> (41,144).
    assert_eq!(fb.get(41, 144), WHITE_RGB);
    // Disabled item gray: 'L' row0 bit0 -> (40,160).
    assert_eq!(fb.get(40, 160), [120, 120, 120]);
    // Footer centered at y=272: 11 chars * 8 = 88 -> x=68: 'A' bit1 -> (69,272).
    assert_eq!(fb.get(69, 272), [120, 120, 120]);
    // Background stays black.
    assert_eq!(fb.get(0, 0), BLACK);
    assert_eq!(fb.get(223, 287), BLACK);
}

#[test]
fn menu_decoration_parade() {
    let mut fb = Frame::new(224, 288);
    let screen = MenuScreen {
        title: "T".into(),
        items: Vec::new(),
        selected: 0,
        footer: String::new(),
        decorated: true,
    };
    compose_menu(&mut fb, &screen);
    // Blinky body pixel at (40+5, 72+1).
    assert_eq!(fb.get(45, 73), RED_RGB);
    // Pac at x=144: local (3,7) yellow -> (147,79).
    assert_eq!(fb.get(147, 79), YELLOW_RGB);
    // Without decoration that row is empty.
    let mut fb2 = Frame::new(224, 288);
    let mut plain = screen.clone();
    plain.decorated = false;
    compose_menu(&mut fb2, &plain);
    assert_eq!(fb2.get(45, 73), BLACK);
}

#[test]
fn loading_bar_reflects_progress() {
    let count_yellow =
        |fb: &Frame| (0..224).filter(|&x| fb.get(x, 151) == YELLOW_RGB).count() as i32;
    for (progress, expect) in [(0.0f32, 0), (0.5, 78), (1.0, 156)] {
        let mut fb = Frame::new(224, 288);
        let screen = LoadingScreen {
            message: "LOADING MAPS".into(),
            progress,
        };
        compose_loading(&mut fb, &screen);
        assert_eq!(count_yellow(&fb), expect, "progress {progress}");
        // Border corners white.
        assert_eq!(fb.get(32, 148), WHITE_RGB);
        assert_eq!(fb.get(191, 159), WHITE_RGB);
    }
    // Out-of-range progress clamps.
    let mut fb = Frame::new(224, 288);
    compose_loading(
        &mut fb,
        &LoadingScreen {
            message: String::new(),
            progress: 7.0,
        },
    );
    assert_eq!(count_yellow(&fb), 156);
}
