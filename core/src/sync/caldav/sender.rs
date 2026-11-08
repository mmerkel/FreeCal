//! The CalDAV Sender: the Outgoing Sync of a writable CalDAV Calendar,
//! which sends its Unsent Changes (ADR 0015). CalDAV sends one resource per
//! UID (RFC 4791 §4.1), so Unsent Changes are merged per UID, whatever
//! their recurrence IDs (ADR 0012). For each UID, the Sender compares its
//! Event Rows with its CalDAV Server Object, found by Calendar and UID:
//! Event Rows and a Server Object are an update, Event Rows alone a create,
//! a Server Object alone a delete, and neither leaves nothing to send. Each
//! write is conditional, so that it never writes over a version FreeCal
//! hasn't read: an update or a delete on the ETag the Mirror holds, and a
//! create on nothing being at its address. A success updates the Mirror and
//! removes the UID's Unsent Changes; a server change met while sending
//! leaves them in place, for the Incoming Sync to find (ADR 0014). The
//! Sender records nothing in the Change Journal, which holds every change
//! since it was made.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::resource::resource_to_send;
use super::{LOGGED_UID_LENGTH, deleted_by_sender, mirror};
use crate::event::{Loaded, ServerObject};
use crate::journal::{self, SentEventRows};
use crate::remote::{Precondition, Remote, WriteOutcome};
use crate::store::{CalendarToSync, Store};
use crate::sync_status::about_account;
use crate::{CalendarId, Result};

/// Sends the Unsent Changes of the CalDAV Calendar `calendar` through its
/// Account's server `remote`, UID by UID, in the order they were first
/// changed, building new `VEVENT`s with `now` as their time of revision. A
/// Read-only Calendar's wait until it is writable, and so do those of a
/// Calendar whose First Sync is unfinished, as the Mirror doesn't yet hold
/// every Server Object, so a create could meet a UID the server holds. A
/// failure about the whole Account stops it; any other failure of one UID
/// leaves its Unsent Changes in place and the others are still sent, and
/// the first is returned.
pub(crate) fn send(
    remote: &dyn Remote,
    store: &Mutex<Store>,
    &(calendar_id, ref url, ref state): &CalendarToSync,
    now: DateTime<Utc>,
) -> Result<()> {
    if state.is_none() {
        return Ok(());
    }
    let uids = {
        let store = store.lock().unwrap();
        if store.find_calendar(calendar_id)?.read_only {
            return Ok(());
        }
        store.unsent_uids(calendar_id)?
    };
    let mut failed = None;
    for uid in uids {
        let to_send = ToSend {
            calendar_id,
            url,
            uid: &uid,
        };
        match to_send.send(remote, store, now) {
            Ok(()) => {}
            Err(error) if about_account(&error) => return Err(error),
            Err(error) => _ = failed.get_or_insert(error),
        }
    }
    failed.map_or(Ok(()), Err)
}

/// The UID `uid` of the CalDAV Calendar `calendar_id` at `url`, whose
/// Unsent Changes the Sender sends.
struct ToSend<'a> {
    calendar_id: CalendarId,
    url: &'a str,
    uid: &'a str,
}

/// What the Mirror takes once the server took a write.
enum Mirrored {
    /// Nothing was sent, as there was nothing to send: the server's
    /// version stays as the Mirror holds it.
    NothingSent,
    /// The server holds this Server Object now.
    Written(ServerObject),
    /// The server holds no Server Object of the UID now.
    Deleted,
}

impl ToSend<'_> {
    /// Sends the UID's Event Rows as one CalDAV resource, built on its
    /// Server Object's text ([`resource_to_send`]), or deletes the Server
    /// Object when it has no Event Rows. With neither, its Unsent Changes
    /// are about Events gone before they were sent, or about a Combined
    /// Event Key no Event Row holds any more, and are dropped; so are those
    /// of Event Rows that build the Server Object's text unchanged. An
    /// update or a delete waits while the Mirror holds no ETag of the Server
    /// Object (ADR 0015's amendment), and so does a UID whose resource can't
    /// be built, a problem of its Events, written to the Log. No network
    /// request runs while the Local Store is locked.
    fn send(&self, remote: &dyn Remote, store: &Mutex<Store>, now: DateTime<Utc>) -> Result<()> {
        let (built, held, sending) = {
            let store = store.lock().unwrap();
            let loaded = Loaded::of_uid(&*store, self.calendar_id, self.uid)?;
            let held = store.server_object(self.calendar_id, self.uid)?;
            let built =
                resource_to_send(&loaded, held.as_ref().map(|held| held.data.as_str()), now);
            let sending = journal::sending(&store, self.calendar_id, self.uid)?;
            (built, held, sending)
        };
        let built = match built {
            Ok(built) => built,
            Err(error) => {
                log::warn!("{}", self.unbuilt_line(&error.to_string()));
                return Ok(());
            }
        };
        let outcome = match (built, held) {
            (None, None) => WriteOutcome::Done(Mirrored::NothingSent),
            (Some(text), Some(held)) if text == held.data => {
                WriteOutcome::Done(Mirrored::NothingSent)
            }
            (Some(text), None) => {
                let name = new_resource_name(self.uid);
                remote
                    .put(self.url, &name, &text, Precondition::Absent)?
                    .map(|put| Mirrored::Written(sent_object(put.href, put.etag, text)))
            }
            (built, Some(held)) => {
                let Some(etag) = &held.etag else {
                    return Ok(());
                };
                match built {
                    Some(text) => remote
                        .put(self.url, &held.href, &text, Precondition::Matches(etag))?
                        .map(|put| Mirrored::Written(sent_object(held.href, put.etag, text))),
                    None => remote
                        .delete(self.url, &held.href, etag)?
                        .map(|()| Mirrored::Deleted),
                }
            }
        };
        match outcome {
            WriteOutcome::Done(mirrored) => self.sent(store, &mirrored, &sending),
            WriteOutcome::ServerChanged => Ok(()),
        }
    }

    /// Writes `mirrored` to the Mirror and records that `sending` was sent,
    /// in one Database Transaction, so that the Mirror never holds a
    /// version whose Unsent Changes are still there to send, or the other
    /// way round. A Server Object written without an ETag is fetched by the
    /// next Incoming Sync (ADR 0015); so are the Ignored Resources holding
    /// the UID of a deleted one, one of which takes it over
    /// ([`deleted_by_sender`]).
    fn sent(
        &self,
        store: &Mutex<Store>,
        mirrored: &Mirrored,
        sending: &SentEventRows,
    ) -> Result<()> {
        store.lock().unwrap().write(|database_transaction| {
            match mirrored {
                Mirrored::NothingSent => {}
                Mirrored::Written(object) => {
                    mirror(database_transaction, self.calendar_id, object, self.uid)?;
                }
                Mirrored::Deleted => {
                    deleted_by_sender(database_transaction, self.calendar_id, self.uid)?
                }
            }
            journal::sent(database_transaction, sending)
        })
    }

    /// The Log line for a UID whose resource couldn't be built, because of
    /// `why`. The UID is untrusted, so it is escaped, which keeps a newline
    /// in it from starting another line, and cut to [`LOGGED_UID_LENGTH`]
    /// characters.
    fn unbuilt_line(&self, why: &str) -> String {
        let uid: String = self.uid.chars().take(LOGGED_UID_LENGTH).collect();
        format!(
            "Couldn't send the Events of UID {uid:?} in Calendar {}: {why:?}; their Unsent \
             Changes wait",
            self.calendar_id.0
        )
    }
}

/// The Server Object the server holds once it took `data` at `href`,
/// with the ETag `etag` it gave, if any.
fn sent_object(href: String, etag: Option<String>, data: String) -> ServerObject {
    ServerObject { href, etag, data }
}

/// The most characters of a UID that [`new_resource_name`] takes as it is.
const NAMED_UID_LENGTH: usize = 100;

/// The name of a new CalDAV resource holding the UID `uid`, relative to its
/// Calendar: the UID itself when it is at most [`NAMED_UID_LENGTH`]
/// characters of those no server or client encodes in an address, else
/// the SHA-256 of it in hex, so that an untrusted UID never makes an
/// address of its own. Either way with `.ics`. One UID always gets the same
/// name, so that a create sent again, as after FreeCal stopped before
/// storing its success, fails its precondition rather than making a second
/// resource, and the Incoming Sync mirrors the first.
fn new_resource_name(uid: &str) -> String {
    let plain = !uid.is_empty()
        && uid.len() <= NAMED_UID_LENGTH
        && uid
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if plain {
        return format!("{uid}.ics");
    }
    let hash: String = Sha256::digest(uid.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{hash}.ics")
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use chrono_tz::Tz;

    use super::*;
    use crate::event::{EventFields, StoredWhen};
    use crate::journal::{TransactionOrigin, record};
    use crate::remote::{Changes, FetchToo, KnownVersions, Put, SyncPoint};
    use crate::store::StoredEvent;
    use crate::{AvailableCalendar, Provider};

    #[test]
    fn a_new_resource_is_named_after_its_uid_only_when_no_one_encodes_it() {
        assert_eq!(new_resource_name("4f0c-9a1b_x.y"), "4f0c-9a1b_x.y.ics");
        let hashed = |uid: &str| {
            let name = new_resource_name(uid);
            assert_eq!(name.len(), 64 + 4, "{uid:?}");
            assert!(name.ends_with(".ics"));
            assert!(name[..64].bytes().all(|byte| byte.is_ascii_hexdigit()));
            name
        };
        for uid in ["", "a/b", "../up", "a b", "a@b", "ü", "a\nb", "%2F"] {
            hashed(uid);
        }
        hashed(&"x".repeat(NAMED_UID_LENGTH + 1));
        assert_eq!(
            new_resource_name(&"x".repeat(NAMED_UID_LENGTH)).len(),
            NAMED_UID_LENGTH + 4
        );
        assert_ne!(hashed("a/b"), hashed("a/c"));
    }

    #[test]
    fn the_log_line_of_a_uid_that_cant_be_sent_escapes_and_cuts_it() {
        let uid = format!("a\n{}", "x".repeat(LOGGED_UID_LENGTH));
        let to_send = ToSend {
            calendar_id: CalendarId(7),
            url: "/user/work/",
            uid: &uid,
        };

        let line = to_send.unbuilt_line("bad\ntime");

        assert_eq!(
            line,
            format!(
                "Couldn't send the Events of UID \"a\\n{}\" in Calendar 7: \"bad\\ntime\"; \
                 their Unsent Changes wait",
                "x".repeat(LOGGED_UID_LENGTH - 2)
            )
        );
        assert!(!line.contains('\n'));
    }

    /// A server that takes every create and runs `meanwhile` while it
    /// does, as another thread may change the Local Store while a request
    /// runs.
    struct Server<'a> {
        meanwhile: RefCell<Option<Box<dyn FnOnce() + 'a>>>,
    }

    impl Remote for Server<'_> {
        fn provider(&self) -> Provider {
            Provider::CalDav
        }

        fn discover(&self) -> Result<Vec<AvailableCalendar>> {
            unreachable!()
        }

        fn server_read_only(&self, _: &str) -> Result<bool> {
            unreachable!()
        }

        fn all_events(
            &self,
            _: &str,
            _: Tz,
            _: &mut dyn FnMut(Vec<ServerObject>, Tz) -> Result<()>,
        ) -> Result<SyncPoint> {
            unreachable!()
        }

        fn changes(
            &self,
            _: &str,
            _: Option<&str>,
            _: &KnownVersions,
            _: Tz,
            _: FetchToo,
        ) -> Result<Changes> {
            unreachable!()
        }

        fn put(
            &self,
            calendar: &str,
            href: &str,
            _: &str,
            precondition: Precondition,
        ) -> Result<WriteOutcome<Put>> {
            assert_eq!(precondition, Precondition::Absent);
            if let Some(meanwhile) = self.meanwhile.borrow_mut().take() {
                meanwhile();
            }
            Ok(WriteOutcome::Done(Put {
                href: format!("{calendar}{href}"),
                etag: Some("\"1\"".to_owned()),
            }))
        }

        fn delete(&self, _: &str, _: &str, _: &str) -> Result<WriteOutcome<()>> {
            unreachable!()
        }
    }

    fn dentist(calendar_id: CalendarId, title: &str) -> StoredEvent {
        StoredEvent {
            calendar_id,
            fields: EventFields {
                title: title.to_owned(),
                when: StoredWhen {
                    all_day: false,
                    start: "2026-10-05T09:00:00".to_owned(),
                    end: "2026-10-05T10:00:00".to_owned(),
                    start_zone: Some("Europe/Berlin".to_owned()),
                    end_zone: Some("Europe/Berlin".to_owned()),
                },
                location: String::new(),
                description: String::new(),
                description_html: false,
                zone_pinned: false,
                rule: None,
                exdates: String::new(),
                recurrence_id: None,
            },
        }
    }

    /// Not visible through the core interface, as a change can't be made
    /// while a request runs there, so not a seam test.
    #[test]
    fn a_change_made_while_its_uid_is_sent_stays_unsent() {
        let directory = tempfile::TempDir::new().unwrap();
        let store = Mutex::new(Store::open(&directory.path().join("freecal.sqlite3")).unwrap());
        let calendar_id = store.lock().unwrap().writable_server_calendar();
        let change = |change: &dyn Fn(&mut journal::JournalTransaction) -> Result<()>| {
            record(
                &store.lock().unwrap(),
                TransactionOrigin::UserAction,
                0,
                |journal_transaction| change(journal_transaction),
            )
            .unwrap();
        };
        let mut id = None;
        record(
            &store.lock().unwrap(),
            TransactionOrigin::UserAction,
            0,
            |journal_transaction| {
                id = Some(journal_transaction.create("dentist", &dentist(calendar_id, "Dentist"))?);
                Ok(())
            },
        )
        .unwrap();
        let id = id.unwrap();
        let server = Server {
            meanwhile: RefCell::new(Some(Box::new(|| {
                change(&|journal_transaction| {
                    journal_transaction.replace(id, &dentist(calendar_id, "Dentist, moved"))
                });
            }))),
        };
        let calendar = (
            calendar_id,
            "/user/work/".to_owned(),
            Some(SyncPoint { token: None }),
        );

        send(&server, &store, &calendar, Utc::now()).unwrap();

        let store = store.lock().unwrap();
        let held = store
            .server_object(calendar_id, "dentist")
            .unwrap()
            .unwrap();
        assert_eq!(held.href, "/user/work/dentist.ics");
        assert!(held.data.contains("SUMMARY:Dentist\r\n"));
        assert_eq!(store.unsent_uids(calendar_id).unwrap(), ["dentist"]);
    }
}
