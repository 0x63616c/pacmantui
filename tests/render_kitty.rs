//! Kitty escape assembly: chunk boundaries, o=z header, id alternation,
//! sync brackets — asserted on the produced byte strings via a Vec<u8> sink.

use std::io::Read as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use pacmantui::render::test_api::*;

/// One parsed APC graphics command: (control data, payload).
struct Apc {
    control: String,
    payload: Vec<u8>,
}

fn parse_apcs(buf: &[u8]) -> Vec<Apc> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < buf.len() {
        if &buf[i..i + 3] == b"\x1b_G" {
            let start = i + 3;
            let mut j = start;
            while j + 1 < buf.len() && &buf[j..j + 2] != b"\x1b\\" {
                j += 1;
            }
            let body = &buf[start..j];
            let (control, payload) = match body.iter().position(|&b| b == b';') {
                Some(p) => (&body[..p], body[p + 1..].to_vec()),
                None => (body, Vec::new()),
            };
            out.push(Apc {
                control: String::from_utf8(control.to_vec()).unwrap(),
                payload,
            });
            i = j + 2;
        } else {
            i += 1;
        }
    }
    out
}

fn has_key(control: &str, kv: &str) -> bool {
    control.split(',').any(|part| part == kv)
}

fn params(w: usize, h: usize, compress: bool) -> FrameParams {
    FrameParams {
        width: w,
        height: h,
        row: 5,
        col: 7,
        new_id: 42,
        old_id: 43,
        compress,
    }
}

/// Patterned RGB buffer (compresses but is not trivial).
fn rgb_buf(w: usize, h: usize) -> Vec<u8> {
    (0..w * h * 3).map(|i| (i % 251) as u8).collect()
}

#[test]
fn sync_brackets_and_cursor_anchor() {
    let rgb = rgb_buf(8, 8);
    let mut out = Vec::new();
    write_frame(&mut out, &rgb, &params(8, 8, false));
    assert!(
        out.starts_with(b"\x1b[?2026h"),
        "must open a synchronized update"
    );
    assert!(
        out.ends_with(b"\x1b[?2026l"),
        "must close the synchronized update"
    );
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("\x1b[5;7H"),
        "cursor must move to the anchor cell"
    );
}

#[test]
fn transmit_header_keys_uncompressed() {
    let rgb = rgb_buf(8, 8);
    let mut out = Vec::new();
    write_frame(&mut out, &rgb, &params(8, 8, false));
    let apcs = parse_apcs(&out);
    let head = &apcs[0].control;
    for kv in ["a=t", "f=24", "s=8", "v=8", "i=42", "q=2", "m=0"] {
        assert!(has_key(head, kv), "missing {kv} in {head}");
    }
    assert!(!head.contains("o=z"), "no o=z when compression is off");
}

#[test]
fn transmit_header_has_oz_when_compressed() {
    let rgb = rgb_buf(8, 8);
    let mut out = Vec::new();
    write_frame(&mut out, &rgb, &params(8, 8, true));
    let apcs = parse_apcs(&out);
    assert!(
        has_key(&apcs[0].control, "o=z"),
        "compressed frame must carry o=z"
    );
}

#[test]
fn chunking_4096_multiple_of_4_and_m_flags() {
    // 70x40 -> 8400 raw bytes -> 11200 base64 chars -> 3 chunks.
    let (w, h) = (70, 40);
    let rgb = rgb_buf(w, h);
    let mut out = Vec::new();
    write_frame(&mut out, &rgb, &params(w, h, false));
    let apcs = parse_apcs(&out);
    let transmit: Vec<&Apc> = apcs
        .iter()
        .filter(|a| a.control.contains("a=t") || a.control.starts_with("m="))
        .collect();
    assert_eq!(transmit.len(), 3, "expected exactly 3 chunks");
    // First chunk: full control data + m=1.
    assert!(has_key(&transmit[0].control, "a=t"));
    assert!(has_key(&transmit[0].control, "m=1"));
    // Middle chunk: only m=1.
    assert_eq!(transmit[1].control, "m=1");
    // Final chunk: m=0.
    assert_eq!(transmit[2].control, "m=0");
    // Sizes: non-final chunks exactly 4096 (multiple of 4), final <= 4096.
    assert_eq!(transmit[0].payload.len(), CHUNK);
    assert_eq!(transmit[1].payload.len(), CHUNK);
    assert_eq!(transmit[2].payload.len(), 11200 - 2 * CHUNK);
    for c in &transmit[..2] {
        assert_eq!(c.payload.len() % 4, 0);
    }
    // Reassembled base64 decodes to the original pixels.
    let all: Vec<u8> = transmit.iter().flat_map(|c| c.payload.clone()).collect();
    let decoded = B64.decode(&all).expect("valid base64");
    assert_eq!(decoded, rgb);
}

#[test]
fn compressed_payload_roundtrips_through_zlib() {
    let (w, h) = (64, 64);
    let rgb = rgb_buf(w, h);
    let mut out = Vec::new();
    write_frame(&mut out, &rgb, &params(w, h, true));
    let apcs = parse_apcs(&out);
    let all: Vec<u8> = apcs
        .iter()
        .filter(|a| a.control.contains("a=t") || a.control.starts_with("m="))
        .flat_map(|c| c.payload.clone())
        .collect();
    let compressed = B64.decode(&all).expect("valid base64");
    let mut decompressed = Vec::new();
    flate2::read::ZlibDecoder::new(&compressed[..])
        .read_to_end(&mut decompressed)
        .expect("valid zlib stream");
    assert_eq!(decompressed, rgb);
}

#[test]
fn placement_and_delete_commands() {
    let rgb = rgb_buf(8, 8);
    let mut out = Vec::new();
    write_frame(&mut out, &rgb, &params(8, 8, true));
    let apcs = parse_apcs(&out);
    let place = apcs
        .iter()
        .find(|a| a.control.contains("a=p"))
        .expect("placement command present");
    for kv in ["a=p", "i=42", "p=1", "z=-1", "C=1", "q=2"] {
        assert!(
            has_key(&place.control, kv),
            "missing {kv} in {}",
            place.control
        );
    }
    let del = apcs
        .iter()
        .find(|a| a.control.contains("a=d"))
        .expect("delete command present");
    for kv in ["a=d", "d=i", "i=43", "q=2"] {
        assert!(has_key(&del.control, kv), "missing {kv} in {}", del.control);
    }
    // Order: transmit chunks, then placement, then delete.
    let p_idx = apcs.iter().position(|a| a.control.contains("a=p")).unwrap();
    let d_idx = apcs.iter().position(|a| a.control.contains("a=d")).unwrap();
    assert!(p_idx < d_idx, "place new before deleting old");
    assert!(apcs[0].control.contains("a=t"));
}

#[test]
fn id_alternation_across_frames() {
    let rgb = rgb_buf(8, 8);
    let mut out = Vec::new();

    write_frame(&mut out, &rgb, &params(8, 8, true)); // new=42, old=43
    let apcs = parse_apcs(&out);
    assert!(has_key(&apcs[0].control, "i=42"));
    assert!(
        apcs.iter()
            .any(|a| a.control.contains("a=d") && has_key(&a.control, "i=43"))
    );

    let mut p2 = params(8, 8, true); // swapped, as the renderer does each frame
    p2.new_id = 43;
    p2.old_id = 42;
    write_frame(&mut out, &rgb, &p2);
    let apcs = parse_apcs(&out);
    assert!(has_key(&apcs[0].control, "i=43"));
    assert!(
        apcs.iter()
            .any(|a| a.control.contains("a=d") && has_key(&a.control, "i=42"))
    );
}

#[test]
fn sink_is_cleared_between_frames() {
    let rgb = rgb_buf(8, 8);
    let mut out = vec![1u8, 2, 3]; // stale content must not leak
    write_frame(&mut out, &rgb, &params(8, 8, false));
    assert!(out.starts_with(b"\x1b[?2026h"));
}
