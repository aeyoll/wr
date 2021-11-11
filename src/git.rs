use std::{env, path::Path};

use anyhow::Error;
use git2::{Config, Cred, RemoteCallbacks};

pub fn ref_by_branch(branch: &str) -> String {
    format!("refs/heads/{}:refs/heads/{}", branch, branch)
}

pub fn create_remove_callback() -> Result<RemoteCallbacks<'static>, Error> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, _username_from_url, _allowed_types| {
        Cred::ssh_key(
            "git",
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });

    Ok(cb)
}

pub fn get_gitflow_branch_name(branch: &str) -> String {
    let current_dir = env::current_dir().unwrap();
    let path = format!("{}/.git/config", current_dir.display());
    let config = Config::open(Path::new(&path)).unwrap();

    let config_path = format!("gitflow.branch.{}", &branch);
    config.get_string(&config_path).unwrap()
}
