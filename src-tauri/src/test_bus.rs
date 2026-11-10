//! A D-Bus session bus for the tests, on which fakes stand in for the
//! desktop's services.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use zbus::blocking::connection::Builder;

/// A session bus of the test's own, so that the bus routes signals by
/// the match rules FreeCal asks for, as the real one does.
pub struct PrivateBus {
    daemon: Child,
    address: String,
    _config: tempfile::NamedTempFile,
}

/// A session bus without the session's service directories, so that it
/// never starts a real service, as a real portal would write the user's
/// files: a name no fake owns stays missing.
const CONFIG: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir=/tmp</listen>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#;

impl PrivateBus {
    pub fn start() -> Self {
        let config = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(config.path(), CONFIG).unwrap();
        let mut daemon = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config.path().display()))
            .args(["--nofork", "--print-address=1"])
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
            _config: config,
        }
    }

    pub fn connect(&self) -> Builder<'static> {
        Builder::address(self.address.as_str()).unwrap()
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}
