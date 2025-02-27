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
use dweb::helpers::convert::address_tuple_from_address_or_name;

/// Open a browser to view a website on Autonomi.
///
/// A 'with names' server must be running and a local DNS has been set up.
/// (Start the server with 'dweb serve --use-domains')
pub(crate) fn handle_browse_with_names(dweb_name: String, address_name_or_link: &String) {
    let (history_address, archive_address) =
        address_tuple_from_address_or_name(&address_name_or_link);

    let register_url = format!("http://api-dweb.au:8080/dweb/v0/dwebname/register/{dweb_name}/");

    let url = if history_address.is_some() {
        format!("{register_url}{}", history_address.unwrap().to_hex())
    } else if archive_address.is_some() {
        format!("{register_url}{:x}", archive_address.unwrap())
    } else {
        format!("http://{dweb_name}.www-dweb.au:8080")
    };

    println!("DEBUG url: {url}");
    let _ = open::that(url);
}

/// Open a browser to view a website on Autonomi.
/// Requires a 'dweb server-quick' to be running which avoids the need for a local DNS to have been set up.
/// Note: the server-quick spawns a server for each directory/website being accessed, so ports will run out if the servers are never killed.
pub(crate) fn handle_browse_with_ports(
    address_name_or_link: &String,
    version: Option<u32>,
    remote_path: Option<String>,
    host: &String,
    port: Option<u16>,
) {
    if !is_main_server_with_ports_running() {
        println!("Please  start the serve-quick server before using 'dweb browse-quick'");
        println!("For help, type 'dweb serve-quick --help");
        return;
    }

    // If the main server is running it will handle the URL and spawn a new server one is not already running

    let port = port.unwrap_or(crate::services::SERVER_PORTS_MAIN_PORT);
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
    let route = format!("/dweb-link/v{version}/{address_name_or_link}/{remote_path}");

    let url = format!("http://{host}:{port}{route}");
    println!("DEBUG url: {url}");

    let _ = open::that(url);
}
