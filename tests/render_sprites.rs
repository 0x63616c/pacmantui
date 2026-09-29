//! Sprite art invariants: dimensions, symmetry, direction derivation, remaps,
//! fruit signatures, death-animation shrink (CPU-only).

use pacmantui::render::test_api::*;
use pacmantui::types::{Dir, GhostId};

fn count(sp: &Sprite, idx: u8) -> usize {
    sp.data.iter().filter(|&&v| v == idx).count()
}

fn at(sp: &Sprite, x: usize, y: usize) -> u8 {
    sp.data[y * sp.w + x]
}

#[test]
fn sprite_dimensions() {
    for sp in [
        &PAC_CLOSED,
        &PAC_OPEN_R,
        &PAC_OPEN_L,
        &PAC_OPEN_U,
        &PAC_OPEN_D,
        &GHOST_A,
        &GHOST_B,
        &FRIGHT_A,
        &FRIGHT_B,
        &EYES_R,
        &EYES_L,
        &EYES_U,
        &EYES_D,
    ] {
        assert_eq!((sp.w, sp.h), (16, 16));
        assert_eq!(sp.data.len(), 256);
    }
    for i in 0..8 {
        let f = fruit_sprite(i);
        assert_eq!((f.w, f.h), (16, 16));
    }
    assert_eq!((ENERGIZER.w, ENERGIZER.h), (8, 8));
    assert_eq!(ENERGIZER.data.len(), 64);
}

#[test]
fn pac_closed_is_a_symmetric_disc() {
    for y in 0..16 {
        for x in 0..16 {
            assert_eq!(
                at(&PAC_CLOSED, x, y),
                at(&PAC_CLOSED, 15 - x, y),
                "asym at ({x},{y})"
            );
        }
    }
    // ~13px disc: a solid chunk of yellow, transparent border ring.
    let yellow = count(&PAC_CLOSED, YELLOW);
    assert!((120..=160).contains(&yellow), "disc size {yellow}");
    for i in 0..16 {
        assert_eq!(at(&PAC_CLOSED, i, 0), TRANSPARENT);
        assert_eq!(at(&PAC_CLOSED, i, 15), TRANSPARENT);
        assert_eq!(at(&PAC_CLOSED, 0, i), TRANSPARENT);
        assert_eq!(at(&PAC_CLOSED, 15, i), TRANSPARENT);
    }
}

#[test]
fn mouth_wedge_faces_each_direction() {
    // Mouth pixel deep on the facing side is cut out of the disc.
    assert_eq!(at(&PAC_CLOSED, 12, 7), YELLOW);
    assert_eq!(at(&PAC_OPEN_R, 12, 7), TRANSPARENT);
    assert_eq!(at(&PAC_OPEN_L, 3, 7), TRANSPARENT);
    assert_eq!(at(&PAC_OPEN_D, 7, 12), TRANSPARENT);
    assert_eq!(at(&PAC_OPEN_U, 7, 3), TRANSPARENT);
    // The back of the head stays solid.
    assert_eq!(at(&PAC_OPEN_R, 3, 7), YELLOW);
    assert_eq!(at(&PAC_OPEN_L, 12, 7), YELLOW);
    assert_eq!(at(&PAC_OPEN_U, 7, 12), YELLOW);
    assert_eq!(at(&PAC_OPEN_D, 7, 3), YELLOW);
}

#[test]
fn chomp_cycle_mapping() {
    // Phase 0 -> direction-neutral closed disc for every direction.
    for d in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
        assert!(std::ptr::eq(pac_sprite(d, 0), &PAC_CLOSED));
        assert!(std::ptr::eq(pac_sprite(d, 4), &PAC_CLOSED));
    }
    // Phase 2 -> full open; phases 1 and 3 -> the same half frame.
    assert!(std::ptr::eq(pac_sprite(Dir::Right, 2), &PAC_OPEN_R));
    assert!(std::ptr::eq(
        pac_sprite(Dir::Right, 1),
        pac_sprite(Dir::Right, 3)
    ));
    // Half-open is between closed and open in mouth size.
    let half = pac_sprite(Dir::Right, 1);
    let (c, h, o) = (
        count(&PAC_CLOSED, YELLOW),
        count(half, YELLOW),
        count(&PAC_OPEN_R, YELLOW),
    );
    assert!(c > h && h > o, "closed {c} > half {h} > open {o}");
}

#[test]
fn death_animation_shrinks_then_bursts() {
    let mut prev = usize::MAX;
    for f in 0..=5 {
        let y = count(pac_death_sprite(f), YELLOW);
        assert!(y <= prev, "frame {f} grew: {y} > {prev}");
        assert!(y > 0, "frames 0..=5 keep some body");
        prev = y;
    }
    for f in [6, 7] {
        let sp = pac_death_sprite(f);
        assert_eq!(count(sp, YELLOW), 0, "burst frames are white only");
        assert!(count(sp, WHITE) > 0);
    }
    // Clamped beyond the last frame.
    assert!(std::ptr::eq(pac_death_sprite(9), pac_death_sprite(7)));
}

#[test]
fn ghost_wave_frames_differ_only_in_skirt() {
    assert_ne!(GHOST_A.data, GHOST_B.data);
    // Rows above the skirt are identical.
    assert_eq!(&GHOST_A.data[..14 * 16], &GHOST_B.data[..14 * 16]);
    assert!(count(&GHOST_A, BODY) > 100, "solid body fill");
    assert!(std::ptr::eq(ghost_body(0), &GHOST_A));
    assert!(std::ptr::eq(ghost_body(1), &GHOST_B));
}

#[test]
fn eyes_look_toward_direction() {
    // Pupils (blue) at direction-dependent spots; sclera white around them.
    assert_eq!(at(&EYES_R, 4, 4), MAZE_BLUE);
    assert_eq!(at(&EYES_R, 11, 4), MAZE_BLUE);
    assert_eq!(at(&EYES_L, 2, 4), MAZE_BLUE);
    assert_eq!(at(&EYES_L, 9, 4), MAZE_BLUE);
    assert_eq!(at(&EYES_U, 3, 3), MAZE_BLUE);
    assert_eq!(at(&EYES_D, 3, 6), MAZE_BLUE);
    // The base sclera stays white where no pupil sits.
    assert_eq!(at(&EYES_R, 2, 4), WHITE);
    assert_eq!(at(&EYES_L, 4, 4), WHITE);
    for (d, sp) in [
        (Dir::Right, &EYES_R),
        (Dir::Left, &EYES_L),
        (Dir::Up, &EYES_U),
        (Dir::Down, &EYES_D),
    ] {
        assert!(std::ptr::eq(eyes_sprite(d), sp));
    }
}

#[test]
fn ghost_color_remaps() {
    assert_eq!(ghost_remap(GhostId::Blinky)[BODY as usize], RED);
    assert_eq!(ghost_remap(GhostId::Pinky)[BODY as usize], PINK);
    assert_eq!(ghost_remap(GhostId::Inky)[BODY as usize], CYAN);
    assert_eq!(ghost_remap(GhostId::Clyde)[BODY as usize], ORANGE);
    // Everything else stays identity.
    let m = ghost_remap(GhostId::Blinky);
    for i in 0..16u8 {
        if i != BODY {
            assert_eq!(m[i as usize], i);
        }
    }
    // Palette colors match the arcade-ish spec.
    assert_eq!(rgb(RED), [255, 0, 0]);
    assert_eq!(rgb(PINK), [255, 184, 255]);
    assert_eq!(rgb(CYAN), [0, 255, 255]);
    assert_eq!(rgb(ORANGE), [255, 184, 82]);
    assert_eq!(rgb(FRIGHT_BLUE), [33, 33, 255]);
}

#[test]
fn frightened_and_flash_variant() {
    // Blue body with white face features.
    assert!(count(&FRIGHT_A, FRIGHT_BLUE) > 100);
    assert!(count(&FRIGHT_A, WHITE) > 10);
    assert!(std::ptr::eq(fright_sprite(0), &FRIGHT_A));
    assert!(std::ptr::eq(fright_sprite(1), &FRIGHT_B));
    // Flash remap: blue -> white, white -> red; transparent stays.
    assert_eq!(FRIGHT_FLASH_REMAP[FRIGHT_BLUE as usize], WHITE);
    assert_eq!(FRIGHT_FLASH_REMAP[WHITE as usize], RED);
    assert_eq!(FRIGHT_FLASH_REMAP[TRANSPARENT as usize], TRANSPARENT);
}

#[test]
fn fruit_signature_colors() {
    let sig: [(u8, &[u8]); 8] = [
        (0, &[RED, TAN, GREEN]),     // cherry
        (1, &[RED, GREEN, WHITE]),   // strawberry
        (2, &[ORANGE, GREEN]),       // peach
        (3, &[RED, TAN]),            // apple
        (4, &[GREEN, TAN]),          // grapes
        (5, &[CYAN, RED, YELLOW]),   // galaxian
        (6, &[YELLOW, WHITE, CYAN]), // bell
        (7, &[CYAN, WHITE]),         // key
    ];
    for (idx, colors) in sig {
        let sp = fruit_sprite(idx);
        for &c in colors {
            assert!(count(sp, c) > 0, "fruit {idx} missing color index {c}");
        }
        assert!(
            count(sp, TRANSPARENT) > 60,
            "fruit {idx} must not fill the cell"
        );
    }
    // Defensive wrap.
    assert!(std::ptr::eq(fruit_sprite(8), fruit_sprite(0)));
}

#[test]
fn energizer_is_a_peach_blob() {
    assert!(count(&ENERGIZER, DOT_PEACH) > 40);
    assert_eq!(at(&ENERGIZER, 0, 0), TRANSPARENT);
    assert_eq!(at(&ENERGIZER, 4, 4), DOT_PEACH);
}

#[test]
fn life_icon_is_a_pac() {
    let sp = life_sprite();
    assert_eq!((sp.w, sp.h), (16, 16));
    assert!(count(sp, YELLOW) > 80);
}
