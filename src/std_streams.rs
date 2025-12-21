// SPDX-FileCopyrightText: 2023 - 2025 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::LazyLock;

#[cfg(all(feature = "async", feature = "serde"))]
use crate::path_buf::PathBuf;
#[cfg(all(feature = "async", not(feature = "serde")))]
use async_std::path::PathBuf;
#[cfg(not(feature = "async"))]
use std::path::PathBuf;
#[cfg(feature = "async")]
use {
    async_std::fs::File,
    async_std::io::BufReadExt,
    async_std::io::{self, BufRead, BufReader, Write, WriteExt},
    async_std::path::Path,
    async_std::stream::Stream,
};
#[cfg(not(feature = "async"))]
use {
    std::fs::File,
    std::io::{self, BufRead, BufReader, Write},
    std::path::Path,
};

pub const STREAM_PATH_STR: &str = "-";
pub static STREAM_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from_str(STREAM_PATH_STR)
        .expect("Failed to create path from \"-\"; that should be impossible")
});

/// Denotes/identifies/specifies a stream,
/// either stdin, stdout, or a file-path.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StreamIdent {
    /// Denotes the standard input-stream.
    StdIn,
    /// Denotes the standard output-stream.
    StdOut,
    /// Denotes a file-path and whether it is an input stream.
    ///
    /// Note that the path "-" here would not have a special meaning,
    /// it would simply denote a file with that name.
    Path(PathBuf, bool),
}

impl StreamIdent {
    #[must_use]
    pub const fn new_std(r#in: bool) -> Self {
        if r#in { Self::StdIn } else { Self::StdOut }
    }

    #[must_use]
    pub fn from_path_opt<P: AsRef<Path> + ?Sized + Unpin + Send + Sync>(
        ident: Option<&P>,
        r#in: bool,
    ) -> Self {
        Self::from((ident, r#in))
    }

    #[must_use]
    pub fn from_path_buf_opt(ident: Option<PathBuf>, r#in: bool) -> Self {
        Self::from((ident, r#in))
    }

    pub fn from_path<P: AsRef<Path> + ?Sized + Unpin + Send + Sync>(ident: &P, r#in: bool) -> Self {
        Self::from((ident, r#in))
    }

    #[must_use]
    pub fn from_path_buf(ident: PathBuf, r#in: bool) -> Self {
        Self::from((ident, r#in))
    }

    /// Returns a human oriented description of the identified stream.
    ///
    /// This might be useful for logging.
    ///
    /// This returns:
    ///
    /// - "file-system stream (in|out): '<FILE-NAME>'" if self identifies a path
    /// - otherwise:
    ///   - "stdin" if `self.in` is `true`
    ///   - "stdout" if it is `false`
    #[must_use]
    pub fn description(&self) -> Cow<'static, str> {
        match self {
            Self::StdIn => Cow::Borrowed("stdin"),
            Self::StdOut => Cow::Borrowed("stdout"),
            Self::Path(path, r#in) => Cow::Owned(format!(
                "file-system stream ({}): '{}'",
                if *r#in { "in" } else { "out" },
                path.display()
            )),
        }
    }

    /// Creates a reader from a string identifier.
    /// Both `None` and `Some("-")` mean stdin.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::io;
    /// # use async_std::path::Path;
    /// # use async_std::path::PathBuf;
    /// # use std::str::FromStr;
    /// use cli_utils_hoijui::StreamIdent;
    /// #[cfg(feature = "async")]
    /// use async_std::io::BufReadExt;
    ///
    /// # #[cfg(feature = "async")]
    /// # async fn create_input_reader_example() -> io::Result<()> {
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(None, true); // reads from stdin
    /// let mut reader = in_stream_ident.create_input_reader().await?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_opt::<Path>(None, true); // reads from stdin
    /// let mut reader = in_stream_ident.create_input_reader().await?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(Some("-".into()), true); // reads from stdin
    /// let mut reader = in_stream_ident.create_input_reader().await?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(Some("my_dir/my_file.txt".into()), true); // reads from file "$CWD/my_dir/my_file.txt"
    /// let mut reader = in_stream_ident.create_input_reader().await?;
    ///
    /// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossible!");
    /// let in_stream_ident = StreamIdent::from_path_opt(Some(path_buf.as_path()), true); // reads from file "$CWD/my_dir/my_file.txt"
    /// let mut reader = in_stream_ident.create_input_reader().await?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(Some(path_buf.into()), true); // reads from file "$CWD/my_dir/my_file.txt"
    /// let mut reader = in_stream_ident.create_input_reader().await?;
    ///
    /// let mut buffer = String::new();
    /// loop {
    ///     let line_size = reader.read_line(&mut buffer).await?;
    ///     if line_size == 0 {
    ///         break;
    ///     }
    ///     print!("{}", buffer);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// - if a file path is specified, and it is not possible to read from it
    /// - if this method is called on an output stream specifier
    #[cfg(feature = "async")]
    pub async fn create_input_reader(&self) -> io::Result<Box<dyn BufRead + Unpin>> {
        match self {
            Self::StdIn => Ok(Self::create_input_reader_stdin()),
            Self::Path(path, true) => Self::create_input_reader_file(path).await,
            Self::StdOut | Self::Path(_, false) => Err(io::Error::other(
                "Can not create an input reader from an output stream identifier!",
            )),
        }
    }

    /// Creates a reader from a string identifier.
    /// Both `None` and `Some("-")` mean stdin.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::io;
    /// # use std::path::Path;
    /// # use std::path::PathBuf;
    /// # use std::str::FromStr;
    /// use cli_utils_hoijui::StreamIdent;
    ///
    /// # #[cfg(not(feature = "async"))]
    /// # fn create_input_reader_example() -> io::Result<()> {
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(None, true); // reads from stdin
    /// let mut reader = in_stream_ident.create_input_reader()?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_opt::<Path>(None, true); // reads from stdin
    /// let mut reader = in_stream_ident.create_input_reader()?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(Some("-".into()), true); // reads from stdin
    /// let mut reader = in_stream_ident.create_input_reader()?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(Some("my_dir/my_file.txt".into()), true); // reads from file "$CWD/my_dir/my_file.txt"
    /// let mut reader = in_stream_ident.create_input_reader()?;
    ///
    /// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossible!");
    /// let in_stream_ident = StreamIdent::from_path_opt(Some(path_buf.as_path()), true); // reads from file "$CWD/my_dir/my_file.txt"
    /// let mut reader = in_stream_ident.create_input_reader()?;
    ///
    /// let in_stream_ident = StreamIdent::from_path_buf_opt(Some(path_buf), true); // reads from file "$CWD/my_dir/my_file.txt"
    /// let mut reader = in_stream_ident.create_input_reader()?;
    ///
    /// let mut buffer = String::new();
    /// loop {
    ///     let line_size = reader.read_line(&mut buffer)?;
    ///     if line_size == 0 {
    ///         break;
    ///     }
    ///     print!("{}", buffer);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// - if a file path is specified, and it is not possible to read from it
    /// - if this method is called on an output stream specifier
    #[cfg(not(feature = "async"))]
    pub fn create_input_reader(&self) -> io::Result<Box<dyn BufRead>> {
        match self {
            Self::StdIn => Ok(Self::create_input_reader_stdin()),
            Self::Path(path, true) => Self::create_input_reader_file(path),
            Self::StdOut | Self::Path(_, false) => Err(io::Error::other(
                "Can not create an input reader from an output stream identifier!",
            )),
        }
    }

    /// Creates a reader from a file-path.
    /// See [`create_input_reader`].
    ///
    /// # Errors
    ///
    /// If a file path is specified, and it is not possible to read from it.
    #[cfg(feature = "async")]
    pub async fn create_input_reader_file<P: AsRef<Path> + ?Sized + Send + Sync>(
        file_path: &P,
    ) -> io::Result<Box<dyn BufRead + Unpin>> {
        let file = File::open(file_path).await?;
        Ok(Box::new(BufReader::new(file)))
    }

    /// Creates a reader from a file-path.
    /// See [`create_input_reader`].
    ///
    /// # Errors
    ///
    /// If a file path is specified, and it is not possible to read from it.
    #[cfg(not(feature = "async"))]
    pub fn create_input_reader_file<P: AsRef<Path> + ?Sized + Send + Sync>(
        file_path: &P,
    ) -> io::Result<Box<dyn BufRead>> {
        let file = File::open(file_path)?;
        Ok(Box::new(BufReader::new(file)))
    }

    /// Creates a reader that reads from stdin.
    /// See [`create_input_reader`].
    #[must_use]
    #[cfg(feature = "async")]
    pub fn create_input_reader_stdin() -> Box<dyn BufRead + Unpin> {
        Box::new(BufReader::new(io::stdin()))
    }

    /// Creates a reader that reads from stdin.
    /// See [`create_input_reader`].
    #[must_use]
    #[cfg(not(feature = "async"))]
    pub fn create_input_reader_stdin() -> Box<dyn BufRead> {
        Box::new(BufReader::new(io::stdin()))
    }

    /// Creates a writer from a string identifier.
    /// Both `None` and `Some("-")` mean stdout.
    /// See also: [`write_to_file`]
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::io;
    /// # use async_std::path::Path;
    /// # use async_std::path::PathBuf;
    /// # use std::str::FromStr;
    /// use cli_utils_hoijui::StreamIdent;
    /// # #[cfg(feature = "async")]
    /// use async_std::io::WriteExt;
    ///
    /// # #[cfg(feature = "async")]
    /// # async fn create_output_writer_example() -> io::Result<()> {
    /// let lines = vec!["line 1", "line 2", "line 3"];
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(None, false); // writes to stdout
    /// let mut writer = out_stream_ident.create_output_writer().await?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_opt::<Path>(None, false); // writes to stdout
    /// let mut writer = out_stream_ident.create_output_writer().await?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(Some("-".into()), false); // writes to stdout
    /// let mut writer = out_stream_ident.create_output_writer().await?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(Some("my_dir/my_file.txt".into()), false); // writes to file "$CWD/my_dir/my_file.txt"
    /// let mut writer = out_stream_ident.create_output_writer().await?;
    ///
    /// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossible!");
    /// let out_stream_ident = StreamIdent::from_path_opt(Some(path_buf.as_path()), false); // writes to file "$CWD/my_dir/my_file.txt"
    /// let mut writer = out_stream_ident.create_output_writer().await?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(Some(path_buf.into()), false); // writes to file "$CWD/my_dir/my_file.txt"
    /// let mut writer = out_stream_ident.create_output_writer().await?;
    ///
    /// for line in lines {
    ///     writer.write_all(line.as_bytes()).await?;
    ///     writer.write_all(b"\n").await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// - if a file path is specified, and it is not possible to write to it
    /// - if this method is called on an input stream specifier
    #[cfg(feature = "async")]
    pub async fn create_output_writer(&self) -> io::Result<Box<dyn Write + Unpin + Send + Sync>> {
        match self {
            Self::StdOut => Ok(Self::create_output_writer_stdout()),
            Self::Path(path, false) => Self::create_output_writer_file(path).await,
            Self::StdIn | Self::Path(_, true) => Err(io::Error::other(
                "Can not create an output writer from an input stream identifier!",
            )),
        }
    }

    /// Creates a writer from a string identifier.
    /// Both `None` and `Some("-")` mean stdout.
    /// See also: [`write_to_file`]
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::io;
    /// # use std::path::Path;
    /// # use std::path::PathBuf;
    /// # use std::str::FromStr;
    /// use cli_utils_hoijui::StreamIdent;
    /// # #[cfg(feature = "async")]
    /// use async_std::io::WriteExt;
    ///
    /// # #[cfg(not(feature = "async"))]
    /// # fn create_output_writer_example() -> io::Result<()> {
    /// let lines = vec!["line 1", "line 2", "line 3"];
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(None, false); // writes to stdout
    /// let mut writer = out_stream_ident.create_output_writer()?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_opt::<Path>(None, false); // writes to stdout
    /// let mut writer = out_stream_ident.create_output_writer()?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(Some("-".into()), false); // writes to stdout
    /// let mut writer = out_stream_ident.create_output_writer()?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(Some("my_dir/my_file.txt".into()), false); // writes to file "$CWD/my_dir/my_file.txt"
    /// let mut writer = out_stream_ident.create_output_writer()?;
    ///
    /// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossible!");
    /// let out_stream_ident = StreamIdent::from_path_opt(Some(path_buf.as_path()), false); // writes to file "$CWD/my_dir/my_file.txt"
    /// let mut writer = out_stream_ident.create_output_writer()?;
    ///
    /// let out_stream_ident = StreamIdent::from_path_buf_opt(Some(path_buf), false); // writes to file "$CWD/my_dir/my_file.txt"
    /// let mut writer = out_stream_ident.create_output_writer()?;
    ///
    /// for line in lines {
    ///     writer.write_all(line.as_bytes())?;
    ///     writer.write_all(b"\n")?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// - if a file path is specified, and it is not possible to write to it
    /// - if this method is called on an input stream specifier
    #[cfg(not(feature = "async"))]
    pub fn create_output_writer(&self) -> io::Result<Box<dyn Write>> {
        match self {
            Self::StdOut => Ok(Self::create_output_writer_stdout()),
            Self::Path(path, false) => Self::create_output_writer_file(path),
            Self::StdIn | Self::Path(_, true) => Err(io::Error::other(
                "Can not create an output writer from an input stream identifier!",
            )),
        }
    }

    /// Creates a writer that writes to a file.
    /// See [`create_output_writer`].
    ///
    /// # Errors
    ///
    /// If a file path is specified, and it is not possible to write to it.
    #[cfg(feature = "async")]
    pub async fn create_output_writer_file<P: AsRef<Path> + ?Sized + Send + Sync>(
        file_path: &P,
    ) -> io::Result<Box<dyn Write + Unpin + Send + Sync>> {
        let file = File::open(file_path).await?;
        Ok(Box::new(file) as Box<dyn Write + Unpin + Send + Sync>)
    }

    /// Creates a writer that writes to a file.
    /// See [`create_output_writer`].
    ///
    /// # Errors
    ///
    /// If a file path is specified, and it is not possible to write to it.
    #[cfg(not(feature = "async"))]
    pub fn create_output_writer_file<P: AsRef<Path> + ?Sized + Send + Sync>(
        file_path: &P,
    ) -> io::Result<Box<dyn Write>> {
        let file = File::create(file_path)?;
        Ok(Box::new(file) as Box<dyn Write>)
    }

    /// Creates a writer that writes to stdout.
    /// See [`create_output_writer`].
    #[cfg(feature = "async")]
    #[must_use]
    pub fn create_output_writer_stdout() -> Box<dyn Write + Unpin + Send + Sync> {
        Box::new(io::stdout())
    }
    #[cfg(not(feature = "async"))]
    #[must_use]
    pub fn create_output_writer_stdout() -> Box<dyn Write> {
        Box::new(io::stdout())
    }
}

impl<P: AsRef<Path> + ?Sized + Unpin + Send + Sync> From<(Option<&P>, bool)> for StreamIdent {
    fn from((ident, r#in): (Option<&P>, bool)) -> Self {
        if let Some(file_path) = ident {
            return Self::from((file_path, r#in));
        }
        Self::new_std(r#in)
    }
}

impl From<(Option<PathBuf>, bool)> for StreamIdent {
    fn from((ident, r#in): (Option<PathBuf>, bool)) -> Self {
        if let Some(file_path) = ident {
            return Self::from((file_path, r#in));
        }
        Self::new_std(r#in)
    }
}

// impl<PB: Into<PathBuf>> From<(Option<PB>, bool)> for StreamIdent {
//     fn from((ident, r#in): (Option<PathBuf>, bool)) -> Self {
//         if let Some(file_path) = ident {
//             return Self::from((file_path, r#in));
//         }
//         Self::new_std(r#in)
//     }
// }

impl<P: AsRef<Path> + ?Sized + Unpin + Send + Sync> From<(&P, bool)> for StreamIdent {
    fn from((ident, r#in): (&P, bool)) -> Self {
        if ident.as_ref() != STREAM_PATH.as_path() {
            return Self::Path(PathBuf::from(ident.as_ref()), r#in);
        }
        Self::new_std(r#in)
    }
}

impl From<(PathBuf, bool)> for StreamIdent {
    fn from((ident, r#in): (PathBuf, bool)) -> Self {
        if ident.as_path() != STREAM_PATH.as_path() {
            return Self::Path(ident, r#in);
        }
        Self::new_std(r#in)
    }
}

/// Removes an EOL indicator from the end of the given string,
/// if one is present.
/// Removes either:
///
/// - "\r\n" as used in DOS and Windows, or
/// - "\n" as used in most of the rest of the universe, or
/// - "" if none of the above is present.
///
/// # Examples
///
/// ```rust
/// use cli_utils_hoijui::remove_eol;
///
/// # fn remove_eol_example() {
/// let mut line = String::from("my lines text\n");
/// let line_clean = String::from("my lines text");
/// remove_eol(&mut line);
/// assert_eq!(line, line_clean);
/// # }
/// ```
pub fn remove_eol(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
}

/// Creates an async stream of lines ("`Stream<String>`")
/// from an input stream (`BufReader`).
///
/// # Example
///
/// ```rust
/// # use std::io;
/// use cli_utils_hoijui::lines_iterator;
/// # #[cfg(feature = "async")]
/// use async_std::io::BufReader;
/// # #[cfg(feature = "async")]
/// use async_std::stream::StreamExt;
///
/// # #[cfg(feature = "async")]
/// # async fn lines_iterator_example<R: async_std::io::BufRead + Unpin>(reader: &mut BufReader<R>) -> io::Result<()> {
///     let mut lines_stream = lines_iterator(reader, true);
///     while let Some(line) = lines_stream.next().await {
///         println!("{}", &line?)
///     }
/// #     Ok(())
/// # }
/// ```
///
/// # Panics
///
/// - if `strip_eol` is `false`,
///   because that is not supported by the underlying function we use.
#[cfg(feature = "async")]
pub fn lines_iterator<R: async_std::io::BufRead + Unpin>(
    reader: &mut BufReader<R>,
    strip_eol: bool,
) -> impl Stream<Item = io::Result<String>> {
    assert!(
        strip_eol,
        "Async lines stream (~= iterator) always skips new-lines, \
so `strip_eol` must be `true`."
    );
    reader.lines()
}

/// Creates a line iterator ("`Iterator<String>`")
/// from an input stream (`BufRead`).
///
/// # Example
///
/// ```rust
/// # use std::io;
/// use cli_utils_hoijui::lines_iterator;
///
/// # #[cfg(not(feature = "async"))]
/// # fn lines_iterator_example(reader: &mut impl io::BufRead) -> io::Result<()> {
///     for line in lines_iterator(reader, true) {
///         println!("{}", &line?)
///     }
/// #     Ok(())
/// # }
/// ```
#[cfg(not(feature = "async"))]
pub fn lines_iterator(
    reader: &mut impl BufRead,
    strip_eol: bool,
) -> impl std::iter::Iterator<Item = io::Result<String>> + '_ {
    let mut buffer = String::new();
    std::iter::from_fn(move || {
        buffer.clear();
        reader.read_line(&mut buffer).map_or_else(
            |err| Some(Err(err)),
            |read_bytes| {
                if read_bytes == 0 {
                    // This means most likely that:
                    // > This reader has reached its "end of file"
                    // > and will likely no longer be able to produce bytes
                    // as can be read here:
                    // https://docs.w3cub.com/rust/std/io/trait.read#tymethod.read
                    //eprintln!("Zero bytes read, ending it here (assuming EOF).");
                    None // end of iterator
                } else {
                    // io::stdout().write_all(repl_vars_in(vars, &buffer, fail_on_missing)?.as_bytes())?;
                    if strip_eol {
                        remove_eol(&mut buffer);
                    }
                    Some(Ok(buffer.clone()))
                }
            },
        )
    })
}

/// Writes a list of strings to a file;
/// one per line.
/// See also: [`create_output_writer`]
///
/// # Example
///
/// ```rust
/// # use std::io;
/// # use async_std::path::Path;
/// # use async_std::path::PathBuf;
/// # use std::str::FromStr;
/// use cli_utils_hoijui::StreamIdent;
/// use cli_utils_hoijui::write_to_file;
///
/// # #[cfg(feature = "async")]
/// # async fn write_to_file_example() -> io::Result<()> {
/// let lines = vec!["line 1", "line 2", "line 3"];
///
/// let out_stream_ident = StreamIdent::StdOut; // writes to stdout
/// write_to_file(&lines, &out_stream_ident).await?;
///
/// let out_stream_ident = StreamIdent::Path(PathBuf::from("my_dir/my_file.txt").into(), false); // writes to file "$CWD/my_dir/my_file.txt"
/// write_to_file(&lines, &out_stream_ident).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// If writing to `destination` failed.
#[cfg(feature = "async")]
pub async fn write_to_file<L: AsRef<str> + Send + Sync>(
    lines: impl IntoIterator<Item = L>,
    destination: &StreamIdent,
) -> io::Result<()> {
    let writer = destination.create_output_writer().await?;

    let mut writer_pinned = Box::into_pin(writer);
    for line in lines {
        writer_pinned.write_all(line.as_ref().as_bytes()).await?;
        writer_pinned.write_all(b"\n").await?;
    }

    Ok(())
}

/// Writes a list of strings to a file;
/// one per line.
/// See also: [`create_output_writer`]
///
/// # Example
///
/// ```rust
/// # use std::io;
/// # use std::path::PathBuf;
/// # use std::str::FromStr;
/// use cli_utils_hoijui::StreamIdent;
/// use cli_utils_hoijui::write_to_file;
///
/// # #[cfg(not(feature = "async"))]
/// # fn write_to_file_example() -> io::Result<()> {
/// let lines = vec!["line 1", "line 2", "line 3"];
///
/// let out_stream_ident = StreamIdent::StdOut; // writes to stdout
/// write_to_file(&lines, &out_stream_ident)?;
///
/// let out_stream_ident = StreamIdent::Path(PathBuf::from("my_dir/my_file.txt"), false); // writes to file "$CWD/my_dir/my_file.txt"
/// write_to_file(&lines, &out_stream_ident)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// If writing to `destination` failed.
#[cfg(not(feature = "async"))]
pub fn write_to_file<L: AsRef<str>>(
    lines: impl IntoIterator<Item = L>,
    destination: &StreamIdent,
) -> io::Result<()> {
    let mut writer = destination.create_output_writer()?;

    for line in lines {
        writer.write_all(line.as_ref().as_bytes())?;
        writer.write_all(b"\n")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "async")]
    use {async_std::io::BufReader, async_std::stream::Stream, async_std::stream::StreamExt};

    use super::*;

    fn test_remove_eol_check(input: &str, expected: &str) {
        let mut actual = String::from(input);
        remove_eol(&mut actual);
        assert_eq!(expected, actual);
    }

    macro_rules! test_remove_eol {
        ($($name:ident: { input: $input:expr, expected: $expected:expr, },)*) => {
        $(
            #[test]
            fn $name() {
                // let (input, expected) = $pair;
                test_remove_eol_check($input, $expected);
            }
        )*
        }
    }

    test_remove_eol! {
        test_remove_eol_simple_none: {
            input: "my lines text",
            expected: "my lines text",
        },
        test_remove_eol_simple_nl: {
            input: "my lines text\n",
            expected: "my lines text",
        },
        test_remove_eol_simple_cr_nl: {
            input: "my lines text\r\n",
            expected: "my lines text",
        },
        test_remove_eol_simple_nl_cr: {
            input: "my lines text\n\r",
            expected: "my lines text\n\r",
        },
        test_remove_eol_simple_cr: {
            input: "my lines text\r",
            expected: "my lines text\r",
        },
    }

    fn stream_ident_from_path_opt_in_check(input: Option<&Path>, r#in: bool, expected_path: bool) {
        let expected = if expected_path {
            StreamIdent::Path(PathBuf::from(input.unwrap()), r#in)
        } else {
            StreamIdent::new_std(r#in)
        };
        let actual = StreamIdent::from_path_opt(input, r#in);
        assert_eq!(actual, expected);
    }

    fn stream_ident_from_path_opt_check(input: Option<&Path>, expected_path: bool) {
        stream_ident_from_path_opt_in_check(input, false, expected_path);
        stream_ident_from_path_opt_in_check(input, true, expected_path);
    }

    macro_rules! stream_ident_from_path_opt {
        ($($name:ident: { input: $input:expr, expected_path: $expected_path:expr, },)*) => {
        $(
            #[test]
            fn $name() {
                stream_ident_from_path_opt_check($input, $expected_path);
            }
        )*
        }
    }

    stream_ident_from_path_opt! {
        stream_ident_from_path_opt_none: {
            input: None,
            expected_path: false,
        },
        stream_ident_from_path_opt_std_const: {
            input: Some(STREAM_PATH.as_path()),
            expected_path: false,
        },
        stream_ident_from_path_opt_std_str_const: {
            input: Some(PathBuf::from(STREAM_PATH_STR).as_path()),
            expected_path: false,
        },
        stream_ident_from_path_opt_std_str_lit: {
            input: Some(PathBuf::from("-").as_path()),
            expected_path: false,
        },
        stream_ident_from_path_opt_path: {
            input: Some(PathBuf::from("/pth/x").as_path()),
            expected_path: true,
        },
    }

    fn stream_ident_from_path_buf_opt_in_check(
        input: Option<PathBuf>,
        r#in: bool,
        expected_path: bool,
    ) {
        let expected = if expected_path {
            StreamIdent::Path(input.clone().unwrap(), r#in)
        } else {
            StreamIdent::new_std(r#in)
        };
        let actual = StreamIdent::from_path_buf_opt(input, r#in);
        assert_eq!(actual, expected);
    }

    fn stream_ident_from_path_buf_opt_check(input: Option<PathBuf>, expected_path: bool) {
        stream_ident_from_path_buf_opt_in_check(input.clone(), false, expected_path);
        stream_ident_from_path_buf_opt_in_check(input, true, expected_path);
    }

    macro_rules! stream_ident_from_path_buf_opt {
        ($($name:ident: { input: $input:expr, expected_path: $expected_path:expr, },)*) => {
        $(
            #[test]
            fn $name() {
                stream_ident_from_path_buf_opt_check($input, $expected_path);
            }
        )*
        }
    }

    stream_ident_from_path_buf_opt! {
        stream_ident_from_path_buf_opt_none: {
            input: None,
            expected_path: false,
        },
        stream_ident_from_path_buf_opt_std_const: {
            input: Some(STREAM_PATH.clone()),
            expected_path: false,
        },
        stream_ident_from_path_buf_opt_std_str_const: {
            input: Some(PathBuf::from(STREAM_PATH_STR)),
            expected_path: false,
        },
        stream_ident_from_path_buf_opt_std_str_lit: {
            input: Some(PathBuf::from("-")),
            expected_path: false,
        },
        stream_ident_from_path_buf_opt_path: {
            input: Some(PathBuf::from("/pth/x")),
            expected_path: true,
        },
    }

    #[cfg(feature = "async")]
    async fn try_concat_stream<T, E>(
        stream: impl Stream<Item = Result<T, E>>,
    ) -> Result<Vec<T>, E> {
        let mut res = vec![];
        let mut stream_pinned = std::pin::pin!(stream);
        while let Some(item) = stream_pinned.next().await {
            res.push(item?);
        }
        Ok(res)
    }

    #[cfg(feature = "async")]
    async fn test_lines_iterator_check(
        input: &str,
        expected: &[&str],
        strip_eol: bool,
    ) -> io::Result<()> {
        let input_bytes = input.as_bytes();
        let mut input_buf_reader = BufReader::new(input_bytes);
        let actual = try_concat_stream(lines_iterator(&mut input_buf_reader, strip_eol)).await?;
        assert_eq!(expected, &actual);
        Ok(())
    }
    #[cfg(not(feature = "async"))]
    fn test_lines_iterator_check(
        input: &str,
        expected: &[&str],
        strip_eol: bool,
    ) -> io::Result<()> {
        let mut input_bytes = input.as_bytes();
        let actual = lines_iterator(&mut input_bytes, strip_eol).collect::<io::Result<Vec<_>>>()?;
        assert_eq!(expected, &actual);
        Ok(())
    }

    #[cfg(not(feature = "async"))]
    macro_rules! test_lines_iterator {
        ($($name:ident: { input: $input:expr, expected: $expected:expr, },)*) => {
        $(
            #[test]
            fn $name() -> io::Result<()> {
                test_lines_iterator_check($input, $expected, false)
            }
        )*
        }
    }

    #[cfg(not(feature = "async"))]
    test_lines_iterator! {
        test_lines_iterator_simple_1: {
            input: "line 1\nline 2\nline 3",
            expected: &["line 1\n", "line 2\n", "line 3"],
        },
        test_lines_iterator_simple_2: {
            input: "line 1\nline 2\nline 3\n",
            expected: &["line 1\n", "line 2\n", "line 3\n"],
        },
        test_lines_iterator_windows_1: {
            input: "line 1\r\nline 2\r\nline 3",
            expected: &["line 1\r\n", "line 2\r\n", "line 3"],
        },
        test_lines_iterator_windows_2: {
            input: "line 1\r\nline 2\r\nline 3\r\n",
            expected: &["line 1\r\n", "line 2\r\n", "line 3\r\n"],
        },
        test_lines_iterator_mixed_1: {
            input: "line 1\nline 2\r\nline 3",
            expected: &["line 1\n", "line 2\r\n", "line 3"],
        },
        test_lines_iterator_mixed_2: {
            input: "line 1\r\nline 2\nline 3",
            expected: &["line 1\r\n", "line 2\n", "line 3"],
        },
        test_lines_iterator_mixed_3: {
            input: "line 1\nline 2\r\nline 3\n",
            expected: &["line 1\n", "line 2\r\n", "line 3\n"],
        },
        test_lines_iterator_mixed_4: {
            input: "line 1\r\nline 2\nline 3\r\n",
            expected: &["line 1\r\n", "line 2\n", "line 3\r\n"],
        },
    }

    #[cfg(feature = "async")]
    macro_rules! test_lines_iterator_strip {
        ($($name:ident: $input:expr,)*) => {
        $(
            #[tokio::test]
            async fn $name() -> io::Result<()> {
                test_lines_iterator_check($input, &["line 1", "line 2", "line 3"], true).await
            }
        )*
        }
    }
    #[cfg(not(feature = "async"))]
    macro_rules! test_lines_iterator_strip {
        ($($name:ident: $input:expr,)*) => {
        $(
            #[test]
            fn $name() -> io::Result<()> {
                test_lines_iterator_check($input, &["line 1", "line 2", "line 3"], true)
            }
        )*
        }
    }

    test_lines_iterator_strip! {
        test_lines_iterator_strip_simple_1:
            "line 1\nline 2\nline 3",
        test_lines_iterator_strip_simple_2:
            "line 1\nline 2\nline 3\n",
        test_lines_iterator_strip_windows_1:
            "line 1\r\nline 2\r\nline 3",
        test_lines_iterator_strip_windows_2:
            "line 1\r\nline 2\r\nline 3\r\n",
        test_lines_iterator_strip_mixed_1:
            "line 1\nline 2\r\nline 3",
        test_lines_iterator_strip_mixed_2:
            "line 1\r\nline 2\nline 3",
        test_lines_iterator_strip_mixed_3:
            "line 1\nline 2\r\nline 3\n",
        test_lines_iterator_strip_mixed_4:
            "line 1\r\nline 2\nline 3\r\n",
    }
}
