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

pub mod file;
pub mod form;
pub mod name;
// pub mod publish;

use actix_web::{
    get,
    http::{header, header::ContentType, header::HeaderMap, StatusCode},
    HttpRequest, HttpResponse, HttpResponseBuilder, Responder,
};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::SecretKey;

use dweb::client::DwebClient;
use dweb::storage::DwebType;

use crate::services::helpers::*;

/// Request headers for Mutate (POST/PUT) of Autonomi types. Not all headers apply to all types.
///
/// Notes:
///
/// 1. These headers have corresponding query parameters which if present will override the request header
///
/// 2. These headers are not included in the OpenAPI/SwaggerUI to keep it simple
///
/// tries: u32,  optional number of time to try a mutation operation before returning failure (0 = unlimited)
pub const HEADER_ANT_API_TRIES: &str = "Ant-API-Tries";

/// object_name: Option<String>,   optional name, used to allow more than one object of the relevant type per owner secret
pub const HEADER_ANT_OBJECT_NAME: &str = "Ant-Object-Name";

/// owner_secret: Option<String>,   optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations)
pub const HEADER_ANT_OWNER_SECRET: &str = "Ant-Owner-Secret";

/// object_derivation_index: Option<String>,   optional 32 character string to use instead of the dweb default when deriving keys for objects of this type
pub const HEADER_ANT_DERIVATION_INDEX: &str = "Ant-Derivation-Index";

#[derive(Deserialize, ToSchema)]
pub struct MutateQueryParams {
    /// An optional name for the object being created or updated. Only one object of each type is permitted per object_name.
    object_name: Option<String>,
    /// The number of times to try a mutation operation until returning failure. (0 = unlimited)
    tries: Option<u32>,
    /// ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
    owner_secret: Option<String>,
    /// ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    type_derivation_key: Option<[u8; 32]>,
}

/// Validated parameters based on request query and headers
pub struct ParsedRequestParams {
    /// The number of times to try a mutation operation until returning failure. (0 = unlimited)
    pub tries: u32,
    /// An optional name for the object being created or updated. Only one object of each type is permitted per object_name.
    pub object_name: Option<String>,
    /// ("Ant-Owner-Secret" = Option<String>, Header, description = "optional secret key. Used to override the key selected for use by the server (for mutation and decryption operations"),
    pub owner_secret: Option<SecretKey>,
    /// ("Ant-Derivation-Index" = Option<String>, Header, description = "optional 32 character string to use instead of the dweb default when deriving keys for objects of this type"),
    pub type_derivation_index: Option<[u8; 32]>,
}

impl Default for ParsedRequestParams {
    fn default() -> Self {
        Self {
            tries: 1,
            object_name: None,
            owner_secret: None,
            type_derivation_index: None,
        }
    }
}

impl ParsedRequestParams {
    /// Process request headers and query parameters. Query parameters have precedance over request headers
    ///
    /// Returns ParsedRequestParams or an error if an invalid parameter was encountered
    pub(crate) fn process_header_and_mutate_query_params(
        client: &DwebClient,
        headers: &HeaderMap,
        query_params: &MutateQueryParams,
    ) -> Result<ParsedRequestParams> {
        // TODO return error if appropriate - I'm not sure it is worth reporting header_valud.to_str() errors
        let mut tries = query_params.tries.clone();
        if tries.is_none() {
            if let Some(header_value) = headers.get(HEADER_ANT_API_TRIES) {
                if let Ok(header_str) = header_value.to_str() {
                    if let Ok(tries_u32) = header_str.parse::<u32>() {
                        tries = Some(tries_u32)
                    }
                };
            };
        }
        let tries = tries.unwrap_or(client.api_control.tries);

        // TODO return error if appropriate - I'm not sure it is worth reporting header_valud.to_str() errors
        let mut object_name = query_params.object_name.clone();
        if object_name.is_none() {
            if let Some(header_value) = headers.get(HEADER_ANT_OBJECT_NAME) {
                if let Ok(header_str) = header_value.to_str() {
                    object_name = Some(header_str.to_string())
                };
            };
        };

        // TODO consider not reporting header_valud.to_str() errors
        let mut owner_secret = None;
        if let Some(header_value) = headers.get(HEADER_ANT_OWNER_SECRET) {
            match header_value.to_str() {
                Ok(header_str) => match SecretKey::from_hex(header_str) {
                    Ok(secret) => owner_secret = Some(secret),
                    Err(e) => {
                        return Err(eyre!(
                        "Request header {HEADER_ANT_OWNER_SECRET} is not a valid secret key - {e}"
                    ))
                    }
                },
                Err(e) => {
                    return Err(eyre!(
                        "Request header {HEADER_ANT_OWNER_SECRET} is not a string - {e}"
                    ))
                }
            };
        };

        // TODO consider not reporting header_valud.to_str() errors
        let mut type_derivation_index = None;
        if let Some(header_value) = headers.get(HEADER_ANT_DERIVATION_INDEX) {
            match header_value.to_str() {
                Ok(header_str) => {
                    match header_str.as_bytes().try_into() {
                        Ok(index) => type_derivation_index = Some(index),
                        Err(e) =>
                        return Err(eyre!("Request header {HEADER_ANT_DERIVATION_INDEX} is not a 32 character string - {e}"))
                    }
                },
                Err(e) => {
                    return Err(eyre!(
                        "Request header {HEADER_ANT_DERIVATION_INDEX} is not a string - {e}"
                    ))
                }
            };
        };

        Ok(ParsedRequestParams {
            tries,
            object_name,
            owner_secret,
            type_derivation_index,
            ..Default::default()
        })
    }
}

/// MutateResult is used to return the result of POST or PUT operations for several network data types
#[derive(Serialize, Deserialize, ToSchema)]
pub struct MutateResult {
    /// DwebType of the data stored
    pub dweb_type: DwebType,

    /// REST operation description (e.g. "/pointer POST")
    pub rest_operation: String,

    /// The HTTP status code returned for this upload
    pub status_code: u16,
    /// Information about the operation, such as "success" or an explanation of an error.
    ///
    /// Either "success" or an explanatory error message.
    pub status_message: String,
    /// The cost incurred in ANT (1 ANT = 10^18 Attos)
    pub cost_in_ant: String,
    /// The cost incurred in ARB-ETH (1 ARB-ETH = 10^18 gas)
    pub cost_in_arb_eth: String,

    /// Name of the resource when data_type is "file"
    pub file_name: String,
    /// Full local system path of the resource (returned by /publish APIs)
    pub full_path: String,
    /// Optional name provided to differentiate objects of the same type which created with the same owner secret
    pub object_name: String,
    /// Hex encoded address of a data map or of other stored data. Only returned when uploading data as public
    ///
    /// Returned for public data of type: PublicFile, PublicArchive, History, Register, Pointer, Scratchpad, Vault
    pub network_address: String,
    /// Hex encoded data map for the uploaded data. Only returned when uploading data as private.
    ///
    /// This data_map has not been stored and will be needed in order to access the data later.
    pub data_map: String,
}

impl Default for MutateResult {
    fn default() -> MutateResult {
        MutateResult {
            dweb_type: DwebType::Unknown,
            rest_operation: "".to_string(),
            status_code: StatusCode::IM_A_TEAPOT.as_u16(),
            status_message: "".to_string(),
            cost_in_ant: "0.0".to_string(),
            cost_in_arb_eth: "0.0".to_string(),
            object_name: "".to_string(),
            network_address: "".to_string(),
            data_map: "".to_string(),
            file_name: "".to_string(),
            full_path: "".to_string(),
        }
    }
}

impl MutateResult {
    /// Return an HttpResponse containing the MutateResult
    ///
    /// The rest_handler string (e.g. "archive::post_private()") is only for debugging
    /// and used only if there is a problem inside this function.
    pub fn response(&self, rest_handler: &str) -> HttpResponse {
        let json = match serde_json::to_string(&self) {
            Ok(json) => json,
            Err(e) => {
                return make_error_response_page(
                    Some(StatusCode::INTERNAL_SERVER_ERROR),
                    &mut HttpResponse::InternalServerError(),
                    self.rest_operation.clone(),
                    &format!("{rest_handler} failed to encode JSON result - {e}"),
                )
            }
        };

        println!("DEBUG MutateResult as JSON: {json:?}");

        let status_code = StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::BAD_GATEWAY);
        if !status_code.is_success() {
            return make_error_response_page(
                Some(status_code),
                &mut HttpResponseBuilder::new(status_code),
                self.rest_operation.to_string(),
                &format!("{rest_handler} {}", self.status_message),
            );
        }

        HttpResponseBuilder::new(status_code)
            .insert_header(ContentType(mime::APPLICATION_JSON))
            .body(json)
    }

    /// Create a response based on the HTTP status code in the PUT result
    ///
    /// If the response is success it will return the MutateResult as a JSON encoded payload
    ///
    /// The rest_operation (e.g. "/archive-private POST error") and error_source (e.g. "archive::post_private()")
    /// are used for error responses to construct a descriptive HTML response, at least for now. These should
    /// be provided even in case of an OK response though, in case there is an error
    /// serialising the MutateResult as JSON (unlikely though that is).
    pub fn make_response(&self, rest_operation: &str, rest_handler: &str) -> HttpResponse {
        let json = match serde_json::to_string(&self) {
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

        println!("DEBUG mutate_result as JSON: {json:?}");

        let status_code = StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::BAD_GATEWAY);
        if !status_code.is_success() {
            return make_error_response_page(
                Some(status_code),
                &mut HttpResponseBuilder::new(status_code),
                rest_operation.to_string(),
                &format!("{rest_handler} {}", self.status_message),
            );
        }

        HttpResponseBuilder::new(status_code)
            .insert_header(ContentType(mime::APPLICATION_JSON))
            .body(json)
    }
}

/// Get the proxy identifier and version of the dweb API
#[utoipa::path(
    responses(
        (status = StatusCode::OK, description = "Returns the base route for the dweb APIs (e.g. '/dweb-0').
        This identifies the server as the 'dweb' proxy, and the version of the dweb API being served (e.g. '0').", body = str)

    ),
    tags = ["Server"],
)]
#[get("/ant-proxy-id")]
pub async fn ant_proxy_id(_request: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_PLAIN))
        .body(dweb::api::DWEB_API_ROUTE)
}
