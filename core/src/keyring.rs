/// The system keyring, where server Accounts' passwords and tokens live,
/// never in the Local Store. The shell provides the real one (Secret
/// Service); tests replace it.
pub trait Keyring: Send + Sync {
    /// Keeps `secret` under `key`, replacing any secret there. `label` names
    /// it to the user in apps that show the keyring.
    fn store(&self, key: &str, label: &str, secret: &str) -> Result<(), KeyringError>;

    /// The secret under `key`, if there is one.
    fn secret(&self, key: &str) -> Result<Option<String>, KeyringError>;

    /// Deletes the secret under `key`, if there is one.
    fn delete(&self, key: &str) -> Result<(), KeyringError>;
}

/// Why the keyring couldn't be used, as when there is none or it stayed
/// locked. The text is for logs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct KeyringError(pub String);
