//! Scene composition tests through the compositor's public interface: maze
//! auto-tiling, gameplay frame, HUD, menu, loading — all CPU-only composed
//! frames, no tty, no `Map` (synthetic grids drive the `Maze` seam).

use pacmantui::render::compose::{Compositor, Frame, Maze, Overlay, TileKind};
use pacmantui::render::layout::FrameLayout;
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

/// Synthetic-grid adapter on the compositor's `Maze` seam (the test twin of
/// the production `Map` adapter).
struct Grid {
    id: &'static str,
    tw: i32,
    th: i32,
    kind: fn(i32, i32) -> TileKind,
    fruit_px: (i32, i32),
}

impl Maze for Grid {
    fn id(&self) -> &str {
        self.id
    }
    fn size(&self) -> (i32, i32) {
        (self.tw, self.th)
    }
    fn kind(&self, x: i32, y: i32) -> TileKind {
        (self.kind)(x, y)
    }
    fn fruit_px(&self) -> (i32, i32) {
        self.fruit_px
    }
}

fn compose_on(g: &Grid, st: &RenderState, paused: bool, game_over: bool) -> Frame {
    let mut c = Compositor::new();
    c.game(g, st, Overlay { paused, game_over }).clone()
}

/// Bare state for maze-layer tests: no pellets, no visible actors, minimal
/// HUD (one life, no fruit history), so the maze band holds layer pixels
/// only and the HUD bands hold at most white text.
fn bare_state(tw: i32, th: i32) -> RenderState {
    let mut st = base_state();
    st.pac_visible = false;
    for g in &mut st.ghosts {
        g.visible = false;
    }
    st.lives = 1;
    st.fruit_history.clear();
    st.pellets = vec![false; (tw * th) as usize];
    st
}

// --- maze auto-tiling --------------------------------------------------------

fn wall_block_kind(x: i32, y: i32) -> TileKind {
    // 8x8 open grid with a 2x2 wall block at tiles (3,3)-(4,4).
    if (3..=4).contains(&x) && (3..=4).contains(&y) {
        TileKind::Wall
    } else {
        TileKind::Open
    }
}

/// The all-open 8x8 grid pads like Vertigo: maze band starts at pixel 24.
const WOY: i32 = 24;

fn wall_block_grid() -> Grid {
    let g = Grid {
        id: "wall-block-8x8",
        tw: 8,
        th: 8,
        kind: wall_block_kind,
        fruit_px: (32, 32),
    };
    let lay = FrameLayout::new(g.tw, g.th, &g.kind);
    assert_eq!(lay.maze_origin_px(), (0, WOY));
    g
}

#[test]
fn wall_block_outline_with_rounded_corners() {
    let g = wall_block_grid();
    let fb = compose_on(&g, &bare_state(8, 8), false, false);
    // Top contour of tile (3,3): line at local y=2, x 4..7 (trimmed at the
    // rounded corner) -> maze px y=26, x=28..31.
    for x in 28..=31 {
        assert_eq!(fb.get(x, 26 + WOY), MAZE_BLUE_RGB, "top line x={x}");
    }
    // Left contour: maze px x=26, y=28..31.
    for y in 28..=31 {
        assert_eq!(fb.get(26, y + WOY), MAZE_BLUE_RGB, "left line y={y}");
    }
    // Rounded NW corner diagonal pixel at local (3,3) -> (27,27).
    assert_eq!(fb.get(27, 27 + WOY), MAZE_BLUE_RGB);
    // Line continues into tile (4,3): y=26, x=32..35, and its NE corner.
    assert_eq!(fb.get(33, 26 + WOY), MAZE_BLUE_RGB);
    assert_eq!(fb.get(36, 27 + WOY), MAZE_BLUE_RGB);
    // Block interior stays black (no fill, contour only).
    assert_eq!(fb.get(31, 31 + WOY), BLACK);
    assert_eq!(fb.get(32, 32 + WOY), BLACK);
    // Far away stays black.
    assert_eq!(fb.get(4, 4 + WOY), BLACK);
}

#[test]
fn concave_corner_connects_perpendicular_lines() {
    // All wall except a single open tile at (4,4): the walls around it get
    // concave arcs at the diagonal tiles. The single-tile open box embeds
    // its own dead rows, so this grid gets no padding (maze origin 0).
    fn kind(x: i32, y: i32) -> TileKind {
        if (x, y) == (4, 4) {
            TileKind::Open
        } else {
            TileKind::Wall
        }
    }
    let g = Grid {
        id: "concave-9x9",
        tw: 9,
        th: 9,
        kind,
        fruit_px: (36, 36),
    };
    assert_eq!(FrameLayout::new(9, 9, &kind).maze_origin_px(), (0, 0));
    let fb = compose_on(&g, &bare_state(9, 9), false, false);
    // Tile (3,3) is the NW diagonal neighbour: concave SE arc pixels at
    // local (5,7),(6,6),(7,5) -> global (29,31),(30,30),(31,29).
    assert_eq!(fb.get(30, 30), MAZE_BLUE_RGB);
    assert_eq!(fb.get(29, 31), MAZE_BLUE_RGB);
    assert_eq!(fb.get(31, 29), MAZE_BLUE_RGB);
}

#[test]
fn door_renders_pink_bar() {
    fn kind(x: i32, y: i32) -> TileKind {
        if (x, y) == (1, 1) {
            TileKind::Door
        } else {
            TileKind::Open
        }
    }
    let g = Grid {
        id: "door-3x3",
        tw: 3,
        th: 3,
        kind,
        fruit_px: (12, 12),
    };
    let oy = 24; // all-open 3x3 grid pads like Vertigo
    assert_eq!(FrameLayout::new(3, 3, &kind).maze_origin_px(), (0, oy));
    let fb = compose_on(&g, &bare_state(3, 3), false, false);
    for x in 8..16 {
        assert_eq!(fb.get(x, 11 + oy), DOOR_PINK_RGB, "bar x={x}");
        assert_eq!(fb.get(x, 12 + oy), DOOR_PINK_RGB);
    }
    assert_eq!(fb.get(8, 10 + oy), BLACK);
    assert_eq!(fb.get(8, 13 + oy), BLACK);
}

#[test]
fn white_flash_variant_recolors_lines() {
    let g = wall_block_grid();
    let blue = compose_on(&g, &bare_state(8, 8), false, false);
    let mut st = bare_state(8, 8);
    st.sequence = Sequence::LevelFlash {
        tick: timings::LEVEL_CLEAR_FREEZE_TICKS, // first white half-period
    };
    let white = compose_on(&g, &st, false, false);
    assert_eq!(blue.get(28, 26 + WOY), MAZE_BLUE_RGB);
    assert_eq!(white.get(28, 26 + WOY), WHITE_RGB);
    // Same geometry inside the maze band: every blue pixel is white and
    // vice versa (the bare maze band holds nothing but contour lines).
    for y in WOY..(WOY + 64) {
        for x in 0..64 {
            let b = blue.get(x, y);
            let w = white.get(x, y);
            assert_eq!(
                b != BLACK,
                w != BLACK,
                "geometry mismatch at ({x},{y}): blue {b:?} vs white {w:?}"
            );
            if b != BLACK {
                assert_eq!(b, MAZE_BLUE_RGB);
                assert_eq!(w, WHITE_RGB);
            }
        }
    }
}

#[test]
fn dead_space_padding_stays_black_and_border_gets_outer_line() {
    // 6x8 grid shaped like the classic screen: HUD rows 0-1 and 6-7 all wall
    // (dead space), border walls rows 2/5 + cols 0/5, open interior. One
    // embedded dead row on top -> the frame adds 2 more (maze origin 8).
    fn kind(x: i32, y: i32) -> TileKind {
        if y <= 1 || y >= 6 || y == 2 || y == 5 || x == 0 || x == 5 {
            TileKind::Wall
        } else {
            TileKind::Open
        }
    }
    let g = Grid {
        id: "dead-space-6x8",
        tw: 6,
        th: 8,
        kind,
        fruit_px: (24, 32),
    };
    let oy = 8;
    assert_eq!(FrameLayout::new(6, 8, &kind).maze_origin_px(), (0, oy));
    let fb = compose_on(&g, &bare_state(6, 8), false, false);
    // Dead space (map px rows 0..16 and 48..64): the maze layer draws
    // nothing there — only black, or the frame's own white HUD text.
    for y in (0..16).chain(48..64) {
        for x in 0..48 {
            let px = fb.get(x, y + oy);
            assert!(
                px == BLACK || px == WHITE_RGB,
                "non-HUD pixel {px:?} in dead space at ({x},{y})"
            );
            assert_ne!(px, MAZE_BLUE_RGB, "maze pixel in dead space at ({x},{y})");
        }
    }
    // Top border tile (1,2): outer contour line at map px y = 2*8+2 = 18.
    assert_eq!(fb.get(12, 18 + oy), MAZE_BLUE_RGB);
    // And inner contour facing the open interior at y = 2*8+5 = 21.
    assert_eq!(fb.get(12, 21 + oy), MAZE_BLUE_RGB);
}

#[test]
fn flash_phase_from_tick() {
    // Composed-frame phase check: a known wall pixel is white exactly during
    // the flash's white half-periods (after the freeze, for N flashes).
    let f = timings::LEVEL_CLEAR_FREEZE_TICKS;
    let h = timings::LEVEL_CLEAR_FLASH_HALF_TICKS;
    let n = timings::LEVEL_CLEAR_FLASHES;
    let g = wall_block_grid();
    let is_white = |tick: u32| {
        let mut st = bare_state(8, 8);
        st.sequence = Sequence::LevelFlash { tick };
        let fb = compose_on(&g, &st, false, false);
        match fb.get(28, 26 + WOY) {
            WHITE_RGB => true,
            MAZE_BLUE_RGB => false,
            other => panic!("wall pixel is neither blue nor white: {other:?}"),
        }
    };
    assert!(!is_white(0));
    assert!(!is_white(f - 1));
    assert!(is_white(f)); // first white half-period
    assert!(is_white(f + h - 1));
    assert!(!is_white(f + h)); // back to blue
    assert!(is_white(f + 2 * h)); // second flash
    assert!(!is_white(f + 2 * n * h)); // sequence over
    assert!(!is_white(f + 2 * n * h + 100));
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
        pac_visible: true,
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

/// Frame-pixel y of map-pixel y on the test grid: the 10x10 all-maze grid
/// gets 3 padding tile rows on top, so the maze band starts at pixel 24.
const OY: i32 = 24;

fn compose(state: &RenderState, paused: bool, game_over: bool) -> Frame {
    let g = Grid {
        id: "test-10x10",
        tw: 10,
        th: 10,
        kind: test_kind,
        fruit_px: (40, 60), // overlay anchor (map px) inside the test maze
    };
    let lay = FrameLayout::new(g.tw, g.th, &g.kind);
    assert_eq!(lay.maze_origin_px(), (0, OY));
    compose_on(&g, state, paused, game_over)
}

#[test]
fn game_frame_actors_pellets_and_walls() {
    let mut st = base_state();
    st.pellets[2 * 10 + 2] = true; // dot at (2,2)
    st.pellets[3 * 10 + 3] = true; // energizer at (3,3)
    let fb = compose(&st, false, false);

    // Dot: peach 2x2 at the tile center.
    assert_eq!(fb.get(19, 19 + OY), PEACH_RGB);
    assert_eq!(fb.get(20, 20 + OY), PEACH_RGB);
    assert_eq!(fb.get(18, 18 + OY), BLACK);
    // Energizer with blink off: not drawn.
    assert_eq!(fb.get(28, 28 + OY), BLACK);
    // Pac closed disc centered on his position (tile 5,5 center = 44,44).
    assert_eq!(fb.get(43, 43 + OY), YELLOW_RGB);
    // Blinky at tile (7,7): red body, white sclera, blue pupil (looking right).
    assert_eq!(fb.get(57, 53 + OY), RED_RGB);
    assert_eq!(fb.get(54, 56 + OY), WHITE_RGB);
    assert_eq!(fb.get(56, 56 + OY), MAZE_BLUE_RGB);
    // Border wall double line from the maze layer survives compositing.
    assert_eq!(fb.get(2, 44 + OY), MAZE_BLUE_RGB);
    assert_eq!(fb.get(5, 44 + OY), MAZE_BLUE_RGB);
    // Untouched interior is black.
    assert_eq!(fb.get(68, 68 + OY), BLACK);
}

#[test]
fn energizer_blinks_with_state_flag() {
    let mut st = base_state();
    st.pellets[3 * 10 + 3] = true;
    st.energizer_blink_on = true;
    let fb = compose(&st, false, false);
    assert_eq!(fb.get(28, 28 + OY), PEACH_RGB);
    // Missing pellet bit -> nothing, even when blinking.
    let mut st2 = base_state();
    st2.energizer_blink_on = true;
    let fb2 = compose(&st2, false, false);
    assert_eq!(fb2.get(28, 28 + OY), BLACK);
}

#[test]
fn popup_drawn_in_mini_font() {
    let mut st = base_state();
    st.popups.push((TilePos::new(7, 5), 200, 10));
    let fb = compose(&st, false, false);
    // Centered on tile center (60,44): "200" starts at x=55, top row y=42.
    assert_eq!(fb.get(55, 42 + OY), CYAN_RGB);
    assert_eq!(fb.get(56, 42 + OY), CYAN_RGB);
    assert_eq!(fb.get(57, 42 + OY), CYAN_RGB);
}

#[test]
fn fruit_on_board_uses_history_symbol() {
    let mut st = base_state();
    st.fruit = Some(TilePos::new(5, 7)); // center (44,60), sprite at (36,52)
    st.fruit_history = vec![1]; // strawberry: red body with green calyx
    let fb = compose(&st, false, false);
    // Strawberry body pixel local (4,4) -> map px (40,56).
    assert_eq!(fb.get(40, 56 + OY), RED_RGB);
}

#[test]
fn overlays_ready_game_over_paused() {
    let mut st = base_state();
    st.sequence = Sequence::Ready { tick: 0 };
    let fb = compose(&st, false, false);
    // "READY!" yellow, centered on fruit anchor x=40, top map y=56: R at x=16.
    assert_eq!(fb.get(16, 56 + OY), YELLOW_RGB);

    let st2 = base_state();
    let fb2 = compose(&st2, false, true);
    // "GAME  OVER" red: G at x=0, top row bit1 -> map px (1,56).
    assert_eq!(fb2.get(1, 56 + OY), RED_RGB);

    let fb3 = compose(&st2, true, false);
    // "PAUSED" white 32px above the anchor row: P at map px (16,24).
    assert_eq!(fb3.get(16, 24 + OY), WHITE_RGB);
    // Without flags/sequence: nothing at those spots.
    let fb4 = compose(&st2, false, false);
    assert_eq!(fb4.get(16, 56 + OY), BLACK);
    assert_eq!(fb4.get(16, 24 + OY), BLACK);
}

#[test]
fn frightened_and_eyes_ghost_states() {
    let mut st = base_state();
    st.ghosts[0].frightened = true;
    let fb = compose(&st, false, false);
    // Frightened body blue at map px (57,53) instead of red.
    assert_eq!(fb.get(57, 53 + OY), [33, 33, 255]);

    st.fright_flash = Some(true);
    let fb2 = compose(&st, false, false);
    // Flash phase: body white.
    assert_eq!(fb2.get(57, 53 + OY), WHITE_RGB);

    let mut st3 = base_state();
    st3.ghosts[0].state = GhostState::Eyes;
    let fb3 = compose(&st3, false, false);
    // Eyes only: body pixel transparent (black), sclera white remains.
    assert_eq!(fb3.get(57, 53 + OY), BLACK);
    assert_eq!(fb3.get(54, 56 + OY), WHITE_RGB);
}

#[test]
fn death_animation_uses_shrinking_frames() {
    let mut st = base_state();
    st.sequence = Sequence::DeathAnim {
        tick: timings::DEATH_ANIM_TICKS - 1,
    };
    let fb = compose(&st, false, false);
    // Final frames are the white starburst: no yellow left at pac's center.
    assert_ne!(fb.get(43, 43 + OY), YELLOW_RGB);
}

#[test]
fn invisible_ghosts_are_not_drawn() {
    let st = base_state(); // Pinky/Inky/Clyde at (1,1) but invisible
    let fb = compose(&st, false, false);
    // Tile (1,1) center (12,12): sprite would cover (4..20, 4..20); probe
    // body pixels inside tile (1,1), clear of the border wall contour lines.
    assert_eq!(fb.get(12, 12 + OY), BLACK);
    assert_eq!(fb.get(14, 10 + OY), BLACK);
}

// --- HUD ---------------------------------------------------------------------

/// A 28x36 classic-shaped grid (its own dead rows, so zero padding): the
/// frame layout equals the map grid, like the classic map's.
fn classic_shape(_x: i32, y: i32) -> TileKind {
    if (4..=32).contains(&y) {
        TileKind::Open
    } else {
        TileKind::Wall
    }
}

/// Compose a HUD-only frame (no pellets, no visible actors) on the
/// classic-shaped grid: the HUD anchors land exactly as on the classic map.
fn hud_frame(st: &RenderState) -> Frame {
    let g = Grid {
        id: "classic-shape-28x36",
        tw: 28,
        th: 36,
        kind: classic_shape,
        fruit_px: (112, 164),
    };
    let lay = FrameLayout::new(g.tw, g.th, &g.kind);
    assert_eq!(lay.frame_tiles(), (28, 36));
    assert_eq!(lay.maze_origin_px(), (0, 0));
    compose_on(&g, st, false, false)
}

fn hud_state() -> RenderState {
    let mut st = base_state();
    st.pac_visible = false;
    for g in &mut st.ghosts {
        g.visible = false;
    }
    st.pellets = vec![false; 28 * 36];
    st
}

#[test]
fn hud_score_lives_and_fruit_history() {
    let fb = hud_frame(&hud_state());
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
    let mut st = hud_state();
    st.score = 1_000_005;
    st.lives = 1; // no spare icons
    st.fruit_history.clear();
    let fb = hud_frame(&st);
    // Displays "05": two glyphs ending at col 6 -> x=40..56.
    assert_eq!(fb.get(41, 8), WHITE_RGB); // '0' left edge pixel (0x78 bit1)
    // Nothing further left than the two digits on the score row... the high
    // score is drawn from x=104; between x=56 and 104 the row is empty.
    for x in 56..104 {
        assert_eq!(fb.get(x, 8), BLACK, "unexpected pixel at ({x},8)");
    }
}

// --- menu / loading ----------------------------------------------------------

fn menu_frame(screen: &MenuScreen) -> Frame {
    let mut c = Compositor::new();
    c.menu(screen).clone()
}

#[test]
fn menu_title_selection_and_footer() {
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
    let fb = menu_frame(&screen);
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
    let screen = MenuScreen {
        title: "T".into(),
        items: Vec::new(),
        selected: 0,
        footer: String::new(),
        decorated: true,
    };
    let fb = menu_frame(&screen);
    // Blinky body pixel at (40+5, 72+1).
    assert_eq!(fb.get(45, 73), RED_RGB);
    // Pac at x=144: local (3,7) yellow -> (147,79).
    assert_eq!(fb.get(147, 79), YELLOW_RGB);
    // Without decoration that row is empty.
    let mut plain = screen.clone();
    plain.decorated = false;
    let fb2 = menu_frame(&plain);
    assert_eq!(fb2.get(45, 73), BLACK);
}

#[test]
fn loading_bar_reflects_progress() {
    let count_yellow =
        |fb: &Frame| (0..224).filter(|&x| fb.get(x, 151) == YELLOW_RGB).count() as i32;
    let loading_frame = |message: &str, progress: f32| {
        let mut c = Compositor::new();
        c.loading(&LoadingScreen {
            message: message.into(),
            progress,
        })
        .clone()
    };
    for (progress, expect) in [(0.0f32, 0), (0.5, 78), (1.0, 156)] {
        let fb = loading_frame("LOADING MAPS", progress);
        assert_eq!(count_yellow(&fb), expect, "progress {progress}");
        // Border corners white.
        assert_eq!(fb.get(32, 148), WHITE_RGB);
        assert_eq!(fb.get(191, 159), WHITE_RGB);
    }
    // Out-of-range progress clamps.
    let fb = loading_frame("", 7.0);
    assert_eq!(count_yellow(&fb), 156);
}
