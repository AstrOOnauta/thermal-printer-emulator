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

/// Reads one connection to its end. The job starts with the first byte: a connection that
/// closes without sending anything leaves no job behind.
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
        let id = match job {
            Some(id) => id,
            None => {
                let id = {
                    let mut jobs = lock(&shared.jobs);
                    let id = jobs.start(peer).id;
                    let _ = events.send(Event::Jobs(jobs.summaries()));
                    id
                };
                log::info!("job_started id={id} peer={peer}");
                job = Some(id);
                id
            }
        };
        if lock(&shared.jobs).append(id, &buffer[..read]).is_err() {
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
