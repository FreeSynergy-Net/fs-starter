//! The parts of the starter shell that this host does not have yet, named instead of left out without a word.
//!
//! Pattern: Value: one constant list and the one line that says it.

/// The code of the line (a part of the skeleton that is planned and not built).
pub const CODE: &str = "skeleton-part-pending";

/// The code of the open point of the update: the host runs the start hooks, but no hook is bound to `fs-update` yet.
pub const UPDATE_CODE: &str = "update-binding-pending";

/// The parts that are not built: the starter pulls them in when the host is joined to it.
pub const PENDING: [&str; 5] = ["single-instance", "session", "update", "help", "mcp"];

/// The line for the person: the code and the parts.
#[must_use]
pub fn note() -> String {
    format!(
        "fs-starter: {CODE}: {} (not built yet; this host only opens the window of a description; MCP stays off); \
         {UPDATE_CODE}: the update hook is a no-op until fs-update is bound (ADR 0105 open point 9)",
        PENDING.join(", ")
    )
}
