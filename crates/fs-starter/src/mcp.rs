//! The MCP switch (`fs.mcp.enabled`, [`crate::MCP_ENABLED`]): the starter asks it before the MCP entry is served.
//!
//! MCP is off by default (ADR 0097 decision 2). The starter only asks a yes or no through [`McpSwitch`]; where the value is read
//! (the settings of this machine, ADR 0100) is the program's `-bin`, so fs-starter has no edge to a settings store. Never "AI is
//! allowed" (that is a right in fs-access), only "the local AI entry is open".
//!
//! Pattern: Strategy ([`McpSwitch`]) with the safe standard [`McpOff`].

/// Whether the local MCP entry is switched on.
pub trait McpSwitch {
    /// `fs.mcp.enabled` of this machine.
    fn enabled(&self) -> bool;
}

/// The standard until a settings store is wired: MCP stays off.
#[derive(Clone, Copy, Debug, Default)]
pub struct McpOff;

impl McpSwitch for McpOff {
    fn enabled(&self) -> bool {
        false
    }
}
