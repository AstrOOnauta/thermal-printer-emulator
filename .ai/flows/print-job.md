# Print job

From the POS's TCP connection to a job in the list. Decisions 1–3 in `stack.md` are the
product rules behind this flow.

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

A job is created only when the connection opens like ESC/POS:

| First bytes          | Verdict                                                     |
| -------------------- | ----------------------------------------------------------- |
| `ESC @` (initialize) | accept: every ESC/POS library sends it first                |
| `DLE …` / `GS …`     | accept: drivers that open with a status query or a command  |
| a lone `ESC`         | wait for the next byte (TCP may split `ESC` from `@`)       |
| anything else        | reject: close the connection, no job, `connection_rejected` |

The bytes read before the verdict are kept and become the start of the job. Rejected on
purpose: HTTP, TLS, PJL and its UEL (`ESC %-12345X`), Redis, null-byte banner grabs.

**Trade-off**: plain text (`echo hi | nc host 9100`) and a job that opens with another
command (e.g. `ESC t` before `ESC @`) are rejected too. Real ESC/POS software always
initializes first, and the README says so. If a real POS trips this, revisit the
decision; don't add a blocklist of protocols.

## Connection → job (`receive`)

- The job starts with the first bytes that pass the filter. A connection that closes
  without sending anything, or is rejected, leaves no job.
- Bytes are read in 8 KiB chunks and appended to the job in `jobs.rs`.
- The job ends when:

| Event                                  | `JobState`         |
| -------------------------------------- | ------------------ |
| The client closes the connection (EOF) | `done`             |
| No byte for 5 min (`idle_timeout`)     | `idle_timeout`     |
| Over the size limits (see below)       | `too_large`        |
| Reset or another read error            | `connection_error` |

The server closes the connection in every case except `done`.

- Until the parser exists (P2), **one connection = one job**. P2 also ends a receipt at a
  cut command (decision 1).

## Jobs in memory (`jobs.rs`)

- Newest **100** jobs within **32 MB** in total; the oldest **finished** job is dropped
  first. A job being received is never dropped.
- Per job: **16 MB**. When a job would go over its own limit, or receiving jobs alone fill
  the 32 MB, the job ends `too_large` and the rest is not read. So memory stays bounded
  whatever the clients send: about 32 MB of jobs plus 16 × 8 KiB of read buffers.
- `JobSummary` (`id`, `peer`, `started_at` / `ended_at` in unix ms, `state`, `size`) is what
  leaves Rust. Never the bytes.

## Status in the UI

- Header badge (`src/app/listener-status/`): colored dot + "Listening on port N" /
  "Port N is in use" / "Port N is blocked" / "Can't open port N".
- While failed, a banner under the header says what to do ("Close the other app; the
  emulator starts on its own"): the bind retry makes it recover without a click.
- The tray's first item shows the same line (`flows/app-lifecycle.md`).

## Webview (P1)

`JobsScreen` lists the jobs newest first: local time, client IP (`peerHost` drops the
port), size (`—` while receiving: there are no progress events) and the state as a colored
dot plus text. Empty list: "Waiting for print jobs". P2 replaces this screen with the
rendered receipts. Bridge details in `conventions/bridge.md`.

## Concurrency

- One Tokio task per connection on `tauri::async_runtime`. The connection cap is a
  `Semaphore`; a task's permit is released when it ends.
- `listener::Shared` holds `Mutex<Jobs>` and `Mutex<ListenerStatus>` (std mutexes: short
  sections, never held across `.await`). `listener::lock` recovers a poisoned lock instead
  of spreading a panic: no invariant spans a lock.
- Every change is sent on an `mpsc` channel (`Event::Jobs` with the whole list,
  `Event::Status`) **while the lock is held**, so events arrive in the same order as the
  changes.

## Logs

`listening`, `bind_failed`, `connection_refused`, `connection_rejected peer first_bytes`
(at most 4 bytes, never job content), `job_started id peer`,
`job_ended id peer state bytes ms`, `read_failed`. Never the job's bytes.

## Tests

`src-tauri/tests/listener.rs` against real sockets on `127.0.0.1:0` with fast `Limits`:
a job until EOF, start announced, empty connection, idle timeout, size limit, connection
cap, bind retry while the port is taken, rejected protocols, a status-query opening, an
`ESC` split from its `@`. `classify` has unit tests in `listener.rs`. `jobs.rs` unit tests cover eviction and the
limits. Before trusting a change here, run the test binary several times in parallel: it
was 60/60 green when written.
