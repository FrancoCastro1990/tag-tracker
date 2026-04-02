# tag-tracker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI that tracks time across multiple activities ("trackers"), displays the active one in waybar, and calculates daily earnings in CLP.

**Architecture:** Single binary + waybar polling. CLI commands modify SQLite directly. Waybar calls `tag-tracker waybar` every 5s (with signal 10 for instant refresh). No daemon, no IPC.

**Tech Stack:** Rust (edition 2024), clap 4 (derive, subcommands), rusqlite 0.32 (bundled), chrono 0.4, serde/serde_json, thiserror 2, directories 6, colored 2.

---

## File Structure

```
tag-tracker/
├── Cargo.toml
├── src/
│   ├── main.rs              # entry point, clap subcommand dispatch
│   ├── error.rs             # AppError enum, Result<T> alias
│   ├── db/
│   │   ├── mod.rs           # pub mod connection, tracker_repo, session_repo
│   │   ├── connection.rs    # Database struct, schema, migrations
│   │   ├── tracker_repo.rs  # TrackerRepo CRUD
│   │   └── session_repo.rs  # SessionRepo CRUD + time calculations
│   ├── domain/
│   │   ├── mod.rs           # pub mod tracker, session
│   │   ├── tracker.rs       # Tracker struct, TrackerState enum
│   │   └── session.rs       # Session struct
│   ├── cli/
│   │   ├── mod.rs           # Cli struct with clap derive, Command enum
│   │   └── commands.rs      # handler functions for each command
│   └── waybar/
│       ├── mod.rs           # pub mod output
│       └── output.rs        # WaybarOutput struct, JSON generation
```

---

### Task 1: Project scaffold and error module

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/error.rs`

- [ ] **Step 1: Initialize the Rust project**

```bash
cd /home/franco/proyectos-personales/tag-tracker
cargo init --name tag-tracker
```

- [ ] **Step 2: Replace Cargo.toml with project dependencies**

```toml
[package]
name = "tag-tracker"
version = "0.1.0"
edition = "2024"
description = "CLI time tracker with waybar integration"
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
directories = "6"
colored = "2"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Create src/error.rs**

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

- [ ] **Step 4: Create minimal src/main.rs**

```rust
mod error;

fn main() {
    println!("tag-tracker");
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git init
git add Cargo.toml Cargo.lock src/main.rs src/error.rs
git commit -m "feat: scaffold project with error module"
```

---

### Task 2: Domain models

**Files:**
- Create: `src/domain/mod.rs`
- Create: `src/domain/tracker.rs`
- Create: `src/domain/session.rs`

- [ ] **Step 1: Create src/domain/mod.rs**

```rust
pub mod session;
pub mod tracker;
```

- [ ] **Step 2: Create src/domain/tracker.rs**

```rust
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackerState {
    Created,
    Active,
    Paused,
}

impl fmt::Display for TrackerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackerState::Created => write!(f, "created"),
            TrackerState::Active => write!(f, "active"),
            TrackerState::Paused => write!(f, "paused"),
        }
    }
}

impl FromStr for TrackerState {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "created" => Ok(TrackerState::Created),
            "active" => Ok(TrackerState::Active),
            "paused" => Ok(TrackerState::Paused),
            other => Err(format!("Invalid tracker state: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracker {
    pub id: Option<i64>,
    pub name: String,
    pub color: String,
    pub icon_path: Option<String>,
    pub hourly_rate: i64,
    pub state: TrackerState,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_state_display() {
        assert_eq!(TrackerState::Created.to_string(), "created");
        assert_eq!(TrackerState::Active.to_string(), "active");
        assert_eq!(TrackerState::Paused.to_string(), "paused");
    }

    #[test]
    fn tracker_state_from_str() {
        assert_eq!("created".parse::<TrackerState>().unwrap(), TrackerState::Created);
        assert_eq!("active".parse::<TrackerState>().unwrap(), TrackerState::Active);
        assert_eq!("paused".parse::<TrackerState>().unwrap(), TrackerState::Paused);
        assert!("invalid".parse::<TrackerState>().is_err());
    }
}
```

- [ ] **Step 3: Create src/domain/session.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub tracker_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
}
```

- [ ] **Step 4: Wire domain into main.rs**

Add `mod domain;` to `src/main.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test domain`
Expected: 2 tests pass (tracker_state_display, tracker_state_from_str).

- [ ] **Step 6: Commit**

```bash
git add src/domain/ src/main.rs
git commit -m "feat: add domain models (Tracker, Session, TrackerState)"
```

---

### Task 3: Database connection and schema

**Files:**
- Create: `src/db/mod.rs`
- Create: `src/db/connection.rs`

- [ ] **Step 1: Create src/db/mod.rs**

```rust
pub mod connection;
pub mod session_repo;
pub mod tracker_repo;
```

- [ ] **Step 2: Create src/db/connection.rs with tests**

```rust
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
```

- [ ] **Step 3: Wire db into main.rs**

Add `mod db;` to `src/main.rs`.

- [ ] **Step 4: Run tests**

Run: `cargo test db::connection`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/db/ src/main.rs
git commit -m "feat: add database connection with schema and migrations"
```

---

### Task 4: TrackerRepo with tests

**Files:**
- Create: `src/db/tracker_repo.rs`

- [ ] **Step 1: Write the failing test file first**

Create `src/db/tracker_repo.rs` with the full implementation and tests:

```rust
use crate::db::connection::Database;
use crate::domain::tracker::{Tracker, TrackerState};
use crate::error::{AppError, Result};

pub struct TrackerRepo<'a> {
    db: &'a Database,
}

impl<'a> TrackerRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, tracker: &Tracker) -> Result<i64> {
        self.db.conn().execute(
            "INSERT INTO trackers (name, color, icon_path, hourly_rate, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                tracker.name,
                tracker.color,
                tracker.icon_path,
                tracker.hourly_rate,
                tracker.state.to_string(),
                tracker.created_at,
            ],
        )?;
        Ok(self.db.conn().last_insert_rowid())
    }

    pub fn get_by_id(&self, id: i64) -> Result<Tracker> {
        self.db
            .conn()
            .query_row(
                "SELECT id, name, color, icon_path, hourly_rate, state, created_at FROM trackers WHERE id = ?1",
                rusqlite::params![id],
                row_to_tracker,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("Tracker with id {id}"))
                }
                other => AppError::Database(other),
            })
    }

    pub fn get_all(&self) -> Result<Vec<Tracker>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, name, color, icon_path, hourly_rate, state, created_at FROM trackers ORDER BY name",
        )?;
        let trackers = stmt
            .query_map([], row_to_tracker)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(trackers)
    }

    pub fn find_by_name(&self, name: &str) -> Result<Option<Tracker>> {
        let result = self.db.conn().query_row(
            "SELECT id, name, color, icon_path, hourly_rate, state, created_at FROM trackers WHERE name = ?1",
            rusqlite::params![name],
            row_to_tracker,
        );
        match result {
            Ok(tracker) => Ok(Some(tracker)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    pub fn find_active(&self) -> Result<Option<Tracker>> {
        let result = self.db.conn().query_row(
            "SELECT id, name, color, icon_path, hourly_rate, state, created_at FROM trackers WHERE state = 'active'",
            [],
            row_to_tracker,
        );
        match result {
            Ok(tracker) => Ok(Some(tracker)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    pub fn update(&self, tracker: &Tracker) -> Result<()> {
        let id = tracker
            .id
            .ok_or_else(|| AppError::Validation("Cannot update a tracker without an id.".into()))?;
        let affected = self.db.conn().execute(
            "UPDATE trackers SET name = ?1, color = ?2, icon_path = ?3, hourly_rate = ?4, state = ?5 WHERE id = ?6",
            rusqlite::params![
                tracker.name,
                tracker.color,
                tracker.icon_path,
                tracker.hourly_rate,
                tracker.state.to_string(),
                id,
            ],
        )?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Tracker with id {id}")));
        }
        Ok(())
    }

    pub fn update_state(&self, id: i64, state: TrackerState) -> Result<()> {
        let affected = self.db.conn().execute(
            "UPDATE trackers SET state = ?1 WHERE id = ?2",
            rusqlite::params![state.to_string(), id],
        )?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Tracker with id {id}")));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM trackers WHERE id = ?1", rusqlite::params![id])?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Tracker with id {id}")));
        }
        Ok(())
    }
}

fn row_to_tracker(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tracker> {
    let state_str: String = row.get(5)?;
    let state = state_str
        .parse::<TrackerState>()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?;
    Ok(Tracker {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        icon_path: row.get(3)?,
        hourly_rate: row.get(4)?,
        state,
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Database {
        Database::in_memory().unwrap()
    }

    fn sample_tracker(name: &str) -> Tracker {
        Tracker {
            id: None,
            name: name.to_string(),
            color: "#55a555".to_string(),
            icon_path: None,
            hourly_rate: 15000,
            state: TrackerState::Created,
            created_at: "2026-04-01T10:00:00".to_string(),
        }
    }

    #[test]
    fn create_and_get() {
        let db = setup();
        let repo = TrackerRepo::new(&db);

        let id = repo.create(&sample_tracker("Work A")).unwrap();
        assert!(id > 0);

        let fetched = repo.get_by_id(id).unwrap();
        assert_eq!(fetched.name, "Work A");
        assert_eq!(fetched.color, "#55a555");
        assert_eq!(fetched.hourly_rate, 15000);
        assert_eq!(fetched.state, TrackerState::Created);
    }

    #[test]
    fn get_all() {
        let db = setup();
        let repo = TrackerRepo::new(&db);

        repo.create(&sample_tracker("B")).unwrap();
        repo.create(&sample_tracker("A")).unwrap();

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "A"); // ordered by name
    }

    #[test]
    fn find_by_name() {
        let db = setup();
        let repo = TrackerRepo::new(&db);
        repo.create(&sample_tracker("Test")).unwrap();

        assert!(repo.find_by_name("Test").unwrap().is_some());
        assert!(repo.find_by_name("Nope").unwrap().is_none());
    }

    #[test]
    fn find_active() {
        let db = setup();
        let repo = TrackerRepo::new(&db);

        let id = repo.create(&sample_tracker("Work")).unwrap();
        assert!(repo.find_active().unwrap().is_none());

        repo.update_state(id, TrackerState::Active).unwrap();
        let active = repo.find_active().unwrap().unwrap();
        assert_eq!(active.name, "Work");
    }

    #[test]
    fn update_tracker() {
        let db = setup();
        let repo = TrackerRepo::new(&db);

        let id = repo.create(&sample_tracker("Old")).unwrap();
        let mut tracker = repo.get_by_id(id).unwrap();
        tracker.name = "New".to_string();
        tracker.hourly_rate = 20000;
        repo.update(&tracker).unwrap();

        let updated = repo.get_by_id(id).unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.hourly_rate, 20000);
    }

    #[test]
    fn delete_tracker() {
        let db = setup();
        let repo = TrackerRepo::new(&db);

        let id = repo.create(&sample_tracker("Bye")).unwrap();
        repo.delete(id).unwrap();
        assert!(repo.get_by_id(id).is_err());
    }

    #[test]
    fn unique_name_constraint() {
        let db = setup();
        let repo = TrackerRepo::new(&db);

        repo.create(&sample_tracker("Unique")).unwrap();
        assert!(repo.create(&sample_tracker("Unique")).is_err());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test db::tracker_repo`
Expected: 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/db/tracker_repo.rs
git commit -m "feat: add TrackerRepo with CRUD operations"
```

---

### Task 5: SessionRepo with time calculations

**Files:**
- Create: `src/db/session_repo.rs`

- [ ] **Step 1: Create src/db/session_repo.rs with full implementation and tests**

```rust
use chrono::{Local, NaiveDate};

use crate::db::connection::Database;
use crate::domain::session::Session;
use crate::error::{AppError, Result};

pub struct SessionRepo<'a> {
    db: &'a Database,
}

impl<'a> SessionRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn start(&self, tracker_id: i64) -> Result<i64> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        self.db.conn().execute(
            "INSERT INTO sessions (tracker_id, started_at) VALUES (?1, ?2)",
            rusqlite::params![tracker_id, now],
        )?;
        Ok(self.db.conn().last_insert_rowid())
    }

    pub fn stop_active(&self, tracker_id: i64) -> Result<()> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        self.db.conn().execute(
            "UPDATE sessions SET ended_at = ?1 WHERE tracker_id = ?2 AND ended_at IS NULL",
            rusqlite::params![now, tracker_id],
        )?;
        Ok(())
    }

    pub fn today_seconds(&self, tracker_id: i64) -> Result<i64> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.today_seconds_for_date(tracker_id, &today)
    }

    /// Calculate total seconds for a tracker on a given date.
    /// For sessions still active (ended_at IS NULL), uses current time as end.
    fn today_seconds_for_date(&self, tracker_id: i64, date: &str) -> Result<i64> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let total: i64 = self.db.conn().query_row(
            "SELECT COALESCE(SUM(
                strftime('%s', COALESCE(ended_at, ?1)) - strftime('%s', started_at)
            ), 0)
            FROM sessions
            WHERE tracker_id = ?2 AND date(started_at) = ?3",
            rusqlite::params![now, tracker_id, date],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    pub fn close_stale_sessions(&self) -> Result<u64> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let affected = self.db.conn().execute(
            "UPDATE sessions SET ended_at = date(started_at) || 'T23:59:59'
             WHERE ended_at IS NULL AND date(started_at) < ?1",
            rusqlite::params![today],
        )?;
        Ok(affected as u64)
    }

    pub fn delete_by_tracker(&self, tracker_id: i64) -> Result<()> {
        self.db.conn().execute(
            "DELETE FROM sessions WHERE tracker_id = ?1",
            rusqlite::params![tracker_id],
        )?;
        Ok(())
    }
}

pub fn format_duration(total_seconds: i64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

pub fn calculate_earnings(total_seconds: i64, hourly_rate: i64) -> i64 {
    (total_seconds as f64 / 3600.0 * hourly_rate as f64).round() as i64
}

pub fn format_clp(amount: i64) -> String {
    let s = amount.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    format!("${}", result.chars().rev().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tracker_repo::TrackerRepo;
    use crate::domain::tracker::{Tracker, TrackerState};

    fn setup() -> Database {
        Database::in_memory().unwrap()
    }

    fn create_tracker(db: &Database, name: &str) -> i64 {
        let repo = TrackerRepo::new(db);
        repo.create(&Tracker {
            id: None,
            name: name.to_string(),
            color: "#55a555".to_string(),
            icon_path: None,
            hourly_rate: 15000,
            state: TrackerState::Created,
            created_at: "2026-04-01T10:00:00".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn start_creates_open_session() {
        let db = setup();
        let tracker_id = create_tracker(&db, "Test");
        let repo = SessionRepo::new(&db);

        let session_id = repo.start(tracker_id).unwrap();
        assert!(session_id > 0);

        // Session should have NULL ended_at
        let ended: Option<String> = db
            .conn()
            .query_row(
                "SELECT ended_at FROM sessions WHERE id = ?1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(ended.is_none());
    }

    #[test]
    fn stop_active_closes_session() {
        let db = setup();
        let tracker_id = create_tracker(&db, "Test");
        let repo = SessionRepo::new(&db);

        repo.start(tracker_id).unwrap();
        repo.stop_active(tracker_id).unwrap();

        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE tracker_id = ?1 AND ended_at IS NULL",
                rusqlite::params![tracker_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn today_seconds_returns_zero_for_no_sessions() {
        let db = setup();
        let tracker_id = create_tracker(&db, "Test");
        let repo = SessionRepo::new(&db);

        assert_eq!(repo.today_seconds(tracker_id).unwrap(), 0);
    }

    #[test]
    fn today_seconds_with_closed_session() {
        let db = setup();
        let tracker_id = create_tracker(&db, "Test");
        let today = Local::now().format("%Y-%m-%d").to_string();

        // Insert a closed 1-hour session
        db.conn()
            .execute(
                "INSERT INTO sessions (tracker_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    tracker_id,
                    format!("{today}T10:00:00"),
                    format!("{today}T11:00:00"),
                ],
            )
            .unwrap();

        let repo = SessionRepo::new(&db);
        let seconds = repo.today_seconds(tracker_id).unwrap();
        assert_eq!(seconds, 3600); // 1 hour
    }

    #[test]
    fn close_stale_sessions_closes_old_sessions() {
        let db = setup();
        let tracker_id = create_tracker(&db, "Test");

        // Insert an open session from yesterday
        db.conn()
            .execute(
                "INSERT INTO sessions (tracker_id, started_at) VALUES (?1, '2020-01-01T10:00:00')",
                rusqlite::params![tracker_id],
            )
            .unwrap();

        let repo = SessionRepo::new(&db);
        let closed = repo.close_stale_sessions().unwrap();
        assert_eq!(closed, 1);

        // Should now have ended_at set
        let ended: Option<String> = db
            .conn()
            .query_row(
                "SELECT ended_at FROM sessions WHERE tracker_id = ?1",
                rusqlite::params![tracker_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(ended.is_some());
    }

    #[test]
    fn format_duration_tests() {
        assert_eq!(format_duration(0), "0m");
        assert_eq!(format_duration(300), "5m");
        assert_eq!(format_duration(3600), "1h 00m");
        assert_eq!(format_duration(5400), "1h 30m");
        assert_eq!(format_duration(9060), "2h 31m");
    }

    #[test]
    fn calculate_earnings_tests() {
        assert_eq!(calculate_earnings(3600, 15000), 15000); // 1h
        assert_eq!(calculate_earnings(1800, 15000), 7500);  // 30m
        assert_eq!(calculate_earnings(0, 15000), 0);
    }

    #[test]
    fn format_clp_tests() {
        assert_eq!(format_clp(0), "$0");
        assert_eq!(format_clp(500), "$500");
        assert_eq!(format_clp(1500), "$1.500");
        assert_eq!(format_clp(15000), "$15.000");
        assert_eq!(format_clp(1500000), "$1.500.000");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test db::session_repo`
Expected: 8 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/db/session_repo.rs
git commit -m "feat: add SessionRepo with time calculations and formatting"
```

---

### Task 6: Waybar JSON output

**Files:**
- Create: `src/waybar/mod.rs`
- Create: `src/waybar/output.rs`

- [ ] **Step 1: Create src/waybar/mod.rs**

```rust
pub mod output;
```

- [ ] **Step 2: Create src/waybar/output.rs with tests**

```rust
use serde::Serialize;

use crate::db::connection::Database;
use crate::db::session_repo::{SessionRepo, calculate_earnings, format_clp, format_duration};
use crate::db::tracker_repo::TrackerRepo;
use crate::domain::tracker::TrackerState;
use crate::error::Result;

#[derive(Debug, Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
    pub class: String,
}

pub fn generate(db: &Database) -> Result<WaybarOutput> {
    let tracker_repo = TrackerRepo::new(db);
    let session_repo = SessionRepo::new(db);

    let active = tracker_repo.find_active()?;
    let trackers = tracker_repo.get_all()?;

    let text = match &active {
        Some(tracker) => {
            let seconds = session_repo.today_seconds(tracker.id.unwrap())?;
            let duration = format_duration(seconds);
            let icon = tracker
                .icon_path
                .as_deref()
                .unwrap_or("");
            if icon.is_empty() {
                format!(" {} {} ", tracker.name, duration)
            } else {
                format!(" {} {} {} ", icon, tracker.name, duration)
            }
        }
        None => String::new(),
    };

    let mut tooltip_lines: Vec<String> = Vec::new();
    let mut total_seconds: i64 = 0;
    let mut total_earnings: i64 = 0;

    for tracker in &trackers {
        let tracker_id = tracker.id.unwrap();
        let seconds = session_repo.today_seconds(tracker_id)?;
        let earnings = calculate_earnings(seconds, tracker.hourly_rate);
        total_seconds += seconds;
        total_earnings += earnings;

        let symbol = match tracker.state {
            TrackerState::Active => "●",
            TrackerState::Paused => "◉",
            TrackerState::Created => "○",
        };

        if seconds > 0 || tracker.state == TrackerState::Active {
            tooltip_lines.push(format!(
                "{} {}: {} ({})",
                symbol,
                tracker.name,
                format_duration(seconds),
                format_clp(earnings),
            ));
        }
    }

    if !tooltip_lines.is_empty() {
        tooltip_lines.push("──────────".to_string());
        tooltip_lines.push(format!(
            "Total: {} | {}",
            format_duration(total_seconds),
            format_clp(total_earnings),
        ));
    }

    let class = if active.is_some() {
        "active".to_string()
    } else {
        "idle".to_string()
    };

    Ok(WaybarOutput {
        text,
        tooltip: tooltip_lines.join("\n"),
        class,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tracker::Tracker;
    use chrono::Local;

    fn setup_db_with_tracker(name: &str, state: TrackerState) -> (Database, i64) {
        let db = Database::in_memory().unwrap();
        let tracker_repo = TrackerRepo::new(&db);
        let id = tracker_repo
            .create(&Tracker {
                id: None,
                name: name.to_string(),
                color: "#55a555".to_string(),
                icon_path: None,
                hourly_rate: 15000,
                state: TrackerState::Created,
                created_at: "2026-04-01T10:00:00".to_string(),
            })
            .unwrap();
        if state != TrackerState::Created {
            tracker_repo.update_state(id, state).unwrap();
        }
        (db, id)
    }

    #[test]
    fn idle_when_no_trackers() {
        let db = Database::in_memory().unwrap();
        let output = generate(&db).unwrap();
        assert_eq!(output.text, "");
        assert_eq!(output.class, "idle");
        assert_eq!(output.tooltip, "");
    }

    #[test]
    fn idle_when_no_active_tracker() {
        let (db, _) = setup_db_with_tracker("Work", TrackerState::Paused);
        let output = generate(&db).unwrap();
        assert_eq!(output.text, "");
        assert_eq!(output.class, "idle");
    }

    #[test]
    fn active_tracker_shows_in_text() {
        let (db, id) = setup_db_with_tracker("Work", TrackerState::Active);

        // Insert a closed 1-hour session today
        let today = Local::now().format("%Y-%m-%d").to_string();
        db.conn()
            .execute(
                "INSERT INTO sessions (tracker_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, format!("{today}T10:00:00"), format!("{today}T11:00:00")],
            )
            .unwrap();

        let output = generate(&db).unwrap();
        assert_eq!(output.class, "active");
        assert!(output.text.contains("Work"));
        assert!(output.text.contains("1h 00m"));
        assert!(output.tooltip.contains("●"));
        assert!(output.tooltip.contains("$15.000"));
    }

    #[test]
    fn output_serializes_to_valid_json() {
        let db = Database::in_memory().unwrap();
        let output = generate(&db).unwrap();
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"text\""));
        assert!(json.contains("\"tooltip\""));
        assert!(json.contains("\"class\""));
    }
}
```

- [ ] **Step 3: Wire waybar into main.rs**

Add `mod waybar;` to `src/main.rs`.

- [ ] **Step 4: Run tests**

Run: `cargo test waybar::output`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/waybar/ src/main.rs
git commit -m "feat: add waybar JSON output generation"
```

---

### Task 7: CLI definition and command handlers

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/commands.rs`

- [ ] **Step 1: Create src/cli/mod.rs with clap subcommand definitions**

```rust
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "tag-tracker",
    version,
    about = "CLI time tracker with waybar integration",
    long_about = "Track time across multiple activities, display in waybar, and calculate daily earnings in CLP."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage trackers (add, list, edit, delete)
    Tracker {
        #[command(subcommand)]
        action: TrackerAction,
    },
    /// Activate a tracker (pauses any currently active one)
    Activate {
        /// Name of the tracker to activate
        name: String,
    },
    /// Pause the currently active tracker
    Pause,
    /// Show today's status: time per tracker, earnings, totals
    Status,
    /// Output JSON for waybar module
    Waybar,
}

#[derive(Subcommand, Debug)]
pub enum TrackerAction {
    /// Add a new tracker
    Add {
        /// Tracker name
        name: String,
        /// Hex color (e.g. "#55a555")
        #[arg(long)]
        color: String,
        /// Hourly rate in CLP (e.g. 15000)
        #[arg(long)]
        rate: i64,
        /// Path to icon file (optional)
        #[arg(long)]
        icon: Option<PathBuf>,
    },
    /// List all trackers
    List,
    /// Edit a tracker's properties
    Edit {
        /// Current name of the tracker to edit
        name: String,
        /// New name
        #[arg(long)]
        new_name: Option<String>,
        /// New hex color
        #[arg(long)]
        color: Option<String>,
        /// New hourly rate in CLP
        #[arg(long)]
        rate: Option<i64>,
        /// New icon path
        #[arg(long)]
        icon: Option<PathBuf>,
    },
    /// Delete a tracker and its sessions
    Delete {
        /// Name of the tracker to delete
        name: String,
    },
}

pub mod commands;
```

- [ ] **Step 2: Create src/cli/commands.rs with all command handlers**

```rust
use std::process::Command as ProcessCommand;

use colored::Colorize;

use crate::db::connection::Database;
use crate::db::session_repo::{SessionRepo, calculate_earnings, format_clp, format_duration};
use crate::db::tracker_repo::TrackerRepo;
use crate::domain::tracker::{Tracker, TrackerState};
use crate::error::{AppError, Result};
use crate::waybar::output;

pub fn tracker_add(
    db: &Database,
    name: String,
    color: String,
    rate: i64,
    icon: Option<String>,
) -> Result<()> {
    let repo = TrackerRepo::new(db);

    if repo.find_by_name(&name)?.is_some() {
        return Err(AppError::Validation(format!(
            "Tracker '{name}' already exists."
        )));
    }

    let now = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let tracker = Tracker {
        id: None,
        name: name.clone(),
        color,
        icon_path: icon,
        hourly_rate: rate,
        state: TrackerState::Created,
        created_at: now,
    };

    repo.create(&tracker)?;
    println!("Tracker '{}' created.", name.green());
    Ok(())
}

pub fn tracker_list(db: &Database) -> Result<()> {
    let repo = TrackerRepo::new(db);
    let trackers = repo.get_all()?;

    if trackers.is_empty() {
        println!("No trackers found. Use 'tag-tracker tracker add' to create one.");
        return Ok(());
    }

    println!(
        " {:<15} {:<12} {:<10} {:<10}",
        "Name", "Rate/hr", "Color", "State"
    );
    println!("{}", "─".repeat(50));

    for t in &trackers {
        let state_display = match t.state {
            TrackerState::Active => t.state.to_string().green().to_string(),
            TrackerState::Paused => t.state.to_string().yellow().to_string(),
            TrackerState::Created => t.state.to_string().dimmed().to_string(),
        };
        println!(
            " {:<15} {:<12} {:<10} {}",
            t.name,
            format_clp(t.hourly_rate),
            t.color,
            state_display,
        );
    }
    Ok(())
}

pub fn tracker_edit(
    db: &Database,
    name: String,
    new_name: Option<String>,
    color: Option<String>,
    rate: Option<i64>,
    icon: Option<String>,
) -> Result<()> {
    let repo = TrackerRepo::new(db);
    let mut tracker = repo
        .find_by_name(&name)?
        .ok_or_else(|| AppError::NotFound(format!("Tracker '{name}'")))?;

    if let Some(n) = new_name {
        tracker.name = n;
    }
    if let Some(c) = color {
        tracker.color = c;
    }
    if let Some(r) = rate {
        tracker.hourly_rate = r;
    }
    if let Some(i) = icon {
        tracker.icon_path = Some(i);
    }

    repo.update(&tracker)?;
    println!("Tracker '{}' updated.", tracker.name.green());
    Ok(())
}

pub fn tracker_delete(db: &Database, name: String) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let tracker = tracker_repo
        .find_by_name(&name)?
        .ok_or_else(|| AppError::NotFound(format!("Tracker '{name}'")))?;

    let id = tracker.id.unwrap();
    tracker_repo.delete(id)?;
    println!("Tracker '{}' deleted.", name.red());
    Ok(())
}

pub fn activate(db: &Database, name: String) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let session_repo = SessionRepo::new(db);

    let target = tracker_repo
        .find_by_name(&name)?
        .ok_or_else(|| AppError::NotFound(format!("Tracker '{name}'")))?;

    let target_id = target.id.unwrap();

    // Pause current active tracker if any
    if let Some(active) = tracker_repo.find_active()? {
        let active_id = active.id.unwrap();
        if active_id == target_id {
            println!("Tracker '{}' is already active.", name.yellow());
            return Ok(());
        }
        session_repo.stop_active(active_id)?;
        tracker_repo.update_state(active_id, TrackerState::Paused)?;
    }

    // Activate target
    tracker_repo.update_state(target_id, TrackerState::Active)?;
    session_repo.start(target_id)?;

    signal_waybar();
    println!("Tracker '{}' activated.", name.green());
    Ok(())
}

pub fn pause(db: &Database) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let session_repo = SessionRepo::new(db);

    let active = tracker_repo
        .find_active()?
        .ok_or_else(|| AppError::Validation("No active tracker to pause.".into()))?;

    let id = active.id.unwrap();
    session_repo.stop_active(id)?;
    tracker_repo.update_state(id, TrackerState::Paused)?;

    signal_waybar();
    println!("Tracker '{}' paused.", active.name.yellow());
    Ok(())
}

pub fn status(db: &Database) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let session_repo = SessionRepo::new(db);
    let trackers = tracker_repo.get_all()?;

    if trackers.is_empty() {
        println!("No trackers found.");
        return Ok(());
    }

    let mut total_seconds: i64 = 0;
    let mut total_earnings: i64 = 0;

    for tracker in &trackers {
        let tracker_id = tracker.id.unwrap();
        let seconds = session_repo.today_seconds(tracker_id)?;
        let earnings = calculate_earnings(seconds, tracker.hourly_rate);
        total_seconds += seconds;
        total_earnings += earnings;

        let (symbol, name_display) = match tracker.state {
            TrackerState::Active => (
                "●".green().to_string(),
                format!("Active: {}", tracker.name.green()),
            ),
            TrackerState::Paused => (
                "◉".yellow().to_string(),
                format!("Paused: {}", tracker.name.yellow()),
            ),
            TrackerState::Created => (
                "○".dimmed().to_string(),
                format!("Created: {}", tracker.name.dimmed()),
            ),
        };

        println!("{} {}", symbol, name_display);
        if seconds > 0 {
            println!("  Time today: {}", format_duration(seconds));
            println!("  Earned:     {}", format_clp(earnings));
        } else {
            println!("  Not started today");
        }
        println!();
    }

    println!("{}", "─".repeat(35));
    println!(
        "Total today: {} | {}",
        format_duration(total_seconds),
        format_clp(total_earnings),
    );
    Ok(())
}

pub fn waybar(db: &Database) -> Result<()> {
    let result = output::generate(db)?;
    println!("{}", serde_json::to_string(&result).unwrap());
    Ok(())
}

fn signal_waybar() {
    let _ = ProcessCommand::new("pkill")
        .args(["-RTMIN+10", "waybar"])
        .status();
}
```

- [ ] **Step 3: Wire cli into main.rs and update main function**

Replace `src/main.rs` with:

```rust
mod cli;
mod db;
mod domain;
mod error;
mod waybar;

use std::path::PathBuf;

use clap::Parser;
use directories::ProjectDirs;

use crate::cli::{Cli, Command, TrackerAction};
use crate::db::connection::Database;
use crate::db::session_repo::SessionRepo;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> error::Result<()> {
    let cli = Cli::parse();

    let db_path = get_db_path()?;
    let db = Database::new(&db_path)?;

    // Close stale sessions from previous days
    let session_repo = SessionRepo::new(&db);
    let closed = session_repo.close_stale_sessions()?;
    if closed > 0 {
        eprintln!("Closed {closed} stale session(s) from previous days.");
    }

    match cli.command {
        Command::Tracker { action } => match action {
            TrackerAction::Add {
                name,
                color,
                rate,
                icon,
            } => cli::commands::tracker_add(
                &db,
                name,
                color,
                rate,
                icon.map(|p| p.to_string_lossy().to_string()),
            ),
            TrackerAction::List => cli::commands::tracker_list(&db),
            TrackerAction::Edit {
                name,
                new_name,
                color,
                rate,
                icon,
            } => cli::commands::tracker_edit(
                &db,
                name,
                new_name,
                color,
                rate,
                icon.map(|p| p.to_string_lossy().to_string()),
            ),
            TrackerAction::Delete { name } => cli::commands::tracker_delete(&db, name),
        },
        Command::Activate { name } => cli::commands::activate(&db, name),
        Command::Pause => cli::commands::pause(&db),
        Command::Status => cli::commands::status(&db),
        Command::Waybar => cli::commands::waybar(&db),
    }
}

fn get_db_path() -> error::Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "tag-tracker")
        .ok_or_else(|| error::AppError::Validation("Could not determine data directory.".into()))?;
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;
    Ok(data_dir.join("tag-tracker.db"))
}
```

- [ ] **Step 4: Verify it compiles and all tests pass**

Run: `cargo build && cargo test`
Expected: Compiles. All tests pass (2 domain + 3 connection + 7 tracker_repo + 8 session_repo + 4 waybar = 24 tests).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add src/cli/ src/main.rs
git commit -m "feat: add CLI commands and main entry point"
```

---

### Task 8: Waybar config and Hyprland integration

**Files:**
- Modify: `~/.config/waybar/config.jsonc`
- Modify: `~/.config/waybar/style.css`
- Modify: `~/.config/hypr/hyprland.conf`

- [ ] **Step 1: Build release binary and install**

```bash
cd /home/franco/proyectos-personales/tag-tracker
cargo build --release
```

Copy or symlink the binary so it's in PATH:

```bash
cp target/release/tag-tracker ~/.local/bin/tag-tracker
```

- [ ] **Step 2: Add waybar module to config.jsonc**

Add `"custom/tag-tracker"` to the `modules-left` array (after `"hyprland/workspaces"`).

Add the module definition:

```jsonc
"custom/tag-tracker": {
    "exec": "tag-tracker waybar",
    "return-type": "json",
    "interval": 5,
    "signal": 10,
    "tooltip": true,
    "on-click": "tag-tracker pause"
}
```

- [ ] **Step 3: Add waybar styles to style.css**

Append to `~/.config/waybar/style.css`:

```css
/* tag-tracker module */
#custom-tag-tracker {
    margin: 0 4px;
    padding: 0 8px;
    border-radius: 4px;
    transition: all 0.3s ease;
}

#custom-tag-tracker.active {
    font-weight: bold;
}
```

- [ ] **Step 4: Add exec-shutdown to Hyprland config**

Append to `~/.config/hypr/hyprland.conf`:

```
exec-shutdown = tag-tracker pause
```

- [ ] **Step 5: Restart waybar**

```bash
omarchy-restart-waybar
```

- [ ] **Step 6: Commit config changes**

```bash
cd /home/franco/proyectos-personales/tag-tracker
git add -A
git commit -m "docs: add waybar and hyprland integration instructions"
```

---

### Task 9: End-to-end manual verification

- [ ] **Step 1: Create test trackers**

```bash
tag-tracker tracker add "Empresa A" --color "#55a555" --rate 15000
tag-tracker tracker add "Empresa B" --color "#5555a5" --rate 20000
tag-tracker tracker list
```

Expected: Both trackers listed with state "created".

- [ ] **Step 2: Activate and verify**

```bash
tag-tracker activate "Empresa A"
tag-tracker status
```

Expected: Empresa A shows as active with time counting.

- [ ] **Step 3: Switch trackers**

```bash
tag-tracker activate "Empresa B"
tag-tracker status
```

Expected: Empresa A paused (with accumulated time), Empresa B active.

- [ ] **Step 4: Pause all**

```bash
tag-tracker pause
tag-tracker status
```

Expected: Both paused with their times.

- [ ] **Step 5: Verify waybar output**

```bash
tag-tracker waybar
```

Expected: Valid JSON with empty text (since all paused).

```bash
tag-tracker activate "Empresa A"
tag-tracker waybar
```

Expected: JSON with text containing "Empresa A" and class "active".

- [ ] **Step 6: Verify waybar module appears in bar**

Check that the tag-tracker module is visible in waybar. Hover to see tooltip with daily summary.

- [ ] **Step 7: Run full test suite one final time**

```bash
cargo test && cargo clippy
```

Expected: All 24+ tests pass, zero clippy warnings.
