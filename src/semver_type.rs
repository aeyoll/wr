use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, clap::ValueEnum, Default)]
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
        write!(f, "{:?}", self)
    }
}
