use anyhow::{anyhow, Error};
use duct::cmd;
use std::{env, path::Path};

use crate::{DEVELOP_BRANCH, MASTER_BRANCH};

pub struct System;

impl System {
    /// Test if git is installed
    fn check_git(&self) -> Result<(), Error> {
        let output = cmd!("which", "git").stdout_capture().run()?;

        match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(anyhow!("\"git\" not found. Please install git.")),
        }
    }

    /// Test if git-flow is installed
    fn check_git_flow(&self) -> Result<(), Error> {
        let output = cmd!("git", "flow", "version").stdout_capture().run()?;

        match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(anyhow!("\"git-flow\" not found. Please install git-flow.")),
        }
    }

    /// Test if git-flow AVH is installed
    fn check_git_flow_version(&self) -> Result<(), Error> {
        let output = cmd!("git", "flow", "version").read()?;

        match output.contains("AVH").then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("You have the wrong version of git flow installed. If you are on MacOS, make sure to install 'git-flow-avh'"))
        }
    }

    /// Test if a file exists
    fn file_exists(&self, file_name: String) -> bool {
        let current_dir = env::current_dir().unwrap();
        let path = format!("{}/{}", current_dir.display(), file_name);

        Path::new(&path).exists()
    }

    /// Test is the command is launched into a git directory
    fn has_git_repository(&self) -> Result<(), Error> {
        match self.file_exists(".git/config".to_string()).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("Please launch wr in a git repository")),
        }
    }

    /// Test if the repository is initializated with git flow
    fn is_git_flow_initialized(&self) -> Result<(), Error> {
        let output = cmd!("git", "flow", "config")
            .stdout_capture()
            .stderr_capture()
            .run();

        match output {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow!("Please run 'git flow init'.")),
        }
    }

    /// Test the active branch in a git repository
    fn is_on_branch(&self, branch_name: String) -> Result<(), Error> {
        let output = cmd!("git", "rev-parse", "--abbrev-ref", "HEAD")
            .read()
            .unwrap();
        match (output == branch_name).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("Please checkout the {} branch", branch_name)),
        }
    }

    /// Test if an upstream branch is correclty defined
    fn is_upsteam_branch_defined(&self, branch_name: String) -> Result<(), Error> {
        let output = cmd!(
            "git",
            "rev-parse",
            "--symbolic-full-name",
            "--abbrev-ref",
            format!("{branch_name}@{{u}}", branch_name = branch_name)
        )
        .stdout_capture()
        .stderr_capture()
        .run();

        match output {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow!("
                Upstream branches are not correctly defined.
                Please run 'git checkout {branch_name} && git branch --set-upstream-to=origin/{branch_name} {branch_name}'.",
                branch_name=branch_name
            )),
        }
    }

    /// Test if a repository is synced with the origin
    fn is_repository_synced_with_origin(&self) -> Result<(), Error> {
        let local = cmd!("git", "rev-parse", "@").read()?;
        let remote = cmd!("git", "rev-parse", "@{u}",).read()?;
        let base = cmd!("git", "merge-base", "@", "@{u}",).read()?;

        let need_pull = local != remote && local != base && remote == base;

        match (need_pull).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("Please run 'git fetch', 'git pull origin {develop}', 'git checkout {master} && git pull origin {master}'.", develop=*DEVELOP_BRANCH, master=*MASTER_BRANCH))
        }
    }

    /// Test if the repository has a .gitlab-ci.yml
    // fn has_gitlab_ci(&self) -> bool {
    //     self.file_exists(".gitlab-ci.yml".to_string())
    // }

    /// Test if repository is clean
    fn is_repository_clean(&self) -> Result<(), Error> {
        let output = cmd!("git", "status", "--porcelain",).read()?;

        match (output.is_empty()).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!(
                "Dirty. Please commit your last changes before running wr."
            )),
        }
    }

    /// Perform system checks
    pub fn system_check(&self) -> Result<(), Error> {
        debug!("Checking for git.");
        self.check_git()?;

        debug!("Checking for git-flow.");
        self.check_git_flow()?;

        debug!("Checking for git-flow version.");
        self.check_git_flow_version()?;

        debug!("Checking for git repository.");
        self.has_git_repository()?;

        debug!("Checking if the repository has git-flow initialized.");
        self.is_git_flow_initialized()?;

        debug!("Checking if the repository is on the develop branch.");
        self.is_on_branch(DEVELOP_BRANCH.to_string())?;

        debug!("Checking if upsteams are defined.");
        self.is_upsteam_branch_defined(MASTER_BRANCH.to_string())?;
        self.is_upsteam_branch_defined(DEVELOP_BRANCH.to_string())?;

        debug!("Checking if the repository is up-to-date with origin.");
        self.is_repository_synced_with_origin()?;

        /*
        info!("Checking for .gitlab-ci.yml.");
        if self.has_gitlab_ci() {
            info!(".gitlab-ci.yml found");
        } else {
            warn!(".gitlab-ci.yml not found");
        }
        */

        debug!("Checking if repository is clean.");
        self.is_repository_clean()?;

        Ok(())
    }
}
