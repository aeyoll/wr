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

impl Pipeline {
    /// Pipeline statuses we wait for when looking up a deploy target.
    pub fn is_active_for_deploy(&self) -> bool {
        self.status == "skipped" || self.status == "running"
    }
}

/// First pipeline id that is skipped or running (newest-first list).
pub fn find_active_pipeline_id(pipelines: &[Pipeline]) -> Option<u64> {
    pipelines
        .iter()
        .find(|pipeline| pipeline.is_active_for_deploy())
        .map(|pipeline| pipeline.id)
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_pipeline(id: u64, status: &str) -> Pipeline {
        let t = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap().into();
        Pipeline {
            id,
            status: status.to_string(),
            r#ref: "main".to_string(),
            sha: "abc".to_string(),
            web_url: "https://example.com".to_string(),
            created_at: t,
            updated_at: t,
        }
    }

    #[test]
    fn pipeline_can_be_deserialized_from_json() {
        let json = r#"
        {
            "id": 12345,
            "status": "running",
            "ref": "main",
            "sha": "abc123def456",
            "web_url": "https://gitlab.com/project/-/pipelines/12345",
            "created_at": "2023-01-01T12:00:00+00:00",
            "updated_at": "2023-01-01T12:30:00+00:00"
        }
        "#;

        let pipeline: Pipeline = serde_json::from_str(json).unwrap();
        assert_eq!(pipeline.id, 12345);
        assert_eq!(pipeline.status, "running");
        assert_eq!(pipeline.r#ref, "main");
    }

    #[test]
    fn status_state_deserializes_all_variants() {
        let cases = [
            ("\"created\"", StatusState::Created),
            ("\"waiting_for_resource\"", StatusState::WaitingForResource),
            ("\"preparing\"", StatusState::Preparing),
            ("\"pending\"", StatusState::Pending),
            ("\"running\"", StatusState::Running),
            ("\"success\"", StatusState::Success),
            ("\"failed\"", StatusState::Failed),
            ("\"canceled\"", StatusState::Canceled),
            ("\"skipped\"", StatusState::Skipped),
            ("\"manual\"", StatusState::Manual),
            ("\"scheduled\"", StatusState::Scheduled),
        ];

        for (json, expected) in cases {
            assert_eq!(serde_json::from_str::<StatusState>(json).unwrap(), expected);
        }
    }

    #[test]
    fn status_state_fails_with_invalid_value() {
        for invalid in ["\"invalid\"", "\"RUNNING\"", "\"Success\"", "\"\"", "null"] {
            assert!(serde_json::from_str::<StatusState>(invalid).is_err());
        }
    }

    #[test]
    fn find_active_pipeline_id_prefers_first_skipped_or_running() {
        let pipelines = vec![
            sample_pipeline(1, "success"),
            sample_pipeline(2, "skipped"),
            sample_pipeline(3, "running"),
        ];
        assert_eq!(find_active_pipeline_id(&pipelines), Some(2));
    }

    #[test]
    fn find_active_pipeline_id_none_when_no_match() {
        let pipelines = vec![sample_pipeline(1, "success"), sample_pipeline(2, "failed")];
        assert_eq!(find_active_pipeline_id(&pipelines), None);
    }
}
