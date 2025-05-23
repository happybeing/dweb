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

use dweb::cache::spawn::{detect_server_protocol, ServerProtocol};
use dweb::web::{DWEB_SERVICE_API, LOCALHOST_STR};

/// Shared function to determine protocol and build URL consistently
fn determine_protocol_and_build_url(
    host: &str,
    port: u16,
    route: &str,
) -> String {
    let (is_running, detected_protocol) = detect_server_protocol();
    
    if !is_running {
        // If no server detected, default to HTTP
        return format!("http://{host}:{port}{route}");
    }

    // Use auto-detection to determine protocol
    let use_https = if let Some(protocol) = detected_protocol {
        let detected_https = matches!(protocol, ServerProtocol::Https);
        if detected_https {
            println!("Auto-detected HTTPS server on port {}", dweb::web::DEFAULT_HTTPS_PORT);
        } else {
            println!("Auto-detected HTTP server on port {}", dweb::web::SERVER_PORTS_MAIN_PORT);
        }
        detected_https
    } else {
        // Fallback to HTTP if detection fails
        println!("Could not auto-detect protocol, defaulting to HTTP");
        false
    };
    
    let protocol = if use_https { "https" } else { "http" };
    format!("{protocol}://{host}:{port}{route}")
}

/// Open a browser to view a website on Autonomi.
///
/// A 'with hosts' server must be running and a local DNS has been set up.
/// (Start the server with 'dweb serve --experimental')
//
// TODO support --register-as?
pub(crate) fn handle_browse_with_hosts(
    _dweb_name: Option<String>,
    address_name_or_link: &String,
    version: Option<u32>,
    remote_path: Option<String>,
    host: Option<&String>,
    port: Option<u16>,
) {
    let (is_running, _) = detect_server_protocol();
    
    if !is_running {
        println!("Please start the dweb server before using 'dweb open'");
        println!("For help, type 'dweb serve --help'");
        return;
    }

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
    let url = determine_protocol_and_build_url(host, port, &route);
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
    version: Option<u32>,
    as_name: Option<String>,
    remote_path: Option<String>,
    host: Option<&String>,
    port: Option<u16>,
) {
    let (is_running, detected_protocol) = detect_server_protocol();
    
    if !is_running {
        println!("Please start the dweb server before using 'dweb open'");
        println!("For help, type 'dweb serve --help'");
        return;
    }

    // If the main server is running it will handle the URL and spawn a new server one is not already running

    let default_host = LOCALHOST_STR.to_string();
    let host = host.unwrap_or(&default_host);
    
    // Use auto-detection to determine the correct port if not explicitly set
    let port = port.unwrap_or_else(|| {
        if let Some(protocol) = detected_protocol {
            match protocol {
                ServerProtocol::Https => dweb::web::DEFAULT_HTTPS_PORT,
                ServerProtocol::Http => dweb::web::SERVER_PORTS_MAIN_PORT,
            }
        } else {
            dweb::web::SERVER_PORTS_MAIN_PORT
        }
    });
    
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
    let url = determine_protocol_and_build_url(host, port, &route);
    println!("DEBUG url: {url}");

    let _ = open::that(url);
}
