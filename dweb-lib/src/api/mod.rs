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

use color_eyre::eyre::{eyre, Error, Result};

///! A Rust interface to dweb server APIs
///!
///! TODO keep this and the with ports APIs in sync
use crate::trove::HistoryAddress;
use crate::web::name::RecognisedName;
use crate::web::request::main_server_request;

/// The dweb::api is a native Rust API that handles http interaction with the dweb server.
///
/// The server can be accessed directly from any language, but this API simplifies the process
/// for Rust apps.

// IMPLEMENTATION: keep these wrappers slim and let the server
// do parameter checks, and return the raw results immediately
// for checking by the caller.

// TODO break this out into modules here and in the with ports server

pub const DWEB_API_ROUTE_V0: &str = "/dweb/v0"; // Route for dweb API v0
pub const DWEB_API_ROUTE_V0_ANT: &str = "/dweb/v0/ant"; // Route route for

pub const DWEB_API_ROUTE: &str = DWEB_API_ROUTE_V0;

/// Register a name with the main server
pub async fn name_register(
    dweb_name: &str,
    history_address: HistoryAddress,
    host: Option<&String>,
    port: Option<u16>,
) -> Result<()> {
    let url_path = format!(
        "{DWEB_API_ROUTE}/name_register/{dweb_name}/{}",
        history_address.to_hex()
    );

    match main_server_request(&url_path, host, port).await {
        Ok(_json_value) => Ok(()),
        Err(e) => Err(eyre!(Into::<Error>::into(e))),
    }
}

/// Query the server for a list of recognised names
pub async fn name_list(host: Option<&String>, port: Option<u16>) -> Result<Vec<RecognisedName>> {
    let url_path = format!("{DWEB_API_ROUTE}/name_list");
    match main_server_request(&url_path, host, port).await {
        Ok(json) => {
            let vec: Vec<RecognisedName> = serde_json::from_str(&json)?;
            Ok(vec)
        }
        Err(e) => Err(eyre!(e)),
    }
}
