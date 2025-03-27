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

pub mod data;
pub mod directory;
pub mod file;
pub mod name;

use actix_web::{get, http::header, HttpRequest, HttpResponse, Responder};

/// Get the proxy identifier and version of the dweb API
#[utoipa::path(
    responses(
        (status = 200, description = "Returns the base route for the dweb APIs (e.g. '/dweb-0').
        This identifies the server as the 'dweb' proxy, and the version of the dweb API being served (e.g. '0').", body = str)

    ),
    tags = ["manual", dweb::api::DWEB_API_ROUTE],
)]
#[get("/ant-proxy-id")]
pub async fn ant_proxy_id(_request: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_PLAIN))
        .body(dweb::api::DWEB_API_ROUTE)
}
