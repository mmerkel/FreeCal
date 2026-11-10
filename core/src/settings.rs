use serde::{Deserialize, Serialize};

/// Whether the window is light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Follows the system's light or dark style, also when it changes.
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    /// How the Local Store records the appearance.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Appearance::System => "system",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        }
    }

    /// A stored appearance is untrusted like every stored field; anything
    /// unknown is the default.
    pub(crate) fn parse(stored: &str) -> Self {
        [Appearance::System, Appearance::Light, Appearance::Dark]
            .into_iter()
            .find(|appearance| appearance.as_str() == stored)
            .unwrap_or_default()
    }
}

/// The settings of the Settings dialog, as the Local Store keeps them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub appearance: Appearance,
    /// Whether closing the window keeps FreeCal running in the background
    /// where a system tray exists (US-103, US-106).
    pub run_in_background: bool,
    /// Whether FreeCal starts at login (US-107). The shell turns it on or
    /// off on the desktop before it is stored.
    pub start_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::default(),
            run_in_background: true,
            start_at_login: false,
        }
    }
}

/// A change to the settings: only the settings it gives change, so that
/// each change in the Settings dialog writes just that one setting.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub appearance: Option<Appearance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub run_in_background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub start_at_login: Option<bool>,
}
