// SPDX-FileCopyrightText: 2021 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::BoxError;
use thiserror::Error;

/// This serves to wrap/represent `std::**()` and `Option` return values as `Result`s,
/// like the one of [`std::fs::PathBuf::file_name()`], or [`std::OsStr::to_str()`].
#[derive(Error, Debug)]
pub enum Error {
    #[error("Represents a `Option::None` value as an error.")]
    None,

    /// A required properties value could not be evaluated
    #[error(r#"The file name ends in "..", and does therefore not represent a file/directory/valid path."#)]
    PathNotAFile,

    #[error(
        "The string is not valid UTF-8, and can thus not be represented by a normal rust string."
    )]
    NotValidUtf8,

    #[cfg(feature = "url_parse_error")]
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    /// Represents all cases of `std::io::Error`.
    #[error(transparent)]
    IO(#[from] std::io::Error),

    /// Represents all other cases of `std::error::Error`.
    #[error(transparent)]
    Boxed(#[from] BoxError),
}
