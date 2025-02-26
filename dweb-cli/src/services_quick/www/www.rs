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

// use actix_web::{body, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web::{
    body,
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    get, guard,
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    post,
    web::{self, Data},
    App, Error, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use mime;

use dweb::cache::directory_with_port::DirectoryVersionWithPort;
use dweb::web::fetch::{fetch_website_version, response_redirect, response_with_body};

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
    client: Data<dweb::client::AutonomiClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
) -> HttpResponse {
    let path = request.path().to_string();
    println!("DEBUG www_handler({path})...");
    if our_directory_version.is_none() {
        return response_with_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(String::from("www_handler() failed to determine host")),
        );
    };
    let our_directory_version = <std::option::Option<
        dweb::cache::directory_with_port::DirectoryVersionWithPort,
    > as Clone>::clone(&our_directory_version.as_ref())
    .unwrap();
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
        .lookup_web_resource(&path)
    {
        Ok((file_address, content_type)) => match client.data_get_public(file_address).await {
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
                        "DirectoryTree::lookup_web_resource({}) failed: {e}",
                        path.as_str()
                    ))),
                );
            }
        },
        Err(e) => {
            let status_code = if let Ok(status_code) = StatusCode::from_u16(e.as_u16()) {
                status_code
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            return response_with_body(
                status_code,
                Some(String::from(format!(
                    "DirectoryTree::lookup_web_resource({}) failed",
                    path.as_str()
                ))),
            );
        }
    };
}
