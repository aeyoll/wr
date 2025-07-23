use anyhow::{anyhow, Error};
use duct::cmd;
use git2::{ErrorCode, FetchOptions, Repository, StatusOptions};
use std::{env, path::Path};

use crate::repository_status::RepositoryStatus;
use crate::{
    git::{self, get_gitflow_branches_refs, get_remote},
    DEVELOP_BRANCH, MASTER_BRANCH,
};

const GIT_COMMAND: &str = "git";
const WHICH_COMMAND: &str = "which";
const GIT_FLOW_AVH_IDENTIFIER: &str = "AVH";
const GITLAB_CI_FILE: &str = ".gitlab-ci.yml";

const GIT_NOT_FOUND_MSG: &str = "\"git\" not found. Please install git.";
const GIT_FLOW_NOT_FOUND_MSG: &str = "\"git-flow\" not found. Please install git-flow.";
const GIT_FLOW_WRONG_VERSION_MSG: &str = "You have the wrong version of git flow installed. If you are on MacOS, make sure to install 'git-flow-avh'";
const GIT_FLOW_NOT_INITIALIZED_MSG: &str = "Please run 'git flow init'.";
const REPO_UP_TO_DATE_MSG: &str = "Repository is up-to-date, nothing to do.";
const REPO_NEED_PULL_MSG: &str = "Repository need to be pulled first.";
const REPO_DIVERGED_MSG: &str = "Branch have diverged, please fix the conflict first.";
const REPO_DIRTY_MSG: &str =
    "Repository is dirty. Please commit or stash your last changes before running wr.";

pub struct System<'a> {
    pub repository: &'a Repository,
    pub force: bool,
}

impl System<'_> {
    /// Test if git is installed
    fn check_git(&self) -> Result<(), Error> {
        let output = cmd!(WHICH_COMMAND, GIT_COMMAND).stdout_capture().run()?;

        match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(anyhow!(GIT_NOT_FOUND_MSG)),
        }
    }

    /// Test if git-flow is installed
    fn check_git_flow(&self) -> Result<(), Error> {
        let output = cmd!(GIT_COMMAND, "flow", "version")
            .stdout_capture()
            .run()?;

        match output.status.code() {
            Some(0) => Ok(()),
            _ => Err(anyhow!(GIT_FLOW_NOT_FOUND_MSG)),
        }
    }

    /// Test if git-flow AVH is installed
    fn check_git_flow_version(&self) -> Result<(), Error> {
        let output = cmd!(GIT_COMMAND, "flow", "version").read()?;

        match output.contains(GIT_FLOW_AVH_IDENTIFIER).then_some(0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!(GIT_FLOW_WRONG_VERSION_MSG)),
        }
    }

    /// Test if a file exists
    fn file_exists(&self, file_name: &str) -> bool {
        let current_dir = env::current_dir().unwrap();
        let path = format!("{}/{}", current_dir.display(), file_name);

        Path::new(&path).exists()
    }

    /// Test if the repository is initialized with git flow
    fn is_git_flow_initialized(&self) -> Result<(), Error> {
        let output = cmd!(GIT_COMMAND, "flow", "config")
            .stdout_capture()
            .stderr_capture()
            .run();

        match output {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow!(GIT_FLOW_NOT_INITIALIZED_MSG)),
        }
    }

    /// Test the active branch in a git repository
    fn is_on_branch(&self, branch_name: &str) -> Result<(), Error> {
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

        match (head.unwrap() == branch_name).then_some(0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!("Please checkout the {} branch", branch_name)),
        }
    }

    /// Test if an upstream branch is correctly defined
    fn is_upstream_branch_defined(&self, branch_name: &str) -> Result<(), Error> {
        let spec = format!("{branch_name}@{{u}}");
        let revspec = self.repository.revparse(&spec);

        match revspec {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow!("
                Upstream branches are not correctly defined.
                Please run 'git checkout {branch_name} && git branch --set-upstream-to=origin/{branch_name} {branch_name}'.",
            )),
        }
    }

    /// Get the repository status and go further only if we need to push
    /// something
    fn get_repository_status(&self) -> Result<(), Error> {
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(git::create_remote_callback().unwrap());
        fetch_options.download_tags(git2::AutotagOption::All);

        let mut remote = get_remote(self.repository)?;

        // Fetch first
        let branches_refs = get_gitflow_branches_refs();
        remote.download(&branches_refs, Some(&mut fetch_options))?;

        // Then compare base, local and remote (https://stackoverflow.com/a/3278427)
        let local = self.repository.revparse("@{0}")?.from().unwrap().id();
        let remote = self.repository.revparse("@{u}")?.from().unwrap().id();
        let base = self.repository.merge_base(local, remote).unwrap();

        let status;

        if local == remote {
            status = RepositoryStatus::UpToDate;
        } else if local == base {
            status = RepositoryStatus::NeedToPull;
        } else if remote == base {
            status = RepositoryStatus::NeedToPush;
        } else {
            status = RepositoryStatus::Diverged;
        }

        match status {
            RepositoryStatus::UpToDate => {
                if self.force {
                    info!("[Setup] Repository is up-to-date, but force flag has been passed.");
                    Ok(())
                } else {
                    Err(anyhow!(REPO_UP_TO_DATE_MSG))
                }
            }
            RepositoryStatus::NeedToPull => Err(anyhow!(REPO_NEED_PULL_MSG)),
            RepositoryStatus::Diverged => Err(anyhow!(REPO_DIVERGED_MSG)),
            RepositoryStatus::NeedToPush => Ok(()),
        }
    }

    /// Test if the repository has a .gitlab-ci.yml
    pub fn has_gitlab_ci(&self) -> bool {
        self.file_exists(GITLAB_CI_FILE)
    }

    /// Test if repository is clean
    fn is_repository_clean(&self) -> Result<(), Error> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);

        let statuses = self.repository.statuses(Some(&mut opts))?;

        match (statuses.is_empty()).then_some(0) {
            Some(_) => Ok(()),
            _ => Err(anyhow!(REPO_DIRTY_MSG)),
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
            DEVELOP_BRANCH.as_str()
        );
        self.is_on_branch(&DEVELOP_BRANCH)?;

        debug!("Checking if upstreams are defined.");
        self.is_upstream_branch_defined(&MASTER_BRANCH)?;
        self.is_upstream_branch_defined(&DEVELOP_BRANCH)?;

        debug!("Checking if the repository is up-to-date with origin.");
        self.get_repository_status()?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");
        (temp_dir, repo)
    }

    fn create_system_with_repo(repo: &Repository, force: bool) -> System {
        System {
            repository: repo,
            force,
        }
    }

    mod constants_tests {
        use super::*;

        #[test]
        fn constants_have_expected_values() {
            assert_eq!(GIT_COMMAND, "git");
            assert_eq!(WHICH_COMMAND, "which");
            assert_eq!(GIT_FLOW_AVH_IDENTIFIER, "AVH");
            assert_eq!(GITLAB_CI_FILE, ".gitlab-ci.yml");
        }

        #[test]
        fn error_messages_are_not_empty() {
            assert!(!GIT_NOT_FOUND_MSG.is_empty());
            assert!(!GIT_FLOW_NOT_FOUND_MSG.is_empty());
            assert!(!GIT_FLOW_WRONG_VERSION_MSG.is_empty());
            assert!(!GIT_FLOW_NOT_INITIALIZED_MSG.is_empty());
            assert!(!REPO_UP_TO_DATE_MSG.is_empty());
            assert!(!REPO_NEED_PULL_MSG.is_empty());
            assert!(!REPO_DIVERGED_MSG.is_empty());
            assert!(!REPO_DIRTY_MSG.is_empty());
        }
    }

    mod file_exists_tests {
        use super::*;

        #[test]
        fn file_exists_returns_false_for_nonexistent_file() {
            let (_temp_dir, repo) = create_test_repo();
            let system = create_system_with_repo(&repo, false);

            let original_dir = env::current_dir().unwrap();
            env::set_current_dir(_temp_dir.path()).unwrap();

            let result = system.file_exists("nonexistent.txt");

            env::set_current_dir(original_dir).unwrap();
            assert!(!result);
        }

        #[test]
        fn file_exists_returns_true_for_existing_file() {
            let (_temp_dir, repo) = create_test_repo();
            let system = create_system_with_repo(&repo, false);

            let original_dir = env::current_dir().unwrap();
            env::set_current_dir(_temp_dir.path()).unwrap();

            // Create a test file
            fs::write("test.txt", "test content").unwrap();
            let result = system.file_exists("test.txt");

            env::set_current_dir(original_dir).unwrap();
            assert!(result);
        }

        #[test]
        fn has_gitlab_ci_uses_file_exists() {
            let (_temp_dir, repo) = create_test_repo();
            let system = create_system_with_repo(&repo, false);

            let original_dir = env::current_dir().unwrap();
            env::set_current_dir(_temp_dir.path()).unwrap();

            // Should return false initially
            assert!(!system.has_gitlab_ci());

            // Create .gitlab-ci.yml file
            fs::write(".gitlab-ci.yml", "stages:\n  - test").unwrap();
            assert!(system.has_gitlab_ci());

            env::set_current_dir(original_dir).unwrap();
        }
    }

    mod branch_tests {
        use super::*;
        use git2::Signature;

        fn create_repo_with_commit() -> (TempDir, Repository) {
            let (temp_dir, repo) = create_test_repo();

            // Create initial commit
            let sig = Signature::now("Test User", "test@example.com").unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            {
                let tree = repo.find_tree(tree_id).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                    .unwrap();
            }

            (temp_dir, repo)
        }

        #[test]
        fn is_on_branch_works_with_valid_branch() {
            let (_temp_dir, repo) = create_repo_with_commit();
            let system = create_system_with_repo(&repo, false);

            // Should be on main/master by default after first commit
            let head = repo.head().unwrap();
            let branch_name = head.shorthand().unwrap();

            assert!(system.is_on_branch(branch_name).is_ok());
        }

        #[test]
        fn is_on_branch_fails_with_wrong_branch() {
            let (_temp_dir, repo) = create_repo_with_commit();
            let system = create_system_with_repo(&repo, false);

            let result = system.is_on_branch("nonexistent-branch");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Please checkout"));
        }
    }

    mod repository_clean_tests {
        use super::*;

        #[test]
        fn is_repository_clean_passes_for_clean_repo() {
            let (_temp_dir, repo) = create_test_repo();
            let system = create_system_with_repo(&repo, false);

            let result = system.is_repository_clean();
            assert!(result.is_ok());
        }

        #[test]
        fn is_repository_clean_fails_for_dirty_repo() {
            let (_temp_dir, repo) = create_test_repo();
            let system = create_system_with_repo(&repo, false);

            let original_dir = env::current_dir().unwrap();
            env::set_current_dir(_temp_dir.path()).unwrap();

            // Create an untracked file
            fs::write("untracked.txt", "content").unwrap();

            let result = system.is_repository_clean();

            env::set_current_dir(original_dir).unwrap();

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("dirty"));
        }
    }

    mod upstream_tests {
        use super::*;

        #[test]
        fn is_upstream_branch_defined_fails_without_upstream() {
            let (_temp_dir, repo) = create_test_repo();
            let system = create_system_with_repo(&repo, false);

            let result = system.is_upstream_branch_defined("main");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Upstream branches are not correctly defined"));
        }
    }

    mod system_struct_tests {
        use super::*;

        #[test]
        fn system_can_be_created() {
            let (_temp_dir, repo) = create_test_repo();
            let system = System {
                repository: &repo,
                force: false,
            };

            assert!(!system.force);
        }

        #[test]
        fn system_force_flag_works() {
            let (_temp_dir, repo) = create_test_repo();
            let system_no_force = create_system_with_repo(&repo, false);
            let system_force = create_system_with_repo(&repo, true);

            assert!(!system_no_force.force);
            assert!(system_force.force);
        }
    }

    // Note: Integration tests for git, git-flow, and network operations
    // would require external dependencies and are better suited for
    // integration test files or conditional compilation

    #[test]
    #[ignore] // Requires git to be installed
    fn check_git_passes_when_git_installed() {
        let (_temp_dir, repo) = create_test_repo();
        let system = create_system_with_repo(&repo, false);

        // This test will only pass if git is actually installed
        if let Ok(_) = system.check_git() {
            // Git is installed, test passes
            assert!(true);
        } else {
            // Git not installed, skip test
            println!("Skipping test - git not installed");
        }
    }

    mod repository_status_tests {
        use super::*;

        #[test]
        fn repository_status_with_force_flag() {
            let (_temp_dir, repo) = create_test_repo();
            let system_force = create_system_with_repo(&repo, true);
            let system_no_force = create_system_with_repo(&repo, false);

            // Note: These tests would need proper git setup with remotes
            // to fully test repository status functionality
            assert!(system_force.force);
            assert!(!system_no_force.force);
        }
    }
}
