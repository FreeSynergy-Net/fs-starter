//! The settings the scaffold owns (ADR 0100 item 3: the owner declares): `fs.rpc.single.ask` (the person is asked first about a
//! server that could not be ended; default yes) and `fs.mcp.enabled` (MCP may be switched on; default NO, MCP is off by default).
//! The key names are the constants of fs-rpc (`SINGLE_ASK_KEY`, `MCP_ENABLED_KEY`); a test keeps both in step. The words are
//! MessageFormat 2 files of this crate ([`TEXT_EN`], [`TEXT_DE`]; ADR 0057).
//!
//! Decided (strategy, review K4b): `fs.rpc.single.ask` group `system`, place profile; `fs.mcp.enabled` group `ai`, PER DEVICE (a local entry of this
//! machine; a profile value would open it on other devices). `fs.mcp.enabled` means "the AI entry (local) is open", never "AI is allowed"
//! (ADR 0100 E7: that is a right in fs-access). The starter asks the switch only through [`crate::McpSwitch`] (standard
//! [`crate::McpOff`]); no code here reads the store. The property "human only" (an AI never sees or changes the switch,
//! ADR 0091 item 5) follows with K6 in fs-settings; the `-bin` must not wire a reader before that.
//!
//! Pattern: Strategy ([`Declares`], as every block) + constants (the texts are data).

use fs_settings::{
    Coded, Declaration, Declares, GroupId, Kind, Owner, OwnerKind, SettingCode, SettingKey, Value,
};

/// `fs.rpc.single.ask`: owner and local part (the owner is the framework area `rpc`).
pub const SINGLE_ASK: (&str, &str) = ("rpc", "single.ask");
/// `fs.mcp.enabled`.
pub const MCP_ENABLED: (&str, &str) = ("mcp", "enabled");

/// The English words of the settings of the scaffold.
pub const TEXT_EN: &str = include_str!("../texts/en.mf2");
/// The German words of the settings of the scaffold.
pub const TEXT_DE: &str = include_str!("../texts/de.mf2");

/// The declarations of the scaffold's own settings.
#[derive(Clone, Copy, Debug, Default)]
pub struct StarterSettings;

impl StarterSettings {
    /// The key of one of the constants above.
    ///
    /// # Errors
    /// `key-invalid` (never for the constants; a test proves it).
    pub fn key(which: (&str, &str)) -> Result<SettingKey, Coded<SettingCode>> {
        SettingKey::new(&Owner::new(OwnerKind::Framework, which.0)?, which.1)
    }
}

impl Declares for StarterSettings {
    fn declarations(&self) -> Result<Vec<Declaration>, Coded<SettingCode>> {
        Ok(vec![
            Declaration::new(
                Self::key(SINGLE_ASK)?,
                Kind::Flag,
                Value::Flag(true),
                GroupId::new("system")?,
            ),
            Declaration::new(
                Self::key(MCP_ENABLED)?,
                Kind::Flag,
                Value::Flag(false),
                GroupId::new("ai")?,
            )
            .per_device(),
        ])
    }
}
