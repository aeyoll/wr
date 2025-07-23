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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn job_can_be_deserialized_from_json() {
        let json = r#"
        {
            "id": 12345,
            "status": "running",
            "name": "deploy_prod"
        }
        "#;

        let job: Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.id, 12345);
        assert_eq!(job.status, StatusState::Running);
        assert_eq!(job.name, "deploy_prod");
    }

    #[test]
    fn job_handles_all_status_states() {
        let status_tests = vec![
            ("created", StatusState::Created),
            ("waiting_for_resource", StatusState::WaitingForResource),
            ("preparing", StatusState::Preparing),
            ("pending", StatusState::Pending),
            ("running", StatusState::Running),
            ("success", StatusState::Success),
            ("failed", StatusState::Failed),
            ("canceled", StatusState::Canceled),
            ("skipped", StatusState::Skipped),
            ("manual", StatusState::Manual),
            ("scheduled", StatusState::Scheduled),
        ];

        for (status_str, expected_status) in status_tests {
            let json = format!(
                r#"
            {{
                "id": 1,
                "status": "{}",
                "name": "test_job"
            }}
            "#,
                status_str
            );

            let job: Job = serde_json::from_str(&json).unwrap();
            assert_eq!(
                job.status, expected_status,
                "Failed for status: {}",
                status_str
            );
        }
    }

    #[test]
    fn job_has_correct_field_types() {
        let json = r#"
        {
            "id": 9876543210,
            "status": "success",
            "name": "very-long-job-name-with-special-chars_123"
        }
        "#;

        let job: Job = serde_json::from_str(json).unwrap();

        // Test u64 can handle large IDs
        assert_eq!(job.id, 9876543210u64);

        // Test string handling
        assert_eq!(job.name, "very-long-job-name-with-special-chars_123");
        assert!(job.name.len() > 20);
    }

    #[test]
    fn job_debug_formatting_works() {
        let json = r#"
        {
            "id": 123,
            "status": "running",
            "name": "test_job"
        }
        "#;

        let job: Job = serde_json::from_str(json).unwrap();
        let debug_str = format!("{:?}", job);

        assert!(debug_str.contains("Job"));
        assert!(debug_str.contains("123"));
        assert!(debug_str.contains("test_job"));
    }

    #[test]
    fn job_deserialization_fails_with_invalid_status() {
        let json = r#"
        {
            "id": 123,
            "status": "invalid_status",
            "name": "test_job"
        }
        "#;

        let result: Result<Job, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn job_deserialization_fails_with_missing_fields() {
        let test_cases = vec![
            r#"{"status": "running", "name": "test"}"#, // missing id
            r#"{"id": 123, "name": "test"}"#,           // missing status
            r#"{"id": 123, "status": "running"}"#,      // missing name
        ];

        for json in test_cases {
            let result: Result<Job, _> = serde_json::from_str(json);
            assert!(result.is_err(), "Should fail for JSON: {}", json);
        }
    }
}
