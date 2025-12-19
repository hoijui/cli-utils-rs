// SPDX-FileCopyrightText: 2022 - 2025 Robin Vobruba <hoijui.quaero@gmail.com>
// SPDX-FileCopyrightText: 2020 Armin Becher <becherarmin@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "serde")]
use crate::path_buf::PathBuf;
#[cfg(not(feature = "serde"))]
use async_std::path::PathBuf;
use {async_std::path::Path, async_walkdir::WalkDir, futures::StreamExt};

pub type PathFilterRet = Result<bool, std::io::Error>;
pub type PathFilter = dyn Fn(&Path) -> PathFilterRet + Send + Sync;

pub fn create_combined_filter(
    filters: Vec<Box<impl Fn(&Path) -> PathFilterRet + Send + Sync>>,
) -> impl Fn(&Path) -> PathFilterRet + Send + Sync {
    move |file: &Path| {
        for filter in &filters {
            if !filter(file)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Supplied Input file is missing file name: '{0:#?}'")]
    MissingFileName(PathBuf),

    #[error("Failed to canonicalize (~- make absolute) path '{0:?}' with error: {1:?}")]
    FailedToMakeAbsolute(PathBuf, std::io::Error),

    #[error("I/O Error: '{0:#?}'")]
    IO(#[from] std::io::Error),
}

/// Searches for markup source files according to the configuration,
/// and stores them in `result`.
///
/// # Errors
///
/// If any of the (markup) files supplied or found through scanning supplied dirs
/// has no name (e.g. '.').
/// The code-logic should prevent this from ever happening.
pub async fn scan<F: Fn(&Path) -> PathFilterRet + Send + Sync, C: AsyncFnMut(PathBuf)>(
    root: &Path,
    filter: &F,
    collector: &mut C,
) -> Result<(), Error> {
    #[cfg(feature = "logging")]
    log::debug!("Searching for files in directory '{root:?}' ...");

    let mut dir_walker = WalkDir::new(root);
    loop {
        match dir_walker.next().await {
            Some(Ok(entry)) => {
                if let Ok(file_type) = entry.file_type().await
                    && !file_type.is_dir()
                {
                    add(filter, entry.path().as_ref(), collector).await?;
                }
            }
            Some(Err(_err)) => (),
            None => break,
        }
    }

    Ok(())
}

/// Stores a single file in `result`,
/// if it is accessible
/// and a markup source file according to the configuration.
///
/// # Errors
///
/// If the supplied `file` has no name (e.g. '.').
/// The code-logic should prevent this from ever being supplied.
pub async fn add<F: Fn(&Path) -> PathFilterRet + Send + Sync, C: AsyncFnMut(PathBuf)>(
    filter: &F,
    file: &Path,
    collector: &mut C,
) -> Result<(), Error> {
    if !filter(file)? {
        return Ok(());
    }
    #[cfg(feature = "logging")]
    log::debug!("Found file: '{file:?}'");
    collector(file.into()).await;

    Ok(())
}

/// Searches for markup source files according to the configuration,
/// and stores them in `result`.
///
/// # Errors
///
/// If a file or path supplied does not exist,
/// or if any file supplied or found through scanning has no name (e.g. '.').
/// The code-logic should prevent the second case from ever happening.
pub async fn find<F: Fn(&Path) -> PathFilterRet + Send + Sync>(
    root: &Path,
    filter: &F,
) -> Result<Vec<PathBuf>, Error> {
    let mut result = vec![];
    let mut collector = async |file: PathBuf| result.push(file);
    scan(root, &filter, &mut collector).await?;
    Ok(result)
}
