# Print job

From the POS's TCP connection to receipts in the list. Decisions 1–3 and 5 in `stack.md`
are the product rules behind this flow; the decoder itself is in `conventions/escpos.md`.

```
POS ─TCP─▶ listener.rs ─ classify ─▶ capture.rs ─ Decoder ─▶ receipts.rs ─ changed ─▶ webview
           (bind, accept,  (first     (bytes →      (parser +    (memory,
            limits)         bytes)     receipts)     printer)     limits)
```

## Listener (`listener.rs`)

```
listener::run(settings.addr(), Limits::PRODUCTION, shared)   (commands::restart_listener)
  bind ── fails ──▶ status Failed { port, error } ─▶ retry every 3 s (logged once per change)
   │
   └─ ok ──▶ status Listening { port } ─▶ accept loop
                 │
                 ├─ 16 connections open already ─▶ close the new one (logged)
                 └─ else: one task per connection ─▶ receive()
```

- `listener::bind` never shares a port another app holds. `SO_REUSEADDR` is what lets a
  rebind succeed while our old connections sit in TIME_WAIT (30–60 s after a restart or a
  relaunch, which would otherwise show a false "port in use"), but on macOS it also lets a
  wildcard bind share a port another app listens on at one address (or the reverse), and
  the app would say "Listening" while the other one got the jobs. So: on Linux it is
  always on (Linux never allows that sharing); on macOS the bind first goes without it and
  retries with it only when nothing answers on the port (loopback probe: what holds it is
  TIME_WAIT); Windows needs neither. Both cases are tested.
- Connection tasks live in a `JoinSet` owned by `run`: aborting the listener task (a
  restart) aborts every open connection too, so "only this computer" applies at once.
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

Each accepted connection gets a `Capture` with its own `Decoder`, set up with the paper
width and default code page in the settings at that moment. Bytes are read in 8 KiB
chunks and fed to it.

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
| The task stops (restart, panic)          | `connection_error` |
| Over a size limit (see below)            | `too_large`        |
| Reset or another read error              | `connection_error` |

The server closes the connection in every case except a client close. A connection that
sends nothing gets 10 s (`first_bytes_timeout`) instead of 5 min, so silent connections
cannot hold the 16 slots. A task that stops before `finish` (aborted by a restart, or a
panic) still ends its receipt: `Capture`'s `Drop` closes it as `connection_error`, so it
never stays "printing" and in memory.

## Receipts in memory (`receipts.rs`)

- Newest **100** receipts within **32 MB**, counting raw bytes **and** the print model
  (text, bitmaps, a fixed overhead per block). The oldest **finished** receipt is dropped
  first; one still printing never is.
- Per receipt: **16 MB** (`Limits::max_receipt_bytes`), counted from the end of the
  previous receipt (`Capture::open_bytes`), so a POS that keeps one connection open for
  many receipts is never cut off. Past it, or when printing receipts alone fill the 32 MB,
  the receipt ends `too_large` and the rest is not read.
- Memory is bounded: 32 MB of receipts, plus up to 16 MB per open connection waiting to
  become a receipt (bytes before the first output, an unfinished command), times 16
  connections.
- Consecutive feeds merge into one block.
- `ReceiptSummary` (`id`, `peer`, `started_at` / `ended_at` in unix ms, `state`, `cut`,
  `drawer`, `beeps`, `size`, `paper`, `width`, `height`) is what the list gets.
  `ReceiptView` adds the `blocks` to draw. Raw bytes stay in Rust (`Receipts::raw`, read by
  "Save .bin").

## Status in the UI

- The top bar's left side (`src/app/status-bar/`) says whether and where the emulator
  listens, in one line: "● Point your POS at `192.168.1.20:9100`" with a Copy button (LAN,
  from `get_lan_address`), "Only this computer can print, at `127.0.0.1:9100`"
  (`bind: local`), "No network found…" with `127.0.0.1`, "Starting…", or "● Port 9100 is in
  use" / "is blocked" / "Can't open port".
- While failed, a banner under the bar says what to do ("Close the other app; the emulator
  starts on its own"): the bind retry makes it recover without a click.
- The tray's first item shows the same status (`flows/app-lifecycle.md`).
- The empty list's hint ("Send ESC/POS jobs to `ip:port`") and its Print test receipt button
  use the same address and are hidden while the port is not open, so they never contradict
  the banner.

## Webview: receipts on paper

`ReceiptsScreen` shows the receipts **oldest first, newest at the bottom**, like a paper
roll, each as a `ReceiptCard`: a header (local time, client IP, size, state when not
`done`, "Drawer opened" / "Beep ×n" badges) and the paper, white in both themes, with a
torn edge when the receipt was cut. Drawing details in `design-system.md` § Receipt
rendering.

- **Following the end** (`use-follow-bottom.ts`): while the user is at the end (within
  64 px), the list follows new paper; once they scroll up to read, it stays put and a pill
  ("↓ New receipt" / "↓ 3 new receipts") counts what arrived; clicking it goes to the end.
  The list opens at the end. Arrivals are counted by id (ids only grow), so the count is
  right at the 100-receipt limit too. When the oldest receipts are dropped while the user
  reads above the end, the list scrolls by their height so the receipt being read stays
  put (each card's `offsetTop` is compared with the last change's, `data-receipt-id`).
  Clearing (the list unmounts) starts over: at the end, nothing unseen. The scroller has `overflow-anchor: none`: it positions itself,
  and the browser's scroll anchoring must not move it too. Scroll writes go through a ref
  (the React Compiler forbids mutating a state-held element).
- **Paper feed**: a receipt that finishes while the window is open (`ended_at` after the
  screen opened) is revealed top to bottom (`.paper-print`, a `clip-path` animation), and,
  when the user is at the end, the list scrolls to the end with the **same duration and
  curve** (`paper-feed.ts`: about 1200 dots/s like a thermal printer, clamped to
  300–1400 ms; `easeOutCubic` = `--ease-out-cubic`). The printed edge then stays on the
  window's bottom edge, like paper leaving a printer's slot. Scroll events caused by the
  feed are ignored; a wheel, touch, pointer press (a scrollbar drag) or key from the user
  stops it. More paper arriving
  mid-feed retargets it. Reduced motion: no scroll animation, a 150 ms fade instead of the
  reveal. Receipts already there when the window opens never animate.
- Canvases get their CSS size at render (`Slice`), not when drawn: a canvas without one is
  300×150 until its effect runs, and that brief shrink moved the scroll position.

- **Only what is near the visible area is drawn** (`useNearViewport`, an
  `IntersectionObserver` on the scrolling section with a 1500 px margin), per slice: a
  receipt is drawn in slices of at most 4096 device pixels (`sliceHeight`: fewer dots at a
  higher pixel ratio or zoom), and a slice far away keeps its size with a 0×0 canvas, its
  pixels given back; a card far away has no slices at all. Placeholders keep the
  receipt's `width × height`, so scrolling does not jump (the "too tall" and "no longer
  in memory" notes are the exceptions). Past 200,000 dots (25 m of paper,
  `MAX_DRAWN_HEIGHT`) the rest is not drawn and a note says so: a hostile job can't make
  the window create thousands of canvases. Slices are keyed by index, so a zoom redraws
  them in place instead of remounting them (a remount paints one blank frame).
- Images are decoded only for the rows inside the slice being drawn (`visibleRows`), so a
  tall image costs its slices, not its whole height each time.
- A receipt's print model is fetched (`get_receipt`) once it is no longer `printing`, the
  first time its card comes near, and kept by the card: scrolling back draws it again
  without another fetch. While printing, the card shows an empty strip: there are no
  progress events.
- A receipt that finishes while the window is open is announced to screen readers ("Receipt
  printed at 10:08:12", a `role="status"` line in `App`). The list is focusable (`tabIndex` 0) so the keyboard can scroll it; when it replaces the empty state, focus moves to it
  instead of falling to the page.
- A receipt dropped from memory (`get_receipt` → `null`) shows "No longer in memory".

## History tools

- **Clear** (trash icon in the top bar, shown when there are receipts) asks first, in a
  native modal `<dialog>` ("Clear all receipts?", Cancel focused, Esc closes), then drops
  every finished receipt (`Receipts::clear`, the total is recomputed); one still printing
  stays. A modal fits here: it confirms a destructive action, and an inline "Clear all?"
  label widened the bar until it wrapped.
- **Copy text** (copy icon in each finished receipt's header): the receipt as plain text
  (`receipt-text.ts`, tested): each segment at its column in font A columns, so
  right-aligned prices stay right-aligned; feeds become blank lines, images
  `[image W×H]`; barcodes keep their human-readable text. For bug reports and test
  assertions.
- **Show commands** (each finished receipt's header): a table under the paper, scrolled
  into view when it opens, with every
  ESC/POS command of the receipt: offset, mnemonic, what it does (translated) and bytes
  (`commands-panel/`, `conventions/escpos.md` § Inspect). Text is decoded in the code page
  the receipt started with (each receipt keeps it, `CodePages`), not today's setting.
- **Save .bin** (download icon in each finished receipt's header): writes its raw bytes to
  `Downloads/receipt-<started_at>-<id>.bin` and reveals the file (`opener`, Rust side,
  written on a blocking thread). A receipt that does not open with `ESC @` (any but the
  first of a connection) gets `ESC @ ESC t n` in front (`commands::replayable`): without it the
  connection filter would turn the replay away, and an earlier `ESC t` would be lost. So
  every saved file replays on its own (`nc 127.0.0.1 9100 < receipt.bin`), with the code
  page it printed with; other state an earlier receipt set (alignment, sizes) starts from
  the defaults, as on any fresh connection.

## Unseen receipts

Receipts that finish while the window is not in front (`document.hasFocus()` false) are
counted (`use-unseen-badge.ts`, same "newly finished" rule as the sounds) and shown with
`set_unseen`: a badge on the Dock icon (macOS, some Linux docks), the count next to the
menu bar icon on macOS (the Dock icon is hidden while the window is closed), and the tray
tooltip ("Thermal Printer Emulator · 3 new") everywhere. Windows has no badge count. The
count clears when the window gets focus.

## Zoom

The paper zoom setting (75–200 %) scales the receipts: the canvas is redrawn at the new
scale (`drawSlice(…, scale)`), never stretched, so codes and font B stay sharp. Margins,
widths and placeholders scale with it; the feed's speed stays in printer dots
(`feedDuration(distance / scale)`). A receipt wider than the window scrolls sideways.

## Sound

When a receipt finishes and the `sound` setting is on, the webview plays
(`src/shared/utils/sounds.ts`, WebAudio, no sound file):

- **the printing sound** for every receipt: band-passed noise pulsing at 38 Hz (the print
  head and its stepper motor), quiet, lasting as long as the paper takes to come out
  (`feedDuration` of the receipt's height, the same as the feed animation);
- then **the beep** when the POS asked for it with `ESC B`: a 2.7 kHz buzzer, 120 ms per
  beep, at most 3.

`pendingSounds` (tested) decides: each receipt sounds once, after it stops printing;
receipts already in the list when the screen opens never sound. WebKit lets the
`AudioContext` resume without a user gesture (checked: `suspended` → `running`), and
sounds play with the window behind others.

## Test receipt (`test_receipt.rs`)

"Print test receipt" (tray, the empty list, the top bar) builds a sample
receipt and **sends it through our own socket** to `127.0.0.1:<port>`, like a POS would,
so it proves the listener is up. It then shows up in the list like any receipt.

- Content: title, "the emulator is working", port, paper and code page, a line of accents
  and symbols (`Ação Café Ñandú Ü € £ ½ ─│┌┐`), bold, underline, reverse, font B, a
  CODE128 with HRI, a QR code to the repository, partial cut.
- Text is in the UI language (`Strings.test_*` in `locale.rs`) and encoded in the
  **configured default code page** (`CodePage::encode`, `?` for missing characters), and
  the rules fit the paper (48 or 32 columns). With CP437 the accents line prints `Aç?o`
  and `?` for `€`: the receipt shows why the code page setting matters.
- Errors: not listening → `testReceipt.errors.notListening`; connect/write failure (5 s
  timeout) → `testReceipt.errors.send`, logged as `test_receipt_failed`.
- Unit tests decode the built bytes with our own decoder: no unknown command, a barcode
  and a QR, the cut, the code page and paper honored.

## Concurrency

- One Tokio task per connection on `tauri::async_runtime`. The connection cap is a
  `Semaphore`; a task's permit is released when it ends.
- `listener::Shared` holds `Mutex<Receipts>` and `Mutex<ListenerStatus>` (std mutexes:
  short sections, never held across `.await`). `listener::lock` recovers a poisoned lock
  instead of spreading a panic: no invariant spans a lock.
- After a change (a receipt starts or ends, the list is cleared, the status changes) the
  code calls `Shared::changed`, a `Notify`. The forwarder (`shell::forward_changes`) wakes,
  reads the current list and status, emits only what differs from what it last sent, then
  pauses 50 ms. Changes made meanwhile become one update: a client printing receipts as
  fast as it can never queues memory (an unbounded channel of whole lists did, measured
  in GB) and the webview renders at most 20 times a second.

## Logs

`listening`, `bind_failed`, `connection_refused`, `connection_rejected peer first_bytes`
(at most 4 bytes), `connection_accepted`, `receipt_started id peer`,
`receipt_ended id state cut bytes drawer beeps`, `connection_leftovers unprinted_bytes
unknown_commands`, `connection_ended peer state bytes receipts ms`, `read_failed`. Never
the receipt's bytes or text.

## Tests

- `src-tauri/tests/listener.rs`, real sockets on `127.0.0.1:0` with fast `Limits`: a
  receipt until EOF, cuts splitting one connection (raw bytes per receipt), printing
  announced, status-only connections, idle timeout, a silent connection closed early,
  the byte limit (and that it counts each receipt), connection cap, rejected protocols, an
  `ESC` split from its `@`, bind retry while the port is taken, no sharing of a port
  another app holds, stopping the listener ends its connections, status replies read back
  from the socket.
- `capture.rs` units: one receipt per cut, the same split for every chunking, status
  queries and bare cuts, a drawer pulse alone, text without `LF`, the store limit.
- `receipts.rs` units: eviction, limits, finished receipts frozen, height equal to the
  sum of the blocks (merged feeds saturate per block). `classify` units in
  `listener.rs`.
- Before trusting a change here, run the integration binary several times in parallel: it
  was 60/60 green when written.
