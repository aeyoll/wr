use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum SemverType {
    Major,
    Minor,
    #[default]
    Patch,
}

impl FromStr for SemverType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Major" => Ok(SemverType::Major),
            "Minor" => Ok(SemverType::Minor),
            "Patch" => Ok(SemverType::Patch),
            _ => Err("Unknown SemverType"),
        }
    }
}

impl fmt::Display for SemverType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_semver_type_is_patch() {
        assert_eq!(SemverType::default(), SemverType::Patch);
    }

    #[test]
    fn from_str_parses_correctly() {
        assert_eq!("Major".parse::<SemverType>().unwrap(), SemverType::Major);
        assert_eq!("Minor".parse::<SemverType>().unwrap(), SemverType::Minor);
        assert_eq!("Patch".parse::<SemverType>().unwrap(), SemverType::Patch);
    }

    #[test]
    fn from_str_fails_for_invalid_input() {
        assert!("Invalid".parse::<SemverType>().is_err());
        assert!("major".parse::<SemverType>().is_err()); // case sensitive
        assert!("minor".parse::<SemverType>().is_err()); // case sensitive
        assert!("patch".parse::<SemverType>().is_err()); // case sensitive
        assert!("".parse::<SemverType>().is_err());
        assert!("MAJOR".parse::<SemverType>().is_err());
    }

    #[test]
    fn from_str_error_message() {
        let error = "Invalid".parse::<SemverType>().unwrap_err();
        assert_eq!(error, "Unknown SemverType");
    }

    #[test]
    fn display_formatting() {
        assert_eq!(format!("{}", SemverType::Major), "Major");
        assert_eq!(format!("{}", SemverType::Minor), "Minor");
        assert_eq!(format!("{}", SemverType::Patch), "Patch");
    }

    #[test]
    fn debug_formatting() {
        assert_eq!(format!("{:?}", SemverType::Major), "Major");
        assert_eq!(format!("{:?}", SemverType::Minor), "Minor");
        assert_eq!(format!("{:?}", SemverType::Patch), "Patch");
    }

    #[test]
    fn semver_type_equality() {
        assert_eq!(SemverType::Major, SemverType::Major);
        assert_eq!(SemverType::Minor, SemverType::Minor);
        assert_eq!(SemverType::Patch, SemverType::Patch);

        assert_ne!(SemverType::Major, SemverType::Minor);
        assert_ne!(SemverType::Major, SemverType::Patch);
        assert_ne!(SemverType::Minor, SemverType::Patch);
    }

    #[test]
    fn semver_type_clone() {
        let semver = SemverType::Major;
        let cloned = semver.clone();
        assert_eq!(semver, cloned);
    }

    #[test]
    fn semver_type_copy() {
        let semver = SemverType::Major;
        let copied = semver; // Copy semantics
        assert_eq!(semver, copied);
    }

    #[test]
    fn all_variants_covered() {
        let variants = [SemverType::Major, SemverType::Minor, SemverType::Patch];

        for variant in variants {
            // Ensure all variants can be formatted
            let _display = format!("{}", variant);
            let _debug = format!("{:?}", variant);

            // Ensure all variants can be cloned
            let _cloned = variant.clone();
        }
    }

    #[test]
    fn parse_and_format_roundtrip() {
        let variants = [SemverType::Major, SemverType::Minor, SemverType::Patch];

        for variant in variants {
            let formatted = format!("{}", variant);
            let parsed = formatted.parse::<SemverType>().unwrap();
            assert_eq!(variant, parsed);
        }
    }
}
