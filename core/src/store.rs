//! The Local Store: one SQLite database (ADR 0004).

use std::path::Path;

use rusqlite::Connection;

use crate::{Account, AccountId, Provider, Result};

/// Schema migrations, applied in order. `PRAGMA user_version` records how many
/// have run.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        provider TEXT NOT NULL
    );
    INSERT INTO account (provider) VALUES ('local');
"];

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, provider FROM account ORDER BY id")?;
        let accounts = stmt
            .query_map([], |row| {
                let provider: String = row.get(1)?;
                Ok(Account {
                    id: AccountId(row.get(0)?),
                    provider: Provider::parse(&provider).unwrap_or_else(|| {
                        panic!("unknown Provider in the Local Store: {provider}")
                    }),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(accounts)
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let applied: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let tx = conn.transaction()?;
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        tx.execute_batch(migration)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    tx.commit()?;
    Ok(())
}
