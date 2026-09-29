//! Scene composition: auto-tiled maze layer, gameplay frame, HUD, menu and
//! loading screens. All functions here compose into the CPU framebuffer and
//! are pure (no terminal I/O), so they are unit-testable without a tty.
//!
//! Visual layout facts (28x36 tiles, HUD rows, colors, sprite metrics) per
//! docs/research/dossier-mechanics.md.

use super::font;
use super::framebuffer::Frame;
use super::sprites;
use super::{LoadingScreen, MenuScreen};
use crate::sim::timings;
use crate::types::{Dir, GhostId, GhostState, RenderState, Sequence, TilePos};

/// Fixed logical resolution for menu/loading scenes (classic screen size).
pub const MENU_W: usize = 224;
pub const MENU_H: usize = 288;

const WHITE: [u8; 3] = [255, 255, 255];
const GRAY: [u8; 3] = [120, 120, 120];
const YELLOW: [u8; 3] = [255, 255, 0];
const RED: [u8; 3] = [255, 0, 0];
const POPUP_CYAN: [u8; 3] = [0, 255, 255];
const POPUP_PINK: [u8; 3] = [255, 184, 255];

/// Renderer-side abstraction of map cells (decouples composition from `Map`
/// so tests can drive it with synthetic grids).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Open,
    Wall,
    Door,
    Dot,
    Energizer,
}

impl TileKind {
    fn is_wall(self) -> bool {
        matches!(self, TileKind::Wall)
    }
}

// --- maze static layer -------------------------------------------------------

/// Compose the static maze layer (walls + house door) for a `tw` x `th` tile
/// grid into an RGB buffer of `tw*8 x th*8`. Walls are auto-tiled from cell
/// adjacency into classic-look thin blue contour lines with rounded corners
/// (white variant for the level flash). Dead-space padding (e.g. HUD rows)
/// outside the maze bounding box is left black.
pub fn compose_maze_layer(
    tw: i32,
    th: i32,
    kind_at: &dyn Fn(i32, i32) -> TileKind,
    white: bool,
) -> Vec<u8> {
    let wpx = (tw * 8) as usize;
    let hpx = (th * 8) as usize;
    let mut buf = vec![0u8; wpx * hpx * 3];

    // Bounding box of non-wall tiles; the maze border band is one tile beyond.
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for ty in 0..th {
        for tx in 0..tw {
            if !kind_at(tx, ty).is_wall() {
                x0 = x0.min(tx);
                y0 = y0.min(ty);
                x1 = x1.max(tx);
                y1 = y1.max(ty);
            }
        }
    }
    if x0 > x1 {
        return buf; // no open tiles at all
    }
    let (bx0, by0, bx1, by1) = (x0 - 1, y0 - 1, x1 + 1, y1 + 1);

    // Openness for contour purposes: beyond the expanded box (dead space /
    // off-map) counts as open so the outermost wall band gets its outer
    // contour, giving the classic double-line border.
    let open = |tx: i32, ty: i32| -> bool {
        if tx < bx0 || tx > bx1 || ty < by0 || ty > by1 {
            return true;
        }
        if tx < 0 || tx >= tw || ty < 0 || ty >= th {
            return true;
        }
        !kind_at(tx, ty).is_wall()
    };

    let line = if white {
        WHITE
    } else {
        sprites::rgb(sprites::MAZE_BLUE)
    };
    let door = sprites::rgb(sprites::DOOR_PINK);

    for ty in by0.max(0)..=by1.min(th - 1) {
        for tx in bx0.max(0)..=bx1.min(tw - 1) {
            match kind_at(tx, ty) {
                TileKind::Wall => draw_wall_tile(&mut buf, wpx, tx, ty, &open, line),
                TileKind::Door => {
                    // Pink horizontal bar across the door tile.
                    for ly in 3..5 {
                        for lx in 0..8 {
                            put(&mut buf, wpx, tx * 8 + lx, ty * 8 + ly, door);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    buf
}

fn put(buf: &mut [u8], wpx: usize, x: i32, y: i32, rgb: [u8; 3]) {
    if x < 0 || y < 0 || x as usize >= wpx {
        return;
    }
    let o = (y as usize * wpx + x as usize) * 3;
    if o + 3 <= buf.len() {
        buf[o..o + 3].copy_from_slice(&rgb);
    }
}

/// Contour stencil for one wall tile: 1px lines 2px inside each open-facing
/// edge, quarter-round convex corners, and concave connecting arcs where a
/// diagonal neighbour is open. Local coords 0..8.
fn draw_wall_tile(
    buf: &mut [u8],
    wpx: usize,
    tx: i32,
    ty: i32,
    open: &dyn Fn(i32, i32) -> bool,
    rgb: [u8; 3],
) {
    let n = open(tx, ty - 1);
    let s = open(tx, ty + 1);
    let w = open(tx - 1, ty);
    let e = open(tx + 1, ty);
    let nw = open(tx - 1, ty - 1);
    let ne = open(tx + 1, ty - 1);
    let sw = open(tx - 1, ty + 1);
    let se = open(tx + 1, ty + 1);
    let (px0, py0) = (tx * 8, ty * 8);
    let mut p = |lx: i32, ly: i32| put(buf, wpx, px0 + lx, py0 + ly, rgb);

    if n {
        for x in (if w { 4 } else { 0 })..=(if e { 3 } else { 7 }) {
            p(x, 2);
        }
    }
    if s {
        for x in (if w { 4 } else { 0 })..=(if e { 3 } else { 7 }) {
            p(x, 5);
        }
    }
    if w {
        for y in (if n { 4 } else { 0 })..=(if s { 3 } else { 7 }) {
            p(2, y);
        }
    }
    if e {
        for y in (if n { 4 } else { 0 })..=(if s { 3 } else { 7 }) {
            p(5, y);
        }
    }
    // Convex rounded corners (two adjacent open sides).
    if n && w {
        p(3, 3);
    }
    if n && e {
        p(4, 3);
    }
    if s && w {
        p(3, 4);
    }
    if s && e {
        p(4, 4);
    }
    // Concave connecting arcs (diagonal open, both adjacent sides solid).
    if !n && !w && nw {
        p(2, 0);
        p(1, 1);
        p(0, 2);
    }
    if !n && !e && ne {
        p(5, 0);
        p(6, 1);
        p(7, 2);
    }
    if !s && !w && sw {
        p(2, 7);
        p(1, 6);
        p(0, 5);
    }
    if !s && !e && se {
        p(5, 7);
        p(6, 6);
        p(7, 5);
    }
}

/// Level-flash phase from the LevelFlash tick: after the freeze, alternate
/// white/blue per half-period for the configured number of flashes.
pub fn flash_is_white(tick: u32) -> bool {
    if tick < timings::LEVEL_CLEAR_FREEZE_TICKS {
        return false;
    }
    let half = (tick - timings::LEVEL_CLEAR_FREEZE_TICKS) / timings::LEVEL_CLEAR_FLASH_HALF_TICKS;
    half < timings::LEVEL_CLEAR_FLASHES * 2 && half.is_multiple_of(2)
}

// --- gameplay frame ----------------------------------------------------------

/// Everything `draw_game` needs besides the map-cell closure.
pub struct GameView<'a> {
    /// Cached static maze layer (exactly tw*8 x th*8 RGB).
    pub layer: &'a [u8],
    pub tw: i32,
    pub th: i32,
    pub state: &'a RenderState,
    /// Fruit/overlay anchor: pixel center of the fruit spawn.
    pub fruit_px: (i32, i32),
    pub paused: bool,
    pub game_over: bool,
}

/// Compose one full gameplay frame over the cached maze layer.
pub fn draw_game(fb: &mut Frame, view: &GameView, kind_at: impl Fn(i32, i32) -> TileKind) {
    let st = view.state;
    fb.copy_from(view.layer);

    // Pellets: peach 2x2 dots at tile centers; energizers blink.
    for ty in 0..view.th {
        for tx in 0..view.tw {
            let present = st
                .pellets
                .get((ty * view.tw + tx) as usize)
                .copied()
                .unwrap_or(false);
            if !present {
                continue;
            }
            match kind_at(tx, ty) {
                TileKind::Dot => {
                    fb.fill_rect(
                        tx * 8 + 3,
                        ty * 8 + 3,
                        2,
                        2,
                        sprites::rgb(sprites::DOT_PEACH),
                    );
                }
                TileKind::Energizer => {
                    if st.energizer_blink_on {
                        fb.blit(&sprites::ENERGIZER, tx * 8, ty * 8);
                    }
                }
                _ => {}
            }
        }
    }

    // Fruit on the board.
    if let Some(t) = st.fruit {
        let sp = sprites::fruit_sprite(current_fruit_index(st));
        fb.blit(sp, t.x * 8 + 4 - 8, t.y * 8 + 4 - 8);
    }

    // Pac-Man (16x16 sprite centered on his pixel position).
    let freeze = match st.sequence {
        Sequence::GhostScoreFreeze { ghost, score, .. } => Some((ghost, score)),
        _ => None,
    };
    let (pac_x, pac_y) = (st.pac_pos.x.px() - 8, st.pac_pos.y.px() - 8);
    match st.sequence {
        Sequence::DeathAnim { tick } => {
            let frame = (tick.saturating_mul(8) / timings::DEATH_ANIM_TICKS.max(1)).min(7) as u8;
            fb.blit(sprites::pac_death_sprite(frame), pac_x, pac_y);
        }
        Sequence::GhostScoreFreeze { .. } => {} // pac hidden while score shows
        Sequence::LevelFlash { .. } => {
            fb.blit(&sprites::PAC_CLOSED, pac_x, pac_y);
        }
        _ => {
            fb.blit(sprites::pac_sprite(st.pac_dir, st.pac_anim), pac_x, pac_y);
        }
    }

    // Ghosts (reverse order so Blinky ends up on top).
    for g in st.ghosts.iter().rev() {
        if !g.visible {
            continue;
        }
        if let Some((eaten, _)) = freeze
            && g.id == eaten
        {
            continue; // replaced by the score popup
        }
        let (gx, gy) = (g.pos.x.px() - 8, g.pos.y.px() - 8);
        match g.state {
            GhostState::Eyes | GhostState::Entering => {
                fb.blit(sprites::eyes_sprite(g.dir), gx, gy);
            }
            _ if g.frightened => {
                let sp = sprites::fright_sprite(g.anim);
                if st.fright_flash == Some(true) {
                    fb.blit_remap(sp, gx, gy, &sprites::FRIGHT_FLASH_REMAP);
                } else {
                    fb.blit(sp, gx, gy);
                }
            }
            _ => {
                fb.blit_remap(
                    sprites::ghost_body(g.anim),
                    gx,
                    gy,
                    &sprites::ghost_remap(g.id),
                );
                fb.blit(sprites::eyes_sprite(g.dir), gx, gy);
            }
        }
    }

    // Ghost-eaten score at the eaten ghost's position.
    if let Some((eaten, score)) = freeze
        && let Some(g) = st.ghosts.iter().find(|g| g.id == eaten)
    {
        font::draw_mini_number(fb, g.pos.x.px(), g.pos.y.px(), score, POPUP_CYAN);
    }

    // Score popups (skip one duplicating the frozen ghost's).
    for &(tile, value, _ticks) in &st.popups {
        if let Some((eaten, score)) = freeze
            && value == score
            && ghost_tile(st, eaten) == Some(tile)
        {
            continue;
        }
        let color = if matches!(value, 200 | 400 | 800 | 1600) {
            POPUP_CYAN
        } else {
            POPUP_PINK
        };
        font::draw_mini_number(fb, tile.x * 8 + 4, tile.y * 8 + 4, value, color);
    }

    draw_hud(fb, st, view.tw, view.th);

    // Sequence / app overlays, anchored on the classic fruit row.
    let (ax, ay) = (view.fruit_px.0, view.fruit_px.1 - 4);
    if matches!(st.sequence, Sequence::Ready { .. }) {
        draw_text_centered(fb, ax, ay, "READY!", YELLOW);
    }
    if view.game_over {
        draw_text_centered(fb, ax, ay, "GAME  OVER", RED);
    }
    if view.paused {
        draw_text_centered(fb, ax, ay - 32, "PAUSED", WHITE);
    }
}

fn ghost_tile(st: &RenderState, id: GhostId) -> Option<TilePos> {
    st.ghosts.iter().find(|g| g.id == id).map(|g| g.pos.tile())
}

/// Current level's fruit symbol: most recent HUD-history entry, else the
/// classic per-level sequence.
fn current_fruit_index(st: &RenderState) -> u8 {
    if let Some(&f) = st.fruit_history.last() {
        return f % 8;
    }
    classic_fruit_for_level(st.level)
}

fn classic_fruit_for_level(level: u32) -> u8 {
    match level {
        0 | 1 => 0,
        2 => 1,
        3 | 4 => 2,
        5 | 6 => 3,
        7 | 8 => 4,
        9 | 10 => 5,
        11 | 12 => 6,
        _ => 7,
    }
}

fn draw_text_centered(fb: &mut Frame, cx: i32, y: i32, text: &str, rgb: [u8; 3]) {
    font::draw_text(fb, cx - font::text_width(text) / 2, y, text, rgb);
}

// --- HUD ---------------------------------------------------------------------

/// HUD: top rows 0-2 (1UP + score, HIGH SCORE + value), bottom two tile rows
/// (spare lives left, fruit history right). Arcade conventions: score
/// right-aligned ending at col 6, display rolls at 999,999.
pub fn draw_hud(fb: &mut Frame, st: &RenderState, tw: i32, th: i32) {
    font::draw_text(fb, 3 * 8, 0, "1UP", WHITE);
    let hs_label = "HIGH SCORE";
    font::draw_text(
        fb,
        (tw * 8 - font::text_width(hs_label)) / 2,
        0,
        hs_label,
        WHITE,
    );

    let score = format!("{:02}", st.score % 1_000_000);
    font::draw_text(fb, 7 * 8 - font::text_width(&score), 8, &score, WHITE);

    let high = format!("{:02}", st.high_score % 1_000_000);
    font::draw_text(
        fb,
        (tw / 2 + 3) * 8 - font::text_width(&high),
        8,
        &high,
        WHITE,
    );

    // Spare lives as mini pac icons, bottom-left.
    let ly = (th - 2) * 8;
    let spare = st.lives.saturating_sub(1).min(5) as i32;
    for k in 0..spare {
        fb.blit(sprites::life_sprite(), (2 + 2 * k) * 8, ly);
    }

    // Fruit history, bottom-right, most recent at the right edge.
    for (k, &f) in st.fruit_history.iter().rev().take(7).enumerate() {
        let x = (tw - 4) * 8 - (k as i32) * 16;
        if x < 0 {
            break;
        }
        fb.blit(sprites::fruit_sprite(f % 8), x, ly);
    }
}

// --- menu / loading ----------------------------------------------------------

/// Pixel-rendered menu: bitmap-font title (2x), optional decorative sprite
/// parade, item list with selected-item highlight, right-aligned value
/// column, footer. Letterboxed on black by the shared framebuffer path.
pub fn compose_menu(fb: &mut Frame, screen: &MenuScreen) {
    fb.clear();
    let w = fb.width() as i32;

    let title = screen.title.as_str();
    let k = if font::text_width(title) * 2 <= w {
        2
    } else {
        1
    };
    font::draw_text_scaled(
        fb,
        (w - font::text_width(title) * k) / 2,
        32,
        title,
        YELLOW,
        k,
    );

    if screen.decorated {
        // Ghost parade chasing a pac across the marquee row.
        let y = 72;
        for (i, id) in GhostId::ALL.iter().enumerate() {
            let x = 40 + i as i32 * 24;
            fb.blit_remap(&sprites::GHOST_A, x, y, &sprites::ghost_remap(*id));
            fb.blit(sprites::eyes_sprite(Dir::Right), x, y);
        }
        fb.blit(&sprites::PAC_OPEN_R, 40 + 4 * 24 + 8, y);
    }

    let y0 = 128;
    for (i, item) in screen.items.iter().enumerate() {
        let y = y0 + i as i32 * 16;
        let selected = i == screen.selected;
        let color = if !item.enabled {
            GRAY
        } else if selected {
            YELLOW
        } else {
            WHITE
        };
        if selected {
            font::draw_text(fb, 24, y, ">", YELLOW);
        }
        font::draw_text(fb, 40, y, &item.label, color);
        if let Some(v) = &item.value {
            font::draw_text(fb, w - 24 - font::text_width(v), y, v, color);
        }
    }

    draw_text_centered(fb, w / 2, fb.height() as i32 - 16, &screen.footer, GRAY);
}

/// Pixel-rendered loading screen: message + bordered progress bar + percent.
pub fn compose_loading(fb: &mut Frame, screen: &LoadingScreen) {
    fb.clear();
    let w = fb.width() as i32;
    draw_text_centered(fb, w / 2, 120, &screen.message, WHITE);

    // Bar frame: 160x12 centered, 1px white border.
    let (bx, by, bw, bh) = (w / 2 - 80, 148, 160, 12);
    fb.fill_rect(bx, by, bw, 1, WHITE);
    fb.fill_rect(bx, by + bh - 1, bw, 1, WHITE);
    fb.fill_rect(bx, by, 1, bh, WHITE);
    fb.fill_rect(bx + bw - 1, by, 1, bh, WHITE);

    let progress = screen.progress.clamp(0.0, 1.0);
    let fill = ((bw - 4) as f32 * progress) as i32;
    fb.fill_rect(bx + 2, by + 2, fill, bh - 4, YELLOW);

    let pct = format!("{}%", (progress * 100.0).round() as u32);
    draw_text_centered(fb, w / 2, by + bh + 8, &pct, GRAY);
}
