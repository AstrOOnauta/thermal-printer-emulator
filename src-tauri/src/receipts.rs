//! Printed receipts, in memory only (decision 3 in `.ai/stack.md`): the newest
//! `max_receipts` within `max_bytes`, oldest finished receipt dropped first. A receipt
//! still printing is never dropped; when it would push the total over the limit, it is
//! refused instead. The total counts the raw bytes **and** the print model, so memory
//! stays bounded whatever the clients send.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::escpos::model::{Block, Paper};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptState {
    Printing,
    /// Ended by a cut, or by the client closing the connection.
    Done,
    /// The connection sent nothing for the idle timeout.
    IdleTimeout,
    /// Over the connection or memory limit; the rest was not read.
    TooLarge,
    /// Reset or another read error.
    ConnectionError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cut {
    Full,
    Partial,
}

/// What the receipt list shows. Never the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReceiptSummary {
    pub id: u64,
    pub peer: String,
    /// Unix ms.
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub state: ReceiptState,
    pub cut: Option<Cut>,
    /// `ESC p` / `DLE DC4 1` received.
    pub drawer: bool,
    pub beeps: u16,
    /// Raw bytes received for this receipt.
    pub size: usize,
    pub paper: Paper,
    /// Printable width in dots.
    pub width: u16,
    /// Paper length so far, in dots. With `width`, lets the webview reserve the space
    /// before drawing.
    pub height: u32,
}

/// A receipt to draw: its summary plus the print model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReceiptView {
    #[serde(flatten)]
    pub summary: ReceiptSummary,
    pub blocks: Vec<Block>,
}

struct Receipt {
    summary: ReceiptSummary,
    raw: Vec<u8>,
    blocks: Vec<Block>,
    /// Bytes this receipt counts against the store's limit.
    weight: usize,
}

/// Adding would take the store over its limit even after dropping every finished receipt.
#[derive(Debug, PartialEq, Eq)]
pub struct TooLarge;

pub struct Receipts {
    receipts: VecDeque<Receipt>,
    total: usize,
    next_id: u64,
    max_receipts: usize,
    max_bytes: usize,
}

impl Default for Receipts {
    fn default() -> Self {
        Self::new(100, 32 * 1024 * 1024)
    }
}

/// Rough heap size of a block: the text and bitmaps it holds plus fixed overhead.
fn weight(block: &Block) -> usize {
    const OVERHEAD: usize = 48;
    match block {
        Block::Line {
            segments, images, ..
        } => {
            OVERHEAD
                + segments
                    .iter()
                    .map(|segment| OVERHEAD + segment.text.len())
                    .sum::<usize>()
                + images
                    .iter()
                    .map(|image| OVERHEAD + image.bitmap.data.len())
                    .sum::<usize>()
        }
        Block::Image(image) => OVERHEAD + image.bitmap.data.len(),
        Block::Feed { .. } => OVERHEAD,
    }
}

impl Receipts {
    pub fn new(max_receipts: usize, max_bytes: usize) -> Self {
        Self {
            receipts: VecDeque::new(),
            total: 0,
            next_id: 1,
            max_receipts,
            max_bytes,
        }
    }

    /// A new receipt, printing, with the raw bytes received before its first output.
    pub fn start(&mut self, peer: SocketAddr, paper: Paper, raw: Vec<u8>) -> Result<u64, TooLarge> {
        if !self.make_room(raw.len(), 1) {
            return Err(TooLarge);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.total += raw.len();
        self.receipts.push_back(Receipt {
            summary: ReceiptSummary {
                id,
                peer: peer.to_string(),
                started_at: now_ms(),
                ended_at: None,
                state: ReceiptState::Printing,
                cut: None,
                drawer: false,
                beeps: 0,
                size: raw.len(),
                paper,
                width: paper.dots(),
                height: 0,
            },
            weight: raw.len(),
            raw,
            blocks: Vec::new(),
        });
        Ok(id)
    }

    pub fn add_raw(&mut self, id: u64, bytes: &[u8]) -> Result<(), TooLarge> {
        self.grow(id, bytes.len(), |receipt| {
            receipt.raw.extend_from_slice(bytes);
            receipt.summary.size = receipt.raw.len();
        })
    }

    /// Appends a block; a feed right after a feed grows it instead.
    pub fn add_block(&mut self, id: u64, block: Block) -> Result<(), TooLarge> {
        if let (Block::Feed { height: more }, Some(receipt)) = (&block, self.printing_mut(id)) {
            if let Some(Block::Feed { height }) = receipt.blocks.last_mut() {
                *height = height.saturating_add(*more);
                receipt.summary.height += u32::from(*more);
                return Ok(());
            }
        }
        let size = weight(&block);
        let block_height = u32::from(block.height());
        self.grow(id, size, |receipt| {
            receipt.blocks.push(block);
            receipt.summary.height += block_height;
        })
    }

    pub fn drawer(&mut self, id: u64) {
        if let Some(receipt) = self.printing_mut(id) {
            receipt.summary.drawer = true;
        }
    }

    pub fn beep(&mut self, id: u64) {
        if let Some(receipt) = self.printing_mut(id) {
            receipt.summary.beeps = receipt.summary.beeps.saturating_add(1);
        }
    }

    pub fn finish(&mut self, id: u64, state: ReceiptState, cut: Option<Cut>) {
        if let Some(receipt) = self.printing_mut(id) {
            receipt.summary.state = state;
            receipt.summary.cut = cut;
            receipt.summary.ended_at = Some(now_ms());
        }
    }

    /// Drops every finished receipt; one still printing stays.
    pub fn clear(&mut self) {
        self.receipts
            .retain(|receipt| receipt.summary.state == ReceiptState::Printing);
        self.total = self.receipts.iter().map(|receipt| receipt.weight).sum();
    }

    /// Oldest first.
    pub fn summaries(&self) -> Vec<ReceiptSummary> {
        self.receipts
            .iter()
            .map(|receipt| receipt.summary.clone())
            .collect()
    }

    pub fn view(&self, id: u64) -> Option<ReceiptView> {
        let receipt = self
            .receipts
            .iter()
            .find(|receipt| receipt.summary.id == id)?;
        Some(ReceiptView {
            summary: receipt.summary.clone(),
            blocks: receipt.blocks.clone(),
        })
    }

    pub fn raw(&self, id: u64) -> Option<&[u8]> {
        let receipt = self
            .receipts
            .iter()
            .find(|receipt| receipt.summary.id == id)?;
        Some(&receipt.raw)
    }

    fn printing_mut(&mut self, id: u64) -> Option<&mut Receipt> {
        self.receipts.iter_mut().find(|receipt| {
            receipt.summary.id == id && receipt.summary.state == ReceiptState::Printing
        })
    }

    /// Makes room for `size` more bytes, then lets `change` apply them to a receipt that is
    /// still printing. Nothing changes on error.
    fn grow(
        &mut self,
        id: u64,
        size: usize,
        change: impl FnOnce(&mut Receipt),
    ) -> Result<(), TooLarge> {
        if self.printing_mut(id).is_none() {
            return Ok(());
        }
        if !self.make_room(size, 0) {
            return Err(TooLarge);
        }
        // `make_room` only drops finished receipts, so this one is still there.
        let receipt = self
            .printing_mut(id)
            .expect("a printing receipt is never dropped");
        change(receipt);
        receipt.weight += size;
        self.total += size;
        Ok(())
    }

    /// Drops the oldest finished receipts until `size` more bytes and `extra` more receipts
    /// fit. False if printing receipts alone leave no room for the bytes.
    fn make_room(&mut self, size: usize, extra: usize) -> bool {
        while self.receipts.len() + extra > self.max_receipts || self.total + size > self.max_bytes
        {
            let Some(index) = self
                .receipts
                .iter()
                .position(|receipt| receipt.summary.state != ReceiptState::Printing)
            else {
                return self.total + size <= self.max_bytes;
            };
            let dropped = self.receipts.remove(index).expect("index from position");
            self.total -= dropped.weight;
        }
        true
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> SocketAddr {
        "192.168.1.10:50000".parse().expect("valid address")
    }

    fn feed(height: u16) -> Block {
        Block::Feed { height }
    }

    fn finished(receipts: &mut Receipts, raw: &[u8]) -> u64 {
        let id = receipts
            .start(peer(), Paper::Mm80, raw.to_vec())
            .expect("fits");
        receipts.finish(id, ReceiptState::Done, Some(Cut::Full));
        id
    }

    fn ids(receipts: &Receipts) -> Vec<u64> {
        receipts
            .summaries()
            .iter()
            .map(|receipt| receipt.id)
            .collect()
    }

    #[test]
    fn keeps_raw_bytes_blocks_and_flags() {
        let mut receipts = Receipts::default();
        let id = receipts
            .start(peer(), Paper::Mm58, b"\x1b@".to_vec())
            .expect("fits");
        receipts.add_raw(id, b"Hi\n").expect("fits");
        receipts.add_block(id, feed(30)).expect("fits");
        receipts.add_block(id, feed(10)).expect("merges");
        receipts.drawer(id);
        receipts.beep(id);
        receipts.finish(id, ReceiptState::Done, Some(Cut::Partial));

        let view = receipts.view(id).expect("exists");
        assert_eq!(view.summary.size, 5);
        assert_eq!(view.summary.state, ReceiptState::Done);
        assert_eq!(view.summary.cut, Some(Cut::Partial));
        assert!(view.summary.drawer);
        assert_eq!(view.summary.beeps, 1);
        assert_eq!((view.summary.paper, view.summary.width), (Paper::Mm58, 384));
        assert_eq!(view.blocks, vec![feed(40)], "consecutive feeds merge");
        assert_eq!(view.summary.height, 40);
        assert_eq!(receipts.raw(id), Some(&b"\x1b@Hi\n"[..]));
    }

    #[test]
    fn drops_the_oldest_finished_receipt_over_count_or_bytes() {
        let mut receipts = Receipts::new(2, 1000);
        let first = finished(&mut receipts, b"a");
        let second = finished(&mut receipts, b"b");
        let third = finished(&mut receipts, b"c");
        assert_eq!(ids(&receipts), vec![second, third]);
        assert!(!ids(&receipts).contains(&first));

        let mut receipts = Receipts::new(10, 10);
        finished(&mut receipts, b"123456");
        let kept = finished(&mut receipts, b"123456");
        assert_eq!(ids(&receipts), vec![kept]);
    }

    #[test]
    fn never_drops_a_printing_receipt() {
        let mut receipts = Receipts::new(1, 1000);
        let printing = receipts
            .start(peer(), Paper::Mm80, Vec::new())
            .expect("fits");
        let other = receipts
            .start(peer(), Paper::Mm80, Vec::new())
            .expect("fits");
        assert_eq!(ids(&receipts), vec![printing, other]);
        receipts.finish(printing, ReceiptState::Done, None);
        receipts.add_raw(other, b"x").expect("fits");
        assert_eq!(ids(&receipts), vec![other]);
    }

    #[test]
    fn refuses_when_printing_receipts_fill_the_store() {
        let mut receipts = Receipts::new(10, 100);
        let id = receipts
            .start(peer(), Paper::Mm80, vec![0; 60])
            .expect("fits");
        assert_eq!(
            receipts.add_block(id, feed(1)),
            Err(TooLarge),
            "48 + 60 > 100"
        );
        assert_eq!(
            receipts.start(peer(), Paper::Mm80, vec![0; 50]),
            Err(TooLarge)
        );
        receipts
            .add_raw(id, &[0; 40])
            .expect("exactly the limit fits");
    }

    #[test]
    fn finished_receipts_take_no_more_changes() {
        let mut receipts = Receipts::default();
        let id = finished(&mut receipts, b"a");
        receipts.add_raw(id, b"b").expect("ignored");
        receipts.add_block(id, feed(1)).expect("ignored");
        receipts.drawer(id);
        let view = receipts.view(id).expect("exists");
        assert_eq!(
            (view.summary.size, view.blocks.len(), view.summary.drawer),
            (1, 0, false)
        );
        assert_eq!(receipts.view(999), None);
    }

    #[test]
    fn clear_keeps_only_printing_receipts() {
        let mut receipts = Receipts::new(10, 1000);
        finished(&mut receipts, b"done");
        let printing = receipts
            .start(peer(), Paper::Mm80, b"open".to_vec())
            .expect("fits");
        receipts.clear();
        assert_eq!(ids(&receipts), vec![printing]);
        // The freed bytes are available again.
        receipts
            .add_raw(printing, &[0; 996])
            .expect("exactly the limit fits");
    }
}
