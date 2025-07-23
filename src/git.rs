use regex::Regex;
use std::{env, path::Path};

use anyhow::{anyhow, Error};
use git2::{Config, Cred, Remote, RemoteCallbacks, Repository};

use crate::{DEVELOP_BRANCH, MASTER_BRANCH};

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
    let path = format!("{}/.git/config", current_dir.display());
    let config = Config::open(Path::new(&path)).unwrap();
    config
}

/// Get gitlab host from the environment variable
pub fn get_gitlab_host() -> String {
    env::var("GITLAB_HOST").unwrap_or_else(|_| "gitlab.com".to_string())
}

/// Get gitlab token from the environment variable
pub fn get_gitlab_token() -> String {
    env::var("GITLAB_TOKEN").unwrap_or_else(|_| "".to_string())
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
    let config_path = "remote.origin.url";
    let remote_url = config.get_string(config_path).unwrap();

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
    let remote = repository.find_remote("origin")?;
    debug!("Found git repository's remote.");

    Ok(remote)
}

/// Get the gitflow branches refs
pub fn get_gitflow_branches_refs() -> Vec<String> {
    let branches = [MASTER_BRANCH.to_string(), DEVELOP_BRANCH.to_string()];
    let branches_refs: Vec<String> = branches.iter().map(|a| ref_by_branch(a)).collect();
    branches_refs
}

#[cfg(test)]
mod tests {
    use crate::git::{extract_project_name_from_remote_url, ref_by_branch, ref_by_tag};

    #[test]
    fn format_a_branch_ref() {
        assert_eq!("refs/heads/main:refs/heads/main", ref_by_branch("main"));
    }

    #[test]
    fn format_a_tag_ref() {
        assert_eq!("refs/tags/1.0.0:refs/tags/1.0.0", ref_by_tag("1.0.0"));
    }

    #[test]
    fn extracts_project_name_from_a_ssh_remote_url() {
        assert_eq!(
            "aeyoll/wr",
            extract_project_name_from_remote_url("git@github.com:aeyoll/wr.git")
        )
    }
}
