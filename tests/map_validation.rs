//! Malformed-map corpus: at least one test per validation rule in
//! docs/map-format.md, asserting the MapError variant and message substring.

use pacmantui::map::{Map, MapError};
use pacmantui::types::{Fix8, PxPos, TilePos};

/// Minimal valid map used as the mutation base. 11x7, one-row house with a
/// door on top, ring corridor, two energizers, 22 dots.
const MINI_GRID: &str = "\
###########
#o.......o#
#.###-###.#
#.#HHHHH#.#
#.#######.#
#.........#
###########";

fn mini_doc(grid: &str) -> String {
    format!(
        r#"format_version = 1
name = "Mini"
id = "mini"

grid = '''
{grid}
'''

no_up_tiles = [[1, 5]]

[spawns]
pac = {{ tile = [5, 5], facing = "left" }}
blinky = {{ tile = [5, 1], facing = "left" }}
pinky = {{ tile = [5, 3], facing = "down" }}
inky = {{ tile = [4, 3], facing = "up" }}
clyde = {{ tile = [6, 3], facing = "up" }}

[scatter]
blinky = [10, -1]
pinky = [0, -1]
inky = [10, 7]
clyde = [0, 7]

[house]
center = {{ tile = [5, 3] }}
eyes_target = [5, 1]

[fruit]
pos = {{ tile = [4, 5] }}
"#
    )
}

fn mini() -> String {
    mini_doc(MINI_GRID)
}

#[track_caller]
fn assert_invalid(src: &str, substr: &str) {
    match Map::parse(src) {
        Err(MapError::Invalid(msg)) => {
            assert!(
                msg.contains(substr),
                "Invalid message {msg:?} does not contain {substr:?}"
            );
        }
        other => panic!("expected Invalid(..{substr:?}..), got {other:?}"),
    }
}

#[track_caller]
fn assert_syntax(src: &str, substr: &str) {
    match Map::parse(src) {
        Err(MapError::Syntax { msg, .. }) => {
            assert!(
                msg.contains(substr),
                "Syntax message {msg:?} does not contain {substr:?}"
            );
        }
        other => panic!("expected Syntax(..{substr:?}..), got {other:?}"),
    }
}

// --- the base map itself is valid ---

#[test]
fn mini_base_is_valid() {
    let m = Map::parse(&mini()).expect("mini base map must validate");
    assert_eq!(m.dots_total(), 22);
    assert_eq!(m.pellets_total(), 24);
    assert_eq!(m.door_tiles(), &[TilePos::new(5, 2)]);
    assert_eq!(
        m.pac_spawn(),
        PxPos {
            x: Fix8::from_px(44),
            y: Fix8::from_px(44)
        }
    );
}

// --- version handling ---

#[test]
fn wrong_version_is_unsupported() {
    let src = mini().replace("format_version = 1", "format_version = 3");
    assert_eq!(
        Map::parse(&src).unwrap_err(),
        MapError::UnsupportedVersion(3)
    );
}

#[test]
fn missing_version() {
    let src = mini().replace("format_version = 1\n", "");
    assert_invalid(&src, "format_version");
}

#[test]
fn non_integer_version() {
    let src = mini().replace("format_version = 1", "format_version = \"1\"");
    assert_invalid(&src, "`format_version` must be an integer");
}

// --- TOML / schema errors ---

#[test]
fn toml_garbage_is_syntax_error() {
    assert_syntax("this is ] not [ toml =", "");
}

#[test]
fn unknown_key_is_rejected() {
    let src = mini() + "\nwibble = 1\n";
    assert_syntax(&src, "wibble");
}

#[test]
fn missing_ghost_spawn() {
    let src = mini().replace("pinky = { tile = [5, 3], facing = \"down\" }\n", "");
    assert_syntax(&src, "pinky");
}

#[test]
fn bad_facing_value() {
    let src = mini().replace(
        "facing = \"left\" }\nblinky",
        "facing = \"north\" }\nblinky",
    );
    assert_syntax(&src, "north");
}

// --- metadata ---

#[test]
fn empty_name() {
    let src = mini().replace("name = \"Mini\"", "name = \"\"");
    assert_invalid(&src, "name must not be empty");
}

#[test]
fn bad_id_charset() {
    let src = mini().replace("id = \"mini\"", "id = \"Mini!\"");
    assert_invalid(&src, "map id");
}

// --- grid shape and characters ---

#[test]
fn empty_grid() {
    let src = mini().replace(&format!("'''\n{MINI_GRID}\n'''"), "''");
    assert_invalid(&src, "grid is empty");
}

#[test]
fn non_rectangular_grid() {
    let src = mini().replace("#o.......o#", "#o.......o##");
    assert_invalid(&src, "rectangular");
}

#[test]
fn illegal_grid_char() {
    let src = mini().replace("#.........#", "#....X....#");
    assert_invalid(&src, "illegal character 'X'");
}

// --- board structure ---

#[test]
fn no_dots() {
    let src = mini().replace('.', "_");
    assert_invalid(&src, "no dots");
}

#[test]
fn no_door() {
    let src = mini().replace("#.###-###.#", "#.#######.#");
    assert_invalid(&src, "no door");
}

#[test]
fn no_house() {
    let src = mini().replace("#.#HHHHH#.#", "#.#######.#");
    assert_invalid(&src, "no ghost-house interior");
}

#[test]
fn door_not_adjacent_to_house() {
    // Extra door dropped into the bottom corridor, far from the house.
    let src = mini().replace("#.........#", "#....-....#");
    assert_invalid(&src, "not adjacent to any house interior");
}

#[test]
fn house_not_sealed() {
    // Open the house floor onto the corridor below.
    let src = mini().replace("#.#######.#", "#.##.####.#");
    assert_invalid(&src, "without a door");
}

// --- spawns ---

#[test]
fn pac_spawn_on_wall() {
    let src = mini().replace("pac = { tile = [5, 5]", "pac = { tile = [0, 0]");
    assert_invalid(&src, "pac spawn occupies tile (0, 0)");
}

#[test]
fn pac_spawn_on_door() {
    let src = mini().replace("pac = { tile = [5, 5]", "pac = { tile = [5, 2]");
    assert_invalid(&src, "pac spawn occupies tile (5, 2) which is a door");
}

#[test]
fn ghost_spawn_on_wall() {
    let src = mini().replace("blinky = { tile = [5, 1]", "blinky = { tile = [0, 0]");
    assert_invalid(&src, "blinky spawn occupies tile (0, 0)");
}

#[test]
fn ghost_spawn_on_door() {
    let src = mini().replace("pinky = { tile = [5, 3]", "pinky = { tile = [5, 2]");
    assert_invalid(&src, "pinky spawn occupies tile (5, 2)");
}

#[test]
fn pac_spawn_overlapping_ghost() {
    let src = mini().replace("pac = { tile = [5, 5]", "pac = { tile = [5, 1]");
    assert_invalid(&src, "overlaps the blinky spawn");
}

#[test]
fn spawn_tile_out_of_grid() {
    let src = mini().replace("pac = { tile = [5, 5]", "pac = { tile = [99, 5]");
    assert_invalid(&src, "outside the 11x7 grid");
}

#[test]
fn spawn_offset_out_of_range() {
    let src = mini().replace(
        "pac = { tile = [5, 5]",
        "pac = { tile = [5, 5], offset_px = [5, 0]",
    );
    assert_invalid(&src, "offset_px");
}

// --- special points ---

#[test]
fn house_center_outside_house() {
    let src = mini().replace("center = { tile = [5, 3] }", "center = { tile = [5, 1] }");
    assert_invalid(&src, "house center (5, 1) is not inside the house interior");
}

#[test]
fn eyes_target_not_walkable() {
    let src = mini().replace("eyes_target = [5, 1]", "eyes_target = [0, 0]");
    assert_invalid(&src, "eyes target (0, 0) is not a walkable board tile");
}

#[test]
fn fruit_on_wall() {
    let src = mini().replace("pos = { tile = [4, 5] }", "pos = { tile = [0, 0] }");
    assert_invalid(&src, "fruit position occupies tile (0, 0)");
}

// --- scatter / no-up ---

#[test]
fn scatter_target_out_of_range() {
    let src = mini().replace("blinky = [10, -1]", "blinky = [1000, -1]");
    assert_invalid(&src, "scatter target (1000, -1) is out of range");
}

#[test]
fn no_up_tile_not_walkable() {
    let src = mini().replace("no_up_tiles = [[1, 5]]", "no_up_tiles = [[0, 0]]");
    assert_invalid(&src, "no-up tile (0, 0) is not a walkable tile");
}

#[test]
fn duplicate_no_up_tile() {
    let src = mini().replace("no_up_tiles = [[1, 5]]", "no_up_tiles = [[1, 5], [1, 5]]");
    assert_invalid(&src, "duplicate no-up tile (1, 5)");
}

// --- warp pairing ---

#[test]
fn unpaired_warp_tile() {
    // Open the bottom corridor onto the left edge only.
    let src = mini().replace("#.........#", "..........#");
    assert_invalid(&src, "unpaired warp tile");
}

#[test]
fn paired_warp_tiles_are_accepted() {
    // Opening both ends of the bottom corridor makes a legal tunnel.
    let src = mini().replace("#.........#", "...........");
    let m = Map::parse(&src).expect("paired tunnel must validate");
    let y = Fix8::from_px(5 * 8 + 4);
    let out = PxPos {
        x: Fix8::from_px(-1),
        y,
    };
    assert_eq!(
        m.warp(out),
        PxPos {
            x: Fix8::from_px(11 * 8 - 1),
            y
        }
    );
}

// --- reachability ---

#[test]
fn unreachable_dot_in_sealed_pocket() {
    let grid = "\
###########
#o.......o#
#.###-###.#
#.#HHHHH#.#
#.#######.#
#....#....#
#####.#####
###########";
    // (5,6) is a dot sealed on all sides; pac moves to (1,5).
    let src = mini_doc(grid).replace("pac = { tile = [5, 5]", "pac = { tile = [1, 5]");
    assert_invalid(&src, "dot at (5, 6) is unreachable from the pac spawn");
}

#[test]
fn dot_behind_door_is_unreachable_for_pac() {
    // A dot pocket whose only access is through the house doors: Pac-Man
    // must not be able to route through doors, so validation rejects it.
    let grid = "\
###########
#o.......o#
#.###-###.#
#.#HHHHH#.#
#.###-###.#
#.###.###.#
#....#....#
###########";
    let src = mini_doc(grid)
        .replace("pac = { tile = [5, 5]", "pac = { tile = [1, 6]")
        .replace("pos = { tile = [4, 5] }", "pos = { tile = [1, 5] }");
    assert_invalid(&src, "dot at (5, 5) is unreachable from the pac spawn");
}

#[test]
fn door_blocking_eyes_return() {
    // Wall directly above the door: ghosts can never cross between the house
    // and the board, so eyes could never return.
    let src = mini()
        .replace("#o.......o#", "#o...#...o#")
        .replace("blinky = { tile = [5, 1]", "blinky = { tile = [1, 1]")
        .replace("eyes_target = [5, 1]", "eyes_target = [1, 1]");
    assert_invalid(&src, "no ghost route between the house center");
}

#[test]
fn in_house_ghost_disconnected_from_house_center() {
    // Second, fully sealed house cell at (7,3): pinky spawns there but eyes
    // and releases route through the real house at (3,3).
    let grid = "\
###########
#o.......o#
#.#-#####.#
#.#H###H#.#
#.#######.#
#.........#
###########";
    let src = mini_doc(grid)
        .replace("center = { tile = [5, 3] }", "center = { tile = [3, 3] }")
        .replace("pinky = { tile = [5, 3]", "pinky = { tile = [7, 3]")
        .replace("inky = { tile = [4, 3]", "inky = { tile = [3, 3]")
        .replace("clyde = { tile = [6, 3]", "clyde = { tile = [3, 3]")
        .replace("eyes_target = [5, 1]", "eyes_target = [3, 1]");
    assert_invalid(
        &src,
        "pinky spawn tile (7, 3) is not reachable from the house center",
    );
}

#[test]
fn outside_ghost_unreachable_from_pac() {
    // Blinky spawns in a sealed path pocket at (9,5) (no pellet there, so
    // pellet reachability passes; the ghost-spawn check must catch it).
    let grid = "\
###########
#o.......o#
#.###-###.#
#.#HHHHH#.#
#.#########
#.......#_#
###########";
    let src = mini_doc(grid).replace("blinky = { tile = [5, 1]", "blinky = { tile = [9, 5]");
    assert_invalid(
        &src,
        "blinky spawn tile (9, 5) is not reachable from the pac spawn",
    );
}

// --- rules ---

#[test]
fn threshold_scale_must_be_positive() {
    let src = mini() + "\n[rules]\nthreshold_scale = 0.0\n";
    assert_invalid(&src, "threshold_scale");
}

#[test]
fn threshold_scale_override_applies() {
    let src = mini() + "\n[rules]\nthreshold_scale = 2.0\n";
    let m = Map::parse(&src).expect("override map must validate");
    assert_eq!(m.scale_dot_threshold(10), 20);
    assert_eq!(m.scale_dot_threshold(7), 14);
}

#[test]
fn default_scaling_uses_pellet_ratio() {
    let m = Map::parse(&mini()).expect("mini base map must validate");
    // mini has 24 pellets: round(v * 24 / 244).
    assert!(!m.is_classic());
    assert_eq!(m.scale_dot_threshold(61), 6); // 6.0 exactly
    assert_eq!(m.scale_dot_threshold(10), 1); // 0.98 -> 1
    assert_eq!(m.scale_dot_threshold(0), 0);
}

// --- io ---

#[test]
fn load_file_missing_is_io_error() {
    let err = Map::load_file(std::path::Path::new("maps/does-not-exist.pmtoml")).unwrap_err();
    assert!(matches!(err, MapError::Io(_)), "expected Io, got {err:?}");
}
