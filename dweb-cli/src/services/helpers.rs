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
    http::{header, StatusCode},
    HttpResponse, HttpResponseBuilder,
};
use color_eyre::eyre::{eyre, Result};

use dweb::web::name::validate_dweb_name;

pub const AS_NAME_NONE: &str = "anonymous";

/// parse dweb address_or_name url with an 'as name'
///
///     [v{version}/]{address_or_name}{remote_path}
///
/// returns a tuple of:
///     Option<u64> // version if present
///
/// Note:
///     version is an optional integer (u32)
///     address_or_name is the site to visit
///     remote_path is the resource to load from the site
///
pub fn parse_versioned_path_params(
    params: &String,
) -> Result<(Option<u32>, String, String, String)> {
    // Parse params manually so we can support with and without version
    println!("DEBUG parse_versioned_path_params_with_as_name() {params}");

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
pub fn parse_versioned_path_params_with_as_name(
    params: &String,
) -> Result<(Option<u32>, String, String, String)> {
    // Parse params manually so we can support with and without version
    println!("DEBUG parse_versioned_path_params() {params}");

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

pub(crate) fn make_error_response_page(
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
    <h3>{heading} error</h3>
    {status_code} {message}
    <br/><br/><a href='_back'>Go back</a>
    </body>"
    );

    response_builder
        .insert_header(header::ContentType(mime::TEXT_HTML))
        .body(body)
}
