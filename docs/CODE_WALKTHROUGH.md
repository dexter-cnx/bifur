# BIFUR Code Walkthrough

## 1. Workspace boundaries

BIFUR is split into three crates:

- `crates/core`: pure Rust domain/infrastructure code. **No GPUI dependency is allowed here.**
- `crates/gpui_app`: macOS-first GPUI frontend.
- `crates/bridge`: `flutter_rust_bridge` boundary for a future Flutter frontend.

The frontend is intentionally replaceable. File operations, preview decisions, terminal ownership, parser state, and command history belong in core.

## 2. File model

`crates/core/src/fs_model.rs` contains `PaneState` and `FileEntry`.

`PaneState::read_dir` reads a directory, builds serializable entries, and sorts directories before files. Paths remain `PathBuf` internally so Unix non-UTF-8 filenames stay lossless.

M1 adds frontend-neutral selection primitives:

- `select_next()`
- `select_previous()`
- `enter() -> bool`
- `up() -> bool`

The boolean result lets a frontend repaint or synchronize dependent state only when navigation actually changes something.

## 3. Preview

`crates/core/src/preview.rs` maps a selected path into `PreviewKind` using `mime_guess`. Text previews use a bounded prefix read rather than loading an entire large file into memory. Images are tagged for image rendering, and unknown content is treated as binary.

## 4. Terminal architecture

```text
GPUI TerminalView
      │ ScreenBuffer snapshot
      ▼
core::terminal::TerminalSession
      │ owns PTY + writer + reader thread
      ▼
portable-pty::NativePtySystem
      │
      ├─ Unix PTY on macOS/Linux
      └─ ConPTY on Windows
```

### TerminalSession

`crates/core/src/terminal/session.rs` owns the native PTY master, shell child, input writer, dedicated output reader thread, shared `ScreenBuffer`, and `CommandBlock` history.

The GPUI crate never reads raw PTY bytes. It calls `screen_snapshot()` and renders the returned core model.

`resize()` updates both native PTY dimensions and the screen model. `send_input()` writes to the running shell. `set_cwd()` sends an escaped shell-specific change-directory command and updates the session state.

### CWD safety

`crates/core/src/terminal/file_aware.rs` centralizes cwd command construction:

- POSIX shells: single-quote escaping
- PowerShell: `Set-Location -LiteralPath`
- cmd.exe: `cd /d`

This replaces the unsafe pattern of interpolating a path directly into `cd '...'`.

### ScreenBuffer

`crates/core/src/terminal/parser.rs` is the UI-neutral terminal surface. The parser supports printable text, CR/LF, backspace, tab, scrolling, and suppresses basic CSI escape sequences from visible output.

PTY reads are arbitrary byte chunks, so `ScreenBuffer` retains incomplete trailing UTF-8 bytes between reads. A split Thai/Unicode code point therefore survives intact instead of being replaced by U+FFFD.

The parser is intentionally replaceable. Full VT100/xterm cursor and style support can be added later without changing the frontend contract.

### CommandBlock

`crates/core/src/terminal/history.rs` stores command, output, cwd, exit code, and timestamp. Keeping this model from the beginning enables future AI features to work on structured command blocks instead of reconstructing history from rendered terminal text.

## 5. GPUI interaction layer

`crates/gpui_app/src/main.rs` owns only presentation and interaction state. `BifurApp` now has a GPUI `FocusHandle` and tracks focus on the application root.

The root receives unmodified key events:

- `Tab`: switch active pane
- `Down` / `j`: select next entry
- `Up` / `k`: select previous entry
- `Enter`: enter selected directory
- `Backspace`: move to parent directory

All directory-changing operations funnel through `sync_terminal_cwd()`. It copies the active pane `PathBuf` and calls `TerminalSession::set_cwd()`. This enforces the file-aware terminal rule without giving GPUI ownership of the PTY.

`crates/gpui_app/src/terminal_view.rs` still depends on `ScreenBuffer`, not on `portable-pty`.

## 6. Flutter bridge

`crates/bridge/src/lib.rs` exposes file listing and batch-rename preview as the first FRB APIs. Terminal support should later use opaque core handles/events and must not expose GPUI types.

## 7. Dependency direction

Allowed:

```text
gpui_app -> core
bridge   -> core
core     -> portable-pty / notify / walkdir / rayon / mime_guess / regex
```

Forbidden:

```text
core -> gpui
gpui_app -> bridge
core -> Flutter types
```

This dependency rule preserves the future Flutter frontend option.

## 8. M1 remaining work

The current M1 slice establishes pane focus/navigation and cwd synchronization. Remaining interactive-terminal work is:

1. terminal-vs-pane focus mode and raw key forwarding to `TerminalSession::send_input()`
2. event-driven repaint when the PTY reader updates `ScreenBuffer`
3. PTY resize from GPUI terminal bounds
4. `notify`-driven pane refresh outside the render path
5. fuller VT100/xterm semantics while preserving the `ScreenBuffer` API
