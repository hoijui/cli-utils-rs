// SPDX-FileCopyrightText: 2021 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides a simple way to initialize logging,
//! and to later change the log level of the application.
//!
//! # Example
//!
//! ```rust
//! use cli_utils::BoxResult;
//! use cli_utils::logging;
//!
//! #[tokio::main]
//! async fn main() -> BoxResult<()> {
//!     let log_reload_handle = logging::setup(clap::crate_name!())?;
//!
//!     let cli_args = cli::parse()?;
//!
//!     if cli_args.verbose {
//!         logging::set_log_level(&log_reload_handle, LevelFilter::DEBUG)?;
//!     } else if cli_args.quiet {
//!         logging::set_log_level(&log_reload_handle, LevelFilter::WARN)?;
//!     } else {
//!         logging::set_log_level(&log_reload_handle, LevelFilter::INFO)?;
//!     }
//!     crate::process(&cli_args.conf).await?;
//!
//!     return Ok(());
//! }
//! ```

use std::io;

use crate::BoxResult;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    fmt,
    layer::Layered,
    prelude::*,
    reload::{self, Handle},
    util::TryInitError,
    EnvFilter, Registry,
};

type ReloadHandle = Handle<LevelFilter, Layered<EnvFilter, Registry, Registry>>;

/// Sets up logging, with a way to change the log level later on,
/// and with all output going to stderr,
/// as suggested by <https://clig.dev/>.
///
/// # Errors
///
/// If initializing the registry (logger) failed.
pub fn setup(crate_name: &str) -> Result<ReloadHandle, TryInitError> {
    // NOTE It is crucial to first set the lowest log level,
    //      as apparently, any level that is lower then this one
    //      will be ignored when trying to set it later on.
    //      Later though, the level can be changed up and down as desired.
    let level_filter = LevelFilter::TRACE;
    let (filter, reload_handle_filter) = reload::Layer::new(level_filter);

    let l_stderr = fmt::layer().map_writer(move |_| io::stderr);

    let registry = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{crate_name}=debug,tower_http=debug").into()),
        )
        .with(filter)
        .with(l_stderr);
    registry.try_init()?;

    Ok(reload_handle_filter)
}

pub fn set_log_level(reload_handle: &ReloadHandle, level: LevelFilter) -> BoxResult<()> {
    reload_handle.modify(|filter| *filter = level)?;
    Ok(())
}
