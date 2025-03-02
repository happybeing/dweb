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

use color_eyre::eyre::Result;

/// Submit a request to the main with ports server and return a JSON result on success
///
/// url_path should begin with '/' and contains the API path and any parameters for the
/// request.
///
/// You don't need to provide host or port unless you wish to override the defaults.
///
/// This function assumes some defaults to save having to build the request from scratch
/// but it is fine to do that and not use this function. If doing that yourself, it is
/// recommended that you use the helper function make_serve_with_ports_host() to construct
/// the host/port part of the URL.
///
pub async fn main_server_request(
    url_path: &str,
    host: Option<&String>,
    port: Option<u16>,
) -> Result<String> {
    let url_string = make_main_server_url(host, port, url_path);
    println!("main_server_request() request: {url_string}");

    let response: reqwest::Response = reqwest::Client::builder()
        .build()?
        .get(&url_string)
        .header("Accept", "application/json")
        .send()
        .await?;

    let body = response.text().await?;
    Ok(body)
}

// Default to 'with ports' server
pub fn make_main_server_url(host: Option<&String>, port: Option<u16>, url_path: &str) -> String {
    let default_host = crate::web::LOCALHOST_STR.to_string();
    let host = host.unwrap_or(&default_host);
    let port = port.unwrap_or(crate::web::SERVER_PORTS_MAIN_PORT);
    format!("http://{host}:{port}{url_path}")
}
