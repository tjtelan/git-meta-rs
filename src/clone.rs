use crate::{GitCredentials, GitRepo};

use std::path::Path;
use std::process::{Command, Stdio};

use color_eyre::eyre::Result;
use log::{debug, info};

impl GitRepo {
    // TODO: Can we make this mut self?
    pub fn git_clone<P: AsRef<Path>>(&self, target: P) -> Result<GitRepo> {
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

        // Ensure we don't lose the credentials while updating
        let mut git_repo: GitRepo = repo.into();
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
