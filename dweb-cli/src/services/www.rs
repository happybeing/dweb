/*
 Copyright (c) 2024-2025 Mark Hughes

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

// Handlers for routes such as /dweb-info etc
pub(crate) mod dweb_info;
pub(crate) mod dweb_open;
pub(crate) mod dweb_version;

use actix_web::{http::StatusCode, web::Data, HttpRequest, HttpResponse};

use dweb::cache::directory_with_port::DirectoryVersionWithPort;
use dweb::files::directory::get_content;
use dweb::web::fetch::response_with_body;

use super::helpers::*;

/// Handle Autonomi www requests of the form:
///     http://localhost:<PORT>/here/is/a/path.html
///
/// This service uses one port for each History (website) in order to allow
/// viewing without extra setup of a local DNS. When access to a new
/// site is requested, it is looked up in a map and if not present
/// a new server is spawned on a new port to serve those requests,
/// and the request will be re-directed to that port.
///
/// Most routes will be handled in the same way as local redirect
/// handler.
///
pub async fn www_handler(
    request: HttpRequest,
    // path: web::Path<String>,
    is_main_server: Data<bool>,
    client: Data<dweb::client::DwebClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
) -> HttpResponse {
    let path = request.path().to_string();
    println!("DEBUG www_handler({path})...");

    // If we're the main server arriving here means no API handler for the route
    if *is_main_server.into_inner() {
        return make_error_response_page(
            Some(StatusCode::NOT_FOUND),
            &mut HttpResponse::NotFound(),
            "main dweb server error".to_string(),
            &format!("- check the URL is a valid API"),
        );
    }

    let our_directory_version = if our_directory_version.is_some() {
        our_directory_version.as_ref().clone().unwrap()
    } else {
        return make_error_response_page(
            Some(StatusCode::INTERNAL_SERVER_ERROR),
            &mut HttpResponse::InternalServerError(),
            "dweb www error".to_string(),
            &format!("Unable to access our_directory_version - probably a bug"),
        );
    };

    println!("DEBUG our_directory_version:");
    println!("            port: {}", our_directory_version.port);
    println!(
        " history_address: {:?}",
        our_directory_version.history_address
    );
    println!("         version: {:?}", our_directory_version.version);
    println!(
        " archive_address: {}",
        our_directory_version.archive_address
    );

    match our_directory_version
        .directory_tree
        .lookup_file(&path, true)
    {
        Ok((datamap_chunk, data_address, content_type)) => {
            match get_content(&client, datamap_chunk, data_address).await {
                Ok(content) => {
                    let mut response = HttpResponse::Ok();
                    if let Some(content_type) = content_type {
                        response.insert_header(("Content-Type", content_type.as_str()));
                    }
                    return response.body(content);
                }
                Err(e) => {
                    return response_with_body(
                        StatusCode::BAD_GATEWAY,
                        Some(String::from(format!(
                            "Tree::lookup_file({}) failed: {e}",
                            path.as_str()
                        ))),
                    );
                }
            }
        }
        Err(e) => {
            let status_code = if let Ok(status_code) = StatusCode::from_u16(e.as_u16()) {
                status_code
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            return response_with_body(
                status_code,
                Some(String::from(format!(
                    "Tree::lookup_file({}) failed",
                    path.as_str()
                ))),
            );
        }
    };
}
