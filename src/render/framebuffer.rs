//! CPU pixel framebuffer: RGB compose surface + integer nearest upscaling.
//!
//! Everything here is pure (no terminal I/O). [`Frame`]'s public interface is
//! read-only (dimensions, pixel/byte access) — the surface callers and tests
//! consume through the compositor; the draw primitives are crate-internal
//! implementation, pinned by this module's own tests at the internal seam.

use super::sprites::{self, Sprite};

/// Row-major RGB framebuffer at the logical (unscaled) resolution.
#[derive(Debug, Clone)]
pub struct Frame {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Frame {
    pub(crate) fn new(w: usize, h: usize) -> Frame {
        Frame {
            w,
            h,
            px: vec![0u8; w * h * 3],
        }
    }

    pub fn width(&self) -> usize {
        self.w
    }

    pub fn height(&self) -> usize {
        self.h
    }

    pub fn data(&self) -> &[u8] {
        &self.px
    }

    /// Resize (content becomes all-black).
    pub(crate) fn resize(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        self.px.clear();
        self.px.resize(w * h * 3, 0);
    }

    /// Fill with opaque black.
    pub(crate) fn clear(&mut self) {
        self.px.fill(0);
    }

    /// Clear to black, then copy `src` (whole frame-width RGB rows) starting
    /// at pixel row `y0`. `src` must fit below `y0`.
    pub(crate) fn copy_rows_at(&mut self, src: &[u8], y0: usize) {
        let row = self.w * 3;
        assert!(
            row > 0 && src.len().is_multiple_of(row),
            "layer width mismatch"
        );
        let start = y0 * row;
        assert!(
            start + src.len() <= self.px.len(),
            "layer rows out of range"
        );
        self.px.fill(0);
        self.px[start..start + src.len()].copy_from_slice(src);
    }

    /// Set one pixel; out-of-bounds coordinates are ignored.
    pub(crate) fn put(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let o = (y as usize * self.w + x as usize) * 3;
        self.px[o..o + 3].copy_from_slice(&rgb);
    }

    /// Read one pixel (panics out of bounds; test/diagnostic helper).
    pub fn get(&self, x: i32, y: i32) -> [u8; 3] {
        assert!(x >= 0 && y >= 0 && x < self.w as i32 && y < self.h as i32);
        let o = (y as usize * self.w + x as usize) * 3;
        [self.px[o], self.px[o + 1], self.px[o + 2]]
    }

    /// Fill a rectangle, clipped to the frame.
    pub(crate) fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: [u8; 3]) {
        for dy in 0..h {
            for dx in 0..w {
                self.put(x + dx, y + dy, rgb);
            }
        }
    }

    /// Blit a sprite with index 0 transparent, clipped to the frame.
    pub(crate) fn blit(&mut self, sp: &Sprite, x: i32, y: i32) {
        self.blit_remap(sp, x, y, &sprites::IDENTITY);
    }

    /// Blit through a palette remap table (index 0 stays transparent).
    pub(crate) fn blit_remap(&mut self, sp: &Sprite, x: i32, y: i32, remap: &[u8; 16]) {
        for sy in 0..sp.h {
            let dy = y + sy as i32;
            if dy < 0 || dy >= self.h as i32 {
                continue;
            }
            for sx in 0..sp.w {
                let dx = x + sx as i32;
                if dx < 0 || dx >= self.w as i32 {
                    continue;
                }
                let idx = remap[(sp.data[sy * sp.w + sx] & 0x0F) as usize];
                if idx == sprites::TRANSPARENT {
                    continue;
                }
                let o = (dy as usize * self.w + dx as usize) * 3;
                self.px[o..o + 3].copy_from_slice(&sprites::PALETTE[idx as usize]);
            }
        }
    }
}

/// Integer nearest-neighbour upscale of a w*h RGB buffer by factor `k` into
/// `dst` (cleared first). k=1 is a plain copy. This is the tty adapter's
/// letterbox scaling, not part of the compositor's interface.
pub(crate) fn scale_up(src: &[u8], w: usize, h: usize, k: usize, dst: &mut Vec<u8>) {
    assert_eq!(src.len(), w * h * 3, "source size mismatch");
    assert!(k >= 1);
    dst.clear();
    if k == 1 {
        dst.extend_from_slice(src);
        return;
    }
    dst.reserve(src.len() * k * k);
    let row_bytes = w * 3 * k;
    for y in 0..h {
        let row_start = dst.len();
        let src_row = &src[y * w * 3..(y + 1) * w * 3];
        for x in 0..w {
            let px = &src_row[x * 3..x * 3 + 3];
            for _ in 0..k {
                dst.extend_from_slice(px);
            }
        }
        for _ in 1..k {
            dst.extend_from_within(row_start..row_start + row_bytes);
        }
    }
}

// Unit tests at the internal seam: the draw primitives and adapter scaling
// are implementation detail of the compositor/renderer, pinned here rather
// than through the public interface (moved verbatim from
// tests/render_framebuffer.rs when test_api was retired).
#[cfg(test)]
mod tests {
    use super::super::sprites::{IDENTITY, RED, TRANSPARENT, YELLOW, body_remap};
    use super::*;

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
}
