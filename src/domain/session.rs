use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub tracker_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
}
