use crate::db::connection::Database;
use crate::domain::tracker::{Tracker, TrackerState, TrackerType};
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
            "INSERT INTO trackers (name, color, icon_path, hourly_rate, state, created_at, shortcut, tracker_type, salary, weekly_hours) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                tracker.name,
                tracker.color,
                tracker.icon_path,
                tracker.hourly_rate,
                tracker.state.to_string(),
                tracker.created_at,
                tracker.shortcut,
                tracker.tracker_type.to_string(),
                tracker.salary,
                tracker.weekly_hours,
            ],
        )?;
        Ok(self.db.conn().last_insert_rowid())
    }

    #[allow(dead_code)]
    pub fn get_by_id(&self, id: i64) -> Result<Tracker> {
        self.db
            .conn()
            .query_row(
                "SELECT id, name, color, icon_path, hourly_rate, state, created_at, shortcut, tracker_type, salary, weekly_hours FROM trackers WHERE id = ?1",
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
            "SELECT id, name, color, icon_path, hourly_rate, state, created_at, shortcut, tracker_type, salary, weekly_hours FROM trackers ORDER BY name",
        )?;
        let trackers = stmt
            .query_map([], row_to_tracker)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(trackers)
    }

    pub fn find_by_name(&self, name: &str) -> Result<Option<Tracker>> {
        let result = self.db.conn().query_row(
            "SELECT id, name, color, icon_path, hourly_rate, state, created_at, shortcut, tracker_type, salary, weekly_hours FROM trackers WHERE name = ?1",
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
            "SELECT id, name, color, icon_path, hourly_rate, state, created_at, shortcut, tracker_type, salary, weekly_hours FROM trackers WHERE state = 'active'",
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
            "UPDATE trackers SET name = ?1, color = ?2, icon_path = ?3, hourly_rate = ?4, state = ?5, shortcut = ?6, tracker_type = ?7, salary = ?8, weekly_hours = ?9 WHERE id = ?10",
            rusqlite::params![
                tracker.name,
                tracker.color,
                tracker.icon_path,
                tracker.hourly_rate,
                tracker.state.to_string(),
                tracker.shortcut,
                tracker.tracker_type.to_string(),
                tracker.salary,
                tracker.weekly_hours,
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

    pub fn next_available_shortcut(&self) -> Result<Option<i64>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT shortcut FROM trackers WHERE shortcut IS NOT NULL ORDER BY shortcut",
        )?;
        let used: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        for n in 1..=9 {
            if !used.contains(&n) {
                return Ok(Some(n));
            }
        }
        Ok(None)
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
    let type_str: String = row.get(8)?;
    let tracker_type = type_str
        .parse::<TrackerType>()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?;
    Ok(Tracker {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        icon_path: row.get(3)?,
        hourly_rate: row.get(4)?,
        state,
        created_at: row.get(6)?,
        shortcut: row.get(7)?,
        tracker_type,
        salary: row.get(9)?,
        weekly_hours: row.get(10)?,
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
            shortcut: None,
            tracker_type: TrackerType::Freelance,
            salary: None,
            weekly_hours: None,
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
