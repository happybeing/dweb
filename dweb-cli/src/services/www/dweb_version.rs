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

use actix_web::{dev::HttpServiceFactory, get, web, web::Data, HttpRequest, HttpResponse};

use dweb::cache::directory_with_port::*;
use dweb::web::fetch::response_redirect;
use dweb::web::LOCALHOST_STR;

use super::make_error_response;

pub fn init_dweb_version() -> impl HttpServiceFactory {
    actix_web::web::scope("/dweb-version").service(dweb_version)
}

/// /dweb-version/[VERSION]
///
/// Opens the specified VERSION of the current site. The first version is 1.
///
/// To open the most recent version use VERSION 'latest'.
///
/// Examples (replace <PORT> with the port currently in the address bar):
///
/// Switch to version 3: http://127.0.0.1:<PORT>/dweb-version/3
///
/// Switch to most recent: http://127.0.0.1:<PORT>/dweb-version/latest
#[get("/{version}")]
pub async fn dweb_version(
    request: HttpRequest,
    version: web::Path<String>,
    _client: Data<dweb::client::DwebClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    _is_local_network: Data<bool>,
) -> HttpResponse {
    let version = if version.as_str() == "latest" {
        ""
    } else {
        version.as_str()
    };

    println!("DEBUG dweb_version(v{version})...");

    if our_directory_version.is_some() {
        let directory_version = our_directory_version.as_ref().clone().unwrap();
        println!("DEBUG {directory_version}");

        if let Some(history_address) = directory_version.history_address {
            let url_path = format!("/dweb-open/v{version}/{}", history_address.to_hex());
            let host = request.uri().host().unwrap_or(LOCALHOST_STR);
            response_redirect(&request, host, None, Some(url_path))
        } else {
            return make_error_response(
                None,
                &mut HttpResponse::InternalServerError(),
                "/dweb_version handler error".to_string(),
                &format!("You cannot select a version as this is a directory not a History"),
            );
        }
    } else {
        return make_error_response(
            None,
            &mut HttpResponse::InternalServerError(),
            "/dweb_version handler error".to_string(),
            &format!("Unable to access our_directory_version - probably a bug"),
        );
    }
}
