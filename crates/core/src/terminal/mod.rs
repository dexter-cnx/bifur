//! UI-independent terminal subsystem.
//!
//! Architecture: frontend TerminalView -> TerminalSession -> portable-pty.

pub mod file_aware;
pub mod history;
pub mod parser;
pub mod session;
pub mod viewport;

pub use file_aware::FileAwareTerminal;
pub use history::CommandBlock;
pub use parser::{Cell, ScreenBuffer};
pub use session::{TerminalConfig, TerminalEvent, TerminalSession};
pub use viewport::TerminalViewport;
