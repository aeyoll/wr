use std::env;

use anyhow::Error;
use git2::{Cred, RemoteCallbacks};

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
