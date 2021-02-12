use color_eyre::eyre::Result;
use git_meta::GitRepo;

use std::env;

fn main() -> Result<()> {
    let current_dir = env::current_dir()?;

    let repo = GitRepo::open(current_dir, None, None)?;

    print!("{:?}", repo);

    Ok(())
}
