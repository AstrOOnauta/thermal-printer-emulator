use super::*;
use crate::escpos::parser::Parser;

fn print(paper: Paper, bytes: &[u8]) -> Vec<Output> {
    let mut parser = Parser::default();
    let mut printer = Printer::new(paper);
    let mut out = Vec::new();
    parser.feed(bytes, |command, _| printer.apply(command, &mut out));
    printer.finish(&mut out);
    out
}

fn lines(bytes: &[u8]) -> Vec<Vec<Segment>> {
    print(Paper::Mm80, bytes)
        .into_iter()
        .filter_map(|output| match output {
            Output::Block(Block::Line { segments, .. }) => Some(segments),
            _ => None,
        })
        .collect()
}

fn texts(segments: &[Segment]) -> Vec<(u16, &str)> {
    segments
        .iter()
        .map(|segment| (segment.x, segment.text.as_str()))
        .collect()
}

#[test]
fn prints_a_line_in_font_a() {
    let out = print(Paper::Mm80, b"\x1b@Hello\n");
    let Output::Block(Block::Line {
        height,
        ascent,
        segments,
        ..
    }) = &out[0]
    else {
        panic!("{out:?}");
    };
    assert_eq!((*height, *ascent), (30, 24));
    assert_eq!(texts(segments), vec![(0, "Hello")]);
    assert_eq!(segments[0].advance, 12);
    assert_eq!(segments[0].font, Font::A);
}

#[test]
fn aligns_center_and_right_within_the_paper() {
    assert_eq!(texts(&lines(b"\x1ba\x01Hi\n")[0]), vec![(276, "Hi")]);
    assert_eq!(texts(&lines(b"\x1ba\x02Hi\n")[0]), vec![(552, "Hi")]);
    let out = print(Paper::Mm58, b"\x1ba\x01Hi\n");
    let Output::Block(Block::Line { segments, .. }) = &out[0] else {
        panic!("{out:?}");
    };
    assert_eq!(segments[0].x, 180);
}

#[test]
fn alignment_applies_from_the_next_line() {
    let all = lines(b"Left\x1ba\x01\nMid\n");
    assert_eq!(all[0][0].x, 0);
    assert_eq!(all[1][0].x, (576 - 36) / 2);
}

#[test]
fn a_style_change_mid_line_stays_on_one_line() {
    let all = lines(b"Total: \x1bE\x011,50\x1bE\x00!\n");
    assert_eq!(all.len(), 1);
    assert_eq!(
        texts(&all[0]),
        vec![(0, "Total: "), (84, "1,50"), (132, "!")]
    );
    assert!(all[0][1].bold && !all[0][0].bold && !all[0][2].bold);
}

#[test]
fn double_size_doubles_cells_and_line_height() {
    let out = print(Paper::Mm80, b"\x1b!\x30AB\n");
    let Output::Block(Block::Line {
        height,
        ascent,
        segments,
        ..
    }) = &out[0]
    else {
        panic!("{out:?}");
    };
    assert_eq!((*height, *ascent), (48, 48));
    assert_eq!(
        (segments[0].width, segments[0].height, segments[0].advance),
        (2, 2, 24)
    );
    let gs = lines(b"\x1d!\x21A\n");
    assert_eq!((gs[0][0].width, gs[0][0].height), (3, 2));
}

#[test]
fn wraps_at_48_columns_in_font_a_and_64_in_font_b() {
    let line_a = [b'x'; 49];
    let all = lines(&[&line_a[..], b"\n"].concat());
    assert_eq!(all.len(), 2);
    assert_eq!(all[0][0].text.len(), 48);
    assert_eq!(texts(&all[1]), vec![(0, "x")]);

    let line_b = [b'y'; 64];
    let all = lines(&[&b"\x1bM\x01"[..], &line_b, b"\n"].concat());
    assert_eq!(all.len(), 1);
    assert_eq!(all[0][0].advance, 9);
}

#[test]
fn empty_lines_and_feeds_become_one_feed_block() {
    let out = print(Paper::Mm80, b"A\n\n\n\x1bJ\x0a\x1bd\x02B\n");
    assert_eq!(out.len(), 3, "{out:?}");
    assert_eq!(
        out[1],
        Output::Block(Block::Feed {
            height: 30 + 30 + 10 + 60
        })
    );
    assert!(matches!(out[2], Output::Block(Block::Line { .. })));
}

#[test]
fn feed_lines_after_text_counts_the_printed_line() {
    let out = print(Paper::Mm80, b"A\x1bd\x03");
    assert!(matches!(
        out[0],
        Output::Block(Block::Line { height: 30, .. })
    ));
    assert_eq!(out[1], Output::Block(Block::Feed { height: 60 }));
}

#[test]
fn decodes_text_in_the_selected_code_page() {
    assert_eq!(lines(b"\x1bt\x02Caf\x82 \x87\n")[0][0].text, "Café ç");
    // 0xC6 is "ã" in CP850 but "╞" in CP437.
    assert_eq!(
        lines(b"\x1bt\x02\x1b@\xc6\n")[0][0].text,
        "╞",
        "ESC @ restores CP437"
    );
}

#[test]
fn initialize_resets_style_and_drops_the_buffer() {
    let all = lines(b"\x1b!\x38lost\x1b@kept\n");
    assert_eq!(all.len(), 1);
    assert_eq!(texts(&all[0]), vec![(0, "kept")]);
    assert_eq!((all[0][0].width, all[0][0].bold), (1, false));
}

#[test]
fn margins_tabs_and_positions() {
    assert_eq!(lines(b"\x1dL\x20\x00A\n")[0][0].x, 32);
    assert_eq!(texts(&lines(b"A\tB\n")[0]), vec![(0, "A"), (96, "B")]);
    assert_eq!(
        texts(&lines(b"\x1bD\x04\x00A\tB\n")[0]),
        vec![(0, "A"), (48, "B")]
    );
    assert_eq!(texts(&lines(b"\x1b$\x64\x00A\n")[0]), vec![(100, "A")]);
    assert_eq!(
        texts(&lines(b"A\x1b\\\x0a\x00B\n")[0]),
        vec![(0, "A"), (22, "B")]
    );
    // Centered inside a 288-dot area that starts at 32.
    assert_eq!(
        lines(b"\x1dL\x20\x00\x1dW\x20\x01\x1ba\x01AB\n")[0][0].x,
        32 + 132
    );
}

#[test]
fn character_spacing_widens_the_advance() {
    let all = lines(b"\x1b \x02\x1b!\x20AB\n");
    assert_eq!(all[0][0].advance, (12 + 2) * 2);
}

#[test]
fn cut_ends_the_line_and_feeds_first() {
    let out = print(Paper::Mm80, b"A\x1dVB\x10");
    assert!(matches!(out[0], Output::Block(Block::Line { .. })));
    assert_eq!(out[1], Output::Block(Block::Feed { height: 16 }));
    assert_eq!(out[2], Output::Cut { partial: true });
    assert_eq!(
        print(Paper::Mm80, b"\x1dV\x00"),
        vec![Output::Cut { partial: false }]
    );
}

#[test]
fn side_effects_and_unknown_commands() {
    assert_eq!(
        print(Paper::Mm80, b"\x1bp\x00\x19\xfa\x1bB\x01\x01"),
        vec![Output::DrawerPulse, Output::Beep]
    );
    let mut printer = Printer::new(Paper::Mm80);
    let mut out = Vec::new();
    printer.apply(Command::Unknown([0x1b, 0x01]), &mut out);
    assert_eq!(printer.unknown_commands, 1);
}

#[test]
fn text_without_a_final_line_feed_still_prints() {
    assert_eq!(
        texts(&lines(b"\x1b@no newline")[0]),
        vec![(0, "no newline")]
    );
}

#[test]
fn random_bytes_never_panic() {
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..2_000 {
        let len = (next() % 512) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| match next() % 4 {
                0 => [0x1b, 0x1d, 0x0a, 0x09][(next() % 4) as usize],
                _ => next() as u8,
            })
            .collect();
        for paper in [Paper::Mm80, Paper::Mm58] {
            for output in print(paper, &bytes) {
                if let Output::Block(Block::Line { segments, .. }) = output {
                    for segment in &segments {
                        // Text stays on the paper. A lone character at the start of
                        // the area may not (a margin that leaves no room for it):
                        // wrapping could not make it fit.
                        let count = segment.text.chars().count() as u32;
                        let glyph = u32::from(segment.font.cell().0) * u32::from(segment.width);
                        let right =
                            u32::from(segment.x) + u32::from(segment.advance) * (count - 1) + glyph;
                        assert!(
                            right <= u32::from(paper.dots()) || count == 1,
                            "{segment:?} ends at {right} on {paper:?}"
                        );
                    }
                }
            }
        }
    }
}

fn images(bytes: &[u8]) -> Vec<Placed> {
    print(Paper::Mm80, bytes)
        .into_iter()
        .filter_map(|output| match output {
            Output::Block(Block::Image(placed)) => Some(placed),
            _ => None,
        })
        .collect()
}

#[test]
fn raster_images_are_aligned_scaled_and_cropped() {
    let centered = images(b"\x1ba\x01\x1dv0\x00\x01\x00\x02\x00\xff\x81");
    assert_eq!(
        (
            centered[0].x,
            centered[0].bitmap.width,
            centered[0].bitmap.height
        ),
        (284, 8, 2)
    );
    let quadruple = images(b"\x1dv0\x03\x01\x00\x01\x00\x80");
    assert_eq!(
        (quadruple[0].bitmap.width, quadruple[0].bitmap.height),
        (16, 2)
    );
    assert!(quadruple[0].bitmap.get(1, 1) && !quadruple[0].bitmap.get(2, 0));
    let mut wide = b"\x1dv0\x00\x50\x00\x01\x00".to_vec();
    wide.extend([0xff; 80]);
    assert_eq!(images(&wide)[0].bitmap.width, 576, "cropped to the paper");
}

#[test]
fn raster_after_text_prints_the_text_first() {
    let out = print(Paper::Mm80, b"Logo:\x1dv0\x00\x01\x00\x01\x00\xff");
    assert!(matches!(out[0], Output::Block(Block::Line { .. })));
    assert!(matches!(out[1], Output::Block(Block::Image(_))));
}

#[test]
fn bit_image_stripes_stack_without_gaps() {
    // ESC 3 24, then two 24-dot double-density stripes of 2 columns, each ended by LF.
    let stripe = b"\x1b*\x21\x02\x00\xff\x00\x00\x00\x00\x01\n";
    let out = print(Paper::Mm80, &[&b"\x1b3\x18"[..], stripe, stripe].concat());
    assert_eq!(out.len(), 2, "{out:?}");
    for block in &out {
        let Output::Block(Block::Line {
            height,
            images,
            segments,
            ..
        }) = block
        else {
            panic!("{block:?}");
        };
        assert_eq!(*height, 24);
        assert!(segments.is_empty());
        let bitmap = &images[0].bitmap;
        assert_eq!((bitmap.width, bitmap.height), (2, 24));
        assert!(bitmap.get(0, 0) && bitmap.get(0, 7) && !bitmap.get(0, 8));
        assert!(bitmap.get(1, 23) && !bitmap.get(1, 0));
    }
}

#[test]
fn eight_dot_single_density_is_stretched() {
    let out = print(Paper::Mm80, b"\x1b*\x00\x01\x00\x80\n");
    let Output::Block(Block::Line { images, height, .. }) = &out[0] else {
        panic!("{out:?}");
    };
    let bitmap = &images[0].bitmap;
    assert_eq!((bitmap.width, bitmap.height), (2, 24));
    assert!(bitmap.get(1, 2) && !bitmap.get(0, 3));
    assert_eq!(*height, 30, "the line spacing is taller than the stripe");
}

#[test]
fn stored_graphics_print_on_request() {
    let store = b"\x1d(L\x0b\x000p0\x02\x01\x31\x08\x00\x01\x00\xf0";
    assert!(images(store).is_empty(), "storing prints nothing");
    let printed = images(&[&store[..], b"\x1ba\x02\x1d(L\x02\x0002"].concat());
    assert_eq!((printed[0].x, printed[0].bitmap.width), (576 - 16, 16));
    assert!(images(b"\x1d(L\x02\x0002").is_empty(), "nothing stored");
}

#[test]
fn barcodes_are_centered_with_hri_below() {
    let out = print(Paper::Mm80, b"\x1ba\x01\x1dH\x02\x1dk\x02590123412345\x00");
    let Output::Block(Block::Image(bars)) = &out[0] else {
        panic!("{out:?}");
    };
    // EAN-13 is 95 modules of 3 dots, 162 dots tall by default.
    assert_eq!(
        (bars.x, bars.bitmap.width, bars.bitmap.height),
        (145, 285, 162)
    );
    assert!(bars.bitmap.get(0, 0) && !bars.bitmap.get(3, 0) && bars.bitmap.get(6, 161));
    let Output::Block(Block::Line { segments, .. }) = &out[1] else {
        panic!("{out:?}");
    };
    assert_eq!(
        texts(segments),
        vec![(145 + (285 - 13 * 12) / 2, "5901234123457")]
    );
}

#[test]
fn barcode_settings_and_hri_above() {
    let out = print(
        Paper::Mm80,
        b"\x1dh\x28\x1dw\x02\x1dH\x01\x1df\x01\x1dkI\x06{BAb12",
    );
    assert!(
        matches!(&out[0], Output::Block(Block::Line { segments, .. })
        if segments[0].text == "Ab12" && segments[0].font == Font::B)
    );
    let Output::Block(Block::Image(bars)) = &out[1] else {
        panic!("{out:?}");
    };
    // CODE128: start + 4 characters + checksum (11 modules each) + stop (13), 2 dots.
    assert_eq!(
        (bars.bitmap.width, bars.bitmap.height),
        ((6 * 11 + 13) * 2, 40)
    );
}

#[test]
fn barcodes_wider_than_the_paper_or_invalid_are_skipped() {
    let long = [&b"\x1dw\x06\x1dk\x49\x20{B"[..], &[b'X'; 30]].concat();
    assert!(images(&long).is_empty());
    assert!(
        images(b"\x1dk\x02ABC\x00").is_empty(),
        "EAN-13 takes digits"
    );
}

#[test]
fn a_qr_code_wider_than_the_paper_is_not_printed() {
    let mut bytes = b"\x1d(k\x03\x001C\x10".to_vec();
    let data = [b'A'; 300];
    let length = (data.len() + 3) as u16;
    bytes.extend(b"\x1d(k");
    bytes.extend(length.to_le_bytes());
    bytes.extend(b"1P0");
    bytes.extend(data);
    bytes.extend(b"\x1d(k\x03\x001Q0");
    assert!(images(&bytes).is_empty(), "53 modules × 16 dots > 576");
}

#[test]
fn a_character_past_the_area_wraps_even_on_an_empty_line() {
    // `ESC $` to dot 570: "A" (12 dots) would end past 576.
    let all = lines(b"\x1b$\x3a\x02ABC\n");
    assert_eq!(texts(all.last().expect("a line")), vec![(0, "ABC")]);
}

#[test]
fn a_tab_stop_past_the_area_goes_to_its_end() {
    // 58 mm: 32 columns of font A. Stop at column 40; after it, "X" wraps.
    let mut bytes = b"\x1bD\x28\x00".to_vec();
    bytes.extend([b'a'; 30]);
    bytes.extend(b"\tX\n");
    let segments: Vec<Vec<Segment>> = print(Paper::Mm58, &bytes)
        .into_iter()
        .filter_map(|output| match output {
            Output::Block(Block::Line { segments, .. }) => Some(segments),
            _ => None,
        })
        .collect();
    assert_eq!(texts(&segments[1]), vec![(0, "X")]);
}

#[test]
fn qr_codes_use_the_stored_data_and_module_size() {
    let qr = b"\x1ba\x01\x1d(k\x03\x00\x31\x43\x04\x1d(k\x05\x00\x31\x50\x30hi\x1d(k\x03\x00\x31\x51\x30";
    let printed = images(qr);
    // "hi" fits version 1: 21 modules of 4 dots.
    assert_eq!(
        (
            printed[0].x,
            printed[0].bitmap.width,
            printed[0].bitmap.height
        ),
        (246, 84, 84)
    );
    // Top-left finder pattern: dark corner, light ring at module 1.
    assert!(printed[0].bitmap.get(0, 0) && printed[0].bitmap.get(3, 3));
    assert!(!printed[0].bitmap.get(4, 4));
    assert!(
        images(b"\x1d(k\x03\x00\x31\x51\x30").is_empty(),
        "nothing stored"
    );
}

#[test]
fn answers_status_requests_like_an_idle_printer() {
    let replies = |bytes: &[u8]| -> Vec<Vec<u8>> {
        print(Paper::Mm80, bytes)
            .into_iter()
            .filter_map(|output| match output {
                Output::Reply(reply) => Some(reply),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        replies(b"\x10\x04\x01\x10\x04\x02\x10\x04\x03\x10\x04\x04"),
        vec![vec![0x12]; 4]
    );
    assert_eq!(replies(b"\x1dr\x01\x1dr2"), vec![vec![0x00]; 2]);
    assert_eq!(replies(b"\x1dI\x02"), vec![vec![0x02]]);
    assert_eq!(
        replies(b"\x1dIB"),
        vec![b"_Thermal Printer Emulator\0".to_vec()]
    );
    assert!(
        replies(b"\x10\x04\x05\x1dr\x09\x1dI\x7f").is_empty(),
        "unknown requests"
    );
}

#[test]
fn initialize_restores_the_configured_code_page() {
    let mut printer = Printer::with_code_page(Paper::Mm80, CodePage::Cp850);
    let mut out = Vec::new();
    let mut parser = Parser::default();
    // 0xC6 is "ã" in CP850; after ESC t 0 and ESC @ it must be CP850 again.
    parser.feed(b"\xc6\x1bt\x00\x1b@\xc6\n", |command, _| {
        printer.apply(command, &mut out)
    });
    let Output::Block(Block::Line { segments, .. }) = &out[0] else {
        panic!("{out:?}");
    };
    assert_eq!(segments[0].text, "ã");
}

#[test]
fn out_of_range_settings_keep_the_previous_value() {
    let mut printer = Printer::new(Paper::Mm80);
    let mut out = Vec::new();
    for command in [
        Command::QrErrorCorrection(51),
        Command::QrErrorCorrection(7),
        Command::QrModuleSize(6),
        Command::QrModuleSize(17),
        Command::BarcodeWidth(3),
        Command::BarcodeWidth(9),
        Command::BarcodeHeight(80),
        Command::BarcodeHeight(0),
    ] {
        printer.apply(command, &mut out);
    }
    assert_eq!((printer.qr.level, printer.qr.module), (51, 6), "GS ( k");
    assert_eq!(
        (printer.barcode.module, printer.barcode.height),
        (3, 80),
        "GS w, GS h"
    );
}
