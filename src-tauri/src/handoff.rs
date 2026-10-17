//! The hand-off from a second launch to the running FreeCal (ADR 0005), over
//! the D-Bus session bus. The first launch owns a bus name that stands for
//! its Local Store. A second launch finds it taken, passes on its arguments,
//! working directory and activation token, and exits. The token is what lets
//! the running FreeCal come to the front on Wayland, where the compositor
//! raises a window only for the launch the user just made.

use std::path::Path;

use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;

const PATH: &str = "/io/github/mmerkel/FreeCal/Launch";
const INTERFACE: &str = "io.github.mmerkel.FreeCal.Launch";

/// A launch of FreeCal, as a second launch passes it on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub args: Vec<String>,
    /// For relative paths among `args`.
    pub cwd: String,
    /// What the launcher gave this launch to activate a window with:
    /// `XDG_ACTIVATION_TOKEN` on Wayland, `DESKTOP_STARTUP_ID` on X11.
    pub activation_token: Option<String>,
}

impl Launch {
    /// This process's launch.
    pub fn current() -> Self {
        Self {
            args: std::env::args_os()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            cwd: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            activation_token: ["XDG_ACTIVATION_TOKEN", "DESKTOP_STARTUP_ID"]
                .into_iter()
                .find_map(|name| std::env::var(name).ok().filter(|token| !token.is_empty())),
        }
    }
}

pub enum Claim {
    /// This is the running FreeCal; later launches reach it while the
    /// connection is kept.
    Running(Connection),
    /// Another FreeCal runs on this Local Store and has taken this launch.
    HandedOff,
}

/// Makes this the FreeCal that later launches on the same Local Store hand
/// off to, calling `on_launch` for each, or hands `launch` to the one that
/// already is.
pub fn claim(
    name: &str,
    launch: &Launch,
    on_launch: impl Fn(Launch) + Send + Sync + 'static,
) -> zbus::Result<Claim> {
    let receiver = Receiver(Box::new(on_launch));
    match Builder::session()?
        .name(name)?
        .replace_existing_names(false)
        .allow_name_replacements(false)
        .serve_at(PATH, receiver)?
        .build()
    {
        Ok(connection) => Ok(Claim::Running(connection)),
        Err(zbus::Error::NameTaken) => {
            Connection::session()?.call_method(
                Some(name),
                PATH,
                Some(INTERFACE),
                "Launched",
                &(
                    &launch.args,
                    &launch.cwd,
                    launch.activation_token.as_deref().unwrap_or_default(),
                ),
            )?;
            Ok(Claim::HandedOff)
        }
        Err(error) => Err(error),
    }
}

/// The bus name that stands for the Local Store in `data_dir`, so that
/// FreeCals on separate data directories run side by side (ADR 0005). The
/// hash is FNV-1a, which unlike `std`'s hasher stays the same across builds,
/// so an updated FreeCal still reaches one started before the update.
pub fn local_store_name(identifier: &str, data_dir: &Path) -> String {
    let hash = data_dir
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    // A bus name element can't start with a digit.
    format!("{identifier}.Store{hash:016x}")
}

struct Receiver(Box<dyn Fn(Launch) + Send + Sync>);

#[zbus::interface(name = "io.github.mmerkel.FreeCal.Launch")]
impl Receiver {
    fn launched(&self, args: Vec<String>, cwd: String, activation_token: String) {
        (self.0)(Launch {
            args,
            cwd,
            activation_token: Some(activation_token).filter(|token| !token.is_empty()),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    /// Needs a D-Bus session bus, as FreeCal does.
    #[test]
    fn a_second_launch_hands_its_arguments_and_token_to_the_running_freecal() {
        let name = format!("io.github.mmerkel.FreeCal.Test{}", std::process::id());
        let (sender, received) = mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let first = Launch {
            args: vec!["freecal".into()],
            cwd: "/".into(),
            activation_token: None,
        };
        let Claim::Running(_running) = claim(&name, &first, move |launch| {
            sender.lock().unwrap().send(launch).unwrap();
        })
        .expect("claim the bus name") else {
            panic!("the first launch should run");
        };
        let second = Launch {
            args: vec!["freecal".into(), "party.ics".into()],
            cwd: "/home/someone".into(),
            activation_token: Some("kwin-token-1".into()),
        };

        let claimed = claim(&name, &second, |_| {
            panic!("the second launch serves nothing")
        });

        assert!(matches!(claimed, Ok(Claim::HandedOff)));
        let got = received.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(got, second);
    }
}
