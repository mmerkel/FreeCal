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
    /// it; FreeCal asks again either way.
    #[zbus(signal)]
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
        if let Err(error) = network_changes(&on_change) {
            eprintln!("FreeCal can't read the network status: {error}");
        }
    });
}

fn network_changes(on_change: &impl Fn(bool)) -> zbus::Result<()> {
    let connection = Connection::session()?;
    let monitor = NetworkMonitorProxyBlocking::new(&connection)?;
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
    for signal in manager.receive_prepare_for_sleep()? {
        if !signal.args()?.start {
            on_wake();
        }
    }
    Ok(())
}
