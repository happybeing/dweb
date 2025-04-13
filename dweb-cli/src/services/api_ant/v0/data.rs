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

/// Get data from the network using a hex encoded datamap or data address
#[utoipa::path(
    responses(
        (status = 200, description = "Success"),
        (status = StatusCode::BAD_REQUEST, description = "The datamap_or_address is not a valid address"),
        (status = StatusCode::NOT_FOUND, description = "The data was not found or a network error occured"),
        ),
    tags = ["Autonomi"],
    params(
        ("datamap_or_address", description = "the hex encoded datamap or data address of public or private data"),
    )
)]
#[get("/data/{datamap_or_address}")]
pub async fn get(
    request: HttpRequest,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let (datamap_chunk, _, data_address) = tuple_from_datamap_address_or_name(&params.into_inner());

    let content = if datamap_chunk.is_some() {
        match client.client.data_get(&datamap_chunk.unwrap()).await {
            Ok(bytes) => bytes,
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    "/data error".to_string(),
                    &format!("/data failed to get file from network - {e}"),
                );
            }
        }
    } else if data_address.is_some() {
        match client.client.data_get_public(&data_address.unwrap()).await {
            Ok(bytes) => bytes,
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    "/data error".to_string(),
                    &format!("/data failed to get file from network - {e}"),
                );
            }
        }
    } else {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            "/data error".to_string(),
            "/data datamap_or_address not valid",
        );
    };

    HttpResponseBuilder::new(StatusCode::OK).body(content)
}

/// Put data to the network including its datamap
///
/// Note: for large datasets see the API: /dweb-0/form-upload-file-list
///
/// TODO /data-public POST
fn avoid_comment_error() {}
