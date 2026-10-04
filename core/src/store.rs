//! The Local Store: one SQLite database (ADR 0004).

use std::path::Path;

use rusqlite::{Connection, TransactionBehavior, params};

use crate::{Account, AccountId, Calendar, CalendarId, Colour, CoreError, Provider, Result};

/// Schema migrations, applied in order. `PRAGMA user_version` records how many
/// have run.
const MIGRATIONS: &[&str] = &[
    "
    CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        provider TEXT NOT NULL
    );
    INSERT INTO account (provider) VALUES ('local');
    ",
    "
    CREATE TABLE calendar (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account (id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        colour TEXT NOT NULL,
        shown INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX calendar_account ON calendar (account_id);
    ",
];

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, provider FROM account ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<(i64, String)>>>()?;
        rows.into_iter()
            .map(|(id, provider)| {
                Ok(Account {
                    id: AccountId(id),
                    provider: Provider::parse(&provider)
                        .ok_or(CoreError::UnknownProvider(provider))?,
                })
            })
            .collect()
    }

    pub fn find_account(&self, id: AccountId) -> Result<Option<Account>> {
        Ok(self
            .list_accounts()?
            .into_iter()
            .find(|account| account.id == id))
    }

    pub fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, account_id, name, colour, shown FROM calendar ORDER BY id")?;
        let calendars = stmt
            .query_map([], |row| {
                Ok(Calendar {
                    id: CalendarId(row.get(0)?),
                    account_id: AccountId(row.get(1)?),
                    name: row.get(2)?,
                    // A stored colour is untrusted like every stored field.
                    colour: Colour::parse(&row.get::<_, String>(3)?).unwrap_or_default(),
                    shown: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(calendars)
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
        })
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

    pub fn delete_calendar(&self, id: CalendarId) -> Result<()> {
        match self
            .conn
            .execute("DELETE FROM calendar WHERE id = ?1", [id.0])?
        {
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
}

fn migrate(conn: &mut Connection) -> Result<()> {
    // IMMEDIATE takes the write lock before `user_version` is read, so two
    // FreeCal processes opening a fresh Local Store at once can't both see
    // version 0 and both try to create the schema.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let applied: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let known = MIGRATIONS.len() as i64;
    if applied > known {
        return Err(CoreError::NewerLocalStore {
            found: applied,
            known,
        });
    }
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        tx.execute_batch(migration)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    tx.commit()?;
    Ok(())
}
