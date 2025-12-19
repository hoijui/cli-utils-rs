// SPDX-FileCopyrightText: 2022 - 2025 Robin Vobruba <hoijui.quaero@gmail.com>
// SPDX-FileCopyrightText: 2020 Armin Becher <becherarmin@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::convert::TryFrom;
use std::fmt::Display;

use regex::Regex;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wildmatch::WildMatch;

#[cfg(feature = "file_traversal")]
use crate::file_traversal::PathFilterRet;
#[cfg(all(feature = "async", feature = "serde"))]
use crate::path_buf::PathBuf;
#[cfg(feature = "async")]
use async_std::path::Path;
#[cfg(all(feature = "async", not(feature = "serde")))]
use async_std::path::PathBuf;
#[cfg(not(feature = "async"))]
use std::path::{Path, PathBuf};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ignore path '{0:?}' not found: {1:?}")]
    FailedToCanonicalize(PathBuf, std::io::Error),

    #[error(
        "Ignore path '{0:?}' is neither a dir nor a regular file; \
Do not know how to use i."
    )]
    UnknownPathType(PathBuf),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IgnorePath {
    /// Matches the whole path, so basically a full,
    /// canonical, absolute path to a file
    Whole(PathBuf),
    /// Matches only a prefix of the path.
    Prefix(PathBuf),
    /// Matches paths matching a glob.
    Glob(WildMatch),
    /// Matches [paths matching a regex.
    #[cfg_attr(feature = "serde", serde(with = "serde_regex"))]
    Regex(Regex),
}

impl IgnorePath {
    #[must_use]
    pub fn matches(&self, abs_path: &Path) -> bool {
        match self {
            Self::Whole(path) => <PathBuf as AsRef<Path>>::as_ref(path) == abs_path,
            Self::Prefix(path) => abs_path.starts_with(<PathBuf as AsRef<Path>>::as_ref(path)), //Into::<&Path>::into(path)),
            Self::Glob(glob) => glob.matches(abs_path.to_string_lossy().as_ref()),
            Self::Regex(regex) => regex
                .captures(abs_path.to_string_lossy().as_ref())
                .is_some(),
        }
    }

    #[cfg(feature = "file_traversal")]
    #[must_use]
    pub fn create_filter(
        ignore_paths: Vec<Self>,
    ) -> Box<dyn Fn(&Path) -> PathFilterRet + Send + Sync> {
        Box::new(move |file: &Path| {
            let abs_path = into_absolute(file)?;
            if ignore_paths
                .iter()
                .any(|ignore_path| ignore_path.matches(abs_path.as_ref()))
            {
                #[cfg(feature = "logging")]
                log::debug!(
                    "Ignoring file '{}', because it is in the ignore paths list.",
                    file.display()
                );
                return Ok(false);
            }
            Ok(true)
        })
    }
}

impl Display for IgnorePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Whole(path) | Self::Prefix(path) => path.display().fmt(f),
            Self::Glob(glob) => glob.fmt(f),
            Self::Regex(regex) => regex.fmt(f),
        }
    }
}

impl TryFrom<&Path> for IgnorePath {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let can_path =
            into_absolute(path).map_err(|err| Error::FailedToCanonicalize(path.into(), err))?;
        #[cfg_attr(not(feature = "async"), allow(clippy::useless_conversion))]
        if can_path.is_file() {
            Ok(Self::Whole(can_path.into()))
        } else if can_path.is_dir() {
            Ok(Self::Prefix(can_path.into()))
        } else {
            Err(Error::UnknownPathType(can_path.into()))
        }
    }
}

impl TryFrom<&str> for IgnorePath {
    type Error = Error;

    fn try_from(path_str: &str) -> Result<Self, Self::Error> {
        Self::try_from(Path::new(path_str))
    }
}

/// This does a path canonicalization, which is - very roughly -
/// the same like making the path absolute.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * `path` does not exist.
/// * A non-final component in path is not a directory.
pub fn into_absolute<P: AsRef<Path>>(path: P) -> std::io::Result<std::path::PathBuf> {
    // TODO FIXME NOTE We use `std::fs::canonicalize` here, even though there is `async_std::fs::canonicalize`, because we can not use async in this trait, and using a special async version of this trait would be an anti-pattern:
    // TODO FIXME NOTE <https://users.rust-lang.org/t/is-there-a-way-to-await-inside-a-from-or-tryfrom/68576/5>
    // TODO FIXME NOTE BUT: The anti-pattern is actually, to use such an expensive function in a TryFrom at all!
    std::fs::canonicalize(path.as_ref())
}

/// This does a path canonicalization, which is - very roughly -
/// the same like making the path absolute.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * `path` does not exist.
/// * A non-final component in path is not a directory.
pub fn into_absolute_async<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    // TODO FIXME NOTE We use `std::fs::canonicalize` here, even though there is `async_std::fs::canonicalize`, because we can not use async in this trait, and using a special async version of this trait would be an anti-pattern:
    // TODO FIXME NOTE <https://users.rust-lang.org/t/is-there-a-way-to-await-inside-a-from-or-tryfrom/68576/5>
    // TODO FIXME NOTE BUT: The anti-pattern is actually, to use such an expensive function in a TryFrom at all!
    #[cfg_attr(not(feature = "async"), allow(clippy::useless_conversion))]
    into_absolute(path.as_ref()).map(PathBuf::from)
}

/// Parses the argument into an [`IgnorePath`].
///
/// # Errors
///
/// If the argument is not a valid path glob.
pub fn parse(path_str: &str) -> Result<IgnorePath, String> {
    IgnorePath::try_from(path_str).map_err(|err| format!("{err:?}"))
}

/// Checks if the argument is a valid ignore path (=> path glob).
///
/// # Errors
/// If the argument is not a valid path glob.
// pub fn is_valid(path_str: &str) -> Result<(), String> {
pub fn is_valid<S: AsRef<str>>(path_str: S) -> Result<(), String> {
    parse(path_str.as_ref()).map(|_| ())
}
