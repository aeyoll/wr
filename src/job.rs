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

impl Job {
    /// Whether this job is the deploy job we can still play.
    pub fn is_playable_deploy(&self, deploy_job_name: &str) -> bool {
        self.name.contains(deploy_job_name)
            && self.status != StatusState::Failed
            && self.status != StatusState::Success
    }
}

/// First playable deploy job matching `deploy_job_name`.
pub fn find_playable_deploy_job<'a>(jobs: &'a [Job], deploy_job_name: &str) -> Option<&'a Job> {
    jobs.iter()
        .find(|job| job.is_playable_deploy(deploy_job_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: u64, name: &str, status: StatusState) -> Job {
        Job {
            id,
            name: name.to_string(),
            status,
        }
    }

    #[test]
    fn job_can_be_deserialized_from_json() {
        let json = r#"{"id": 12345, "status": "running", "name": "deploy_prod"}"#;
        let job: Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.id, 12345);
        assert_eq!(job.status, StatusState::Running);
        assert_eq!(job.name, "deploy_prod");
    }

    #[test]
    fn job_deserialization_fails_with_invalid_status() {
        let json = r#"{"id": 123, "status": "invalid_status", "name": "test_job"}"#;
        assert!(serde_json::from_str::<Job>(json).is_err());
    }

    #[test]
    fn job_deserialization_fails_with_missing_fields() {
        for json in [
            r#"{"status": "running", "name": "test"}"#,
            r#"{"id": 123, "name": "test"}"#,
            r#"{"id": 123, "status": "running"}"#,
        ] {
            assert!(serde_json::from_str::<Job>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn find_playable_deploy_job_matches_name_and_skips_terminal() {
        let jobs = vec![
            job(1, "build", StatusState::Success),
            job(2, "deploy_prod", StatusState::Failed),
            job(3, "deploy_prod", StatusState::Manual),
            job(4, "deploy_staging", StatusState::Created),
        ];

        let found = find_playable_deploy_job(&jobs, "deploy_prod").unwrap();
        assert_eq!(found.id, 3);
        assert!(
            find_playable_deploy_job(&jobs, "deploy_staging")
                .unwrap()
                .id
                == 4
        );
        assert!(find_playable_deploy_job(&jobs, "missing").is_none());
    }

    #[test]
    fn find_playable_deploy_job_rejects_success_and_failed() {
        let jobs = vec![
            job(1, "deploy_prod", StatusState::Success),
            job(2, "deploy_prod", StatusState::Failed),
        ];
        assert!(find_playable_deploy_job(&jobs, "deploy_prod").is_none());
    }
}
