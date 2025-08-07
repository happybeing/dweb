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

use autonomi::{scratchpad::ScratchpadError, Bytes, Scratchpad, ScratchpadAddress, SecretKey};

use dweb::helpers::retry::retry_until_ok;
use dweb::storage::DwebType;
use dweb::token::Spends;
use dweb::types::{
    derive_named_object_secret, PRIVATE_SCRATCHPAD_DERIVATION_INDEX,
    PUBLIC_SCRATCHPAD_DERIVATION_INDEX,
};

use crate::services::api_dweb::v0::{MutateQueryParams, MutateResult, ParsedRequestParams};
use crate::services::helpers::*;

/// Get a private Scratchpad from the network using a hex encoded ScratchpadAddress
/// TODO example JSON
///
/// Attempts to decrypt the data with your owner secret. This will only work
/// if you did not create the scratchpad with a name. To get a named scratchpad
/// and decrypt its content, use the scratchpad GET without an address parameter.
#[utoipa::path(
    params(("scratchpad_address" = String, Path, description = "the hex encoded address of a Scratchpad on the network"),),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebScratchpad]),
        (status = StatusCode::BAD_REQUEST, description = "The scratchpad address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The scratchpad was not found or a network error occured"),
        ),
    tags = ["Dweb Autonomi"],
)]
#[get("/scratchpad-private/{scratchpad_address}")]
pub async fn scratchpad_private_get(
    request: HttpRequest,
    scratchpad_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "private Scratchpad";
    let rest_operation = "/scratchpad-private GET";
    let rest_handler = "scratchpad_private_get()";

    let scratchpad_address = ScratchpadAddress::from_hex(&scratchpad_address.into_inner());

    let scratchpad = match scratchpad_address {
        Ok(scratchpad_address) => {
            println!(
                "DEBUG {rest_operation} calling client.scratchpad_get({})",
                scratchpad_address.to_hex()
            );
            match client.client.scratchpad_get(&scratchpad_address).await {
                Ok(scratchpad) => scratchpad,
                Err(e) => match e {
                    ScratchpadError::Fork(scratchpads) => scratchpads[0].clone(),
                    e => {
                        return make_error_response_page(
                            None,
                            &mut HttpResponse::NotFound(),
                            rest_operation.to_string(),
                            &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
                        );
                    }
                },
            }
        }
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::BAD_REQUEST),
                &mut HttpResponse::BadRequest(),
                rest_operation.to_string(),
                &format!("{rest_operation} failed due to invalid {REST_TYPE} address - {e}"),
            )
        }
    };

    let mut dweb_scratchpad = DwebScratchpad {
        dweb_type: DwebType::PrivateScratchpad,
        scratchpad_address: scratchpad.address().to_hex(),
        data_encoding: scratchpad.data_encoding(),
        encrypted_data: scratchpad.encrypted_data().to_vec(),
        counter: scratchpad.counter(),
        ..Default::default()
    };

    // Attempt decryption. This will only work if the scratchpad was created
    // using this owner_secret and without an object_name.

    // TODO use separate owner_secret from DwebClient when available
    match dweb::helpers::get_app_secret_key() {
        Ok(owner_secret) => match scratchpad.decrypt_data(&derive_named_object_secret(owner_secret, PRIVATE_SCRATCHPAD_DERIVATION_INDEX, &None, None, None)) {
            Ok(bytes) => {
                dweb_scratchpad.unencrypted_data = bytes.to_vec();
                println!("DEBUG {rest_handler} decrypted data: {bytes:?}");
            },
            Err(_e) => println!("DEBUG {rest_handler} scratchpad decryption failed. This will fail if scratchpad was created with an object_name. In that case use the route which takes an object_name not scratchpad_address.")
        },
            Err(_e) => println!("DEBUG {rest_handler} unable to decrypt content - failed to get owner_secret")
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

    // println!("DEBUG DwebScratchpad as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

// TODO consider selecting Scratchpad based on modifying the following to:
// 1. filter by highest counter
// 2. choose the first scratchpad ordered by hash of content
//
// The above is simple and always chooses the same fork from a given set.
// TODO provide an API which returns an array, which is either the
// only Scratchpad returned, or all the forks returned. The client can
// then select manually.

/// @zettawatt's code from: https://github.com/zettawatt/colonylib/blob/b0d0ef8767cb0061d8965a4e9c0621b2986e0bf4/src/pod.rs#L1084
/// Selects the newest scratchpad from a vector of scratchpads based on timestamp comments.
///
/// This function reads the encrypted data from each scratchpad, looks for a timestamp comment
/// in the first line (format: #<RFC3339_timestamp>), and returns the scratchpad with the
/// latest timestamp. If only one scratchpad has a timestamp, it's assumed to be the newest.
/// If none have timestamps, the first scratchpad in the vector is returned.
///
/// # Parameters
///
/// * `scratchpads` - Vector of scratchpads to compare
///
/// # Returns
///
/// Returns the scratchpad with the latest timestamp, or the first one if no timestamps are found.
// fn select_newest_scratchpad(scratchpads: Vec<Scratchpad>) -> Scratchpad {
//     if scratchpads.is_empty() {
//         panic!("Cannot select from empty scratchpads vector");
//     }

//     if scratchpads.len() == 1 {
//         return scratchpads[0].clone();
//     }

//     let mut newest_scratchpad = &scratchpads[0];
//     let mut newest_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;

//     for scratchpad in &scratchpads {
//         // Extract the encrypted data and convert to string
//         let data = scratchpad.encrypted_data();
//         if let Ok(data_string) = String::from_utf8(data.to_vec()) {
//             // Check if the first line is a timestamp comment
//             if let Some(first_line) = data_string.lines().next() {
//                 if first_line.starts_with('#') && first_line.len() > 1 {
//                     let timestamp_str = &first_line[1..]; // Remove the '#' prefix

//                     // Try to parse the timestamp
//                     if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
//                         let utc_timestamp = timestamp.with_timezone(&chrono::Utc);

//                         // Check if this is the newest timestamp so far
//                         if newest_timestamp.is_none() || utc_timestamp > newest_timestamp.unwrap() {
//                             newest_timestamp = Some(utc_timestamp);
//                             newest_scratchpad = scratchpad;
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     // If we found at least one timestamp, return the newest one
//     // If no timestamps were found, return the first scratchpad
//     newest_scratchpad.clone()
// }

/// Get a private Scratchpad you own, with optional name
/// TODO example JSON
#[utoipa::path(
    params(
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination"),
        ("Ant-App-ID" = Option<String>, Header, description = "a unique string identifier for this app (as suggested by Autonomi and used to derive the VaultContentType used by an app)")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebScratchpad]),
        (status = StatusCode::BAD_REQUEST, description = "The scratchpad address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The scratchpad was not found or a network error occured"),
        ),
    tags = ["Dweb Autonomi"],
)]
#[get("/scratchpad-private")]
pub async fn scratchpad_private_get_owned(
    query_params: web::Query<MutateQueryParams>,
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "private Scratchpad";
    let rest_operation = "/scratchpad-private GET";
    let rest_handler = "scratchpad_private_get_owned()";

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_mutable_type_header_and_query_params(
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

    // This method contains the logic for determining which if any app ID is to
    // be used as well as deriving the object's owner secret.
    let scratchpad_secret =
        match request_params.derive_object_owner_secret(PRIVATE_SCRATCHPAD_DERIVATION_INDEX) {
            Ok(derived_secret) => derived_secret,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::BAD_REQUEST),
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to derive owner secret for {REST_TYPE} - {e}"),
                );
            }
        };

    let scratchpad_address = ScratchpadAddress::new(scratchpad_secret.public_key());

    let scratchpad = match client.client.scratchpad_get(&scratchpad_address).await {
        Ok(scratchpad) => scratchpad,
        Err(e) => match e {
            ScratchpadError::Fork(scratchpads) => scratchpads[0].clone(),
            e => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
                );
            }
        },
    };

    let mut dweb_scratchpad = DwebScratchpad {
        dweb_type: DwebType::PrivateScratchpad,
        scratchpad_address: scratchpad.address().to_hex(),
        data_encoding: scratchpad.data_encoding(),
        encrypted_data: scratchpad.encrypted_data().to_vec(),
        counter: scratchpad.counter(),
        ..Default::default()
    };

    match scratchpad.decrypt_data(&scratchpad_secret) {
        Ok(bytes) => {
            dweb_scratchpad.unencrypted_data = bytes.to_vec();
            println!("DEBUG {rest_operation} successfully decrypted scratchpad data");
        }
        Err(e) => {
            println!("DEBUG {rest_operation} failed to decrypt scratchpad data failed - {e}")
        }
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

    // println!("DEBUG DwebScratchpad as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Create a new private Scratchpad on the network
///
/// Notes:
/// - if you leave the Scratchpad content_type as 0 and provide an app ID, the content_type will be
/// set in the scratchpad to the value of app_name_to_vault_content_type(app_id + "-scratchpad-" + object_name).
///
/// - this implementation differs from the Autonomi APIs in that you can have
/// any number of scratchpads with the same owner but different names, and these will
/// not clash with other types also using the same owner.
///
/// TODO example JSON
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination"),
        ("Ant-App-ID" = Option<String>, Header, description = "a unique string identifier for this app (as suggested by Autonomi and used to derive the VaultContentType used by an app)")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    request_body(content = DwebScratchpad, content_type = "application/json"),
    responses(
        (status = StatusCode::CREATED, description = "A MutateResult featuring either status 201 with cost and the network address of the created Scratchpad, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Dweb Autonomi"],
)]
#[post("/scratchpad-private")]
pub async fn scratchpad_private_post(
    request: HttpRequest,
    scratchpad: web::Json<DwebScratchpad>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "private Scratchpad";
    let dweb_type = DwebType::PrivateScratchpad;
    let rest_operation = "/scratchpad-private POST".to_string();
    let rest_handler = "scratchpad_private_post()";

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_mutable_type_header_and_query_params(
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

    // This method contains the logic for determining which if any app ID is to
    // be used as well as deriving the object's owner secret.
    let scratchpad_secret =
        match request_params.derive_object_owner_secret(PRIVATE_SCRATCHPAD_DERIVATION_INDEX) {
            Ok(derived_secret) => derived_secret,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::BAD_REQUEST),
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to derive owner secret for {REST_TYPE} - {e}"),
                );
            }
        };

    let payment_option = client.payment_option().clone();
    let mut content_type = scratchpad.data_encoding;
    if content_type == 0 && request_params.app_id.is_some() {
        let object_name = request_params.object_name.unwrap_or("".to_string());
        let scratchpad_id_string = format!(
            "{}-scratchpad-{}",
            request_params.app_id.unwrap(),
            &object_name
        );
        content_type = autonomi::client::vault::app_name_to_vault_content_type(scratchpad_id_string)
    }

    println!(
        "DEBUG scratchpad.unencrypted_data: {:?}",
        scratchpad.unencrypted_data
    );
    let initial_data = match scratchpad.unencrypted_data.clone().try_into_bytes() {
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
        request_params.tries,
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
            Err(e) => {
                println!("DEBUG /scratchpad-private POST failed to create scratchpad - {e}");
                return Err(eyre!(e));
            }
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
                    println!("DEBUG {rest_operation} cost in ANT    : {cost_in_ant}");
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

/// Update an existing private Scratchpad on the network
/// TODO example JSON
#[utoipa::path(
    put,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination"),
        ("Ant-App-ID" = Option<String>, Header, description = "a unique string identifier for this app (as suggested by Autonomi and used to derive the VaultContentType used by an app)")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    request_body(content = DwebScratchpad, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "A MutateResult featuring either status 200 with cost and the network address of the created Scratchpad, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Dweb Autonomi"],
)]
#[put("/scratchpad-private")]
pub async fn scratchpad_private_put(
    request: HttpRequest,
    scratchpad: web::Json<DwebScratchpad>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "private Scratchpad";
    let dweb_type = DwebType::PrivateScratchpad;
    let rest_operation = "/scratchpad-private PUT".to_string();
    let rest_handler = "scratchpad_private_put()";
    let dweb_scratchpad = scratchpad.into_inner();

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_mutable_type_header_and_query_params(
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

    // This method contains the logic for determining which if any app ID is to
    // be used as well as deriving the object's owner secret.
    let scratchpad_secret =
        match request_params.derive_object_owner_secret(PRIVATE_SCRATCHPAD_DERIVATION_INDEX) {
            Ok(derived_secret) => derived_secret,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::BAD_REQUEST),
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to derive owner secret for {REST_TYPE} - {e}"),
                );
            }
        };

    let content_type = dweb_scratchpad.data_encoding;

    let new_data = match dweb_scratchpad.unencrypted_data.try_into_bytes() {
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

    let payment_option = client.payment_option().clone();

    let result = retry_until_ok(
        request_params.tries,
        &rest_handler,
        (scratchpad_secret.clone(), content_type, new_data.clone(), payment_option.clone(), client.client.clone()),
        async move |(scratchpad_secret, content_type, new_data, payment_option, client)| {
            match client
                .scratchpad_update(&scratchpad_secret, content_type, &new_data)
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => match e {
                    ScratchpadError::Fork(scratchpads) => {
                        let counter = scratchpads[0].counter() + 1;
                        let new_scratchpad = Scratchpad::new(&scratchpad_secret, content_type, &new_data, counter);
                        client
                            .scratchpad_put(new_scratchpad, payment_option)
                            .await
                            .map_err(|e| eyre!(e))?;
                        Ok(())
                    }
                    _ => Err(eyre!(e)),
                }
            }
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

///////////////////////////// Public Scratchpad ///////////////////////////////

/// Get a public Scratchpad from the network using a hex encoded ScratchpadAddress
/// TODO example JSON
///
/// Scratchpad data is assumed to be unencrypted
#[utoipa::path(
    params(("scratchpad_address" = String, Path, description = "the hex encoded address of a Scratchpad on the network"),),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebScratchpad]),
        (status = StatusCode::BAD_REQUEST, description = "The scratchpad address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The scratchpad was not found or a network error occured"),
        ),
    tags = ["Dweb Autonomi"],
)]
#[get("/scratchpad-public/{scratchpad_address}")]
pub async fn scratchpad_public_get(
    request: HttpRequest,
    scratchpad_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "public Scratchpad";
    let rest_operation = "/scratchpad-public GET";
    let rest_handler = "scratchpad_public_get()";

    let scratchpad_address = ScratchpadAddress::from_hex(&scratchpad_address.into_inner());

    let scratchpad = match scratchpad_address {
        Ok(scratchpad_address) => {
            println!(
                "DEBUG {rest_operation} calling client.scratchpad_get({})",
                scratchpad_address.to_hex()
            );
            match client.client.scratchpad_get(&scratchpad_address).await {
                Ok(scratchpad) => scratchpad,
                Err(e) => match e {
                    ScratchpadError::Fork(scratchpads) => scratchpads[0].clone(),
                    e => {
                        return make_error_response_page(
                            None,
                            &mut HttpResponse::NotFound(),
                            rest_operation.to_string(),
                            &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
                        );
                    }
                },
            }
        }
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::BAD_REQUEST),
                &mut HttpResponse::BadRequest(),
                rest_operation.to_string(),
                &format!("/scratchpad GET failed due to invalid {REST_TYPE} address - {e}"),
            )
        }
    };

    let dweb_scratchpad = DwebScratchpad {
        dweb_type: DwebType::PublicScratchpad,
        scratchpad_address: scratchpad.address().to_hex(),
        data_encoding: scratchpad.data_encoding(),
        unencrypted_data: scratchpad.encrypted_data().to_vec(),
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

    // println!("DEBUG DwebScratchpad as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Get a public Scratchpad you own, with optional name
/// TODO example JSON
///
/// Scratchpad data is assumed to be unencrypted
#[utoipa::path(
    params(
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination"),
        ("Ant-App-ID" = Option<String>, Header, description = "a unique string identifier for this app (as suggested by Autonomi and used to derive the VaultContentType used by an app)")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebScratchpad]),
        (status = StatusCode::BAD_REQUEST, description = "The scratchpad address is not valid"),
        (status = StatusCode::NOT_FOUND, description = "The scratchpad was not found or a network error occured"),
        ),
    tags = ["Dweb Autonomi"],
)]
#[get("/scratchpad-public")]
pub async fn scratchpad_public_get_owned(
    query_params: web::Query<MutateQueryParams>,
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "public Scratchpad";
    let rest_operation = "/scratchpad-public GET";
    let rest_handler = "scratchpad_public_get_owned()";

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_mutable_type_header_and_query_params(
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

    // This method contains the logic for determining which if any app ID is to
    // be used as well as deriving the object's owner secret.
    let scratchpad_secret =
        match request_params.derive_object_owner_secret(PUBLIC_SCRATCHPAD_DERIVATION_INDEX) {
            Ok(derived_secret) => derived_secret,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::BAD_REQUEST),
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to derive owner secret for {REST_TYPE} - {e}"),
                );
            }
        };

    let scratchpad_address = ScratchpadAddress::new(scratchpad_secret.public_key());

    let scratchpad = match client.client.scratchpad_get(&scratchpad_address).await {
        Ok(scratchpad) => scratchpad,
        Err(e) => match e {
            ScratchpadError::Fork(scratchpads) => scratchpads[0].clone(),
            e => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to get {REST_TYPE} from network - {e}"),
                );
            }
        },
    };

    let mut dweb_scratchpad = DwebScratchpad {
        dweb_type: DwebType::PublicScratchpad,
        scratchpad_address: scratchpad.address().to_hex(),
        data_encoding: scratchpad.data_encoding(),
        unencrypted_data: scratchpad.encrypted_data().to_vec(),
        counter: scratchpad.counter(),
        ..Default::default()
    };

    match scratchpad.decrypt_data(&scratchpad_secret) {
        Ok(bytes) => {
            dweb_scratchpad.unencrypted_data = bytes.to_vec();
            println!("DEBUG {rest_operation} successfully decrypted scratchpad data");
        }
        Err(e) => {
            println!("DEBUG {rest_operation} failed to decrypt scratchpad data failed - {e}")
        }
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

    // println!("DEBUG DwebScratchpad as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Create a new public Scratchpad on the network
/// TODO example JSON
///
/// Scratchpad data is assumed to be unencrypted
///
/// Note: This implementation differs from the Autonomi APIs in that you can have
/// any number of scratchpads with the same owner but different names, and these will
/// not clash with other types also using the same owner.
///
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination"),
        ("Ant-App-ID" = Option<String>, Header, description = "a unique string identifier for this app (as suggested by Autonomi and used to derive the VaultContentType used by an app)")),
      // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    request_body(content = DwebScratchpad, content_type = "application/json"),
    responses(
        (status = StatusCode::CREATED, description = "A MutateResult featuring either status 201 with cost and the network address of the created Scratchpad, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Dweb Autonomi"],
)]
#[post("/scratchpad-public")]
pub async fn scratchpad_public_post(
    request: HttpRequest,
    scratchpad: web::Json<DwebScratchpad>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "public Scratchpad";
    let dweb_type = DwebType::PublicScratchpad;
    let rest_operation = "/scratchpad-public POST".to_string();
    let rest_handler = "scratchpad_public_post()";

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_mutable_type_header_and_query_params(
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

    // This method contains the logic for determining which if any app ID is to
    // be used as well as deriving the object's owner secret.
    let scratchpad_secret =
        match request_params.derive_object_owner_secret(PUBLIC_SCRATCHPAD_DERIVATION_INDEX) {
            Ok(derived_secret) => derived_secret,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::BAD_REQUEST),
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to derive owner secret for {REST_TYPE} - {e}"),
                );
            }
        };

    let payment_option = client.payment_option().clone();
    let content_type = scratchpad.data_encoding;

    let initial_data = match scratchpad.unencrypted_data.clone().try_into_bytes() {
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

    let public_scratchpad =
        create_public_scratchpad(&scratchpad_secret, content_type, &initial_data, 0);

    let spends = Spends::new(&client, None).await;
    let result = retry_until_ok(
        request_params.tries,
        &rest_operation,
        (public_scratchpad, payment_option),
        async move |(public_scratchpad, payment_option)| match client
            .client
            .scratchpad_put(public_scratchpad, payment_option)
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
                    println!("DEBUG {rest_operation} cost in ANT    : {cost_in_ant}");
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

/// Update an existing public Scratchpad on the network
/// TODO example JSON
///
/// Scratchpad data is assumed to be unencrypted
#[utoipa::path(
    put,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each put, 0 means unlimited. This overrides the API control setting in the server."),
        ("object_name" = Option<String>, Query, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination"),
        ("Ant-App-ID" = Option<String>, Header, description = "a unique string identifier for this app (as suggested by Autonomi and used to derive the VaultContentType used by an app)")),
        // Support Query params using headers but don't document in the SwaggerUI to keep it simple
        // ("Ant-API-Tries" = Option<u32>, Header, description = "optional number of time to try a mutation operation before returning failure (0 = unlimited)"),
        // ("Ant-Object-Name" = Option<String>, Header, description = "optional name, used to allow more than one scratchpad per owner secret/app id combination")),
        // ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
        // ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    request_body(content = DwebScratchpad, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "A MutateResult featuring either status 200 with cost and the network address of the created Scratchpad, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;500 INTERNAL_SERVER_ERROR: Error reading posted data or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;502 BAD_GATEWAY: Autonomi network error<br/>", body = MutateResult,)
    ),
    tags = ["Dweb Autonomi"],
)]
#[put("/scratchpad-public")]
pub async fn scratchpad_public_put(
    request: HttpRequest,
    scratchpad: web::Json<DwebScratchpad>,
    query_params: web::Query<MutateQueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    const REST_TYPE: &str = "public Scratchpad";
    let dweb_type = DwebType::PublicScratchpad;
    let rest_operation = "/scratchpad-public PUT".to_string();
    let rest_handler = "scratchpad_public_put()";
    let dweb_scratchpad = scratchpad.into_inner();

    let client = &client.into_inner();
    let request_params = match ParsedRequestParams::process_mutable_type_header_and_query_params(
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

    // This method contains the logic for determining which if any app ID is to
    // be used as well as deriving the object's owner secret.
    let scratchpad_secret =
        match request_params.derive_object_owner_secret(PUBLIC_SCRATCHPAD_DERIVATION_INDEX) {
            Ok(derived_secret) => derived_secret,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::BAD_REQUEST),
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_handler} failed to derive owner secret for {REST_TYPE} - {e}"),
                );
            }
        };

    let payment_option = client.payment_option().clone();
    let content_type = dweb_scratchpad.data_encoding;

    let new_data = match dweb_scratchpad.unencrypted_data.try_into_bytes() {
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

    let public_scratchpad = create_public_scratchpad(
        &scratchpad_secret,
        content_type,
        &new_data,
        dweb_scratchpad.counter,
    );

    // Updates to a public Scratchpad are charged because an Autonomi API limitation
    let spends = Spends::new(&client, None).await;
    let result = retry_until_ok(
        request_params.tries,
        &rest_handler,
        (public_scratchpad, payment_option),
        async move |(public_scratchpad, payment_option)| match client
            .client
            .scratchpad_put(public_scratchpad, payment_option)
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
            let (cost_in_ant, cost_in_arb_eth) = match spends {
                Ok(spends) => {
                    let (cost_in_ant, cost_in_arb_eth) = spends.get_spend_strings().await;
                    println!("DEBUG {rest_operation} cost in ANT    : {cost_in_ant}");
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

/// Create a new public Scratchpad (offline)
fn create_public_scratchpad(
    scratchpad_secret: &SecretKey,
    data_encoding: u64,
    unencrypted_data: &Bytes,
    counter: u64,
) -> Scratchpad {
    println!("DEBUG Creating public scratchpad with encoding {data_encoding}");
    let owner_public = scratchpad_secret.public_key();
    let address = ScratchpadAddress::new(owner_public);

    // We don't encrypt of course
    let encrypted_data = unencrypted_data.clone();

    let bytes_to_sign =
        Scratchpad::bytes_for_signature(address, data_encoding, &encrypted_data, counter);
    let signature = scratchpad_secret.sign(&bytes_to_sign);

    Scratchpad::new_with_signature(
        owner_public,
        data_encoding,
        encrypted_data,
        counter,
        signature,
    )
}

/// A representation of the Autonomi Scratchpad for web clients
///
/// Exactly one target is allowed, so make sure unused targets are empty strings
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DwebScratchpad {
    dweb_type: DwebType,
    scratchpad_address: String,
    data_encoding: u64,
    encrypted_data: Vec<u8>,
    unencrypted_data: Vec<u8>,
    counter: u64,
}

impl Default for DwebScratchpad {
    fn default() -> DwebScratchpad {
        DwebScratchpad {
            dweb_type: DwebType::PrivateScratchpad,
            scratchpad_address: "".to_string(),
            counter: 0,
            data_encoding: 0,
            encrypted_data: Vec::<u8>::new(),
            unencrypted_data: Vec::<u8>::new(),
        }
    }
}
