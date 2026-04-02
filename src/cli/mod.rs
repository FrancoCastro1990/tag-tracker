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
        /// Hourly rate in CLP (e.g. 15000, default: 0)
        #[arg(long, default_value_t = 0)]
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
