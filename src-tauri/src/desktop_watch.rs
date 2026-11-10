//! What the Linux desktop tells FreeCal about the computer: for the Sync
//! Scheduler (spec: "Sync timing"), the network status from the
//! NetworkMonitor portal, which also works outside Flatpak, and waking from
//! sleep from logind; and for quitting (spec: "Quitting"), the session
//! ending, from the Inhibit portal's session monitor for a logout and from
//! logind for a shutdown. Each watch runs on a thread of its own for as
//! long as FreeCal runs. Without a portal or logind, nothing is reported:
//! the network is then taken as available, and Offline is decided by each
//! Account's requests. It also tells whether the desktop has a system tray,
//! which decides what closing the window does (US-103, US-104).

use std::collections::HashMap;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

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

    /// Sent with `start` before the computer shuts down or reboots.
    #[zbus(signal)]
    fn prepare_for_shutdown(&self, start: bool) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.portal.Inhibit",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait Inhibit {
    /// Starts a session monitor, which sends `StateChanged`. It answers
    /// with a request, whose response FreeCal doesn't need.
    fn create_monitor(
        &self,
        window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    /// Tells the portal FreeCal is ready for the session to end.
    fn query_end_response(&self, session_handle: &ObjectPath<'_>) -> zbus::Result<()>;

    /// The session's state changed; its `session-state` is 1 while it runs,
    /// 2 when its end is asked for and 3 while it ends.
    #[zbus(signal)]
    fn state_changed(
        &self,
        session_handle: ObjectPath<'_>,
        state: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
trait StatusNotifierWatcher {
    /// Whether a tray shows the StatusNotifierItems registered with it.
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> zbus::Result<bool>;
}

/// How long the tray check waits for the desktop at most. It runs while
/// the window closes, which a hung desktop must not hold up for long.
const TRAY_CHECK_TIMEOUT: Duration = Duration::from_secs(1);

/// Whether the desktop has a system tray right now: a StatusNotifierWatcher
/// with a host registered (spec: "Tray detection"). Asked at every window
/// close, so that a tray turned on while FreeCal runs counts at the next
/// one (US-105). When the desktop can't be asked, there is none.
pub fn tray_present() -> bool {
    zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.method_timeout(TRAY_CHECK_TIMEOUT).build())
        .and_then(|connection| tray_host_registered(&connection))
        .unwrap_or(false)
}

fn tray_host_registered(connection: &Connection) -> zbus::Result<bool> {
    StatusNotifierWatcherProxyBlocking::builder(connection)
        // One reading, so no cache that would follow the changes.
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()?
        .is_status_notifier_host_registered()
}

/// The Inhibit portal's `session-state` once the session's end is asked
/// for; 3, while it ends, is later.
const SESSION_QUERY_END: u32 = 2;

/// Calls `on_change` with `true` when the session starts to end: on logout,
/// as the Inhibit portal's session monitor reports, and on shutdown or
/// reboot, as logind reports. Then the window's close must not wait for
/// Unsent Changes (spec: "Quitting"). It calls it with `false` when the
/// session runs on, as after a logout or shutdown that was cancelled.
pub fn watch_session_end(on_change: impl Fn(bool) + Clone + Send + 'static) {
    let on_logout = on_change.clone();
    std::thread::spawn(move || {
        let ends = Connection::session().and_then(|connection| logouts(&connection, &on_logout));
        if let Err(error) = ends {
            eprintln!("FreeCal can't tell when the session ends: {error}");
        }
    });
    std::thread::spawn(move || {
        let ends = Connection::system().and_then(|connection| shutdowns(&connection, &on_change));
        if let Err(error) = ends {
            eprintln!("FreeCal can't tell when the computer shuts down: {error}");
        }
    });
}

fn logouts(connection: &Connection, on_change: &impl Fn(bool)) -> zbus::Result<()> {
    let inhibit = InhibitProxyBlocking::new(connection)?;
    // Subscribed before the monitor starts, so that no change falls in the
    // gap.
    let changes = inhibit.receive_state_changed()?;
    inhibit.create_monitor("", HashMap::new())?;
    for change in changes {
        let args = change.args()?;
        let state = args
            .state
            .get("session-state")
            .and_then(|state| u32::try_from(state).ok());
        if let Some(state) = state {
            on_change(state >= SESSION_QUERY_END);
        }
        if state == Some(SESSION_QUERY_END) {
            inhibit.query_end_response(&args.session_handle)?;
        }
    }
    Ok(())
}

fn shutdowns(connection: &Connection, on_change: &impl Fn(bool)) -> zbus::Result<()> {
    let manager = LoginManagerProxyBlocking::new(connection)?;
    for notice in manager.receive_prepare_for_shutdown()? {
        on_change(notice.args()?.start);
    }
    Ok(())
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
    use crate::test_bus::PrivateBus;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
    const PORTAL_INTERFACE: &str = "org.freedesktop.portal.NetworkMonitor";

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

    const SESSION: &str = "/org/freedesktop/portal/desktop/session/1_1/freecal";

    /// Stands in for xdg-desktop-portal's Inhibit: notes when a monitor
    /// starts, and each session whose end FreeCal said it is ready for.
    struct FakeInhibit {
        started: mpsc::Sender<()>,
        ready: mpsc::Sender<String>,
    }

    #[zbus::interface(name = "org.freedesktop.portal.Inhibit")]
    impl FakeInhibit {
        fn create_monitor(
            &self,
            _window: &str,
            _options: HashMap<&str, Value<'_>>,
        ) -> OwnedObjectPath {
            self.started.send(()).unwrap();
            ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/t")
                .unwrap()
                .into()
        }

        fn query_end_response(&self, session_handle: ObjectPath<'_>) {
            self.ready.send(session_handle.to_string()).unwrap();
        }
    }

    #[test]
    fn a_logout_ends_the_session_once_its_end_is_asked_for_until_it_runs_on() {
        let bus = PrivateBus::start();
        let (started, starts) = mpsc::channel();
        let (ready, readies) = mpsc::channel();
        let portal = bus
            .connect()
            .name("org.freedesktop.portal.Desktop")
            .unwrap()
            .serve_at(PORTAL_PATH, FakeInhibit { started, ready })
            .unwrap()
            .build()
            .unwrap();
        let freecal = bus.connect().build().unwrap();
        let (sender, ends) = mpsc::channel();
        std::thread::spawn(move || {
            logouts(&freecal, &|ending| sender.send(ending).unwrap()).unwrap()
        });
        let wait = Duration::from_secs(5);
        // FreeCal subscribes before it starts the monitor.
        starts.recv_timeout(wait).unwrap();
        let send_session_state = |session_state: u32| {
            let state = HashMap::from([("session-state", Value::from(session_state))]);
            portal
                .emit_signal(
                    None::<()>,
                    PORTAL_PATH,
                    "org.freedesktop.portal.Inhibit",
                    "StateChanged",
                    &(ObjectPath::try_from(SESSION).unwrap(), state),
                )
                .unwrap();
        };

        send_session_state(1);
        send_session_state(SESSION_QUERY_END);

        assert!(!ends.recv_timeout(wait).unwrap());
        assert!(ends.recv_timeout(wait).unwrap());
        assert_eq!(readies.recv_timeout(wait).unwrap(), SESSION);

        // The logout was cancelled.
        send_session_state(1);

        assert!(!ends.recv_timeout(wait).unwrap());
    }

    /// Stands in for logind's Manager, which only sends signals here.
    struct FakeLogin;

    #[zbus::interface(name = "org.freedesktop.login1.Manager")]
    impl FakeLogin {}

    /// Out of reach of the type zbus makes for the NetworkMonitor's
    /// `changed` signal, which the property's code would clash with.
    mod fake_tray {
        /// Stands in for the StatusNotifierWatcher of a desktop's tray.
        pub struct FakeWatcher {
            pub host: bool,
        }

        #[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
        impl FakeWatcher {
            #[zbus(property)]
            fn is_status_notifier_host_registered(&self) -> bool {
                self.host
            }
        }
    }
    use fake_tray::FakeWatcher;

    fn tray_check(watcher: Option<FakeWatcher>) -> bool {
        let bus = PrivateBus::start();
        let _desktop = watcher.map(|watcher| {
            bus.connect()
                .name("org.kde.StatusNotifierWatcher")
                .unwrap()
                .serve_at("/StatusNotifierWatcher", watcher)
                .unwrap()
                .build()
                .unwrap()
        });
        tray_host_registered(&bus.connect().build().unwrap()).unwrap_or(false)
    }

    #[test]
    fn a_tray_exists_where_a_watcher_has_a_host_registered() {
        assert!(tray_check(Some(FakeWatcher { host: true })));
    }

    #[test]
    fn a_watcher_without_a_host_is_no_tray() {
        assert!(!tray_check(Some(FakeWatcher { host: false })));
    }

    #[test]
    fn without_a_watcher_there_is_no_tray() {
        assert!(!tray_check(None));
    }

    #[test]
    fn a_shutdown_ends_the_session() {
        let bus = PrivateBus::start();
        let login = bus
            .connect()
            .name("org.freedesktop.login1")
            .unwrap()
            .serve_at("/org/freedesktop/login1", FakeLogin)
            .unwrap()
            .build()
            .unwrap();
        let freecal = bus.connect().build().unwrap();
        let (sender, ends) = mpsc::channel();
        std::thread::spawn(move || {
            shutdowns(&freecal, &|ending| sender.send(ending).unwrap()).unwrap()
        });

        // The watch subscribes on its own thread, and a signal sent before
        // that is lost, so it is sent until the watch reports it.
        let reported = (0..50).any(|_| {
            login
                .emit_signal(
                    None::<()>,
                    "/org/freedesktop/login1",
                    "org.freedesktop.login1.Manager",
                    "PrepareForShutdown",
                    &true,
                )
                .unwrap();
            ends.recv_timeout(Duration::from_millis(100)) == Ok(true)
        });

        assert!(reported);
    }
}
