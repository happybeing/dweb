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
    dev::HttpServiceFactory, get, http::StatusCode, web, web::Data, HttpRequest, HttpResponse,
    HttpResponseBuilder,
};
use color_eyre::eyre::{eyre, Result};

use dweb::api::name_register;
use dweb::cache::directory_with_port::*;
use dweb::helpers::convert::address_tuple_from_address_or_name;
use dweb::web::fetch::response_redirect;
use dweb::web::name::validate_dweb_name;
use dweb::web::LOCALHOST_STR;

use crate::services::serve_with_ports;

pub fn init_dweb_open() -> impl HttpServiceFactory {
    actix_web::web::scope("/dweb-open").service(dweb_open)
}

pub fn init_dweb_open_as() -> impl HttpServiceFactory {
    actix_web::web::scope("/dweb-open-as").service(dweb_open_as)
}

const AS_NAME_NONE: &str = "anonymous";

// dweb_open parses the parameters manually to allow the version portion
// to be ommitted, and support easier manual construction:
//
// url: http://127.0.0.1:8080/dweb-open/[v{version}/][{as_name}/]{address_or_name}{remote_path}
//
#[get("/{params:.*}")]
pub async fn dweb_open_as(
    request: HttpRequest,
    // params: web::Path<(String, String, String)>,
    params: web::Path<String>,
    client: Data<dweb::client::AutonomiClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
) -> HttpResponse {
    println!("DEBUG dweb_open_as()...");

    let params = params.into_inner();
    let decoded_params = match parse_dweb_open_as(&params) {
        Ok(params) => params,
        Err(ant_bootstrape) => {
            return make_error_response(
                None,
                &mut HttpResponse::BadRequest(),
                "/dweb_open_as handler error".to_string(),
                "/dweb-open_as invalid parameters: {params}",
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

// dweb_open parses the parameters manually to allow the version portion
// to be ommitted, and support easier manual construction:
//
// url: http://127.0.0.1:8080/dweb-open/[v{version}/][{as_name}/]{address_or_name}{remote_path}
//
#[get("/{params:.*}")]
pub async fn dweb_open(
    request: HttpRequest,
    // params: web::Path<(String, String, String)>,
    params: web::Path<String>,
    client: Data<dweb::client::AutonomiClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
) -> HttpResponse {
    println!("DEBUG dweb_open()...");

    let params = params.into_inner();
    let decoded_params = match parse_dweb_open(&params) {
        Ok(params) => params,
        Err(_e) => {
            return make_error_response(
                None,
                &mut HttpResponse::BadRequest(),
                "/dweb_open handler error".to_string(),
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

pub async fn handle_dweb_open(
    request: &HttpRequest,
    client: Data<dweb::client::AutonomiClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    is_local_network: Data<bool>,
    decoded_params: &(Option<u32>, String, String, String),
) -> HttpResponse {
    let (version, as_name, address_or_name, remote_path) = decoded_params;
    let version = version.clone();

    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response(
            None,
            &mut HttpResponse::BadRequest(),
            "/dweb_open handler error".to_string(),
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
                        return make_error_response(
                            None,
                            &mut HttpResponse::BadGateway(),
                            "/dweb_open handler error".to_string(),
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
            return make_error_response(
                None,
                &mut HttpResponse::BadGateway(),
                "/dweb_open handler error".to_string(),
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

/// Parse the path part of a dweb-open URL, which in full is:
///
/// url: http://127.0.0.1:<PORT>/[v{version}/]{address_or_name}{remote_path}
pub fn parse_dweb_open(params: &String) -> Result<(Option<u32>, String, String, String)> {
    // Parse params manually so we can support with and without version
    println!("DEBUG parse_dweb_open_params() {params}");

    // We have two or three parameters depending on whether a version is included.
    // So split in a way we can get either combination depending on what we find.
    let (first, rest) = params.split_once('/').unwrap_or((&params, ""));
    let first_rest = String::from(rest);
    let (second, rest) = first_rest.split_once('/').unwrap_or((&first_rest, ""));
    let second_rest = String::from(rest);

    println!("1:{first} 2: {second} r: {rest}");

    // If it validates as a DWEB-NAME it can't be a version (because they start with two alphabetic characters)

    let (version, address_or_name, remote_path) = match parse_version_string(&first) {
        Ok(version) => (version, second, second_rest),
        Err(_) => (None, first, first_rest),
    };

    println!("version:{version:?} as_name: {AS_NAME_NONE}, address_or_name: {address_or_name} remote_path: {remote_path}");
    Ok((
        version,
        AS_NAME_NONE.to_string(),
        address_or_name.to_string(),
        remote_path,
    ))
}

/// Parse the path part of a /dweb-open-as URL, which in full is:
///
/// url: http://127.0.0.1:<PORT>/[v{version}/]/{as_name}/{address_or_name}{remote_path}
///
/// Note:
///     version is an optional integer (u32)
///     as_name must either be a DWEB-NAME to register, or 'anomymous'
///     address_or_name is the site to visit
///     remote_path is the resource to load from the site
///
pub fn parse_dweb_open_as(params: &String) -> Result<(Option<u32>, String, String, String)> {
    // Parse params manually so we can support with and without version
    println!("DEBUG parse_dweb_open_as() {params}");

    // We have two or three parameters depending on whether a version is included.
    // So split in a way we can get either combination depending on what we find.
    let (first, rest) = params.split_once('/').unwrap_or((&params, ""));
    let first_rest = String::from(rest);
    let (second, rest) = first_rest.split_once('/').unwrap_or((&first_rest, ""));
    let second_rest = String::from(rest);
    let (third, rest) = second_rest.split_once('/').unwrap_or((&second_rest, ""));
    let third_rest = String::from(rest);

    println!("1:{first} 2: {second}  3: {third} r: {rest}");

    // If it validates as a DWEB-NAME it can't be a version (because they start with two alphabetic characters)

    let (version, as_name, address_or_name, remote_path) = match parse_version_string(&first) {
        Ok(version) => {
            println!("BINGO 1");
            (version, second, third, third_rest)
        }
        Err(_) => match validate_dweb_name(first) {
            Ok(_as_name) => {
                println!("BINGO 2");
                (None, first, second, second_rest)
            }
            Err(_) => {
                println!("BINGO 3");
                let msg = format!("/dweb-open-as parameters not valid: '{params}'");
                println!("DEBUG {msg}");
                return Err(eyre!(msg));
            }
        },
    };

    println!("version:{version:?} as_name: {as_name} address_or_name: {address_or_name} remote_path: {remote_path}");
    Ok((
        version,
        as_name.to_string(),
        address_or_name.to_string(),
        remote_path,
    ))
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
    heading: String,
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
    <h3>{heading}</h3>
    {status_code} {message}
    </body>"
    );

    response_builder.body(body)
}
