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
9. Frontend-specific key/event types stay outside core. Terminal byte protocol encoding belongs in core.

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

## M1 delivered

- keyboard dual-pane navigation and terminal cwd synchronization
- `F6` pane-vs-terminal input mode
- event-driven terminal repaint from PTY activity
- terminal resize from GPUI window bounds
- core-owned modifier-aware navigation encoding
- core-owned Ctrl/control-sequence encoding
- GPUI input policy regression harness
- production GPUI terminal input routed through the tested policy
- AltGr preservation for letter, digit, and punctuation layouts
- macOS Option/Meta transformed-glyph fallback
- pre-push formatting and preflight guardrails via `make setup-hooks`

## Current M1 branch

Branch: `feature/m1-pane-refresh-model`

Current slice:

- add core pane refresh semantics before filesystem watcher wiring
- preserve the selected entry by lossless path when a directory refreshes
- clamp selection safely when the selected entry disappears
- keep background I/O orchestration in the frontend while state transition semantics remain in core

## M1 remaining work

1. Wire `notify` to pane refresh without blocking GPUI render.
2. Move directory reads triggered by watcher events off the UI/render path and apply results through `PaneState::replace_entries()`.
3. Expand parser toward VT100/xterm semantics while preserving the `ScreenBuffer` API.
4. Pin a known-good GPUI revision once macOS validation is complete.
5. Update code walkthrough after watcher/refresh architecture lands.

## Current limitations

- file watching is not wired yet
- initial directory loading and explicit Enter/Backspace navigation still perform synchronous directory reads
- parser coverage is still intentionally smaller than full VT100/xterm behavior
- Windows PTY architecture is present but not yet validated with the GPUI frontend

## Validation commands

```bash
make format-check
make preflight
```

Equivalent CI checks include:

```bash
cargo fmt --all -- --check
cargo test -p bifur-core
cargo check -p bifur
cargo check -p bifur-bridge
```

Run `make setup-hooks` once per clone so formatting/preflight runs before every push.

## Definition of done for M1

- Tab switches active pane.
- Selection navigation works without mouse.
- Enter opens a directory; Backspace goes up.
- Terminal cwd follows the active pane after every navigation/switch.
- Terminal accepts keyboard input and displays shell output.
- PTY output triggers repaint without blocking the UI thread.
- Terminal resize follows GPUI bounds.
- Pane contents react to filesystem changes without blocking render.
- Core tests stay green and core still has zero GPUI dependencies.
- Handoff and code walkthrough are updated with architectural changes.
