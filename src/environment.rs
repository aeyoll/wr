use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Environment {
    Production,
    Staging,
}

impl Default for Environment {
    fn default() -> Environment {
        Environment::Production
    }
}

impl FromStr for Environment {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Production" => Ok(Environment::Production),
            "Staging" => Ok(Environment::Staging),
            _ => Err("Unknown environment"),
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
