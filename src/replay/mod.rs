//! Input recording/replay for deterministic tests and scenario tooling.
//!
//! # Text format (version 1)
//!
//! Line-oriented UTF-8. `#` starts a comment line; blank lines are ignored.
//!
//! ```text
//! pacmantui-replay v1
//! map=classic
//! difficulty=normal
//! seed=123456789
//! inputs=..RRRRRRUU
//! LLLL.DDD
//! ```
//!
//! - The first non-comment line must be the magic `pacmantui-replay v1`.
//! - `map` is the map id ([`crate::map::Map::id`]), `difficulty` is
//!   `normal`|`hard`, `seed` is the decimal PRNG seed.
//! - `inputs=` starts the input section: one character per sim tick —
//!   `.` = no direction held, `U`/`L`/`D`/`R` = that direction held.
//!   Every following line continues the input stream until EOF.
//!
//! A replay is honest by construction: it captures only the per-tick
//! [`InputFrame`]s a player could have produced, and replaying feeds them
//! through the ordinary [`crate::sim::Game::tick`] path.

use crate::types::{Difficulty, Dir, InputFrame};

pub const MAGIC: &str = "pacmantui-replay v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    pub map_id: String,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub inputs: Vec<InputFrame>,
}

impl Replay {
    pub fn new(map_id: String, difficulty: Difficulty, seed: u64) -> Replay {
        Replay {
            map_id,
            difficulty,
            seed,
            inputs: Vec::new(),
        }
    }

    pub fn push(&mut self, input: InputFrame) {
        self.inputs.push(input);
    }

    /// Serialize to the documented text format (module docs).
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(MAGIC);
        out.push('\n');
        out.push_str(&format!("map={}\n", self.map_id));
        out.push_str(&format!(
            "difficulty={}\n",
            match self.difficulty {
                Difficulty::Normal => "normal",
                Difficulty::Hard => "hard",
            }
        ));
        out.push_str(&format!("seed={}\n", self.seed));
        out.push_str("inputs=");
        for (i, frame) in self.inputs.iter().enumerate() {
            // Wrap input lines at 60 chars for readable diffs.
            if i > 0 && i % 60 == 0 {
                out.push('\n');
            }
            out.push(match frame.dir {
                None => '.',
                Some(Dir::Up) => 'U',
                Some(Dir::Left) => 'L',
                Some(Dir::Down) => 'D',
                Some(Dir::Right) => 'R',
            });
        }
        out.push('\n');
        out
    }

    pub fn from_text(src: &str) -> Result<Replay, String> {
        let mut lines = src
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        match lines.next() {
            Some(l) if l == MAGIC => {}
            Some(l) => return Err(format!("bad magic line {l:?}; expected {MAGIC:?}")),
            None => return Err("empty replay file".into()),
        }

        let mut map_id: Option<String> = None;
        let mut difficulty: Option<Difficulty> = None;
        let mut seed: Option<u64> = None;
        let mut inputs: Option<Vec<InputFrame>> = None;

        for line in lines {
            if let Some(frames) = &mut inputs {
                parse_input_chars(line, frames)?;
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, got {line:?}"))?;
            match key.trim() {
                "map" => map_id = Some(value.trim().to_string()),
                "difficulty" => {
                    difficulty = Some(match value.trim() {
                        "normal" => Difficulty::Normal,
                        "hard" => Difficulty::Hard,
                        other => return Err(format!("unknown difficulty {other:?}")),
                    })
                }
                "seed" => {
                    seed = Some(
                        value
                            .trim()
                            .parse::<u64>()
                            .map_err(|e| format!("bad seed {value:?}: {e}"))?,
                    )
                }
                "inputs" => {
                    let mut frames = Vec::new();
                    parse_input_chars(value.trim(), &mut frames)?;
                    inputs = Some(frames);
                }
                other => return Err(format!("unknown key {other:?}")),
            }
        }

        Ok(Replay {
            map_id: map_id.ok_or("missing map=")?,
            difficulty: difficulty.ok_or("missing difficulty=")?,
            seed: seed.ok_or("missing seed=")?,
            inputs: inputs.ok_or("missing inputs=")?,
        })
    }
}

fn parse_input_chars(s: &str, out: &mut Vec<InputFrame>) -> Result<(), String> {
    for c in s.chars() {
        let dir = match c {
            '.' => None,
            'U' => Some(Dir::Up),
            'L' => Some(Dir::Left),
            'D' => Some(Dir::Down),
            'R' => Some(Dir::Right),
            other => return Err(format!("bad input char {other:?} (want . U L D R)")),
        };
        out.push(InputFrame { dir });
    }
    Ok(())
}
