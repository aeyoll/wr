use gitlab::Gitlab;
use semver::Version;

use crate::{environment::Environment, git, semver_type::SemverType};
use anyhow::Error;
use git2::{BranchType, PushOptions, Remote, Repository};

use dialoguer::{theme::ColorfulTheme, Confirm};
use duct::cmd;

use crate::DEVELOP_BRANCH;

pub struct Release {
    pub gitlab: Gitlab,
    pub repository: Repository,
    pub environment: Environment,
    pub semver_type: SemverType,
}

impl Release {
    /// Fetch the latest tag from a git repository
    fn get_last_tag(&self) -> Result<Version, Error> {
        let tags = self.repository.tag_names(None).unwrap();

        let latest = tags
            .iter()
            .map(|x| Version::parse(x.unwrap()).unwrap())
            .max_by(|x, y| x.cmp(y))
            .unwrap();

        Ok(latest)
    }

    /// Compute the next tag from the existing tag
    fn get_next_tag(&self) -> Result<Version, Error> {
        let default_version = Version::parse("1.0.0").unwrap();
        let last_tag = self.get_last_tag().unwrap_or(default_version);
        let mut next_tag = last_tag;

        match self.semver_type {
            SemverType::Major => next_tag.major += 1,
            SemverType::Minor => next_tag.minor += 1,
            SemverType::Patch => next_tag.patch += 1,
        }

        Ok(next_tag)
    }

    /// Fetch the remote
    fn get_remote(&self) -> Result<Remote, Error> {
        let remote = self.repository.find_remote("origin")?;

        Ok(remote)
    }

    ///
    fn push_branch(&self, branch_name: String) -> Result<(), Error> {
        let mut remote = self.get_remote()?;

        remote.push(
            &[git::ref_by_branch(&branch_name)],
            Some(&mut PushOptions::new()),
        )?;

        Ok(())
    }

    /// Create the new release
    pub fn create(&self) -> Result<(), Error> {
        let next_tag = self.get_next_tag()?;

        info!("This will create release tag {}.", next_tag);

        match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to continue?")
            .interact_opt()
            .unwrap()
        {
            Some(true) => {
                info!("Creating release {}.", next_tag);
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
            }
            Some(false) => info!("Cancelling."),
            None => info!("Aborting."),
        }

        Ok(())
    }

    /// Deploy to the staging environment
    pub fn deploy_staging(&self) -> Result<(), Error> {
        self.push_branch("develop".to_string())?;
        Ok(())
    }

    /// Deploy to the production environment
    pub fn deploy_prod(&self) -> Result<(), Error> {
        let mut po = git2::PushOptions::new();
        po.remote_callbacks(git::create_remote_callback().unwrap());

        let branches: Vec<String> = self
            .repository
            .branches(Some(BranchType::Local))
            .unwrap()
            .map(|a| a.unwrap())
            .map(|(a, _)| a.name().unwrap().unwrap().to_string())
            .collect();

        let refs: Vec<String> = branches.iter().map(|a| git::ref_by_branch(a)).collect();

        let mut remote = self.get_remote()?;
        remote.push(&refs, Some(&mut po))?;
        Ok(())
    }

    /// Deploy the release
    pub fn deploy(&self) -> Result<(), Error> {
        if self.environment == Environment::Production {
            self.deploy_prod()?;
        } else if self.environment == Environment::Staging {
            self.deploy_staging()?;
        }

        Ok(())
    }
}
