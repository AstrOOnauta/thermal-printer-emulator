use super::*;

fn commands(bytes: &[u8]) -> Vec<Command> {
    let mut parser = Parser::default();
    let mut out = Vec::new();
    parser.feed(bytes, |command, _| out.push(command));
    assert_eq!(parser.pending_len(), 0, "left incomplete: {bytes:02x?}");
    out
}

fn one(bytes: &[u8]) -> Command {
    let mut all = commands(bytes);
    assert_eq!(all.len(), 1, "{bytes:02x?} → {all:?}");
    all.remove(0)
}

#[test]
fn text_runs_stop_at_control_bytes() {
    assert_eq!(
        commands(b"Hello\nWorld"),
        vec![
            Command::Text(b"Hello".to_vec()),
            Command::LineFeed,
            Command::Text(b"World".to_vec()),
        ]
    );
    assert_eq!(one(b"\x80\xff"), Command::Text(vec![0x80, 0xff]));
}

#[test]
fn esc_commands() {
    assert_eq!(one(b"\x1b@"), Command::Initialize);
    assert_eq!(one(b"\x1b!\x38"), Command::PrintMode(0x38));
    assert_eq!(one(b"\x1bE\x01"), Command::Emphasis(true));
    assert_eq!(one(b"\x1bG\x00"), Command::Emphasis(false));
    assert_eq!(one(b"\x1b-\x02"), Command::Underline(2));
    assert_eq!(one(b"\x1b-1"), Command::Underline(1));
    assert_eq!(one(b"\x1ba\x01"), Command::Align(Alignment::Center));
    assert_eq!(one(b"\x1ba2"), Command::Align(Alignment::Right));
    assert_eq!(one(b"\x1bM1"), Command::Font(1));
    assert_eq!(one(b"\x1bt\x02"), Command::CodeTable(2));
    assert_eq!(one(b"\x1b \x04"), Command::CharSpacing(4));
    assert_eq!(one(b"\x1b2"), Command::LineSpacing(None));
    assert_eq!(one(b"\x1b3\x40"), Command::LineSpacing(Some(0x40)));
    assert_eq!(one(b"\x1bJ\x10"), Command::FeedDots(0x10));
    assert_eq!(one(b"\x1bd\x03"), Command::FeedLines(3));
    assert_eq!(one(b"\x1b$\x00\x01"), Command::AbsolutePosition(256));
    assert_eq!(one(b"\x1b\\\xf6\xff"), Command::RelativePosition(-10));
    assert_eq!(one(b"\x1bD\x08\x10\x00"), Command::TabStops(vec![8, 16]));
    assert_eq!(one(b"\x1bp\x00\x19\xfa"), Command::DrawerPulse);
    assert_eq!(one(b"\x1bB\x02\x01"), Command::Beep);
    assert_eq!(
        one(b"\x1bm"),
        Command::Cut {
            partial: true,
            feed: 0
        }
    );
}

#[test]
fn gs_commands() {
    assert_eq!(
        one(b"\x1d!\x11"),
        Command::CharSize {
            width: 2,
            height: 2
        }
    );
    assert_eq!(
        one(b"\x1d!\x70"),
        Command::CharSize {
            width: 8,
            height: 1
        }
    );
    assert_eq!(one(b"\x1dB\x01"), Command::Reverse(true));
    assert_eq!(one(b"\x1dL\x10\x00"), Command::LeftMargin(16));
    assert_eq!(one(b"\x1dW\x80\x01"), Command::PrintAreaWidth(384));
    assert_eq!(
        one(b"\x1dV\x00"),
        Command::Cut {
            partial: false,
            feed: 0
        }
    );
    assert_eq!(
        one(b"\x1dV1"),
        Command::Cut {
            partial: true,
            feed: 0
        }
    );
    assert_eq!(
        one(b"\x1dVB\x05"),
        Command::Cut {
            partial: true,
            feed: 5
        }
    );
    assert_eq!(one(b"\x1dr\x01"), Command::Status(StatusRequest::Status(1)));
    assert_eq!(
        one(b"\x1dI\x42"),
        Command::Status(StatusRequest::PrinterId(0x42))
    );
}

#[test]
fn dle_real_time_commands() {
    assert_eq!(
        one(b"\x10\x04\x01"),
        Command::Status(StatusRequest::Realtime(1))
    );
    assert_eq!(
        one(b"\x10\x04\x07\x01"),
        Command::Status(StatusRequest::Realtime(7))
    );
    assert_eq!(one(b"\x10\x14\x01\x00\x01"), Command::DrawerPulse);
    assert_eq!(
        one(b"\x10\x14\x08\x01\x03\x14\x01\x06\x02\x08"),
        Command::Ignored
    );
    assert_eq!(
        commands(b"\x10A"),
        vec![Command::Ignored, Command::Text(b"A".to_vec())]
    );
}

#[test]
fn skips_data_carrying_commands_by_their_length() {
    // Each one is followed by text that must survive intact.
    for skipped in [
        &b"\x1d8L\x02\x00\x00\x00\x30\x45"[..], // graphics, unknown function
        b"\x1d*\x01\x01\x01\x02\x03\x04\x05\x06\x07\x08", // downloaded image 1×1
        b"\x1b&\x03\x41\x42\x01\x01\x02\x03\x01\x04\x05\x06", // 2 user chars
        b"\x1cq\x01\x01\x00\x01\x00\x01\x02\x03\x04\x05\x06\x07\x08", // NV image
        b"\x1b(A\x02\x00\x01\x02",              // ESC ( A
    ] {
        let mut stream = skipped.to_vec();
        stream.extend_from_slice(b"ok");
        assert_eq!(
            commands(&stream),
            vec![Command::Ignored, Command::Text(b"ok".to_vec())],
            "{skipped:02x?}"
        );
    }
}

#[test]
fn image_commands_carry_their_data() {
    assert_eq!(
        one(b"\x1dv0\x01\x02\x00\x02\x00\xaa\xbb\xcc\xdd"),
        Command::Raster {
            mode: 1,
            row_bytes: 2,
            height: 2,
            data: vec![0xaa, 0xbb, 0xcc, 0xdd]
        }
    );
    assert_eq!(
        one(b"\x1b*\x00\x03\x00\x01\x02\x03"),
        Command::BitImage {
            mode: 0,
            columns: 3,
            data: vec![1, 2, 3]
        }
    );
    assert_eq!(
        one(b"\x1b*\x21\x02\x00\x01\x02\x03\x04\x05\x06"),
        Command::BitImage {
            mode: 33,
            columns: 2,
            data: vec![1, 2, 3, 4, 5, 6]
        }
    );
    assert_eq!(
        one(b"\x1d(L\x0b\x000p0\x01\x01\x31\x08\x00\x01\x00\xff"),
        Command::StoreGraphics {
            width: 8,
            height: 1,
            scale_x: 1,
            scale_y: 1,
            data: vec![0xff]
        }
    );
    assert_eq!(
        one(b"\x1d8L\x0b\x00\x00\x000p0\x02\x02\x31\x08\x00\x01\x00\x0f"),
        Command::StoreGraphics {
            width: 8,
            height: 1,
            scale_x: 2,
            scale_y: 2,
            data: vec![0x0f]
        }
    );
    assert_eq!(one(b"\x1d(L\x02\x0002"), Command::PrintGraphics);
}

#[test]
fn gs_paren_l_ignores_a_header_bigger_than_its_data() {
    // 65535 × 65535 dots announced, one byte sent.
    assert_eq!(
        one(b"\x1d(L\x0b\x000p0\x01\x01\x31\xff\xff\xff\xff\xaa"),
        Command::Ignored
    );
}

#[test]
fn esc_d_ends_where_the_reference_says() {
    assert_eq!(
        commands(b"\x1bD\x08\x10\x09Total\n"),
        vec![
            Command::TabStops(vec![8, 16]),
            Command::Tab,
            Command::Text(b"Total".to_vec()),
            Command::LineFeed,
        ],
        "a value that does not ascend ends the list and stays data"
    );
    let all: Vec<u8> = (1..=32).collect();
    let mut bytes = b"\x1bD".to_vec();
    bytes.extend(&all);
    bytes.push(0);
    assert_eq!(one(&bytes), Command::TabStops(all.clone()));
    bytes.pop();
    bytes.push(b'A');
    assert_eq!(
        commands(&bytes),
        vec![Command::TabStops(all), Command::Text(b"A".to_vec())],
        "32 values at most"
    );
}

#[test]
fn gs_k_function_a_without_nul_is_bounded() {
    let mut bytes = b"\x1dk\x04".to_vec();
    bytes.extend([b'A'; 300]);
    assert_eq!(
        commands(&bytes),
        vec![Command::Ignored, Command::Text(vec![b'A'; 300])]
    );
}

#[test]
fn barcode_and_qr_commands() {
    assert_eq!(one(b"\x1dh\x50"), Command::BarcodeHeight(80));
    assert_eq!(one(b"\x1dw\x02"), Command::BarcodeWidth(2));
    assert_eq!(one(b"\x1dH2"), Command::HriPosition(2));
    assert_eq!(one(b"\x1df\x01"), Command::HriFont(1));
    assert_eq!(
        one(b"\x1dk\x04AB\x00"),
        Command::Barcode {
            symbology: 4,
            data: b"AB".to_vec()
        }
    );
    assert_eq!(
        one(b"\x1dkI\x03{BA"),
        Command::Barcode {
            symbology: 73,
            data: b"{BA".to_vec()
        }
    );
    assert_eq!(one(b"\x1d(k\x03\x00\x31\x43\x05"), Command::QrModuleSize(5));
    assert_eq!(
        one(b"\x1d(k\x03\x00\x31\x45\x31"),
        Command::QrErrorCorrection(49)
    );
    assert_eq!(
        one(b"\x1d(k\x05\x00\x31\x50\x30hi"),
        Command::QrStore(b"hi".to_vec())
    );
    assert_eq!(one(b"\x1d(k\x03\x00\x31\x51\x30"), Command::QrPrint);
    assert_eq!(
        one(b"\x1d(k\x03\x00\x30\x43\x05"),
        Command::Ignored,
        "PDF417"
    );
}

#[test]
fn unknown_commands_skip_two_bytes() {
    assert_eq!(
        commands(b"\x1b\x01ok"),
        vec![Command::Unknown([ESC, 0x01]), Command::Text(b"ok".to_vec())]
    );
    assert_eq!(one(b"\x1d\x7e"), Command::Unknown([GS, 0x7e]));
}

#[test]
fn waits_for_incomplete_commands() {
    let mut parser = Parser::default();
    let mut out = Vec::new();
    parser.feed(b"A\x1b", |command, _| out.push(command));
    assert_eq!(out, vec![Command::Text(b"A".to_vec())]);
    assert_eq!(parser.pending_len(), 1);
    parser.feed(b"!", |command, _| out.push(command));
    assert_eq!(parser.pending_len(), 2);
    parser.feed(b"\x08", |command, _| out.push(command));
    assert_eq!(out.last(), Some(&Command::PrintMode(8)));
    assert_eq!(parser.pending_len(), 0);
}

#[test]
fn reports_stream_offsets_after_each_command() {
    let mut parser = Parser::default();
    let mut ends = Vec::new();
    parser.feed(b"AB\x1dV", |_, end| ends.push(end));
    parser.feed(b"\x00C", |_, end| ends.push(end));
    assert_eq!(ends, vec![2, 5, 6]);
}

const SAMPLE: &[u8] = b"\x1b@\x1ba\x01\x1b!\x30Store\n\x1b!\x00\x1bt\x02Caf\x82 1,50\n\
    \x1dv0\x00\x01\x00\x02\x00\xf0\x0f\x1dkI\x04{B12\x1d(k\x03\x00\x31\x45\x31\
    \x1bD\x08\x00\tTab\n\x10\x04\x01\x1bd\x03\x1dVA\x10";

#[test]
fn any_split_parses_like_the_whole_stream() {
    let whole = commands(SAMPLE);
    for split in 1..SAMPLE.len() {
        let mut parser = Parser::default();
        let mut out = Vec::new();
        parser.feed(&SAMPLE[..split], |command, end| out.push((command, end)));
        parser.feed(&SAMPLE[split..], |command, end| out.push((command, end)));
        // Text may arrive in two runs; everything else must match exactly.
        let merged = merge_text(out.into_iter().map(|(command, _)| command).collect());
        assert_eq!(merged, merge_text(whole.clone()), "split at {split}");
    }
}

fn merge_text(commands: Vec<Command>) -> Vec<Command> {
    let mut merged: Vec<Command> = Vec::new();
    for command in commands {
        if let (Command::Text(more), Some(Command::Text(previous))) = (&command, merged.last_mut())
        {
            previous.extend_from_slice(more);
            continue;
        }
        merged.push(command);
    }
    merged
}

/// xorshift64: deterministic, no dependency.
fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn random_bytes_never_panic_and_never_stall() {
    let mut state = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..2_000 {
        let len = (random(&mut state) % 512) as usize;
        let bytes: Vec<u8> = (0..len)
            // Bias towards command bytes so the interesting paths run.
            .map(|_| match random(&mut state) % 4 {
                0 => [ESC, GS, FS, DLE][(random(&mut state) % 4) as usize],
                _ => random(&mut state) as u8,
            })
            .collect();
        let mut parser = Parser::default();
        let mut consumed = 0;
        for chunk in bytes.chunks(1 + (random(&mut state) % 16) as usize) {
            parser.feed(chunk, |_, end| consumed = end);
        }
        assert_eq!(consumed as usize + parser.pending_len(), bytes.len());
    }
}
