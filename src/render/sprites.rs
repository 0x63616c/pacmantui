//! Original pixel-art sprites as const arrays of palette indices.
//!
//! All art here is original, authored for pacmantui in the arcade *style*
//! (shapes/colors evoke the classic look) — none of it is copied ROM bitmap
//! data. Sprites are ASCII-authored byte strings decoded at compile time into
//! palette-index arrays; direction variants are derived by const mirror /
//! transpose transforms. Index 0 is transparent and composites over black.

use crate::types::{Dir, GhostId};

/// One drawable sprite: `w*h` palette indices, row-major, index 0 transparent.
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub data: &'static [u8],
}

// --- palette ------------------------------------------------------------

pub const TRANSPARENT: u8 = 0;
pub const YELLOW: u8 = 1;
pub const RED: u8 = 2;
pub const PINK: u8 = 3;
pub const CYAN: u8 = 4;
pub const ORANGE: u8 = 5;
pub const FRIGHT_BLUE: u8 = 6;
pub const WHITE: u8 = 7;
pub const MAZE_BLUE: u8 = 8;
pub const DOT_PEACH: u8 = 9;
pub const DOOR_PINK: u8 = 10;
pub const GREEN: u8 = 11;
pub const TAN: u8 = 12;
/// Ghost-body placeholder index; always remapped to a real color before draw.
pub const BODY: u8 = 14;

pub const PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],       // 0 transparent (never drawn)
    [255, 255, 0],   // 1 pac yellow
    [255, 0, 0],     // 2 blinky red / cherry
    [255, 184, 255], // 3 pinky pink
    [0, 255, 255],   // 4 inky cyan
    [255, 184, 82],  // 5 clyde orange
    [33, 33, 255],   // 6 frightened blue
    [255, 255, 255], // 7 white
    [33, 33, 222],   // 8 maze blue / eye pupil
    [255, 184, 174], // 9 dot/energizer peach
    [255, 184, 222], // 10 house-door pink
    [0, 255, 0],     // 11 leaf/grape green
    [222, 151, 73],  // 12 stem tan
    [0, 0, 0],       // 13 unused
    [255, 0, 0],     // 14 BODY placeholder (always remapped)
    [0, 0, 0],       // 15 unused
];

pub const fn rgb(idx: u8) -> [u8; 3] {
    PALETTE[idx as usize]
}

/// Identity palette remap (used by plain blits).
pub const IDENTITY: [u8; 16] = identity_remap();

const fn identity_remap() -> [u8; 16] {
    let mut m = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        m[i] = i as u8;
        i += 1;
    }
    m
}

/// Identity remap with the BODY placeholder sent to `color`.
pub const fn body_remap(color: u8) -> [u8; 16] {
    let mut m = identity_remap();
    m[BODY as usize] = color;
    m
}

pub fn ghost_remap(id: GhostId) -> [u8; 16] {
    body_remap(match id {
        GhostId::Blinky => RED,
        GhostId::Pinky => PINK,
        GhostId::Inky => CYAN,
        GhostId::Clyde => ORANGE,
    })
}

/// Frightened white-flash phase: blue body -> white, white face -> red.
pub const FRIGHT_FLASH_REMAP: [u8; 16] = fright_flash_remap();

const fn fright_flash_remap() -> [u8; 16] {
    let mut m = identity_remap();
    m[FRIGHT_BLUE as usize] = WHITE;
    m[WHITE as usize] = RED;
    m
}

// --- compile-time art decoding & transforms ------------------------------

/// Decode ASCII sprite art into palette indices. Compile-time only.
const fn decode<const N: usize>(src: &[u8; N]) -> [u8; N] {
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = match src[i] {
            b'.' => TRANSPARENT,
            b'Y' => YELLOW,
            b'R' => RED,
            b'P' => PINK,
            b'C' => CYAN,
            b'O' => ORANGE,
            b'B' => FRIGHT_BLUE,
            b'W' => WHITE,
            b'M' => MAZE_BLUE,
            b'D' => DOT_PEACH,
            b'K' => DOOR_PINK,
            b'G' => GREEN,
            b'T' => TAN,
            b'F' => BODY,
            _ => panic!("invalid sprite art character"),
        };
        i += 1;
    }
    out
}

const fn mirror_x16(src: [u8; 256]) -> [u8; 256] {
    let mut out = [0u8; 256];
    let mut y = 0;
    while y < 16 {
        let mut x = 0;
        while x < 16 {
            out[y * 16 + x] = src[y * 16 + (15 - x)];
            x += 1;
        }
        y += 1;
    }
    out
}

const fn mirror_y16(src: [u8; 256]) -> [u8; 256] {
    let mut out = [0u8; 256];
    let mut y = 0;
    while y < 16 {
        let mut x = 0;
        while x < 16 {
            out[y * 16 + x] = src[(15 - y) * 16 + x];
            x += 1;
        }
        y += 1;
    }
    out
}

/// Swap axes: a right-facing mouth becomes a down-facing one.
const fn transpose16(src: [u8; 256]) -> [u8; 256] {
    let mut out = [0u8; 256];
    let mut y = 0;
    while y < 16 {
        let mut x = 0;
        while x < 16 {
            out[y * 16 + x] = src[x * 16 + y];
            x += 1;
        }
        y += 1;
    }
    out
}

const fn sprite16(data: &'static [u8; 256]) -> Sprite {
    Sprite { w: 16, h: 16, data }
}

// --- Pac-Man ---------------------------------------------------------------
// ~13px disc centered in the 16x16 cell; chomp = closed / half / full-open
// wedge toward the travel direction. The closed frame is direction-neutral.

const PAC_CLOSED_D: [u8; 256] = decode(
    b"................\
      ......YYYY......\
      ....YYYYYYYY....\
      ...YYYYYYYYYY...\
      ..YYYYYYYYYYYY..\
      ..YYYYYYYYYYYY..\
      .YYYYYYYYYYYYYY.\
      .YYYYYYYYYYYYYY.\
      .YYYYYYYYYYYYYY.\
      ..YYYYYYYYYYYY..\
      ..YYYYYYYYYYYY..\
      ...YYYYYYYYYY...\
      ....YYYYYYYY....\
      ......YYYY......\
      ................\
      ................",
);

const PAC_HALF_R_D: [u8; 256] = decode(
    b"................\
      ......YYYY......\
      ....YYYYYYYY....\
      ...YYYYYYYYYY...\
      ..YYYYYYYYYYYY..\
      ..YYYYYYYYYYY...\
      .YYYYYYYYYY.....\
      .YYYYYYY........\
      .YYYYYYYYYY.....\
      ..YYYYYYYYYYY...\
      ..YYYYYYYYYYYY..\
      ...YYYYYYYYYY...\
      ....YYYYYYYY....\
      ......YYYY......\
      ................\
      ................",
);

const PAC_OPEN_R_D: [u8; 256] = decode(
    b"................\
      ......YYYY......\
      ....YYYYYYYY....\
      ...YYYYYYYYYY...\
      ..YYYYYYYYYY....\
      ..YYYYYYYY......\
      .YYYYYYY........\
      .YYYYY..........\
      .YYYYYYY........\
      ..YYYYYYYY......\
      ..YYYYYYYYYY....\
      ...YYYYYYYYYY...\
      ....YYYYYYYY....\
      ......YYYY......\
      ................\
      ................",
);

const PAC_HALF_L_D: [u8; 256] = mirror_x16(PAC_HALF_R_D);
const PAC_OPEN_L_D: [u8; 256] = mirror_x16(PAC_OPEN_R_D);
const PAC_HALF_DN_D: [u8; 256] = transpose16(PAC_HALF_R_D);
const PAC_OPEN_DN_D: [u8; 256] = transpose16(PAC_OPEN_R_D);
const PAC_HALF_UP_D: [u8; 256] = mirror_y16(PAC_HALF_DN_D);
const PAC_OPEN_UP_D: [u8; 256] = mirror_y16(PAC_OPEN_DN_D);

pub static PAC_CLOSED: Sprite = sprite16(&PAC_CLOSED_D);
pub static PAC_HALF_R: Sprite = sprite16(&PAC_HALF_R_D);
pub static PAC_HALF_L: Sprite = sprite16(&PAC_HALF_L_D);
pub static PAC_HALF_U: Sprite = sprite16(&PAC_HALF_UP_D);
pub static PAC_HALF_D: Sprite = sprite16(&PAC_HALF_DN_D);
pub static PAC_OPEN_R: Sprite = sprite16(&PAC_OPEN_R_D);
pub static PAC_OPEN_L: Sprite = sprite16(&PAC_OPEN_L_D);
pub static PAC_OPEN_U: Sprite = sprite16(&PAC_OPEN_UP_D);
pub static PAC_OPEN_D: Sprite = sprite16(&PAC_OPEN_DN_D);

/// Chomp cycle: 0 closed, 1 half, 2 open, 3 half (then repeat).
pub fn pac_sprite(dir: Dir, anim: u8) -> &'static Sprite {
    match anim % 4 {
        1 | 3 => match dir {
            Dir::Up => &PAC_HALF_U,
            Dir::Left => &PAC_HALF_L,
            Dir::Down => &PAC_HALF_D,
            Dir::Right => &PAC_HALF_R,
        },
        2 => match dir {
            Dir::Up => &PAC_OPEN_U,
            Dir::Left => &PAC_OPEN_L,
            Dir::Down => &PAC_OPEN_D,
            Dir::Right => &PAC_OPEN_R,
        },
        _ => &PAC_CLOSED,
    }
}

/// Sprite for a spare-life icon in the HUD.
pub fn life_sprite() -> &'static Sprite {
    &PAC_OPEN_L
}

// --- Pac-Man death (8-frame shrink into a starburst) -----------------------

const DEATH_1_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      ................\
      ................\
      .YYY........YYY.\
      .YYYY......YYYY.\
      .YYYYYY..YYYYYY.\
      ..YYYYYYYYYYYY..\
      ..YYYYYYYYYYYY..\
      ...YYYYYYYYYY...\
      ....YYYYYYYY....\
      ......YYYY......\
      ................\
      ................",
);

const DEATH_2_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ..YY........YY..\
      ..YYYY....YYYY..\
      ..YYYYY..YYYYY..\
      ...YYYYYYYYYY...\
      ....YYYYYYYY....\
      ......YYYY......\
      ................\
      ................",
);

const DEATH_3_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ...YY......YY...\
      ...YYY....YYY...\
      ....YYYYYYYY....\
      ......YYYY......\
      ................\
      ................",
);

const DEATH_4_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      .....Y....Y.....\
      .....YYYYYY.....\
      ......YYYY......\
      ................\
      ................",
);

const DEATH_5_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      .......YY.......\
      .......YY.......\
      ................\
      ................",
);

const DEATH_6_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      ................\
      .......WW.......\
      .......WW.......\
      ..W....WW....W..\
      ...W........W...\
      .WW..........WW.\
      ................\
      ...W........W...\
      ..W..........W..\
      .......WW.......\
      .......WW.......\
      ................",
);

const DEATH_7_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ................\
      .......WW.......\
      ................\
      ................\
      ..W..........W..\
      ................\
      .W............W.\
      ................\
      ..W..........W..\
      ................\
      ................\
      .......WW.......\
      ................",
);

static PAC_DEATH: [Sprite; 8] = [
    sprite16(&PAC_OPEN_UP_D),
    sprite16(&DEATH_1_D),
    sprite16(&DEATH_2_D),
    sprite16(&DEATH_3_D),
    sprite16(&DEATH_4_D),
    sprite16(&DEATH_5_D),
    sprite16(&DEATH_6_D),
    sprite16(&DEATH_7_D),
];

/// Death animation frame 0..=7 (clamped).
pub fn pac_death_sprite(frame: u8) -> &'static Sprite {
    &PAC_DEATH[frame.min(7) as usize]
}

// --- Ghosts -----------------------------------------------------------------
// Body uses the BODY placeholder index; remap it to the ghost's color at blit
// time. Two skirt "wave" frames. Eyes are a separate per-direction overlay.

const GHOST_A_D: [u8; 256] = decode(
    b"................\
      .....FFFFFF.....\
      ...FFFFFFFFFF...\
      ..FFFFFFFFFFFF..\
      ..FFFFFFFFFFFF..\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFF..FFFF..FFF.\
      ................",
);

const GHOST_B_D: [u8; 256] = decode(
    b"................\
      .....FFFFFF.....\
      ...FFFFFFFFFF...\
      ..FFFFFFFFFFFF..\
      ..FFFFFFFFFFFF..\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FFFFFFFFFFFFFF.\
      .FF...FFFF...FF.\
      ................",
);

pub static GHOST_A: Sprite = sprite16(&GHOST_A_D);
pub static GHOST_B: Sprite = sprite16(&GHOST_B_D);

pub fn ghost_body(anim: u8) -> &'static Sprite {
    if anim.is_multiple_of(2) {
        &GHOST_A
    } else {
        &GHOST_B
    }
}

// Eyes overlay: white sclera pair; 2x2 blue pupils stamped toward the
// looking direction at compile time. Also used alone for "eyes" state.

const EYES_BASE_D: [u8; 256] = decode(
    b"................\
      ................\
      ................\
      ...WW.....WW....\
      ..WWWW...WWWW...\
      ..WWWW...WWWW...\
      ..WWWW...WWWW...\
      ...WW.....WW....\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................\
      ................",
);

/// Stamp 2x2 pupils at (px,py) in the left eye and (px+7,py) in the right.
const fn stamp_pupils(mut s: [u8; 256], px: usize, py: usize) -> [u8; 256] {
    let mut dy = 0;
    while dy < 2 {
        let mut dx = 0;
        while dx < 2 {
            s[(py + dy) * 16 + px + dx] = MAZE_BLUE;
            s[(py + dy) * 16 + px + 7 + dx] = MAZE_BLUE;
            dx += 1;
        }
        dy += 1;
    }
    s
}

const EYES_R_D: [u8; 256] = stamp_pupils(EYES_BASE_D, 4, 4);
const EYES_L_D: [u8; 256] = stamp_pupils(EYES_BASE_D, 2, 4);
const EYES_U_D: [u8; 256] = stamp_pupils(EYES_BASE_D, 3, 3);
const EYES_DN_D: [u8; 256] = stamp_pupils(EYES_BASE_D, 3, 6);

pub static EYES_R: Sprite = sprite16(&EYES_R_D);
pub static EYES_L: Sprite = sprite16(&EYES_L_D);
pub static EYES_U: Sprite = sprite16(&EYES_U_D);
pub static EYES_D: Sprite = sprite16(&EYES_DN_D);

pub fn eyes_sprite(dir: Dir) -> &'static Sprite {
    match dir {
        Dir::Up => &EYES_U,
        Dir::Left => &EYES_L,
        Dir::Down => &EYES_D,
        Dir::Right => &EYES_R,
    }
}

// Frightened: blue body with white face; flash phase via FRIGHT_FLASH_REMAP.

const FRIGHT_A_D: [u8; 256] = decode(
    b"................\
      .....BBBBBB.....\
      ...BBBBBBBBBB...\
      ..BBBBBBBBBBBB..\
      ..BBBBBBBBBBBB..\
      .BBBBBBBBBBBBBB.\
      .BBBWWBBBBWWBBB.\
      .BBBWWBBBBWWBBB.\
      .BBBBBBBBBBBBBB.\
      .BBBBBBBBBBBBBB.\
      .BWWBBWWBBWWBBB.\
      .BBBWWBBWWBBWWB.\
      .BBBBBBBBBBBBBB.\
      .BBBBBBBBBBBBBB.\
      .BBB..BBBB..BBB.\
      ................",
);

const FRIGHT_B_D: [u8; 256] = decode(
    b"................\
      .....BBBBBB.....\
      ...BBBBBBBBBB...\
      ..BBBBBBBBBBBB..\
      ..BBBBBBBBBBBB..\
      .BBBBBBBBBBBBBB.\
      .BBBWWBBBBWWBBB.\
      .BBBWWBBBBWWBBB.\
      .BBBBBBBBBBBBBB.\
      .BBBBBBBBBBBBBB.\
      .BWWBBWWBBWWBBB.\
      .BBBWWBBWWBBWWB.\
      .BBBBBBBBBBBBBB.\
      .BBBBBBBBBBBBBB.\
      .BB...BBBB...BB.\
      ................",
);

pub static FRIGHT_A: Sprite = sprite16(&FRIGHT_A_D);
pub static FRIGHT_B: Sprite = sprite16(&FRIGHT_B_D);

pub fn fright_sprite(anim: u8) -> &'static Sprite {
    if anim.is_multiple_of(2) {
        &FRIGHT_A
    } else {
        &FRIGHT_B
    }
}

// --- Fruit -------------------------------------------------------------------
// Order matches rules::BonusSymbol: cherry, strawberry, peach, apple, grapes,
// galaxian, bell, key.

const CHERRY_D: [u8; 256] = decode(
    b"................\
      ...........TT...\
      ..........TT.GG.\
      ........TTT...G.\
      ......TT..T.....\
      ....TT....T.....\
      ...T......T.....\
      ..RRRR....T.....\
      .RRRRRR..RRRR...\
      .RWRRRR.RRRRRR..\
      .RWRRRR.RWRRRR..\
      .RRRRRR.RWRRRR..\
      ..RRRR..RRRRRR..\
      ........RRRRRR..\
      .........RRRR...\
      ................",
);

const STRAWBERRY_D: [u8; 256] = decode(
    b"................\
      ......G..G......\
      ...G...GG...G...\
      ....GGGGGGGG....\
      ..RRRRRRRRRRRR..\
      ..RRWRRRRRWRRR..\
      .RRRRRRWRRRRRR..\
      .RWRRRRRRRRRWR..\
      .RRRRRWRRRRRRR..\
      ..RRRRRRRRWRR...\
      ..RWRRRWRRRRR...\
      ...RRRRRRRRR....\
      ....RRWRRRR.....\
      ......RRRR......\
      .......RR.......\
      ................",
);

const PEACH_D: [u8; 256] = decode(
    b"................\
      ..........GG....\
      .......G.GG.....\
      .......GGG......\
      ....OOOOOOOO....\
      ...OOOOOOOOOO...\
      ..OOOOOOOOOOOO..\
      ..OOOOOOOOOOOO..\
      ..OOOOOOOOOOOO..\
      ..OOOOOOOOOOOO..\
      ..OOOOOOOOOOOO..\
      ...OOOOOOOOOO...\
      ....OOOOOOOO....\
      ......OOOO......\
      ................\
      ................",
);

const APPLE_D: [u8; 256] = decode(
    b"................\
      ........TT......\
      .......TT.......\
      ...RRRR.RRRR....\
      ..RRRRRRRRRRRR..\
      .RRWWRRRRRRRRRR.\
      .RWWRRRRRRRRRRR.\
      .RRRRRRRRRRRRRR.\
      .RRRRRRRRRRRRRR.\
      .RRRRRRRRRRRRRR.\
      ..RRRRRRRRRRRR..\
      ..RRRRRRRRRRRR..\
      ...RRRRR.RRRR...\
      ....RRR...RR....\
      ................\
      ................",
);

const GRAPES_D: [u8; 256] = decode(
    b"................\
      .......TT.......\
      ......TT........\
      ..GG..TT..GG....\
      .GGGG.GG.GGGG...\
      .GGGGGGGGGGGG...\
      ..GGGGGGGGGG....\
      ..GG.GGG.GGG....\
      .GGGGGGGGGGGG...\
      .GGGGGGGGGGG....\
      ..GGGGGGGGG.....\
      ...GG.GGG.......\
      ...GGGGGG.......\
      ....GGGG........\
      .....GG.........\
      ................",
);

const GALAXIAN_D: [u8; 256] = decode(
    b"................\
      .......YY.......\
      ......YYYY......\
      .......YY.......\
      ...C...RR...C...\
      ..CC..RRRR..CC..\
      .CCC.RRRRRR.CCC.\
      .CCCRRRRRRRRCCC.\
      .CCRRRRYYRRRRCC.\
      ..CRRRRYYRRRRC..\
      ...RRRR..RRRR...\
      ....RRR..RRR....\
      .....RR..RR.....\
      ......R..R......\
      ................\
      ................",
);

const BELL_D: [u8; 256] = decode(
    b"................\
      .......YY.......\
      ......YYYY......\
      .....YYYYYY.....\
      .....YYYYYY.....\
      ....YYYYYYYY....\
      ....YYYYYYYY....\
      ....YYWYYYYY....\
      ...YYYWYYYYYY...\
      ...YYYYYYYYYY...\
      ..YYYYYYYYYYYY..\
      ..YYYYYYYYYYYY..\
      ..WWWWWWWWWWWW..\
      ......CCCC......\
      ................\
      ................",
);

const KEY_D: [u8; 256] = decode(
    b"................\
      ......CCCC......\
      .....CC..CC.....\
      .....CC..CC.....\
      ......CCCC......\
      .......WW.......\
      .......WW.......\
      .......WW.......\
      .......WWWW.....\
      .......WW.......\
      .......WW.......\
      .......WWWW.....\
      .......WW.......\
      ................\
      ................\
      ................",
);

static FRUITS: [Sprite; 8] = [
    sprite16(&CHERRY_D),
    sprite16(&STRAWBERRY_D),
    sprite16(&PEACH_D),
    sprite16(&APPLE_D),
    sprite16(&GRAPES_D),
    sprite16(&GALAXIAN_D),
    sprite16(&BELL_D),
    sprite16(&KEY_D),
];

/// Fruit sprite by symbol index 0..=7 (cherry..key); wraps defensively.
pub fn fruit_sprite(idx: u8) -> &'static Sprite {
    &FRUITS[(idx % 8) as usize]
}

// --- Energizer (8x8 blob, blinks) --------------------------------------------

const ENERGIZER_D: [u8; 64] = decode(
    b"..DDDD..\
      .DDDDDD.\
      DDDDDDDD\
      DDDDDDDD\
      DDDDDDDD\
      DDDDDDDD\
      .DDDDDD.\
      ..DDDD..",
);

pub static ENERGIZER: Sprite = Sprite {
    w: 8,
    h: 8,
    data: &ENERGIZER_D,
};
