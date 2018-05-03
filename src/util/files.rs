/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2018 the precached developers

    This file is part of precached.

    Precached is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Precached is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Precached.  If not, see <http://www.gnu.org/licenses/>.
*/

extern crate globset;
extern crate rayon;
extern crate zstd;

use self::globset::{Glob, GlobSetBuilder};
use constants;
use rayon::prelude::*;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

lazy_static! {
    pub static ref GLOB_SET: Arc<Mutex<Option<globset::GlobSet>>> = { Arc::new(Mutex::new(None)) };
}

/// Returns a `Vec<String>` containing all the lines of the text file `filename`
pub fn get_lines_from_file(filename: &Path) -> io::Result<Vec<String>> {
    let file = try!(OpenOptions::new().read(true).open(filename));

    let mut result = vec![];

    let mut reader = BufReader::new(file);

    'LINE_LOOP: loop {
        let mut line = String::new();
        let len = reader.read_line(&mut line).unwrap_or(0);

        if len > 0 {
            result.push(line);
        } else {
            break 'LINE_LOOP;
        }
    }

    Ok(result)
}

/// Read the compressed text file `filename`.
/// This function transparently decompresses Zstd compressed text files
pub fn read_compressed_text_file(filename: &Path) -> io::Result<String> {
    let file = try!(OpenOptions::new().read(true).open(filename));

    let decompressed = zstd::decode_all(BufReader::new(file)).unwrap();
    let result = String::from_utf8(decompressed).unwrap();

    Ok(result)
}

/// Read the uncompressed text file `filename`.
/// This function does *not* perform transparent de-compression
pub fn read_uncompressed_text_file(filename: &Path) -> io::Result<String> {
    let file = try!(OpenOptions::new().read(true).open(filename));

    let mut result = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut result)?;

    Ok(result)
}

/// Write `text` to the compressed file `filename`.
/// This function transparently compresses the file using Zstd compression
pub fn write_text_file(filename: &Path, text: &str) -> io::Result<()> {
    let mut file = try!(OpenOptions::new().write(true).truncate(true).create(true).open(filename));

    let compressed = try!(zstd::encode_all(BufReader::new(text.as_bytes()), constants::ZSTD_COMPRESSION_RATIO));
    file.write_all(&compressed)?;
    // file.sync_data()?;

    Ok(())
}

// Create directory `dirname`
pub fn mkdir(dirname: &Path) -> io::Result<()> {
    fs::create_dir_all(dirname)?;

    Ok(())
}

// Remove (empty) directory `dirname`
pub fn rmdir(dirname: &Path) -> io::Result<()> {
    fs::remove_dir(dirname)?;

    Ok(())
}

/// Echo (rewrite) `text` + "\n" to the file `filename`
pub fn echo(filename: &Path, text: String) -> io::Result<()> {
    let mut file = try!(OpenOptions::new().write(true).append(false).open(filename));

    let mut buffer = text;
    buffer.push_str("\n");

    file.write_all(buffer.into_bytes().as_slice())?;

    Ok(())
}

/// Echo (append) `text` + "\n" to the file `filename`
pub fn append(filename: &Path, mut text: String) -> io::Result<()> {
    let mut file = try!(OpenOptions::new().write(true).append(true).open(filename));

    text.push_str("\n");

    file.write_all(text.into_bytes().as_slice())?;

    Ok(())
}

/// Delete file `filename` if `dry_run` is set to `false`
pub fn remove_file(filename: &Path, dry_run: bool) -> io::Result<()> {
    trace!("Deleting file: {:?}", filename);

    if !dry_run {
        fs::remove_file(filename)
    } else if let Err(result) = fs::metadata(filename) {
        Err(result)
    } else {
        Ok(())
    }
}

/// Validates `filename`
/// Returns `true` if the filename is valid, `false` otherwise
pub fn is_filename_valid(filename: &Path) -> bool {
    if filename.to_string_lossy().into_owned().is_empty() {
        return false;
    }

    true
}

/// Validates `filename`. Check if it is a valid regular file identified by an absolute path
/// Returns `true` if the file is a valid regular file, `false` otherwise
pub fn is_file_valid(filename: &Path) -> bool {
    if !filename.is_absolute() {
        return false;
    }

    if is_directory(filename) {
        return false;
    }

    true
}

/// Returns `true` if `filename` matches a pattern in `pattern`, otherwise returns `false`.
/// The Vec<String> may contain any POSIX compatible glob pattern.
pub fn is_file_blacklisted(filename: &Path, pattern: &[PathBuf]) -> bool {
    match GLOB_SET.lock() {
        Err(e) => {
            error!("Could not lock a shared data structure! {}", e);
            false
        }

        Ok(mut gs_opt) => {
            if gs_opt.is_none() {
                // construct a glob set at the first iteration
                let mut builder = GlobSetBuilder::new();
                for p in pattern.iter() {
                    builder.add(Glob::new(p.to_string_lossy().into_owned().as_str()).unwrap());
                }
                let set = builder.build().unwrap();

                *gs_opt = Some(set.clone());

                // glob_set already available
                let matches = set.matches(&filename.to_string_lossy().into_owned());

                !matches.is_empty()
            } else {
                // glob_set already available
                let matches = gs_opt.clone().unwrap().matches(&filename.to_string_lossy().into_owned());

                !matches.is_empty()
            }
        }
    }
}

/// Returns `true` if the file `filename` is accessible
pub fn is_file_accessible(filename: &Path) -> bool {
    fs::metadata(filename).is_ok()
}

/// Returns the size in bytes of the file `filename`
pub fn get_file_size(filename: &Path) -> io::Result<u64> {
    Ok(fs::metadata(filename)?.len())
}

/// Returns `true` if `filename` is a regular file, otherwise returns `false`
pub fn is_file(filename: &Path) -> bool {
    filename.is_file()
}

/// Returns `true` if `dirname` is a directory, otherwise returns `false`
pub fn is_directory(dirname: &Path) -> bool {
    dirname.is_dir()
}

/// Shortens a long path in `filename` by replacing components of it with an ellipsis "…" sequence
/// Returns the shortened path
pub fn ellipsize_filename(filename: &str, max_len: usize) -> Result<String, &'static str> {
    if max_len < 6 {
        Ok(String::from("<…>"))
    } else if filename.len() > max_len {
        let mut result = String::new();

        let cutoff = filename.len() as i64 - max_len as i64;
        let max_len = min(filename.len() as i64 - 1, max_len as i64) as i64;

        let lower_bound: i64 = max((max_len as i64 - 1) / 2 - cutoff / 2, 0 as i64);
        let upper_bound: i64 = min((max_len as i64 - 1) / 2 + 1 + cutoff / 2, filename.len() as i64 - 1 as i64);

        if lower_bound < 0 {
            return Err("Lower bound is < 0");
        } else if lower_bound > filename.len() as i64 - 1 {
            return Err("Lower bound is > filename.len()");
        }

        if upper_bound < 0 {
            return Err("Upper bound is < 0");
        } else if upper_bound > filename.len() as i64 - 1 {
            return Err("Upper bound is > filename.len()");
        }

        let lower_bound: usize = lower_bound as usize;
        let upper_bound: usize = upper_bound as usize;

        result.push_str(&filename[..lower_bound]);
        result.push_str("…");
        result.push_str(&filename[upper_bound..]);

        Ok(result)
    } else {
        Ok(String::from(filename))
    }
}

/// Recursively enumerate files and directories in `dir`
pub fn visit_dirs(dir: &Path, cb: &mut FnMut(&Path)) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            visit_dirs(&path, cb)?;
        } else {
            // trace!("{:#?}", &entry);
            cb(&entry.path());
        }
    }

    Ok(())
}

/// Recursively enumerate files and directories in `entries`, calls callback function `cb` for each entry
pub fn walk_directories<F>(entries: &[PathBuf], cb: &mut F) -> io::Result<()>
where
    F: FnMut(&Path),
{
    // TODO: Parallelize? Check ordering!
    for e in entries.iter() {
        let path = Path::new(e);
        if path.is_file() {
            cb(path);
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dirs(&path, cb)?;
                } else {
                    // trace!("{:#?}", &entry);
                    cb(&entry.path());
                }
            }
        } else {
            error!("Error while enumerating {:?}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use util::files::*;

    #[test]
    fn test_ellipsize_filename() {
        let filename = "/123/123/123";

        let result = ellipsize_filename(filename, 5).unwrap();
        assert_eq!("<…>", result);

        let result = ellipsize_filename(filename, 10).unwrap();
        assert_eq!("/12…23/123", result);
    }
}
