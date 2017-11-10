/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017 the precached developers

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
extern crate zstd;

use self::globset::{Glob, GlobSetBuilder};
use constants;
use std::cell::RefCell;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::path;
use std::path::Path;
use std::sync::{Arc, Mutex};

lazy_static! {
    pub static ref GLOB_SET: Arc<Mutex<Option<globset::GlobSet>>> = { Arc::new(Mutex::new(None)) };
}

/// Returns a `Vec<String>` containing all the lines of the file `filename`
pub fn get_lines_from_file(filename: &str) -> io::Result<Vec<String>> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

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
/// This function transparently decompresses Zstd compressed files
pub fn read_compressed_text_file(filename: &str) -> io::Result<String> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

    let decompressed = zstd::decode_all(BufReader::new(file)).unwrap();
    let result = String::from_utf8(decompressed).unwrap();

    Ok(result)
}

/// Read the uncompressed text file `filename`.
/// This function does *not* perform transparent de-compression
pub fn read_uncompressed_text_file(filename: &str) -> io::Result<String> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

    let mut result = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut result)?;

    Ok(result)
}

/// Write `text` to the compressed file `filename`.
/// This function transparently compresses the file using Zstd compression
pub fn write_text_file(filename: &str, text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
    );


    let compressed = zstd::encode_all(
        BufReader::new(text.as_bytes()),
        constants::ZSTD_COMPRESSION_RATIO,
    ).unwrap();
    file.write_all(&compressed)?;
    // file.sync_data()?;

    Ok(())
}

// Create directory `dirname`
pub fn mkdir(dirname: &str) -> io::Result<()> {
    fs::create_dir_all(dirname)?;

    Ok(())
}

// Remove (empty) directory `dirname`
pub fn rmdir(dirname: &str) -> io::Result<()> {
    fs::remove_dir(dirname)?;

    Ok(())
}

/// Echo (rewrite) `text` + "\n" to the file `filename`
pub fn echo(filename: &str, text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new().write(true).append(false).open(&path));

    let mut buffer = text;
    buffer.push_str("\n");

    file.write_all(buffer.into_bytes().as_slice())?;

    Ok(())
}

/// Echo (append) `text` + "\n" to the file `filename`
pub fn append(filename: &str, mut text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new().write(true).append(true).open(&path));

    text.push_str("\n");

    file.write_all(text.into_bytes().as_slice())?;

    Ok(())
}

/// Delete file `filename` if `dry_run` is set to `false`
pub fn remove_file(filename: &str, dry_run: bool) -> io::Result<()> {
    trace!("deleting file: '{}'", &filename);

    if !dry_run {
        fs::remove_file(filename)
    } else {
        if let Err(result) = fs::metadata(Path::new(&filename)) {
            Err(result)
        } else {
            Ok(())
        }
    }
}

/// Validates `filename`
/// Returns `true` if the filename is valid, `false` otherwise
pub fn is_filename_valid(filename: &String) -> bool {
    let f = filename.trim();

    if f.len() == 0 {
        return false;
    }

    return true;
}

/// Validates `filename`. Check if it is a valid regular file identified by an absolute path
/// Returns `true` if the file is a valid regular file, `false` otherwise
pub fn is_file_valid(filename: &String) -> bool {
    if !Path::new(filename).is_absolute() {
        return false;
    }

    if is_directory(filename) {
        return false;
    }

    return true;
}

/// Returns `true` if `filename` matches a pattern in `pattern`, otherwise returns `false`.
/// The Vec<String> may contain any POSIX compatible glob pattern.
pub fn is_file_blacklisted(filename: &String, pattern: &Vec<String>) -> bool {
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
                    builder.add(Glob::new(p).unwrap());
                }
                let set = builder.build().unwrap();

                *gs_opt = Some(set.clone());

                // glob_set already available
                let matches = set.matches(&filename);

                matches.len() > 0
            } else {
                // glob_set already available
                let matches = gs_opt.clone().unwrap().matches(&filename);

                matches.len() > 0
            }
        }
    }
}

/// Returns `true` if the file `filename` is accessible
pub fn is_file_accessible(filename: &String) -> bool {
    fs::metadata(Path::new(&filename)).is_ok()
}

/// Returns the size in bytes of the file `filename`
pub fn get_file_size(filename: &String) -> io::Result<u64> {
    Ok(fs::metadata(Path::new(&filename))?.len())
}

/// Returns `true` if `filename` is a regular file, otherwise returns `false`
pub fn is_file(filename: &String) -> bool {
    Path::new(filename).is_file()
}

/// Returns `true` if `dirname` is a directory, otherwise returns `false`
pub fn is_directory(dirname: &String) -> bool {
    Path::new(dirname).is_dir()
}

/// Shortens a long path in `filename` by replacing components of it with an ellipsis "..." sequence
/// Returns the shortened path
pub fn ellipsize_filename(filename: &String) -> String {
    let mut result = String::from("");

    const MAX_LEN: usize = 50;
    const CUTOFF: usize = 10;

    if filename.len() > MAX_LEN {
        result.push_str(&filename[0..MAX_LEN / 2 - CUTOFF]);
        result.push_str(&"...");
        result.push_str(&filename[MAX_LEN / 2 + CUTOFF..]);

    } else {
        result += filename;
    }

    result
}

/// Recursively enumerate files and directories in `dir`
pub fn visit_dirs(dir: &Path, cb: &mut FnMut(&Path)) -> io::Result<()> {
    if dir.is_dir() {
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
    }

    Ok(())
}

/// Recursively enumerate files and directories in `entries`, calls callback function `cb` for each entry
pub fn walk_directories<F>(entries: &Vec<String>, cb: &mut F) -> io::Result<()>
where
    F: FnMut(&Path),
{
    for e in entries.iter() {
        let path = Path::new(e);
        if path.is_file() {
            cb(path);
        } else {
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
        }
    }

    Ok(())
}
