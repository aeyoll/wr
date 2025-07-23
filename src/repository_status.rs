#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RepositoryStatus {
    UpToDate,
    NeedToPull,
    NeedToPush,
    Diverged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_status_debug_formatting() {
        let statuses = vec![
            RepositoryStatus::UpToDate,
            RepositoryStatus::NeedToPull,
            RepositoryStatus::NeedToPush,
            RepositoryStatus::Diverged,
        ];

        for status in &statuses {
            let debug_str = format!("{:?}", status);
            assert!(!debug_str.is_empty());
            assert!(debug_str.len() > 5); // Should be meaningful names
        }
    }

    #[test]
    fn repository_status_equality() {
        assert_eq!(RepositoryStatus::UpToDate, RepositoryStatus::UpToDate);
        assert_eq!(RepositoryStatus::NeedToPull, RepositoryStatus::NeedToPull);
        assert_eq!(RepositoryStatus::NeedToPush, RepositoryStatus::NeedToPush);
        assert_eq!(RepositoryStatus::Diverged, RepositoryStatus::Diverged);

        assert_ne!(RepositoryStatus::UpToDate, RepositoryStatus::NeedToPull);
        assert_ne!(RepositoryStatus::NeedToPush, RepositoryStatus::Diverged);
    }

    #[test]
    fn repository_status_clone() {
        let status = RepositoryStatus::UpToDate;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn repository_status_copy() {
        let status = RepositoryStatus::Diverged;
        let copied = status; // Copy semantics
        assert_eq!(status, copied);
    }

    #[test]
    fn repository_status_pattern_matching() {
        let test_cases = vec![
            (RepositoryStatus::UpToDate, "up_to_date"),
            (RepositoryStatus::NeedToPull, "need_to_pull"),
            (RepositoryStatus::NeedToPush, "need_to_push"),
            (RepositoryStatus::Diverged, "diverged"),
        ];

        for (status, expected) in test_cases {
            let result = match status {
                RepositoryStatus::UpToDate => "up_to_date",
                RepositoryStatus::NeedToPull => "need_to_pull",
                RepositoryStatus::NeedToPush => "need_to_push",
                RepositoryStatus::Diverged => "diverged",
            };
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn repository_status_all_variants_covered() {
        // Ensure all variants can be created and used
        let variants = vec![
            RepositoryStatus::UpToDate,
            RepositoryStatus::NeedToPull,
            RepositoryStatus::NeedToPush,
            RepositoryStatus::Diverged,
        ];

        for variant in variants {
            // Test that all variants can be formatted and cloned
            let _debug = format!("{:?}", variant);
            let _cloned = variant.clone();
            let _copied = variant;
        }
    }

    #[test]
    fn repository_status_represents_git_states() {
        // Test that the enum variants make sense for Git repository states

        // UpToDate: local and remote are the same
        let up_to_date = RepositoryStatus::UpToDate;
        assert_eq!(format!("{:?}", up_to_date), "UpToDate");

        // NeedToPull: remote has changes that local doesn't
        let need_pull = RepositoryStatus::NeedToPull;
        assert_eq!(format!("{:?}", need_pull), "NeedToPull");

        // NeedToPush: local has changes that remote doesn't
        let need_push = RepositoryStatus::NeedToPush;
        assert_eq!(format!("{:?}", need_push), "NeedToPush");

        // Diverged: both local and remote have different changes
        let diverged = RepositoryStatus::Diverged;
        assert_eq!(format!("{:?}", diverged), "Diverged");
    }

    #[test]
    fn repository_status_can_be_used_in_collections() {
        use std::collections::HashSet;

        let mut status_set = HashSet::new();
        status_set.insert(RepositoryStatus::UpToDate);
        status_set.insert(RepositoryStatus::NeedToPull);
        status_set.insert(RepositoryStatus::NeedToPush);
        status_set.insert(RepositoryStatus::Diverged);

        assert_eq!(status_set.len(), 4);
        assert!(status_set.contains(&RepositoryStatus::UpToDate));
        assert!(status_set.contains(&RepositoryStatus::Diverged));
    }
}
