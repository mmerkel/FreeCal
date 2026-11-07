//! What the Linux desktop tells FreeCal about the computer, for the Sync
//! Scheduler (spec: "Sync timing"): the network status from the
//! NetworkMonitor portal, which also works outside Flatpak, and waking from
//! sleep from logind. Each watch runs on a thread of its own for as long as
//! FreeCal runs. Without a portal or logind, nothing is reported: the
//! network is then taken as available, and Offline is decided by each
//! Account's requests.

use zbus::blocking::Connection;

#[zbus::proxy(
    interface = "org.freedesktop.portal.NetworkMonitor",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait NetworkMonitor {
    /// Whether there is a network at all; a local one counts.
    fn get_available(&self) -> zbus::Result<bool>;

    /// The network status changed. Older portals send the new values with
    /// it; FreeCal asks again either way. Its name is in lower case, unlike
    /// the PascalCase zbus would make of the method's name.
    #[zbus(signal, name = "changed")]
    fn changed(&self) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    /// Sent with `start` before the computer sleeps, and without once it
    /// woke.
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

/// Calls `on_change` with whether there is a network at all: once now, and
/// again on each change the portal reports.
pub fn watch_network(on_change: impl Fn(bool) + Send + 'static) {
    std::thread::spawn(move || {
        let changes =
            Connection::session().and_then(|connection| network_changes(&connection, &on_change));
        if let Err(error) = changes {
            eprintln!("FreeCal can't read the network status: {error}");
        }
    });
}

fn network_changes(connection: &Connection, on_change: &impl Fn(bool)) -> zbus::Result<()> {
    let monitor = NetworkMonitorProxyBlocking::new(connection)?;
    // Subscribed before the first read, so that no change falls in the gap.
    let changes = monitor.receive_changed()?;
    on_change(monitor.get_available()?);
    for _ in changes {
        on_change(monitor.get_available()?);
    }
    Ok(())
}

/// Calls `on_wake` each time the computer wakes from sleep.
pub fn watch_wake(on_wake: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        if let Err(error) = wakes(&on_wake) {
            eprintln!("FreeCal can't tell when the computer wakes: {error}");
        }
    });
}

fn wakes(on_wake: &impl Fn()) -> zbus::Result<()> {
    let connection = Connection::system()?;
    let manager = LoginManagerProxyBlocking::new(&connection)?;
    for notice in manager.receive_prepare_for_sleep()? {
        if !notice.args()?.start {
            on_wake();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;
    use zbus::blocking::connection::Builder;

    const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
    const PORTAL_INTERFACE: &str = "org.freedesktop.portal.NetworkMonitor";

    /// A session bus of the test's own, so that the bus routes signals by
    /// the match rules FreeCal asks for, as the real one does.
    struct PrivateBus {
        daemon: Child,
        address: String,
    }

    impl PrivateBus {
        fn start() -> Self {
            let mut daemon = Command::new("dbus-daemon")
                .args(["--session", "--nofork", "--print-address=1"])
                .stdout(Stdio::piped())
                .spawn()
                .expect("the tests need dbus-daemon");
            let mut address = String::new();
            BufReader::new(daemon.stdout.take().unwrap())
                .read_line(&mut address)
                .unwrap();
            Self {
                daemon,
                address: address.trim().to_owned(),
            }
        }

        fn connect(&self) -> Builder<'static> {
            Builder::address(self.address.as_str()).unwrap()
        }
    }

    impl Drop for PrivateBus {
        fn drop(&mut self) {
            let _ = self.daemon.kill();
            let _ = self.daemon.wait();
        }
    }

    /// Stands in for xdg-desktop-portal's NetworkMonitor.
    struct FakeNetworkMonitor {
        available: Arc<AtomicBool>,
    }

    #[zbus::interface(name = "org.freedesktop.portal.NetworkMonitor")]
    impl FakeNetworkMonitor {
        fn get_available(&self) -> bool {
            self.available.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn each_change_the_portal_reports_reads_the_network_status_again() {
        let bus = PrivateBus::start();
        let available = Arc::new(AtomicBool::new(true));
        let portal = bus
            .connect()
            .name("org.freedesktop.portal.Desktop")
            .unwrap()
            .serve_at(
                PORTAL_PATH,
                FakeNetworkMonitor {
                    available: available.clone(),
                },
            )
            .unwrap()
            .build()
            .unwrap();
        let freecal = bus.connect().build().unwrap();
        let (sender, readings) = mpsc::channel();
        std::thread::spawn(move || {
            network_changes(&freecal, &|available| sender.send(available).unwrap()).unwrap()
        });
        let next = || readings.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(next());

        available.store(false, Ordering::SeqCst);
        // The portal names its signal "changed", in lower case.
        portal
            .emit_signal(None::<()>, PORTAL_PATH, PORTAL_INTERFACE, "changed", &())
            .unwrap();

        assert!(!next());
    }
}
