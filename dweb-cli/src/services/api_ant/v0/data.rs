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

// use actix_multipart::form::bytes::Bytes;
use actix_web::{
    get,
    http::header::ContentType,
    http::StatusCode,
    post,
    web::{self, Data},
    HttpRequest, HttpResponse, HttpResponseBuilder,
};
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::{Chunk, ChunkAddress};

use dweb::helpers::{convert::*, retry::retry_until_ok};
use dweb::storage::DwebType;

use crate::services::api_dweb::v0::PutResult;
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
pub async fn data_get(
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
fn avoid_comment_error1() {}

/// Get a chunk from the network using a hex encoded chunk address
#[utoipa::path(
    responses(
        (status = 200, description = "Success"),
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

    let chunk_address = match ChunkAddress::try_from_hex(&chunk_address) {
        Ok(address) => address,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/chunk error".to_string(),
                &format!("/chunk not a chunk address: '{chunk_address}' - {e}"),
            );
        }
    };

    let content = match client.client.chunk_get(&chunk_address).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/chunk error".to_string(),
                &format!("/chunk failed to get chunk from network - {e}"),
            );
        }
    };

    HttpResponseBuilder::new(StatusCode::OK).body(content.value)
}

/// Store a chunk (aka libp2p record) on the network
///
///
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each file upload, 0 means unlimited. This overrides the API control setting in the server.")),
    request_body(content = DwebChunk, content_type = "application/json"),
    responses(
        (status = 200, description = "Success"),
        (status = StatusCode::BAD_REQUEST, description = "The chunk_address is not valid"),
        (status = 413, description = "The POST request body content was too large"),
        ),
    responses(
        (status = 200, description = "A PutResult featuring either status 200 with cost and data address on the network, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>\
        &nbsp;&nbsp;&nbsp;413 CONTENT_TOO_LARGE: The POST request body content was too large<br/>", body = PutResult,
            example = json!("{\"file_name\": \"\", \"status\": \"200\", \"cost_in_attos\": \"12\", \"data_address\": \"a9cd8dd0c9f2b9dc71ad548d1f37fcba6597d5eb1be0b8c63793802cc6c7de27\", \"data_map\": \"\", \"message\": \"\" }")),
    ),
    tags = ["Autonomi"],
)]
#[post("/chunk")]
pub async fn chunk_post(
    request: HttpRequest,
    chunk: web::Json<DwebChunk>,
    query_params: web::Query<QueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let tries = query_params.tries.unwrap_or(client.api_control.tries);

    let chunk = Chunk::new(chunk.into_inner().content.clone().into());

    if chunk.is_too_big() {
        let status_code = StatusCode::from_u16(413).unwrap_or(StatusCode::BAD_REQUEST);
        let status_message = format!(
            "/chunk POST failed because request body exceeds Chunk::DEFAULT_MAX_SIZE ({} bytes)",
            Chunk::DEFAULT_MAX_SIZE
        );
        println!("DEBUG {status_message}");
        return make_error_response_page(
            Some(status_code),
            &mut HttpResponseBuilder::new(status_code),
            "/chunk POST error".to_string(),
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

    let put_result = match result {
        Ok(result) => {
            println!(
                "DEBUG chunk_post() stored Chunk on the network at address {}",
                chunk.address()
            );
            let mut put_result = PutResult::new(
                DwebType::Chunk,
                StatusCode::OK,
                "success".to_string(),
                result.0,
            );

            put_result.data_address = result.1.to_hex();
            put_result
        }
        Err(e) => {
            let status_message =
                format!("put_archive_private() failed store PrivateArchive on the network - {e}");
            println!("DEBUG {status_message}");
            // return PutResult::new(
            //     DwebType::Chunk,
            //     StatusCode::BAD_GATEWAY,
            //     status_message,
            //     AttoTokens::zero(),
            // );
            return make_error_response_page(
                Some(StatusCode::BAD_GATEWAY),
                &mut HttpResponse::BadGateway(),
                "/chunk POST error".to_string(),
                &format!("/chunk POST failed to encode JSON result - {e}"),
            );
        }
    };

    let json = match serde_json::to_string(&put_result) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                "/archive-public POST error".to_string(),
                &format!("archive::post_public() failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG put_result as JSON: {json:?}");
    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

#[derive(Deserialize, ToSchema)]
struct QueryParams {
    tries: Option<u32>,
}

/// A representation of the Autonomi PublicArchive for web clients
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DwebChunk {
    content: String,
}
