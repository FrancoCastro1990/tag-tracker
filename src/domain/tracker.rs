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
