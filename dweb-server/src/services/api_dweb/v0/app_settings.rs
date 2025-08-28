/*
 Copyright (c) 2025 Mark Hughes

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
    http::{header::ContentType, StatusCode},
    web::Data,
    HttpRequest, HttpResponse,
};

use dweb::cache::directory_with_port::*;

//use crate::services::api_dweb::v0::DwebNetworkSettings;
use crate::services::helpers::*;

/// Get information about the app and network settings
///
/// Note: information related to the app server will not be available
/// when this request is submitted to the main dweb server
///
/// url: <code>/dweb-0/app-settings</code>
#[utoipa::path(
    responses(
        (status = StatusCode::OK, body = [DwebNetworkSettings],
            description = "JSON encoded information about the app server and connected network", body = str)
        ),
    tags = ["Dweb"],
)]
#[get("/app-settings")]
pub async fn app_settings(
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    _is_main_server: Data<bool>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/app-settings GET".to_string();
    let rest_handler = "app_settings()";

    let mut app_port: i32 = -1;
    if our_directory_version.is_some() {
        let directory_version = our_directory_version.as_ref().clone().unwrap();
        app_port = directory_version.port as i32;
    } else {
        println!("WARNING {rest_operation} not made on an app server so some information will not be available in the response");
    };

    let settings = DwebNetworkSettings {
        network_id: client.client.evm_network().to_string(),
        is_local: client.is_local,

        app_port: app_port,
    };

    let json = match serde_json::to_string(&settings) {
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

// TODO move to v0
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
/// Information about the Autonomi network connection and the app's server
#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct DwebNetworkSettings {
    // Network
    network_id: String,
    is_local: bool,

    // The app server port when serving the app from network. Otherwise -1, such as when using a local development server)
    app_port: i32,
}
