use git2::Oid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RepositoryStatus {
    UpToDate,
    NeedToPull,
    NeedToPush,
    Diverged,
}

impl RepositoryStatus {
    /// Compare local, upstream, and merge-base OIDs (see https://stackoverflow.com/a/3278427).
    pub fn classify(local: Oid, remote: Oid, base: Oid) -> Self {
        if local == remote {
            Self::UpToDate
        } else if local == base {
            Self::NeedToPull
        } else if remote == base {
            Self::NeedToPush
        } else {
            Self::Diverged
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; 20]).unwrap()
    }

    #[test]
    fn classify_up_to_date_when_local_equals_remote() {
        let a = oid(1);
        assert_eq!(
            RepositoryStatus::classify(a, a, oid(2)),
            RepositoryStatus::UpToDate
        );
    }

    #[test]
    fn classify_need_to_pull_when_local_equals_base() {
        let local = oid(1);
        let remote = oid(2);
        assert_eq!(
            RepositoryStatus::classify(local, remote, local),
            RepositoryStatus::NeedToPull
        );
    }

    #[test]
    fn classify_need_to_push_when_remote_equals_base() {
        let local = oid(1);
        let remote = oid(2);
        assert_eq!(
            RepositoryStatus::classify(local, remote, remote),
            RepositoryStatus::NeedToPush
        );
    }

    #[test]
    fn classify_diverged_when_all_distinct() {
        assert_eq!(
            RepositoryStatus::classify(oid(1), oid(2), oid(3)),
            RepositoryStatus::Diverged
        );
    }
}
