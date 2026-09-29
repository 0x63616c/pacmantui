//! CPU-only framebuffer compositor invariants (no tty).

use pacmantui::render::test_api::*;

const BLACK: [u8; 3] = [0, 0, 0];
const YELLOW_RGB: [u8; 3] = [255, 255, 0];
const RED_RGB: [u8; 3] = [255, 0, 0];

// 2x2 test sprite: transparent corners, YELLOW on the anti-diagonal.
static TEST_DATA: [u8; 4] = [TRANSPARENT, YELLOW, YELLOW, TRANSPARENT];
static TEST_SPRITE: Sprite = Sprite {
    w: 2,
    h: 2,
    data: &TEST_DATA,
};

#[test]
fn new_frame_is_all_black() {
    let fb = Frame::new(16, 8);
    assert_eq!(fb.width(), 16);
    assert_eq!(fb.height(), 8);
    assert_eq!(fb.data().len(), 16 * 8 * 3);
    assert!(fb.data().iter().all(|&b| b == 0));
}

#[test]
fn put_get_roundtrip_and_oob_ignored() {
    let mut fb = Frame::new(8, 8);
    fb.put(3, 4, [1, 2, 3]);
    assert_eq!(fb.get(3, 4), [1, 2, 3]);
    // Out-of-bounds writes are silent no-ops.
    fb.put(-1, 0, [9, 9, 9]);
    fb.put(0, -1, [9, 9, 9]);
    fb.put(8, 0, [9, 9, 9]);
    fb.put(0, 8, [9, 9, 9]);
    assert_eq!(fb.get(0, 0), BLACK);
    assert_eq!(fb.get(7, 7), BLACK);
}

#[test]
fn fill_rect_clips_and_leaves_rest_black() {
    let mut fb = Frame::new(8, 8);
    fb.fill_rect(6, 6, 4, 4, [5, 5, 5]); // overlaps bottom-right corner
    assert_eq!(fb.get(6, 6), [5, 5, 5]);
    assert_eq!(fb.get(7, 7), [5, 5, 5]);
    assert_eq!(fb.get(5, 5), BLACK);
    assert_eq!(fb.get(5, 7), BLACK);
}

#[test]
fn blit_exact_position_with_transparency() {
    let mut fb = Frame::new(16, 16);
    fb.put(5, 5, [10, 20, 30]); // background under a transparent sprite pixel
    fb.blit(&TEST_SPRITE, 5, 5);
    // Transparent corners leave the background untouched.
    assert_eq!(fb.get(5, 5), [10, 20, 30]);
    assert_eq!(fb.get(6, 6), BLACK);
    // Opaque pixels land at the exact position.
    assert_eq!(fb.get(6, 5), YELLOW_RGB);
    assert_eq!(fb.get(5, 6), YELLOW_RGB);
    // Everything else is untouched black.
    assert_eq!(fb.get(7, 5), BLACK);
    assert_eq!(fb.get(4, 5), BLACK);
}

#[test]
fn blit_remap_changes_palette_index() {
    let mut fb = Frame::new(8, 8);
    fb.blit_remap(&TEST_SPRITE, 0, 0, &body_remap(RED)); // identity except BODY
    assert_eq!(fb.get(1, 0), YELLOW_RGB); // YELLOW unaffected by body remap
    let mut remap = IDENTITY;
    remap[YELLOW as usize] = RED;
    let mut fb2 = Frame::new(8, 8);
    fb2.blit_remap(&TEST_SPRITE, 0, 0, &remap);
    assert_eq!(fb2.get(1, 0), RED_RGB);
    // Remap never makes transparent pixels drawn.
    assert_eq!(fb2.get(0, 0), BLACK);
}

#[test]
fn blit_clips_at_negative_and_far_edges() {
    let mut fb = Frame::new(8, 8);
    fb.blit(&TEST_SPRITE, -1, 0); // sprite pixel (1,0) lands at (0,0)
    assert_eq!(fb.get(0, 0), YELLOW_RGB);
    fb.blit(&TEST_SPRITE, 7, 7); // only sprite pixel (0,1)? -> (7,8) OOB; (1,0)->(8,7) OOB
    // In-bounds part: sprite (0,0) transparent at (7,7); nothing else fits.
    assert_eq!(fb.get(7, 7), BLACK);
}

#[test]
fn scale_up_identity_at_1x() {
    let src = vec![1u8, 2, 3, 4, 5, 6]; // 2x1
    let mut dst = Vec::new();
    scale_up(&src, 2, 1, 1, &mut dst);
    assert_eq!(dst, src);
}

#[test]
fn scale_up_2x_replicates_pixels_and_rows() {
    // 2x1: pixel A=[1,2,3], pixel B=[4,5,6]
    let src = vec![1u8, 2, 3, 4, 5, 6];
    let mut dst = Vec::new();
    scale_up(&src, 2, 1, 2, &mut dst);
    assert_eq!(dst.len(), 2 * 3 * 4);
    let row: Vec<u8> = vec![1, 2, 3, 1, 2, 3, 4, 5, 6, 4, 5, 6];
    assert_eq!(&dst[0..12], &row[..]);
    assert_eq!(&dst[12..24], &row[..]); // duplicated row
}

#[test]
fn scale_up_3x_size() {
    let src = vec![7u8; 4 * 2 * 3];
    let mut dst = Vec::new();
    scale_up(&src, 4, 2, 3, &mut dst);
    assert_eq!(dst.len(), 4 * 2 * 3 * 9);
    assert!(dst.iter().all(|&b| b == 7));
}

#[test]
fn resize_clears_to_black() {
    let mut fb = Frame::new(4, 4);
    fb.put(0, 0, [9, 9, 9]);
    fb.resize(6, 2);
    assert_eq!(fb.width(), 6);
    assert_eq!(fb.height(), 2);
    assert!(fb.data().iter().all(|&b| b == 0));
}
