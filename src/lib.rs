//! # Git-meta
//!
//! Git-meta is a collection of functionality for gathering information about git repos and commits

//! You can open an existing repo with `GitRepo::open(path)`
//! (Branch and commits provided for example. Provide `None` to use current checked out values)
//!
//! ```ignore
//! use std::path::PathBuf;
//! use git_meta::GitRepo;
//! GitRepo::open(
//!         PathBuf::from("/path/to/repo"),
//!         Some("main".to_string()),
//!         Some("b24fe6112e97eb9ee0cc1fd5aaa520bf8814f6c3".to_string()))
//!     .expect("Unable to clone repo");
//! ```
//!
//! You can create a new repo for cloning with `GitRepo::new(url)`
//!
//! ```ignore
//! use std::path::PathBuf;
//! use git_meta::{GitCredentials, GitRepo};
//! use mktemp::Temp;
//! let temp_dir = Temp::new_dir().expect("Unable to create test clone dir");
//!
//! let creds = GitCredentials::SshKey {
//!     username: "git".to_string(),
//!     public_key: None,
//!     private_key: PathBuf::from("/path/to/private/key"),
//!     passphrase: None,
//! };
//!
//! GitRepo::new("https://github.com/tjtelan/git-meta-rs")
//!     .expect("Unable to create GitRepo")
//!     .with_credentials(Some(creds))
//!     .to_clone()
//!     .git_clone_shallow(temp_dir.as_path())
//!     .expect("Unable to clone repo");
//! ```
//!
//! *Note:* Shallow cloning requires `git` CLI to be installed

use chrono::prelude::*;
use color_eyre::eyre::Report;
use git2::Repository;
use hex::ToHex;

#[doc(hidden)]
pub mod clone;
#[doc(hidden)]
pub mod info;
#[doc(hidden)]
pub mod types;

#[doc(hidden)]
pub mod repo;

//// Can I use this as an empty trait for trait objects
//pub trait GitInfo {}
//

// Re-export our types in the root
#[doc(inline)]
pub use crate::types::*;

impl GitCommitMeta {
    /// Trait bound for `id` is to convert the output from:
    /// `git2::Commit.id().as_bytes()` into a `String`
    pub fn new<I: ToHex + AsRef<[u8]>>(id: I) -> GitCommitMeta {
        GitCommitMeta {
            id: hex::encode(id),
            message: None,
            timestamp: None,
        }
    }

    /// `time` is intended to convert output from:
    /// `git2::Commit.time().seconds()` into `Datetime<Utc>`
    pub fn with_timestamp(mut self, time: i64) -> Self {
        let naive_datetime = NaiveDateTime::from_timestamp(time, 0);
        let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);

        self.timestamp = Some(datetime);
        self
    }

    /// Set the commit message
    pub fn with_message(mut self, msg: Option<String>) -> Self {
        self.message = msg;
        self
    }
}

impl TryFrom<Repository> for GitRepo {
    type Error = Report;

    fn try_from(repo: Repository) -> Result<Self, Self::Error> {
        GitRepo::open(repo.path().to_path_buf(), None, None)
    }
}

impl From<&GitRepoInfo> for GitRepo {
    /// Convert from `&GitRepoInfo` to `GitRepo`.
    fn from(repo: &GitRepoInfo) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path.clone(),
        }
    }
}

impl From<&GitRepoCloneRequest> for GitRepo {
    /// Convert from `&GitRepoCloneRequest` to `GitRepo`.
    fn from(repo: &GitRepoCloneRequest) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path.clone(),
        }
    }
}

impl From<GitRepo> for GitRepoCloneRequest {
    /// Convert from `GitRepo` to `GitRepoCloneRequest`.
    fn from(repo: GitRepo) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path,
        }
    }
}

impl From<&GitRepo> for GitRepoCloneRequest {
    /// Convert from `GitRepo` to `GitRepoCloneRequest`.
    fn from(repo: &GitRepo) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path.clone(),
        }
    }
}

impl From<&GitRepo> for GitRepoInfo {
    /// Convert from `GitRepo` to `GitRepoCloneRequest`.
    fn from(repo: &GitRepo) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path.clone(),
        }
    }
}
impl From<&GitRepoInfo> for GitRepoCloneRequest {
    /// Convert from `&GitRepoInfo` to `GitRepoCloneRequest`.
    fn from(repo: &GitRepoInfo) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path.clone(),
        }
    }
}

impl From<&GitRepoCloneRequest> for GitRepoInfo {
    /// Convert from `&GitRepoCloneRequest` to `GitRepoInfo`.
    fn from(repo: &GitRepoCloneRequest) -> Self {
        Self {
            url: repo.url.clone(),
            head: repo.head.clone(),
            credentials: repo.credentials.clone(),
            branch: repo.branch.clone(),
            path: repo.path.clone(),
        }
    }
}
