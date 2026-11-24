//! What happens when something is opened that is already open (`fs.open.when-open`): the four words of the setting as a type.
//! The words are the choices of `catalog/settings.json`; a test keeps both equal (G4).
//!
//! Pattern: Strategy (the closed list of behaviours the setting chooses from; `Ask` defers the choice to the person).

/// What to do when the thing is already open in a window.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhenOpen {
    /// Ask the person first (the default).
    #[default]
    Ask,
    /// Switch to the window it is open in.
    Switch,
    /// Open it as a tab (follows; until then it behaves like `Ask`).
    Tab,
    /// Open another window.
    Window,
}

impl WhenOpen {
    /// All of them, in the order of the choices of the setting.
    pub const ALL: [Self; 4] = [Self::Ask, Self::Switch, Self::Tab, Self::Window];

    /// The word of the setting.
    #[must_use]
    pub const fn as_choice(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Switch => "switch",
            Self::Tab => "tab",
            Self::Window => "window",
        }
    }

    /// The behaviour of a word of the setting; `None` for a word that is no choice.
    #[must_use]
    pub fn from_choice(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|w| w.as_choice() == word)
    }
}
