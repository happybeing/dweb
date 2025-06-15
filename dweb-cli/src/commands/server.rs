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

use color_eyre::{eyre::eyre, Report, Result};

use autonomi::AttoTokens;

use dweb::client::{ApiControl, DwebClient};
use dweb::history::HistoryAddress;
use dweb::storage::{publish_or_update_files, report_content_published_or_updated};
use dweb::token::{show_spend_return_value, Spends};
use dweb::web::request::{main_server_request, make_main_server_url};
use dweb::web::{LOCALHOST_STR, SERVER_HOSTS_MAIN_PORT, SERVER_PORTS_MAIN_PORT};

use crate::cli_options::{Opt, ServerCommands, Subcommands};

pub(crate) async fn connect_and_announce(
    local_network: bool,
    alpha_network: bool,
    api_control: ApiControl,
    announce: bool,
) -> (DwebClient, bool) {
    let client =
        dweb::client::DwebClient::initialise_and_connect(local_network, alpha_network, api_control)
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
    let (client, is_local_network) = connect_and_announce(local, alpha, api_control, true).await;

    // Start the main server (for port based browsing), which will handle /dweb-open URLs  opened by 'dweb open'
    let default_host = LOCALHOST_STR.to_string();
    let host = host.unwrap_or(default_host);
    let port = port.unwrap_or(SERVER_PORTS_MAIN_PORT);
    match crate::services::serve_with_ports(
        &client,
        None,
        host,
        Some(port),
        false,
        is_local_network,
    )
    .await
    {
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
