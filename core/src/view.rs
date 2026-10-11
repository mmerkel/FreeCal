use serde::{Deserialize, Serialize};

/// The views the grid offers. FreeCal opens in month view on first launch,
/// then in the last-used one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum View {
    Day,
    Week,
    #[default]
    Month,
}

impl View {
    /// How the Local Store records the view.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            View::Day => "day",
            View::Week => "week",
            View::Month => "month",
        }
    }

    /// A stored view is untrusted like every stored field; anything unknown
    /// is the default.
    pub(crate) fn parse(stored: &str) -> Self {
        [View::Day, View::Week, View::Month]
            .into_iter()
            .find(|view| view.as_str() == stored)
            .unwrap_or_default()
    }
}
