//! The writes no guard holds: accounts, Calendars, settings, the sidebar,
//! the last-used view and the time zone rules.

use rusqlite::params;

use crate::store::Store;
use crate::{
    Account, AccountId, Appearance, AvailableCalendar, Calendar, CalendarId, Colour, CoreError,
    Provider, Result, ServerLogin, SidebarState, View,
};

/// Whether a newly added server Calendar starts marked read-only, so that
/// the user can watch sync work before trusting FreeCal to write (US-18).
/// Until 1.0.
const NEW_SERVER_CALENDARS_READ_ONLY: bool = true;

impl Store {
    /// Adds a server Account whose secret is in the keyring under
    /// `keyring_key`.
    pub fn insert_server_account(
        &self,
        provider: Provider,
        server: &ServerLogin,
        keyring_key: &str,
    ) -> Result<Account> {
        self.conn.execute(
            "INSERT INTO account (provider, url, username, keyring_key) VALUES (?1, ?2, ?3, ?4)",
            params![provider.as_str(), server.url, server.username, keyring_key],
        )?;
        Ok(Account {
            id: AccountId(self.conn.last_insert_rowid()),
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
        self.conn.execute(
            "INSERT INTO calendar (account_id, name, colour) VALUES (?1, ?2, ?3)",
            params![account_id.0, name, colour.as_str()],
        )?;
        Ok(Calendar {
            id: CalendarId(self.conn.last_insert_rowid()),
            account_id,
            name: name.to_owned(),
            colour: colour.clone(),
            shown: true,
            read_only: false,
            server_read_only: false,
            first_sync: false,
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
        self.conn.execute(
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
        self.find_calendar(CalendarId(self.conn.last_insert_rowid()))
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

    /// Shows `id` and hides every other Calendar, in one statement so that
    /// nothing changes when `id` doesn't exist.
    pub fn show_only_calendar(&self, id: CalendarId) -> Result<()> {
        match self.conn.execute(
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
        match self.conn.execute(sql, params![id.0, value])? {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    pub fn set_last_used_view(&self, view: View) -> Result<()> {
        self.conn
            .execute("UPDATE app_state SET view = ?1", [view.as_str()])?;
        Ok(())
    }

    pub fn set_sidebar(&self, sidebar: SidebarState) -> Result<()> {
        self.conn.execute(
            "UPDATE app_state SET sidebar_shown = ?1, sidebar_width = ?2",
            params![sidebar.shown, sidebar.width],
        )?;
        Ok(())
    }

    pub fn set_appearance(&self, appearance: Appearance) -> Result<()> {
        self.conn.execute(
            "UPDATE app_state SET appearance = ?1",
            [appearance.as_str()],
        )?;
        Ok(())
    }

    /// Records the time zone rules the derived UTC instants are computed
    /// with now.
    pub(crate) fn set_time_zone_rules(&self, rules: &str) -> Result<()> {
        self.conn
            .execute("UPDATE app_state SET time_zone_rules = ?1", [rules])?;
        Ok(())
    }
}
