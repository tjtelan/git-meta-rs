use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{GitCredentials, GitRepo, GitRepoCloneRequest, GitRepoInfo};
use git_url_parse::GitUrl;

use color_eyre::eyre::Result;
use tracing::{debug, info};

impl GitRepoCloneRequest {
    /// Create a new `GitRepo` with `url`.
    /// Use along with `with_*` methods to set other fields of `GitRepo`.
    /// Use `GitRepoCloner` if you need to clone the repo, and convert back with `GitRepo.into()`
    pub fn new<S: AsRef<str>>(url: S) -> Result<Self> {
        Ok(Self {
            url: GitUrl::parse(url.as_ref()).expect("url failed to parse as GitUrl"),
            credentials: None,
            head: None,
            branch: None,
            path: None,
        })
    }

    /// Set the location of `GitRepo` on the filesystem
    pub fn with_path(mut self, path: PathBuf) -> Self {
        // We want to get the absolute path of the directory of the repo
        self.path = Some(fs::canonicalize(path).expect("Directory was not found"));
        self
    }

    /// Intended to be set with the remote name branch of GitRepo
    pub fn with_branch(mut self, branch: Option<String>) -> Self {
        if let Some(b) = branch {
            self.branch = Some(b);
        }
        self
    }

    // TODO: Fix this for clone
    ///// Reinit `GitRepo` with commit id
    //pub fn with_commit(mut self, commit_id: Option<String>) -> Self {
    //    self = GitRepo::open(self.path.expect("No path set"), self.branch, commit_id)
    //        .expect("Unable to open GitRepo with commit id");
    //    self
    //}

    /// Set `GitCredentials` for private repos.
    /// `None` indicates public repo
    pub fn with_credentials(mut self, creds: Option<GitCredentials>) -> Self {
        self.credentials = creds;
        self
    }

    pub fn to_repo(&self) -> GitRepo {
        self.into()
    }

    pub fn to_info(&self) -> GitRepoInfo {
        self.into()
    }

    // TODO: Can we make this mut self?
    pub fn git_clone<P: AsRef<Path>>(&self, target: P) -> Result<GitRepo> {
        let git_info: GitRepoInfo = self.into();
        let cb = git_info.build_git2_remotecallback();

        let mut builder = git2::build::RepoBuilder::new();
        let mut fetch_options = git2::FetchOptions::new();

        fetch_options.remote_callbacks(cb);
        builder.fetch_options(fetch_options);

        if let Some(b) = &self.branch {
            builder.branch(b);
        }

        let repo = match builder.clone(&self.url.to_string(), target.as_ref()) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to clone: {}", e),
        };

        // Ensure we don't lose the credentials while updating
        let mut git_repo: GitRepo = repo.try_into()?;
        git_repo = git_repo.with_credentials(self.credentials.clone());

        Ok(git_repo)
    }

    // TODO: Can we make this mut self?
    pub fn git_clone_shallow<P: AsRef<Path>>(&self, target: P) -> Result<GitRepo> {
        let repo = if let Some(cred) = self.credentials.clone() {
            match cred {
                crate::GitCredentials::SshKey {
                    username,
                    public_key,
                    private_key,
                    passphrase,
                } => {
                    let mut parsed_uri = self.url.trim_auth();
                    parsed_uri.user = Some(username.to_string());

                    let shell_clone_command = Command::new("git")
                        .arg("clone")
                        .arg(format!("{}", parsed_uri))
                        .arg(format!("{}", target.as_ref().display()))
                        .arg("--no-single-branch")
                        .arg("--depth=1")
                        .arg("--config")
                        .arg(format!(
                            "core.sshcommand=ssh -i {privkey_path}",
                            privkey_path = private_key
                                .clone()
                                .into_os_string()
                                .into_string()
                                .expect("Couldn't convert path to string")
                        ))
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                        .expect("failed to run git clone");

                    let clone_out = shell_clone_command
                        .wait_with_output()
                        .expect("failed to open stdout");

                    debug!("Clone output: {:?}", clone_out);

                    // Re-create the GitCredentials
                    let creds = GitCredentials::SshKey {
                        username,
                        public_key,
                        private_key,
                        passphrase,
                    };

                    GitRepo::open(target.as_ref().to_path_buf(), None, None)
                        .unwrap_or_else(|_| {
                            panic!("Failed to open shallow clone dir: {:?}", clone_out)
                        })
                        .with_credentials(Some(creds))
                }
                crate::GitCredentials::UserPassPlaintext { username, password } => {
                    let mut cli_remote_url = self.url.clone();
                    cli_remote_url.user = Some(username.to_string());
                    cli_remote_url.token = Some(password.to_string());

                    let shell_clone_command = Command::new("git")
                        .arg("clone")
                        .arg(format!("{}", cli_remote_url))
                        .arg(format!("{}", target.as_ref().display()))
                        .arg("--no-single-branch")
                        .arg("--depth=1")
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                        .expect("Failed to run git clone");

                    let clone_out = shell_clone_command.stdout.expect("Failed to open stdout");

                    // Re-create the GitCredentials
                    let creds = GitCredentials::UserPassPlaintext { username, password };

                    GitRepo::open(target.as_ref().to_path_buf(), None, None)
                        .unwrap_or_else(|_| {
                            panic!("Failed to open shallow clone dir: {:?}", clone_out)
                        })
                        .with_credentials(Some(creds))
                }
            }
        } else {
            let parsed_uri = self.url.trim_auth();

            info!("Url: {}", format!("{}", parsed_uri));
            info!("Directory: {}", format!("{}", target.as_ref().display()));

            let shell_clone_command = Command::new("git")
                .arg("clone")
                .arg(format!("{}", parsed_uri))
                .arg(format!("{}", target.as_ref().display()))
                .arg("--no-single-branch")
                .arg("--depth=1")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to run git clone");

            let clone_out = shell_clone_command
                .wait_with_output()
                .expect("Failed to wait for output")
                .stdout;

            GitRepo::open(target.as_ref().to_path_buf(), None, None)
                .unwrap_or_else(|_| panic!("Failed to open shallow clone dir: {:?}", clone_out))
        };

        Ok(repo)
    }
}
