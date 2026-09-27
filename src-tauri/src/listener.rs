//! TCP listener: accepts print jobs on port 9100, like a network receipt printer. Every
//! connection runs in its own task, so a slow or broken client never blocks the others.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{sleep, timeout};

use crate::jobs::{JobState, JobSummary, Jobs};

/// Every interface (decision 2): POS terminals print from other machines.
pub const DEFAULT_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 9100));

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
}

impl Limits {
    pub const PRODUCTION: Self = Self {
        max_connections: 16,
        idle_timeout: Duration::from_secs(5 * 60),
        bind_retry: Duration::from_secs(3),
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

/// What changed, for the UI. `Jobs` carries the whole list, oldest first: at most 100 small
/// summaries, sent when a job starts or ends.
#[derive(Clone, Debug)]
pub enum Event {
    Jobs(Vec<JobSummary>),
    Status(ListenerStatus),
}

/// State the listener writes and the UI commands read.
pub struct Shared {
    pub jobs: Mutex<Jobs>,
    pub status: Mutex<ListenerStatus>,
}

impl Shared {
    pub fn new(jobs: Jobs) -> Self {
        Self {
            jobs: Mutex::new(jobs),
            status: Mutex::new(ListenerStatus::Starting),
        }
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
pub async fn run(
    addr: SocketAddr,
    limits: Limits,
    shared: Arc<Shared>,
    events: mpsc::UnboundedSender<Event>,
) {
    let listener = loop {
        match TcpListener::bind(addr).await {
            Ok(listener) => break listener,
            Err(error) => {
                let status = ListenerStatus::Failed {
                    port: addr.port(),
                    error: BindError::from(&error),
                };
                if set_status(&shared, &events, status) {
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
    set_status(&shared, &events, ListenerStatus::Listening { port });
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
        let events = events.clone();
        tokio::spawn(async move {
            receive(stream, peer, limits.idle_timeout, &shared, &events).await;
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

/// Reads one connection to its end. The job starts once the first bytes pass `classify`:
/// a connection that closes without sending anything, or is rejected, leaves no job.
async fn receive(
    mut stream: TcpStream,
    peer: SocketAddr,
    idle_timeout: Duration,
    shared: &Shared,
    events: &mpsc::UnboundedSender<Event>,
) {
    let started = Instant::now();
    let mut buffer = vec![0u8; READ_BUFFER_BYTES];
    let mut job: Option<u64> = None;
    // Bytes read before the verdict: a lone `ESC` at most, plus the read that decides.
    let mut head: Vec<u8> = Vec::new();

    let state = loop {
        let read = match timeout(idle_timeout, stream.read(&mut buffer)).await {
            Err(_) => break JobState::IdleTimeout,
            Ok(Ok(0)) => break JobState::Done,
            Ok(Ok(read)) => read,
            Ok(Err(error)) => {
                log::warn!("read_failed peer={peer} error={error}");
                break JobState::ConnectionError;
            }
        };
        let chunk = match job {
            Some(id) => (id, &buffer[..read]),
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
                let id = {
                    let mut jobs = lock(&shared.jobs);
                    let id = jobs.start(peer).id;
                    let _ = events.send(Event::Jobs(jobs.summaries()));
                    id
                };
                log::info!("job_started id={id} peer={peer}");
                job = Some(id);
                (id, head.as_slice())
            }
        };
        if lock(&shared.jobs).append(chunk.0, chunk.1).is_err() {
            break JobState::TooLarge;
        }
    };

    let Some(id) = job else {
        return;
    };
    let summary = {
        let mut jobs = lock(&shared.jobs);
        let summary = jobs.finish(id, state);
        let _ = events.send(Event::Jobs(jobs.summaries()));
        summary
    };
    let size = summary.map_or(0, |summary| summary.size);
    log::info!(
        "job_ended id={id} peer={peer} state={state:?} bytes={size} ms={}",
        started.elapsed().as_millis()
    );
}

/// Stores and announces a new status. False when it did not change.
fn set_status(
    shared: &Shared,
    events: &mpsc::UnboundedSender<Event>,
    status: ListenerStatus,
) -> bool {
    let mut current = lock(&shared.status);
    if *current == status {
        return false;
    }
    *current = status.clone();
    let _ = events.send(Event::Status(status));
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
