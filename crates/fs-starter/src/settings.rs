//! The settings the scaffold owns (ADR 0100 item 3: the owner declares), as DATA in `catalog/settings.json` (read with `CatalogFile` of fs-settings, no Rust
//! declaration per setting): `fs.open.when-open` (what happens when it is already open; default ask), `fs.rpc.single.ask` (the person is asked first about a
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
//! Pattern: Strategy ([`Declares`], as every block) over data (the declarations and the texts are files).

use fs_settings::{
    CatalogFile, Coded, Declaration, Declares, Owner, OwnerKind, SettingCode, SettingKey,
};

/// `fs.rpc.single.ask`: owner and local part (the owner is the framework area `rpc`).
pub const SINGLE_ASK: (&str, &str) = ("rpc", "single.ask");
/// `fs.mcp.enabled`.
pub const MCP_ENABLED: (&str, &str) = ("mcp", "enabled");

/// The English words of the settings of the scaffold.
pub const TEXT_EN: &str = include_str!("../texts/en.mf2");
/// The German words of the settings of the scaffold.
pub const TEXT_DE: &str = include_str!("../texts/de.mf2");

/// `fs.open.when-open`: what happens when something is opened that is already open ([`crate::WhenOpen`]).
pub const WHEN_OPEN: (&str, &str) = ("open", "when-open");

/// The package file of the scaffold's own settings (E2): the declarations are DATA, read with [`CatalogFile`] (fs-settings).
pub const SETTINGS_JSON: &str = include_str!("../catalog/settings.json");

/// The name the file is read under (in `catalog-invalid` and `catalog-clash`).
pub const SETTINGS_FILE: &str = "fs-starter";

/// The declarations of the scaffold's own settings, read from [`SETTINGS_JSON`].
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

    /// The file, read and checked.
    ///
    /// # Errors
    /// `catalog-invalid` (never for the file of this crate; a test proves it).
    pub fn file() -> Result<CatalogFile, Coded<SettingCode>> {
        CatalogFile::parse(SETTINGS_JSON, SETTINGS_FILE)
    }
}

impl Declares for StarterSettings {
    fn declarations(&self) -> Result<Vec<Declaration>, Coded<SettingCode>> {
        Self::file()?.declarations()
    }
}
