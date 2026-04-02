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
    if let Some(ref c) = color {
        validate_color(c)?;
    }
    if let Some(r) = rate {
        validate_rate(r)?;
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

    let was_active = tracker.state == TrackerState::Active;
    let id = tracker.id.unwrap();
    tracker_repo.delete(id)?;

    if was_active {
        signal_waybar();
    }

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
