use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: u64,
    pub status: String,
    r#ref: String,
    sha: String,
    web_url: String,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum StatusState {
    /// The check was created.
    #[serde(rename = "created")]
    Created,
    /// The check is waiting for some other resource.
    #[serde(rename = "waiting_for_resource")]
    WaitingForResource,
    /// The check is currently being prepared.
    #[serde(rename = "preparing")]
    Preparing,
    /// The check is queued.
    #[serde(rename = "pending")]
    Pending,
    /// The check is currently running.
    #[serde(rename = "running")]
    Running,
    /// The check succeeded.
    #[serde(rename = "success")]
    Success,
    /// The check failed.
    #[serde(rename = "failed")]
    Failed,
    /// The check was canceled.
    #[serde(rename = "canceled")]
    Canceled,
    /// The check was skipped.
    #[serde(rename = "skipped")]
    Skipped,
    /// The check is waiting for manual action.
    #[serde(rename = "manual")]
    Manual,
    /// The check is scheduled to run at some point in time.
    #[serde(rename = "scheduled")]
    Scheduled,
}
