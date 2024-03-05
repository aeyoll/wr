use serde::Deserialize;

use crate::pipeline::StatusState;

#[derive(Debug, Deserialize)]
pub struct Job {
    /// The ID of the job.
    pub id: u64,
    /// The status of the job.
    pub status: StatusState,
    /// The name of the job.
    pub name: String,
}
