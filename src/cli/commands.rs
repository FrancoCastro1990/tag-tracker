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
    validate_color(&color)?;
    validate_rate(rate)?;

    let repo = TrackerRepo::new(db);

    if repo.find_by_name(&name)?.is_some() {
        return Err(AppError::Validation(format!(
            "Tracker '{name}' already exists."
        )));
    }

    let shortcut = repo.next_available_shortcut()?;

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
        shortcut,
    };

    repo.create(&tracker)?;

    if let Some(s) = shortcut {
        println!(
            "Tracker '{}' created. Shortcut: SUPER ALT CTRL + {}",
            name.green(),
            s
        );
    } else {
        println!(
            "Tracker '{}' created. No shortcut available (max 9).",
            name.green()
        );
    }

    crate::keybindings::sync(db)?;
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
        " {:<5} {:<15} {:<12} {:<10} {:<10}",
        "Key", "Name", "Rate/hr", "Color", "State"
    );
    println!("{}", "─".repeat(55));

    for t in &trackers {
        let state_display = match t.state {
            TrackerState::Active => t.state.to_string().green().to_string(),
            TrackerState::Paused => t.state.to_string().yellow().to_string(),
            TrackerState::Created => t.state.to_string().dimmed().to_string(),
        };
        let key_display = match t.shortcut {
            Some(s) => format!("[{}]", s),
            None => " - ".to_string(),
        };
        println!(
            " {:<5} {:<15} {:<12} {:<10} {}",
            key_display,
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
    shortcut: Option<i64>,
) -> Result<()> {
    let repo = TrackerRepo::new(db);
    let mut tracker = repo
        .find_by_name(&name)?
        .ok_or_else(|| AppError::NotFound(format!("Tracker '{name}'")))?;

    let name_changed = new_name.is_some();
    let shortcut_changed = shortcut.is_some();

    if let Some(n) = new_name {
        tracker.name = n;
    }
    if let Some(ref c) = color {
        validate_color(c)?;
    }
    if let Some(r) = rate {
        validate_rate(r)?;
    }
    if let Some(s) = shortcut {
        validate_shortcut(s)?;
        // Check if shortcut is already taken by another tracker
        let all = repo.get_all()?;
        if let Some(other) = all.iter().find(|t| t.shortcut == Some(s) && t.id != tracker.id) {
            return Err(AppError::Validation(format!(
                "Shortcut {} is already assigned to '{}'.",
                s, other.name
            )));
        }
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
    if let Some(s) = shortcut {
        tracker.shortcut = Some(s);
    }

    repo.update(&tracker)?;
    println!("Tracker '{}' updated.", tracker.name.green());

    if (name_changed || shortcut_changed) && tracker.shortcut.is_some() {
        crate::keybindings::sync(db)?;
    }

    Ok(())
}

pub fn tracker_delete(db: &Database, name: String) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let tracker = tracker_repo
        .find_by_name(&name)?
        .ok_or_else(|| AppError::NotFound(format!("Tracker '{name}'")))?;

    let was_active = tracker.state == TrackerState::Active;
    let id = tracker.id.unwrap();
    tracker_repo.delete(id)?;

    if was_active {
        signal_waybar();
    }

    crate::keybindings::sync(db)?;
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

pub fn status(db: &Database, date: Option<String>, name: Option<String>) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let session_repo = SessionRepo::new(db);

    let is_today = date.is_none();
    let date_str = match &date {
        Some(d) => validate_date(d)?,
        None => chrono::Local::now().format("%Y-%m-%d").to_string(),
    };

    let trackers = match &name {
        Some(n) => {
            let tracker = tracker_repo
                .find_by_name(n)?
                .ok_or_else(|| AppError::NotFound(format!("Tracker '{n}'")))?;
            vec![tracker]
        }
        None => tracker_repo.get_all()?,
    };

    if trackers.is_empty() {
        println!("No trackers found.");
        return Ok(());
    }

    if !is_today {
        println!("Report for {}", date.as_ref().unwrap().bold());
        println!();
    }

    let (time_label, no_time_label, total_label) = if is_today {
        ("Time today:", "Not started today", "Total today:")
    } else {
        ("Time:", "No sessions", "Total:")
    };

    let mut total_seconds: i64 = 0;
    let mut total_earnings: i64 = 0;

    for tracker in &trackers {
        let tracker_id = tracker.id.unwrap();
        let seconds = session_repo.today_seconds_for_date(tracker_id, &date_str)?;
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
            println!("  {time_label} {}", format_duration(seconds));
            println!("  Earned:     {}", format_clp(earnings));
        } else {
            println!("  {no_time_label}");
        }
        println!();
    }

    println!("{}", "─".repeat(35));
    println!(
        "{total_label} {} | {}",
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

pub fn generate_eww_json(db: &Database) -> Result<String> {
    let tracker_repo = TrackerRepo::new(db);
    let session_repo = SessionRepo::new(db);
    let trackers = tracker_repo.get_all()?;

    let mut tracker_entries = Vec::new();
    let mut total_seconds: i64 = 0;
    let mut total_earnings_val: i64 = 0;

    for tracker in &trackers {
        let tracker_id = tracker.id.unwrap();
        let seconds = session_repo.today_seconds(tracker_id)?;
        let earnings = calculate_earnings(seconds, tracker.hourly_rate);
        total_seconds += seconds;
        total_earnings_val += earnings;

        if seconds > 0 || tracker.state == TrackerState::Active {
            tracker_entries.push(serde_json::json!({
                "name": tracker.name,
                "color": tracker.color,
                "state": tracker.state.to_string().to_lowercase(),
                "duration": format_duration(seconds),
                "earnings": format_clp(earnings),
            }));
        }
    }

    let output = serde_json::json!({
        "trackers": tracker_entries,
        "total_duration": format_duration(total_seconds),
        "total_earnings": format_clp(total_earnings_val),
    });

    Ok(serde_json::to_string(&output).unwrap())
}

pub fn eww(db: &Database) -> Result<()> {
    println!("{}", generate_eww_json(db)?);
    Ok(())
}

pub fn menu(db: &Database) -> Result<()> {
    let tracker_repo = TrackerRepo::new(db);
    let trackers = tracker_repo.get_all()?;
    let active = tracker_repo.find_active()?;

    let mut options: Vec<String> = Vec::new();

    for t in &trackers {
        let symbol = match t.state {
            TrackerState::Active => "⏸",
            TrackerState::Paused => "󰔛",
            TrackerState::Created => "󰔛",
        };
        options.push(format!("{}  {}", symbol, t.name));
    }

    if options.is_empty() {
        return Ok(());
    }

    let input = options.join("\n");
    let walker = ProcessCommand::new("walker")
        .args(["--dmenu", "--placeholder", "Select tracker..."])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut child = match walker {
        Ok(c) => c,
        Err(_) => return Ok(()), // Walker not available, silently exit
    };

    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        let _ = stdin.write_all(input.as_bytes());
    }

    let output = child.wait_with_output()?;
    let selection = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if selection.is_empty() {
        return Ok(()); // User cancelled
    }

    // Extract tracker name (after the icon + spaces)
    let tracker_name = selection
        .split("  ")
        .last()
        .unwrap_or(&selection)
        .to_string();

    // If selected tracker is the active one, pause it; otherwise activate it
    if let Some(ref a) = active {
        if a.name == tracker_name {
            return pause(db);
        }
    }

    activate(db, tracker_name)
}

pub fn sync_keybindings(db: &Database) -> Result<()> {
    crate::keybindings::sync(db)?;
    println!("Keybindings synced with Hyprland.");
    Ok(())
}

fn validate_date(input: &str) -> Result<String> {
    let parsed = chrono::NaiveDate::parse_from_str(input, "%d/%m/%Y").map_err(|_| {
        AppError::Validation(format!("Invalid date '{input}'. Use DD/MM/YYYY format."))
    })?;
    Ok(parsed.format("%Y-%m-%d").to_string())
}

fn validate_color(color: &str) -> Result<()> {
    if color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Invalid color '{color}'. Use hex format: #RRGGBB"
        )))
    }
}

fn validate_shortcut(shortcut: i64) -> Result<()> {
    if (1..=9).contains(&shortcut) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Shortcut must be between 1 and 9, got {shortcut}."
        )))
    }
}

fn validate_rate(rate: i64) -> Result<()> {
    if rate >= 0 {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Hourly rate cannot be negative.".into(),
        ))
    }
}

fn signal_waybar() {
    let _ = ProcessCommand::new("pkill")
        .args(["-RTMIN+11", "waybar"])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eww_output_with_active_tracker() {
        let db = crate::db::connection::Database::in_memory().unwrap();
        let tracker_repo = crate::db::tracker_repo::TrackerRepo::new(&db);

        let id = tracker_repo
            .create(&crate::domain::tracker::Tracker {
                id: None,
                name: "Work".to_string(),
                color: "#55a555".to_string(),
                icon_path: None,
                hourly_rate: 15000,
                state: crate::domain::tracker::TrackerState::Created,
                created_at: "2026-04-01T10:00:00".to_string(),
                shortcut: None,
            })
            .unwrap();
        tracker_repo
            .update_state(id, crate::domain::tracker::TrackerState::Active)
            .unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        db.conn()
            .execute(
                "INSERT INTO sessions (tracker_id, started_at, ended_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, format!("{today}T10:00:00"), format!("{today}T11:00:00")],
            )
            .unwrap();

        let json = generate_eww_json(&db).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["trackers"][0]["name"], "Work");
        assert_eq!(parsed["trackers"][0]["color"], "#55a555");
        assert_eq!(parsed["trackers"][0]["state"], "active");
        assert_eq!(parsed["trackers"][0]["duration"], "1h 00m");
        assert_eq!(parsed["trackers"][0]["earnings"], "$15.000");
        assert_eq!(parsed["total_duration"], "1h 00m");
        assert_eq!(parsed["total_earnings"], "$15.000");
    }

    #[test]
    fn eww_output_empty_when_no_sessions() {
        let db = crate::db::connection::Database::in_memory().unwrap();
        let json = generate_eww_json(&db).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["trackers"].as_array().unwrap().is_empty());
        assert_eq!(parsed["total_duration"], "0m");
        assert_eq!(parsed["total_earnings"], "$0");
    }

    #[test]
    fn validate_date_valid_inputs() {
        assert_eq!(validate_date("02/04/2026").unwrap(), "2026-04-02");
        assert_eq!(validate_date("31/12/2025").unwrap(), "2025-12-31");
        assert_eq!(validate_date("01/01/2020").unwrap(), "2020-01-01");
    }

    #[test]
    fn validate_date_invalid_format() {
        assert!(validate_date("2026-04-02").is_err());
        assert!(validate_date("not-a-date").is_err());
        assert!(validate_date("").is_err());
    }

    #[test]
    fn validate_date_impossible_dates() {
        assert!(validate_date("31/02/2026").is_err());
        assert!(validate_date("29/02/2025").is_err());
    }
}
