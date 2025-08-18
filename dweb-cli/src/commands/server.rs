/*
 Copyright (c) 2025- Mark Hughes

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU Affero General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU Affero
  Public License for more details.

 You should have received a copy of the GNU Affero General Public License
 along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use color_eyre::{eyre::eyre, Result};

use dweb::client::{ApiControl, DwebClient};

pub(crate) async fn connect_and_announce(
    local_network: bool,
    alpha_network: bool,
    host: Option<String>,
    port: Option<u16>,
    api_control: ApiControl,
    announce: bool,
) -> (DwebClient, bool) {
    let client = dweb::client::DwebClient::initialise_and_connect(
        local_network,
        alpha_network,
        host,
        port,
        None,
        None,
        api_control,
    )
    .await
    .expect("Failed to connect to Autonomi Network");

    if announce {
        if local_network {
            println!("-> local network: {}", client.network);
        } else if alpha_network {
            println!("-> alpha network {}", client.network);
        } else {
            println!("-> public network {}", client.network);
        };
    };

    (client, local_network)
}

pub(crate) async fn start_in_foreground(
    local: bool,
    alpha: bool,
    api_control: ApiControl,
    host: Option<String>,
    port: Option<u16>,
    logdir: Option<String>,
) -> Result<bool> {
    let (client, is_local_network) =
        connect_and_announce(local, alpha, host, port, api_control, true).await;

    // Start the main server (for port based browsing), which will handle /dweb-open URLs  opened by 'dweb open'
    let host = client.host.clone();
    let port = client.port;
    match crate::services::serve_with_ports(&client, None, false, is_local_network).await {
        Ok(_) => return Ok(true),
        Err(e) => {
            println!("{e:?}");
            return Err(eyre!(e));
        }
    }
}

pub async fn start_in_background(
    local: bool,
    alpha: bool,
    api_control: ApiControl,
    host: Option<String>,
    port: Option<u16>,
    logdir: Option<String>,
) {
}
