//! The listener against real sockets on 127.0.0.1, each test on its own ephemeral port.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use thermal_printer_emulator_lib::listener::{
    self, BindError, Event, Limits, ListenerStatus, Shared,
};
use thermal_printer_emulator_lib::receipts::{Cut, ReceiptState, ReceiptSummary, Receipts};
use thermal_printer_emulator_lib::settings::Settings;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

const FAST: Limits = Limits {
    max_connections: 4,
    idle_timeout: Duration::from_secs(5),
    bind_retry: Duration::from_millis(50),
    max_connection_bytes: 1024 * 1024,
};

struct Harness {
    shared: Arc<Shared>,
    events: mpsc::UnboundedReceiver<Event>,
    addr: SocketAddr,
}

impl Harness {
    async fn start(limits: Limits) -> Self {
        let addr: SocketAddr = "127.0.0.1:0".parse().expect("valid address");
        let shared = Arc::new(Shared::new(Receipts::default(), Settings::default()));
        let (sender, events) = mpsc::unbounded_channel();
        tokio::spawn(listener::run(addr, limits, Arc::clone(&shared), sender));
        let mut harness = Self {
            shared,
            events,
            addr,
        };
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

    /// The receipt list once it has `count` receipts and the newest is in `state`.
    async fn receipts_when(&mut self, count: usize, state: ReceiptState) -> Vec<ReceiptSummary> {
        loop {
            if let Event::Receipts(receipts) = self.next().await {
                if receipts.len() == count
                    && receipts.last().map(|receipt| receipt.state) == Some(state)
                {
                    return receipts;
                }
            }
        }
    }

    async fn connect(&self) -> TcpStream {
        TcpStream::connect(self.addr).await.expect("connects")
    }

    async fn send(&self, bytes: &[u8]) {
        let mut stream = self.connect().await;
        stream.write_all(bytes).await.expect("writes");
        stream.shutdown().await.expect("closes");
    }

    fn raw(&self, id: u64) -> Vec<u8> {
        listener::lock(&self.shared.receipts)
            .raw(id)
            .expect("receipt exists")
            .to_vec()
    }
}

/// True once the server closed the connection (EOF or reset), false if it is still open.
async fn closed_by_server(stream: &mut TcpStream) -> bool {
    let mut byte = [0u8; 1];
    match timeout(Duration::from_millis(500), stream.read(&mut byte)).await {
        Ok(Ok(0)) | Ok(Err(_)) => true,
        Ok(Ok(_)) => panic!("unexpected reply"),
        Err(_) => false,
    }
}

#[tokio::test]
async fn prints_a_receipt_until_the_client_closes() {
    let mut harness = Harness::start(FAST).await;
    harness.send(b"\x1b@Hello\n").await;

    let receipts = harness.receipts_when(1, ReceiptState::Done).await;
    assert_eq!(receipts[0].size, 8);
    assert_eq!(receipts[0].cut, None);
    assert!(receipts[0].peer.starts_with("127.0.0.1:"));
    let view = listener::lock(&harness.shared.receipts)
        .view(receipts[0].id)
        .expect("exists");
    assert_eq!(view.blocks.len(), 1);
}

#[tokio::test]
async fn cuts_split_one_connection_into_receipts() {
    let mut harness = Harness::start(FAST).await;
    harness
        .send(b"\x1b@One\n\x1dV\x00\x1b@Two\n\x1dV\x01")
        .await;

    let receipts = harness.receipts_when(2, ReceiptState::Done).await;
    assert_eq!(receipts[0].cut, Some(Cut::Full));
    assert_eq!(receipts[1].cut, Some(Cut::Partial));
    assert_eq!(harness.raw(receipts[0].id), b"\x1b@One\n\x1dV\x00");
    assert_eq!(harness.raw(receipts[1].id), b"\x1b@Two\n\x1dV\x01");
}

#[tokio::test]
async fn announces_a_receipt_while_it_prints() {
    let mut harness = Harness::start(FAST).await;
    let mut stream = harness.connect().await;
    stream.write_all(b"\x1b@Line\n").await.expect("writes");

    harness.receipts_when(1, ReceiptState::Printing).await;
    drop(stream);
    harness.receipts_when(1, ReceiptState::Done).await;
}

#[tokio::test]
async fn status_only_connections_leave_no_receipt() {
    let mut harness = Harness::start(FAST).await;
    harness.send(b"\x10\x04\x01").await;
    drop(harness.connect().await);
    harness.send(b"\x1b@A\n").await;

    let receipts = harness.receipts_when(1, ReceiptState::Done).await;
    assert_eq!(harness.raw(receipts[0].id), b"\x1b@A\n");
}

#[tokio::test]
async fn closes_an_idle_connection() {
    let limits = Limits {
        idle_timeout: Duration::from_millis(100),
        ..FAST
    };
    let mut harness = Harness::start(limits).await;
    let mut stream = harness.connect().await;
    stream.write_all(b"\x1b@waiting").await.expect("writes");

    let receipts = harness.receipts_when(1, ReceiptState::IdleTimeout).await;
    assert_eq!(receipts[0].size, 9, "the unfinished line still printed");
    assert!(closed_by_server(&mut stream).await);
}

#[tokio::test]
async fn stops_a_connection_over_its_byte_limit() {
    let limits = Limits {
        max_connection_bytes: 16,
        ..FAST
    };
    let mut harness = Harness::start(limits).await;
    let mut stream = harness.connect().await;
    stream
        .write_all(b"\x1b@First line\n")
        .await
        .expect("writes");
    harness.receipts_when(1, ReceiptState::Printing).await;
    stream.write_all(&[b'x'; 64]).await.expect("writes");

    harness.receipts_when(1, ReceiptState::TooLarge).await;
    assert!(closed_by_server(&mut stream).await);
}

#[tokio::test]
async fn refuses_connections_over_the_limit() {
    let limits = Limits {
        max_connections: 1,
        ..FAST
    };
    let mut harness = Harness::start(limits).await;
    let mut first = harness.connect().await;
    first.write_all(b"\x1b@A\n").await.expect("writes");
    harness.receipts_when(1, ReceiptState::Printing).await;

    let mut second = harness.connect().await;
    assert!(closed_by_server(&mut second).await);
    assert!(
        !closed_by_server(&mut first).await,
        "the first one stays open"
    );

    // The slot frees up when the first connection ends.
    drop(first);
    harness.receipts_when(1, ReceiptState::Done).await;
    harness.send(b"\x1b@B\n").await;
    harness.receipts_when(2, ReceiptState::Done).await;
}

#[tokio::test]
async fn rejects_connections_that_are_not_escpos() {
    let mut harness = Harness::start(FAST).await;
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
    harness.send(b"\x1b@ok\n").await;

    let receipts = harness.receipts_when(1, ReceiptState::Done).await;
    assert_eq!(harness.raw(receipts[0].id), b"\x1b@ok\n");
}

#[tokio::test]
async fn keeps_an_esc_split_from_its_at() {
    let mut harness = Harness::start(FAST).await;
    let mut stream = harness.connect().await;
    stream.set_nodelay(true).expect("sets nodelay");
    stream.write_all(b"\x1b").await.expect("writes");
    tokio::time::sleep(Duration::from_millis(50)).await;
    stream.write_all(b"@Hi\n").await.expect("writes");
    stream.shutdown().await.expect("closes");

    let receipts = harness.receipts_when(1, ReceiptState::Done).await;
    assert_eq!(
        harness.raw(receipts[0].id),
        b"\x1b@Hi\n",
        "the held ESC is kept"
    );
}

#[tokio::test]
async fn retries_until_the_port_is_free() {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").expect("binds");
    let addr = taken.local_addr().expect("has an address");
    let shared = Arc::new(Shared::new(Receipts::default(), Settings::default()));
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
async fn answers_status_requests_on_the_socket() {
    let harness = Harness::start(FAST).await;
    let mut stream = harness.connect().await;
    stream.write_all(b"\x10\x04\x01").await.expect("writes");
    let mut reply = [0u8; 1];
    timeout(WAIT, stream.read_exact(&mut reply))
        .await
        .expect("a reply in time")
        .expect("reads");
    assert_eq!(reply, [0x12], "online, no error");

    stream.write_all(b"\x1dIB").await.expect("writes");
    let mut name = [0u8; 26];
    timeout(WAIT, stream.read_exact(&mut name))
        .await
        .expect("a reply in time")
        .expect("reads");
    assert_eq!(&name, b"_Thermal Printer Emulator\0");
}
