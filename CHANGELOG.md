# [0.0.1]() (2021-02-06)
- Merging functionality from [Orbital](https://github.com/orbitalci/orbital) and from [git-event-rs](https://github.com/tjtelan/git-event-rs)
- Introduced `GitRepo` struct
- Introduced `GitCommitMeta` impl `new` with some options.
- Converting `GitCommitMeta::epoch_time` timestamps from `i64` to `GitCommitchrono::DateTime<Utc>`
- Convert commit hash from Vec[u8] to String
- `GitCredential::SshKey` take in `PathBuf` to clarify usage
- Introduced temporary `GitRepoCloner` struct with `From<GitRepo>` and vice-versa