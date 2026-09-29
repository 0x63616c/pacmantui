//! Persistence tests: corrupt/missing tolerance, table separation, capping
//! (acceptance A2).

use std::fs;
use std::path::PathBuf;

use pacmantui::app::persist::{MAX_SCORES_PER_TABLE, Store};
use pacmantui::types::Difficulty;

/// Fresh scratch dir per test (no env vars: tests run in parallel).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pacmantui-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn missing_files_load_defaults() {
    let store = Store::load(scratch("missing"));
    assert_eq!(store.settings.map, "classic");
    assert_eq!(store.settings.difficulty(), Difficulty::Normal);
    assert_eq!(store.high_score("classic", Difficulty::Normal), 0);
}

#[test]
fn corrupt_files_load_defaults() {
    let dir = scratch("corrupt");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("settings.toml"), "map = [not valid toml").unwrap();
    fs::write(dir.join("scores.toml"), "\u{0}\u{1}binary junk").unwrap();
    let store = Store::load(dir);
    assert_eq!(store.settings.map, "classic");
    assert!(store.top_scores("classic", Difficulty::Normal).is_empty());
}

#[test]
fn saves_and_reloads_round_trip() {
    let dir = scratch("roundtrip");
    let mut store = Store::load(dir.clone());
    store.settings.map = "vertigo".into();
    store.settings.set_difficulty(Difficulty::Hard);
    assert!(store.record_score("vertigo", Difficulty::Hard, 5000, 3));
    store.save().unwrap();

    let re = Store::load(dir);
    assert_eq!(re.settings.map, "vertigo");
    assert_eq!(re.settings.difficulty(), Difficulty::Hard);
    assert_eq!(re.high_score("vertigo", Difficulty::Hard), 5000);
    assert_eq!(re.top_scores("vertigo", Difficulty::Hard)[0].level, 3);
}

#[test]
fn tables_are_separated_by_map_and_difficulty() {
    let mut store = Store::load(scratch("separate"));
    store.record_score("classic", Difficulty::Normal, 100, 1);
    store.record_score("classic", Difficulty::Hard, 200, 1);
    store.record_score("vertigo", Difficulty::Normal, 300, 1);
    assert_eq!(store.high_score("classic", Difficulty::Normal), 100);
    assert_eq!(store.high_score("classic", Difficulty::Hard), 200);
    assert_eq!(store.high_score("vertigo", Difficulty::Normal), 300);
    assert_eq!(store.high_score("vertigo", Difficulty::Hard), 0);
}

#[test]
fn tables_cap_at_max_and_sort_best_first() {
    let mut store = Store::load(scratch("cap"));
    for i in 1..=15u32 {
        store.record_score("classic", Difficulty::Normal, i * 10, 1);
    }
    let top = store.top_scores("classic", Difficulty::Normal);
    assert_eq!(top.len(), MAX_SCORES_PER_TABLE);
    assert_eq!(top[0].score, 150);
    assert_eq!(top.last().unwrap().score, 60);
    // Other tables unaffected by the trim.
    store.record_score("vertigo", Difficulty::Normal, 1, 1);
    assert_eq!(store.top_scores("vertigo", Difficulty::Normal).len(), 1);
}

#[test]
fn zero_scores_are_not_recorded_and_high_flag_is_correct() {
    let mut store = Store::load(scratch("zero"));
    assert!(!store.record_score("classic", Difficulty::Normal, 0, 1));
    assert!(store.top_scores("classic", Difficulty::Normal).is_empty());
    assert!(store.record_score("classic", Difficulty::Normal, 50, 1));
    assert!(!store.record_score("classic", Difficulty::Normal, 40, 1));
    assert!(store.record_score("classic", Difficulty::Normal, 60, 2));
}
