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
    http::{header, header::ContentType, StatusCode},
    HttpRequest, HttpResponse, HttpResponseBuilder, Responder,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use dweb::storage::DwebType;

use crate::services::helpers::*;

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
    /// The cost incurred by the operation
    pub cost_in_attos: String,

    /// Name of the resource when data_type is "file"
    pub file_name: String,
    /// Full local system path of the resource (returned by /publish APIs)
    pub full_path: String,
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
            cost_in_attos: "0.0".to_string(),
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
