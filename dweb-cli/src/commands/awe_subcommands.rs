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

use color_eyre::Result;

use dweb::storage::{publish_or_update_files, report_content_published_or_updated};

use crate::cli_options::{Opt, Subcommands};

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    match opt.cmd {
        Some(Subcommands::Estimate { files_root }) => {
            let client = dweb::client::AutonomiClient::initialise_and_connect(None)
                .await
                .expect("Failed to connect to Autonomi Network");
            match client.client.file_cost(&files_root).await {
                Ok(tokens) => println!("Cost estimate: {tokens}"),
                Err(e) => println!("Unable to estimate cost: {e}"),
            }
        }
        Some(Subcommands::Publish_new {
            files_root,
            // name: String, // TODO when NRS, re-instate name
            website_config,
            is_new_network: _,
        }) => {
            let client = dweb::client::AutonomiClient::initialise_and_connect(None)
                .await
                .expect("Failed to connect to Autonomi Network");

            let (history_address, version) =
                match publish_or_update_files(&client, &files_root, None, website_config).await {
                    Ok(history_address) => history_address,
                    Err(e) => {
                        println!("Failed to publish files: {e}");
                        return Err(e);
                    }
                };

            report_content_published_or_updated(&history_address, version, &files_root, true, true);
        }
        Some(Subcommands::Publish_update {
            files_root,
            // name: String, // TODO when NRS, re-instate name
            history_address,
            website_config,
        }) => {
            let client = dweb::client::AutonomiClient::initialise_and_connect(None)
                .await
                .expect("Failed to connect to Autonomi Network");

            let (history_address, version) = publish_or_update_files(
                &client,
                &files_root,
                Some(history_address),
                website_config,
            )
            .await?;

            report_content_published_or_updated(
                &history_address,
                version,
                &files_root,
                true,
                false,
            );
        }

        Some(Subcommands::Browse {
            url: _,
            history_version: _,
        }) => {
            return Ok(false); // Command not yet complete, is the signal to start browser
        }

        Some(Subcommands::Inspect_register {
            register_address,
            print_register_summary,
            print_type,
            print_size,
            print_audit,
            print_merkle_reg,
            entries_range,
            include_files,
            files_args,
        }) => {
            match crate::commands::cmd_inspect::handle_inspect_pointer(
                register_address,
                print_register_summary,
                print_type,
                print_size,
                print_audit,
                print_merkle_reg,
                entries_range,
                include_files,
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

        Some(Subcommands::Inspect_files {
            files_metadata_address,
            files_args,
        }) => {
            match crate::commands::cmd_inspect::handle_inspect_files(
                files_metadata_address,
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

        Some(Subcommands::Serve { host: _, port: _ }) => {
            println!("TODO: implement subcommand 'serve'");
        }

        // Default is not to return, but open the browser by continuing
        None {} => {
            return Ok(false); // Command not yet complete, is the signal to start browser
        }
    }
    Ok(true)
}
