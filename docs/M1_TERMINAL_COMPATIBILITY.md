# M1 Terminal Compatibility Status

This document tracks the UI-neutral terminal parser work in `bifur-core`.
GPUI and any future Flutter frontend consume `ScreenBuffer`; neither frontend should parse PTY bytes directly.

## Implemented

- Incremental UTF-8 decoding across PTY read boundaries
- Raw ANSI control bytes excluded from visible text
- DEC application cursor-key mode (`CSI ?1 h/l`)
- Relative cursor movement: `CUU/CUD/CUF/CUB` (`A/B/C/D`)
- Next/previous line: `CNL/CPL` (`E/F`)
- Absolute column/row: `CHA/VPA` (`G/d`)
- Absolute cursor position: `CUP/HVP` (`H/f`)
- Display/line erase: `ED/EL` (`J/K`, modes 0/1/2)
- ANSI cursor save/restore: `CSI s/u`
- Legacy DECSC/DECRC: `ESC 7/8`, including rendition state
- Pending-autowrap handling
- Screen resize preserving recent rows and cursor/saved-cursor state
- SGR reset, bold, standard/bright foreground/background colors
- xterm 256-color and 24-bit truecolor SGR
- GPUI rendering of per-cell foreground/background/bold rendition
- Character editing: `ICH/DCH/ECH` (`@/P/X`)
- CSI intermediate-byte guard so unsupported variants are not misrouted as character edits
- BCE-style active-background fill for erase and character-edit blank cells

## Next parser gaps

Priority order for the remaining M1 compatibility work:

1. Line editing: `IL/DL` (`CSI L/M`)
2. DEC scrolling margins / scroll region: `DECSTBM` (`CSI top;bottom r`)
3. Region-aware scrolling and newline behavior
4. Scroll commands needed by common shell/TUI output
5. Additional SGR attributes only where `Cell`/frontend rendering can represent them correctly
6. Parser robustness around private parameters and intermediate bytes

Line editing must be implemented together with scroll-region semantics or with an explicit full-screen-only contract that can later be narrowed to the active region without changing `ScreenBuffer`'s public model.

## Validation still required for M1

- Physical macOS watcher/refresh burst validation
- Pin a known-good GPUI revision after macOS validation
- Decide whether initial synchronous Enter/Backspace reads should become async
- Keep terminal parser/state in `bifur-core`; frontend key handling and rendering stay outside core

## Completed PR sequence

- #26: CSI cursor movement
- #27: erase semantics
- #28: cursor save/restore
- #29: basic SGR rendition
- #30: extended SGR colors
- #31: extended CSI cursor positioning
- #32: terminal character editing semantics
