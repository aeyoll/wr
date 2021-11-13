use anyhow::{anyhow, Error};
use duct::cmd;
use git2::{ErrorCode, FetchOptions, Repository, StatusOptions};
use std::{env, path::Path};

use crate::{
    git::{self, get_gitflow_branches_refs, get_remote},
    DEVELOP_BRANCH, MASTER_BRANCH,
};

pub struct System<'a> {
    pub repository: &'a Repository,
}

impl System<'_> {
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
        let head = match self.repository.head() {
            Ok(head) => Some(head),
            Err(ref e)
                if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound =>
            {
                None
            }
            Err(e) => return Err(anyhow!(e)),
        };
        let head = head.as_ref().and_then(|h| h.shorthand());

        match (head.unwrap() == branch_name).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("Please checkout the {} branch", branch_name)),
        }
    }

    /// Test if an upstream branch is correctly defined
    fn is_upsteam_branch_defined(&self, branch_name: String) -> Result<(), Error> {
        let spec = format!("{branch_name}@{{u}}", branch_name = branch_name);
        let revspec = self.repository.revparse(&spec);

        match revspec {
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
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(git::create_remote_callback().unwrap());
        fetch_options.download_tags(git2::AutotagOption::All);

        let mut remote = get_remote(self.repository)?;

        // Fetch first
        let branches_refs: Vec<String> = get_gitflow_branches_refs();
        remote.fetch(&branches_refs, Some(&mut fetch_options), None)?;

        // Then compare local and remote
        let local = self.repository.revparse("@{0}")?.from().unwrap().id();
        let remote = self.repository.revparse("@{u}")?.from().unwrap().id();

        let is_up_to_date = local == remote;

        match (is_up_to_date).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("Please update the repository first")),
        }
    }

    /// Test if the repository has a .gitlab-ci.yml
    fn has_gitlab_ci(&self) -> bool {
        self.file_exists(".gitlab-ci.yml".to_string())
    }

    /// Test if repository is clean
    fn is_repository_clean(&self) -> Result<(), Error> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);

        let statuses = self.repository.statuses(Some(&mut opts))?;

        match (statuses.is_empty()).then(|| 0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!(
                "Repository is dirty. Please commit or stash your last changes before running wr."
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

        debug!("Checking if the repository has git-flow initialized.");
        self.is_git_flow_initialized()?;

        debug!(
            "Checking if the repository is on the {} branch.",
            DEVELOP_BRANCH.to_string()
        );
        self.is_on_branch(DEVELOP_BRANCH.to_string())?;

        debug!("Checking if upsteams are defined.");
        self.is_upsteam_branch_defined(MASTER_BRANCH.to_string())?;
        self.is_upsteam_branch_defined(DEVELOP_BRANCH.to_string())?;

        debug!("Checking if the repository is up-to-date with origin.");
        self.is_repository_synced_with_origin()?;

        debug!("Checking for .gitlab-ci.yml.");
        if self.has_gitlab_ci() {
            debug!(".gitlab-ci.yml found");
        } else {
            warn!(".gitlab-ci.yml not found");
        }

        debug!("Checking if repository is clean.");
        self.is_repository_clean()?;

        Ok(())
    }
}
