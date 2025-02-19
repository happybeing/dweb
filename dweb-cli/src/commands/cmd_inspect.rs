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
use chrono::offset::Utc;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Result};
use std::time::{Duration, UNIX_EPOCH};
use xor_name::XorName;

use ant_protocol::storage::{GraphEntry, GraphEntryAddress, Pointer, PointerAddress};
use autonomi::client::key_derivation::{DerivationIndex, MainPubkey};
use blsttc::PublicKey;

use dweb::autonomi::access::network::NetworkPeers;
use dweb::client::AutonomiClient;
use dweb::helpers::convert::str_to_xor_name;
use dweb::helpers::graph_entry::graph_entry_get;
use dweb::trove::History;
use dweb::trove::{directory_tree::DirectoryTree, HistoryAddress};

use crate::cli_options::{EntriesRange, FilesArgs};

/// Implement 'inspect-history' subcommand
pub async fn handle_inspect_history(
    peers: NetworkPeers,
    history_address: HistoryAddress,
    print_history_full: bool,
    entries_range: Option<EntriesRange>,
    include_files: bool,
    graph_keys: bool,
    shorten_hex_strings: bool,
    files_args: FilesArgs,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    let mut history = match History::<DirectoryTree>::from_history_address(
        client.clone(),
        history_address,
        true,
        0,
    )
    .await
    {
        Ok(pointer) => pointer,
        Err(e) => {
            let message = format!("Failed to get History pointer from network - {e}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };

    print_history(&client, &history, print_history_full, shorten_hex_strings);
    if let Some(entries_range) = entries_range {
        let size = history.num_entries();
        let first = if entries_range.start.is_some() {
            entries_range.start.unwrap()
        } else {
            0
        };

        let last = if entries_range.end.is_some() {
            entries_range.end.unwrap()
        } else {
            size - 1
        };

        if last > size - 1 {
            return Err(eyre!(
                "range exceeds maximum register entry which is {}",
                size - 1
            ));
        }

        println!("  entries {first} to {last:2}:");
        let mut index = first;
        let mut entry_iter = history.get_graph_entry(index).await?;

        while index <= last {
            let pointer_indicator = if history.pointer_counter() == index {
                "P>"
            } else {
                "  "
            };
            println!("{pointer_indicator}  entry {index:4.}:");
            print_graphentry(
                &client,
                "    ",
                &entry_iter,
                graph_keys,
                print_history_full,
                shorten_hex_strings,
                Some(&history),
            )
            .await?;
            let archive_address = str_to_xor_name(&hex::encode(entry_iter.content))?;
            if include_files {
                println!("    entry {index} - fetching content at {archive_address:x}");
                match DirectoryTree::from_archive_address(&client, archive_address).await {
                    Ok(directory) => {
                        let _ = print_files("      ", &directory, &files_args);
                    }
                    Err(e) => {
                        println!("Failed to get website directory from network");
                        return Err(eyre!(e));
                    }
                };
            }
            index = index + 1;
            if index <= last {
                entry_iter = match history.get_child_entry_of(&entry_iter).await {
                    Some(entry) => entry,
                    None => return Err(eyre!("failed to get child entry for history")),
                }
            };
        }
    }

    Ok(())
}

fn print_history(
    _client: &AutonomiClient,
    history: &History<DirectoryTree>,
    full: bool,
    shorten_hex_strings: bool,
) {
    println!("history address  : {}", history.history_address().to_hex());

    let mut type_string = format!("{}", hex::encode(History::<DirectoryTree>::trove_type()));

    let mut pointer_string = if let Ok(pointer_address) =
        History::<DirectoryTree>::pointer_address_from_history_address(history.history_address())
    {
        pointer_address.to_hex()
    } else {
        String::from("history.pointer_address_from_history_address() not valid - probably a bug")
    };

    let mut root_string = history
        .history_address()
        .to_underlying_graph_root()
        .to_hex();

    let mut head_string = if let Ok(head) = history.head_entry_address() {
        head.to_hex()
    } else {
        String::from("history.head_entry_address() not valid - probably a bug")
    };

    if shorten_hex_strings {
        type_string = format!("{}", History::<DirectoryTree>::trove_type());
        pointer_string = if let Ok(pointer_address) =
            History::<DirectoryTree>::pointer_address_from_history_address(
                history.history_address(),
            ) {
            format!("{}", pointer_address.xorname())
        } else {
            String::from(
                "history.pointer_address_from_history_address() not valid - probably a bug",
            )
        };
        root_string = format!(
            "{}",
            history
                .history_address()
                .to_underlying_graph_root()
                .xorname()
        );

        head_string = if let Ok(head) = history.head_entry_address() {
            format!("{}", head.xorname())
        } else {
            String::from("history.head_entry_address() not valid - probably a bug")
        };
    }

    println!("  type           : {type_string}",);
    println!("  size           : {}", history.num_entries());

    if full {
        println!("  pointer address: {pointer_string}");
        println!("  graph root     : {root_string}");
        println!("  graph head     : {head_string}");
    }
}

/// Implement 'inspect-pointer' subcommand
pub async fn handle_inspect_pointer(
    peers: NetworkPeers,
    pointer_address: PointerAddress,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    let pointer = match client.client.pointer_get(&pointer_address).await {
        Ok(pointer) => pointer,
        Err(e) => {
            let message = format!("Failed to get pointer from network - {e}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };

    print_pointer(&pointer, &pointer_address);

    Ok(())
}

fn print_pointer(pointer: &Pointer, pointer_address: &PointerAddress) {
    println!("pointer     : {}", pointer_address.to_hex());
    println!("  target    : {:x}", pointer.target().xorname());
    println!("  counter   : {}", pointer.counter());
}

/// Implement 'inspect-graphentry' subcommand
pub async fn handle_inspect_graphentry(
    peers: NetworkPeers,
    graph_entry_address: GraphEntryAddress,
    full: bool,
    shorten_hex_strings: bool,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    let graph_entry = graph_entry_get(&client.client, &graph_entry_address).await?;

    print_graphentry(
        &client,
        "",
        &graph_entry,
        false,
        full,
        shorten_hex_strings,
        None,
    )
    .await?;
    Ok(())
}

/// Print full or partial details for a GraphEntry
/// If History is Some, shows parent and descendent as network addresses rather than public keys
pub async fn print_graphentry(
    _client: &AutonomiClient,
    indent: &str,
    graph_entry: &GraphEntry,
    graph_keys: bool,
    full: bool,
    shorten_hex_strings: bool,
    history: Option<&History<DirectoryTree>>,
) -> Result<()> {
    let history = if graph_keys { None } else { history };
    if full {
        graph_entry_print_address(indent, &graph_entry.address());
        graph_entry_print_owner(indent, &graph_entry, shorten_hex_strings);
        let _ = graph_entry_print_parents(indent, &graph_entry, shorten_hex_strings, history).await;
        graph_entry_print_descendents(indent, &graph_entry, shorten_hex_strings, history);
        graph_entry_print_content(indent, &graph_entry, shorten_hex_strings);
        graph_entry_print_signature(indent, &graph_entry, shorten_hex_strings);
    } else {
        graph_entry_print_address(indent, &graph_entry.address());
        graph_entry_print_content(indent, &graph_entry, shorten_hex_strings);
    }

    Ok(())
}

fn graph_entry_print_address(indent: &str, graph_entry_address: &GraphEntryAddress) {
    println!("{indent}address   : {}", graph_entry_address.to_hex());
}

fn graph_entry_print_owner(indent: &str, graph_entry: &GraphEntry, shorten_hex_strings: bool) {
    let mut hex_string = graph_entry.owner.to_hex();
    if shorten_hex_strings {
        hex_string = String::from(&format!("{hex_string:.6}.."));
    };

    println!("{indent}  owner      : {hex_string}");
}

/// If history is Some prints address rather than public key of parent(s)
async fn graph_entry_print_parents(
    indent: &str,
    graph_entry: &GraphEntry,
    shorten_hex_strings: bool,
    history: Option<&History<DirectoryTree>>,
) -> Result<()> {
    print!("{indent}  parents    : ");
    let mut parents = graph_entry.parents.iter();

    while let Some(public_key) = parents.next() {
        let mut xor_string = if history.is_none() {
            public_key.to_hex()
        } else {
            GraphEntryAddress::from_owner(*public_key).to_hex()
        };

        if shorten_hex_strings {
            xor_string = String::from(&format!("{xor_string:.6}.."));
        };
        print!("[{xor_string}] ");
    }
    println!("");
    Ok(())
}

/// If history is Some prints address rather than public key of parent(s)
fn graph_entry_print_descendents(
    indent: &str,
    graph_entry: &GraphEntry,
    shorten_hex_strings: bool,
    history: Option<&History<DirectoryTree>>,
) {
    print!("{indent}  descendents: ");
    let mut descendents = graph_entry.descendants.iter();
    while let Some((public_key, derivation_index)) = descendents.next() {
        let mut xor_string = if history.is_none() {
            public_key.to_hex()
        } else {
            let next_derivation = DerivationIndex::from_bytes(*derivation_index);
            let next_entry_pk: PublicKey =
                MainPubkey::from(history.as_ref().unwrap().history_address().owner)
                    .derive_key(&next_derivation)
                    .into();
            let child = GraphEntryAddress::from_owner(next_entry_pk);
            child.to_hex()
        };

        if shorten_hex_strings {
            xor_string = String::from(&format!("{xor_string:.6}.."));
        };
        print!("[{xor_string}] ");
    }
    println!("");
}

fn graph_entry_print_content(indent: &str, graph_entry: &GraphEntry, shorten_hex_strings: bool) {
    let mut hex_string: String = hex::encode(&graph_entry.content);
    if shorten_hex_strings {
        hex_string = String::from(&format!("{hex_string:.6}.."));
    };

    println!("{indent}  content    : {hex_string}",);
}

fn graph_entry_print_signature(indent: &str, graph_entry: &GraphEntry, shorten_hex_strings: bool) {
    let mut hex_string: String = hex::encode(&graph_entry.signature.to_bytes());
    if shorten_hex_strings {
        hex_string = String::from(&format!("{hex_string:.6}.."));
    };

    println!("{indent}  signature  : {hex_string}");
}

fn print_files(indent: &str, directory: &DirectoryTree, files_args: &FilesArgs) -> Result<()> {
    let directory_stats = directory_stats(directory)?;

    let _ = print_counts(indent, directory, directory_stats.0);
    let _ = print_total_size(indent, directory_stats.1);

    if files_args.print_paths || files_args.print_all_details {
        for (path_string, path_map) in directory.directory_map.paths_to_files_map.iter() {
            for (file_name, xor_name, metadata) in path_map.iter() {
                if files_args.print_all_details {
                    let created: DateTime<Utc> =
                        (UNIX_EPOCH + Duration::from_secs(metadata.created)).into();
                    let modified: DateTime<Utc> =
                        (UNIX_EPOCH + Duration::from_secs(metadata.modified)).into();
                    let created = created.format("%Y-%m-%d %H:%M:%S").to_string();
                    let modified = modified.format("%Y-%m-%d %H:%M:%S").to_string();

                    let size = metadata.size;
                    let extra = metadata.extra.clone().unwrap_or(String::from(""));
                    println!(
                        "{indent}{xor_name:x} c({created}) m({modified}) \"{path_string}{file_name}\" {size} bytes and JSON: \"{extra}\"",
                    );
                } else {
                    println!("{indent}{xor_name:x} \"{path_string}{file_name}\"");
                }
            }
        }
    }

    Ok(())
}

fn directory_stats(directory: &DirectoryTree) -> Result<(usize, u64)> {
    let mut files_count: usize = 0;
    let mut total_bytes: u64 = 0;

    for (_, directory_map) in directory.directory_map.paths_to_files_map.iter() {
        files_count = files_count + directory_map.len();

        for directory_entry in directory_map {
            total_bytes = total_bytes + directory_entry.2.size
        }
    }

    Ok((files_count, total_bytes))
}

fn print_counts(indent: &str, directory: &DirectoryTree, count_files: usize) -> Result<()> {
    println!(
        "{indent}directories: {}",
        directory.directory_map.paths_to_files_map.len()
    );
    println!("{indent}files      : {count_files}");
    Ok(())
}

fn print_total_size(indent: &str, total_bytes: u64) -> Result<()> {
    println!("{indent}total bytes: {total_bytes}");
    Ok(())
}

/// Implement 'inspect-files' subcommand
///
pub async fn handle_inspect_files(
    peers: NetworkPeers,
    archive_address: XorName,
    files_args: FilesArgs,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    println!("fetching directory at {archive_address:x}");
    match DirectoryTree::from_archive_address(&client, archive_address).await {
        Ok(directory) => {
            let _ = print_files("", &directory, &files_args);
        }
        Err(e) => {
            println!("Failed to get website directory from network");
            return Err(eyre!(e).into());
        }
    };
    Ok(())
}
