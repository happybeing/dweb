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

// TODO implement a lazy static map of handles and basic details of each spawned server
// TODO when the main server app shuts down it can shut these down (if that is needed?)
// TODO web API and CLI for listing active ports and what they are serving
// TODO see TODOs in serve_with_ports()

use std::net::TcpStream;
use std::time::Duration;
use crate::web::{SERVER_PORTS_MAIN_PORT, DEFAULT_HTTPS_PORT, LOCALHOST_STR};

#[derive(Debug, Clone, Copy)]
pub enum ServerProtocol {
    Http,
    Https,
}

pub fn is_main_server_with_ports_running() -> bool {
    return true; // TODO look-up the main server in the spawned servers struct
}

/// Detect if a dweb server is running and which protocol (HTTP/HTTPS) it uses
/// Returns (is_running, protocol) where protocol is None if no server is detected
pub fn detect_server_protocol() -> (bool, Option<ServerProtocol>) {
    let timeout = Duration::from_millis(500);
    
    // Try HTTPS port first (8443)
    if let Ok(_) = TcpStream::connect_timeout(
        &format!("{}:{}", LOCALHOST_STR, DEFAULT_HTTPS_PORT).parse().unwrap(),
        timeout
    ) {
        return (true, Some(ServerProtocol::Https));
    }
    
    // Try HTTP port (8080)
    if let Ok(_) = TcpStream::connect_timeout(
        &format!("{}:{}", LOCALHOST_STR, SERVER_PORTS_MAIN_PORT).parse().unwrap(),
        timeout
    ) {
        return (true, Some(ServerProtocol::Http));
    }
    
    (false, None)
}
