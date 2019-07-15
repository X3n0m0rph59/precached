/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2019 the precached developers

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

use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use crate::util::mountinfo::MountInfo;

/// Maps the path `filename` from a different mount namespace into the current
/// mount namespace, using information from the supplied `MountInfo`
pub fn find_source_path(mounts: &[MountInfo], filename: &Path) -> Option<PathBuf> {
    let mut result = None;
    let mut longest_match_source = None;
    let mut longest_match_dest = None;
    let mut match_len = 0;

    for mount in mounts {
        if filename.starts_with(&mount.dest) {
            let p = PathBuf::from(&mount.source);
            let len = mount.source.components().count();

            if len >= match_len {
                longest_match_source = Some(p);
                longest_match_dest = Some(PathBuf::from(&mount.dest));

                match_len = len;
            }
        }
    }

    if let Some(longest_match_source) = longest_match_source {
        let longest_match_dest = longest_match_dest.unwrap();

        let len = longest_match_dest.iter().count();
        let f = PathBuf::from_iter(filename.iter().skip(len));
        let f = longest_match_source.join(f);

        result = Some(f);
    }

    result
}
