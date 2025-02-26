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

use dweb::cache::spawn::is_main_server_quick_running;
use dweb::client::AutonomiClient;
use dweb::helpers::convert::address_tuple_from_address_or_name;
use dweb::trove::HistoryAddress;
use dweb::web::LOCALHOST_STR;

/// Open a browser to view a website on Autonomi.
/// A 'dweb serve' must be running and a local DNS has been set up.
pub(crate) fn handle_browse(
    dweb_name: String,
    history_address: HistoryAddress,
    // _archive_address: Option<XorName>, // Only if I support feature("fixed-dweb-hosts")
) {
    let url = format!(
        "http://api-dweb.au:8080/dweb/v0/dwebname/register/{dweb_name}/{}",
        history_address
    );
    println!("DEBUG url: {url}");
    let _ = open::that(url);
}

/// Open a browser to view a website on Autonomi.
/// Requires a 'dweb server-quick' to be running which avoids the need for a local DNS to have been set up.
/// Note: the server-quick spawns a server for each directory/website being accessed, so ports will run out if the servers are never killed.
pub(crate) fn handle_browse_quick(
    address_or_name: &String,
    version: Option<u32>,
    remote_path: Option<String>,
    port: Option<u16>,
) {
    // let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    // if history_address.is_none() && archive_address.is_none() {
    //     println!("Error: the ADDRESS supplied is not a recognised DWEB-NAME, HISTORY-ADDRESS or ARCHIVE-ADDRESS");
    //     return;
    // }

    if !is_main_server_quick_running() {
        println!("Please  start the serve-quick server before using 'dweb browse-quick'");
        println!("For help, type 'dweb serve-quick --help");
        return;
    }

    // If the main server is running it will handle the URL and spawn a new server one is not already running

    let port = port.unwrap_or(crate::services::SERVER_QUICK_MAIN_PORT);
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
    let route = format!("/dweb-link/v{version}/{address_or_name}/{remote_path}");

    let url = format!("http://{LOCALHOST_STR}:{port}{route}");
    println!("DEBUG url: {url}");

    let _ = open::that(url);
}
