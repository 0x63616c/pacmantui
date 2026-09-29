//! Pure main-menu model: item list, navigation, and value cycling.
//!
//! Kept free of terminal I/O so menu behavior is unit-testable
//! (tests/app_menu.rs). The app layer feeds [`crate::render::Key`]s in and
//! renders the [`MenuScreen`] this model builds.
//!
//! The pixel font is uppercase-only (A-Z, digits, minimal punctuation), so
//! every label produced here must be uppercase.

use crate::render::{Key, MenuItem, MenuScreen};
use crate::types::Difficulty;

/// Row indices in the fixed main-menu layout.
pub const ROW_PLAY: usize = 0;
pub const ROW_MAP: usize = 1;
pub const ROW_DIFFICULTY: usize = 2;
pub const ROW_SCORES: usize = 3;
pub const ROW_CONTROLS: usize = 4;
pub const ROW_QUIT: usize = 5;
pub const ROWS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Nothing to do (possibly a selection/value change; re-render).
    None,
    Play,
    Scores,
    Controls,
    Quit,
}

#[derive(Debug, Clone)]
pub struct MainMenu {
    pub selected: usize,
    pub map_idx: usize,
    pub difficulty: Difficulty,
}

impl MainMenu {
    pub fn new(map_idx: usize, difficulty: Difficulty) -> MainMenu {
        MainMenu {
            selected: ROW_PLAY,
            map_idx,
            difficulty,
        }
    }

    /// Apply one key. `n_maps` is the number of selectable maps (>= 1).
    pub fn on_key(&mut self, key: Key, n_maps: usize) -> MenuAction {
        match key {
            Key::Up => self.selected = (self.selected + ROWS - 1) % ROWS,
            Key::Down => self.selected = (self.selected + 1) % ROWS,
            Key::Left => self.cycle(n_maps, false),
            Key::Right => self.cycle(n_maps, true),
            Key::Enter => match self.selected {
                ROW_PLAY => return MenuAction::Play,
                ROW_MAP | ROW_DIFFICULTY => self.cycle(n_maps, true),
                ROW_SCORES => return MenuAction::Scores,
                ROW_CONTROLS => return MenuAction::Controls,
                ROW_QUIT => return MenuAction::Quit,
                _ => {}
            },
            Key::Quit => return MenuAction::Quit,
            _ => {}
        }
        MenuAction::None
    }

    fn cycle(&mut self, n_maps: usize, forward: bool) {
        match self.selected {
            ROW_MAP if n_maps > 0 => {
                self.map_idx = if forward {
                    (self.map_idx + 1) % n_maps
                } else {
                    (self.map_idx + n_maps - 1) % n_maps
                };
            }
            ROW_DIFFICULTY => {
                self.difficulty = match self.difficulty {
                    Difficulty::Normal => Difficulty::Hard,
                    Difficulty::Hard => Difficulty::Normal,
                };
            }
            _ => {}
        }
    }

    /// Build the drawable screen. `map_names` are display names (any case).
    pub fn screen(&self, map_names: &[String]) -> MenuScreen {
        let map_name = map_names
            .get(self.map_idx)
            .cloned()
            .unwrap_or_else(|| "?".into())
            .to_uppercase();
        let item = |label: &str, value: Option<String>| MenuItem {
            label: label.into(),
            value,
            enabled: true,
        };
        MenuScreen {
            title: "PACMANTUI".into(),
            items: vec![
                item("PLAY", None),
                item("MAP", Some(map_name)),
                item(
                    "DIFFICULTY",
                    Some(
                        match self.difficulty {
                            Difficulty::Normal => "NORMAL",
                            Difficulty::Hard => "HARD",
                        }
                        .into(),
                    ),
                ),
                item("HIGH SCORES", None),
                item("CONTROLS", None),
                item("QUIT", None),
            ],
            selected: self.selected,
            footer: "ARROWS MOVE - ENTER SELECT".into(),
            decorated: true,
        }
    }
}

/// Static controls screen (uppercase-only labels; see module docs).
pub fn controls_screen() -> MenuScreen {
    let row = |label: &str, value: &str| MenuItem {
        label: label.into(),
        value: Some(value.into()),
        enabled: true,
    };
    MenuScreen {
        title: "CONTROLS".into(),
        items: vec![
            row("MOVE", "ARROWS OR WASD"),
            row("PAUSE", "P"),
            row("RESTART", "R WHEN PAUSED"),
            row("MENU", "ESC"),
            row("QUIT", "Q OR CTRL C"),
        ],
        selected: usize::MAX,
        footer: "ESC BACK".into(),
        decorated: false,
    }
}

/// High-score screen for one (map, difficulty) table.
/// `rows` are (score, level) best-first; `map_name`/`difficulty` label the table.
pub fn scores_screen(map_name: &str, difficulty: Difficulty, rows: &[(u32, u32)]) -> MenuScreen {
    let mut items: Vec<MenuItem> = vec![MenuItem {
        label: map_name.to_uppercase(),
        value: Some(
            match difficulty {
                Difficulty::Normal => "NORMAL",
                Difficulty::Hard => "HARD",
            }
            .into(),
        ),
        enabled: false,
    }];
    if rows.is_empty() {
        items.push(MenuItem {
            label: "NO SCORES YET".into(),
            value: None,
            enabled: true,
        });
    }
    for (i, (score, level)) in rows.iter().take(8).enumerate() {
        items.push(MenuItem {
            label: format!("{}.", i + 1),
            value: Some(format!("{score} L{level}")),
            enabled: true,
        });
    }
    MenuScreen {
        title: "HIGH SCORES".into(),
        items,
        selected: usize::MAX,
        footer: "< > MAP - ESC BACK".into(),
        decorated: false,
    }
}
