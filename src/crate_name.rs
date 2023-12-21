use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    path::{self, Path, PathBuf},
};

use serde::{de::Error, Deserialize, Serialize};

mod forbidden;

use forbidden::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct CrateName(String);

impl CrateName {
    pub fn new(crate_name: &str) -> Result<Self, CrateNameError> {
        let crate_name_lower = crate_name.to_lowercase();

        if !crate_name_lower.is_ascii() {
            return Err(CrateNameError::NotASCII);
        }

        let first_char = crate_name_lower
            .chars()
            .next()
            .ok_or(CrateNameError::Empty)?;

        if !first_char.is_alphabetic() {
            return Err(CrateNameError::NotAlphabeticFirstChar);
        }

        if crate_name_lower.len() > MAX_CRATE_NAME_LENGTH {
            return Err(CrateNameError::TooLong);
        }

        if !crate_name_lower
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(CrateNameError::ForbiddenChar);
        }

        if FORBIDDEN_CRATE_NAMES.contains(&crate_name_lower.as_str()) {
            return Err(CrateNameError::Forbidden);
        }

        Ok(Self(crate_name_lower))
    }

    pub fn from_index_path(path: &Path) -> Result<Self, IndexPathError> {
        let mut components_iter = path.components();
        let components: Vec<_> = components_iter
            .by_ref()
            .skip_while(|c| c == &path::Component::RootDir)
            .take(3)
            .map(|c| match c {
                path::Component::Prefix(_)
                | path::Component::RootDir
                | path::Component::CurDir
                | path::Component::ParentDir => Err(IndexPathError::ForbiddenComponent),
                path::Component::Normal(c) => {
                    let c = c.to_str().ok_or(IndexPathError::ForbiddenComponent)?;

                    if !c.is_ascii() {
                        return Err(IndexPathError::ForbiddenComponent);
                    }

                    Ok(c)
                }
            })
            .collect::<Result<_, _>>()?;

        if components_iter.next().is_some() {
            return Err(IndexPathError::TooManyComponents);
        }

        let crate_name = match components.len() {
            0 | 1 => return Err(IndexPathError::TooFewComponents),
            2 => match components[0] {
                "1" => {
                    if components[1].len() != 1 {
                        return Err(IndexPathError::Inconsistent);
                    }

                    components[1]
                }

                "2" => {
                    if components[1].len() != 2 {
                        return Err(IndexPathError::Inconsistent);
                    }

                    components[1]
                }

                _ => return Err(IndexPathError::Inconsistent),
            },

            3 => {
                if components[0] == "3" {
                    if components[2].len() != 3 || components[1] != &components[2][0..1] {
                        return Err(IndexPathError::Inconsistent);
                    }

                    components[2]
                } else {
                    if components[2].len() < 4
                        || components[0] != &components[2][0..2]
                        || components[1] != &components[2][2..4]
                    {
                        return Err(IndexPathError::Inconsistent);
                    }

                    components[2]
                }
            }

            _ => unreachable!(),
        };

        Ok(Self::new(crate_name)?)
    }

    pub fn index_path(&self) -> PathBuf {
        match self.0.len() {
            0 => unreachable!(),
            1 => PathBuf::from("1").join(&self.0),
            2 => PathBuf::from("2").join(&self.0),
            3 => PathBuf::from("3").join(&self.0[0..1]).join(&self.0),
            _ => PathBuf::from(&self.0[0..2])
                .join(&self.0[2..4])
                .join(&self.0),
        }
    }

    pub fn crate_path(&self, version: semver::Version) -> PathBuf {
        PathBuf::from(&self.0)
            .join(version.to_string())
            .join(format!("{}.crate", &self.0))
    }
}

impl Display for CrateName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl<'de> Deserialize<'de> for CrateName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: Cow<'de, str> = Deserialize::deserialize(deserializer)?;
        Self::new(&s).map_err(D::Error::custom)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexPathError {
    #[error("Index path is inconsistent")]
    Inconsistent,
    #[error("Index path has a forbidden component")]
    ForbiddenComponent,
    #[error("Index path has too few components")]
    TooFewComponents,
    #[error("Index path has too many components")]
    TooManyComponents,

    #[error("Index path is for an invalid crate name: {_0}")]
    CrateName(#[from] CrateNameError),
}

const MAX_CRATE_NAME_LENGTH: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum CrateNameError {
    #[error("Crate names cannot be empty")]
    Empty,

    #[error("Crate names must be ASCII")]
    NotASCII,

    #[error("Crate names must be composed of alphanumeric characters, plus - and _")]
    ForbiddenChar,

    #[error("This crate name is forbidden")]
    Forbidden,

    #[error("The first character of crate names must be alphabetic")]
    NotAlphabeticFirstChar,

    #[error(
        "Crate names must be at most {} characters in length",
        MAX_CRATE_NAME_LENGTH
    )]
    TooLong,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn crate_name_new_doesnt_panic(s in "\\PC*") {
            let _ = CrateName::new(&s);
        }

        #[test]
        fn crate_name_from_index_path_doesnt_panic(s in "\\PC*") {
            let _ = CrateName::from_index_path(Path::new(&s));
        }

        #[test]
        fn crate_name_is_consistent(s in "\\PC*") {
            if let Ok(name) = CrateName::new(&s) {
                assert_eq!(&name.0, &CrateName::from_index_path(&name.index_path()).unwrap().0);
            }
        }

        #[test]
        fn crate_name_index_path_is_consistent(s in "\\PC*") {
            if let Ok(name) = CrateName::from_index_path(Path::new(&s)) {
                assert_eq!(Path::new(&s.to_lowercase()), &name.index_path());
                name.index_path().to_str().unwrap();
            }
        }
    }
}
