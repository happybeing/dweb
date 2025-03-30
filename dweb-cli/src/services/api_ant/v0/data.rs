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

use actix_web::{
    get,
    http::StatusCode,
    web::{self, Data},
    HttpRequest, HttpResponse, HttpResponseBuilder,
};

use dweb::helpers::convert::*;

use crate::services::helpers::*;

/// Get data from the network
///
#[utoipa::path(
    responses(
        (status = 200)
        ),
    tags = [dweb::api::ANT_API_ROUTE],
    params(
        ("data_address", description = "The hexadecimal address of a datamap on Autonomi"),
    ),
)]
#[get("/data-public/{data_address}")]
pub async fn data_get_public(
    request: HttpRequest,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let data_address = match str_to_data_address(&params.into_inner()) {
        Ok(data_address) => data_address,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/data error".to_string(),
                &format!("/data address not valid - {e}"),
            );
        }
    };

    let content = match client.data_get_public(data_address).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/data error".to_string(),
                &format!("/data failed to get file from network - {e}"),
            );
        }
    };

    HttpResponseBuilder::new(StatusCode::OK).body(content)
}
