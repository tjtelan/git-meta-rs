// """
// $ git diff --name-only 9c6c5e65c3590e299316d34718674de333bdd9c8  c097ad2a8c07bf2e3df64e6e603eee0473ad8133
// CHANGELOG.md
// Cargo.toml
// README.md
// src/clone.rs
// src/info.rs
// src/lib.rs
// src/types.rs
// """

use color_eyre::eyre::Result;
use git_meta::GitRepo;

use std::env;

fn main() -> Result<()> {
    let current_dir = env::current_dir()?;

    let repo = GitRepo::open(current_dir, None, None)?;

    println!(
        "{:?}",
        repo.list_files_changed(
            "9c6c5e65c3590e299316d34718674de333bdd9c8",
            "c097ad2a8c07bf2e3df64e6e603eee0473ad8133"
        )
    );

    println!(
        "Has Cargo.toml changed?: {:?}",
        repo.has_path_changed("Cargo.toml")
    );

    println!("Has src changed?: {:?}", repo.has_path_changed("src"));

    println!(
        "Has LICENSE changed?: {:?}",
        repo.has_path_changed("LICENSE")
    );

    Ok(())
}
