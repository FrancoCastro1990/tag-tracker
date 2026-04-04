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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackerType {
    Freelance,
    Contract,
}

impl fmt::Display for TrackerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackerType::Freelance => write!(f, "freelance"),
            TrackerType::Contract => write!(f, "contract"),
        }
    }
}

impl FromStr for TrackerType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "freelance" => Ok(TrackerType::Freelance),
            "contract" => Ok(TrackerType::Contract),
            other => Err(format!("Invalid tracker type: {other}")),
        }
    }
}

pub fn calculate_contract_rate(salary: i64, weekly_hours: i64) -> i64 {
    (salary as f64 / (weekly_hours as f64 * 4.33)).round() as i64
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
    pub shortcut: Option<i64>,
    pub tracker_type: TrackerType,
    pub salary: Option<i64>,
    pub weekly_hours: Option<i64>,
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

    #[test]
    fn tracker_type_display() {
        assert_eq!(TrackerType::Freelance.to_string(), "freelance");
        assert_eq!(TrackerType::Contract.to_string(), "contract");
    }

    #[test]
    fn tracker_type_from_str() {
        assert_eq!("freelance".parse::<TrackerType>().unwrap(), TrackerType::Freelance);
        assert_eq!("contract".parse::<TrackerType>().unwrap(), TrackerType::Contract);
        assert!("invalid".parse::<TrackerType>().is_err());
    }

    #[test]
    fn calculate_contract_rate_basic() {
        // 1_500_000 / (45 * 4.33) = 1_500_000 / 194.85 ≈ 7698
        let rate = calculate_contract_rate(1_500_000, 45);
        assert_eq!(rate, 7698);
    }

    #[test]
    fn calculate_contract_rate_round_trip() {
        let rate = calculate_contract_rate(1_000_000, 40);
        // 1_000_000 / (40 * 4.33) = 1_000_000 / 173.2 ≈ 5774
        assert_eq!(rate, 5774);
    }
}
