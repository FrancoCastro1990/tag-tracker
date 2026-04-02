use chrono::Local;

use crate::db::connection::Database;
use crate::error::Result;

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
