use git_meta::GitRepo;
use mktemp::Temp;

#[test]
fn files_changed() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
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
        .list_files_changed(
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
fn files_not_changed() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    let files = vec!["LICENSE", ".gitignore"];

    for f in repo
        .list_files_changed(
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
fn dir_changed() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    assert!(repo.has_path_changed_between("src", "9c6c5e", "c097ad"));
}

#[test]
fn non_existent_dir_changed() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    assert!(!repo.has_path_changed("not_a_dir"));
}