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

use std::u16;

use dweb::cache::spawn::is_main_server_with_ports_running;
use dweb::web::{DWEB_SERVICE_API, LOCALHOST_STR};

/// Open a browser to view a website on Autonomi.
///
/// A 'with hosts' server must be running and a local DNS has been set up.
/// (Start the server with 'dweb serve --experimental')
//
// TODO support --register-as?
pub(crate) fn handle_browse_with_hosts(
    _dweb_name: Option<String>,
    address_name_or_link: &String,
    version: Option<u64>,
    remote_path: Option<String>,
    host: Option<&String>,
    port: Option<u16>,
) {
    let default_host = DWEB_SERVICE_API.to_string();
    let host = host.unwrap_or(&default_host);
    let port = port.unwrap_or(dweb::web::SERVER_HOSTS_MAIN_PORT);
    let version = if version.is_some() {
        &format!("{}", version.unwrap())
    } else {
        ""
    };
    let mut remote_path = remote_path.unwrap_or(String::from(""));
    if !remote_path.is_empty() && !remote_path.starts_with("/") {
        remote_path = format!("/{remote_path}");
    }

    // open a browser on a localhost URL at that port
    let route = format!("/dweb-open/v{version}/{address_name_or_link}/{remote_path}");

    let url = format!("http://{host}:{port}{route}");
    println!("DEBUG url: {url}");

    let _ = open::that(url);
}

/// Open a browser to view a website on Autonomi.
/// Requires a 'dweb serve' to be running which avoids the need for a local DNS to have been set up.
/// Note: the serve spawns a dedicated server per directory/website being accessed, so ports will run out if the servers are never killed.
//
// TODO support --register-as or leave that only for --experimental?
pub(crate) fn handle_browse_with_ports(
    address_name_or_link: &String,
    version: Option<u64>,
    as_name: Option<String>,
    remote_path: Option<String>,
    host: Option<String>,
    port: Option<u16>,
) {
    if !is_main_server_with_ports_running() {
        println!("Please  start the dweb server before using 'dweb open'");
        println!("For help, type 'dweb serve --help");
        return;
    }

    // If the main server is running it will handle the URL and spawn a new server one is not already running

    let host = host.unwrap_or(LOCALHOST_STR.to_string());
    let port = port.unwrap_or(dweb::web::SERVER_PORTS_MAIN_PORT);
    let version = if version.is_some() {
        &format!("{}", version.unwrap())
    } else {
        ""
    };
    let mut remote_path = remote_path.unwrap_or(String::from(""));
    if !remote_path.is_empty() && !remote_path.starts_with("/") {
        remote_path = format!("/{remote_path}");
    }

    // open a browser on a localhost URL at that port
    let route = if let Some(as_name) = as_name {
        format!("/dweb-open-as/v{version}/{as_name}/{address_name_or_link}/{remote_path}")
    } else {
        format!("/dweb-open/v{version}/{address_name_or_link}/{remote_path}")
    };
    let url = format!("http://{host}:{port}{route}");
    println!("DEBUG url: {url}");

    let _ = open::that(url);
}
