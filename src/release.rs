use semver::Version;
use std::thread::sleep;
use std::time::Duration;

use crate::error::ReleaseError;
use crate::{
    environment::Environment,
    git::{self, get_gitflow_branches_refs, get_remote},
    job::{find_playable_deploy_job, Job},
    pipeline::{find_active_pipeline_id, Pipeline, StatusState},
    semver_type::SemverType,
};
use git2::{PushOptions, Repository};
use gitlab::{
    api::{
        common::SortOrder,
        projects::{self, pipelines::PipelineOrderBy},
        Query,
    },
    Gitlab,
};

use dialoguer::{theme::ColorfulTheme, Confirm};
use duct::cmd;

use crate::{DEVELOP_BRANCH, GITLAB_HOST, PROJECT_NAME};

fn gitlab_err(err: impl ToString) -> ReleaseError {
    ReleaseError::Gitlab {
        message: err.to_string(),
    }
}

/// Highest parseable semver tag in the repository.
pub fn last_tag(repository: &Repository) -> Result<Version, ReleaseError> {
    let tags = repository.tag_names(None).unwrap();

    let latest_tag = tags
        .iter()
        .filter_map(|x| x.ok().flatten().and_then(|s| Version::parse(s).ok()))
        .max_by(|x, y| x.cmp(y));

    match latest_tag {
        Some(version) => Ok(version),
        None => Err(ReleaseError::NoTagFound),
    }
}

/// Next version from `last_tag`, or `1.0.0` when none exist.
pub fn next_tag(repository: &Repository, semver_type: SemverType) -> Result<Version, ReleaseError> {
    let next: Version = match last_tag(repository) {
        Ok(last) => {
            let mut next = last;

            match semver_type {
                SemverType::Major => {
                    next.major += 1;
                    next.minor = 0;
                    next.patch = 0;
                }
                SemverType::Minor => {
                    next.minor += 1;
                    next.patch = 0;
                }
                SemverType::Patch => next.patch += 1,
            }

            next
        }
        Err(_) => Version::new(1, 0, 0),
    };

    Ok(next)
}

pub struct Release<'a> {
    pub gitlab: Gitlab,
    pub repository: &'a Repository,
    pub environment: Environment,
    pub semver_type: SemverType,
    /// Skip the interactive confirmation (for CI usage)
    pub assume_yes: bool,
}

impl Release<'_> {
    /// Fetch the latest tag from a git repository
    pub fn get_last_tag(&self) -> Result<Version, ReleaseError> {
        last_tag(self.repository)
    }

    /// Compute the next tag from the existing tag
    pub fn get_next_tag(&self) -> Result<Version, ReleaseError> {
        next_tag(self.repository, self.semver_type)
    }

    /// Push a branch to the remote
    fn push_branch(&self, branch_name: &str) -> Result<(), ReleaseError> {
        let mut push_options = self.get_push_options();
        let mut remote = get_remote(self.repository)?;

        remote.push(&[git::ref_by_branch(branch_name)], Some(&mut push_options))?;

        Ok(())
    }

    /// Create a production release
    pub fn create_production_release(&self) -> Result<(), ReleaseError> {
        let next_tag = self.get_next_tag()?;

        info!("[Release] This will create release tag {next_tag}.");

        let confirmation = if self.assume_yes {
            info!("[Release] Confirmation skipped (--yes).");
            Some(true)
        } else {
            Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you want to continue?")
                .interact_opt()
                .unwrap()
        };

        match confirmation {
            Some(true) => {
                info!("[Release] Creating release {next_tag}.");
                cmd!("git", "flow", "release", "start", next_tag.to_string())
                    .stdout_capture()
                    .stderr_capture()
                    .read()?;
                cmd!(
                    "git",
                    "flow",
                    "release",
                    "finish",
                    "-m",
                    next_tag.to_string(),
                    next_tag.to_string()
                )
                .stdout_capture()
                .stderr_capture()
                .read()?;

                cmd!("git", "checkout", DEVELOP_BRANCH.to_string())
                    .stdout_capture()
                    .stderr_capture()
                    .read()?;

                Ok(())
            }
            Some(false) => Err(ReleaseError::Cancelled),
            None => Err(ReleaseError::Aborted),
        }
    }

    /// Create the new release
    pub fn create(&self) -> Result<(), ReleaseError> {
        match self.environment {
            Environment::Production => self.create_production_release(),
            Environment::Staging => Ok(()),
        }
    }

    /// Get the push options
    pub fn get_push_options(&self) -> PushOptions<'static> {
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(git::create_remote_callback());
        push_options
    }

    /// Deploy to the staging environment
    pub fn push_staging(&self) -> Result<(), ReleaseError> {
        self.push_branch(&DEVELOP_BRANCH)?;
        Ok(())
    }

    /// Deploy to the production environment
    pub fn push_production(&self) -> Result<(), ReleaseError> {
        let mut push_options = self.get_push_options();
        let mut remote = get_remote(self.repository)?;

        // Push main and develop branches
        let branches_refs = get_gitflow_branches_refs();
        remote.push(&branches_refs, Some(&mut push_options))?;

        // Push only the current release tag
        let current_tag = self.get_last_tag()?;
        let tag_ref = git::ref_by_tag(&current_tag.to_string());
        remote.push(&[tag_ref], Some(&mut push_options))?;

        Ok(())
    }

    /// Push the release
    pub fn push(&self) -> Result<(), ReleaseError> {
        match self.environment {
            Environment::Production => self.push_production()?,
            Environment::Staging => self.push_staging()?,
        }

        Ok(())
    }

    /// Get a job by its id
    pub fn get_job(&self, job_id: u64) -> Result<Job, ReleaseError> {
        let job_endpoint = projects::jobs::Job::builder()
            .project(PROJECT_NAME.as_str())
            .job(job_id)
            .build()
            .unwrap();
        let job: Job = job_endpoint.query(&self.gitlab).map_err(gitlab_err)?;
        Ok(job)
    }

    /// Get the last pipeline id
    pub fn get_last_pipeline_id(&self) -> Result<u64, ReleaseError> {
        let mut last_pipeline_id: u64 = 0;
        let pipeline_ref = self.environment.get_pipeline_ref();
        let timeout = 60;
        let mut counter = 0;

        while last_pipeline_id == 0 && counter < timeout {
            sleep(Duration::from_secs(1));

            let pipelines_endpoint = projects::pipelines::Pipelines::builder()
                .project(PROJECT_NAME.as_str())
                .ref_(pipeline_ref)
                .order_by(PipelineOrderBy::Id)
                .sort(SortOrder::Descending)
                .build()
                .unwrap();

            let pipelines: Vec<Pipeline> =
                pipelines_endpoint.query(&self.gitlab).map_err(gitlab_err)?;

            if let Some(id) = find_active_pipeline_id(&pipelines) {
                last_pipeline_id = id;
            }

            counter += 1;
        }

        if last_pipeline_id == 0 {
            return Err(ReleaseError::PipelineNotFound);
        }

        Ok(last_pipeline_id)
    }

    /// Deploy to the environment
    pub fn deploy(&self) -> Result<(), ReleaseError> {
        info!("[Deploy] Fetching latest pipeline.");
        if let Ok(last_pipeline_id) = self.get_last_pipeline_id() {
            let pipeline_url = format!(
                "https://{}/{}/-/pipelines/{}",
                *GITLAB_HOST, *PROJECT_NAME, last_pipeline_id
            );
            info!("[Deploy] Pipeline id {last_pipeline_id} is running ({pipeline_url}).");

            let jobs_endpoint = projects::pipelines::PipelineJobs::builder()
                .project(PROJECT_NAME.as_str())
                .pipeline(last_pipeline_id)
                .build()
                .unwrap();

            let jobs: Vec<Job> = jobs_endpoint.query(&self.gitlab).map_err(gitlab_err)?;

            let deploy_job_name = self.environment.get_deploy_job_name();

            if let Some(job) = find_playable_deploy_job(&jobs, deploy_job_name) {
                let job_id = job.id;
                let job_name = job.name.clone();
                // While the job has the "created" state, it means other jobs
                // are pending before.
                let mut job_status = job.status;
                info!("[Deploy] Waiting for previous jobs to be over.");

                while job_status == StatusState::Created {
                    sleep(Duration::from_secs(1));
                    let job: Job = self.get_job(job_id)?;
                    job_status = job.status;
                }

                // Trigger the deploy job
                let play_job_endpoint = projects::jobs::PlayJob::builder()
                    .project(PROJECT_NAME.as_str())
                    .job(job_id)
                    .build()
                    .unwrap();

                gitlab::api::ignore(play_job_endpoint)
                    .query(&self.gitlab)
                    .map_err(gitlab_err)?;

                info!("[Deploy] Playing \"{job_name}\" job.");

                let mut job: Job = self.get_job(job_id)?;

                while job.status != StatusState::Failed && job.status != StatusState::Success {
                    sleep(Duration::from_secs(1));
                    job = self.get_job(job_id)?;
                }

                if job.status == StatusState::Failed {
                    error!("[Deploy] \"{job_name}\" job failed");
                } else if job.status == StatusState::Success {
                    info!("[Deploy] \"{job_name}\" job succeeded")
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use tempfile::TempDir;

    fn mock_gitlab() -> (MockServer, Gitlab) {
        let server = MockServer::start();
        let _user = server.mock(|when, then| {
            when.method(GET).path("/api/v4/user");
            then.status(200).body("{}");
        });
        let host = format!("127.0.0.1:{}", server.port());
        let gitlab = Gitlab::new_insecure(host, "token").expect("gitlab client");
        (server, gitlab)
    }

    fn create_test_repo_with_tags() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");

        repo.remote("origin", "git@gitlab.com:test/project.git")
            .expect("Failed to add remote");

        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let commit_oid = {
            let mut index = repo.index().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap()
        };

        {
            let commit = repo.find_commit(commit_oid).unwrap();
            repo.tag("1.0.0", commit.as_object(), &sig, "Version 1.0.0", false)
                .unwrap();
            repo.tag("1.1.0", commit.as_object(), &sig, "Version 1.1.0", false)
                .unwrap();
            repo.tag("2.0.0", commit.as_object(), &sig, "Version 2.0.0", false)
                .unwrap();
            // Ignored by Version::parse — must not win over numeric tags.
            repo.tag("v3.0.0", commit.as_object(), &sig, "prefixed", false)
                .unwrap();
            repo.tag("not-a-version", commit.as_object(), &sig, "junk", false)
                .unwrap();
        }

        (temp_dir, repo)
    }

    #[test]
    fn get_last_tag_finds_highest_semver_ignoring_invalid() {
        let (_temp_dir, repo) = create_test_repo_with_tags();
        assert_eq!(last_tag(&repo).unwrap(), Version::new(2, 0, 0));
    }

    #[test]
    fn get_last_tag_fails_with_no_tags() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        assert!(matches!(last_tag(&repo), Err(ReleaseError::NoTagFound)));
    }

    #[test]
    fn get_next_tag_increments_by_semver_type() {
        let (_temp_dir, repo) = create_test_repo_with_tags();
        assert_eq!(
            next_tag(&repo, SemverType::Patch).unwrap(),
            Version::new(2, 0, 1)
        );
        assert_eq!(
            next_tag(&repo, SemverType::Minor).unwrap(),
            Version::new(2, 1, 0)
        );
        assert_eq!(
            next_tag(&repo, SemverType::Major).unwrap(),
            Version::new(3, 0, 0)
        );
    }

    #[test]
    fn get_next_tag_defaults_to_1_0_0_with_no_tags() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        assert_eq!(
            next_tag(&repo, SemverType::Patch).unwrap(),
            Version::new(1, 0, 0)
        );
    }

    #[test]
    fn create_staging_is_noop() {
        let (_temp_dir, repo) = create_test_repo_with_tags();
        let (_server, gitlab) = mock_gitlab();
        let release = Release {
            gitlab,
            repository: &repo,
            environment: Environment::Staging,
            semver_type: SemverType::Patch,
            assume_yes: false,
        };
        assert!(release.create().is_ok());
    }
}
