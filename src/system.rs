use duct::cmd;
use git2::{ErrorCode, FetchOptions, Repository, StatusOptions};
use std::{env, path::Path};

use crate::error::SystemError;
use crate::repository_status::RepositoryStatus;
use crate::{
    git::{self, get_gitflow_branches_refs, get_remote},
    DEVELOP_BRANCH, MAIN_BRANCH,
};

const GIT_COMMAND: &str = "git";
const GIT_FLOW_AVH_IDENTIFIER: &str = "AVH";
const GITLAB_CI_FILE: &str = ".gitlab-ci.yml";

pub struct System<'a> {
    pub repository: &'a Repository,
    pub force: bool,
}

impl System<'_> {
    /// Test if git is installed
    fn check_git(&self) -> Result<(), SystemError> {
        let output = cmd!(GIT_COMMAND, "--version").stdout_capture().run()?;

        match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(SystemError::GitNotFound),
        }
    }

    /// Test if git-flow is installed
    fn check_git_flow(&self) -> Result<(), SystemError> {
        let output = cmd!(GIT_COMMAND, "flow", "version")
            .stdout_capture()
            .run()?;

        match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(SystemError::GitFlowNotFound),
        }
    }

    /// Test if git-flow AVH is installed
    fn check_git_flow_version(&self) -> Result<(), SystemError> {
        let output = cmd!(GIT_COMMAND, "flow", "version").read()?;

        match output.contains(GIT_FLOW_AVH_IDENTIFIER).then_some(0) {
            Some(_) => Ok(()),
            _ => Err(SystemError::GitFlowWrongVersion),
        }
    }

    /// Test if a file exists
    fn file_exists(&self, file_name: &str) -> bool {
        let current_dir = env::current_dir().unwrap();
        let path = format!("{}/{}", current_dir.display(), file_name);

        Path::new(&path).exists()
    }

    /// Test if the repository is initialized with git flow
    fn is_git_flow_initialized(&self) -> Result<(), SystemError> {
        let output = cmd!(GIT_COMMAND, "flow", "config")
            .stdout_capture()
            .stderr_capture()
            .run();

        match output {
            Ok(_) => Ok(()),
            Err(_) => Err(SystemError::GitFlowNotInitialized),
        }
    }

    /// Test the active branch in a git repository
    fn is_on_branch(&self, branch_name: &str) -> Result<(), SystemError> {
        let head = match self.repository.head() {
            Ok(head) => Some(head),
            Err(ref e)
                if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound =>
            {
                None
            }
            Err(e) => return Err(SystemError::Git2(e)),
        };
        let head = head.as_ref().and_then(|h| h.shorthand().ok());

        match (head.unwrap() == branch_name).then_some(0) {
            Some(_) => Ok(()),
            _ => Err(SystemError::WrongBranch {
                branch: branch_name.to_string(),
            }),
        }
    }

    /// Test if an upstream branch is correctly defined
    fn is_upstream_branch_defined(&self, branch_name: &str) -> Result<(), SystemError> {
        let spec = format!("{branch_name}@{{u}}");
        let revspec = self.repository.revparse(&spec);

        match revspec {
            Ok(_) => Ok(()),
            Err(_) => Err(SystemError::UpstreamNotDefined {
                branch: branch_name.to_string(),
            }),
        }
    }

    /// Get the repository status and go further only if we need to push
    /// something
    fn get_repository_status(&self) -> Result<(), SystemError> {
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(git::create_remote_callback());
        fetch_options.download_tags(git2::AutotagOption::All);

        let mut remote = get_remote(self.repository)?;

        // Fetch first
        let branches_refs = get_gitflow_branches_refs();
        remote.download(&branches_refs, Some(&mut fetch_options))?;

        // Then compare base, local and remote (https://stackoverflow.com/a/3278427)
        let local = self.repository.revparse("@{0}")?.from().unwrap().id();
        let remote = self.repository.revparse("@{u}")?.from().unwrap().id();
        let base = self.repository.merge_base(local, remote).unwrap();

        self.evaluate_repository_status(RepositoryStatus::classify(local, remote, base))
    }

    /// Map classified status to release proceed / abort (honours `--force`).
    fn evaluate_repository_status(&self, status: RepositoryStatus) -> Result<(), SystemError> {
        match status {
            RepositoryStatus::UpToDate => {
                if self.force {
                    info!("[Setup] Repository is up-to-date, but force flag has been passed.");
                    Ok(())
                } else {
                    Err(SystemError::RepoUpToDate)
                }
            }
            RepositoryStatus::NeedToPull => Err(SystemError::RepoNeedPull),
            RepositoryStatus::Diverged => Err(SystemError::RepoDiverged),
            RepositoryStatus::NeedToPush => Ok(()),
        }
    }

    /// Test if the repository has a .gitlab-ci.yml
    pub fn has_gitlab_ci(&self) -> bool {
        self.file_exists(GITLAB_CI_FILE)
    }

    /// Test if repository is clean
    fn is_repository_clean(&self) -> Result<(), SystemError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            // Refresh the index against HEAD before computing statuses so that
            // libgit2 applies repository settings (e.g. core.autocrlf on Windows)
            // the same way `git status` would, preventing false "dirty" reports.
            .update_index(true);

        let statuses = self.repository.statuses(Some(&mut opts))?;

        if statuses.is_empty() {
            return Ok(());
        }

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("<unknown>");
            debug!(
                "Dirty file detected: {} (status: {:?})",
                path,
                entry.status()
            );
        }

        Err(SystemError::RepoDirty)
    }

    /// Fetch origin without enforcing NeedToPush.
    fn fetch_origin(&self) -> Result<(), SystemError> {
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(git::create_remote_callback());
        fetch_options.download_tags(git2::AutotagOption::All);

        let mut remote = get_remote(self.repository)?;
        let branches_refs = get_gitflow_branches_refs();
        remote.download(&branches_refs, Some(&mut fetch_options))?;

        Ok(())
    }

    /// Shared checks for release and hotfix.
    fn system_check_common(&self) -> Result<(), SystemError> {
        debug!("Checking for git.");
        self.check_git()?;

        debug!("Checking for git-flow.");
        self.check_git_flow()?;

        debug!("Checking for git-flow version.");
        self.check_git_flow_version()?;

        debug!("Checking if the repository has git-flow initialized.");
        self.is_git_flow_initialized()?;

        debug!("Checking if upstreams are defined.");
        self.is_upstream_branch_defined(&MAIN_BRANCH)?;
        self.is_upstream_branch_defined(&DEVELOP_BRANCH)?;

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

    /// Perform system checks for release.
    pub fn system_check(&self) -> Result<(), SystemError> {
        self.system_check_common()?;

        debug!(
            "Checking if the repository is on the {} branch.",
            DEVELOP_BRANCH.as_str()
        );
        self.is_on_branch(&DEVELOP_BRANCH)?;

        debug!("Checking if the repository is up-to-date with origin.");
        self.get_repository_status()?;

        Ok(())
    }

    /// Perform system checks for hotfix (no develop / NeedToPush requirement).
    pub fn hotfix_system_check(&self) -> Result<(), SystemError> {
        self.system_check_common()?;

        debug!("Fetching origin before hotfix.");
        self.fetch_origin()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        (temp_dir, repo)
    }

    fn create_repo_with_commit() -> (TempDir, Repository) {
        let (temp_dir, repo) = create_test_repo();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        {
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }
        (temp_dir, repo)
    }

    #[test]
    #[serial]
    fn has_gitlab_ci_detects_file() {
        let (temp_dir, repo) = create_test_repo();
        let system = System {
            repository: &repo,
            force: false,
        };
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        assert!(!system.has_gitlab_ci());
        fs::write(".gitlab-ci.yml", "stages:\n  - test").unwrap();
        assert!(system.has_gitlab_ci());

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn is_on_branch_checks_current_head() {
        let (_temp_dir, repo) = create_repo_with_commit();
        let system = System {
            repository: &repo,
            force: false,
        };
        let branch_name = repo.head().unwrap().shorthand().unwrap().to_string();
        assert!(system.is_on_branch(&branch_name).is_ok());
        assert!(matches!(
            system.is_on_branch("nonexistent-branch"),
            Err(SystemError::WrongBranch { .. })
        ));
    }

    #[test]
    #[serial]
    fn is_repository_clean_detects_untracked_files() {
        let (temp_dir, repo) = create_test_repo();
        let system = System {
            repository: &repo,
            force: false,
        };
        assert!(system.is_repository_clean().is_ok());

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();
        fs::write("untracked.txt", "content").unwrap();
        let err = system.is_repository_clean().unwrap_err();
        env::set_current_dir(original_dir).unwrap();

        assert!(matches!(err, SystemError::RepoDirty));
    }

    #[test]
    fn is_upstream_branch_defined_fails_without_upstream() {
        let (_temp_dir, repo) = create_test_repo();
        let system = System {
            repository: &repo,
            force: false,
        };
        assert!(matches!(
            system.is_upstream_branch_defined("main"),
            Err(SystemError::UpstreamNotDefined { .. })
        ));
    }

    #[test]
    fn evaluate_repository_status_honours_force_and_blocks_bad_states() {
        let (_temp_dir, repo) = create_test_repo();
        let forced = System {
            repository: &repo,
            force: true,
        };
        let normal = System {
            repository: &repo,
            force: false,
        };

        assert!(forced
            .evaluate_repository_status(RepositoryStatus::UpToDate)
            .is_ok());
        assert!(matches!(
            normal.evaluate_repository_status(RepositoryStatus::UpToDate),
            Err(SystemError::RepoUpToDate)
        ));
        assert!(normal
            .evaluate_repository_status(RepositoryStatus::NeedToPush)
            .is_ok());
        assert!(matches!(
            normal.evaluate_repository_status(RepositoryStatus::NeedToPull),
            Err(SystemError::RepoNeedPull)
        ));
        assert!(matches!(
            normal.evaluate_repository_status(RepositoryStatus::Diverged),
            Err(SystemError::RepoDiverged)
        ));
    }

    #[test]
    fn check_git_passes_when_git_installed() {
        let (_temp_dir, repo) = create_test_repo();
        let system = System {
            repository: &repo,
            force: false,
        };
        assert!(system.check_git().is_ok());
    }
}
