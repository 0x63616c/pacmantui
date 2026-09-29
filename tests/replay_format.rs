//! Replay text-format tests (format spec: src/replay/mod.rs module docs).

use pacmantui::replay::{MAGIC, Replay};
use pacmantui::types::{Difficulty, Dir, InputFrame};

fn frame(dir: Option<Dir>) -> InputFrame {
    InputFrame { dir }
}

#[test]
fn round_trips_all_input_kinds() {
    let mut rp = Replay::new("classic".into(), Difficulty::Hard, 987654321);
    for d in [
        None,
        Some(Dir::Up),
        Some(Dir::Left),
        Some(Dir::Down),
        Some(Dir::Right),
        None,
    ] {
        rp.push(frame(d));
    }
    let text = rp.to_text();
    assert!(text.starts_with(MAGIC));
    assert_eq!(Replay::from_text(&text), Ok(rp));
}

#[test]
fn round_trips_long_streams_across_wrapped_lines() {
    let mut rp = Replay::new("vertigo".into(), Difficulty::Normal, 7);
    for i in 0..1000 {
        rp.push(frame(match i % 5 {
            0 => None,
            1 => Some(Dir::Up),
            2 => Some(Dir::Left),
            3 => Some(Dir::Down),
            _ => Some(Dir::Right),
        }));
    }
    let text = rp.to_text();
    // to_text wraps at 60 chars per line; all lines must reassemble.
    assert!(text.lines().count() > 10);
    assert_eq!(Replay::from_text(&text).unwrap().inputs.len(), 1000);
    assert_eq!(Replay::from_text(&text), Ok(rp));
}

#[test]
fn parses_comments_blank_lines_and_empty_inputs() {
    let src = format!(
        "# a comment\n{MAGIC}\n\nmap=classic\n# mid comment\ndifficulty=normal\nseed=42\ninputs=\n"
    );
    let rp = Replay::from_text(&src).unwrap();
    assert_eq!(rp.map_id, "classic");
    assert_eq!(rp.difficulty, Difficulty::Normal);
    assert_eq!(rp.seed, 42);
    assert!(rp.inputs.is_empty());
}

#[test]
fn rejects_bad_magic_missing_fields_and_bad_chars() {
    assert!(Replay::from_text("").is_err());
    assert!(Replay::from_text("not-a-replay\nmap=x").is_err());
    // Missing seed.
    assert!(Replay::from_text(&format!("{MAGIC}\nmap=x\ndifficulty=normal\ninputs=..")).is_err());
    // Missing inputs section.
    assert!(Replay::from_text(&format!("{MAGIC}\nmap=x\ndifficulty=normal\nseed=1")).is_err());
    // Bad difficulty.
    assert!(
        Replay::from_text(&format!(
            "{MAGIC}\nmap=x\ndifficulty=extreme\nseed=1\ninputs=."
        ))
        .is_err()
    );
    // Bad input character.
    assert!(
        Replay::from_text(&format!(
            "{MAGIC}\nmap=x\ndifficulty=hard\nseed=1\ninputs=..X"
        ))
        .is_err()
    );
    // Bad seed.
    assert!(
        Replay::from_text(&format!(
            "{MAGIC}\nmap=x\ndifficulty=hard\nseed=banana\ninputs=."
        ))
        .is_err()
    );
}
