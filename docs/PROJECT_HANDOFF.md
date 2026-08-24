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

The first M1 slice was merged through PR #2.

Delivered:

### Core

- `PaneState::select_next()`
- `PaneState::select_previous()`
- `enter() -> bool`
- `up() -> bool`
- selection-boundary unit test

### GPUI

- root `FocusHandle`
- root keyboard event handling
- `Tab` switches active pane
- `Up/Down` and `j/k` move selection
- `Enter` opens the selected directory
- `Backspace` moves to the parent directory
- keyboard navigation reveals the selected row using `ScrollHandle`
- all pane/cwd transitions funnel through `sync_terminal_cwd()`
- active pane changes call `TerminalSession::set_cwd()`
- terminal cwd sync errors are surfaced instead of silently diverging

## Current M1 branch

Branch: `feature/m1-terminal-input`

Implemented in this slice:

- explicit pane-vs-terminal input mode
- `F6` toggles terminal input mode
- terminal-focused Enter, Backspace, Tab, Escape, arrows, Space, and single-character keys forward to `TerminalSession::send_input()`
- terminal input failures surface in the GPUI status strip
- terminal panel visually indicates terminal input mode

Modifier-aware terminal control sequences (for example Ctrl+C) are intentionally deferred until the input mapping is modeled explicitly rather than guessed from display strings.

## M1 remaining work

1. Add modifier-aware terminal input mapping, including Ctrl/Alt combinations.
2. Add event-driven GPUI invalidation when the PTY reader updates `ScreenBuffer`.
3. Resize PTY/`ScreenBuffer` from terminal view bounds.
4. Wire `notify` to pane refresh without blocking render.
5. Expand parser toward VT100/xterm semantics while preserving `ScreenBuffer` API.
6. Pin a known-good GPUI revision once macOS validation is complete.

## Current limitations

- terminal accepts the initial unmodified key set, but Ctrl/Alt combinations are not forwarded yet
- PTY output does not yet trigger event-driven repaint
- terminal resize is not connected to GPUI bounds
- file watching is not wired
- directory loading remains synchronous
- Windows PTY architecture is present but not yet validated with the GPUI frontend

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
- PTY output triggers repaint without blocking the UI thread.
- Terminal resize follows GPUI bounds.
- Core tests stay green and core still has zero GPUI dependencies.
- Handoff and code walkthrough are updated with architectural changes.
