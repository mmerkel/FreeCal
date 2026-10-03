use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct AccountId(pub i64);

/// A kind of calendar service FreeCal can talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Local,
}

impl Provider {
    /// How the Local Store records the Provider.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Provider::Local => "local",
        }
    }

    pub(crate) fn parse(stored: &str) -> Option<Self> {
        [Provider::Local]
            .into_iter()
            .find(|provider| provider.as_str() == stored)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    pub id: AccountId,
    pub provider: Provider,
}
