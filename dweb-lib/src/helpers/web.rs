/*
 Copyright (c) 2025- Mark Hughes

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

use actix_web::HttpRequest;
use chrono::offset::Utc;
use chrono::DateTime;
use std::time::{Duration, UNIX_EPOCH};

use crate::trove::directory_tree::DirectoryTreePathMap;

/// Return HTML detailing an HttpRequest including its headers
pub fn request_as_html(request: &HttpRequest) -> String {
    let mut headers = String::from(
        "   <tr><td></td><td></td></tr>
        <tr><td><b>HEADERS:</b></td><td></td></tr>
    ",
    );
    for (key, value) in request.headers().iter() {
        headers += format!("<tr><td>{key:?}</td><td>{value:?}</td></tr>").as_str();
    }

    format!(
        "
        <table rules='all' style='border: solid;'>
           <tr><td></td><td></td></tr>
        <tr><td><b>HttpRequest:</b></td><td></td></tr>
        <tr><td>full_url</td><td>{}</td></tr>
        <tr><td>uri</td><td>{}</td></tr>
        <tr><td>method</td><td>{}</td></tr>
        <tr><td>path</td><td>{}</td></tr>
        <tr><td>query_string</td><td>{}</td></tr>
        <tr><td>peer_addr</td><td>{:?}</td></tr>
        {headers}
        </table>
        ",
        request.full_url(),
        request.uri(),
        request.method(),
        request.path(),
        request.query_string(),
        request.peer_addr(),
    )
}

// A JSON string representing a date from autonomi::files::Metadata
pub fn json_date_from_metadata(date: u64) -> String {
    let date: DateTime<Utc> = (UNIX_EPOCH + Duration::from_secs(date)).into();
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}

// The JSON representation of a DirectoryTree, for the SVAR file manager
pub fn json_for_svar_file_manager(directory_map: &DirectoryTreePathMap) -> String {
    let mut json_string = "[".to_string();
    let mut is_first_item = true;

    for (path, files) in directory_map.paths_to_files_map.iter() {
        let mut path = path.to_string();

        let mut directory_modified: u64 = 0;
        let mut directory_size: u64 = 0;
        for (filename, _data_address, metadata) in files {
            if !is_first_item {
                json_string = json_string + ",\n";
            }
            let file_id = format!("{path}{filename}");
            let file_size = metadata.size;
            let file_modified = json_date_from_metadata(metadata.modified);
            json_string = json_string + &format!("{{\"id\": \"{file_id}\", \"size\": {file_size}, \"date\": \"{file_modified}\", \"type\": \"file\" }}");

            if metadata.modified > directory_modified {
                directory_modified = metadata.modified
            }

            directory_size = directory_size + file_size;
            is_first_item = false;
        }

        if path.ends_with("/") {
            path = path[..path.len() - 1].to_string();
        }

        if path.len() > 0 {
            if !is_first_item {
                json_string = json_string + ",\n";
            }
            let directory_modified = json_date_from_metadata(directory_modified);
            json_string = json_string
            + &format!(
                "{{\"id\": \"{path}\", \"size\": {directory_size}, \"date\": \"{directory_modified}\", \"type\": \"folder\" }}"
            );
        }
        is_first_item = false;
    }

    json_string + "\n]"
}
