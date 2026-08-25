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
10. Filesystem watcher lifecycle and GPUI scheduling stay in the frontend; reusable pane state transitions stay in core.

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
- core `PaneState::replace_entries()` refresh semantics with selection preservation and stale-source rejection
- GPUI-owned `notify` watcher lifecycle for left/right panes
- watcher-triggered directory reads moved to GPUI background executor work
- production snapshot application through core stale-source validation
- re-watch after Enter/Backspace with an authoritative post-watch refresh to close the read/watch gap
- burst coalescing so queued duplicate watcher events cause at most one refresh per pane per batch

## Current M1 branch

Branch: `feature/m1-pane-refresh-coalescing`

Current slice:

- coalesce bursty watcher signals without timer-based latency
- preserve independent left/right refresh requests
- keep events arriving during an active directory read available for the next refresh cycle
- synchronize handoff and code walkthrough with the delivered asynchronous pane refresh architecture

## M1 remaining work

1. Expand parser toward VT100/xterm semantics while preserving the `ScreenBuffer` API.
2. Pin a known-good GPUI revision once macOS validation is complete.
3. Validate watcher/refresh behavior on a physical macOS run with rapid create/delete/rename activity.
4. Decide whether synchronous initial Enter/Backspace directory reads should later become fully asynchronous; watcher-triggered refresh is already off the UI/render path.

## Current limitations

- initial directory loading and explicit Enter/Backspace navigation still perform synchronous directory reads
- parser coverage is still intentionally smaller than full VT100/xterm behavior
- Windows PTY architecture is present but not yet validated with the GPUI frontend
- watcher behavior is production-wired but still needs physical burst validation on macOS

## Validation commands

```bash
make format-check
make preflight
```

Equivalent CI checks include:

```bash
cargo fmt --all -- --check
cargo test -p bifur-core
cargo test -p bifur --lib
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
- Bursty watcher activity does not cause redundant duplicate refreshes for the same pane.
- Core tests stay green and core still has zero GPUI/notify dependencies.
- Handoff and code walkthrough are updated with architectural changes.
