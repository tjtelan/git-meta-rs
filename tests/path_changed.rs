use std::path::PathBuf;

use git_meta::GitRepo;
use mktemp::Temp;

#[test]
fn files_changed_at_commit() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .to_clone()
        .git_clone(&tempdir)
        .unwrap();

    let expected_files = vec![
        "CHANGELOG.md",
        "Cargo.toml",
        "examples/open.rs",
        "src/lib.rs",
    ];

    let changed_files = repo
        .to_info()
        .list_files_changed_at("a7cf222c46ad32f2802e79e1935f753a27adc9e8")
        .unwrap()
        .unwrap();

    for f in expected_files {
        assert!(changed_files.contains(&PathBuf::from(f)))
    }
}

#[test]
fn files_not_changed_at_commit() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .to_clone()
        .git_clone(&tempdir)
        .unwrap();

    let expected_files = vec!["README.md", "src/clone.rs", "src/info.rs"];

    let changed_files = repo
        .to_info()
        .list_files_changed_at("a7cf222c46ad32f2802e79e1935f753a27adc9e8")
        .unwrap()
        .unwrap();

    for f in expected_files {
        assert!(!changed_files.contains(&PathBuf::from(f)))
    }
}

#[test]
fn files_changed_between_2_commits() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .to_clone()
        .git_clone(&tempdir)
        .unwrap();

    let files = vec![
        "CHANGELOG.md",
        "Cargo.toml",
        "README.md",
        "src/clone.rs",
        "src/info.rs",
        "src/lib.rs",
        "src/types.rs",
    ];

    for f in repo
        .to_info()
        .list_files_changed_between(
            "9c6c5e65c3590e299316d34718674de333bdd9c8",
            "c097ad2a8c07bf2e3df64e6e603eee0473ad8133",
        )
        .unwrap()
        .unwrap()
        .iter()
    {
        assert!(files.contains(&f.display().to_string().as_str()))
    }
}

#[test]
fn files_not_changed_between_2_commits() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .to_clone()
        .git_clone(&tempdir)
        .unwrap();

    let files = vec!["LICENSE", ".gitignore"];

    for f in repo
        .to_info()
        .list_files_changed_between(
            "9c6c5e65c3590e299316d34718674de333bdd9c8",
            "c097ad2a8c07bf2e3df64e6e603eee0473ad8133",
        )
        .unwrap()
        .unwrap()
        .iter()
    {
        assert!(!files.contains(&f.display().to_string().as_str()))
    }
}

#[test]
fn dir_changed_between_2_commits() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .to_clone()
        .git_clone(&tempdir)
        .unwrap();

    assert!(repo
        .to_info()
        .has_path_changed_between("src", "9c6c5e", "c097ad")
        .unwrap());
}

#[test]
fn non_existent_dir_changed() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .to_clone()
        .git_clone(&tempdir)
        .unwrap();

    assert!(!repo.to_info().has_path_changed("not_a_dir").unwrap());
}
