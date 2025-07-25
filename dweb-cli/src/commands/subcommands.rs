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

use autonomi::AttoTokens;

use dweb::client::ApiControl;
use dweb::history::HistoryAddress;
use dweb::storage::{publish_or_update_files, report_content_published_or_updated};
use dweb::token::{show_spend_return_value, Spends};
use dweb::web::request::{main_server_request, make_main_server_url};
use dweb::web::{LOCALHOST_STR, SERVER_HOSTS_MAIN_PORT, SERVER_PORTS_MAIN_PORT};

use crate::cli_options::{Opt, ServerCommands, Subcommands};
use crate::commands::server::connect_and_announce;

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    let api_control = ApiControl {
        api_tries: opt.retry_api,
        chunk_retries: opt.retry_failed,
        upload_file_by_file: opt.upload_file_by_file,
        ignore_pointers: opt.ignore_pointers,
        max_fee_per_gas: opt.transaction_opt.max_fee_per_gas,
        use_public_archive: opt.use_old_archive,
        ..Default::default()
    };

    match opt.cmd {
        Some(Subcommands::Serve {
            experimental,
            host,
            port,
        }) => {
            if !experimental {
                let _ = super::server::start_in_foreground(
                    opt.local,
                    opt.alpha,
                    api_control,
                    host,
                    port,
                    None,
                )
                .await;
            } else {
                let default_host = dweb::web::DWEB_SERVICE_API.to_string();
                let host = host.unwrap_or(default_host);
                let port = port.unwrap_or(SERVER_HOSTS_MAIN_PORT);
                let (client, is_local_network) = connect_and_announce(
                    opt.local,
                    opt.alpha,
                    Some(host),
                    Some(port),
                    api_control,
                    true,
                )
                .await;
                // Start the server (for name based browsing), which will handle /dweb-open URLs  opened by 'dweb open --experimental'
                match crate::experimental::serve_with_hosts(client, None, is_local_network).await {
                    Ok(_) => return Ok(true),
                    Err(e) => {
                        println!("{e:?}");
                        return Err(eyre!(e));
                    }
                }
            }
        }

        Some(Subcommands::Server { command }) => match command {
            ServerCommands::Start {
                host,
                port,
                foreground,
                logdir,
            } => {
                if foreground {
                    let _ = super::server::start_in_foreground(
                        opt.local,
                        opt.alpha,
                        api_control,
                        host,
                        port,
                        logdir,
                    )
                    .await;
                } else {
                    let _ = super::server::start_in_background(
                        opt.local,
                        opt.alpha,
                        api_control,
                        host,
                        port,
                        logdir,
                    );
                }
            }

            ServerCommands::Stop { port_or_all } => { // TODO
            }

            ServerCommands::Info { port_or_all } => { // TODO
            }
        },

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
                crate::commands::cmd_browse::handle_browse_with_ports(
                    &address_name_or_link,
                    version,
                    as_name,
                    remote_path,
                    host,
                    port,
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
                    Some(host),
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
                connect_and_announce(opt.local, opt.alpha, None, None, api_control, true).await;
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
            let (client, _) =
                connect_and_announce(opt.local, opt.alpha, None, None, api_control, true).await;
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
                    show_spend_return_value::<(AttoTokens, String, HistoryAddress, u64)>(
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
            let (client, _) =
                connect_and_announce(opt.local, opt.alpha, None, None, api_control, true).await;
            let spends = Spends::new(&client, Some(&"Publish update cost: ")).await?;

            let name = if name.is_none() {
                if let Some(osstr) = files_root.file_name() {
                    dweb::files::directory::osstr_to_string(osstr)
                } else {
                    None
                }
            } else {
                name
            };
            let name = if let Some(name) = name {
                // TODO re-instate once Autonomi have made Pointers reliable:
                // match crate::commands::cmd_heal_history::handle_heal_history(
                //     client.clone(),
                //     app_secret_key.clone(),
                //     &name.clone(),
                //     false,
                //     false,
                //     true,
                // )
                // .await
                // {
                //     Ok(()) => {}
                //     Err(e) => {
                //         println!("{e:?}");
                //         return Err(e);
                //     }
                // }

                name
            } else {
                return Err(eyre!(
                    "DEBUG failed to obtain directory name from files_root: {files_root:?}"
                ));
            };

            let (cost, name, history_address, version) = match publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                Some(name),
                dweb_settings,
                false,
            )
            .await
            {
                Ok(result) => {
                    show_spend_return_value::<(AttoTokens, String, HistoryAddress, u64)>(
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
            let (client, _) = connect_and_announce(
                opt.local,
                opt.alpha,
                None,
                None,
                ApiControl::default(),
                true,
            )
            .await;
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
            let api_control = ApiControl {
                ignore_pointers: true,
                ..Default::default()
            };
            let (client, _) =
                connect_and_announce(opt.local, opt.alpha, None, None, api_control, true).await;
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

        Some(Subcommands::Heal_history {
            name,
            print_history_full,
            shorten_hex_strings,
            graph_keys,
        }) => {
            let api_control = ApiControl {
                ignore_pointers: true,
                ..Default::default()
            };
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let (client, _) =
                connect_and_announce(opt.local, opt.alpha, None, None, api_control, true).await;
            match crate::commands::cmd_heal_history::handle_heal_history(
                client,
                app_secret_key,
                &name,
                print_history_full,
                graph_keys,
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

        Some(Subcommands::Inspect_graphentry {
            graph_entry_address,
            print_full,
            shorten_hex_strings,
        }) => {
            let api_control = ApiControl {
                ..Default::default()
            };
            let (client, _) =
                connect_and_announce(opt.local, opt.alpha, None, None, api_control, true).await;
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
                opt.local,
                opt.alpha,
                None,
                None,
                ApiControl::default(),
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

        Some(Subcommands::Inspect_scratchpad {
            scratchpad_address,
            data_as_text,
        }) => {
            let (client, _) = connect_and_announce(
                opt.local,
                opt.alpha,
                None,
                None,
                ApiControl::default(),
                true,
            )
            .await;
            match crate::commands::cmd_inspect::handle_inspect_scratchpad(
                client,
                scratchpad_address,
                data_as_text,
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

        Some(Subcommands::Inspect_files {
            archive_address,
            files_args,
        }) => {
            let (client, _) = connect_and_announce(
                opt.local,
                opt.alpha,
                None,
                None,
                ApiControl::default(),
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

        Some(Subcommands::Openapi_docs { print, host, port }) => {
            if !dweb::cache::spawn::is_main_server_with_ports_running() {
                println!("Please  start the dweb server before using 'dweb openapi-docs'");
                println!("For help, type 'dweb serve --help");
                return Ok(true);
            }

            let host_string = match host.clone() {
                Some(host) => host,
                None => "".to_string(),
            };
            let host_ref = if host.is_some() {
                Some(&host_string)
            } else {
                None
            };

            if print {
                match main_server_request(host_ref, port, crate::services::openapi::JSON_PATH).await
                {
                    Ok(json) => {
                        println!("{json}");
                    }
                    Err(e) => {
                        println!("Error fetching openapi.json from server - {e}");
                    }
                }
            } else {
                let url =
                    make_main_server_url(host_ref, port, crate::services::openapi::SWAGGER_UI);
                let _ = open::that(url);
            }
            return Ok(true);
        }

        // Default is not to return, but open the browser by continuing
        None {} => {
            println!("No command provided, try 'dweb --help'");
            return Ok(false); // Command not yet complete, is the signal to start browser
        }
    }
    Ok(true)
}
