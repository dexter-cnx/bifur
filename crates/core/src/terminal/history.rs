use serde::{Deserialize, Serialize};

/// One command execution unit retained independently from UI rendering.
///
/// Keeping this in core makes the history reusable by GPUI, Flutter, and
/// future AI features such as command explanation and block-aware context.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandBlock {
    pub id: String,
    pub command: String,
    pub output: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub timestamp: u64,
}
