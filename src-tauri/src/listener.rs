//! TCP listener: accepts print jobs on port 9100, like a network receipt printer. Every
//! connection runs in its own task, so a slow or broken client never blocks the others.

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, Semaphore};
use tokio::time::{sleep, timeout};

use crate::capture::Capture;
use crate::receipts::{ReceiptState, Receipts};
use crate::settings::Settings;

const READ_BUFFER_BYTES: usize = 8 * 1024;
/// Pause after a failed `accept` (e.g. out of file descriptors), so it does not spin.
const ACCEPT_ERROR_PAUSE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_connections: usize,
    /// A connection that sends nothing for this long is closed.
    pub idle_timeout: Duration,
    /// Wait between bind attempts while the port is unavailable.
    pub bind_retry: Duration,
    /// Bytes one connection may send; past it, the receipt ends `too_large`.
    pub max_connection_bytes: usize,
}

impl Limits {
    pub const PRODUCTION: Self = Self {
        max_connections: 16,
        idle_timeout: Duration::from_secs(5 * 60),
        bind_retry: Duration::from_secs(3),
        max_connection_bytes: 16 * 1024 * 1024,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindError {
    PortInUse,
    /// Also Windows' reserved port ranges (Hyper-V / WinNAT), which answer "access denied".
    PermissionDenied,
    Other,
}

impl From<&std::io::Error> for BindError {
    fn from(error: &std::io::Error) -> Self {
        match error.kind() {
            ErrorKind::AddrInUse => Self::PortInUse,
            ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ListenerStatus {
    Starting,
    Listening { port: u16 },
    Failed { port: u16, error: BindError },
}

/// State the listener writes and the UI commands read.
pub struct Shared {
    pub receipts: Mutex<Receipts>,
    pub status: Mutex<ListenerStatus>,
    /// Paper and code page are read by each new connection.
    pub settings: Mutex<Settings>,
    /// Wakes the UI forwarder after receipts or the status changed. It reads the current
    /// state when it wakes, so a burst of changes is one wake-up: a client printing receipts
    /// as fast as it can never queues memory.
    pub changes: Notify,
}

impl Shared {
    pub fn new(receipts: Receipts, settings: Settings) -> Self {
        Self {
            receipts: Mutex::new(receipts),
            status: Mutex::new(ListenerStatus::Starting),
            settings: Mutex::new(settings),
            changes: Notify::new(),
        }
    }

    /// Call after changing `receipts` or `status`.
    pub fn changed(&self) {
        // One stored permit at most: changes made while nobody waits are one wake-up.
        self.changes.notify_one();
    }
}

/// A panic in another task while holding the lock leaves plain data behind (no invariant
/// spans a lock), so keep going instead of spreading the panic.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Binds `addr` (retrying while it is unavailable), then accepts connections forever.
pub async fn run(addr: SocketAddr, limits: Limits, shared: Arc<Shared>) {
    let listener = loop {
        match TcpListener::bind(addr).await {
            Ok(listener) => break listener,
            Err(error) => {
                let status = ListenerStatus::Failed {
                    port: addr.port(),
                    error: BindError::from(&error),
                };
                if set_status(&shared, status) {
                    log::warn!("bind_failed addr={addr} error={error}");
                }
                sleep(limits.bind_retry).await;
            }
        }
    };
    let port = listener
        .local_addr()
        .map(|local| local.port())
        .unwrap_or(addr.port());
    set_status(&shared, ListenerStatus::Listening { port });
    log::info!("listening addr={addr} port={port}");

    let connections = Arc::new(Semaphore::new(limits.max_connections));
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                log::warn!("accept_failed error={error}");
                sleep(ACCEPT_ERROR_PAUSE).await;
                continue;
            }
        };
        let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
            log::warn!("connection_refused peer={peer} reason=too_many_connections");
            continue; // Dropping the stream closes it.
        };
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            receive(stream, peer, limits, &shared).await;
            drop(permit);
        });
    }
}

const ESC: u8 = 0x1b;
const DLE: u8 = 0x10;
const GS: u8 = 0x1d;
/// `ESC @`: initialize printer.
const INITIALIZE: u8 = b'@';

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Accept,
    Reject,
    /// Only a lone `ESC` so far: the next byte decides.
    Incomplete,
}

/// Decision 5: a print job opens with `ESC @` (every ESC/POS library sends it first), or
/// with a status query or command (`DLE …`, `GS …`). Port scanners probing 9100 open with
/// something else: HTTP `G`, TLS `0x16`, PJL `@PJL`, PJL's UEL `ESC %-12345X`, Redis `*`,
/// a null byte. Requiring `@` after `ESC` is what turns the UEL away.
fn classify(head: &[u8]) -> Verdict {
    match head {
        [] | [ESC] => Verdict::Incomplete,
        [ESC, INITIALIZE, ..] | [DLE, ..] | [GS, ..] => Verdict::Accept,
        _ => Verdict::Reject,
    }
}

/// Reads one connection to its end and turns it into receipts (`capture.rs`). Decoding
/// starts once the first bytes pass `classify`: a connection that closes without sending
/// anything, or is rejected, leaves nothing behind.
async fn receive(mut stream: TcpStream, peer: SocketAddr, limits: Limits, shared: &Shared) {
    let started = Instant::now();
    let mut buffer = vec![0u8; READ_BUFFER_BYTES];
    let mut capture: Option<Capture> = None;
    // Bytes read before the verdict: a lone `ESC` at most, plus the read that decides.
    let mut head: Vec<u8> = Vec::new();
    let mut received = 0usize;

    let state = loop {
        let read = match timeout(limits.idle_timeout, stream.read(&mut buffer)).await {
            Err(_) => break ReceiptState::IdleTimeout,
            Ok(Ok(0)) => break ReceiptState::Done,
            Ok(Ok(read)) => read,
            Ok(Err(error)) => {
                log::warn!("read_failed peer={peer} error={error}");
                break ReceiptState::ConnectionError;
            }
        };
        received += read;
        if received > limits.max_connection_bytes {
            break ReceiptState::TooLarge;
        }
        let fed = match capture.as_mut() {
            Some(capture) => capture.feed(&buffer[..read]),
            None => {
                head.extend_from_slice(&buffer[..read]);
                match classify(&head) {
                    Verdict::Incomplete => continue,
                    Verdict::Reject => {
                        let first = &head[..head.len().min(4)];
                        log::info!("connection_rejected peer={peer} first_bytes={first:02x?}");
                        return;
                    }
                    Verdict::Accept => {}
                }
                log::info!("connection_accepted peer={peer}");
                let (paper, code_page) = {
                    let settings = lock(&shared.settings);
                    (settings.paper, settings.code_page())
                };
                capture
                    .insert(Capture::new(peer, paper, code_page, shared))
                    .feed(&std::mem::take(&mut head))
            }
        };
        if fed.is_err() {
            break ReceiptState::TooLarge;
        }
        let replies = capture
            .as_mut()
            .map(Capture::take_replies)
            .unwrap_or_default();
        if !replies.is_empty() {
            // A client that never reads its replies must not hold the task forever.
            match timeout(limits.idle_timeout, stream.write_all(&replies)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    log::warn!("reply_failed peer={peer} error={error}");
                    break ReceiptState::ConnectionError;
                }
                Err(_) => break ReceiptState::IdleTimeout,
            }
        }
    };

    let Some(capture) = capture else {
        return;
    };
    let receipts = capture.finish(state);
    log::info!(
        "connection_ended peer={peer} state={state:?} bytes={received} receipts={receipts} ms={}",
        started.elapsed().as_millis()
    );
}

/// Stores and announces a new status. False when it did not change.
fn set_status(shared: &Shared, status: ListenerStatus) -> bool {
    {
        let mut current = lock(&shared.status);
        if *current == status {
            return false;
        }
        *current = status;
    }
    shared.changed();
    true
}

#[cfg(test)]
mod tests {
    use super::{classify, Verdict};

    #[test]
    fn accepts_escpos_openings() {
        for head in [&b"\x1b@"[..], b"\x1b@Hello", b"\x10\x04\x01", b"\x1dV\x00"] {
            assert_eq!(classify(head), Verdict::Accept, "{head:02x?}");
        }
    }

    #[test]
    fn waits_for_the_byte_after_esc() {
        assert_eq!(classify(b""), Verdict::Incomplete);
        assert_eq!(classify(b"\x1b"), Verdict::Incomplete);
    }

    #[test]
    fn rejects_other_protocols_and_plain_text() {
        for head in [
            &b"GET / HTTP/1.1\r\n"[..],
            b"\x16\x03\x01",
            b"@PJL INFO STATUS",
            b"\x1b%-12345X@PJL",
            b"*1\r\n$4\r\nPING",
            b"\x00",
            b"Hello\n",
            b"\x1bt\x02",
        ] {
            assert_eq!(classify(head), Verdict::Reject, "{head:02x?}");
        }
    }
}
