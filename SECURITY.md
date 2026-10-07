# Security

## Reporting a vulnerability

Please report security issues privately, not in a public issue: use
[**Report a vulnerability**](https://github.com/AstrOOnauta/thermal-printer-emulator/security/advisories/new)
on GitHub. Include what you found, how to reproduce it, and the app version and OS.
You should get an answer within a week.

## What the app exposes

The emulator listens on a TCP port (9100 by default) on **every network interface**, so
POS terminals on other machines can print to it. Anyone who can reach that port can send
it data. By design:

- It only decodes ESC/POS print jobs. Connections that open with anything else (HTTP,
  TLS, PJL, plain text) are closed.
- What it receives is drawn on screen and kept in memory, never run, never written to
  disk unless you save a receipt yourself. Logs record sizes and ids, never receipt
  content.
- Memory and connections are bounded: at most 16 connections, 16 MB per receipt, 32 MB
  of receipts, and idle connections are closed.
- The window cannot reach the network or the file system: only the Rust core does.

To keep it off the network entirely, pick **"only this computer"** in Settings: it then
listens on `127.0.0.1` only.

## Supported versions

Only the latest release gets fixes.
