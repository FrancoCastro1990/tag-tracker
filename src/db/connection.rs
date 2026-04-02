use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.configure_pragmas()?;
        db.initialize_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.configure_pragmas()?;
        db.initialize_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn configure_pragmas(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(())
    }

    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS trackers (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL UNIQUE,
                color       TEXT    NOT NULL,
                icon_path   TEXT,
                hourly_rate INTEGER NOT NULL,
                state       TEXT    NOT NULL DEFAULT 'created',
                created_at  TEXT    NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                tracker_id  INTEGER NOT NULL REFERENCES trackers(id) ON DELETE CASCADE,
                started_at  TEXT    NOT NULL,
                ended_at    TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_tracker_id ON sessions(tracker_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
            ",
        )?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<()> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version < 1 {
            self.conn.execute_batch("PRAGMA user_version = 1;")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_creates_tables() {
        let db = Database::in_memory().unwrap();
        let tables: Vec<String> = db
            .conn()
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"trackers".to_string()));
        assert!(tables.contains(&"sessions".to_string()));
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let db = Database::in_memory().unwrap();
        let fk: i64 = db
            .conn()
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn idempotent_schema_creation() {
        let db1 = Database::in_memory().unwrap();
        let db2 = Database::in_memory().unwrap();
        assert!(db1.conn().is_autocommit());
        assert!(db2.conn().is_autocommit());
    }
}
