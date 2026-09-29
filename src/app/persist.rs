//! High-score and settings persistence.
//!
//! Files live in the config directory (first that applies):
//! `$PACMANTUI_CONFIG_DIR`, `$XDG_CONFIG_HOME/pacmantui`, `~/.config/pacmantui`.
//! Corrupt or missing files are tolerated: loading falls back to defaults and
//! the next save rewrites them. Scores are keyed by (map id, difficulty), so
//! custom-map tables never mix with the classic one (acceptance A2).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::Difficulty;

pub const MAX_SCORES_PER_TABLE: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Map id last selected in the menu.
    pub map: String,
    /// "normal" | "hard" (string keeps the file format stable and typo-tolerant).
    pub difficulty: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            map: "classic".into(),
            difficulty: "normal".into(),
        }
    }
}

impl Settings {
    pub fn difficulty(&self) -> Difficulty {
        match self.difficulty.as_str() {
            "hard" => Difficulty::Hard,
            _ => Difficulty::Normal,
        }
    }
    pub fn set_difficulty(&mut self, d: Difficulty) {
        self.difficulty = difficulty_str(d).into();
    }
}

pub fn difficulty_str(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Normal => "normal",
        Difficulty::Hard => "hard",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub map: String,
    pub difficulty: String,
    pub score: u32,
    pub level: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ScoreFile {
    #[serde(default)]
    entry: Vec<ScoreEntry>,
}

/// Loaded persistence state plus the directory it saves back to.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    pub settings: Settings,
    scores: Vec<ScoreEntry>,
}

impl Store {
    /// Default config directory (see module docs).
    pub fn default_dir() -> PathBuf {
        if let Ok(d) = std::env::var("PACMANTUI_CONFIG_DIR") {
            return PathBuf::from(d);
        }
        if let Ok(d) = std::env::var("XDG_CONFIG_HOME") {
            return Path::new(&d).join("pacmantui");
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Path::new(&home).join(".config").join("pacmantui")
    }

    /// Load from `dir`; any missing/corrupt file yields defaults.
    pub fn load(dir: PathBuf) -> Store {
        let settings = fs::read_to_string(dir.join("settings.toml"))
            .ok()
            .and_then(|s| toml::from_str::<Settings>(&s).ok())
            .unwrap_or_default();
        let scores = fs::read_to_string(dir.join("scores.toml"))
            .ok()
            .and_then(|s| toml::from_str::<ScoreFile>(&s).ok())
            .unwrap_or_default()
            .entry;
        Store {
            dir,
            settings,
            scores,
        }
    }

    /// Best-effort save of both files; errors are returned for logging but the
    /// app treats persistence as non-fatal.
    pub fn save(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        write_atomic(
            &self.dir.join("settings.toml"),
            &toml::to_string(&self.settings).unwrap_or_default(),
        )?;
        let file = ScoreFile {
            entry: self.scores.clone(),
        };
        write_atomic(
            &self.dir.join("scores.toml"),
            &toml::to_string(&file).unwrap_or_default(),
        )
    }

    /// Record a finished game. Returns true if it is a new table best.
    /// Keeps the top [`MAX_SCORES_PER_TABLE`] per (map, difficulty).
    pub fn record_score(
        &mut self,
        map: &str,
        difficulty: Difficulty,
        score: u32,
        level: u32,
    ) -> bool {
        if score == 0 {
            return false;
        }
        let is_high = score > self.high_score(map, difficulty);
        self.scores.push(ScoreEntry {
            map: map.into(),
            difficulty: difficulty_str(difficulty).into(),
            score,
            level,
        });
        // Sort the affected table desc by score and trim its overflow.
        let key = |e: &ScoreEntry| (e.map.clone(), e.difficulty.clone());
        let this_key = (map.to_string(), difficulty_str(difficulty).to_string());
        let mut table: Vec<ScoreEntry> = self
            .scores
            .iter()
            .filter(|e| key(e) == this_key)
            .cloned()
            .collect();
        table.sort_by(|a, b| b.score.cmp(&a.score));
        table.truncate(MAX_SCORES_PER_TABLE);
        self.scores.retain(|e| key(e) != this_key);
        self.scores.extend(table);
        is_high
    }

    /// Highest recorded score for a table (0 when empty).
    pub fn high_score(&self, map: &str, difficulty: Difficulty) -> u32 {
        self.top_scores(map, difficulty)
            .first()
            .map(|e| e.score)
            .unwrap_or(0)
    }

    /// Table entries, best first.
    pub fn top_scores(&self, map: &str, difficulty: Difficulty) -> Vec<ScoreEntry> {
        let d = difficulty_str(difficulty);
        let mut v: Vec<ScoreEntry> = self
            .scores
            .iter()
            .filter(|e| e.map == map && e.difficulty == d)
            .cloned()
            .collect();
        v.sort_by(|a, b| b.score.cmp(&a.score));
        v
    }
}

fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}
