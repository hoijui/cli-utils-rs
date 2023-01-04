// SPDX-FileCopyrightText: 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use once_cell::sync::Lazy;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub static STREAM_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from_str("-").expect("This failing should be impossilbe!"));

/// Creates a reader from a string identifier.
/// Both `None` and `Some("-")` mean stdin.
///
/// # Example
///
/// ```rust
/// # use std::io;
/// # use std::path::PathBuf;
/// # use std::str::FromStr;
/// # fn create_input_reader_example() -> io::Result<()> {
///
/// let in_file = None as Option<&str>; // reads from stdin
/// let mut reader = cli_utils::create_input_reader(in_file)?;
///
/// let in_file = Some("-"); // reads from stdin
/// let mut reader = cli_utils::create_input_reader(in_file)?;
///
/// let in_file = Some("my_dir/my_file.txt"); // reads from file "$CWD/my_dir/my_file.txt"
/// let mut reader = cli_utils::create_input_reader(in_file)?;
///
/// let in_file = Some("my_dir/my_file.txt".to_string()); // reads from file "$CWD/my_dir/my_file.txt"
/// let mut reader = cli_utils::create_input_reader(in_file)?;
///
/// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossilbe!");
/// let in_file = Some(path_buf.as_path()); // reads from file "$CWD/my_dir/my_file.txt"
/// let mut reader = cli_utils::create_input_reader(in_file)?;
///
/// let in_file = Some(path_buf); // reads from file "$CWD/my_dir/my_file.txt"
/// let mut reader = cli_utils::create_input_reader(in_file)?;
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
/// If a file path is specified, and it is not possible to read from it.
pub fn create_input_reader<P: AsRef<Path>>(ident: Option<P>) -> io::Result<Box<dyn BufRead>> {
    if let Some(file_path) = ident {
        if file_path.as_ref() != STREAM_PATH.as_path() {
            let file = File::open(file_path)?;
            return Ok(Box::new(BufReader::new(file)));
        }
    }
    Ok(Box::new(BufReader::new(io::stdin())))
}

/// Creates a writer from a string identifier.
/// Both `None` and `Some("-")` mean stdout.
/// See also: [`write_to_file`]
///
/// # Example
///
/// ```rust
/// # use std::io;
/// # use std::path::PathBuf;
/// # use std::str::FromStr;
/// # fn create_output_writer_example() -> io::Result<()> {
/// let lines = vec!["line 1", "line 2", "line 3"];
///
/// let out_file = None as Option<&str>; // writes to stdout
/// let mut writer = cli_utils::create_output_writer(out_file)?;
///
/// let out_file = Some("-"); // writes to stdout
/// let mut writer = cli_utils::create_output_writer(out_file)?;
///
/// let out_file = Some("my_dir/my_file.txt"); // writes to file "$CWD/my_dir/my_file.txt"
/// let mut writer = cli_utils::create_output_writer(out_file)?;
///
/// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossilbe!");
/// let out_file = Some(path_buf.as_path()); // writes to file "$CWD/my_dir/my_file.txt"
/// let mut writer = cli_utils::create_output_writer(out_file)?;
///
/// let out_file = Some(path_buf); // writes to file "$CWD/my_dir/my_file.txt"
/// let mut writer = cli_utils::create_output_writer(out_file)?;
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
/// If a file path is specified, and it is not possible to write to it.
pub fn create_output_writer<P: AsRef<Path>>(ident: Option<P>) -> io::Result<Box<dyn Write>> {
    if let Some(file_path) = ident {
        if file_path.as_ref() != STREAM_PATH.as_path() {
            let file = File::create(file_path)?;
            return Ok(Box::new(file) as Box<dyn Write>);
        }
    }
    Ok(Box::new(io::stdout()) as Box<dyn Write>)
}

/// Removes an EOL indicator from the end of the given string,
/// if one is present.
/// Removes either:
/// * "\r\n" as used in DOS and Windows, or
/// * "\n" as used in most of the rest of the universe, or
/// * "" if none of the above is present.
///
/// # Examples
///
/// ```rust
/// # fn remove_eol_example() {
/// let mut line = String::from("my lines text\n");
/// let line_clean = String::from("my lines text");
/// cli_utils::remove_eol(&mut line);
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

/// Creates a line iterator ("`Iterator<String>`")
/// from an input stream (`BufRead`).
///
/// # Example
///
/// ```rust
/// # use std::io;
/// fn lines_iterator_example(reader: &mut impl io::BufRead) -> io::Result<()> {
///     for line in cli_utils::lines_iterator(reader, true) {
///         println!("{}", &line?)
///     }
///     Ok(())
/// }
/// ```
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
/// # use std::path::PathBuf;
/// # use std::str::FromStr;
/// # fn write_to_file_example() -> io::Result<()> {
/// let lines = vec!["line 1", "line 2", "line 3"];
///
/// let out_file = None as Option<&str>; // writes to stdout
/// cli_utils::write_to_file(&lines, out_file)?;
///
/// let out_file = Some("-"); // writes to stdout
/// cli_utils::write_to_file(&lines, out_file)?;
///
/// let out_file = Some("my_dir/my_file.txt"); // writes to file "$CWD/my_dir/my_file.txt"
/// cli_utils::write_to_file(&lines, out_file)?;
///
/// let path_buf = PathBuf::from_str("my_dir/my_file.txt").expect("This failing should be impossilbe!");
/// let out_file = Some(path_buf.as_path()); // writes to file "$CWD/my_dir/my_file.txt"
/// cli_utils::write_to_file(&lines, out_file)?;
///
/// let out_file = Some(path_buf); // writes to file "$CWD/my_dir/my_file.txt"
/// cli_utils::write_to_file(&lines, out_file)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// If writing to `destination` failed.
pub fn write_to_file<L: AsRef<str>, P: AsRef<Path>>(
    lines: impl IntoIterator<Item = L>,
    destination: Option<P>,
) -> io::Result<()> {
    let mut writer = crate::tools::create_output_writer(destination)?;

    for line in lines {
        writer.write_all(line.as_ref().as_bytes())?;
        writer.write_all(b"\n")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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
        test_remove_eol_simple_crnl: {
            input: "my lines text\r\n",
            expected: "my lines text",
        },
        test_remove_eol_simple_nlcr: {
            input: "my lines text\n\r",
            expected: "my lines text\n\r",
        },
        test_remove_eol_simple_cr: {
            input: "my lines text\r",
            expected: "my lines text\r",
        },
    }

    fn test_lines_iterator_check(
        input: &str,
        expected: &[&str],
        strip_eol: bool,
    ) -> io::Result<()> {
        let mut input = input.as_bytes();
        let actual = lines_iterator(&mut input, strip_eol).collect::<io::Result<Vec<_>>>()?;
        assert_eq!(expected, &actual);
        Ok(())
    }

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
