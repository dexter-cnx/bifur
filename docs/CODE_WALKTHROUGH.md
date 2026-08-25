# BIFUR Code Walkthrough

## 1. Workspace boundaries

BIFUR is split into three crates:

- `crates/core`: pure Rust domain/infrastructure code. **No GPUI dependency is allowed here.**
- `crates/gpui_app`: macOS-first GPUI frontend.
- `crates/bridge`: `flutter_rust_bridge` boundary for a future Flutter frontend.

The frontend is intentionally replaceable. Reusable file-state transitions, preview decisions, terminal ownership, parser state, and command history belong in core. GPUI event normalization, filesystem watcher lifecycle, and repaint scheduling stay in the GPUI frontend.

## 2. File model

`crates/core/src/fs_model.rs` contains `PaneState` and `FileEntry`.

`PaneState::read_dir` reads a directory, builds serializable entries, and sorts directories before files. Paths remain `PathBuf` internally so Unix non-UTF-8 filenames stay lossless.

M1 adds frontend-neutral navigation and refresh primitives:

- `select_next()`
- `select_previous()`
- `enter() -> bool`
- `up() -> bool`
- `refresh()`
- `replace_entries(source_path, entries) -> bool`

`replace_entries` preserves the selected entry by lossless path when possible, clamps selection if the entry disappears, and rejects stale asynchronous snapshots when `source_path` no longer matches the pane's `current_path`.

## 3. Pane refresh architecture

Filesystem watching belongs to the GPUI frontend, not core.

```text
notify::RecommendedWatcher
        │ PaneSide signal
        ▼
PaneRefreshReceiver
        │ coalesces queued duplicate signals per pane
        ▼
GPUI task captures current PathBuf
        │
        ▼
background PaneRefreshRequest::read()
        │ PaneRefreshSnapshot
        ▼
PaneRefreshSnapshot::apply()
        │
        ▼
core PaneState::replace_entries()
        │ accepted current-source snapshot
        ▼
cx.notify()
```

`PaneWatcher` owns `notify::RecommendedWatcher` and emits only `PaneSide`. It never reads directories or mutates pane state.

`PaneRefreshCoordinator` owns both pane watchers and the refresh channel. Re-watching after Enter/Backspace installs the new watch first and then queues one authoritative refresh, closing the gap between the synchronous navigation read and watcher installation.

`PaneRefreshReceiver` drains already-queued watcher signals and collapses duplicates so a burst causes at most one pending refresh per pane. This is not timer-based debounce: there is no artificial delay, and events that arrive while a directory read is in progress remain queued for the next cycle.

`PaneRefreshRequest` carries only plain frontend data (`PaneSide + PathBuf`) into the background executor. No GPUI context and no mutable `PaneState` crosses into blocking filesystem I/O.

`PaneRefreshSnapshot::apply()` returns `true` only if core accepts the snapshot for the pane's still-current source path. GPUI repaints only after that acceptance.

## 4. Preview

`crates/core/src/preview.rs` maps a selected path into `PreviewKind` using `mime_guess`. Text previews use a bounded prefix read rather than loading an entire large file into memory. Images are tagged for image rendering, and unknown content is treated as binary.

## 5. Terminal architecture

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

## 6. GPUI interaction layer

`crates/gpui_app/src/main.rs` owns presentation and interaction state. `BifurApp` has a GPUI `FocusHandle`, terminal repaint task, pane refresh coordinator, and pane refresh task.

The root receives pane-mode key events:

- `Tab`: switch active pane
- `Down` / `j`: select next entry
- `Up` / `k`: select previous entry
- `Enter`: enter selected directory, re-watch the new path, and synchronize terminal cwd
- `Backspace`: move to parent directory, re-watch the new path, and synchronize terminal cwd
- `F6`: switch between pane input and terminal input

Terminal key translation is tested in the GPUI-side input policy module, while terminal byte-protocol encoding for control/navigation sequences stays in core.

`crates/gpui_app/src/terminal_view.rs` depends on `ScreenBuffer`, not on `portable-pty`.

## 7. Flutter bridge

`crates/bridge/src/lib.rs` exposes file listing and batch-rename preview as the first FRB APIs. Terminal support should later use opaque core handles/events and must not expose GPUI types.

The pane refresh architecture deliberately keeps reusable snapshot/state semantics in core while watcher scheduling remains frontend-specific, so a future Flutter frontend can use its own filesystem event mechanism and still call the same core transitions.

## 8. Dependency direction

Allowed:

```text
gpui_app -> core
gpui_app -> notify
bridge   -> core
core     -> portable-pty / walkdir / rayon / mime_guess / regex
```

Forbidden:

```text
core -> gpui
core -> notify
gpui_app -> bridge
core -> Flutter types
```

This dependency rule preserves the future Flutter frontend option.

## 9. M1 remaining work

Delivered interactive foundations now include pane navigation, terminal focus/input, PTY repaint/resize, and asynchronous watcher-driven pane refresh.

Remaining M1 work is:

1. physical macOS validation of rapid create/delete/rename watcher bursts
2. fuller VT100/xterm semantics while preserving the `ScreenBuffer` API
3. pin a known-good GPUI revision after macOS validation
4. decide whether explicit Enter/Backspace directory reads should later become fully asynchronous
