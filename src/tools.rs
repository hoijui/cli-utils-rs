// SPDX-FileCopyrightText: 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// Creates a reader from a string identifier.
/// Both `None` and `Some("-")` mean stdin.
///
/// # Errors
///
/// If a file path is specified, and it is not possible to read from it.
pub fn create_input_reader(ident: Option<&str>) -> io::Result<Box<dyn BufRead>> {
    match ident {
        None | Some("-") => Ok(Box::new(BufReader::new(io::stdin()))),
        Some(filename) => {
            let file = File::open(filename)?;
            Ok(Box::new(BufReader::new(file)))
        }
    }
}

/// Creates a writer from a string identifier.
/// Both `None` and `Some("-")` mean stdout.
///
/// # Errors
///
/// If a file path is specified, and it is not possible to write to it.
pub fn create_output_writer(ident: Option<&str>) -> io::Result<Box<dyn Write>> {
    match ident {
        None | Some("-") => Ok(Box::new(io::stdout()) as Box<dyn Write>),
        Some(file) => {
            let path = Path::new(file);
            let file = File::create(path)?;
            Ok(Box::new(file) as Box<dyn Write>)
        }
    }
}

/// Removes an EOL indicator from the end of the given string,
/// if one is present.
/// Removes either:
/// * "\r\n" as used in DOS and Windows, or
/// * "\n" as used in most of the rest of the universe, or
/// * "" if none of the above is present.
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
///
/// # Errors
///
/// If writing to `destination` failed.
pub fn write_to_file<L: AsRef<str>>(lines: Vec<L>, destination: Option<&str>) -> io::Result<()> {
    let mut writer = crate::tools::create_output_writer(destination)?;

    for line in lines {
        writer.write_all(line.as_ref().as_bytes())?;
        writer.write_all(b"\n")?;
    }

    Ok(())
}
