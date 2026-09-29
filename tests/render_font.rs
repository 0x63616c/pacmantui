//! Bitmap font rendering invariants (CPU-only, no tty).

use pacmantui::render::test_api::*;

const WHITE_RGB: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];

/// A drawn glyph must match its bitmap exactly, pixel for pixel.
#[test]
fn glyph_renders_exact_bitmap() {
    let mut fb = Frame::new(16, 16);
    draw_text(&mut fb, 0, 0, "1", WHITE_RGB);
    let g = glyph('1');
    for (ry, row) in g.iter().enumerate() {
        for bit in 0..8 {
            let expect = if row & (0x80u8 >> bit) != 0 {
                WHITE_RGB
            } else {
                BLACK
            };
            assert_eq!(fb.get(bit, ry as i32), expect, "mismatch at ({bit},{ry})");
        }
    }
    // Row 7 of the 8x8 cell is always blank.
    for x in 0..8 {
        assert_eq!(fb.get(x, 7), BLACK);
    }
}

#[test]
fn known_glyph_pattern_a() {
    // 'A' top row: .XXXX... (0x78)
    assert_eq!(glyph('A')[0], 0x78);
    // and a full crossbar on row 3 (0xFC = XXXXXX..).
    assert_eq!(glyph('A')[3], 0xFC);
}

#[test]
fn lowercase_maps_to_uppercase() {
    assert_eq!(glyph('a'), glyph('A'));
    assert_eq!(glyph('z'), glyph('Z'));
}

#[test]
fn unknown_char_is_blank() {
    assert_eq!(glyph('~'), [0u8; 7]);
    let mut fb = Frame::new(8, 8);
    draw_text(&mut fb, 0, 0, "~", WHITE_RGB);
    assert!(fb.data().iter().all(|&b| b == 0));
}

#[test]
fn text_advances_8px_per_glyph() {
    assert_eq!(text_width("READY!"), 6 * GLYPH_W);
    let mut fb = Frame::new(24, 8);
    draw_text(&mut fb, 0, 0, "II", WHITE_RGB);
    // Second 'I' top row (0x78 -> pixels x1..=4) shifted by 8.
    assert_eq!(fb.get(8 + 1, 0), WHITE_RGB);
    assert_eq!(fb.get(8 + 4, 0), WHITE_RGB);
    assert_eq!(fb.get(8, 0), BLACK);
    assert_eq!(fb.get(8 + 5, 0), BLACK);
}

#[test]
fn scaled_text_expands_each_pixel() {
    let mut fb = Frame::new(32, 32);
    draw_text_scaled(&mut fb, 0, 0, "I", WHITE_RGB, 2);
    // 'I' row0 = 0x78: bits at columns 1..=4 -> scaled pixels x2..=9, y0..=1.
    for y in 0..2 {
        assert_eq!(fb.get(1, y), BLACK);
        for x in 2..10 {
            assert_eq!(fb.get(x, y), WHITE_RGB, "at ({x},{y})");
        }
        assert_eq!(fb.get(10, y), BLACK);
    }
}

#[test]
fn mini_number_centered_and_gapped() {
    // "200" -> width 3*4-1 = 11, centered on (50,50) -> x0 = 45, y0 = 48.
    assert_eq!(mini_number_width(200), 11);
    let mut fb = Frame::new(64, 64);
    let cyan = [0, 255, 255];
    draw_mini_number(&mut fb, 50, 50, 200, cyan);
    // '2' top row = 0b111 -> pixels (45..=47, 48).
    assert_eq!(fb.get(45, 48), cyan);
    assert_eq!(fb.get(46, 48), cyan);
    assert_eq!(fb.get(47, 48), cyan);
    // 1px gap column between digits.
    assert_eq!(fb.get(48, 48), BLACK);
    // '0' of the second digit starts at x=49.
    assert_eq!(fb.get(49, 48), cyan);
    // Nothing above/below the 5-row band.
    assert_eq!(fb.get(45, 47), BLACK);
    assert_eq!(fb.get(45, 53), BLACK);
}
