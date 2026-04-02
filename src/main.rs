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
