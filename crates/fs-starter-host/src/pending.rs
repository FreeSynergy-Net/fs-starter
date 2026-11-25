//! The parts of the starter shell that this host does not have yet, named instead of left out without a word.
//!
//! Pattern: Value: one constant list and the one line that says it.

/// The code of the line (a part of the skeleton that is planned and not built).
pub const CODE: &str = "skeleton-part-pending";

/// The parts that are not built: the starter pulls them in when the host is joined to it. `update` is not in the list any more: the
/// update hook is built (`crate::update`); what is missing of it is said by its own line (`update-hook-active`).
pub const PENDING: [&str; 3] = ["session", "help", "mcp"];

/// The line for the person: the code and the parts.
#[must_use]
pub fn note() -> String {
    format!(
        "fs-starter: {CODE}: {} (not built yet; this host only opens the window of a description; MCP stays off); {}",
        PENDING.join(", "),
        crate::update::ACTIVE_NOTE
    )
}
