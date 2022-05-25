use chrono::{DateTime, Local};
use semver::Version;
use std::thread::sleep;
use std::time::Duration;
use std::{thread, time};

use crate::{
    environment::Environment,
    git::{self, get_gitflow_branches_refs, get_remote},
    semver_type::SemverType,
};
use anyhow::{anyhow, Error};
use git2::{PushOptions, Repository};
use gitlab::{
    api::{
        common::SortOrder,
        projects::{
            self,
            pipelines::{PipelineOrderBy, PipelineStatus},
        },
        Query,
    },
    Gitlab, Job, StatusState,
};

use dialoguer::{theme::ColorfulTheme, Confirm};
use duct::cmd;
use serde::{Deserialize, Serialize};

use crate::{DEVELOP_BRANCH, MASTER_BRANCH, PROJECT_NAME};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pipeline {
    id: u64,
    status: String,
    r#ref: String,
    sha: String,
    web_url: String,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
}

pub struct Release<'a> {
    pub gitlab: Gitlab,
    pub repository: &'a Repository,
    pub environment: Environment,
    pub semver_type: SemverType,
}

impl Release<'_> {
    /// Fetch the latest tag from a git repository
    fn get_last_tag(&self) -> Result<Version, Error> {
        let tags = self.repository.tag_names(None).unwrap();

        let latest_tag = tags
            .iter()
            .map(|x| Version::parse(x.unwrap()).unwrap())
            .max_by(|x, y| x.cmp(y));

        match latest_tag {
            Some(version) => Ok(version),
            None => Err(anyhow!("No tag found")),
        }
    }

    /// Compute the next tag from the existing tag
    fn get_next_tag(&self) -> Result<Version, Error> {
        let last_tag = self.get_last_tag();

        let next_tag: Version = match last_tag {
            Ok(last_tag) => {
                let mut next_tag = last_tag;

                match self.semver_type {
                    SemverType::Major => next_tag.major += 1,
                    SemverType::Minor => next_tag.minor += 1,
                    SemverType::Patch => next_tag.patch += 1,
                }

                next_tag
            }
            Err(_) => Version::new(1, 0, 0),
        };

        Ok(next_tag)
    }

    ///
    fn push_branch(&self, branch_name: String) -> Result<(), Error> {
        let mut push_options = self.get_push_options();
        let mut remote = get_remote(self.repository)?;

        remote.push(&[git::ref_by_branch(&branch_name)], Some(&mut push_options))?;

        Ok(())
    }

    pub fn create_production_release(&self) -> Result<(), Error> {
        let next_tag = self.get_next_tag()?;

        info!("[release] This will create release tag {}.", next_tag);

        match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to continue?")
            .interact_opt()
            .unwrap()
        {
            Some(true) => {
                info!("[release] Creating release {}.", next_tag);
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
            Some(false) => Err(anyhow!("Cancelling.")),
            None => Err(anyhow!("Aborting.")),
        }
    }

    /// Create the new release
    pub fn create(&self) -> Result<(), Error> {
        match self.environment {
            Environment::Production => self.create_production_release(),
            Environment::Staging => Ok(()),
        }
    }

    pub fn get_push_options(&self) -> PushOptions<'static> {
        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(git::create_remote_callback().unwrap());
        push_options
    }

    /// Deploy to the staging environment
    pub fn push_staging(&self) -> Result<(), Error> {
        self.push_branch(DEVELOP_BRANCH.to_string())?;
        Ok(())
    }

    /// Deploy to the production environment
    pub fn push_production(&self) -> Result<(), Error> {
        let mut push_options = self.get_push_options();

        // Push master and develop branches
        let branches_refs: Vec<String> = get_gitflow_branches_refs();
        let mut remote = get_remote(self.repository)?;
        remote.push(&branches_refs, Some(&mut push_options))?;

        // Push all tags
        let tags = self.repository.tag_names(None).unwrap();
        let tag_refs: Vec<String> = tags
            .iter()
            .map(|a| a.unwrap())
            .map(git::ref_by_tag)
            .collect();
        remote.push(&tag_refs, Some(&mut push_options))?;

        Ok(())
    }

    /// Push the release
    pub fn push(&self) -> Result<(), Error> {
        match self.environment {
            Environment::Production => self.push_production()?,
            Environment::Staging => self.push_staging()?,
        }

        Ok(())
    }

    ///
    pub fn get_deploy_job_name(&self) -> Result<String, Error> {
        let job_name = match self.environment {
            Environment::Production => "deploy_prod".to_string(),
            Environment::Staging => "deploy_staging".to_string(),
        };

        Ok(job_name)
    }

    ///
    pub fn get_pipeline_ref(&self) -> Result<String, Error> {
        let pipeline_ref = match self.environment {
            Environment::Production => MASTER_BRANCH.to_string(),
            Environment::Staging => DEVELOP_BRANCH.to_string(),
        };

        Ok(pipeline_ref)
    }

    ///
    pub fn get_job(&self, job_id: u64) -> Result<Job, Error> {
        let job_endpoint = gitlab::api::projects::jobs::Job::builder()
            .project(PROJECT_NAME.to_string())
            .job(job_id)
            .build()
            .unwrap();
        let job: Job = job_endpoint.query(&self.gitlab)?;
        Ok(job)
    }

    ///
    pub fn deploy(&self) -> Result<(), Error> {
        info!("[Deploy] Waiting 3s for the pipeline to be created.");

        let three_seconds = time::Duration::from_secs(3);
        thread::sleep(three_seconds);

        let pipeline_ref = self.get_pipeline_ref()?;

        let pipelines_endpoint = projects::pipelines::Pipelines::builder()
            .project(PROJECT_NAME.to_string())
            .ref_(pipeline_ref)
            .order_by(PipelineOrderBy::Id)
            .sort(SortOrder::Descending)
            .status(PipelineStatus::Running)
            .build()
            .unwrap();

        let pipelines: Vec<Pipeline> = pipelines_endpoint.query(&self.gitlab)?;

        let last_pipeline: Pipeline = pipelines.into_iter().next().unwrap();
        let last_pipeline_id: u64 = last_pipeline.id;

        let jobs_endpoint = gitlab::api::projects::pipelines::PipelineJobs::builder()
            .project(PROJECT_NAME.to_string())
            .pipeline(last_pipeline_id)
            .build()
            .unwrap();

        let jobs: Vec<Job> = jobs_endpoint.query(&self.gitlab)?;

        let deploy_job_name = self.get_deploy_job_name()?;

        for job in jobs {
            // Only trigger "deploy" jobs
            let is_deploy_job = job.name.contains(&deploy_job_name)
                && job.status != StatusState::Failed
                && job.status != StatusState::Success;

            if is_deploy_job {
                // While the job has the "created" state, it means other jobs
                // are pending before.
                let mut job_status = job.status;
                info!("[Deploy] Waiting for previous jobs to be over.");

                while job_status == StatusState::Created {
                    sleep(Duration::from_secs(1));
                    let job: Job = self.get_job(job.id.value())?;
                    job_status = job.status;
                }

                // Trigger the deploy job
                let play_job_endpoint = gitlab::api::projects::jobs::PlayJob::builder()
                    .project(PROJECT_NAME.to_string())
                    .job(job.id.value())
                    .build()
                    .unwrap();

                gitlab::api::ignore(play_job_endpoint).query(&self.gitlab)?;

                info!("[Deploy] Playing \"{}\" job.", job.name);

                let mut job: Job = self.get_job(job.id.value())?;

                while job.status != StatusState::Failed && job.status != StatusState::Success {
                    sleep(Duration::from_secs(1));
                    job = self.get_job(job.id.value())?;
                }

                if job.status == StatusState::Failed {
                    error!("[Deploy] \"{}\" job failed", job.name);
                } else if job.status == StatusState::Success {
                    info!("[Deploy] \"{}\" job succeded", job.name)
                }

                break;
            }
        }

        Ok(())
    }
}
