/*
 Copyright (c) 2024-2025 Mark Hughes

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

use color_eyre::{eyre::eyre, Report, Result};

use autonomi::{AttoTokens, InitialPeersConfig};

use dweb::client::AutonomiClient;
use dweb::helpers::retry;
use dweb::storage::{publish_or_update_files, report_content_published_or_updated};
use dweb::token::{show_spend_return_value, ShowCost, Spends};
use dweb::trove::HistoryAddress;
use dweb::web::{LOCALHOST_STR, SERVER_HOSTS_MAIN_PORT, SERVER_PORTS_MAIN_PORT};

use crate::cli_options::{Opt, Subcommands};

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    match opt.cmd {
        Some(Subcommands::Serve {
            experimental,
            host,
            port,
        }) => {
            let (client, is_local_network) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                opt.max_fee_per_gas,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;

            if !experimental {
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
            } else {
                // Start the server (for name based browsing), which will handle /dweb-open URLs  opened by 'dweb open --experimental'
                let default_host = dweb::web::DWEB_SERVICE_API.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_HOSTS_MAIN_PORT);
                match crate::experimental::serve_with_hosts(
                    client,
                    None,
                    host,
                    port,
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
        }

        // TODO consider detecting if the relevant server is running and if not starting automatically
        Some(Subcommands::Open {
            address_name_or_link,
            version,
            as_name,
            remote_path,
            host,
            port,
            experimental,
        }) => {
            if !experimental {
                let default_host = LOCALHOST_STR.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_PORTS_MAIN_PORT);
                crate::commands::cmd_browse::handle_browse_with_ports(
                    &address_name_or_link,
                    version,
                    as_name,
                    remote_path,
                    Some(&host),
                    Some(port),
                );
            } else {
                let default_host = dweb::web::DWEB_SERVICE_API.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_HOSTS_MAIN_PORT);
                crate::commands::cmd_browse::handle_browse_with_ports(
                    &address_name_or_link,
                    version,
                    as_name,
                    remote_path,
                    Some(&host),
                    Some(port),
                );
            }
        }

        Some(Subcommands::Name {
            dweb_name,
            history_address,
            host,
            port,
            experimental,
        }) => {
            if !experimental {
                let default_host = LOCALHOST_STR.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_PORTS_MAIN_PORT);
                match crate::commands::cmd_name::handle_name_register(
                    dweb_name,
                    history_address,
                    Some(&host),
                    Some(port),
                )
                .await
                {
                    Ok(_) => (),
                    Err(_) => (),
                };
            } else {
                let default_host = dweb::web::DWEB_SERVICE_API.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_HOSTS_MAIN_PORT);
                match crate::commands::cmd_name::handle_name_register(
                    dweb_name,
                    history_address,
                    Some(&host),
                    Some(port),
                )
                .await
                {
                    Ok(_) => (),
                    Err(_) => (),
                };
            }
        }

        Some(Subcommands::List_names {
            experimental,
            host,
            port,
        }) => {
            if !experimental {
                let default_host = LOCALHOST_STR.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_PORTS_MAIN_PORT);
                match crate::commands::cmd_name::handle_list_names(Some(&host), Some(port)).await {
                    Ok(_) => (),
                    Err(_) => (),
                }
            } else {
                let default_host = dweb::web::DWEB_SERVICE_API.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_HOSTS_MAIN_PORT);
                println!("host: {host} port: {port}");
                match crate::commands::cmd_name::handle_list_names(Some(&host), Some(port)).await {
                    Ok(_) => (),
                    Err(_) => (),
                };
            }
        }

        Some(Subcommands::Estimate { files_root }) => {
            let (client, _) =
                connect_and_announce(opt.peers, opt.show_dweb_costs, None, None, 1, true).await;
            match client.client.file_cost(&files_root).await {
                Ok(tokens) => println!("Cost estimate: {tokens}"),
                Err(e) => println!("Unable to estimate cost: {e}"),
            }
        }

        Some(Subcommands::Publish_new {
            files_root,
            name,
            dweb_settings,
            is_new_network: _,
        }) => {
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let (client, _) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                opt.max_fee_per_gas,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;
            let spends = Spends::new(&client, Some(&"Publish new cost: ")).await?;
            let (cost, name, history_address, version) = match publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                name,
                dweb_settings,
                true,
            )
            .await
            {
                Ok(result) => {
                    show_spend_return_value::<(AttoTokens, String, HistoryAddress, u32)>(
                        &spends, result,
                    )
                    .await
                }
                Err(e) => {
                    println!("Failed to publish files: {e}");
                    return show_spend_return_value::<Result<bool, Report>>(&spends, Err(e)).await;
                }
            };

            report_content_published_or_updated(
                &history_address,
                &name,
                version,
                cost,
                &files_root,
                true,
                true,
                false,
            );
        }
        Some(Subcommands::Publish_update {
            files_root,
            name,
            dweb_settings,
        }) => {
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let (client, _) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                opt.max_fee_per_gas,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;
            let spends = Spends::new(&client, Some(&"Publish update cost: ")).await?;

            let (cost, name, history_address, version) = match publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                name,
                dweb_settings,
                false,
            )
            .await
            {
                Ok(result) => {
                    show_spend_return_value::<(AttoTokens, String, HistoryAddress, u32)>(
                        &spends, result,
                    )
                    .await
                }
                Err(e) => {
                    println!("Failed to publish files: {e}");
                    return show_spend_return_value::<Result<bool, Report>>(&spends, Err(e)).await;
                }
            };

            report_content_published_or_updated(
                &history_address,
                &name,
                version,
                cost,
                &files_root,
                true,
                false,
                false,
            );
        }

        Some(Subcommands::Wallet_info {}) => {
            let (client, _) =
                connect_and_announce(opt.peers, opt.show_dweb_costs, None, None, 1, true).await;
            let tokens = client.wallet.balance_of_tokens().await?;
            let gas = client.wallet.balance_of_gas_tokens().await?;
            let network = client.network.identifier();
            let address = client.wallet.address();
            println!("Address: {address}");
            println!("    Gas: {:.28}", f32::from(gas) / 10e18);
            println!("    ANT: {:.28}", f32::from(tokens) / 10e18);
            println!("network: {network}");
        }

        Some(Subcommands::Inspect_history {
            address_or_name,
            print_history_full,
            entries_range,
            shorten_hex_strings,
            include_files,
            graph_keys,
            files_args,
        }) => {
            let (client, _) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                None,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;
            match crate::commands::cmd_inspect::handle_inspect_history(
                client,
                &address_or_name,
                print_history_full,
                entries_range,
                include_files,
                graph_keys,
                shorten_hex_strings,
                files_args,
            )
            .await
            {
                Ok(()) => return Ok(true),
                Err(e) => {
                    println!("{e:?}");
                    return Err(e);
                }
            }
        }

        Some(Subcommands::Inspect_graphentry {
            graph_entry_address,
            print_full,
            shorten_hex_strings,
        }) => {
            let (client, _) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                None,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;
            match crate::commands::cmd_inspect::handle_inspect_graphentry(
                client,
                graph_entry_address,
                print_full,
                shorten_hex_strings,
            )
            .await
            {
                Ok(()) => return Ok(true),
                Err(e) => {
                    println!("{e:?}");
                    return Err(e);
                }
            }
        }

        Some(Subcommands::Inspect_pointer { pointer_address }) => {
            let (client, _) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                None,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;
            match crate::commands::cmd_inspect::handle_inspect_pointer(client, pointer_address)
                .await
            {
                Ok(()) => return Ok(true),
                Err(e) => {
                    println!("{e:?}");
                    return Err(e);
                }
            }
        }

        Some(Subcommands::Inspect_files {
            archive_address,
            files_args,
        }) => {
            let (client, _) = connect_and_announce(
                opt.peers,
                opt.show_dweb_costs,
                None,
                opt.ignore_pointers,
                opt.retry_api,
                true,
            )
            .await;
            match crate::commands::cmd_inspect::handle_inspect_files(
                client,
                archive_address,
                files_args,
            )
            .await
            {
                Ok(_) => return Ok(true),
                Err(e) => {
                    println!("{e:?}");
                    return Err(e);
                }
            }
        }

        Some(Subcommands::Download {
            awe_url: _,
            filesystem_path: _,
            entries_range: _,
            files_args: _,
        }) => {
            println!("TODO: implement subcommand 'download'");
        }

        // Some(Subcommands::Awesome {}) => {
        //     let site_address = if opt.peers.is_local() {
        //         crate::generated_rs::builtins_local::AWESOME_SITE_HISTORY_LOCAL
        //     } else {
        //         crate::generated_rs::builtins_public::AWESOME_SITE_HISTORY_PUBLIC
        //     };

        //     // TODO replace components with const strings in format():
        //     let url = format!(
        //         "http://api-dweb.au:8080/dweb/v0/dwebname/register/awesome/{}",
        //         site_address
        //     );
        //     println!("DEBUG url: {url}");
        //     let _ = open::that(url);
        // }

        // Default is not to return, but open the browser by continuing
        None {} => {
            println!("No command provided, try 'dweb --help'");
            return Ok(false); // Command not yet complete, is the signal to start browser
        }
    }
    Ok(true)
}

async fn connect_and_announce(
    peers: InitialPeersConfig,
    show_cost: ShowCost,
    max_fee_per_gas: Option<u128>,
    ignore_pointers: Option<bool>,
    retry_api: u32,
    announce: bool,
) -> (AutonomiClient, bool) {
    let is_local_network = peers.local;
    let client = dweb::client::AutonomiClient::initialise_and_connect(
        peers,
        show_cost,
        max_fee_per_gas,
        ignore_pointers,
        retry_api,
    )
    .await
    .expect("Failed to connect to Autonomi Network");

    if announce {
        if is_local_network {
            println!("-> local network: {}", client.network);
        } else {
            println!("-> public network {}", client.network);
        };
    };

    (client, is_local_network)
}
