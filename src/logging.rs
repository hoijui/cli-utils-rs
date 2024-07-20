// SPDX-FileCopyrightText: 2021 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides a simple way to initialize logging,
//! and to later change the log level of the application.
//!
//! # Example
//!
//! ```rust
//! use cli_utils_hoijui as cli_utils;
//! use cli_utils::BoxResult;
//! use cli_utils::logging;
//! use log::LevelFilter;
//!
//! fn main() -> BoxResult<()> {
//!     // TODO Replace "my-crate" with e.g. `clap::crate_name!()`
//!     let log_reload_handle = logging::setup("my-crate")?;
//!
//!     // TODO Parse log_level from CLI args
//!     let log_level = LevelFilter::Info;
//!     logging::set_log_level(&log_reload_handle, log_level)?;
//!
//!     // TODO Run the application
//!
//!     return Ok(());
//! }
//! ```

use std::io;

use crate::BoxResult;
use log::LevelFilter as LogLevelFilter;
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

pub fn set_log_level_tracing(reload_handle: &ReloadHandle, level: LevelFilter) -> BoxResult<()> {
    reload_handle.modify(|filter| *filter = level)?;
    Ok(())
}

const fn convert_to_tracing(level: LogLevelFilter) -> LevelFilter {
    match level {
        LogLevelFilter::Off => LevelFilter::OFF,
        LogLevelFilter::Error => LevelFilter::ERROR,
        LogLevelFilter::Warn => LevelFilter::WARN,
        LogLevelFilter::Info => LevelFilter::INFO,
        LogLevelFilter::Debug => LevelFilter::DEBUG,
        LogLevelFilter::Trace => LevelFilter::TRACE,
    }
}

pub fn set_log_level(reload_handle: &ReloadHandle, level: LogLevelFilter) -> BoxResult<()> {
    set_log_level_tracing(reload_handle, convert_to_tracing(level))
}
