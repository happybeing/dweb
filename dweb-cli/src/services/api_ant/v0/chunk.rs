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
    post,
    web::{self, Data},
    HttpRequest, HttpResponse, HttpResponseBuilder,
};
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::{Chunk, ChunkAddress};

use dweb::helpers::retry::retry_until_ok;
use dweb::storage::DwebType;

use crate::services::api_dweb::v0::MutateResult;
use crate::services::helpers::*;

const REST_TYPE: &str = "Chunk";

/// Put data to the network including its datamap
///
/// Note: for large datasets see the API: /dweb-0/form-upload-file-list
///
/// TODO /data-public POST
fn avoid_comment_error1() {}

/// Get a chunk from the network using a hex encoded chunk address
///
/// TODO update example JSON
#[utoipa::path(
    responses(
        (status = StatusCode::OK, description = "Success"),
        (status = StatusCode::BAD_REQUEST, description = "The chunk_address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The data was not found or a network error occured"),
        ),
    tags = ["Autonomi"],
    params(
        ("chunk_address", description = "the hex encoded address of a chunk (aka 'record' in libp2p)"),
    )
)]
#[get("/chunk/{chunk_address}")]
pub async fn chunk_get(
    request: HttpRequest,
    chunk_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/chunk GET";
    let rest_handler = "chunk_get()";

    let chunk_address = match ChunkAddress::try_from_hex(&chunk_address) {
        Ok(address) => address,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                rest_operation.to_string(),
                &format!("{rest_operation} {rest_handler} error not a chunk address: '{chunk_address}' - {e}"),
            );
        }
    };

    let content = match client.client.chunk_get(&chunk_address).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!(
                    "{rest_operation} {rest_handler} failed to get {REST_TYPE} from network - {e}"
                ),
            );
        }
    };

    HttpResponseBuilder::new(StatusCode::OK).body(content.value)
}

/// Store a chunk (aka libp2p record) on the network
///
/// TODO update example JSON
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each upload, 0 means unlimited. This overrides the API control setting in the server.")),
    request_body(content = Vec<u8>),
    responses(
        (status = StatusCode::OK, description = "Success"),
        (status = StatusCode::BAD_REQUEST, description = "The chunk_address is not valid"),
        (status = 413, description = "The POST request body content was too large"),
        ),
    responses(
        (status = StatusCode::CREATED, description = "A MutateResult featuring either status 201 with cost and data address on the network, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>\
        &nbsp;&nbsp;&nbsp;413 CONTENT_TOO_LARGE: The POST request body content was too large<br/>", body = MutateResult,
            example = json!("{\"file_name\": \"\", \"status\": \"201\", \"cost_in_attos\": \"12\", \"data_address\": \"a9cd8dd0c9f2b9dc71ad548d1f37fcba6597d5eb1be0b8c63793802cc6c7de27\", \"data_map\": \"\", \"message\": \"\" }")),
    ),
    tags = ["Autonomi"],
)]
#[post("/chunk")]
pub async fn chunk_post(
    request: HttpRequest,
    body: web::Bytes,
    query_params: web::Query<QueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/chunk POST";
    let rest_handler = "chunk_post()";
    let dweb_type = DwebType::Chunk;

    let tries = query_params.tries.unwrap_or(client.api_control.tries);
    let chunk = Chunk::new(body);

    if chunk.is_too_big() {
        let status_code = StatusCode::from_u16(413).unwrap_or(StatusCode::BAD_REQUEST);
        let status_message = format!(
            "{rest_operation} failed because request body exceeds Chunk::DEFAULT_MAX_SIZE ({} bytes)",
            Chunk::DEFAULT_MAX_SIZE
        );
        println!("DEBUG {status_message}");
        return make_error_response_page(
            Some(status_code),
            &mut HttpResponseBuilder::new(status_code),
            "{rest_operation} error".to_string(),
            &status_message,
        );
    }

    let client = &client;
    let payment_option = client.payment_option().clone();

    let result = retry_until_ok(
        tries,
        &"chunk_put()",
        (&chunk.clone(), payment_option),
        async move |(chunk, payment_option)| match client
            .client
            .chunk_put(chunk, payment_option)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(eyre!(e)),
        },
    )
    .await;

    match result {
        Ok(result) => {
            println!(
                "DEBUG {rest_handler} stored {REST_TYPE} on the network at address {}",
                chunk.address()
            );
            MutateResult {
                dweb_type,
                status_code: StatusCode::CREATED.as_u16(),
                status_message: "success".to_string(),
                network_address: result.1.to_hex(),
                ..Default::default()
            }
            .response(rest_handler)
        }
        Err(e) => {
            let status_message =
                format!("{rest_handler} failed store {REST_TYPE} on the network - {e}");
            println!("DEBUG {status_message}");
            MutateResult {
                dweb_type,
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                status_message,
                ..Default::default()
            }
            .response(rest_handler)
        }
    }
}

#[derive(Deserialize, ToSchema)]
struct QueryParams {
    tries: Option<u32>,
}

/// A representation of the Autonomi Chunk type for web clients
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DwebChunk {
    content: String,
}
