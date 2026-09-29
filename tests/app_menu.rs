//! Main-menu model tests: navigation, value cycling, actions, screen labels.

use pacmantui::app::menu::{
    self, MainMenu, MenuAction, ROW_CONTROLS, ROW_DIFFICULTY, ROW_MAP, ROW_PLAY, ROW_QUIT,
    ROW_SCORES, ROWS,
};
use pacmantui::render::Key;
use pacmantui::types::Difficulty;

const N_MAPS: usize = 2;

fn names() -> Vec<String> {
    vec!["Classic".into(), "Vertigo".into()]
}

#[test]
fn navigation_wraps_both_ways() {
    let mut m = MainMenu::new(0, Difficulty::Normal);
    assert_eq!(m.selected, ROW_PLAY);
    m.on_key(Key::Up, N_MAPS);
    assert_eq!(m.selected, ROWS - 1);
    m.on_key(Key::Down, N_MAPS);
    assert_eq!(m.selected, ROW_PLAY);
    for _ in 0..ROWS {
        m.on_key(Key::Down, N_MAPS);
    }
    assert_eq!(m.selected, ROW_PLAY);
}

#[test]
fn map_row_cycles_maps_in_both_directions() {
    let mut m = MainMenu::new(0, Difficulty::Normal);
    m.selected = ROW_MAP;
    m.on_key(Key::Right, N_MAPS);
    assert_eq!(m.map_idx, 1);
    m.on_key(Key::Right, N_MAPS);
    assert_eq!(m.map_idx, 0);
    m.on_key(Key::Left, N_MAPS);
    assert_eq!(m.map_idx, 1);
    // Enter also cycles on the map row.
    assert_eq!(m.on_key(Key::Enter, N_MAPS), MenuAction::None);
    assert_eq!(m.map_idx, 0);
}

#[test]
fn difficulty_row_toggles() {
    let mut m = MainMenu::new(0, Difficulty::Normal);
    m.selected = ROW_DIFFICULTY;
    m.on_key(Key::Right, N_MAPS);
    assert_eq!(m.difficulty, Difficulty::Hard);
    m.on_key(Key::Left, N_MAPS);
    assert_eq!(m.difficulty, Difficulty::Normal);
}

#[test]
fn left_right_do_nothing_on_action_rows() {
    let mut m = MainMenu::new(1, Difficulty::Hard);
    for row in [ROW_PLAY, ROW_SCORES, ROW_CONTROLS, ROW_QUIT] {
        m.selected = row;
        assert_eq!(m.on_key(Key::Left, N_MAPS), MenuAction::None);
        assert_eq!(m.on_key(Key::Right, N_MAPS), MenuAction::None);
    }
    assert_eq!(m.map_idx, 1);
    assert_eq!(m.difficulty, Difficulty::Hard);
}

#[test]
fn enter_dispatches_actions_and_q_quits_anywhere() {
    let mut m = MainMenu::new(0, Difficulty::Normal);
    m.selected = ROW_PLAY;
    assert_eq!(m.on_key(Key::Enter, N_MAPS), MenuAction::Play);
    m.selected = ROW_SCORES;
    assert_eq!(m.on_key(Key::Enter, N_MAPS), MenuAction::Scores);
    m.selected = ROW_CONTROLS;
    assert_eq!(m.on_key(Key::Enter, N_MAPS), MenuAction::Controls);
    m.selected = ROW_QUIT;
    assert_eq!(m.on_key(Key::Enter, N_MAPS), MenuAction::Quit);
    m.selected = ROW_PLAY;
    assert_eq!(m.on_key(Key::Quit, N_MAPS), MenuAction::Quit);
}

#[test]
fn screen_labels_are_uppercase_and_reflect_state() {
    let mut m = MainMenu::new(1, Difficulty::Hard);
    m.selected = ROW_DIFFICULTY;
    let s = m.screen(&names());
    assert_eq!(s.items.len(), ROWS);
    assert_eq!(s.selected, ROW_DIFFICULTY);
    assert_eq!(s.items[ROW_MAP].value.as_deref(), Some("VERTIGO"));
    assert_eq!(s.items[ROW_DIFFICULTY].value.as_deref(), Some("HARD"));
    // The pixel font is uppercase-only; lowercase would render blank.
    for item in &s.items {
        assert_eq!(item.label, item.label.to_uppercase(), "{}", item.label);
        if let Some(v) = &item.value {
            assert_eq!(v, &v.to_uppercase());
        }
    }
}

#[test]
fn scores_screen_shows_rows_or_placeholder() {
    let empty = menu::scores_screen("Classic", Difficulty::Normal, &[]);
    assert!(empty.items.iter().any(|i| i.label == "NO SCORES YET"));

    let filled = menu::scores_screen("Vertigo", Difficulty::Hard, &[(5000, 3), (100, 1)]);
    assert_eq!(filled.items[0].label, "VERTIGO");
    assert_eq!(filled.items[0].value.as_deref(), Some("HARD"));
    assert_eq!(filled.items[1].label, "1.");
    assert_eq!(filled.items[1].value.as_deref(), Some("5000 L3"));
    assert_eq!(filled.items[2].value.as_deref(), Some("100 L1"));
}

#[test]
fn controls_screen_is_uppercase_and_complete() {
    let s = menu::controls_screen();
    assert!(s.items.len() >= 5);
    for item in &s.items {
        assert_eq!(item.label, item.label.to_uppercase());
        if let Some(v) = &item.value {
            assert_eq!(v, &v.to_uppercase());
        }
    }
}
