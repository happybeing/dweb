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

use actix_web::{
    http::header::ContentType, http::StatusCode, web::Data, HttpRequest, HttpResponse,
};
use mime::Mime;

use dweb::files::directory::get_content_using_hex;
use dweb::web::fetch::response_with_body;

use super::helpers::*;
use crate::web::etag;

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
    is_main_server: Data<bool>,
    client: Data<dweb::client::DwebClient>,
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

    return make_error_response_page(
        Some(StatusCode::NOT_FOUND),
        &mut HttpResponse::NotFound(),
        "main dweb server error (I'm not main server!)".to_string(),
        &format!("- check the URL is a valid API"),
    );
}
