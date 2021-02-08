use std::collections::HashMap;
use std::path::PathBuf;

use chrono::prelude::*;
use git_url_parse::GitUrl;

/// `GitCredentials` holds authentication information for a remote git repository
#[derive(Clone, Debug, PartialEq)]
pub enum GitCredentials {
    SshKey {
        username: String,
        public_key: Option<PathBuf>,
        private_key: PathBuf,
        passphrase: Option<String>,
    },
    UserPassPlaintext {
        username: String,
        password: String,
    },
}

/// Use `GitRepo::open()` to read a repo on disk. `GitRepo::new()` if you need to clone the repo.
///
/// Clone a repo with `.git_clone()` or `git_clone_shallow()`
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitRepo {
    /// The remote url of the repo
    pub url: GitUrl,
    /// The current commit. This can be configured prior to clone with `with_commit()`
    pub head: Option<GitCommitMeta>,
    /// The ssh key or user/pass needed to clone for private repo
    pub credentials: Option<GitCredentials>,
    /// The name of the remote branch.
    /// This can be configured with a local branch name prior to clone with `with_branch()`.
    pub branch: Option<String>,
    /// The location of the repo on disk
    pub path: Option<PathBuf>,
}

/// `GitCommitMeta` holds basic info about a single commit
#[derive(Clone, Debug, PartialEq)]
pub struct GitCommitMeta {
    /// The SHA-1 hash of the commit
    pub id: String,
    /// The commit message of the commit
    pub message: Option<String>,
    /// The timestamp of the commit in `Utc`
    pub timestamp: Option<DateTime<Utc>>,
}

pub type BranchHeads = HashMap<String, GitCommitMeta>;
