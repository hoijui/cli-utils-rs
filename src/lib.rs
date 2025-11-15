// SPDX-FileCopyrightText: 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "logging")]
pub mod logging;
#[cfg(feature = "std_error")]
pub mod std_error;
mod std_streams;

pub use std_streams::*;

/// This serves as a general purpose, catch-all error type.
///
/// It is widely compatible, owned,
/// other errors can easily be converted to it,
/// and it depends only on `std`.
/// NOTE Try to avoid using this as much as possible,
///      and rather use more specific error types.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
/// This serves as a general purpose, catch-all result type.
/// See [`BoxError`] for more.
pub type BoxResult<T> = Result<T, BoxError>;

// This tests rust code in the README with doc-tests.
// Though, It will not appear in the generated documentation.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
