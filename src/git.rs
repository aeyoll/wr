use regex::Regex;
use std::{env, path::Path};

use anyhow::{anyhow, Error};
use git2::{Config, Cred, Remote, RemoteCallbacks, Repository};

use crate::{DEVELOP_BRANCH, MAIN_BRANCH};

const ORIGIN_REMOTE: &str = "origin";
const DEFAULT_GITLAB_HOST: &str = "gitlab.com";
const GIT_CONFIG_PATH: &str = ".git/config";
const REMOTE_ORIGIN_URL_PATH: &str = "remote.origin.url";

/// Format a git branch ref
pub fn ref_by_branch(branch: &str) -> String {
    format!("refs/heads/{branch}:refs/heads/{branch}")
}

/// Format a git tag ref
pub fn ref_by_tag(tag: &str) -> String {
    format!("refs/tags/{tag}:refs/tags/{tag}")
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
        panic!("No git configuration found at {path}");
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
    let config_path = format!("gitflow.branch.{}", branch);
    config.get_string(&config_path).unwrap()
}

/// Get a Gitlab project name from the remote url set in the config
fn extract_project_name_from_remote_url(remote_url: &str) -> String {
    lazy_static! {
        static ref PROJECT_NAME_REGEX: Regex = Regex::new(
            r"(?x)
            (?:
                [^@\s]+@[^:\s]+:
                |
                https?://[^/\s]+/
            )
            (?P<project_name>[^\s]+?)
            \.git$"
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
pub fn get_remote(repository: &Repository) -> Result<Remote<'_>, Error> {
    debug!("Try to find the remote for current repository.");
    let remote = repository.find_remote(ORIGIN_REMOTE)?;
    debug!("Found git repository's remote.");

    Ok(remote)
}

/// Get the gitflow branches refs
pub fn get_gitflow_branches_refs() -> [String; 2] {
    [ref_by_branch(&MAIN_BRANCH), ref_by_branch(&DEVELOP_BRANCH)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    fn format_a_branch_ref() {
        assert_eq!("refs/heads/main:refs/heads/main", ref_by_branch("main"));
        assert_eq!(
            "refs/heads/feature/test:refs/heads/feature/test",
            ref_by_branch("feature/test")
        );
    }

    #[test]
    fn format_a_tag_ref() {
        assert_eq!("refs/tags/1.0.0:refs/tags/1.0.0", ref_by_tag("1.0.0"));
    }

    #[test]
    fn extracts_project_name_from_ssh_and_https() {
        let cases = [
            ("git@github.com:aeyoll/wr.git", "aeyoll/wr"),
            (
                "git@gitlab.com:group/subgroup/project.git",
                "group/subgroup/project",
            ),
            ("https://gitlab.com/user/project.git", "user/project"),
            ("http://gitlab.example.com/org/team/app.git", "org/team/app"),
            (
                "https://gitlab.com/my-org/my-project-name.git",
                "my-org/my-project-name",
            ),
        ];

        for (url, expected) in cases {
            assert_eq!(extract_project_name_from_remote_url(url), expected, "{url}");
        }
    }

    #[test]
    #[should_panic]
    fn extract_project_name_fails_with_invalid_url() {
        extract_project_name_from_remote_url("invalid-url");
    }

    #[test]
    #[serial]
    fn gitlab_host_defaults_correctly() {
        let original = env::var("GITLAB_HOST").ok();
        env::remove_var("GITLAB_HOST");
        assert_eq!(get_gitlab_host(), DEFAULT_GITLAB_HOST);

        env::set_var("GITLAB_HOST", "gitlab.example.com");
        assert_eq!(get_gitlab_host(), "gitlab.example.com");

        match original {
            Some(val) => env::set_var("GITLAB_HOST", val),
            None => env::remove_var("GITLAB_HOST"),
        }
    }

    #[test]
    #[serial]
    fn gitlab_token_defaults_correctly() {
        let original = env::var("GITLAB_TOKEN").ok();
        env::remove_var("GITLAB_TOKEN");
        assert_eq!(get_gitlab_token(), "");

        env::set_var("GITLAB_TOKEN", "test-token-123");
        assert_eq!(get_gitlab_token(), "test-token-123");

        match original {
            Some(val) => env::set_var("GITLAB_TOKEN", val),
            None => env::remove_var("GITLAB_TOKEN"),
        }
    }

    #[test]
    fn remote_callback_creation_succeeds() {
        assert!(create_remote_callback().is_ok());
    }

    #[test]
    #[serial]
    fn get_repository_fails_in_non_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();

        env::set_current_dir(temp_dir.path()).unwrap();
        let result = get_repository();
        let _ = env::set_current_dir(original_dir);

        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("Please launch wr in a git repository"));
    }

    #[test]
    #[serial]
    fn get_repository_succeeds_in_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        Repository::init(temp_dir.path()).unwrap();
        let original_dir = env::current_dir().unwrap();

        env::set_current_dir(temp_dir.path()).unwrap();
        let result = get_repository();
        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn get_remote_fails_with_no_origin() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        assert!(get_remote(&repo).is_err());
    }

    #[test]
    #[serial]
    fn get_config_fails_with_no_git_config() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();

        env::set_current_dir(temp_dir.path()).unwrap();
        let result = std::panic::catch_unwind(get_config);
        let _ = env::set_current_dir(original_dir);

        assert!(result.is_err());
    }
}
