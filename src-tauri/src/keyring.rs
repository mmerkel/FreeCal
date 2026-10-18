//! The system keyring the core keeps server Accounts' secrets in.

use std::sync::Arc;

use freecal_core::Keyring;
#[cfg(not(target_os = "linux"))]
use freecal_core::KeyringError;

/// The keyring of this system.
pub fn system() -> Arc<dyn Keyring> {
    #[cfg(target_os = "linux")]
    return Arc::new(secret_service::SecretService);
    #[cfg(not(target_os = "linux"))]
    return Arc::new(NoKeyring);
}

/// Stands in where FreeCal has no keyring yet: every request fails.
#[cfg(not(target_os = "linux"))]
struct NoKeyring;

#[cfg(not(target_os = "linux"))]
impl Keyring for NoKeyring {
    fn store(&self, _key: &str, _label: &str, _secret: &str) -> Result<(), KeyringError> {
        Err(no_keyring())
    }

    fn secret(&self, _key: &str) -> Result<Option<String>, KeyringError> {
        Err(no_keyring())
    }

    fn delete(&self, _key: &str) -> Result<(), KeyringError> {
        Err(no_keyring())
    }
}

#[cfg(not(target_os = "linux"))]
fn no_keyring() -> KeyringError {
    KeyringError("no keyring on this system".to_owned())
}

#[cfg(target_os = "linux")]
mod secret_service {
    use std::collections::HashMap;
    use std::future::Future;

    use freecal_core::{Keyring, KeyringError};

    /// The attribute that marks an item as FreeCal's.
    const SCHEMA: (&str, &str) = ("xdg:schema", "io.github.mmerkel.FreeCal.Account");

    /// The Secret Service over D-Bus, such as GNOME Keyring or KWallet, or
    /// inside a Flatpak the Secret portal: oo7 chooses.
    pub struct SecretService;

    impl Keyring for SecretService {
        fn store(&self, key: &str, label: &str, secret: &str) -> Result<(), KeyringError> {
            run(async {
                unlocked()
                    .await?
                    .create_item(label, &attributes(key), secret, true)
                    .await
                    .map_err(failed)
            })
        }

        fn secret(&self, key: &str) -> Result<Option<String>, KeyringError> {
            run(async {
                let keyring = unlocked().await?;
                let found = keyring.search_items(&attributes(key)).await;
                let Some(item) = found.map_err(failed)?.pop() else {
                    return Ok(None);
                };
                item.unlock().await.map_err(failed)?;
                let secret = item.secret().await.map_err(failed)?;
                Ok(Some(String::from_utf8_lossy(&secret).into_owned()))
            })
        }

        fn delete(&self, key: &str) -> Result<(), KeyringError> {
            run(async {
                unlocked()
                    .await?
                    .delete(&attributes(key))
                    .await
                    .map_err(failed)
            })
        }
    }

    fn attributes(key: &str) -> HashMap<&str, &str> {
        HashMap::from([SCHEMA, ("account", key)])
    }

    /// The keyring, unlocked; the desktop may ask the user to unlock it.
    async fn unlocked() -> Result<oo7::Keyring, KeyringError> {
        let keyring = oo7::Keyring::new().await.map_err(failed)?;
        keyring.unlock().await.map_err(failed)?;
        Ok(keyring)
    }

    fn failed(error: oo7::Error) -> KeyringError {
        KeyringError(error.to_string())
    }

    /// Runs `request` to its end on a thread of its own, so that it can be
    /// called from anywhere, also from within an async command.
    fn run<T: Send>(
        request: impl Future<Output = Result<T, KeyringError>> + Send,
    ) -> Result<T, KeyringError> {
        std::thread::scope(|scope| {
            scope
                .spawn(|| tauri::async_runtime::block_on(request))
                .join()
                .expect("the keyring request panicked")
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Uses the real keyring of the session; run it by hand with
        /// `cargo test -p freecal -- --ignored`.
        #[test]
        #[ignore = "needs a desktop session's keyring"]
        fn stores_reads_and_deletes_a_secret() {
            let keyring = SecretService;
            let key = "freecal-test-key";

            keyring.store(key, "FreeCal test", "s3cret").unwrap();
            assert_eq!(keyring.secret(key).unwrap().as_deref(), Some("s3cret"));
            keyring.delete(key).unwrap();
            assert_eq!(keyring.secret(key).unwrap(), None);
        }
    }
}
