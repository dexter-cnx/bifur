# BIFUR Project Handoff

## Product

**BIFUR** is a Path Finder-inspired dual-pane file manager. The name derives from *bifurcate* (split into two), matching the dual-pane interaction model.

Primary frontend: GPUI on macOS first. Future frontend: Flutter, reusing `crates/core` through `crates/bridge`.

## Non-negotiable architecture rules

1. `crates/core` is pure Rust and must not depend on GPUI.
2. Terminal layering is `TerminalView -> TerminalSession -> portable-pty`.
3. `TerminalSession` owns the PTY, parser state, and command history.
4. Frontends render `ScreenBuffer`; they do not parse raw PTY bytes.
5. Use `NativePtySystem` for cross-platform support, including Windows ConPTY.
6. When active pane/cwd changes, synchronize the running terminal with `session.set_cwd()`.
7. Keep `CommandBlock` history as a first-class core model for future AI functionality.

## Implemented baseline

### Core

- directory listing and dual-pane state
- directory enter/up navigation primitives
- preview classification
- batch rename preview
- native PTY session spawn
- dedicated PTY reader thread
- core-owned `ScreenBuffer`
- terminal resize/input/cwd APIs
- POSIX/PowerShell/cmd cwd escaping
- command-block history storage
- parser and cwd unit tests

### GPUI

- dual-pane visual shell
- preview column
- terminal renderer that consumes `ScreenBuffer`
- terminal session bootstrapped at the initial pane cwd

### Flutter bridge

- initial FRB crate
- file listing API
- batch rename preview API

## Current limitations

- GPUI keyboard actions are not wired yet.
- Active-pane switching is not yet calling `TerminalSession::set_cwd()` because pane interaction handlers are still pending.
- Terminal parser is intentionally minimal; full VT100/xterm cursor/style semantics are not implemented.
- `CommandBlock` exists but command-boundary detection is not wired to shell integration yet.
- File watching via `notify` is not wired to pane refresh yet.
- Directory loading is synchronous.
- Windows is architecturally supported by `NativePtySystem`, but GPUI/Windows validation remains future work.

## Next milestone: M1 interactive dual pane + terminal

Recommended order:

1. Add GPUI focus model and actions: Tab, Up/Down or j/k, Enter, Backspace.
2. Make active-pane changes call one method that also invokes `terminal.set_cwd(active_path)`.
3. Forward terminal-focused key input to `TerminalSession::send_input()`.
4. Add event-driven GPUI invalidation so new `ScreenBuffer` output repaints without blocking the UI thread.
5. Add terminal resize based on GPUI bounds.
6. Replace the minimal parser with a VT100/xterm-compatible parser while preserving `ScreenBuffer` API.
7. Wire `notify` to refresh pane contents without blocking the render path.
8. Pin a known-good GPUI revision once the first macOS build is validated.

## Validation commands

```bash
cargo fmt --all -- --check
cargo test -p bifur-core
cargo check --workspace
```

Format before every push. If GPUI upstream breaks, isolate the frontend failure and keep `bifur-core` green.

## Definition of done for M1

- Tab switches active pane.
- Selection navigation works without mouse.
- Enter opens a directory; Backspace goes up.
- Terminal cwd follows the active pane after every navigation/switch.
- Terminal accepts keyboard input and displays shell output.
- Core tests stay green and core still has zero GPUI dependencies.
- Handoff and code walkthrough are updated with architectural changes.
