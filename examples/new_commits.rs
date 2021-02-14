use color_eyre::eyre::Result;
use git_meta::GitRepo;
use mktemp::Temp;

fn main() -> Result<()> {
    let tempdir = Temp::new_dir()?;

    // We're just using this for cloning
    let _clone_repo =
        GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")?.git_clone(&tempdir)?;

    let repo = GitRepo::open(
        tempdir.to_path_buf(),
        Some("main".to_string()),
        Some("f6eb3d6b7998989a48ed1024313fcac401c175fb".to_string()),
    )?;

    println!("Are there new commits?: {:?}", repo.new_commits_exist());

    Ok(())
}
