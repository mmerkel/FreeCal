//! Start at login (US-107) on the Linux desktop, through the XDG autostart
//! entry `~/.config/autostart/<app ID>.desktop`. Inside Flatpak only the
//! Background portal can reach it: FreeCal asks the portal, which writes or
//! deletes it and may ask the user first. Outside Flatpak FreeCal writes or
//! deletes it itself and never asks the portal, which there names the entry
//! after whatever app ID it finds, such as that of the terminal FreeCal was
//! started from (`docs/research/autostart-state.md`). So each entry has one
//! writer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use freecal_core::ErrorCode;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

/// How FreeCal is started at login, and where.
pub struct StartAtLogin {
    /// FreeCal's app ID, which names the autostart entry.
    pub app_id: String,
    /// The name the autostart entry shows.
    pub name: String,
    /// The command that starts FreeCal: inside Flatpak, the one inside the
    /// sandbox, to which the portal adds `flatpak run`.
    pub commandline: Vec<String>,
    /// The XDG autostart directory, e.g. ~/.config/autostart.
    pub autostart_dir: PathBuf,
    /// Whether FreeCal runs in Flatpak.
    pub sandboxed: bool,
}

impl StartAtLogin {
    /// For this FreeCal, with the app ID `app_id`, the name `name` and the
    /// XDG config directory `config_dir`. None when it can't tell its own
    /// program.
    pub fn current(app_id: &str, name: &str, config_dir: &Path) -> Option<Self> {
        let program = std::env::current_exe()
            .ok()?
            .into_os_string()
            .into_string()
            .ok()?;
        Some(Self {
            app_id: app_id.to_owned(),
            name: name.to_owned(),
            commandline: vec![program],
            autostart_dir: config_dir.join("autostart"),
            // Flatpak puts this file in every sandbox.
            sandboxed: Path::new("/.flatpak-info").exists(),
        })
    }

    /// Turns starting at login on or off. Fails with `StartAtLoginRefused`
    /// when the user or the desktop refused it, with `StartAtLoginFailed`
    /// when it couldn't be done.
    pub fn set(&self, on: bool) -> Result<(), ErrorCode> {
        self.set_through(Connection::session, on)
    }

    fn set_through(
        &self,
        session: impl FnOnce() -> zbus::Result<Connection>,
        on: bool,
    ) -> Result<(), ErrorCode> {
        if !self.sandboxed {
            return self
                .write_entry(on)
                .map_err(|_| ErrorCode::StartAtLoginFailed);
        }
        let response =
            session().and_then(|connection| request_background(&connection, on, &self.commandline));
        match portal_outcome(response, on) {
            PortalOutcome::Done => Ok(()),
            PortalOutcome::Refused => Err(ErrorCode::StartAtLoginRefused),
            PortalOutcome::NotDone => Err(ErrorCode::StartAtLoginFailed),
        }
    }

    /// Writes or deletes the autostart entry, outside Flatpak.
    fn write_entry(&self, on: bool) -> std::io::Result<()> {
        let file = self.autostart_dir.join(format!("{}.desktop", self.app_id));
        if !on {
            return match std::fs::remove_file(file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            };
        }
        let exec = exec_line(&self.commandline).ok_or(std::io::ErrorKind::InvalidInput)?;
        std::fs::create_dir_all(&self.autostart_dir)?;
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec={exec}\n",
            self.name
        );
        std::fs::write(file, entry)
    }
}

#[zbus::proxy(
    interface = "org.freedesktop.portal.Background",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait Background {
    /// Asks to run in the background, and with `autostart` in `options` to
    /// start at login. The answer comes as the Response of the returned
    /// request.
    fn request_background(
        &self,
        parent_window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.portal.Request",
    default_service = "org.freedesktop.portal.Desktop"
)]
trait Request {
    /// The answer to a request: 0 when it was done, 1 when the user
    /// cancelled it, 2 when it ended otherwise.
    #[zbus(signal)]
    fn response(&self, response: u32, results: HashMap<String, OwnedValue>) -> zbus::Result<()>;
}

/// A Response's code when the request was done.
const RESPONSE_DONE: u32 = 0;
/// A Response's code when the user cancelled the request.
const RESPONSE_CANCELLED: u32 = 1;

/// Asks the Background portal to turn starting at login on or off. Every
/// request says which, since one without `autostart` deletes the entry.
/// Returns the Response's code and results.
fn request_background(
    connection: &Connection,
    on: bool,
    commandline: &[String],
) -> zbus::Result<(u32, HashMap<String, OwnedValue>)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static REQUESTS: AtomicU32 = AtomicU32::new(0);

    let token = format!("freecal{}", REQUESTS.fetch_add(1, Ordering::Relaxed));
    let sender = connection
        .unique_name()
        .ok_or_else(|| zbus::Error::Failure("no unique name on the bus".to_owned()))?
        .trim_start_matches(':')
        .replace('.', "_");
    // The request's path follows from the token, so FreeCal subscribes to
    // its Response before asking and no answer falls in the gap.
    let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let request = RequestProxyBlocking::builder(connection)
        .path(path)?
        .build()?;
    let mut responses = request.receive_response()?;
    let options = HashMap::from([
        ("handle_token", Value::from(token.as_str())),
        ("autostart", Value::from(on)),
        ("commandline", Value::from(commandline.to_vec())),
        ("dbus-activatable", Value::from(false)),
    ]);
    BackgroundProxyBlocking::new(connection)?.request_background("", options)?;
    let response = responses
        .next()
        .ok_or_else(|| zbus::Error::Failure("the portal never answered".to_owned()))?;
    let args = response.args()?;
    Ok((args.response, args.results))
}

/// What the Background portal did about starting at login.
#[derive(Debug, PartialEq)]
enum PortalOutcome {
    Done,
    /// The user cancelled it, or running in the background isn't allowed.
    Refused,
    /// There was no portal, or it couldn't, as when it can't tell
    /// FreeCal's app ID.
    NotDone,
}

fn portal_outcome(
    response: zbus::Result<(u32, HashMap<String, OwnedValue>)>,
    on: bool,
) -> PortalOutcome {
    let Ok((code, results)) = response else {
        return PortalOutcome::NotDone;
    };
    let result = |key: &str| {
        results
            .get(key)
            .and_then(|value| bool::try_from(value).ok())
    };
    match code {
        RESPONSE_CANCELLED => PortalOutcome::Refused,
        // The entry is then left as it was.
        RESPONSE_DONE if result("background") == Some(false) => PortalOutcome::Refused,
        RESPONSE_DONE if result("autostart") == Some(on) => PortalOutcome::Done,
        _ => PortalOutcome::NotDone,
    }
}

/// The `Exec` value of a desktop entry that runs `commandline`: each
/// argument quoted, then escaped as a string value. None for an argument
/// with a control character, which a desktop entry can't hold.
fn exec_line(commandline: &[String]) -> Option<String> {
    let arguments: Option<Vec<String>> = commandline
        .iter()
        .map(|argument| {
            if argument.chars().any(char::is_control) {
                return None;
            }
            let mut quoted = String::from('"');
            for c in argument.chars() {
                match c {
                    // Escaped for the quoting, then that backslash for the
                    // string value.
                    '"' | '`' | '$' => quoted.extend(['\\', '\\', c]),
                    '\\' => quoted.push_str(r"\\\\"),
                    // A field code otherwise.
                    '%' => quoted.push_str("%%"),
                    c => quoted.push(c),
                }
            }
            quoted.push('"');
            Some(quoted)
        })
        .collect();
    Some(arguments?.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_bus::PrivateBus;
    use std::sync::mpsc;
    use std::time::Duration;

    const APP_ID: &str = "io.github.mmerkel.FreeCal";

    /// What FreeCal asked the portal for.
    #[derive(Debug, PartialEq)]
    struct Asked {
        autostart: bool,
        commandline: Vec<String>,
    }

    /// Out of reach of `super::*`, whose proxies' types the interface's
    /// code could clash with.
    mod fake_portal {
        use super::Asked;
        use std::collections::HashMap;
        use std::sync::mpsc;
        use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

        /// Stands in for xdg-desktop-portal's Background portal: notes what
        /// FreeCal asks for and answers on the request's object path, as the
        /// real one does. `autostart` is what it answers it did; by default
        /// what was asked.
        pub struct FakeBackground {
            pub asked: mpsc::Sender<Asked>,
            pub response: u32,
            pub background: bool,
            pub autostart: Option<bool>,
        }

        #[zbus::interface(name = "org.freedesktop.portal.Background")]
        impl FakeBackground {
            async fn request_background(
                &self,
                #[zbus(header)] header: zbus::message::Header<'_>,
                #[zbus(connection)] connection: &zbus::Connection,
                _parent_window: &str,
                options: HashMap<&str, Value<'_>>,
            ) -> OwnedObjectPath {
                let sender = header
                    .sender()
                    .unwrap()
                    .trim_start_matches(':')
                    .replace('.', "_");
                let token: &str = options["handle_token"].downcast_ref().unwrap();
                let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
                let autostart: bool = options["autostart"].downcast_ref().unwrap();
                let commandline =
                    Vec::<String>::try_from(options["commandline"].try_clone().unwrap()).unwrap();
                self.asked
                    .send(Asked {
                        autostart,
                        commandline,
                    })
                    .unwrap();
                let results = HashMap::from([
                    ("background", Value::from(self.background)),
                    (
                        "autostart",
                        Value::from(self.autostart.unwrap_or(autostart)),
                    ),
                ]);
                connection
                    .emit_signal(
                        None::<()>,
                        path.as_str(),
                        "org.freedesktop.portal.Request",
                        "Response",
                        &(self.response, results),
                    )
                    .await
                    .unwrap();
                ObjectPath::try_from(path).unwrap().into()
            }
        }
    }
    use fake_portal::FakeBackground;

    /// A FreeCal whose autostart directory is a temporary one.
    struct Desktop {
        bus: PrivateBus,
        _portal: Option<zbus::blocking::Connection>,
        asked: mpsc::Receiver<Asked>,
        dir: tempfile::TempDir,
        sandboxed: bool,
    }

    impl Desktop {
        /// With a portal that answers `response`, with whether running in the
        /// background is allowed and what it did about autostart.
        fn with_portal(response: u32, background: bool, autostart: Option<bool>) -> Self {
            let bus = PrivateBus::start();
            let (sender, asked) = mpsc::channel();
            let portal = bus
                .connect()
                .name("org.freedesktop.portal.Desktop")
                .unwrap()
                .serve_at(
                    "/org/freedesktop/portal/desktop",
                    FakeBackground {
                        asked: sender,
                        response,
                        background,
                        autostart,
                    },
                )
                .unwrap()
                .build()
                .unwrap();
            Self {
                bus,
                _portal: Some(portal),
                asked,
                dir: tempfile::tempdir().unwrap(),
                sandboxed: false,
            }
        }

        /// As the real portal does it for FreeCal: what it was asked.
        fn portal_that_does_it() -> Self {
            Self::with_portal(0, true, None)
        }

        fn without_portal() -> Self {
            let (_, asked) = mpsc::channel();
            Self {
                bus: PrivateBus::start(),
                _portal: None,
                asked,
                dir: tempfile::tempdir().unwrap(),
                sandboxed: false,
            }
        }

        fn sandboxed(mut self) -> Self {
            self.sandboxed = true;
            self
        }

        fn start_at_login(&self) -> StartAtLogin {
            StartAtLogin {
                app_id: APP_ID.to_owned(),
                name: "FreeCal".to_owned(),
                commandline: vec!["/opt/Free Cal/freecal".to_owned()],
                autostart_dir: self.dir.path().join("autostart"),
                sandboxed: self.sandboxed,
            }
        }

        fn set(&self, on: bool) -> Result<(), ErrorCode> {
            self.start_at_login()
                .set_through(|| self.bus.connect().build(), on)
        }

        fn asked(&self) -> Asked {
            self.asked.recv_timeout(Duration::from_secs(5)).unwrap()
        }

        fn entry(&self) -> Option<String> {
            let file = self.dir.path().join(format!("autostart/{APP_ID}.desktop"));
            std::fs::read_to_string(file).ok()
        }
    }

    #[test]
    fn inside_flatpak_the_portal_is_asked_with_the_choice_and_freecals_own_command() {
        let desktop = Desktop::portal_that_does_it().sandboxed();

        assert_eq!(desktop.set(true), Ok(()));

        assert_eq!(
            desktop.asked(),
            Asked {
                autostart: true,
                commandline: vec!["/opt/Free Cal/freecal".to_owned()],
            }
        );
        // The portal wrote it.
        assert_eq!(desktop.entry(), None);
    }

    #[test]
    fn turning_it_off_asks_the_portal_too() {
        let desktop = Desktop::portal_that_does_it().sandboxed();

        assert_eq!(desktop.set(false), Ok(()));

        assert!(!desktop.asked().autostart);
    }

    #[test]
    fn a_cancelled_request_is_refused() {
        let desktop = Desktop::with_portal(1, false, Some(false)).sandboxed();

        assert_eq!(desktop.set(true), Err(ErrorCode::StartAtLoginRefused));
        assert_eq!(desktop.entry(), None);
    }

    #[test]
    fn running_in_the_background_not_allowed_is_refused() {
        // The portal then leaves the autostart entry as it was.
        let desktop = Desktop::with_portal(0, false, Some(false)).sandboxed();

        assert_eq!(desktop.set(false), Err(ErrorCode::StartAtLoginRefused));
    }

    #[test]
    fn outside_flatpak_freecal_writes_its_own_entry_without_asking_the_portal() {
        // Outside Flatpak the portal names the entry after whatever app ID it
        // finds, such as the terminal's FreeCal was started from.
        let desktop = Desktop::portal_that_does_it();

        assert_eq!(desktop.set(true), Ok(()));

        assert!(
            desktop
                .asked
                .recv_timeout(Duration::from_millis(200))
                .is_err()
        );

        assert_eq!(
            desktop.entry().unwrap(),
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=FreeCal\n\
             Exec=\"/opt/Free Cal/freecal\"\n"
        );
    }

    #[test]
    fn outside_flatpak_freecal_writes_the_entry_without_a_portal() {
        let desktop = Desktop::without_portal();

        assert_eq!(desktop.set(true), Ok(()));
        assert!(desktop.entry().is_some());

        assert_eq!(desktop.set(false), Ok(()));
        assert_eq!(desktop.entry(), None);
        // Off when it already is.
        assert_eq!(desktop.set(false), Ok(()));
    }

    #[test]
    fn inside_flatpak_only_the_portal_can_do_it() {
        // As the portal answers where it can't tell FreeCal's app ID.
        let failing = Desktop::with_portal(2, false, Some(false)).sandboxed();
        let missing = Desktop::without_portal().sandboxed();

        assert_eq!(failing.set(true), Err(ErrorCode::StartAtLoginFailed));
        assert_eq!(missing.set(true), Err(ErrorCode::StartAtLoginFailed));
        assert_eq!(failing.entry(), None);
        assert_eq!(missing.entry(), None);
    }

    #[test]
    fn an_entry_that_cant_be_written_fails() {
        let desktop = Desktop::without_portal();
        // A file where the directory should be.
        std::fs::write(desktop.dir.path().join("autostart"), "").unwrap();

        assert_eq!(desktop.set(true), Err(ErrorCode::StartAtLoginFailed));
    }

    #[test]
    fn the_command_is_quoted_as_desktop_entries_want_it() {
        let commandline = [r#"/a "b" $c `d` \e 100%"#.to_owned(), "--x".to_owned()];

        assert_eq!(
            exec_line(&commandline),
            Some(r#""/a \\"b\\" \\$c \\`d\\` \\\\e 100%%" "--x""#.to_owned())
        );
        assert_eq!(exec_line(&["/a\nb".to_owned()]), None);
    }
}
