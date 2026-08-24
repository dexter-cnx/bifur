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
8. Filesystem paths remain lossless `PathBuf` values in core; lossy conversion is display/bridge-only.

## M0 status

M0 was merged to `main` through PR #1.

Delivered:

- Rust workspace: `core`, `gpui_app`, `bridge`
- `NativePtySystem` terminal session
- core-owned PTY reader/parser and `ScreenBuffer`
- cross-platform cwd escaping
- `CommandBlock` history model
- GPUI `TerminalView`
- Flutter bridge baseline
- bounded text preview
- lossless internal filesystem paths
- incremental UTF-8 handling across PTY reads
- code walkthrough + handoff + CI

## M1 status

PR #2 delivered keyboard dual-pane navigation and terminal cwd synchronization.

PR #3 delivered terminal input mode:

- `F6` toggles pane-vs-terminal input
- Enter, Backspace, Tab, Escape, arrows, Space, and printable text forward to `TerminalSession::send_input()`
- printable input uses GPUI `key_char`, preserving shifted characters and non-US keyboard layouts
- terminal input errors surface in the GPUI status strip

## Current M1 branch

Branch: `feature/m1-terminal-repaint`

Current hardening slice:

- CI now compiles `bifur` GPUI frontend instead of validating only `bifur-core`
- CI also compiles `bifur-bridge`
- Windows terminal shell selection checks `SHELL`, then `COMSPEC`, then falls back to `pwsh.exe`

This slice intentionally makes frontend/bridge compile failures visible before deeper PTY repaint and resize work lands.

## M1 remaining work

1. Add modifier-aware terminal input mapping, including Ctrl/Alt combinations.
2. Add event-driven GPUI invalidation when the PTY reader updates `ScreenBuffer`.
3. Resize PTY/`ScreenBuffer` from terminal view bounds.
4. Wire `notify` to pane refresh without blocking render.
5. Expand parser toward VT100/xterm semantics while preserving `ScreenBuffer` API.
6. Pin a known-good GPUI revision once macOS validation is complete.

## Current limitations

- Ctrl/Alt terminal combinations are not forwarded yet
- PTY output does not yet trigger event-driven repaint
- terminal resize is not connected to GPUI bounds
- file watching is not wired
- directory loading remains synchronous
- Windows PTY architecture is present but not yet validated with the GPUI frontend

## Validation commands

```bash
cargo fmt --all -- --check
cargo test -p bifur-core
cargo check -p bifur
cargo check -p bifur-bridge
```

Format before every push. CI must compile all touched runtime boundaries, not only core.

## Definition of done for M1

- Tab switches active pane.
- Selection navigation works without mouse.
- Enter opens a directory; Backspace goes up.
- Terminal cwd follows the active pane after every navigation/switch.
- Terminal accepts keyboard input and displays shell output.
- PTY output triggers repaint without blocking the UI thread.
- Terminal resize follows GPUI bounds.
- Core tests stay green and core still has zero GPUI dependencies.
- Handoff and code walkthrough are updated with architectural changes.
