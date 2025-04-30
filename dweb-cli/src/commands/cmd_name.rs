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

use dweb::history::HistoryAddress;

pub(crate) async fn handle_name_register(
    dweb_name: String,
    history_address: HistoryAddress,
    host: Option<&String>,
    port: Option<u16>,
) -> Result<()> {
    dweb::api::name_register(&dweb_name, history_address, host, port).await
}

/// Open a browser to view a website on Autonomi.
///
/// Requires a 'dweb serve' to be running which avoids the need for a local DNS to have been set up.
///
/// Note: 'dweb' serve' spawns a server for each directory/website being accessed, so ports will run out if the servers are never killed. However, you can release all ports by restarting the server.
pub(crate) async fn handle_list_names(host: Option<&String>, port: Option<u16>) -> Result<()> {
    match dweb::api::name_list(host, port).await {
        Ok(names_vec) => {
            for recognised_name in names_vec.iter() {
                println!(
                    "{:40} {}",
                    recognised_name.key, recognised_name.history_address
                )
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
