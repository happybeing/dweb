/*
Copyright (c) 2025 Mark Hughes

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use std::time::{Duration, SystemTime};

use autonomi::files::Metadata;

/// Get autonommi::files;:Metadata for a file.
/// Defaults creation and modification times to zero if any error is encountered.
pub fn metadata_for_file(path: &str) -> Metadata {
    let unix_time = |property: &'static str, time: std::io::Result<SystemTime>| {
        time.inspect_err(|err| {
            println!("Failed to get '{property}' metadata for `{path}`: {err}");
        })
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .inspect_err(|err| {
            println!("'{property}' metadata of `{path}` is before UNIX epoch: {err}");
        })
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
    };

    let mut created = 0;
    let mut modified = 0;
    let mut size = 0;
    if let Ok(fs_metadata) = std::fs::metadata(path) {
        created = unix_time("created", fs_metadata.created());
        modified = unix_time("modified", fs_metadata.modified());
        size = fs_metadata.len()
    };

    Metadata {
        created,
        modified,
        size,
        extra: None,
    }
}
