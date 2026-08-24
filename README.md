# BIFUR

BIFUR is a dual-pane file manager inspired by Path Finder, built in Rust with a GPUI frontend.
The name comes from **bifurcate**: split into two.

## Architecture

```text
crates/gpui_app                    crates/core                    OS
┌─────────────────┐       ┌────────────────────────┐      ┌──────────────┐
│ GPUI UI         │──────▶│ file model / preview   │─────▶│ filesystem   │
│ TerminalView    │──────▶│ TerminalSession        │─────▶│ portable-pty │
└─────────────────┘       │ ScreenBuffer / history │      │ native PTY   │
                          └────────────────────────┘      └──────────────┘
                                   ▲
                                   │
                          crates/bridge (Flutter)
```

`crates/core` must stay pure Rust and must never depend on GPUI. This keeps the engine reusable by a future Flutter frontend through `flutter_rust_bridge`.

## Current baseline

- Dual-pane state and directory listing
- File preview classification
- Batch rename preview logic
- Pure-Rust terminal session using `portable-pty::NativePtySystem`
- Core-owned PTY reader thread and `ScreenBuffer`
- Cross-platform cwd command escaping
- `CommandBlock` history model for future AI features
- GPUI terminal renderer consumes only `ScreenBuffer`
- Flutter bridge skeleton

## Run

```bash
cargo run -p bifur
```

Useful checks:

```bash
cargo fmt --all -- --check
cargo test -p bifur-core
cargo check --workspace
```

## Terminal invariants

1. `GPUI TerminalView -> core::terminal::TerminalSession -> portable-pty`
2. PTY ownership and output parsing stay in `core`
3. `NativePtySystem` is required so Windows can use ConPTY
4. Active-pane path changes must call `TerminalSession::set_cwd`
5. Command history remains modeled as `CommandBlock` for future AI/block workflows

See `docs/CODE_WALKTHROUGH.md` and `docs/PROJECT_HANDOFF.md` for implementation details and next steps.
