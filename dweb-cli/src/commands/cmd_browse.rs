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

use dweb::client::DwebClientConfig;
use dweb::web::LOCALHOST_STR;

/// Open a browser to view a website on Autonomi. Assumes a dweb server is running
///
/// Note: the server will spawn a dedicated server per directory/website being accessed, so ports will
/// run out if the servers are never killed.

// TODO support --register-as
pub(crate) fn dweb_create_and_open_url(
    address_name_or_link: &String,
    version: Option<u64>,
    as_name: Option<String>,
    remote_path: Option<String>,
    host: String,
    port: u16,
) {
    println!("DEBUG dweb_create_and_open_url()...");
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
        format!("/dweb-open-as/v{version}/{as_name}/{address_name_or_link}{remote_path}")
    } else {
        format!("/dweb-open/v{version}/{address_name_or_link}{remote_path}")
    };
    let url = format!("http://{host}:{port}{route}");
    println!("DEBUG url: {url}");

    let _ = open::that(url);
}

pub(crate) async fn open_in_browser(
    address_name_or_link: &String,
    version: Option<u64>,
    as_name: Option<String>,
    remote_path: Option<String>,
    client_config: Option<DwebClientConfig>,
) {
    println!("DEBUG open_in_browser()...");
    let mut client_config = if let Some(client_config) = client_config {
        client_config
    } else {
        DwebClientConfig::default()
    };

    let host = client_config.host.unwrap_or(LOCALHOST_STR.to_string());
    let port = client_config
        .port
        .unwrap_or(dweb::web::SERVER_PORTS_MAIN_PORT);

    client_config.host = Some(host.clone());
    client_config.port = Some(port);

    // Open the URL in the browser
    dweb_create_and_open_url(
        &address_name_or_link,
        version,
        as_name,
        remote_path,
        host.clone(),
        port,
    );

    // Check if there's a dweb server to handle it
    let is_dweb_server_running = if !port_check::is_local_ipv4_port_free(port) {
        // The port is in use
        // TODO check if it is a dweb server (but for now assume it is)
        true
    } else {
        false
    };

    if !is_dweb_server_running {
        // Make builtin names such as 'awesome' available (in addition to opening xor addresses)
        dweb::web::name::register_builtin_names(false);

        println!("Starting main dweb server at {host}:{port}...");
        let mut service = dweb_server::DwebService::new(client_config);
        service.start_blocking(port).await;
    }
}
