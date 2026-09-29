//! Input recording/replay for deterministic tests and scenario tooling.
//!
//! Owner: W2-REPLAY agent (skeleton may be exercised earlier by sim tests).

use crate::types::{Difficulty, InputFrame};

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

    /// Serialize to the documented text format.
    pub fn to_text(&self) -> String {
        todo!("W2-REPLAY")
    }

    pub fn from_text(src: &str) -> Result<Replay, String> {
        let _ = src;
        todo!("W2-REPLAY")
    }
}
