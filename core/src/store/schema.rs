//! The Local Store's schema: its migrations, and the rows a new Local Store
//! starts with.

use rusqlite::{Connection, params};

use crate::{CoreError, Result};

/// Schema migrations, applied in order. `PRAGMA user_version` records how many
/// have run.
const MIGRATIONS: &[&str] = &[
    // The baseline: the schema as it was when the migrations of development
    // were squashed (ticket 63, ADR 0008). The Local Account is the one row
    // a new Local Store starts with; `app_state` has exactly one row.
    //
    // A Calendar's `read_only` is the user's mark, `server_read_only` whether
    // its server doesn't allow changing it. A server Account's Calendar has
    // the `url` it lives at, which names its Available Calendar; whether its
    // First Sync finished; and the sync token (RFC 6578) its server gave for
    // the last Incoming Sync, if it gives them. A CalDAV Account's password lives in
    // the keyring under `keyring_key`.
    //
    // An Event keeps wall times with an Event Time Zone each for start and
    // end, with the UTC instants derived (ADR 0007). AUTOINCREMENT keeps an
    // Event's ID from being given out again, so that the Change Journal
    // never takes two Events for one. An Exception is an Event with the UID
    // of its Recurring Event and the recurrence ID of the Occurrence it
    // changed. A Calendar has at most one Event with a UID that isn't an
    // Exception (ticket 104), so that Exceptions pair with one Recurring
    // Event.
    //
    // Each Provider has Mirror tables of its own (ADR 0015). A CalDAV
    // Server Object mirrors one resource, the only one at its href in its
    // Calendar, with its iCalendar text as received. It holds one UID, the
    // only one holding it in its Calendar, as a resource holding several,
    // or a UID another already holds, isn't mirrored (ADR 0015): a
    // Calendar's Event Rows with one UID find their Corresponding Server
    // Object by it. An Ignored Resource is a CalDAV resource the Sync
    // Engine fetched and doesn't mirror, with the ETag it had, so that it
    // isn't fetched again until it changes, and the UID that a mirrored
    // resource already holds; none for one holding several UIDs or not
    // read (ADR 0015). It isn't part of the Mirror, and an href is never
    // both mirrored and ignored, which the triggers enforce. A Server
    // Object or Ignored Resource without an ETag is fetched on every
    // Incoming Sync: FreeCal couldn't read the server's version, so that a
    // Server Object holds the last one it could, or the server gave none.
    // Recurrence IDs
    // everywhere are stored as obtained and compared by Occurrence, never in
    // SQL (ADR 0012).
    //
    // An Unsent Change is the Combined Event Key of an Event in a server
    // Calendar that FreeCal changed since, to be sent: its UID and
    // recurrence ID, as the Event Row holds them, and `since` when it was
    // first changed (Unix seconds). Every change but an Incoming Sync adds the Event's
    // previous and new Combined Event Key; each is there once, the two
    // indexes keeping it so with and without a recurrence ID (ADR 0015).
    //
    // A Journal Transaction records its Transaction Origin. A Journal Event
    // Modification holds the Event's previous Event Version with its Calendar,
    // none when the change created it, and the hash of its new Event Version,
    // none when the change deleted it (ADR 0009). Neither its `event_id` nor
    // its `calendar_id` has a foreign key, because what they name may be gone
    // since.
    "
    CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        provider TEXT NOT NULL,
        url TEXT,
        username TEXT,
        keyring_key TEXT
    );
    CREATE UNIQUE INDEX one_local_account ON account (provider) WHERE provider = 'local';
    INSERT INTO account (provider) VALUES ('local');
    CREATE TABLE calendar (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account (id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        colour TEXT NOT NULL,
        shown INTEGER NOT NULL DEFAULT 1,
        read_only INTEGER NOT NULL DEFAULT 0,
        url TEXT,
        synced INTEGER NOT NULL DEFAULT 0,
        sync_token TEXT,
        server_read_only INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX calendar_account ON calendar (account_id);
    CREATE UNIQUE INDEX calendar_account_url ON calendar (account_id, url) WHERE url IS NOT NULL;
    CREATE TABLE caldav_server_object (
        id INTEGER PRIMARY KEY,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        href TEXT NOT NULL,
        etag TEXT,
        data TEXT NOT NULL,
        uid TEXT NOT NULL
    );
    CREATE UNIQUE INDEX caldav_server_object_calendar_href ON caldav_server_object (calendar_id, href);
    CREATE UNIQUE INDEX caldav_server_object_calendar_uid ON caldav_server_object (calendar_id, uid);
    CREATE TABLE caldav_ignored_resource (
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        href TEXT NOT NULL,
        etag TEXT,
        uid TEXT
    );
    CREATE UNIQUE INDEX caldav_ignored_resource_calendar_href
        ON caldav_ignored_resource (calendar_id, href);
    CREATE TRIGGER caldav_ignored_resource_not_mirrored
        BEFORE INSERT ON caldav_ignored_resource
        WHEN EXISTS (SELECT 1 FROM caldav_server_object
            WHERE calendar_id = NEW.calendar_id AND href = NEW.href)
        BEGIN SELECT RAISE(ABORT, 'an href is mirrored and ignored'); END;
    CREATE TRIGGER caldav_server_object_not_ignored
        BEFORE INSERT ON caldav_server_object
        WHEN EXISTS (SELECT 1 FROM caldav_ignored_resource
            WHERE calendar_id = NEW.calendar_id AND href = NEW.href)
        BEGIN SELECT RAISE(ABORT, 'an href is mirrored and ignored'); END;
    CREATE TRIGGER caldav_server_object_moved_not_ignored
        BEFORE UPDATE OF href ON caldav_server_object
        WHEN EXISTS (SELECT 1 FROM caldav_ignored_resource
            WHERE calendar_id = NEW.calendar_id AND href = NEW.href)
        BEGIN SELECT RAISE(ABORT, 'an href is mirrored and ignored'); END;
    CREATE TABLE unsent_change (
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        uid TEXT NOT NULL,
        recurrence_id TEXT,
        since INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX unsent_change_combined_event_key ON unsent_change (calendar_id, uid, recurrence_id);
    CREATE UNIQUE INDEX unsent_change_uid ON unsent_change (calendar_id, uid)
        WHERE recurrence_id IS NULL;
    CREATE TABLE event (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        uid TEXT NOT NULL,
        title TEXT NOT NULL,
        all_day INTEGER NOT NULL,
        start_at TEXT NOT NULL,
        end_at TEXT NOT NULL,
        start_zone TEXT,
        end_zone TEXT,
        start_utc INTEGER NOT NULL,
        end_utc INTEGER NOT NULL,
        location TEXT NOT NULL,
        description TEXT NOT NULL,
        description_html INTEGER NOT NULL DEFAULT 0,
        zone_pinned INTEGER NOT NULL DEFAULT 0,
        rrule TEXT,
        exdates TEXT NOT NULL DEFAULT '',
        recurrence_id TEXT,
        CHECK (CASE WHEN all_day
            THEN start_zone IS NULL AND end_zone IS NULL
            ELSE start_zone IS NOT NULL AND end_zone IS NOT NULL END)
    );
    CREATE INDEX event_calendar_start ON event (calendar_id, start_utc);
    CREATE INDEX event_calendar_uid ON event (calendar_id, uid);
    CREATE UNIQUE INDEX event_calendar_uid_not_exception ON event (calendar_id, uid)
        WHERE recurrence_id IS NULL;
    CREATE TABLE journal_transaction (
        id INTEGER PRIMARY KEY,
        origin TEXT NOT NULL,
        made_at INTEGER NOT NULL
    );
    CREATE TABLE journal_event_modification (
        id INTEGER PRIMARY KEY,
        journal_transaction_id INTEGER NOT NULL REFERENCES journal_transaction (id) ON DELETE CASCADE,
        event_id INTEGER NOT NULL,
        uid TEXT NOT NULL,
        hash BLOB,
        undone INTEGER NOT NULL DEFAULT 0,
        calendar_id INTEGER,
        title TEXT,
        all_day INTEGER,
        start_at TEXT,
        end_at TEXT,
        start_zone TEXT,
        end_zone TEXT,
        location TEXT,
        description TEXT,
        description_html INTEGER,
        zone_pinned INTEGER,
        rrule TEXT,
        exdates TEXT,
        recurrence_id TEXT
    );
    CREATE INDEX journal_event_modification_journal_transaction
        ON journal_event_modification (journal_transaction_id);
    CREATE INDEX journal_event_modification_event ON journal_event_modification (event_id, uid);
    CREATE TABLE app_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        view TEXT NOT NULL DEFAULT 'month',
        appearance TEXT NOT NULL DEFAULT 'system',
        time_zone_rules TEXT NOT NULL DEFAULT '',
        sidebar_shown INTEGER NOT NULL DEFAULT 1,
        sidebar_width REAL
    );
    INSERT INTO app_state (id) VALUES (1);
    ",
    // Ticket 15: a server Account's or Calendar's Sync Record, so that a
    // relaunch shows how its last sync ended until it syncs again: when its
    // last sync finished (Unix seconds) and its Sync Outcome as JSON, and
    // when its last successful sync finished. All untrusted when read.
    "
    ALTER TABLE account ADD COLUMN last_sync_at INTEGER;
    ALTER TABLE account ADD COLUMN last_sync_outcome TEXT;
    ALTER TABLE account ADD COLUMN last_synced_at INTEGER;
    ALTER TABLE calendar ADD COLUMN last_sync_at INTEGER;
    ALTER TABLE calendar ADD COLUMN last_sync_outcome TEXT;
    ALTER TABLE calendar ADD COLUMN last_synced_at INTEGER;
    ",
    // Ticket 121: who counts as a Guest (see `guests.rs`). A server
    // Account's Own Addresses, as a JSON array of addresses in the form
    // they are compared in; none for the Local Account. A CalDAV Server
    // Object's attendees, derived from its text and written with it, in the
    // same form. Both untrusted when read.
    "
    ALTER TABLE account ADD COLUMN own_addresses TEXT;
    ALTER TABLE caldav_server_object ADD COLUMN attendees TEXT NOT NULL DEFAULT '[]';
    ",
];

/// The Local Calendars a new Local Store starts with: name, colour and whether
/// it is shown. The colours are the first two of the frontend's
/// `NEW_CALENDAR_ORDER`. The names are English because v1 ships English only.
const STARTING_CALENDARS: &[(&str, &str, bool)] =
    &[("Work", "#3f51b5", true), ("Home", "#ad1457", false)];

/// Puts the starting Calendars into the Local Account of a Local Store that
/// was just created. An existing Local Store never gets them, so deleted
/// ones never come back.
fn add_starting_calendars(conn: &Connection) -> Result<()> {
    for (name, colour, shown) in STARTING_CALENDARS {
        conn.execute(
            "INSERT INTO calendar (account_id, name, colour, shown) \
             SELECT id, ?1, ?2, ?3 FROM account WHERE provider = 'local'",
            params![name, colour, shown],
        )?;
    }
    Ok(())
}

/// Brings the schema to this build's, inside the transaction opening the
/// Local Store began. A new Local Store gets the starting Calendars.
pub(super) fn migrate(conn: &Connection) -> Result<()> {
    let applied: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let known = MIGRATIONS.len() as i64;
    if applied > known {
        return Err(CoreError::NewerLocalStore {
            found: applied,
            known,
        });
    }
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        conn.execute_batch(migration)?;
        conn.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    if applied == 0 {
        add_starting_calendars(conn)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0015: the Local Store refuses an href both mirrored and ignored,
    /// whether the Ignored Resource or the Server Object comes second, or
    /// a Server Object moves to it.
    #[test]
    fn an_href_is_never_both_mirrored_and_ignored() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO caldav_server_object (calendar_id, href, data, uid) \
             VALUES (1, '/mirrored.ics', '', 'mirrored');
             INSERT INTO caldav_ignored_resource (calendar_id, href) VALUES (1, '/ignored.ics');",
        )
        .unwrap();

        for refused in [
            "INSERT INTO caldav_ignored_resource (calendar_id, href) VALUES (1, '/mirrored.ics')",
            "INSERT INTO caldav_server_object (calendar_id, href, data, uid) \
             VALUES (1, '/ignored.ics', '', 'ignored')",
            "UPDATE caldav_server_object SET href = '/ignored.ics'",
        ] {
            assert!(conn.execute(refused, []).is_err(), "{refused}");
        }
        conn.execute(
            "INSERT INTO caldav_ignored_resource (calendar_id, href) VALUES (2, '/mirrored.ics')",
            [],
        )
        .unwrap();
    }
}
