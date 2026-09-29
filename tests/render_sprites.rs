//! Sprite art invariants asserted on composed frames through the
//! compositor's public interface: cell confinement, symmetry, direction
//! derivation, remaps, fruit signatures, death-animation shrink (CPU-only).
//!
//! Stage: a 20x20 all-open grid (one energizer cell), one actor at a time at
//! tile (9,9) — sprite box top-left at frame px (68,92) — probed by
//! sprite-local coordinates.

use pacmantui::render::compose::{Compositor, Frame, Maze, Overlay, TileKind};
use pacmantui::render::layout::FrameLayout;
use pacmantui::types::{
    Dir, GhostId, GhostRender, GhostState, PxPos, RenderState, Sequence, TilePos,
};

const BLACK: [u8; 3] = [0, 0, 0];
const YELLOW_RGB: [u8; 3] = [255, 255, 0];
const RED_RGB: [u8; 3] = [255, 0, 0];
const PINK_RGB: [u8; 3] = [255, 184, 255];
const CYAN_RGB: [u8; 3] = [0, 255, 255];
const ORANGE_RGB: [u8; 3] = [255, 184, 82];
const FRIGHT_BLUE_RGB: [u8; 3] = [33, 33, 255];
const WHITE_RGB: [u8; 3] = [255, 255, 255];
const MAZE_BLUE_RGB: [u8; 3] = [33, 33, 222];
const PEACH_RGB: [u8; 3] = [255, 184, 174];
const GREEN_RGB: [u8; 3] = [0, 255, 0];
const TAN_RGB: [u8; 3] = [222, 151, 73];

/// Maze-band origin of the all-open stage grid (pads like Vertigo).
const OY: i32 = 24;
/// Actor tile (9,9): 16x16 sprite box top-left in frame px.
const BX: i32 = 68;
const BY: i32 = 92;
/// Energizer cell (5,5): 8x8 box top-left in frame px.
const EX: i32 = 40;
const EY: i32 = 64;

struct Stage;

fn stage_kind(x: i32, y: i32) -> TileKind {
    if (x, y) == (5, 5) {
        TileKind::Energizer
    } else {
        TileKind::Open
    }
}

impl Maze for Stage {
    fn id(&self) -> &str {
        "sprite-stage-20x20"
    }
    fn size(&self) -> (i32, i32) {
        (20, 20)
    }
    fn kind(&self, x: i32, y: i32) -> TileKind {
        stage_kind(x, y)
    }
    fn fruit_px(&self) -> (i32, i32) {
        (80, 80)
    }
}

/// Empty stage state: minimal HUD (no spare lives, no fruit strip), no
/// pellets, all actors invisible — a composed frame shows exactly the one
/// sprite a test then switches on.
fn stage_state() -> RenderState {
    let ghost = |id| GhostRender {
        id,
        pos: PxPos::tile_center(TilePos::new(9, 9)),
        dir: Dir::Right,
        state: GhostState::Active,
        frightened: false,
        anim: 0,
        visible: false,
    };
    RenderState {
        pac_pos: PxPos::tile_center(TilePos::new(9, 9)),
        pac_dir: Dir::Right,
        pac_anim: 0,
        ghosts: [
            ghost(GhostId::Blinky),
            ghost(GhostId::Pinky),
            ghost(GhostId::Inky),
            ghost(GhostId::Clyde),
        ],
        fright_flash: None,
        fruit: None,
        popups: Vec::new(),
        score: 0,
        high_score: 0,
        lives: 1,
        level: 1,
        fruit_history: Vec::new(),
        sequence: Sequence::Playing,
        pellets: vec![false; 400],
        energizer_blink_on: false,
        pac_visible: false,
    }
}

fn compose(st: &RenderState) -> Frame {
    let lay = FrameLayout::new(20, 20, &stage_kind);
    assert_eq!(lay.maze_origin_px(), (0, OY));
    let mut c = Compositor::new();
    c.game(&Stage, st, Overlay::default()).clone()
}

/// The sprite's 16x16 box as sprite-local pixels.
fn actor_box(fb: &Frame) -> Vec<[u8; 3]> {
    let mut out = Vec::with_capacity(256);
    for y in 0..16 {
        for x in 0..16 {
            out.push(fb.get(BX + x, BY + y));
        }
    }
    out
}

fn at(px: &[[u8; 3]], x: usize, y: usize) -> [u8; 3] {
    px[y * 16 + x]
}

fn count(px: &[[u8; 3]], rgb: [u8; 3]) -> usize {
    px.iter().filter(|&&p| p == rgb).count()
}

/// Assert every non-black maze-band pixel lies inside the given frame box.
fn assert_confined(fb: &Frame, name: &str, x0: i32, y0: i32, w: i32, h: i32) {
    let mut lit = 0usize;
    for y in OY..(OY + 160) {
        for x in 0..160 {
            if fb.get(x, y) != BLACK {
                lit += 1;
                assert!(
                    (x0..x0 + w).contains(&x) && (y0..y0 + h).contains(&y),
                    "{name}: pixel outside its cell at ({x},{y})"
                );
            }
        }
    }
    assert!(lit > 0, "{name}: sprite drew nothing");
}

// --- cell confinement (sprite metrics, composed) -----------------------------

#[test]
fn sprites_confine_to_their_cells() {
    // Pac: closed and all four open directions.
    let mut st = stage_state();
    st.pac_visible = true;
    assert_confined(&compose(&st), "pac closed", BX, BY, 16, 16);
    for d in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
        st.pac_dir = d;
        st.pac_anim = 2;
        assert_confined(&compose(&st), "pac open", BX, BY, 16, 16);
    }

    // Ghost body (both wave frames), frightened (both), eyes (all dirs).
    let mut st = stage_state();
    st.ghosts[0].visible = true;
    for anim in [0, 1] {
        st.ghosts[0].anim = anim;
        assert_confined(&compose(&st), "ghost body", BX, BY, 16, 16);
    }
    st.ghosts[0].frightened = true;
    for anim in [0, 1] {
        st.ghosts[0].anim = anim;
        assert_confined(&compose(&st), "frightened", BX, BY, 16, 16);
    }
    st.ghosts[0].frightened = false;
    st.ghosts[0].state = GhostState::Eyes;
    for d in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
        st.ghosts[0].dir = d;
        assert_confined(&compose(&st), "eyes", BX, BY, 16, 16);
    }

    // All eight fruit symbols.
    let mut st = stage_state();
    st.fruit = Some(TilePos::new(9, 9));
    for i in 0..8 {
        st.fruit_history = vec![i];
        assert_confined(&compose(&st), "fruit", BX, BY, 16, 16);
    }

    // Energizer: an 8x8 cell.
    let mut st = stage_state();
    st.pellets[5 * 20 + 5] = true;
    st.energizer_blink_on = true;
    assert_confined(&compose(&st), "energizer", EX, EY, 8, 8);
}

// --- pac ---------------------------------------------------------------------

#[test]
fn pac_closed_is_a_symmetric_disc() {
    let mut st = stage_state();
    st.pac_visible = true;
    let px = actor_box(&compose(&st));
    for y in 0..16 {
        for x in 0..16 {
            assert_eq!(at(&px, x, y), at(&px, 15 - x, y), "asym at ({x},{y})");
        }
    }
    // ~13px disc: a solid chunk of yellow, transparent border ring.
    let yellow = count(&px, YELLOW_RGB);
    assert!((120..=160).contains(&yellow), "disc size {yellow}");
    for i in 0..16 {
        assert_eq!(at(&px, i, 0), BLACK);
        assert_eq!(at(&px, i, 15), BLACK);
        assert_eq!(at(&px, 0, i), BLACK);
        assert_eq!(at(&px, 15, i), BLACK);
    }
}

#[test]
fn mouth_wedge_faces_each_direction() {
    let mut st = stage_state();
    st.pac_visible = true;
    let closed = actor_box(&compose(&st));
    st.pac_anim = 2;
    let mut open = |d: Dir| {
        st.pac_dir = d;
        actor_box(&compose(&st))
    };
    let (r, l, u, d) = (
        open(Dir::Right),
        open(Dir::Left),
        open(Dir::Up),
        open(Dir::Down),
    );
    // Mouth pixel deep on the facing side is cut out of the disc.
    assert_eq!(at(&closed, 12, 7), YELLOW_RGB);
    assert_eq!(at(&r, 12, 7), BLACK);
    assert_eq!(at(&l, 3, 7), BLACK);
    assert_eq!(at(&d, 7, 12), BLACK);
    assert_eq!(at(&u, 7, 3), BLACK);
    // The back of the head stays solid.
    assert_eq!(at(&r, 3, 7), YELLOW_RGB);
    assert_eq!(at(&l, 12, 7), YELLOW_RGB);
    assert_eq!(at(&u, 7, 12), YELLOW_RGB);
    assert_eq!(at(&d, 7, 3), YELLOW_RGB);
}

#[test]
fn chomp_cycle_mapping() {
    let mut st = stage_state();
    st.pac_visible = true;
    let mut frame = |d: Dir, anim: u8| {
        st.pac_dir = d;
        st.pac_anim = anim;
        compose(&st)
    };
    // Phase 0 -> direction-neutral closed disc for every direction.
    let closed = frame(Dir::Right, 0);
    for d in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
        assert_eq!(frame(d, 0).data(), closed.data(), "phase 0 not neutral");
        assert_eq!(frame(d, 4).data(), closed.data(), "phase 4 not closed");
    }
    // Phases 1 and 3 -> the same half frame; phase 2 -> full open.
    assert_eq!(frame(Dir::Right, 1).data(), frame(Dir::Right, 3).data());
    // Half-open is between closed and open in mouth size.
    let c = count(&actor_box(&frame(Dir::Right, 0)), YELLOW_RGB);
    let h = count(&actor_box(&frame(Dir::Right, 1)), YELLOW_RGB);
    let o = count(&actor_box(&frame(Dir::Right, 2)), YELLOW_RGB);
    assert!(c > h && h > o, "closed {c} > half {h} > open {o}");
}

#[test]
fn death_animation_shrinks_then_bursts() {
    // DEATH_ANIM_TICKS ticks map onto 8 sprite frames; tick 20*f lands on
    // frame f (160/8 = 20 ticks per frame).
    let mut st = stage_state();
    st.pac_visible = true;
    let mut frame = |tick: u32| {
        st.sequence = Sequence::DeathAnim { tick };
        compose(&st)
    };
    let mut prev = usize::MAX;
    for f in 0..=5u32 {
        let y = count(&actor_box(&frame(f * 20)), YELLOW_RGB);
        assert!(y <= prev, "frame {f} grew: {y} > {prev}");
        assert!(y > 0, "frames 0..=5 keep some body");
        prev = y;
    }
    for f in [6u32, 7] {
        let px = actor_box(&frame(f * 20));
        assert_eq!(count(&px, YELLOW_RGB), 0, "burst frames are white only");
        assert!(count(&px, WHITE_RGB) > 0);
    }
    // Clamped beyond the last frame.
    assert_eq!(frame(7 * 20).data(), frame(1000).data());
}

// --- ghosts ------------------------------------------------------------------

#[test]
fn ghost_wave_frames_differ_only_in_skirt() {
    let mut st = stage_state();
    st.ghosts[0].visible = true;
    let a = actor_box(&compose(&st));
    st.ghosts[0].anim = 1;
    let b = actor_box(&compose(&st));
    assert_ne!(a, b);
    // Rows above the skirt are identical.
    assert_eq!(&a[..14 * 16], &b[..14 * 16]);
    assert!(count(&a, RED_RGB) > 100, "solid body fill");
}

#[test]
fn eyes_look_toward_direction() {
    let mut st = stage_state();
    st.ghosts[0].visible = true;
    st.ghosts[0].state = GhostState::Eyes;
    let mut eyes = |d: Dir| {
        st.ghosts[0].dir = d;
        actor_box(&compose(&st))
    };
    let (r, l, u, d) = (
        eyes(Dir::Right),
        eyes(Dir::Left),
        eyes(Dir::Up),
        eyes(Dir::Down),
    );
    // Pupils (blue) at direction-dependent spots; sclera white around them.
    assert_eq!(at(&r, 4, 4), MAZE_BLUE_RGB);
    assert_eq!(at(&r, 11, 4), MAZE_BLUE_RGB);
    assert_eq!(at(&l, 2, 4), MAZE_BLUE_RGB);
    assert_eq!(at(&l, 9, 4), MAZE_BLUE_RGB);
    assert_eq!(at(&u, 3, 3), MAZE_BLUE_RGB);
    assert_eq!(at(&d, 3, 6), MAZE_BLUE_RGB);
    // The base sclera stays white where no pupil sits.
    assert_eq!(at(&r, 2, 4), WHITE_RGB);
    assert_eq!(at(&l, 4, 4), WHITE_RGB);
}

#[test]
fn ghost_color_remaps() {
    // One ghost per compose: body pixel takes the ghost's color while the
    // eye sclera/pupil stay white/blue (the remap touches only the body).
    let colors = [
        (0usize, RED_RGB), // Blinky
        (1, PINK_RGB),     // Pinky
        (2, CYAN_RGB),     // Inky
        (3, ORANGE_RGB),   // Clyde
    ];
    for (idx, rgb) in colors {
        let mut st = stage_state();
        st.ghosts[idx].visible = true;
        st.ghosts[idx].pos = PxPos::tile_center(TilePos::new(9, 9));
        let px = actor_box(&compose(&st));
        assert_eq!(at(&px, 5, 1), rgb, "ghost {idx} body color");
        assert_eq!(at(&px, 2, 4), WHITE_RGB, "ghost {idx} sclera");
        assert_eq!(at(&px, 4, 4), MAZE_BLUE_RGB, "ghost {idx} pupil");
    }
}

#[test]
fn frightened_and_flash_variant() {
    // Blue body with white face features.
    let mut st = stage_state();
    st.ghosts[0].visible = true;
    st.ghosts[0].frightened = true;
    st.fright_flash = Some(false);
    let a = actor_box(&compose(&st));
    assert!(count(&a, FRIGHT_BLUE_RGB) > 100);
    assert!(count(&a, WHITE_RGB) > 10);
    // Second wave frame is a distinct sprite.
    st.ghosts[0].anim = 1;
    let b = actor_box(&compose(&st));
    assert_ne!(a, b);
    // Flash remap: blue -> white, white -> red; transparent stays.
    st.ghosts[0].anim = 0;
    st.fright_flash = Some(true);
    let f = actor_box(&compose(&st));
    assert!(count(&f, WHITE_RGB) > 100, "flash body white");
    assert!(count(&f, RED_RGB) > 10, "flash face red");
    assert_eq!(at(&f, 0, 0), BLACK, "transparent stays transparent");
}

// --- fruit / energizer / lives ----------------------------------------------

#[test]
fn fruit_signature_colors() {
    let sig: [(u8, &[[u8; 3]]); 8] = [
        (0, &[RED_RGB, TAN_RGB, GREEN_RGB]),     // cherry
        (1, &[RED_RGB, GREEN_RGB, WHITE_RGB]),   // strawberry
        (2, &[ORANGE_RGB, GREEN_RGB]),           // peach
        (3, &[RED_RGB, TAN_RGB]),                // apple
        (4, &[GREEN_RGB, TAN_RGB]),              // grapes
        (5, &[CYAN_RGB, RED_RGB, YELLOW_RGB]),   // galaxian
        (6, &[YELLOW_RGB, WHITE_RGB, CYAN_RGB]), // bell
        (7, &[CYAN_RGB, WHITE_RGB]),             // key
    ];
    let mut st = stage_state();
    st.fruit = Some(TilePos::new(9, 9));
    for (idx, colors) in sig {
        st.fruit_history = vec![idx];
        let px = actor_box(&compose(&st));
        for &c in colors {
            assert!(count(&px, c) > 0, "fruit {idx} missing color {c:?}");
        }
        assert!(count(&px, BLACK) > 60, "fruit {idx} must not fill the cell");
    }
    // Defensive wrap: symbol 8 draws as symbol 0.
    st.fruit_history = vec![8];
    let wrapped = compose(&st);
    st.fruit_history = vec![0];
    assert_eq!(wrapped.data(), compose(&st).data());
}

#[test]
fn energizer_is_a_peach_blob() {
    let mut st = stage_state();
    st.pellets[5 * 20 + 5] = true;
    st.energizer_blink_on = true;
    let fb = compose(&st);
    let mut peach = 0;
    for y in 0..8 {
        for x in 0..8 {
            peach += (fb.get(EX + x, EY + y) == PEACH_RGB) as usize;
        }
    }
    assert!(peach > 40);
    assert_eq!(fb.get(EX, EY), BLACK);
    assert_eq!(fb.get(EX + 4, EY + 4), PEACH_RGB);
}

#[test]
fn life_icon_is_a_pac() {
    // Two lives -> one spare-life icon at (16, lives-row); the stage frame
    // is 20x25 tiles, so the lives row starts at pixel 184.
    let mut st = stage_state();
    st.lives = 2;
    let fb = compose(&st);
    let ly = 184;
    let mut yellow = 0;
    for y in 0..16 {
        for x in 0..160 {
            let px = fb.get(x, ly + y);
            if px == YELLOW_RGB {
                yellow += 1;
                assert!(
                    (16..32).contains(&x),
                    "life icon pixel outside its 16x16 cell at ({x},{y})"
                );
            }
        }
    }
    assert!(yellow > 80);
}
