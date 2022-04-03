#[derive(Debug, Copy, Clone)]
pub enum RepositoryStatus {
    UpToDate,
    NeedToPull,
    NeedToPush,
    Diverged,
}
