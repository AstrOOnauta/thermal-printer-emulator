//! The listener against real sockets on 127.0.0.1, each test on its own ephemeral port.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use thermal_printer_emulator_lib::jobs::{JobState, JobSummary, Jobs};
use thermal_printer_emulator_lib::listener::{
    self, BindError, Event, Limits, ListenerStatus, Shared,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

const FAST: Limits = Limits {
    max_connections: 4,
    idle_timeout: Duration::from_secs(5),
    bind_retry: Duration::from_millis(50),
};

struct Harness {
    events: mpsc::UnboundedReceiver<Event>,
    addr: SocketAddr,
}

impl Harness {
    async fn start(limits: Limits, jobs: Jobs) -> Self {
        let addr: SocketAddr = "127.0.0.1:0".parse().expect("valid address");
        let shared = Arc::new(Shared::new(jobs));
        let (sender, events) = mpsc::unbounded_channel();
        tokio::spawn(listener::run(addr, limits, shared, sender));
        let mut harness = Self { events, addr };
        let port = harness.listening().await;
        harness.addr.set_port(port);
        harness
    }

    async fn listening(&mut self) -> u16 {
        loop {
            if let Event::Status(ListenerStatus::Listening { port }) = self.next().await {
                return port;
            }
        }
    }

    async fn next(&mut self) -> Event {
        timeout(WAIT, self.events.recv())
            .await
            .expect("an event in time")
            .expect("listener still running")
    }

    /// The job list once its newest job reaches `state`.
    async fn jobs_when_last_is(&mut self, state: JobState) -> Vec<JobSummary> {
        loop {
            if let Event::Jobs(jobs) = self.next().await {
                if jobs.last().map(|job| job.state) == Some(state) {
                    return jobs;
                }
            }
        }
    }

    async fn connect(&self) -> TcpStream {
        TcpStream::connect(self.addr).await.expect("connects")
    }

    async fn send_job(&self, bytes: &[u8]) {
        let mut stream = self.connect().await;
        stream.write_all(bytes).await.expect("writes");
        stream.shutdown().await.expect("closes");
    }
}

/// True once the server closed the connection (EOF or reset), false if it is still open.
async fn closed_by_server(stream: &mut TcpStream) -> bool {
    let mut byte = [0u8; 1];
    match timeout(Duration::from_millis(500), stream.read(&mut byte)).await {
        Ok(Ok(0)) | Ok(Err(_)) => true,
        Ok(Ok(_)) => panic!("the emulator never writes in P1"),
        Err(_) => false,
    }
}

#[tokio::test]
async fn receives_a_job_until_the_client_closes() {
    let mut harness = Harness::start(FAST, Jobs::default()).await;
    harness.send_job(b"\x1b@Hello\n").await;

    let jobs = harness.jobs_when_last_is(JobState::Done).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].size, 8);
    assert!(jobs[0].peer.starts_with("127.0.0.1:"));
    assert!(jobs[0].ended_at.is_some());
}

#[tokio::test]
async fn announces_a_job_when_it_starts() {
    let mut harness = Harness::start(FAST, Jobs::default()).await;
    let mut stream = harness.connect().await;
    stream.write_all(b"\x1b@").await.expect("writes");

    let jobs = harness.jobs_when_last_is(JobState::Receiving).await;
    assert_eq!(jobs.len(), 1);
    drop(stream);
    harness.jobs_when_last_is(JobState::Done).await;
}

#[tokio::test]
async fn a_connection_without_bytes_leaves_no_job() {
    let mut harness = Harness::start(FAST, Jobs::default()).await;
    drop(harness.connect().await);
    harness.send_job(b"\x1b@A").await;

    let jobs = harness.jobs_when_last_is(JobState::Done).await;
    assert_eq!(jobs.len(), 1, "only the connection that sent bytes");
}

#[tokio::test]
async fn closes_an_idle_connection() {
    let limits = Limits {
        idle_timeout: Duration::from_millis(100),
        ..FAST
    };
    let mut harness = Harness::start(limits, Jobs::default()).await;
    let mut stream = harness.connect().await;
    stream.write_all(b"\x1b@").await.expect("writes");

    let jobs = harness.jobs_when_last_is(JobState::IdleTimeout).await;
    assert_eq!(jobs[0].size, 2);
    assert!(closed_by_server(&mut stream).await);
}

#[tokio::test]
async fn stops_a_job_over_the_size_limit() {
    let mut harness = Harness::start(FAST, Jobs::new(10, 1024, 16)).await;
    let mut stream = harness.connect().await;
    // A valid opening, then more than the 16-byte limit.
    let mut job = b"\x1b@".to_vec();
    job.extend([b'x'; 62]);
    stream.write_all(&job).await.expect("writes");

    let jobs = harness.jobs_when_last_is(JobState::TooLarge).await;
    assert!(jobs[0].size <= 16);
    assert!(closed_by_server(&mut stream).await);
}

#[tokio::test]
async fn refuses_connections_over_the_limit() {
    let limits = Limits {
        max_connections: 1,
        ..FAST
    };
    let mut harness = Harness::start(limits, Jobs::default()).await;
    let mut first = harness.connect().await;
    first.write_all(b"\x1b@").await.expect("writes");
    harness.jobs_when_last_is(JobState::Receiving).await;

    let mut second = harness.connect().await;
    assert!(closed_by_server(&mut second).await);
    assert!(
        !closed_by_server(&mut first).await,
        "the first one stays open"
    );

    // The slot frees up when the first connection ends.
    drop(first);
    harness.jobs_when_last_is(JobState::Done).await;
    harness.send_job(b"\x1b@B").await;
    let jobs = harness.jobs_when_last_is(JobState::Done).await;
    assert_eq!(jobs.len(), 2);
}

#[tokio::test]
async fn retries_until_the_port_is_free() {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").expect("binds");
    let addr = taken.local_addr().expect("has an address");
    let shared = Arc::new(Shared::new(Jobs::default()));
    let (sender, mut events) = mpsc::unbounded_channel();
    tokio::spawn(listener::run(addr, FAST, Arc::clone(&shared), sender));

    let failed = timeout(WAIT, events.recv()).await.expect("in time");
    assert!(
        matches!(
            failed,
            Some(Event::Status(ListenerStatus::Failed {
                error: BindError::PortInUse,
                ..
            }))
        ),
        "got {failed:?}"
    );

    drop(taken);
    let listening = timeout(WAIT, events.recv()).await.expect("in time");
    assert!(
        matches!(listening, Some(Event::Status(ListenerStatus::Listening { port })) if port == addr.port()),
        "got {listening:?}"
    );
    assert_eq!(
        *listener::lock(&shared.status),
        ListenerStatus::Listening { port: addr.port() }
    );
}

#[tokio::test]
async fn rejects_connections_that_are_not_escpos() {
    let mut harness = Harness::start(FAST, Jobs::default()).await;
    for opening in [
        &b"GET / HTTP/1.1\r\nHost: printer\r\n\r\n"[..],
        b"\x16\x03\x01\x02\x00",
        b"\x1b%-12345X@PJL INFO STATUS\r\n",
        b"Hello\n",
    ] {
        let mut stream = harness.connect().await;
        stream.write_all(opening).await.expect("writes");
        assert!(closed_by_server(&mut stream).await, "{opening:02x?}");
    }
    harness.send_job(b"\x1b@ok").await;

    let jobs = harness.jobs_when_last_is(JobState::Done).await;
    assert_eq!(jobs.len(), 1, "rejected connections leave no job");
    assert_eq!(jobs[0].size, 4);
}

#[tokio::test]
async fn accepts_a_status_query_opening() {
    let mut harness = Harness::start(FAST, Jobs::default()).await;
    harness.send_job(b"\x10\x04\x01").await;
    let jobs = harness.jobs_when_last_is(JobState::Done).await;
    assert_eq!(jobs[0].size, 3);
}

#[tokio::test]
async fn keeps_an_esc_split_from_its_at() {
    let mut harness = Harness::start(FAST, Jobs::default()).await;
    let mut stream = harness.connect().await;
    stream.set_nodelay(true).expect("sets nodelay");
    stream.write_all(b"\x1b").await.expect("writes");
    tokio::time::sleep(Duration::from_millis(50)).await;
    stream.write_all(b"@Hi").await.expect("writes");
    stream.shutdown().await.expect("closes");

    let jobs = harness.jobs_when_last_is(JobState::Done).await;
    assert_eq!(jobs[0].size, 4, "the held ESC is part of the job");
}
