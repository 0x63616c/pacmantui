//! Kitty graphics protocol frame assembly (docs/research/rendering.md §4.3).
//!
//! Pure byte-string assembly into an injectable sink so the exact wire format
//! is unit-testable without a terminal: synchronized-output brackets, cursor
//! anchor, chunked `a=t` transmit (f=24, optional o=z, 4096-byte base64
//! chunks), `a=p` placement under text, delete of the previous frame's id.

use std::borrow::Cow;
use std::io::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

/// Spec maximum for one chunk's base64 payload (multiple of 4).
pub const CHUNK: usize = 4096;

/// Parameters for one full-frame emit.
#[derive(Debug, Clone, Copy)]
pub struct FrameParams {
    /// Transmitted image size in pixels (already integer-scaled).
    pub width: usize,
    pub height: usize,
    /// 1-based cell anchor for the placement.
    pub row: u16,
    pub col: u16,
    /// Image id transmitted+placed this frame.
    pub new_id: u32,
    /// Image id whose placement is deleted this frame.
    pub old_id: u32,
    /// Apply zlib (`o=z`) before base64.
    pub compress: bool,
}

/// Assemble one complete frame into `out` (cleared first): sync-begin, cursor
/// move, chunked transmit of `rgb` (f=24), place new id (z=-1, C=1), delete
/// old id, sync-end. All graphics commands carry q=2.
pub fn write_frame(out: &mut Vec<u8>, rgb: &[u8], p: &FrameParams) {
    assert_eq!(
        rgb.len(),
        p.width * p.height * 3,
        "rgb buffer size mismatch"
    );
    out.clear();
    out.extend_from_slice(b"\x1b[?2026h");
    let _ = write!(out, "\x1b[{};{}H", p.row, p.col);

    let data: Cow<[u8]> = if p.compress {
        let mut enc = flate2::write::ZlibEncoder::new(
            Vec::with_capacity(rgb.len() / 8 + 64),
            flate2::Compression::fast(),
        );
        enc.write_all(rgb).expect("zlib write cannot fail on Vec");
        Cow::Owned(enc.finish().expect("zlib finish cannot fail on Vec"))
    } else {
        Cow::Borrowed(rgb)
    };
    let b64 = B64.encode(data.as_ref());
    let bytes = b64.as_bytes();

    let mut first = true;
    let mut idx = 0;
    loop {
        let end = (idx + CHUNK).min(bytes.len());
        let more = if end < bytes.len() { 1 } else { 0 };
        if first {
            let o = if p.compress { ",o=z" } else { "" };
            let _ = write!(
                out,
                "\x1b_Ga=t,f=24{o},s={},v={},i={},q=2,m={more};",
                p.width, p.height, p.new_id
            );
            first = false;
        } else {
            let _ = write!(out, "\x1b_Gm={more};");
        }
        out.extend_from_slice(&bytes[idx..end]);
        out.extend_from_slice(b"\x1b\\");
        idx = end;
        if more == 0 {
            break;
        }
    }

    let _ = write!(out, "\x1b_Ga=p,i={},p=1,z=-1,C=1,q=2\x1b\\", p.new_id);
    let _ = write!(out, "\x1b_Ga=d,d=i,i={},q=2\x1b\\", p.old_id);
    out.extend_from_slice(b"\x1b[?2026l");
}
