//! Scaling/anchor/letterbox math (pure, no tty).

use pacmantui::render::test_api::*;

const LW: usize = 224;
const LH: usize = 288;

#[test]
fn scale_caps_at_4x() {
    // 2000x1500 px, 100x50 cells: raw fit would be 5x; capped to 4.
    let l = layout_for(2000, 1500, 100, 50, LW, LH);
    assert_eq!(l.scale, 4);
    assert_eq!(l.cell_w, 20);
    assert_eq!(l.cell_h, 30);
    // 224*4=896 wide -> off_x=552 -> col 552/20+1 = 28.
    assert_eq!(l.anchor_col, 28);
    // 288*4=1152 tall -> off_y=174 -> row 174/30+1 = 6.
    assert_eq!(l.anchor_row, 6);
}

#[test]
fn scale_never_below_1_even_when_too_small() {
    let l = layout_for(200, 200, 80, 24, LW, LH);
    assert_eq!(l.scale, 1);
    assert_eq!(l.anchor_col, 1);
    assert_eq!(l.anchor_row, 1);
}

#[test]
fn exact_fit_2x_centers_at_origin() {
    let l = layout_for(448, 576, 100, 40, LW, LH);
    assert_eq!(l.scale, 2);
    assert_eq!(l.anchor_col, 1);
    assert_eq!(l.anchor_row, 1);
}

#[test]
fn horizontal_letterbox_only() {
    // Wide short window: height limits scale to 1, width letterboxes.
    let l = layout_for(1000, 288, 100, 36, LW, LH);
    assert_eq!(l.scale, 1);
    assert_eq!(l.cell_w, 10);
    assert_eq!(l.cell_h, 8);
    // off_x = (1000-224)/2 = 388 -> col 388/10+1 = 39.
    assert_eq!(l.anchor_col, 39);
    assert_eq!(l.anchor_row, 1);
}

#[test]
fn intermediate_scale_picks_floor() {
    // Fits 3x but not 4x.
    let l = layout_for(224 * 3 + 100, 288 * 3 + 50, 90, 45, LW, LH);
    assert_eq!(l.scale, 3);
}

#[test]
fn degenerate_cell_counts_do_not_panic() {
    let l = layout_for(100, 100, 0, 0, LW, LH);
    assert_eq!(l.scale, 1);
    assert!(l.cell_w >= 1 && l.cell_h >= 1);
}

#[test]
fn custom_logical_resolution() {
    // A smaller custom map: 160x160 logical in an 800x800 window -> 4x fits
    // exactly with 5x raw capped? 800/160 = 5 -> capped to 4; offsets center.
    let l = layout_for(800, 800, 80, 40, 160, 160);
    assert_eq!(l.scale, 4);
    // 160*4=640, off=(800-640)/2=80; cell_w=10 -> col 9; cell_h=20 -> row 5.
    assert_eq!(l.anchor_col, 9);
    assert_eq!(l.anchor_row, 5);
}
