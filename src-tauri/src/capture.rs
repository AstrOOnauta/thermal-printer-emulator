//! One connection's bytes → receipts (decision 1 in `.ai/stack.md`).
//!
//! - A receipt starts with its first visible output (a line, an image, a drawer pulse or a
//!   beep), so a connection that only asks for the status leaves nothing in the list.
//! - It ends at a cut, or when the connection ends.
//! - Its raw bytes are cut exactly after the cut command (the decoder reports where each
//!   command ends), so each receipt keeps the bytes that printed it.

use std::net::SocketAddr;

use tokio::sync::mpsc;

use crate::escpos::codepage::CodePage;
use crate::escpos::model::{Output, Paper};
use crate::escpos::Decoder;
use crate::listener::{lock, Event, Shared};
use crate::receipts::{Cut, ReceiptState, TooLarge};

pub struct Capture<'a> {
    peer: SocketAddr,
    paper: Paper,
    shared: &'a Shared,
    events: &'a mpsc::UnboundedSender<Event>,
    decoder: Decoder,
    /// The receipt being printed.
    receipt: Option<u64>,
    /// Raw bytes since the last receipt ended, not yet part of a receipt.
    pending: Vec<u8>,
    /// Stream offset of the next byte `feed` will get.
    offset: u64,
    outputs: Vec<(Output, u64)>,
    /// Status replies waiting to be written to the socket.
    replies: Vec<u8>,
    /// Receipts this connection produced, for the connection log.
    pub receipts: usize,
}

impl<'a> Capture<'a> {
    pub fn new(
        peer: SocketAddr,
        paper: Paper,
        code_page: CodePage,
        shared: &'a Shared,
        events: &'a mpsc::UnboundedSender<Event>,
    ) -> Self {
        Self {
            peer,
            paper,
            shared,
            events,
            decoder: Decoder::new(paper, code_page),
            receipt: None,
            pending: Vec::new(),
            offset: 0,
            outputs: Vec::new(),
            replies: Vec::new(),
            receipts: 0,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Result<(), TooLarge> {
        let start = self.offset;
        self.offset += chunk.len() as u64;
        let mut outputs = std::mem::take(&mut self.outputs);
        self.decoder.feed(chunk, &mut outputs);

        // Raw bytes up to `cursor` belong to a receipt (or to `pending`) already.
        let mut cursor = start;
        let mut result = Ok(());
        for (output, end) in outputs.drain(..) {
            let from = (cursor - start) as usize;
            let to = (end.max(cursor) - start) as usize;
            if let Err(error) = self.add_raw(&chunk[from..to]) {
                result = Err(error);
                break;
            }
            cursor = end.max(cursor);
            if let Err(error) = self.apply(output) {
                result = Err(error);
                break;
            }
        }
        self.outputs = outputs;
        result?;
        self.add_raw(&chunk[(cursor - start) as usize..])
    }

    /// Status replies produced since the last call, to write to the socket.
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    /// The connection ended: prints what is left and closes the receipt with `state`.
    pub fn finish(mut self, state: ReceiptState) -> usize {
        let mut tail = Vec::new();
        self.decoder.finish(&mut tail);
        for output in tail {
            if self.apply(output).is_err() {
                break;
            }
        }
        if let Some(id) = self.receipt.take() {
            self.end(id, state, None);
        }
        let dropped = self.pending.len() + self.decoder.pending_len();
        let unknown = self.decoder.unknown_commands();
        if dropped > 0 || unknown > 0 {
            log::info!(
                "connection_leftovers peer={} unprinted_bytes={dropped} unknown_commands={unknown}",
                self.peer
            );
        }
        self.receipts
    }

    fn apply(&mut self, output: Output) -> Result<(), TooLarge> {
        match output {
            Output::Block(block) => {
                let id = self.ensure_receipt()?;
                lock(&self.shared.receipts).add_block(id, block)
            }
            Output::DrawerPulse => {
                let id = self.ensure_receipt()?;
                lock(&self.shared.receipts).drawer(id);
                Ok(())
            }
            Output::Beep => {
                let id = self.ensure_receipt()?;
                lock(&self.shared.receipts).beep(id);
                Ok(())
            }
            // A status request is not printed output: it never starts a receipt.
            Output::Reply(reply) => {
                self.replies.extend_from_slice(&reply);
                Ok(())
            }
            Output::Cut { partial } => {
                match self.receipt.take() {
                    Some(id) => {
                        let cut = if partial { Cut::Partial } else { Cut::Full };
                        self.end(id, ReceiptState::Done, Some(cut));
                    }
                    // A cut with nothing printed before it: no receipt, drop its bytes.
                    None => self.pending.clear(),
                }
                Ok(())
            }
        }
    }

    fn add_raw(&mut self, bytes: &[u8]) -> Result<(), TooLarge> {
        match self.receipt {
            Some(id) => lock(&self.shared.receipts).add_raw(id, bytes),
            None => {
                self.pending.extend_from_slice(bytes);
                Ok(())
            }
        }
    }

    fn ensure_receipt(&mut self) -> Result<u64, TooLarge> {
        if let Some(id) = self.receipt {
            return Ok(id);
        }
        let raw = std::mem::take(&mut self.pending);
        let id = {
            let mut receipts = lock(&self.shared.receipts);
            let id = receipts.start(self.peer, self.paper, raw)?;
            let _ = self.events.send(Event::Receipts(receipts.summaries()));
            id
        };
        log::info!("receipt_started id={id} peer={}", self.peer);
        self.receipt = Some(id);
        self.receipts += 1;
        Ok(id)
    }

    fn end(&self, id: u64, state: ReceiptState, cut: Option<Cut>) {
        let summary = {
            let mut receipts = lock(&self.shared.receipts);
            receipts.finish(id, state, cut);
            let summaries = receipts.summaries();
            let summary = summaries.iter().find(|receipt| receipt.id == id).cloned();
            let _ = self.events.send(Event::Receipts(summaries));
            summary
        };
        if let Some(summary) = summary {
            log::info!(
                "receipt_ended id={id} state={state:?} cut={cut:?} bytes={} drawer={} beeps={}",
                summary.size,
                summary.drawer,
                summary.beeps
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipts::Receipts;
    use crate::settings::Settings;

    fn peer() -> SocketAddr {
        "10.0.0.2:4000".parse().expect("valid address")
    }

    /// Feeds `chunks` through one connection and returns (state, size, raw, blocks) per
    /// receipt.
    fn capture(chunks: &[&[u8]]) -> Vec<(ReceiptState, Option<Cut>, Vec<u8>, usize)> {
        let shared = Shared::new(Receipts::default(), Settings::default());
        let (events, _received) = mpsc::unbounded_channel();
        let mut capture = Capture::new(peer(), Paper::Mm80, CodePage::DEFAULT, &shared, &events);
        for chunk in chunks {
            capture.feed(chunk).expect("fits");
        }
        capture.finish(ReceiptState::Done);
        let receipts = lock(&shared.receipts);
        receipts
            .summaries()
            .iter()
            .map(|summary| {
                let view = receipts.view(summary.id).expect("exists");
                let raw = receipts.raw(summary.id).expect("exists").to_vec();
                (summary.state, summary.cut, raw, view.blocks.len())
            })
            .collect()
    }

    #[test]
    fn one_receipt_per_cut_with_its_own_bytes() {
        let stream = b"\x1b@A\n\x1dV\x00\x1b@B\n\x1dVB\x00";
        let receipts = capture(&[stream]);
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].2, b"\x1b@A\n\x1dV\x00");
        assert_eq!(receipts[0].1, Some(Cut::Full));
        assert_eq!(receipts[1].2, b"\x1b@B\n\x1dVB\x00");
        assert_eq!(receipts[1].1, Some(Cut::Partial));
    }

    #[test]
    fn splits_the_same_way_whatever_the_chunks() {
        let stream: &[u8] = b"\x1b@A\n\x1dV\x00\x1b@B\n\x1dV\x01tail\n";
        let whole = capture(&[stream]);
        for split in 1..stream.len() {
            assert_eq!(
                capture(&[&stream[..split], &stream[split..]]),
                whole,
                "split {split}"
            );
        }
        assert_eq!(whole.len(), 3);
        assert_eq!(whole[2].2, b"tail\n");
        assert_eq!(whole[2].1, None, "ended by the connection, not a cut");
    }

    #[test]
    fn status_queries_and_bare_cuts_leave_no_receipt() {
        assert!(capture(&[b"\x10\x04\x01\x10\x04\x04"]).is_empty());
        assert!(capture(&[b"\x1b@\x1dV\x00\x1dV\x00"]).is_empty());
        let after_bare_cut = capture(&[b"\x1b@\x1dV\x00X\n"]);
        assert_eq!(after_bare_cut.len(), 1);
        assert_eq!(
            after_bare_cut[0].2, b"X\n",
            "the bare cut's bytes are dropped"
        );
    }

    #[test]
    fn a_drawer_pulse_alone_is_a_receipt() {
        let receipts = capture(&[b"\x1b@\x1bp\x00\x19\xfa"]);
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].3, 0, "no blocks, but the drawer opened");
    }

    #[test]
    fn text_without_line_feed_prints_when_the_connection_ends() {
        let receipts = capture(&[b"\x1b@no newline"]);
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].2, b"\x1b@no newline");
        assert_eq!(receipts[0].3, 1);
    }

    #[test]
    fn the_store_limit_stops_the_capture() {
        let shared = Shared::new(Receipts::new(10, 200), Settings::default());
        let (events, _received) = mpsc::unbounded_channel();
        let mut capture = Capture::new(peer(), Paper::Mm80, CodePage::DEFAULT, &shared, &events);
        capture.feed(b"\x1b@A\n").expect("fits");
        assert_eq!(capture.feed(&[b'x'; 300]), Err(TooLarge));
    }

    #[test]
    fn collects_status_replies_without_starting_a_receipt() {
        let shared = Shared::new(Receipts::default(), Settings::default());
        let (events, _received) = mpsc::unbounded_channel();
        let mut capture = Capture::new(peer(), Paper::Mm80, CodePage::DEFAULT, &shared, &events);
        capture.feed(b"\x10\x04\x01\x1dr\x01").expect("fits");
        assert_eq!(capture.take_replies(), vec![0x12, 0x00]);
        assert!(capture.take_replies().is_empty());
        assert_eq!(capture.finish(ReceiptState::Done), 0);
    }
}
