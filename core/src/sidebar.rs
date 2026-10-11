use serde::{Deserialize, Serialize};

/// The sidebar as the user left it: shown or hidden, and its width in `rem`,
/// or `None` for the default width. The frontend shows the width within its
/// limits without changing what is stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SidebarState {
    pub shown: bool,
    pub width: Option<f64>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            shown: true,
            width: None,
        }
    }
}
