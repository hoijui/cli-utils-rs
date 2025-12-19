// SPDX-FileCopyrightText: 2022 - 2025 Robin Vobruba <hoijui.quaero@gmail.com>
// SPDX-FileCopyrightText: 2020 Armin Becher <becherarmin@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_std::path::Iter;
use async_std::path::Path as AsyncPath;
use async_std::path::PathBuf as AsyncPathBuf;
use std::path::Path as StdPath;
use std::path::PathBuf as StdPathBuf;

use std::{ffi::OsStr, fmt::Display, path::StripPrefixError, str::FromStr};
use {
    serde::{
        de::{Deserialize, Deserializer, Unexpected, Visitor},
        ser::{Serialize, Serializer},
    },
    std::{fmt, str},
};

/// This wrapper exists so we can implement serde on it.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathBuf(AsyncPathBuf);

impl Default for PathBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl PathBuf {
    #[must_use]
    pub fn new() -> Self {
        Self(AsyncPathBuf::new())
    }

    #[must_use]
    pub fn file_name(&self) -> Option<&OsStr> {
        self.0.file_name()
    }

    #[must_use]
    pub fn display(&self) -> std::path::Display<'_> {
        self.0.display()
    }

    #[must_use]
    pub fn is_relative(&self) -> bool {
        self.0.is_relative()
    }

    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.0.is_absolute()
    }

    pub async fn is_file(&self) -> bool {
        self.0.is_file().await
    }

    pub async fn is_dir(&self) -> bool {
        self.0.is_dir().await
    }

    pub async fn exists(&self) -> bool {
        self.0.exists().await
    }

    #[must_use]
    pub fn as_os_str(&self) -> &OsStr {
        self.0.as_os_str()
    }

    #[must_use]
    pub fn extension(&self) -> Option<&OsStr> {
        self.0.extension()
    }

    #[must_use]
    pub fn parent(&self) -> Option<&AsyncPath> {
        self.0.parent()
    }

    #[must_use]
    pub fn as_path(&self) -> &AsyncPath {
        self.0.as_path()
    }

    #[must_use]
    pub fn join<P: AsRef<AsyncPath>>(&self, path: P) -> Self {
        Self(self.0.join(path))
    }

    /// Returns a path that becomes `self` when joined onto `base`.
    ///
    /// # Errors
    ///
    /// If `base` is not a prefix of `self` (i.e., [`starts_with`]
    /// returns `false`), returns [`Err`].
    ///
    /// [`starts_with`]: #method.starts_with
    /// [`Err`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Err
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::{Path, PathBuf};
    ///
    /// let path = Path::new("/test/haha/foo.txt");
    ///
    /// assert_eq!(path.strip_prefix("/"), Ok(Path::new("test/haha/foo.txt")));
    /// assert_eq!(path.strip_prefix("/test"), Ok(Path::new("haha/foo.txt")));
    /// assert_eq!(path.strip_prefix("/test/"), Ok(Path::new("haha/foo.txt")));
    /// assert_eq!(path.strip_prefix("/test/haha/foo.txt"), Ok(Path::new("")));
    /// assert_eq!(path.strip_prefix("/test/haha/foo.txt/"), Ok(Path::new("")));
    /// assert_eq!(path.strip_prefix("test").is_ok(), false);
    /// assert_eq!(path.strip_prefix("/haha").is_ok(), false);
    ///
    /// let prefix = PathBuf::from("/test/");
    /// assert_eq!(path.strip_prefix(prefix), Ok(Path::new("haha/foo.txt")));
    /// ```
    pub fn strip_prefix<P>(&self, base: P) -> Result<&AsyncPath, StripPrefixError>
    where
        P: AsRef<AsyncPath>,
    {
        self.0.strip_prefix(base)
    }

    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        self.0.iter()
    }
}

impl Display for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(f)
    }
}

impl<'a> IntoIterator for &'a PathBuf {
    type Item = &'a std::ffi::OsStr;
    type IntoIter = async_std::path::Iter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl AsRef<StdPath> for PathBuf {
    fn as_ref(&self) -> &StdPath {
        self.0.as_ref()
    }
}

impl AsRef<AsyncPath> for PathBuf {
    fn as_ref(&self) -> &AsyncPath {
        self.0.as_ref()
    }
}

impl FromStr for PathBuf {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AsyncPathBuf::from_str(s).map(PathBuf)
    }
}

impl From<&AsyncPath> for PathBuf {
    fn from(path: &AsyncPath) -> Self {
        Self(AsyncPathBuf::from(path))
    }
}

impl From<&StdPath> for PathBuf {
    fn from(path: &StdPath) -> Self {
        Self(AsyncPathBuf::from(path))
    }
}

impl From<&StdPathBuf> for PathBuf {
    fn from(path: &StdPathBuf) -> Self {
        Self(AsyncPathBuf::from(path))
    }
}

impl From<StdPathBuf> for PathBuf {
    fn from(path: StdPathBuf) -> Self {
        Self(AsyncPathBuf::from(path))
    }
}

impl From<AsyncPathBuf> for PathBuf {
    fn from(path: AsyncPathBuf) -> Self {
        Self(path)
    }
}

impl From<&str> for PathBuf {
    fn from(path: &str) -> Self {
        Self(AsyncPathBuf::from(path))
    }
}

impl From<&PathBuf> for AsyncPathBuf {
    fn from(path: &PathBuf) -> Self {
        Self::from(<PathBuf as AsRef<AsyncPath>>::as_ref(path))
    }
}

impl From<PathBuf> for AsyncPathBuf {
    fn from(path: PathBuf) -> Self {
        Self::from(<PathBuf as AsRef<AsyncPath>>::as_ref(&path))
    }
}

impl From<&PathBuf> for StdPathBuf {
    fn from(path: &PathBuf) -> Self {
        Self::from(<PathBuf as AsRef<StdPath>>::as_ref(path))
    }
}

impl From<PathBuf> for StdPathBuf {
    fn from(path: PathBuf) -> Self {
        Self::from(<PathBuf as AsRef<StdPath>>::as_ref(&path))
    }
}

struct PathBufVisitor;

// #[cfg(feature = "serde")]
impl Visitor<'_> for PathBufVisitor {
    type Value = PathBuf;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("path string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(From::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        str::from_utf8(v)
            .map(From::from)
            .map_err(|_| serde::de::Error::invalid_value(Unexpected::Bytes(v), &self))
    }
}

impl Serialize for PathBuf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_path().as_os_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PathBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(PathBufVisitor)
    }
}
