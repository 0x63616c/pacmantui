//! Bitmap font rendering invariants asserted on composed frames through the
//! compositor's public interface (CPU-only, no tty). Menu item labels land
//! at a fixed anchor (x=40, y=128+16*i, scale 1), which makes them exact
//! glyph probes; the popup mini font is probed through a gameplay frame.

use pacmantui::render::compose::{Compositor, Frame, Maze, Overlay, TileKind};
use pacmantui::render::{MenuItem, MenuScreen};
use pacmantui::types::{
    Dir, GhostId, GhostRender, GhostState, PxPos, RenderState, Sequence, TilePos,
};

const WHITE_RGB: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
const CYAN_RGB: [u8; 3] = [0, 255, 255];

/// First menu item label anchor (scale 1).
const LX: i32 = 40;
const LY: i32 = 128;

/// Compose a menu whose only pixels near the item rows are the given labels
/// (no selection marker: `selected` points past the items).
fn labels_frame(title: &str, labels: &[&str]) -> Frame {
    let screen = MenuScreen {
        title: title.into(),
        items: labels
            .iter()
            .map(|l| MenuItem {
                label: (*l).into(),
                value: None,
                enabled: true,
            })
            .collect(),
        selected: usize::MAX,
        footer: String::new(),
        decorated: false,
    };
    let mut c = Compositor::new();
    c.menu(&screen).clone()
}

/// A drawn glyph must match its bitmap exactly, pixel for pixel.
#[test]
fn glyph_renders_exact_bitmap() {
    // '1' in the 8px font (authored bitmap, bit 7 = leftmost column).
    const ONE: [u8; 7] = [0x30, 0x70, 0x30, 0x30, 0x30, 0x30, 0xFC];
    let fb = labels_frame("", &["1"]);
    for (ry, row) in ONE.iter().enumerate() {
        for bit in 0..8 {
            let expect = if row & (0x80u8 >> bit) != 0 {
                WHITE_RGB
            } else {
                BLACK
            };
            assert_eq!(
                fb.get(LX + bit, LY + ry as i32),
                expect,
                "mismatch at ({bit},{ry})"
            );
        }
    }
    // Row 7 of the 8x8 cell is always blank.
    for x in 0..8 {
        assert_eq!(fb.get(LX + x, LY + 7), BLACK);
    }
}

#[test]
fn known_glyph_pattern_a() {
    let fb = labels_frame("", &["A"]);
    // 'A' top row: .XXXX... (0x78)
    for bit in 0..8 {
        let expect = if 0x78u8 & (0x80 >> bit) != 0 {
            WHITE_RGB
        } else {
            BLACK
        };
        assert_eq!(fb.get(LX + bit, LY), expect, "row 0 bit {bit}");
    }
    // and a full crossbar on row 3 (0xFC = XXXXXX..).
    for bit in 0..8 {
        let expect = if 0xFCu8 & (0x80 >> bit) != 0 {
            WHITE_RGB
        } else {
            BLACK
        };
        assert_eq!(fb.get(LX + bit, LY + 3), expect, "row 3 bit {bit}");
    }
}

#[test]
fn lowercase_maps_to_uppercase() {
    assert_eq!(
        labels_frame("", &["a"]).data(),
        labels_frame("", &["A"]).data()
    );
    assert_eq!(
        labels_frame("", &["z"]).data(),
        labels_frame("", &["Z"]).data()
    );
}

#[test]
fn unknown_char_is_blank() {
    let fb = labels_frame("", &["~"]);
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(fb.get(LX + x, LY + y), BLACK, "at ({x},{y})");
        }
    }
}

#[test]
fn text_advances_8px_per_glyph() {
    let fb = labels_frame("", &["II"]);
    // Second 'I' top row (0x78 -> pixels x1..=4) shifted by 8.
    assert_eq!(fb.get(LX + 8 + 1, LY), WHITE_RGB);
    assert_eq!(fb.get(LX + 8 + 4, LY), WHITE_RGB);
    assert_eq!(fb.get(LX + 8, LY), BLACK);
    assert_eq!(fb.get(LX + 8 + 5, LY), BLACK);
}

#[test]
fn scaled_text_expands_each_pixel() {
    // The menu title draws at 2x when it fits: "I" centered on the 224px
    // frame -> 16px wide at x=104, y=32.
    let fb = labels_frame("I", &[]);
    // 'I' row0 = 0x78: bits at columns 1..=4 -> scaled pixels x2..=9, y0..=1.
    for y in 32..34 {
        assert_eq!(fb.get(104 + 1, y), BLACK);
        for x in 2..10 {
            assert_eq!(fb.get(104 + x, y), [255, 255, 0], "at ({x},{y})"); // title yellow
        }
        assert_eq!(fb.get(104 + 10, y), BLACK);
    }
}

// --- mini 3x5 digit font (score popups) --------------------------------------

#[test]
fn mini_number_centered_and_gapped() {
    // A 200-point popup at tile (7,5) of a 10x10 open grid: centered on the
    // tile center, map px (60,44) -> frame px (60,68) (maze origin 24).
    struct Open10;
    impl Maze for Open10 {
        fn id(&self) -> &str {
            "open-10x10"
        }
        fn size(&self) -> (i32, i32) {
            (10, 10)
        }
        fn kind(&self, _x: i32, _y: i32) -> TileKind {
            TileKind::Open
        }
        fn fruit_px(&self) -> (i32, i32) {
            (40, 40)
        }
    }
    let ghost = |id| GhostRender {
        id,
        pos: PxPos::tile_center(TilePos::new(2, 2)),
        dir: Dir::Right,
        state: GhostState::Active,
        frightened: false,
        anim: 0,
        visible: false,
    };
    let st = RenderState {
        pac_pos: PxPos::tile_center(TilePos::new(2, 2)),
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
        popups: vec![(TilePos::new(7, 5), 200, 10)],
        score: 0,
        high_score: 0,
        lives: 1,
        level: 1,
        fruit_history: Vec::new(),
        sequence: Sequence::Playing,
        pellets: vec![false; 100],
        energizer_blink_on: false,
        pac_visible: false,
    };
    let mut c = Compositor::new();
    let fb = c.game(&Open10, &st, Overlay::default()).clone();
    // "200" is 11px wide (3px digits, 1px gaps) centered on (60,68):
    // '2' top row = 0b111 -> pixels (55..=57, 66); nothing left of x=55.
    assert_eq!(fb.get(55, 66), CYAN_RGB);
    assert_eq!(fb.get(56, 66), CYAN_RGB);
    assert_eq!(fb.get(57, 66), CYAN_RGB);
    assert_eq!(fb.get(54, 66), BLACK);
    // 1px gap column between digits.
    assert_eq!(fb.get(58, 66), BLACK);
    // '0' of the second digit starts at x=59; the last digit ends at x=65.
    assert_eq!(fb.get(59, 66), CYAN_RGB);
    assert_eq!(fb.get(65, 66), CYAN_RGB);
    assert_eq!(fb.get(66, 66), BLACK);
    // Nothing above/below the 5-row band (rows 66..=70).
    assert_eq!(fb.get(55, 65), BLACK);
    assert_eq!(fb.get(55, 71), BLACK);
}
