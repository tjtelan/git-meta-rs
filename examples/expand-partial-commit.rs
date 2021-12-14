// """
// $ git show c097ad2
// [Git log for commit: c097ad2a8c07bf2e3df64e6e603eee0473ad8133]
// """

use color_eyre::eyre::Result;
use git_meta::GitRepo;

use std::env;

fn main() -> Result<()> {
    let current_dir = env::current_dir()?;

    let repo = GitRepo::open(current_dir, None, None)?;

    println!("{:?}", repo.to_info().expand_partial_commit_id("c097ad2"));

    Ok(())
}
