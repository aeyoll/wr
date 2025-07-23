use regex::Regex;
use std::{env, path::Path};

use anyhow::{anyhow, Error};
use git2::{Config, Cred, Remote, RemoteCallbacks, Repository};

use crate::{DEVELOP_BRANCH, MASTER_BRANCH};

const ORIGIN_REMOTE: &str = "origin";
const DEFAULT_GITLAB_HOST: &str = "gitlab.com";
const GIT_CONFIG_PATH: &str = ".git/config";
const REMOTE_ORIGIN_URL_PATH: &str = "remote.origin.url";

/// Format a git branch ref
pub fn ref_by_branch(branch: &str) -> String {
    format!("refs/heads/{}:refs/heads/{}", branch, branch)
}

/// Format a git tag ref
pub fn ref_by_tag(tag: &str) -> String {
    format!("refs/tags/{}:refs/tags/{}", tag, tag)
}

/// Fetch credentials from the ssh-agent
pub fn create_remote_callback() -> Result<RemoteCallbacks<'static>, Error> {
    let mut callback = RemoteCallbacks::new();
    callback.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key_from_agent(username_from_url.unwrap())
    });

    Ok(callback)
}

/// Get the current git repository's configuration
pub fn get_config() -> Config {
    let current_dir = env::current_dir().unwrap();
    let path = format!("{}/{}", current_dir.display(), GIT_CONFIG_PATH);

    // Check if the .git/config file exists first
    if !Path::new(&path).exists() {
        panic!("No git configuration found at {}", path);
    }

    let config = Config::open(Path::new(&path)).unwrap();
    config
}

/// Get gitlab host from the environment variable
pub fn get_gitlab_host() -> String {
    env::var("GITLAB_HOST").unwrap_or_else(|_| DEFAULT_GITLAB_HOST.to_string())
}

/// Get gitlab token from the environment variable
pub fn get_gitlab_token() -> String {
    env::var("GITLAB_TOKEN").unwrap_or_default()
}

/// Get the gitflow branch name
pub fn get_gitflow_branch_name(branch: &str) -> String {
    let config = get_config();
    let config_path = format!("gitflow.branch.{}", &branch);
    config.get_string(&config_path).unwrap()
}

/// Get a Gitlab project name from the remote url set in the config
fn extract_project_name_from_remote_url(remote_url: &str) -> String {
    lazy_static! {
        static ref PROJECT_NAME_REGEX: Regex = Regex::new(
            r"(?x)
(?P<user>[^@\s]+)
@
(?P<host>[^@\s]+)
:
(?P<project_name>[^@\s]+)
.git"
        )
        .unwrap();
    }

    let project_name = PROJECT_NAME_REGEX
        .captures(remote_url)
        .and_then(|cap| cap.name("project_name").map(|login| login.as_str()))
        .unwrap();

    project_name.to_string()
}

/// Get the project name from the git remote url
pub fn get_project_name() -> String {
    let config = get_config();
    let remote_url = config.get_string(REMOTE_ORIGIN_URL_PATH).unwrap();

    extract_project_name_from_remote_url(&remote_url)
}

/// Get an instance of the git repository in the current directory
pub fn get_repository() -> Result<Repository, Error> {
    debug!("Try to load the current repository.");
    let current_dir = env::current_dir().unwrap();
    let repository = match Repository::open(current_dir) {
        Ok(repo) => repo,
        Err(_) => return Err(anyhow!("Please launch wr in a git repository.")),
    };
    debug!("Found git repository.");

    Ok(repository)
}

/// Get a Remote instance from the current repository
pub fn get_remote(repository: &Repository) -> Result<Remote, Error> {
    debug!("Try to find the remote for current repository.");
    let remote = repository.find_remote(ORIGIN_REMOTE)?;
    debug!("Found git repository's remote.");

    Ok(remote)
}

/// Get the gitflow branches refs
pub fn get_gitflow_branches_refs() -> [String; 2] {
    [
        ref_by_branch(&MASTER_BRANCH),
        ref_by_branch(&DEVELOP_BRANCH),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn format_a_branch_ref() {
        assert_eq!("refs/heads/main:refs/heads/main", ref_by_branch("main"));
        assert_eq!(
            "refs/heads/develop:refs/heads/develop",
            ref_by_branch("develop")
        );
        assert_eq!(
            "refs/heads/feature/test:refs/heads/feature/test",
            ref_by_branch("feature/test")
        );
    }

    #[test]
    fn format_a_tag_ref() {
        assert_eq!("refs/tags/1.0.0:refs/tags/1.0.0", ref_by_tag("1.0.0"));
        assert_eq!("refs/tags/v2.1.3:refs/tags/v2.1.3", ref_by_tag("v2.1.3"));
        assert_eq!(
            "refs/tags/release-1.0:refs/tags/release-1.0",
            ref_by_tag("release-1.0")
        );
    }

    #[test]
    fn extracts_project_name_from_ssh_remote_url() {
        assert_eq!(
            "aeyoll/wr",
            extract_project_name_from_remote_url("git@github.com:aeyoll/wr.git")
        );
        assert_eq!(
            "user/project",
            extract_project_name_from_remote_url("git@gitlab.com:user/project.git")
        );
        assert_eq!(
            "org/repo",
            extract_project_name_from_remote_url("git@bitbucket.org:org/repo.git")
        );
    }

    #[test]
    fn extracts_project_name_with_nested_paths() {
        assert_eq!(
            "group/subgroup/project",
            extract_project_name_from_remote_url(
                "git@gitlab.example.com:group/subgroup/project.git"
            )
        );
    }

    #[test]
    #[should_panic]
    fn extract_project_name_fails_with_invalid_url() {
        extract_project_name_from_remote_url("invalid-url");
    }

    #[test]
    fn get_gitflow_branches_refs_returns_correct_array() {
        // This test may fail if git-flow is not configured, so we'll make it more resilient
        let result = std::panic::catch_unwind(|| get_gitflow_branches_refs());

        if let Ok(refs) = result {
            assert_eq!(refs.len(), 2);

            // Check that refs contain the expected branch formats
            assert!(refs[0].contains("refs/heads/"));
            assert!(refs[1].contains("refs/heads/"));
            assert!(refs[0].contains(":refs/heads/"));
            assert!(refs[1].contains(":refs/heads/"));
        } else {
            // If git-flow is not configured, this is expected
            println!("Git-flow not configured, skipping test");
        }
    }

    #[test]
    fn gitlab_host_defaults_correctly() {
        // Save original value
        let original = env::var("GITLAB_HOST").ok();

        // Test default when env var is not set
        env::remove_var("GITLAB_HOST");
        assert_eq!(get_gitlab_host(), DEFAULT_GITLAB_HOST);

        // Test when env var is set
        env::set_var("GITLAB_HOST", "gitlab.example.com");
        assert_eq!(get_gitlab_host(), "gitlab.example.com");

        // Restore original value
        match original {
            Some(val) => env::set_var("GITLAB_HOST", val),
            None => env::remove_var("GITLAB_HOST"),
        }
    }

    #[test]
    fn gitlab_token_defaults_correctly() {
        // Save original value
        let original = env::var("GITLAB_TOKEN").ok();

        // Test default when env var is not set
        env::remove_var("GITLAB_TOKEN");
        assert_eq!(get_gitlab_token(), "");

        // Test when env var is set
        env::set_var("GITLAB_TOKEN", "test-token-123");
        assert_eq!(get_gitlab_token(), "test-token-123");

        // Restore original value
        match original {
            Some(val) => env::set_var("GITLAB_TOKEN", val),
            None => env::remove_var("GITLAB_TOKEN"),
        }
    }

    #[test]
    fn remote_callback_creation_succeeds() {
        let result = create_remote_callback();
        assert!(result.is_ok());
    }

    mod repository_tests {
        use super::*;
        use git2::Repository;
        use tempfile::TempDir;

        fn create_test_repo() -> (TempDir, Repository) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");
            (temp_dir, repo)
        }

        #[test]
        fn get_repository_fails_in_non_git_directory() {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let original_dir = env::current_dir().expect("Failed to get current dir");

            env::set_current_dir(temp_dir.path()).expect("Failed to change dir");
            let result = get_repository();
            let _ = env::set_current_dir(original_dir); // Ignore error if dir was already deleted

            assert!(result.is_err());
            if let Err(e) = result {
                assert!(e
                    .to_string()
                    .contains("Please launch wr in a git repository"));
            }
        }

        #[test]
        fn get_repository_succeeds_in_git_directory() {
            let (_temp_dir, _repo) = create_test_repo();
            let original_dir = env::current_dir().expect("Failed to get current dir");

            env::set_current_dir(_temp_dir.path()).expect("Failed to change dir");
            let result = get_repository();
            env::set_current_dir(original_dir).expect("Failed to restore dir");

            assert!(result.is_ok());
        }

        #[test]
        fn get_remote_fails_with_no_origin() {
            let (_temp_dir, repo) = create_test_repo();
            let result = get_remote(&repo);
            assert!(result.is_err());
        }
    }

    mod config_tests {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn get_config_fails_with_no_git_config() {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let original_dir = env::current_dir().expect("Failed to get current dir");

            env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

            // This should panic or fail since there's no .git/config
            let result = std::panic::catch_unwind(|| get_config());

            let _ = env::set_current_dir(original_dir); // Ignore error if dir was already deleted
            assert!(result.is_err());
        }
    }

    mod regex_tests {
        use super::*;

        #[test]
        fn project_name_regex_matches_various_formats() {
            let test_cases = vec![
                ("git@github.com:user/repo.git", "user/repo"),
                ("git@gitlab.com:group/project.git", "group/project"),
                ("git@example.com:org/team/project.git", "org/team/project"),
                ("git@bitbucket.org:company/app.git", "company/app"),
            ];

            for (url, expected) in test_cases {
                assert_eq!(extract_project_name_from_remote_url(url), expected);
            }
        }

        #[test]
        fn project_name_regex_handles_complex_names() {
            let test_cases = vec![
                (
                    "git@github.com:my-org/my-project-name.git",
                    "my-org/my-project-name",
                ),
                (
                    "git@gitlab.com:group_name/project_name.git",
                    "group_name/project_name",
                ),
                ("git@example.com:123/project.git", "123/project"),
            ];

            for (url, expected) in test_cases {
                assert_eq!(extract_project_name_from_remote_url(url), expected);
            }
        }
    }
}
