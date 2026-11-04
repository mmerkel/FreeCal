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
    // first sync finished; and the sync token (RFC 6578) its server gave for
    // the last sync, if it gives them. A CalDAV Account's password lives in
    // the keyring under `keyring_key`.
    //
    // An Event keeps wall times with an Event Time Zone each for start and
    // end, with the UTC instants derived (ADR 0007). AUTOINCREMENT keeps an
    // Event's ID from being given out again, so that the Change Journal
    // never takes two Events for one. An Exception is an Event with the UID
    // of its Recurring Event and the recurrence ID of the Occurrence it
    // changed.
    //
    // Server Objects mirror the server, with their data as received in its
    // `format`, and `unreadable` once FreeCal couldn't read the server's
    // version, so that the Mirror holds the last version it could (ADR
    // 0010). Each Server Object's Combined Event Keys are the UID and
    // recurrence ID of each of its components, the recurrence ID normalised
    // as the Event Rows hold it: a Calendar's Event Rows with one UID find
    // the Server Object they belong to through them (ADR 0011).
    //
    // An Unsent Change is the Combined Event Key of an Event in a server
    // Calendar that FreeCal changed since, to be sent: its UID and
    // recurrence ID, as the Event Rows hold them, and `since` when it was
    // first changed (Unix seconds). Every change but a sync adds the Event's
    // previous and new Combined Event Key; each is there once, the two
    // indexes keeping it so with and without a recurrence ID (ADR 0011).
    //
    // A Journal Transaction records its Transaction Origin. A Modification
    // holds the Event's previous Event Version with its Calendar, none when
    // the change created it, and the hash of its new Event Version, none
    // when the change deleted it (ADR 0009). Neither its `event_id` nor its
    // `calendar_id` has a foreign key, because what they name may be gone
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
    CREATE TABLE server_object (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        href TEXT NOT NULL,
        etag TEXT,
        data TEXT NOT NULL,
        format TEXT NOT NULL,
        unreadable INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX server_object_calendar_href ON server_object (calendar_id, href);
    CREATE TABLE combined_event_key (
        object_id INTEGER NOT NULL REFERENCES server_object (id) ON DELETE CASCADE,
        uid TEXT NOT NULL,
        recurrence_id TEXT
    );
    CREATE INDEX combined_event_key_object ON combined_event_key (object_id);
    CREATE INDEX combined_event_key_uid ON combined_event_key (uid);
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
    CREATE TABLE journal_transaction (
        id INTEGER PRIMARY KEY,
        origin TEXT NOT NULL,
        made_at INTEGER NOT NULL
    );
    CREATE TABLE modification (
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
    CREATE INDEX modification_journal_transaction ON modification (journal_transaction_id);
    CREATE INDEX modification_event ON modification (event_id, uid);
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
