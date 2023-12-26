use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    str::{self, FromStr},
};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use tracing::warn;
use url::Url;

use crate::{crate_name::CrateName, feature_name::FeatureName};

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndexConfig {
    pub dl: Url,
    pub api: String,
    pub auth_required: bool,
}

#[derive(Default)]
pub struct IndexFile {
    pub entries: Vec<IndexEntry>,
}

impl IndexFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IndexFileError> {
        Ok(Self {
            entries: str::from_utf8(bytes)?
                .lines()
                .map(|l| serde_json::from_str(l.trim_end()))
                .collect::<Result<_, serde_json::Error>>()?,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, IndexFileError> {
        let mut bytes = Vec::new();

        if let Some((first, rest)) = self.entries.split_first() {
            serde_json::to_writer(&mut bytes, first)?;

            for entry in rest {
                bytes.push(b'\n');
                serde_json::to_writer(&mut bytes, entry)?;
            }
        } else {
            warn!("Serializing empty index file");
        }

        Ok(bytes)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexFileError {
    #[error("Invalid UTF-8")]
    Utf8(#[from] str::Utf8Error),
    #[error("JSON error")]
    Json(#[from] serde_json::Error),
}

/// Adapted from https://doc.rust-lang.org/cargo/reference/registry-index.html
///
/// The `v` and `features2` fields are absent, since we always assume `v` is 2.
#[derive(Serialize, Deserialize)]
pub struct IndexEntry {
    /// The name of the package.
    /// This must only contain alphanumeric, `-`, or `_` characters.
    pub name: CrateName,
    /// The version of the package this row is describing.
    /// This must be a valid version number according to the Semantic
    /// Versioning 2.0.0 spec at https://semver.org/.
    pub vers: semver::Version,
    /// Array of direct dependencies of the package.
    pub deps: Vec<IndexDependency>,
    /// A SHA256 checksum of the `.crate` file.
    pub cksum: String,
    /// Set of features defined for the package.
    /// Each feature maps to an array of features or dependencies it enables.
    pub features: BTreeMap<FeatureName, Vec<String>>,
    /// Boolean of whether or not this version has been yanked.
    pub yanked: bool,
    /// The `links` string value from the package's manifest, or null if not
    /// specified. This field is optional and defaults to null.
    #[serde(default)]
    pub links: Option<String>,
    /// The minimal supported Rust version (optional)
    /// This must be a valid version requirement without an operator (e.g. no `=`)
    pub rust_version: Option<MinRustVersion>,
}

#[derive(Serialize, Deserialize)]
pub struct IndexDependency {
    /// Name of the dependency.
    /// If the dependency is renamed from the original package name,
    /// this is the new name. The original package name is stored in
    /// the `package` field.
    pub name: String,
    /// The SemVer requirement for this dependency.
    /// This must be a valid version requirement defined at
    /// https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html.
    pub req: semver::VersionReq,
    /// Array of features (as strings) enabled for this dependency.
    pub features: Vec<String>,
    /// Boolean of whether or not this is an optional dependency.
    pub optional: bool,
    /// Boolean of whether or not default features are enabled.
    pub default_features: bool,
    /// The target platform for the dependency.
    /// null if not a target dependency.
    /// Otherwise, a string such as "cfg(windows)".
    #[serde(default)]
    pub target: Option<String>,
    /// The dependency kind.
    /// "dev", "build", or "normal".
    /// Note: this is a required field, but a small number of entries
    /// exist in the crates.io index with either a missing or null
    /// `kind` field due to implementation bugs.
    pub kind: DependencyKind,
    /// The URL of the index of the registry where this dependency is
    /// from as a string. If not specified or null, it is assumed the
    /// dependency is in the current registry.
    #[serde(default)]
    pub registry: Option<Url>,
    /// If the dependency is renamed, this is a string of the actual
    /// package name. If not specified or null, this dependency is not
    /// renamed.
    #[serde(default)]
    pub package: Option<String>,
}

/// Modified semver::Comparator without the `op`
pub struct MinRustVersion {
    pub major: u64,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    pub pre: semver::Prerelease,
}

impl FromStr for MinRustVersion {
    type Err = MinRustVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // HACK: Bit janky, this should probably be implemented in semver without attempting to parse the `op`

        // This is to reject strings starting with "^", which are undistinguishable from those with no op after semver parses them
        if s.contains('^') {
            return Err(MinRustVersionError::Op);
        }

        let comparator = semver::Comparator::from_str(s)?;

        if comparator.op != semver::Op::Caret {
            return Err(MinRustVersionError::Op);
        }

        Ok(Self {
            major: comparator.major,
            minor: comparator.minor,
            patch: comparator.patch,
            pre: comparator.pre,
        })
    }
}

impl Display for MinRustVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.major)?;

        if let Some(minor) = &self.minor {
            write!(f, ".{}", minor)?;
            if let Some(patch) = &self.patch {
                write!(f, ".{}", patch)?;
                if !self.pre.is_empty() {
                    write!(f, "-{}", self.pre)?;
                }
            }
        }

        Ok(())
    }
}

impl Serialize for MinRustVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MinRustVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Cow<'de, str> = Deserialize::deserialize(deserializer)?;
        Self::from_str(&s).map_err(D::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Dev,
    Build,
    Normal,
}

#[derive(Debug, thiserror::Error)]
pub enum MinRustVersionError {
    #[error("Comparison operator not allowed")]
    Op,
    #[error(transparent)]
    Semver(#[from] semver::Error),
}
