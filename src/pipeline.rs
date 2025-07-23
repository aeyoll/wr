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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json;

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
        assert_eq!(pipeline.sha, "abc123def456");
        assert_eq!(
            pipeline.web_url,
            "https://gitlab.com/project/-/pipelines/12345"
        );
    }

    #[test]
    fn pipeline_can_be_serialized_to_json() {
        let created_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap().into();
        let updated_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 30, 0).unwrap().into();

        let pipeline = Pipeline {
            id: 12345,
            status: "success".to_string(),
            r#ref: "main".to_string(),
            sha: "abc123def456".to_string(),
            web_url: "https://gitlab.com/project/-/pipelines/12345".to_string(),
            created_at: created_time,
            updated_at: updated_time,
        };

        let json = serde_json::to_string(&pipeline).unwrap();
        assert!(json.contains("12345"));
        assert!(json.contains("success"));
        assert!(json.contains("main"));
        assert!(json.contains("abc123def456"));
    }

    #[test]
    fn pipeline_clone_works() {
        let created_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap().into();
        let updated_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 30, 0).unwrap().into();

        let pipeline = Pipeline {
            id: 12345,
            status: "running".to_string(),
            r#ref: "develop".to_string(),
            sha: "def456abc123".to_string(),
            web_url: "https://gitlab.com/project/-/pipelines/12345".to_string(),
            created_at: created_time,
            updated_at: updated_time,
        };

        let cloned = pipeline.clone();
        assert_eq!(pipeline.id, cloned.id);
        assert_eq!(pipeline.status, cloned.status);
        assert_eq!(pipeline.r#ref, cloned.r#ref);
        assert_eq!(pipeline.sha, cloned.sha);
        assert_eq!(pipeline.web_url, cloned.web_url);
    }

    #[test]
    fn pipeline_debug_formatting_works() {
        let created_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap().into();
        let updated_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 30, 0).unwrap().into();

        let pipeline = Pipeline {
            id: 999,
            status: "failed".to_string(),
            r#ref: "feature/test".to_string(),
            sha: "deadbeef".to_string(),
            web_url: "https://example.com".to_string(),
            created_at: created_time,
            updated_at: updated_time,
        };

        let debug_str = format!("{:?}", pipeline);
        assert!(debug_str.contains("Pipeline"));
        assert!(debug_str.contains("999"));
        assert!(debug_str.contains("failed"));
        assert!(debug_str.contains("feature/test"));
        assert!(debug_str.contains("deadbeef"));
    }

    mod status_state_tests {
        use super::*;

        #[test]
        fn status_state_deserializes_all_variants() {
            let test_cases = vec![
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

            for (json_str, expected) in test_cases {
                let result: StatusState = serde_json::from_str(json_str).unwrap();
                assert_eq!(result, expected, "Failed to deserialize: {}", json_str);
            }
        }

        #[test]
        fn status_state_fails_with_invalid_value() {
            let invalid_values = vec![
                "\"invalid\"",
                "\"RUNNING\"", // case sensitive
                "\"Success\"", // case sensitive
                "\"\"",        // empty string
                "null",
            ];

            for invalid in invalid_values {
                let result: Result<StatusState, _> = serde_json::from_str(invalid);
                assert!(result.is_err(), "Should fail for: {}", invalid);
            }
        }

        #[test]
        fn status_state_equality_works() {
            assert_eq!(StatusState::Running, StatusState::Running);
            assert_eq!(StatusState::Success, StatusState::Success);
            assert_ne!(StatusState::Running, StatusState::Success);
            assert_ne!(StatusState::Failed, StatusState::Canceled);
        }

        #[test]
        fn status_state_debug_formatting() {
            let states = vec![
                StatusState::Created,
                StatusState::Running,
                StatusState::Success,
                StatusState::Failed,
            ];

            for state in states {
                let debug_str = format!("{:?}", state);
                assert!(!debug_str.is_empty());
                assert!(debug_str.len() > 3); // Should be a meaningful name
            }
        }

        #[test]
        fn status_state_pattern_matching() {
            let test_state = StatusState::Running;

            let result = match test_state {
                StatusState::Created => "created",
                StatusState::Running => "running",
                StatusState::Success => "success",
                StatusState::Failed => "failed",
                _ => "other",
            };

            assert_eq!(result, "running");
        }

        #[test]
        fn status_state_covers_all_gitlab_states() {
            // Ensure we have all the common GitLab CI/CD states
            let expected_states = vec![
                "created",
                "waiting_for_resource",
                "preparing",
                "pending",
                "running",
                "success",
                "failed",
                "canceled",
                "skipped",
                "manual",
                "scheduled",
            ];

            for state_str in expected_states {
                let json = format!("\"{}\"", state_str);
                let result: Result<StatusState, _> = serde_json::from_str(&json);
                assert!(
                    result.is_ok(),
                    "Missing support for GitLab state: {}",
                    state_str
                );
            }
        }
    }

    mod pipeline_field_tests {
        use super::*;

        #[test]
        fn pipeline_handles_long_values() {
            let json = r#"
            {
                "id": 9999999999999999999,
                "status": "this-is-a-very-long-status-string-that-might-come-from-custom-gitlab-instances",
                "ref": "feature/very-long-branch-name-with-lots-of-details-and-special-chars_123",
                "sha": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                "web_url": "https://very-long-gitlab-instance-hostname.example.com/very/deep/project/path/-/pipelines/9999999999999999999",
                "created_at": "2023-12-31T23:59:59+00:00",
                "updated_at": "2024-01-01T00:00:00+00:00"
            }
            "#;

            let result: Result<Pipeline, _> = serde_json::from_str(json);
            assert!(result.is_ok(), "Should handle long values");

            let pipeline = result.unwrap();
            assert_eq!(pipeline.id, 9999999999999999999u64);
            assert!(pipeline.sha.len() == 64); // Standard Git SHA length
        }

        #[test]
        fn pipeline_handles_special_characters() {
            let json = r#"
            {
                "id": 1,
                "status": "success",
                "ref": "feature/fix-unicode-🚀-support",
                "sha": "abc123",
                "web_url": "https://gitlab.com/user/project/-/pipelines/1",
                "created_at": "2023-01-01T12:00:00+00:00",
                "updated_at": "2023-01-01T12:00:00+00:00"
            }
            "#;

            let result: Result<Pipeline, _> = serde_json::from_str(json);
            assert!(result.is_ok(), "Should handle Unicode characters");

            let pipeline = result.unwrap();
            assert!(pipeline.r#ref.contains("🚀"));
        }

        #[test]
        fn pipeline_ref_field_uses_raw_identifier() {
            // Test that we can access the ref field despite it being a Rust keyword
            let json = r#"
            {
                "id": 1,
                "status": "success",
                "ref": "main",
                "sha": "abc123",
                "web_url": "https://gitlab.com/user/project/-/pipelines/1",
                "created_at": "2023-01-01T12:00:00+00:00",
                "updated_at": "2023-01-01T12:00:00+00:00"
            }
            "#;

            let pipeline: Pipeline = serde_json::from_str(json).unwrap();
            assert_eq!(pipeline.r#ref, "main");
        }
    }
}
