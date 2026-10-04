# Print job

From the POS's TCP connection to receipts in the list. Decisions 1–3 and 5 in `stack.md`
are the product rules behind this flow; the decoder itself is in `conventions/escpos.md`.

```
POS ─TCP─▶ listener.rs ─ classify ─▶ capture.rs ─ Decoder ─▶ receipts.rs ─ Event ─▶ webview
           (bind, accept,  (first     (bytes →      (parser +    (memory,
            limits)         bytes)     receipts)     printer)     limits)
```

## Listener (`listener.rs`)

```
listener::run(DEFAULT_ADDR = 0.0.0.0:9100, Limits::PRODUCTION, shared, events)
  bind ── fails ──▶ status Failed { port, error } ─▶ retry every 3 s (logged once per change)
   │
   └─ ok ──▶ status Listening { port } ─▶ accept loop
                 │
                 ├─ 16 connections open already ─▶ close the new one (logged)
                 └─ else: one task per connection ─▶ receive()
```

- `BindError`: `port_in_use` (`AddrInUse`), `permission_denied` (also Windows' reserved
  port ranges from Hyper-V / WinNAT), `other`.
- A failed `accept` (e.g. out of file descriptors) is logged and retried after 100 ms.
- Every interface, IPv4 only. POS software addresses printers by IPv4.

## Connection filter (`classify`, decision 5)

Decoding starts only when the connection opens like ESC/POS:

| First bytes          | Verdict                                                    |
| -------------------- | ---------------------------------------------------------- |
| `ESC @` (initialize) | accept: every ESC/POS library sends it first               |
| `DLE …` / `GS …`     | accept: drivers that open with a status query or a command |
| a lone `ESC`         | wait for the next byte (TCP may split `ESC` from `@`)      |
| anything else        | reject: close the connection, `connection_rejected`        |

The bytes read before the verdict are kept and decoded first. Rejected on purpose: HTTP,
TLS, PJL and its UEL (`ESC %-12345X`), Redis, null-byte banner grabs.

**Trade-off**: plain text (`echo hi | nc host 9100`) and a job that opens with another
command (e.g. `ESC t` before `ESC @`) are rejected too. Real ESC/POS software always
initializes first, and the README says so. If a real POS trips this, revisit the
decision; don't add a blocklist of protocols.

## Connection → receipts (`receive`, `capture.rs`)

Each accepted connection gets a `Capture` with its own `Decoder` (80 mm paper until P3
makes it a setting). Bytes are read in 8 KiB chunks and fed to it.

- **A receipt starts with its first visible output**: a printed line, an image, a drawer
  pulse or a beep. A connection that only asks for the status (POS software polls it every
  few seconds) or only cuts leaves no receipt.
- **A receipt ends at a cut** (`GS V`, `ESC i`, `ESC m`) with its kind (`full`/`partial`),
  or when the connection ends. Several receipts can share one connection.
- **Raw bytes are split exactly after the cut command**: the decoder reports the stream
  offset where each command ends, so each receipt keeps the bytes that printed it (tested
  at every split point of a stream). Bytes before the first visible output join the
  receipt that output starts; the bytes of a bare cut are dropped.
- Status requests are answered on the socket after each read (`conventions/escpos.md`
  § Status replies); they never start a receipt.
- When the connection ends, the unfinished line is printed (`Printer::finish`) and the open
  receipt takes the connection's end state:

| Event                                    | `ReceiptState`     |
| ---------------------------------------- | ------------------ |
| Cut, or the client closes the connection | `done`             |
| No byte for 5 min (`idle_timeout`)       | `idle_timeout`     |
| Over a size limit (see below)            | `too_large`        |
| Reset or another read error              | `connection_error` |

The server closes the connection in every case except a client close.

## Receipts in memory (`receipts.rs`)

- Newest **100** receipts within **32 MB**, counting raw bytes **and** the print model
  (text, bitmaps, a fixed overhead per block). The oldest **finished** receipt is dropped
  first; one still printing never is.
- Per connection: **16 MB** read (`Limits::max_connection_bytes`). Past it, or when
  printing receipts alone fill the 32 MB, the receipt ends `too_large` and the rest is not
  read. Memory stays bounded whatever the clients send.
- Consecutive feeds merge into one block.
- `ReceiptSummary` (`id`, `peer`, `started_at` / `ended_at` in unix ms, `state`, `cut`,
  `drawer`, `beeps`, `size`) is what the list gets. `ReceiptView` adds `paper`, `width` and
  the `blocks` to draw. Raw bytes stay in Rust (`Receipts::raw`, for the P3 export).

## Status in the UI

- Header badge (`src/app/listener-status/`): colored dot + "Listening on port N" /
  "Port N is in use" / "Port N is blocked" / "Can't open port N".
- While failed, a banner under the header says what to do ("Close the other app; the
  emulator starts on its own"): the bind retry makes it recover without a click.
- The tray's first item shows the same line (`flows/app-lifecycle.md`).

## Webview

`ReceiptsScreen` lists the receipts newest first: local time, client IP (`peerHost` drops
the port), size (`—` while printing) and the state as a colored dot plus text. Empty list:
"Waiting for receipts". The renderer (next P2 commit) draws them on paper. Bridge details
in `conventions/bridge.md`.

## Concurrency

- One Tokio task per connection on `tauri::async_runtime`. The connection cap is a
  `Semaphore`; a task's permit is released when it ends.
- `listener::Shared` holds `Mutex<Receipts>` and `Mutex<ListenerStatus>` (std mutexes:
  short sections, never held across `.await`). `listener::lock` recovers a poisoned lock
  instead of spreading a panic: no invariant spans a lock.
- Every change is sent on an `mpsc` channel (`Event::Receipts` with the whole list when a
  receipt starts or ends, `Event::Status`) **while the lock is held**, so events arrive in
  the same order as the changes.

## Logs

`listening`, `bind_failed`, `connection_refused`, `connection_rejected peer first_bytes`
(at most 4 bytes), `connection_accepted`, `receipt_started id peer`,
`receipt_ended id state cut bytes drawer beeps`, `connection_leftovers unprinted_bytes
unknown_commands`, `connection_ended peer state bytes receipts ms`, `read_failed`. Never
the receipt's bytes or text.

## Tests

- `src-tauri/tests/listener.rs`, real sockets on `127.0.0.1:0` with fast `Limits`: a
  receipt until EOF, cuts splitting one connection (raw bytes per receipt), printing
  announced, status-only connections, idle timeout, byte limit, connection cap, rejected
  protocols, an `ESC` split from its `@`, bind retry while the port is taken, status
  replies read back from the socket.
- `capture.rs` units: one receipt per cut, the same split for every chunking, status
  queries and bare cuts, a drawer pulse alone, text without `LF`, the store limit.
- `receipts.rs` units: eviction, limits, finished receipts frozen. `classify` units in
  `listener.rs`.
- Before trusting a change here, run the integration binary several times in parallel: it
  was 60/60 green when written.
