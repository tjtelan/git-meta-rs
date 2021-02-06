use crate::{GitCredentials, GitRepo, GitRepoCloner};

use color_eyre::eyre::Result;
use git2::Cred;
use git_url_parse::GitUrl;
use log::{debug, info};
use std::path::Path;
use std::process::{Command, Stdio};

impl From<GitRepo> for GitRepoCloner {
    fn from(repo: GitRepo) -> GitRepoCloner {
        GitRepoCloner {
            url: repo.url,
            credentials: repo.credentials,
            branch: repo.branch,
            path: repo.path,
        }
    }
}

impl GitRepoCloner {
    pub fn new<S: AsRef<str>>(url: S) -> Result<GitRepoCloner> {
        Ok(GitRepoCloner {
            url: GitUrl::parse(url.as_ref()).expect("url failed to parse as GitUrl"),
            credentials: None,
            // This is only used by clone()
            branch: None,
            path: None,
        })
    }

    pub fn with_credentials(mut self, creds: GitCredentials) -> Self {
        self.credentials = Some(creds);
        self
    }

    pub fn with_branch<S: ToString>(mut self, branch: S) -> Self {
        self.branch = Some(branch.to_string());
        self
    }

    // TODO: Change return to Result<GitRepo>
    pub fn git_clone<P: AsRef<Path>>(&self, target: P) -> Result<git2::Repository> {
        let cb = self.build_git2_remotecallback();

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

        Ok(repo)
    }

    // TODO: Change return to Result<GitRepo>
    pub fn git_clone_shallow<P: AsRef<Path>>(&self, target: P) -> Result<git2::Repository> {
        let repo = if let Some(cred) = self.credentials.clone() {
            match cred {
                crate::GitCredentials::SshKey {
                    username,
                    public_key: _,
                    private_key,
                    passphrase: _,
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

                    git2::Repository::open(target.as_ref())
                        .expect("Failed to open shallow clone dir")
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
                    git2::Repository::open(target.as_ref()).expect(
                        format!("Failed to open shallow clone dir: {:?}", clone_out).as_str(),
                    )
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

            git2::Repository::open(target.as_ref())
                .expect(format!("Failed to open shallow clone dir: {:?}", clone_out).as_str())
        };

        Ok(repo)
    }

    // FIXME: This is a copy
    pub fn build_git2_remotecallback(&self) -> git2::RemoteCallbacks {
        if let Some(cred) = self.credentials.clone() {
            debug!("Before building callback: {:?}", &cred);

            match cred {
                GitCredentials::SshKey {
                    username,
                    public_key,
                    private_key,
                    passphrase,
                } => {
                    let mut cb = git2::RemoteCallbacks::new();
                    let privkey_path = std::path::PathBuf::from(private_key);

                    cb.credentials(
                        move |_, _, _| match (public_key.clone(), passphrase.clone()) {
                            (None, None) => {
                                Ok(Cred::ssh_key(&username, None, privkey_path.as_path(), None)
                                    .expect("Could not create credentials object for ssh key"))
                            }
                            (None, Some(pp)) => Ok(Cred::ssh_key(
                                &username,
                                None,
                                privkey_path.as_path(),
                                Some(pp.as_ref()),
                            )
                            .expect("Could not create credentials object for ssh key")),
                            (Some(pk), None) => {
                                let pubkey_path = std::path::PathBuf::from(pk);

                                Ok(Cred::ssh_key(
                                    &username,
                                    Some(pubkey_path.as_path()),
                                    privkey_path.as_path(),
                                    None,
                                )
                                .expect("Could not create credentials object for ssh key"))
                            }
                            (Some(pk), Some(pp)) => {
                                let pubkey_path = std::path::PathBuf::from(pk);

                                Ok(Cred::ssh_key(
                                    &username,
                                    Some(pubkey_path.as_path()),
                                    privkey_path.as_path(),
                                    Some(pp.as_ref()),
                                )
                                .expect("Could not create credentials object for ssh key"))
                            }
                        },
                    );

                    cb
                }
                GitCredentials::UserPassPlaintext { username, password } => {
                    let mut cb = git2::RemoteCallbacks::new();
                    cb.credentials(move |_, _, _| {
                        Cred::userpass_plaintext(username.as_str(), password.as_str())
                    });

                    cb
                }
            }
        } else {
            // No credentials. Repo is public
            git2::RemoteCallbacks::new()
        }
    }
}
