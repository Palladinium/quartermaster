use std::fmt::{self, Display, Formatter};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FeatureName(String);

impl FeatureName {
    pub fn new(feature_name: String) -> Result<Self, FeatureNameError> {
        if feature_name.is_empty() {
            return Err(FeatureNameError::Empty);
        }

        if !feature_name.is_ascii() {
            return Err(FeatureNameError::NotASCII);
        }

        if !feature_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(FeatureNameError::ForbiddenChar);
        }

        Ok(Self(feature_name))
    }
}

impl Display for FeatureName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl Serialize for FeatureName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FeatureName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Self::new(s).map_err(D::Error::custom)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FeatureNameError {
    #[error("Feature names cannot be empty")]
    Empty,

    #[error("Feature names must be ASCII")]
    NotASCII,

    #[error("Feature names must be composed of alphanumeric characters, plus - and _")]
    ForbiddenChar,
}
