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
    fn from_str_parses_and_rejects() {
        assert_eq!("Major".parse::<SemverType>().unwrap(), SemverType::Major);
        assert_eq!("Minor".parse::<SemverType>().unwrap(), SemverType::Minor);
        assert_eq!("Patch".parse::<SemverType>().unwrap(), SemverType::Patch);
        assert_eq!(
            "Invalid".parse::<SemverType>().unwrap_err(),
            "Unknown SemverType"
        );
        assert!("major".parse::<SemverType>().is_err());
    }

    #[test]
    fn parse_and_format_roundtrip() {
        for variant in [SemverType::Major, SemverType::Minor, SemverType::Patch] {
            let formatted = format!("{variant}");
            assert_eq!(formatted.parse::<SemverType>().unwrap(), variant);
        }
    }
}
