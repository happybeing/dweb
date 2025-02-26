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

// use actix_web::{body, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web::{
    body, dev::HttpServiceFactory, dev::ServiceRequest, dev::ServiceResponse, get, guard,
    http::header, http::StatusCode, post, web, web::Data, App, Error, HttpRequest, HttpResponse,
    HttpResponseBuilder, HttpServer, Responder,
};
use color_eyre::eyre::{eyre, Result};

use dweb::cache::directory_with_port::*;
use dweb::helpers::convert::address_tuple_from_address_or_name;
use dweb::web::fetch::response_redirect;
use dweb::web::name::validate_dweb_name;
use dweb::web::LOCALHOST_STR;

use crate::services_quick::{register_name, serve_quick};

pub fn init_service() -> impl HttpServiceFactory {
    actix_web::web::scope("/dweb-link").service(dweb_link)
}

// dweb_link parses the parameters manually to allow the version portion
// to be ommitted, and support easier manual construction:
//
// url: http://127.0.0.1:<PORT>/[v{version}/]{address_or_name}{remote_path}
//
#[get("/{params:.*}")]
pub async fn dweb_link(
    request: HttpRequest,
    // params: web::Path<(String, String, String)>,
    params: web::Path<String>,
    client: Data<dweb::client::AutonomiClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
) -> HttpResponse {
    println!("DEBUG dweb_link()...");

    let params = params.into_inner();
    let (version, address_or_name, remote_path) = match parse_dweb_link_params(&params) {
        Ok(params) => params,
        Err(e) => {
            return make_error_response(
                None,
                &mut HttpResponse::BadRequest(),
                "/dweb-link invalid parameters: {params}",
            )
        }
    };

    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response(
            None,
            &mut HttpResponse::BadRequest(),
            &format!("Unrecognised DWEB-NAME or invalid address: '{address_or_name}'"),
        );
    }

    // TODO Check if we are the handler using our_directory_version

    // Look for an existing handler
    // As we've already parsed address_or_name, an error return only means there isn't a handler for this yet
    let directory_version = match lookup_directory_version_with_port(&address_or_name, version) {
        Ok(directory_version) => directory_version,
        Err(_) => {
            // Create a new DirectoryVersionWithPort and spawn a handler for it
            let directory_version = match create_directory_version_with_port(
                &client,
                &address_or_name,
                version,
            )
            .await
            {
                Ok(directory_version) => directory_version,
                Err(e) => {
                    return make_error_response(None,
                        &mut HttpResponse::BadGateway(),
                        &format!("dweb_link() Failed to start a server_quick to serve archive for address: {address_or_name}")
                    );
                }
            };

            if serve_quick(
                &client,
                Some(directory_version.clone()),
                None,
                true,
                *is_local_network.into_inner().as_ref(),
            )
            .await
            .is_err()
            {
                return make_error_response(None,
                    &mut HttpResponse::BadGateway(),
                    &format!("dweb_link() Failed to start a server_quick to serve archive for address: {address_or_name}")
                );
            }
            directory_version
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

/// Parse the path part of a DWEB-LINK URL, which in full is:
///
/// url: http://127.0.0.1:<PORT>/[v{version}/]{address_or_name}{remote_path}
pub fn parse_dweb_link_params(params: &String) -> Result<(Option<u32>, String, String)> {
    // Parse params manually so we can support with and without version
    println!("DEBUG parse_dweb_link_params() {params}");

    // We have two or three parameters depending on whether a version is included.
    // So split in a way we can get either combination depending on what we find.
    let (first, rest) = params.split_once('/').unwrap_or(("", &params));
    let first_rest = String::from(rest);
    let (second, rest) = first_rest.split_once('/').unwrap_or(("", &first_rest));
    let second_rest = String::from(rest);

    println!("1:{first} 2: {second} r: {rest}");

    // If it validates as a DWEB-NAME it can't be a version (because they start with two alphabetic characters)

    let (version, address_or_name, remote_path) = match validate_dweb_name(&first) {
        Ok(_) => (None, first, first_rest),
        Err(e) => match parse_version_string(&first) {
            Ok(version) => (version, second, second_rest),
            Err(_) => {
                let msg = "/dweb-link parameters not valid: '{params}'";
                println!("DEBUG {msg}");
                return Err(eyre!(msg));
            }
        },
    };

    println!("version:{version:?} address_or_name: {address_or_name} remote_path: {remote_path}");
    Ok((version, address_or_name.to_string(), remote_path))
}

/// Parse a string and if valid return an Option<u32>
///
/// Valid version strings consist of a 'v' (or 'V') followed by an optional integer.
/// In orther words: v[<VERSION>], where VERSION is a u32.
pub fn parse_version_string(version_str: &str) -> Result<Option<u32>> {
    if version_str.starts_with("v") || version_str.starts_with("V") {
        let version = version_str[1..].to_string();
        if version.is_empty() {
            Ok(None)
        } else {
            if let Ok(version) = version.parse::<u32>() {
                Ok(Some(version))
            } else {
                // println!("DEBUG parse_version_string({version_str}) failed");
                Err(eyre!("invalid version: '{version_str}'"))
            }
        }
    } else {
        Err(eyre!("invalid version: '{version_str}'"))
    }
}

fn make_error_response(
    status_code: Option<StatusCode>,
    response_builder: &mut HttpResponseBuilder,
    message: &str,
) -> HttpResponse {
    let status_code = if let Some(status_code) = status_code {
        &format!("{status_code}")
    } else {
        ""
    };

    let body = format!(
        "
    <!DOCTYPE html><head></head><body>
    <h3>/dweb-link handler error</h3>
    {status_code} error - <br/>{message}
    </body>"
    );

    response_builder.body(body)
}
