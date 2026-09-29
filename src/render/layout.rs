//! FrameLayout: the one owner of the map-grid→frame mapping.
//!
//! The gameplay frame is the map grid plus any HUD padding rows (black tile
//! rows above/below the maze so the score/lives HUD never overprints an
//! all-maze grid). Before this module the mapping had no owner: the pad rows
//! were computed at every consumer and the pad offset hand-added at each draw
//! site, an implicit contract that broke once (custom-map HUD overprinting
//! the maze). `FrameLayout` is the seam that ends that: computed once per
//! map, it owns the pad rows, the frame dimensions, the maze band origin and
//! the HUD anchors, and every mapping query the draw code needs, so no call
//! site does pad arithmetic itself. Depth over spread: one implementation of
//! the mapping, locality for every future change to it.

use crate::map::Map;
use crate::types::TILE_PX;

use super::scenes::{TileKind, cell_kind};

/// Geometry of one composed gameplay frame for one map. The interface is the
/// set of mapping queries below; the pad rows are an implementation detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLayout {
    /// Map grid size in tiles.
    tw: i32,
    th: i32,
    /// Extra all-black tile rows the frame adds above/below the map grid.
    pad_top: i32,
    pad_bottom: i32,
}

impl FrameLayout {
    /// Compute the layout for a `tw` x `th` tile grid. The classic screen
    /// reserves 3 dead rows on top (1UP/score text rows 0-1 plus a spacer)
    /// and 2 on the bottom (lives and fruit strip); a map whose grid already
    /// embeds that dead space around the maze's open-tile bounding box
    /// (classic) gets zero padding, so its frame equals the map grid; an
    /// all-maze grid (Vertigo) gets the difference as padding rows.
    pub fn new(tw: i32, th: i32, kind_at: &dyn Fn(i32, i32) -> TileKind) -> FrameLayout {
        let (mut y0, mut y1) = (i32::MAX, i32::MIN);
        for ty in 0..th {
            for tx in 0..tw {
                if !matches!(kind_at(tx, ty), TileKind::Wall) {
                    y0 = y0.min(ty);
                    y1 = y1.max(ty);
                }
            }
        }
        let (pad_top, pad_bottom) = if y0 > y1 {
            (0, 0) // no open tiles at all
        } else {
            // Rows above/below the maze border band (one tile beyond the
            // open box).
            let dead_top = (y0 - 1).max(0);
            let dead_bottom = ((th - 1) - (y1 + 1)).max(0);
            ((3 - dead_top).max(0), (2 - dead_bottom).max(0))
        };
        FrameLayout {
            tw,
            th,
            pad_top,
            pad_bottom,
        }
    }

    /// Layout for a real map (the production adapter over [`FrameLayout::new`]).
    pub fn of_map(map: &Map) -> FrameLayout {
        FrameLayout::new(map.width(), map.height(), &|x, y| cell_kind(map, x, y))
    }

    /// Map grid size in tiles.
    pub fn map_tiles(&self) -> (i32, i32) {
        (self.tw, self.th)
    }

    /// Frame size in tiles (map grid plus padding rows).
    pub fn frame_tiles(&self) -> (i32, i32) {
        (self.tw, self.th + self.pad_top + self.pad_bottom)
    }

    /// Frame size in pixels: the composed framebuffer's dimensions.
    pub fn frame_px(&self) -> (usize, usize) {
        let (ftw, fth) = self.frame_tiles();
        ((ftw * TILE_PX) as usize, (fth * TILE_PX) as usize)
    }

    /// Minimum terminal size (pixels) this map needs at 1x scale: the frame
    /// must fit unscaled.
    pub fn min_terminal_px(&self) -> (usize, usize) {
        self.frame_px()
    }

    /// Frame-pixel origin of the maze band (the map grid's rows within the
    /// frame): everything map-anchored draws shifted by this.
    pub fn maze_origin_px(&self) -> (i32, i32) {
        (0, self.pad_top * TILE_PX)
    }

    /// Map pixel coordinates -> frame pixel coordinates.
    pub fn map_px(&self, x: i32, y: i32) -> (i32, i32) {
        let (ox, oy) = self.maze_origin_px();
        (x + ox, y + oy)
    }

    /// Map tile -> frame pixel coordinates of the tile's top-left corner.
    pub fn map_tile_px(&self, tx: i32, ty: i32) -> (i32, i32) {
        self.map_px(tx * TILE_PX, ty * TILE_PX)
    }

    /// HUD text anchors: frame-pixel y of the 1UP/HIGH SCORE row and of the
    /// score-values row (frame rows 0 and 1).
    pub fn hud_text_rows_px(&self) -> (i32, i32) {
        (0, TILE_PX)
    }

    /// HUD lives/fruit strip anchor: frame-pixel y of the bottom two-tile
    /// band's top (the strip's sprites are two tiles tall).
    pub fn hud_lives_row_px(&self) -> i32 {
        let (_, fth) = self.frame_tiles();
        (fth - 2) * TILE_PX
    }
}
