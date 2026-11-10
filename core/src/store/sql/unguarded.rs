//! The Unguarded Writes: accounts, Calendars, settings, the sidebar, the
//! last-used view and the time zone rules. Each is a method of a
//! [`DatabaseTransaction`] and takes no Write Permission.

use chrono::{DateTime, FixedOffset};
use rusqlite::params;

use crate::guests;
use crate::store::DatabaseTransaction;
use crate::sync_status::SyncOutcome;
use crate::{
    Account, AccountId, Appearance, AvailableCalendar, Calendar, CalendarId, Colour, CoreError,
    Provider, Result, ServerLogin, SidebarState, View,
};

/// Whether a newly added server Calendar starts marked read-only, so that
/// the user can watch sync work before trusting FreeCal to write (US-18).
/// Until 1.0.
const NEW_SERVER_CALENDARS_READ_ONLY: bool = true;

impl DatabaseTransaction<'_> {
    /// Adds a server Account whose secret is in the keyring under
    /// `keyring_key`, with its Own Addresses (see [`guests`]).
    pub fn insert_server_account(
        &self,
        provider: Provider,
        server: &ServerLogin,
        keyring_key: &str,
        own_addresses: &[String],
    ) -> Result<Account> {
        self.tx.execute(
            "INSERT INTO account (provider, url, username, keyring_key, own_addresses) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                provider.as_str(),
                server.url,
                server.username,
                keyring_key,
                guests::stored(own_addresses)
            ],
        )?;
        Ok(Account {
            id: AccountId(self.tx.last_insert_rowid()),
            provider,
            server: Some(server.clone()),
        })
    }

    pub fn insert_calendar(
        &self,
        account_id: AccountId,
        name: &str,
        colour: &Colour,
    ) -> Result<Calendar> {
        self.tx.execute(
            "INSERT INTO calendar (account_id, name, colour) VALUES (?1, ?2, ?3)",
            params![account_id.0, name, colour.as_str()],
        )?;
        Ok(Calendar {
            id: CalendarId(self.tx.last_insert_rowid()),
            account_id,
            name: name.to_owned(),
            colour: colour.clone(),
            shown: true,
            read_only: false,
            server_read_only: false,
        })
    }

    /// Adds an Available Calendar as a server Account's Calendar, in the
    /// default colour if it has none, read-only if its server says so.
    pub fn insert_server_calendar(
        &self,
        account_id: AccountId,
        available: &AvailableCalendar,
    ) -> Result<Calendar> {
        let colour = available.colour.clone().unwrap_or_default();
        self.tx.execute(
            "INSERT INTO calendar (account_id, url, name, colour, read_only, server_read_only) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                account_id.0,
                available.url,
                available.name,
                colour.as_str(),
                NEW_SERVER_CALENDARS_READ_ONLY,
                available.read_only
            ],
        )?;
        self.find_calendar(CalendarId(self.tx.last_insert_rowid()))
    }

    /// Records that a Calendar's Incoming Sync finished, with the sync token its
    /// server gave, if any.
    pub fn set_synced(&self, id: CalendarId, token: Option<&str>) -> Result<()> {
        self.update_calendar(
            "UPDATE calendar SET synced = 1, sync_token = ?2 WHERE id = ?1",
            id,
            token,
        )
    }

    /// Keeps in the server Account's Sync Record that a sync finished `at`
    /// with `outcome`, and as its last successful one when Synced. An
    /// Account removed meanwhile, or the Local Account, is left as it is.
    pub fn finish_account_sync(
        &self,
        id: AccountId,
        at: DateTime<FixedOffset>,
        outcome: &SyncOutcome,
    ) -> Result<()> {
        self.finish_sync(
            "UPDATE account SET last_sync_at = ?2, last_sync_outcome = ?3, \
                 last_synced_at = CASE WHEN ?4 THEN ?2 ELSE last_synced_at END \
             WHERE id = ?1 AND provider != 'local'",
            id.0,
            at,
            outcome,
        )
    }

    /// [`Self::finish_account_sync`] for a server Account's Calendar.
    pub fn finish_calendar_sync(
        &self,
        id: CalendarId,
        at: DateTime<FixedOffset>,
        outcome: &SyncOutcome,
    ) -> Result<()> {
        self.finish_sync(
            "UPDATE calendar SET last_sync_at = ?2, last_sync_outcome = ?3, \
                 last_synced_at = CASE WHEN ?4 THEN ?2 ELSE last_synced_at END \
             WHERE id = ?1 \
               AND account_id IN (SELECT id FROM account WHERE provider != 'local')",
            id.0,
            at,
            outcome,
        )
    }

    fn finish_sync(
        &self,
        sql: &str,
        id: i64,
        at: DateTime<FixedOffset>,
        outcome: &SyncOutcome,
    ) -> Result<()> {
        let synced = *outcome == SyncOutcome::Synced;
        self.tx
            .execute(sql, params![id, at.timestamp(), outcome.stored(), synced])?;
        Ok(())
    }

    pub fn set_calendar_name(&self, id: CalendarId, name: &str) -> Result<()> {
        self.update_calendar("UPDATE calendar SET name = ?2 WHERE id = ?1", id, name)
    }

    pub fn set_calendar_colour(&self, id: CalendarId, colour: &Colour) -> Result<()> {
        self.update_calendar(
            "UPDATE calendar SET colour = ?2 WHERE id = ?1",
            id,
            colour.as_str(),
        )
    }

    pub fn set_calendar_shown(&self, id: CalendarId, shown: bool) -> Result<()> {
        self.update_calendar("UPDATE calendar SET shown = ?2 WHERE id = ?1", id, shown)
    }

    /// Sets the user's read-only mark of a Calendar (US-17, US-19).
    pub fn set_calendar_read_only(&self, id: CalendarId, read_only: bool) -> Result<()> {
        self.update_calendar(
            "UPDATE calendar SET read_only = ?2 WHERE id = ?1",
            id,
            read_only,
        )
    }

    /// Keeps whether a server Calendar's server doesn't let the user change
    /// it, as a sync read it. Returns whether that changed; not for a
    /// Calendar removed meanwhile.
    pub fn set_server_read_only(&self, id: CalendarId, read_only: bool) -> Result<bool> {
        let changed = self.tx.execute(
            "UPDATE calendar SET server_read_only = ?2 WHERE id = ?1 AND server_read_only != ?2",
            params![id.0, read_only],
        )?;
        Ok(changed > 0)
    }

    /// Shows `id` and hides every other Calendar, in one statement so that
    /// nothing changes when `id` doesn't exist.
    pub fn show_only_calendar(&self, id: CalendarId) -> Result<()> {
        match self.tx.execute(
            "UPDATE calendar SET shown = (id = ?1) \
             WHERE EXISTS (SELECT 1 FROM calendar WHERE id = ?1)",
            [id.0],
        )? {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    fn update_calendar(
        &self,
        sql: &str,
        id: CalendarId,
        value: impl rusqlite::ToSql,
    ) -> Result<()> {
        match self.tx.execute(sql, params![id.0, value])? {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    pub fn set_last_used_view(&self, view: View) -> Result<()> {
        self.tx
            .execute("UPDATE app_state SET view = ?1", [view.as_str()])?;
        Ok(())
    }

    pub fn set_sidebar(&self, sidebar: SidebarState) -> Result<()> {
        self.tx.execute(
            "UPDATE app_state SET sidebar_shown = ?1, sidebar_width = ?2",
            params![sidebar.shown, sidebar.width],
        )?;
        Ok(())
    }

    pub fn set_appearance(&self, appearance: Appearance) -> Result<()> {
        self.tx.execute(
            "UPDATE app_state SET appearance = ?1",
            [appearance.as_str()],
        )?;
        Ok(())
    }

    pub fn set_run_in_background(&self, on: bool) -> Result<()> {
        self.tx
            .execute("UPDATE app_state SET run_in_background = ?1", [on])?;
        Ok(())
    }

    pub fn set_start_at_login(&self, on: bool) -> Result<()> {
        self.tx
            .execute("UPDATE app_state SET start_at_login = ?1", [on])?;
        Ok(())
    }

    /// Records the time zone rules the derived UTC instants are computed
    /// with now.
    pub(crate) fn set_time_zone_rules(&self, rules: &str) -> Result<()> {
        self.tx
            .execute("UPDATE app_state SET time_zone_rules = ?1", [rules])?;
        Ok(())
    }
}
