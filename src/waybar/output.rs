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
                .unwrap_or("󰔛");
            format!(
                "<span foreground='{}' font_weight='bold'>{} {} · {}</span>",
                tracker.color, icon, tracker.name, duration
            )
        }
        None => "󰔛".to_string(),
    };

    // Collect tracker data for tooltip
    struct TrackerRow {
        symbol: &'static str,
        name: String,
        color: String,
        duration: String,
        earnings: String,
    }

    let mut rows: Vec<TrackerRow> = Vec::new();
    let mut total_seconds: i64 = 0;
    let mut total_earnings: i64 = 0;

    for tracker in &trackers {
        let tracker_id = tracker.id.unwrap();
        let seconds = session_repo.today_seconds(tracker_id)?;
        let earnings = calculate_earnings(seconds, tracker.hourly_rate);
        total_seconds += seconds;
        total_earnings += earnings;

        let symbol = match tracker.state {
            TrackerState::Active => "",
            TrackerState::Paused => "",
            TrackerState::Created => "○",
        };

        if seconds > 0 || tracker.state == TrackerState::Active {
            rows.push(TrackerRow {
                symbol,
                name: tracker.name.clone(),
                color: tracker.color.clone(),
                duration: format_duration(seconds),
                earnings: format_clp(earnings),
            });
        }
    }

    let mut tooltip_lines: Vec<String> = Vec::new();

    if !rows.is_empty() {
        // Calculate column widths for alignment
        let max_name = rows.iter().map(|r| r.name.len()).max().unwrap_or(0);
        let max_dur = rows.iter().map(|r| r.duration.len()).max().unwrap_or(0);
        let max_earn = rows.iter().map(|r| r.earnings.len()).max().unwrap_or(0);

        for row in &rows {
            tooltip_lines.push(format!(
                "{}  <span foreground='{}'>{:<nw$}</span>   <b>{:>dw$}</b>   <span alpha='60%'>{:>ew$}</span>",
                row.symbol,
                row.color,
                row.name,
                row.duration,
                row.earnings,
                nw = max_name,
                dw = max_dur,
                ew = max_earn,
            ));
        }

        let total_dur = format_duration(total_seconds);
        let total_earn = format_clp(total_earnings);

        tooltip_lines.push(format!(
            "<span alpha='30%'>{}</span>",
            "─".repeat(max_name + max_dur + max_earn + 12)
        ));
        tooltip_lines.push(format!(
            "󰔛  {:<nw$}   <b>{:>dw$}</b>   {:>ew$}",
            "Total",
            total_dur,
            total_earn,
            nw = max_name,
            dw = max_dur,
            ew = max_earn,
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
                shortcut: None,
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
        assert_eq!(output.text, "󰔛");
        assert_eq!(output.class, "idle");
        assert_eq!(output.tooltip, "");
    }

    #[test]
    fn idle_when_no_active_tracker() {
        let (db, _) = setup_db_with_tracker("Work", TrackerState::Paused);
        let output = generate(&db).unwrap();
        assert_eq!(output.text, "󰔛");
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
        assert!(output.tooltip.contains(""));
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
