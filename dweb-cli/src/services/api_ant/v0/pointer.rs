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
    http::{header::ContentType, StatusCode},
    post, put,
    web::{self, Data},
    HttpRequest, HttpResponse,
};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::{
    pointer::PointerTarget, AddressParseError, ChunkAddress, GraphEntryAddress, Pointer,
    PointerAddress, ScratchpadAddress,
};

use dweb::helpers::retry::retry_until_ok;
use dweb::token::Spends;
use dweb::types::POINTER_DERIVATION_INDEX;
use dweb::{storage::DwebType, types::derive_named_object_secret};

use crate::services::api_dweb::v0::{MutateQueryParams, MutateResult, ParsedRequestParams};
use crate::services::helpers::*;

const REST_TYPE: &str = "Pointer";

/// Get a Pointer from the network using a hex encoded PointerAddress
///
/// TODO example JSON
#[utoipa::path(
    params(("pointer_address" = String, Path, description = "the hex encoded address of a Pointer on the network"),),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one pointer per owner secret")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebPointer]),
        (status = StatusCode::BAD_REQUEST, description = "The pointer address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The pointer was not found or a network error occured"),
        ),
    tags = ["Autonomi"],
)]
#[get("/pointer/{pointer_address}")]
pub async fn pointer_get(
    request: HttpRequest,
    pointer_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/pointer GET";
    let rest_handler = "pointer_get()";

    let pointer_address = PointerAddress::from_hex(&pointer_address.into_inner());

    let pointer = match pointer_address {
        Ok(pointer_address) => {
            println!(
                "DEBUG {rest_operation} calling client.pointer_get({})",
                pointer_address.to_hex()
            );
            match client.client.pointer_get(&pointer_address).await {
                Ok(pointer) => pointer,
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
                rest_operation.to_string(),
                &format!("/pointer GET failed due to invalid {REST_TYPE} address - {e}"),
            )
        }
    };

    let dweb_pointer = DwebPointer {
        pointer_address: pointer.address().to_hex(),
        counter: pointer.counter(),
        chunk_target_address: pointer.target().to_hex(),
        ..Default::default()
    };

    let json = match serde_json::to_string(&dweb_pointer) {
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

    println!("DEBUG DwebPointer as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Get a Pointer you own with optional name
/// TODO example JSON
#[utoipa::path(
    params(
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one pointer per owner secret")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one pointer per owner secret")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebPointer]),
        (status = StatusCode::BAD_REQUEST, description = "The pointer address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The pointer was not found or a network error occured"),
        ),
    tags = ["Autonomi"],
)]
#[get("/pointer")]
pub async fn pointer_get_owned(
    query_params: web::Query<MutateQueryParams>,
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/pointer GET";
    let rest_handler = "pointer_get_owned()";

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_header_and_mutate_query_params(
        &client,
        request.headers(),
        &mut query_params.into_inner(),
    ) {
        Ok(params) => params,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::BAD_REQUEST),
                &mut HttpResponse::BadRequest(),
                rest_operation.to_string(),
                &format!("{rest_operation} request error - {e}"),
            );
        }
    };

    // TODO use separate owner_secret from DwebClient when available
    let owner_secret =
        request_params
            .owner_secret
            .unwrap_or(match dweb::helpers::get_app_secret_key() {
                Ok(secret_key) => secret_key,
                Err(e) => {
                    return make_error_response_page(
                        Some(StatusCode::INTERNAL_SERVER_ERROR),
                        &mut HttpResponse::InternalServerError(),
                        rest_operation.to_string(),
                        &format!("{rest_handler} failed to get owner secret for {REST_TYPE} - {e}"),
                    );
                }
            });

    let pointer_secret = derive_named_object_secret(
        owner_secret,
        POINTER_DERIVATION_INDEX,
        &request_params.type_derivation_index,
        request_params.object_name,
    );

    let pointer_address = PointerAddress::new(pointer_secret.public_key());

    let pointer = match client.client.pointer_get(&pointer_address).await {
        Ok(pointer) => pointer,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
            );
        }
    };

    let dweb_pointer = DwebPointer {
        pointer_address: pointer.address().to_hex(),
        counter: pointer.counter(),
        chunk_target_address: pointer.target().to_hex(),
        ..Default::default()
    };

    let json = match serde_json::to_string(&dweb_pointer) {
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

    println!("DEBUG DwebPointer as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Create a new Pointer on the network
///
/// Note: This implementation differs from the Autonomi APIs in that you can have
/// any number of pointers with the same owner but different names, and these will
/// not clash with other types also using the same owner.
///
/// TODO example JSON
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one pointer per owner secret")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one pointer per owner secret")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    request_body(content = DwebPointer, content_type = "application/json"),
    responses(
        (status = StatusCode::CREATED, description = "A MutateResult featuring either status 201 with cost and the network address of the created Pointer, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Autonomi"],
)]
#[post("/pointer")]
pub async fn pointer_post(
    request: HttpRequest,
    pointer: web::Json<DwebPointer>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/pointer POST".to_string();
    let rest_handler = "pointer_post()";
    let dweb_type = DwebType::Pointer;

    println!("DEBUG REST query_params: {query_params:?}");
    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_header_and_mutate_query_params(
        &client,
        request.headers(),
        &mut query_params.into_inner(),
    ) {
        Ok(params) => params,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::BAD_REQUEST),
                &mut HttpResponse::BadRequest(),
                rest_operation.to_string(),
                &format!("{rest_operation} request error - {e}"),
            );
        }
    };

    // TODO use separate owner_secret from DwebClient when available
    let owner_secret =
        request_params
            .owner_secret
            .unwrap_or(match dweb::helpers::get_app_secret_key() {
                Ok(secret_key) => secret_key,
                Err(e) => {
                    return make_error_response_page(
                        Some(StatusCode::INTERNAL_SERVER_ERROR),
                        &mut HttpResponse::InternalServerError(),
                        rest_operation.to_string(),
                        &format!("{rest_handler} failed to get owner secret for {REST_TYPE} - {e}"),
                    );
                }
            });

    let dweb_pointer = pointer.into_inner();
    let target = match dweb_pointer.pointer_target() {
        Ok(target) => target,
        Err(e) => {
            return MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::BAD_REQUEST.as_u16(),
                status_message: format!("{rest_handler} failed - {e}"),
                ..Default::default()
            }
            .response(rest_handler);
        }
    };

    let payment_option = client.payment_option().clone();

    let pointer_secret = derive_named_object_secret(
        owner_secret,
        POINTER_DERIVATION_INDEX,
        &request_params.type_derivation_index,
        request_params.object_name,
    );

    let spends = Spends::new(&client, None).await;
    let result = retry_until_ok(
        request_params.tries,
        &rest_operation,
        (pointer_secret, target, payment_option),
        async move |(pointer_secret, target, payment_option)| match client
            .client
            .pointer_create(&pointer_secret, target, payment_option)
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

/// Update an existing Pointer on the network
/// TODO example JSON
#[utoipa::path(
    put,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one pointer per owner secret")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one pointer per owner secret")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    request_body(content = DwebPointer, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "A MutateResult featuring either status 200 with cost and the network address of the created Pointer, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Autonomi"],
)]
#[put("/pointer")]
pub async fn pointer_put(
    request: HttpRequest,
    pointer: web::Json<DwebPointer>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/pointer PUT".to_string();
    let rest_handler = "pointer_put()";
    let dweb_type = DwebType::Pointer;
    let dweb_pointer = pointer.into_inner();

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_header_and_mutate_query_params(
        &client,
        request.headers(),
        &mut query_params.into_inner(),
    ) {
        Ok(params) => params,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::BAD_REQUEST),
                &mut HttpResponse::BadRequest(),
                rest_operation.to_string(),
                &format!("{rest_operation} request error - {e}"),
            );
        }
    };

    // TODO use separate owner_secret from DwebClient when available
    let owner_secret =
        request_params
            .owner_secret
            .unwrap_or(match dweb::helpers::get_app_secret_key() {
                Ok(secret_key) => secret_key,
                Err(e) => {
                    return make_error_response_page(
                        Some(StatusCode::INTERNAL_SERVER_ERROR),
                        &mut HttpResponse::InternalServerError(),
                        rest_operation.to_string(),
                        &format!("{rest_handler} failed to get owner secret for {REST_TYPE} - {e}"),
                    );
                }
            });

    let target = match dweb_pointer.pointer_target() {
        Ok(target) => target,
        Err(e) => {
            return MutateResult {
                rest_operation,
                dweb_type,
                status_code: StatusCode::BAD_REQUEST.as_u16(),
                status_message: format!("{rest_handler} failed - {e}"),
                ..Default::default()
            }
            .response(rest_handler);
        }
    };

    let payment_option = client.payment_option().clone();

    let pointer_secret = derive_named_object_secret(
        owner_secret,
        POINTER_DERIVATION_INDEX,
        &request_params.type_derivation_index,
        request_params.object_name,
    );

    let pointer = Pointer::new(&pointer_secret, dweb_pointer.counter, target);

    let spends = Spends::new(&client, None).await;
    let result = retry_until_ok(
        request_params.tries,
        &rest_handler,
        (pointer, payment_option),
        async move |(pointer, payment_option)| match client
            .client
            .pointer_put(pointer, payment_option)
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
                status_code: StatusCode::OK.as_u16(),
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

/// A representation of the Autonomi Pointer for web clients
///
/// Exactly one target is allowed, so make sure unused targets are empty strings
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DwebPointer {
    pointer_address: String,
    counter: u32,
    /// Only one target is permitted per pointer, each for a different type. Unused targets should be empty strings
    chunk_target_address: String,
    graphentry_target_address: String,
    pointer_target_address: String,
    scratchpad_target_address: String,
}

impl Default for DwebPointer {
    fn default() -> DwebPointer {
        DwebPointer {
            pointer_address: "".to_string(),
            counter: 0,
            chunk_target_address: "".to_string(),
            graphentry_target_address: "".to_string(),
            pointer_target_address: "".to_string(),
            scratchpad_target_address: "".to_string(),
        }
    }
}

impl DwebPointer {
    pub fn pointer_target(&self) -> Result<PointerTarget> {
        if let Ok(target_address) = self.chunk_target_address() {
            Ok(PointerTarget::ChunkAddress(target_address))
        } else if let Ok(target_address) = self.graphentry_target_address() {
            Ok(PointerTarget::GraphEntryAddress(target_address))
        } else if let Ok(target_address) = self.pointer_target_address() {
            Ok(PointerTarget::PointerAddress(target_address))
        } else if let Ok(target_address) = self.scratchpad_target_address() {
            Ok(PointerTarget::ScratchpadAddress(target_address))
        } else {
            Err(eyre!("missing or invalid Pointer target"))
        }
    }

    pub fn chunk_target_address(&self) -> Result<ChunkAddress> {
        Self::into_result(|| ChunkAddress::from_hex(&self.chunk_target_address))
    }

    pub fn graphentry_target_address(&self) -> Result<GraphEntryAddress> {
        Self::into_result(|| GraphEntryAddress::from_hex(&self.graphentry_target_address))
    }

    pub fn pointer_target_address(&self) -> Result<PointerAddress> {
        Self::into_result(|| PointerAddress::from_hex(&self.pointer_target_address))
    }

    pub fn scratchpad_target_address(&self) -> Result<ScratchpadAddress> {
        Self::into_result(|| ScratchpadAddress::from_hex(&self.scratchpad_target_address))
    }

    fn into_result<F, A>(f: F) -> Result<A>
    where
        F: Fn() -> Result<A, AddressParseError>,
    {
        match f() {
            Ok(address) => Ok(address),
            Err(e) => Err(eyre!(e)),
        }
    }
}
