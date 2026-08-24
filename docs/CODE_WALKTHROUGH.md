# BIFUR Code Walkthrough

## 1. Workspace boundaries

BIFUR is split into three crates:

- `crates/core`: pure Rust domain/infrastructure code. **No GPUI dependency is allowed here.**
- `crates/gpui_app`: macOS-first GPUI frontend.
- `crates/bridge`: `flutter_rust_bridge` boundary for a future Flutter frontend.

The frontend is intentionally replaceable. File operations, preview decisions, terminal ownership, parser state, and command history belong in core.

## 2. File model

`crates/core/src/fs_model.rs` contains `PaneState` and `FileEntry`.

`PaneState::read_dir` reads a directory, builds serializable entries, and sorts directories before files. `enter()` and `up()` mutate the current directory and refresh the list. This is synchronous in M0; large-directory background loading is a later milestone.

## 3. Preview

`crates/core/src/preview.rs` maps a selected path into `PreviewKind` using `mime_guess`. Text is capped before returning to a frontend, images are tagged for image rendering, and unknown content is treated as binary.

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

`crates/core/src/terminal/parser.rs` is the UI-neutral terminal surface. The M0 parser supports printable text, CR/LF, backspace, tab, scrolling, and suppresses basic CSI escape sequences from visible output.

The parser is intentionally replaceable. Full VT100/xterm cursor and style support can be added later without changing the frontend contract.

### CommandBlock

`crates/core/src/terminal/history.rs` stores command, output, cwd, exit code, and timestamp. Keeping this model from the beginning enables future AI features to work on structured command blocks instead of reconstructing history from rendered terminal text.

## 5. GPUI terminal renderer

`crates/gpui_app/src/terminal_view.rs` depends on `ScreenBuffer`, not on `portable-pty`. It converts the snapshot into GPUI elements and displays the active cwd.

The M0 app spawns `TerminalSession` at the initial pane directory. Keyboard forwarding, focus, terminal resize, repaint notification, and active-pane cwd synchronization are M1 work.

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
