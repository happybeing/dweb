/*
 Copyright (c) 2024-2025 Mark Hughes

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

use actix_web::{dev::HttpServiceFactory, get, web, web::Data, HttpRequest, HttpResponse};

use dweb::api::name_register;
use dweb::cache::directory_with_port::*;
use dweb::helpers::convert::address_tuple_from_address_or_name;
use dweb::web::fetch::response_redirect;
use dweb::web::LOCALHOST_STR;

use crate::services::helpers::*;
use crate::services::serve_with_ports;

use super::make_error_response_page;

/// Open the content at a given address or name
///
/// url: <code>http://127.0.0.1:8080/dweb-open/[v<VERSION-NUMBER>/]<ADDRESS-OR-NAME><REMOTE-PATH></code>
///
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of a DirectoryTree formatted for an SVAR file manager component.
            <p>Note: this may be changed to return a JSON representation of a DirectoryTree.", body = str)
        ),
    tags = [dweb::api::DWEB_API_ROUTE],
    params(
        ("VERSION-NUMBER" = Option<u64>, description = "Optional version when ADDRESS-OR-NAME refers to a History<DirectoryTree>"),
        ("ADDRESS-OR-NAME", description = "A hexadecimal address or a short name referring to a History or PublicArchive"),
        ("REMOTE-PATH" = Option<String>, description = "Optional path to the resource you wish to open. Must begin with \"/\"")
    )
)]
#[get("/dweb-open/{params:.*}")]
pub async fn dweb_open(
    request: HttpRequest,
    // params: web::Path<(String, String, String)>,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let params = params.into_inner();
    let decoded_params = match parse_versioned_path_params(&params) {
        Ok(params) => params,
        Err(_e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/dweb-open error".to_string(),
                "/dweb-open invalid parameters: {params}",
            )
        }
    };

    handle_dweb_open(
        &request,
        client,
        our_directory_version,
        is_local_network,
        &decoded_params,
    )
    .await
}

/// Open the content at a given address and register a name for it
///
/// url: <code>http://127.0.0.1:8080/dweb-open-as/v<VERSION-NUMBER>/<DWEB-NAME>/<HISTORY-ADDRESS><REMOTE-PATH></code>
///
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of a DirectoryTree formatted for an SVAR file manager component.
            <p>Note: this may be changed to return a JSON representation of a DirectoryTree.", body = str)
        ),
    tags = [dweb::api::DWEB_API_ROUTE],
    params(
        ("VERSION-NUMBER" = Option<u64>, description = "Optional version (integer > 0) of the History<DirectoryTree>"),
        ("DWEB-NAME", description = "The short name to register for the HISTORY-ADDRESS"),
        ("HISTORY-ADDRESS", description = "A hexadecimal address or a short name referring to a content History"),
        ("REMOTE-PATH" = Option<String>, description = "Optional path to the resource you wish to open. Must begin with \"/\"")
    )
)]
#[get("/dweb-open-as/{params:.*}")]
pub async fn dweb_open_as(
    request: HttpRequest,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let params = params.into_inner();
    let decoded_params = match parse_versioned_path_params_with_as_name(&params) {
        Ok(params) => params,
        Err(_ant_bootstrape) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/open-as error".to_string(),
                "/open-as invalid parameters: {params}",
            )
        }
    };

    handle_dweb_open(
        &request,
        client,
        our_directory_version,
        is_local_network,
        &decoded_params,
    )
    .await
}

pub async fn handle_dweb_open(
    request: &HttpRequest,
    client: Data<dweb::client::DwebClient>,
    _our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
    decoded_params: &(Option<u32>, String, String, String),
) -> HttpResponse {
    let (version, as_name, address_or_name, remote_path) = decoded_params;
    let version = version.clone();

    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            "/dweb-open error".to_string(),
            &format!("Unrecognised DWEB-NAME or invalid address: '{address_or_name}'"),
        );
    }

    // TODO Check if we are the handler using our_directory_version

    // Look for an existing handler
    // As we've already parsed address_or_name, an error return only means there isn't a handler for this yet
    let directory_version = match lookup_or_create_directory_version_with_port(
        &client,
        &address_or_name,
        version,
    )
    .await
    {
        Ok((directory_version, from_cache)) => {
            if !from_cache {
                // Not in the cache so spawn a server to handle it
                match serve_with_ports(
                    &client,
                    Some(directory_version.clone()),
                    dweb::web::LOCALHOST_STR.to_string(),
                    None,
                    true,
                    *is_local_network.into_inner().as_ref(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        return make_error_response_page(
                            None,
                            &mut HttpResponse::BadGateway(),
                            "/dweb-open error".to_string(),
                            &format!("{e}. Address: {address_or_name}"),
                        )
                    }
                };

                // Register a valid 'as_name' unless:
                // - the as_name given is AS_NAME_NONE ('anonymous')
                // - the address was an Archive
                if !as_name.is_empty() && as_name != AS_NAME_NONE {
                    if let Some(history_address) = directory_version.history_address {
                        // Using default port here means this won't work for '--experimental'
                        let _ = name_register(&as_name, history_address, None, None).await;
                    }
                };
            };
            directory_version
        }
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadGateway(),
                "/dweb-open error".to_string(),
                &format!("{e}. Address: {address_or_name}"),
            )
        }
    };

    let remote_path = if !remote_path.is_empty() {
        Some(format!("/{remote_path}"))
    } else {
        None
    };

    // Redirect
    response_redirect(
        &request,
        LOCALHOST_STR,
        Some(directory_version.port),
        remote_path,
    )
}
