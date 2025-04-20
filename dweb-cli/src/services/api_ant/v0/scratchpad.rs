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
    body::MessageBody,
    get,
    http::{header::ContentType, StatusCode},
    post, put,
    web::{self, Data},
    HttpRequest, HttpResponse,
};
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::ScratchpadAddress;

use dweb::helpers::retry::retry_until_ok;
use dweb::token::Spends;
use dweb::types::scratchpad_secret_key_from_owner;
use dweb::{storage::DwebType, types::derive_named_object_secret};

use crate::services::api_dweb::v0::{
    process_header_and_query_params, MutateQueryParams, MutateResult,
};
use crate::services::helpers::*;

const REST_TYPE: &str = "Scratchpad";

/// Get a Scratchpad from the network using a hex encoded ScratchpadAddress
/// TODO example JSON
#[utoipa::path(
    params(("scratchpad_address" = String, Path, description = "the hex encoded address of a Scratchpad on the network"),),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebScratchpad]),
        (status = StatusCode::BAD_REQUEST, description = "The scratchpad address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The scratchpad was not found or a network error occured"),
        ),
    tags = ["Autonomi"],
)]
#[get("/scratchpad/{scratchpad_address}")]
pub async fn scratchpad_get(
    request: HttpRequest,
    scratchpad_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/scratchpad GET error";
    let rest_handler = "scratchpad_get()";

    let scratchpad_address = ScratchpadAddress::from_hex(&scratchpad_address.into_inner());

    let scratchpad = match scratchpad_address {
        Ok(scratchpad_address) => {
            println!(
                "DEBUG {rest_operation} calling client.scratchpad_get({})",
                scratchpad_address.to_hex()
            );
            match client.client.scratchpad_get(&scratchpad_address).await {
                Ok(scratchpad) => scratchpad,
                Err(e) => {
                    return make_error_response_page(
                        None,
                        &mut HttpResponse::NotFound(),
                        rest_operation.to_string(),
                        &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
                    );
                }
            }
        }
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::BAD_REQUEST),
                &mut HttpResponse::BadRequest(),
                "/scratchpad GET error".to_string(),
                &format!("/scratchpad GET failed due to invalid {REST_TYPE} address - {e}"),
            )
        }
    };

    let dweb_scratchpad = DwebScratchpad {
        scratchpad_address: scratchpad.address().to_hex(),
        data_encoding: scratchpad.data_encoding(),
        encryped_data: scratchpad.encrypted_data().to_vec(),
        counter: scratchpad.counter(),
        ..Default::default()
    };

    let json = match serde_json::to_string(&dweb_scratchpad) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!("{rest_handler} failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG DwebScratchpad as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Get a Scratchpad you own with optional name
/// TODO example JSON
#[utoipa::path(
    params(
        ("name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Scratchpad-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret")),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebScratchpad]),
        (status = StatusCode::BAD_REQUEST, description = "The scratchpad address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The scratchpad was not found or a network error occured"),
        ),
    tags = ["Autonomi"],
)]
#[get("/scratchpad")]
pub async fn scratchpad_get_owned(
    query_params: web::Query<MutateQueryParams>,
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let client = &client.into_inner();
    let (_tries, scratchpad_name) =
        process_header_and_query_params(&client, request.headers(), &mut query_params.into_inner());

    let rest_operation = "/scratchpad GET error";
    let rest_handler = "scratchpad_get_owned()";

    // TODO use separate owner_secret from DwebClient when available
    let owner_secret = match dweb::helpers::get_app_secret_key() {
        Ok(secret_key) => secret_key,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::InternalServerError(),
                rest_operation.to_string(),
                &format!("{rest_handler} failed to get owner secret for {REST_TYPE} - {e}"),
            );
        }
    };

    let scratchpad_secret = derive_named_object_secret(
        scratchpad_secret_key_from_owner(owner_secret),
        scratchpad_name,
    );
    let scratchpad_address = ScratchpadAddress::new(scratchpad_secret.public_key());

    let scratchpad = match client.client.scratchpad_get(&scratchpad_address).await {
        Ok(scratchpad) => scratchpad,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
            );
        }
    };

    let dweb_scratchpad = DwebScratchpad {
        scratchpad_address: scratchpad.address().to_hex(),
        ..Default::default()
    };

    let json = match serde_json::to_string(&dweb_scratchpad) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!("{rest_handler} failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG DwebScratchpad as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Create a new Scratchpad on the network
///
/// Note: This implementation differs from the Autonomi APIs in that you can have
/// any number of scratchpads with the same owner but different names, and these will
/// not clash with other types also using the same owner.
///
/// TODO example JSON
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Scratchpad-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret")),
    request_body(content = DwebScratchpad, content_type = "application/json"),
    responses(
        (status = StatusCode::CREATED, description = "A MutateResult featuring either status 201 with cost and the network address of the created Scratchpad, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Autonomi"],
)]
#[post("/scratchpad")]
pub async fn scratchpad_post(
    request: HttpRequest,
    scratchpad: web::Json<DwebScratchpad>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let client = &client.into_inner();
    let (tries, scratchpad_name) =
        process_header_and_query_params(&client, request.headers(), &mut query_params.into_inner());

    let rest_operation = "/scratchpad POST".to_string();
    let rest_handler = "scratchpad_post()";
    let dweb_type = DwebType::Scratchpad;

    // TODO use separate owner_secret from DwebClient when available
    let owner_secret = match dweb::helpers::get_app_secret_key() {
        Ok(secret_key) => secret_key,
        Err(e) => {
            return MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                status_message: format!("{rest_handler} failed to load secret key - {e}"),
                ..Default::default()
            }
            .response(rest_handler);
        }
    };

    let payment_option = client.payment_option().clone();

    let scratchpad_secret = derive_named_object_secret(
        scratchpad_secret_key_from_owner(owner_secret),
        scratchpad_name,
    );
    let content_type = scratchpad.data_encoding;

    let initial_data = match scratchpad.unencryped_data.clone().try_into_bytes() {
        Ok(bytes) => bytes,
        Err(_e) => {
            return MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::BAD_REQUEST.as_u16(),
                status_message: format!(
                    "{rest_handler} unencrypted data failed to convert to Bytes"
                ),
                ..Default::default()
            }
            .response(rest_handler);
        }
    };

    let spends = Spends::new(&client, None).await;
    let result = retry_until_ok(
        tries,
        &rest_operation,
        (
            scratchpad_secret,
            content_type,
            initial_data,
            payment_option,
        ),
        async move |(scratchpad_secret, content_type, initial_data, payment_option)| match client
            .client
            .scratchpad_create(
                &scratchpad_secret,
                content_type,
                &initial_data,
                payment_option,
            )
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
                result.1
            );
            let (cost_in_ant, cost_in_arb_eth) = match spends {
                Ok(spends) => {
                    let (cost_in_ant, cost_in_arb_eth) = spends.get_spend_strings().await;
                    println!("DEBUG {rest_operation} cost in ANT  : {cost_in_ant}");
                    println!("DEBUG {rest_operation} cost in ARB-ETH: {cost_in_arb_eth}");
                    (cost_in_ant, cost_in_arb_eth)
                }
                Err(e) => {
                    println!("DEBUG {rest_operation} error: unable to report Spends - {e}");
                    ("unkown".to_string(), "unknown".to_string())
                }
            };

            MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::CREATED.as_u16(),
                status_message: "success".to_string(),
                cost_in_ant,
                cost_in_arb_eth,
                network_address: result.1.to_hex(),
                ..Default::default()
            }
            .response(rest_handler)
        }

        Err(e) => {
            let status_message = format!("failed store {REST_TYPE} on the network - {e}");
            println!("DEBUG {status_message}");
            MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                status_message,
                ..Default::default()
            }
            .response(rest_handler)
        }
    }
}

/// Update an existing Scratchpad on the network
/// TODO example JSON
#[utoipa::path(
    put,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Scratchpad-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret")),
    request_body(content = DwebScratchpad, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "A MutateResult featuring either status 200 with cost and the network address of the created Scratchpad, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Autonomi"],
)]
#[put("/scratchpad")]
pub async fn scratchpad_put(
    request: HttpRequest,
    scratchpad: web::Json<DwebScratchpad>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let client = &client.into_inner();
    let (tries, scratchpad_name) =
        process_header_and_query_params(&client, request.headers(), &mut query_params.into_inner());

    let rest_operation = "/scratchpad PUT".to_string();
    let rest_handler = "scratchpad_put()";
    let dweb_type = DwebType::Scratchpad;
    let dweb_scratchpad = scratchpad.into_inner();

    // TODO use separate owner_secret from DwebClient when available
    let owner_secret = match dweb::helpers::get_app_secret_key() {
        Ok(secret_key) => secret_key,
        Err(e) => {
            return MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                status_message: format!("{rest_handler} failed to load secret key - {e}"),
                ..Default::default()
            }
            .response(rest_handler);
        }
    };

    let scratchpad_secret = derive_named_object_secret(
        scratchpad_secret_key_from_owner(owner_secret),
        scratchpad_name,
    );
    let content_type = dweb_scratchpad.data_encoding;

    let new_data = match dweb_scratchpad.unencryped_data.try_into_bytes() {
        Ok(bytes) => bytes,
        Err(_e) => {
            return MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::BAD_REQUEST.as_u16(),
                status_message: format!(
                    "{rest_handler} unencrypted data failed to convert to Bytes"
                ),
                ..Default::default()
            }
            .response(rest_handler);
        }
    };

    let result = retry_until_ok(
        tries,
        &rest_handler,
        (scratchpad_secret, content_type, new_data),
        async move |(scratchpad_secret, content_type, new_data)| match client
            .client
            .scratchpad_update(&scratchpad_secret, content_type, &new_data)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(eyre!(e)),
        },
    )
    .await;

    match result {
        Ok(_) => {
            println!("DEBUG {rest_handler} stored {REST_TYPE} on the network",);
            MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::OK.as_u16(),
                status_message: "success".to_string(),
                ..Default::default()
            }
            .response(rest_handler)
        }

        Err(e) => {
            let status_message = format!("failed store {REST_TYPE} on the network - {e}");
            println!("DEBUG {status_message}");
            MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                status_message,
                ..Default::default()
            }
            .response(rest_handler)
        }
    }
}

/// A representation of the Autonomi Scratchpad for web clients
///
/// Exactly one target is allowed, so make sure unused targets are empty strings
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DwebScratchpad {
    scratchpad_address: String,
    data_encoding: u64,
    encryped_data: Vec<u8>,
    unencryped_data: Vec<u8>,
    counter: u64,
}

impl Default for DwebScratchpad {
    fn default() -> DwebScratchpad {
        DwebScratchpad {
            scratchpad_address: "".to_string(),
            counter: 0,
            data_encoding: 0,
            encryped_data: Vec::<u8>::new(),
            unencryped_data: Vec::<u8>::new(),
        }
    }
}
