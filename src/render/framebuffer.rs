//! CPU pixel framebuffer: RGB compose surface + integer nearest upscaling.
//!
//! Everything here is pure (no terminal I/O) so it is directly unit-testable.

use super::sprites::{self, Sprite};

/// Row-major RGB framebuffer at the logical (unscaled) resolution.
#[derive(Debug, Clone)]
pub struct Frame {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Frame {
    pub fn new(w: usize, h: usize) -> Frame {
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
    pub fn resize(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        self.px.clear();
        self.px.resize(w * h * 3, 0);
    }

    /// Fill with opaque black.
    pub fn clear(&mut self) {
        self.px.fill(0);
    }

    /// Replace the whole content with `src` (must be exactly w*h*3 bytes).
    pub fn copy_from(&mut self, src: &[u8]) {
        assert_eq!(src.len(), self.px.len(), "layer size mismatch");
        self.px.copy_from_slice(src);
    }

    /// Clear to black, then copy `src` (whole frame-width RGB rows) starting
    /// at pixel row `y0`. `src` must fit below `y0`; with `y0 == 0` and a
    /// full-size `src` this is equivalent to [`Frame::copy_from`].
    pub fn copy_rows_at(&mut self, src: &[u8], y0: usize) {
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
    pub fn put(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
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
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: [u8; 3]) {
        for dy in 0..h {
            for dx in 0..w {
                self.put(x + dx, y + dy, rgb);
            }
        }
    }

    /// Blit a sprite with index 0 transparent, clipped to the frame.
    pub fn blit(&mut self, sp: &Sprite, x: i32, y: i32) {
        self.blit_remap(sp, x, y, &sprites::IDENTITY);
    }

    /// Blit through a palette remap table (index 0 stays transparent).
    pub fn blit_remap(&mut self, sp: &Sprite, x: i32, y: i32, remap: &[u8; 16]) {
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
/// `dst` (cleared first). k=1 is a plain copy.
pub fn scale_up(src: &[u8], w: usize, h: usize, k: usize, dst: &mut Vec<u8>) {
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
