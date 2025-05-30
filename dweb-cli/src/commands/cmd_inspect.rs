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
use blsttc::PublicKey;
use chrono::offset::Utc;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Result};
use std::time::{Duration, UNIX_EPOCH};

use autonomi::client::key_derivation::{DerivationIndex, MainPubkey};
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::{GraphEntry, GraphEntryAddress, Pointer, PointerAddress};
use autonomi::{Scratchpad, ScratchpadAddress};

use dweb::client::DwebClient;
use dweb::files::directory::Tree;
use dweb::helpers::convert::tuple_from_address_or_name;
use dweb::helpers::graph_entry::graph_entry_get;
use dweb::history::History;

use crate::cli_options::{EntriesRange, FilesArgs};

/// Implement 'inspect-history' subcommand
pub async fn handle_inspect_history(
    client: DwebClient,
    address_or_name: &String,
    print_history_full: bool,
    entries_range: Option<EntriesRange>,
    include_files: bool,
    graph_keys: bool,
    shorten_hex_strings: bool,
    files_args: FilesArgs,
) -> Result<()> {
    let history_address = match tuple_from_address_or_name(address_or_name) {
        (Some(history_address), _) => history_address,
        (None, _) => {
            let msg = format!("Not a valid HISTORY-ADDRESS or recognised name: {address_or_name}");
            println!("{msg}");
            return Err(eyre!(msg));
        }
    };

    let mut history =
        match History::<Tree>::from_history_address(client.clone(), history_address, true, 0).await
        {
            Ok(history) => history,
            Err(e) => {
                let message = format!("Failed to get History from network - {e}");
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
            println!(
                "DEBUG INSPECT history.pointer_counter(): {}",
                history.pointer_counter()
            );

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
            let archive_address_hex = hex::encode(entry_iter.content);
            let archive_address = ArchiveAddress::from_hex(&archive_address_hex)?;
            if include_files {
                println!("    entry {index} - fetching content at {archive_address_hex}");
                match Tree::from_archive_address(&client, archive_address).await {
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
                entry_iter = match history.get_child_entry_of(&entry_iter, true).await {
                    Some(entry) => entry,
                    None => return Err(eyre!("failed to get child entry for history")),
                }
            };
        }
    }

    Ok(())
}

fn print_history(
    _client: &DwebClient,
    history: &History<Tree>,
    full: bool,
    shorten_hex_strings: bool,
) {
    println!("history address  : {}", history.history_address().to_hex());

    let mut type_string = format!("{}", hex::encode(History::<Tree>::trove_type().xorname()));

    let mut pointer_string = if let Ok(pointer_address) =
        History::<Tree>::pointer_address_from_history_address(history.history_address())
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
        type_string = format!("{}", History::<Tree>::trove_type());
        pointer_string = if let Ok(pointer_address) =
            History::<Tree>::pointer_address_from_history_address(history.history_address())
        {
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
    client: DwebClient,
    pointer_address: PointerAddress,
) -> Result<()> {
    let pointer = match client.client.pointer_get(&pointer_address).await {
        Ok(pointer) => pointer,
        Err(e) => {
            let message = format!("Failed to get Pointer from network - {e}");
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

/// Implement 'inspect-pointer' subcommand
pub async fn handle_inspect_scratchpad(
    client: DwebClient,
    scratchpad_address: ScratchpadAddress,
    data_as_text: bool,
) -> Result<()> {
    let scratchpad = match client.client.scratchpad_get(&scratchpad_address).await {
        Ok(scratchpad) => scratchpad,
        Err(e) => {
            let message = format!("Failed to get Scratchpad from network - {e}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };

    print_scratchpad(&scratchpad, &scratchpad_address, data_as_text);

    Ok(())
}

fn print_scratchpad(
    scratchpad: &Scratchpad,
    scratchpad_address: &ScratchpadAddress,
    data_as_text: bool,
) {
    println!("scratchpad  : {}", scratchpad_address.to_hex());
    println!("  encoding  : {:x}", scratchpad.data_encoding());
    println!("  counter   : {}", scratchpad.counter());
    println!("  owner     : {}", scratchpad.owner().to_hex());
    println!("  counter   : {}", scratchpad.counter());

    if data_as_text {
        let data_as_vec: Vec<u8> = (*scratchpad.encrypted_data()).clone().into();
        let string = String::from_utf8(data_as_vec).unwrap_or("<not a UTF8 string>".to_string());
        println!("  data      : {string}");
    } else {
        println!("  data      : {:?}", scratchpad.encrypted_data());
    }
}

/// Implement 'inspect-graphentry' subcommand
pub async fn handle_inspect_graphentry(
    client: DwebClient,
    graph_entry_address: GraphEntryAddress,
    full: bool,
    shorten_hex_strings: bool,
) -> Result<()> {
    let graph_entry = graph_entry_get(&client.client, &graph_entry_address, false).await?;

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
    _client: &DwebClient,
    indent: &str,
    graph_entry: &GraphEntry,
    graph_keys: bool,
    full: bool,
    shorten_hex_strings: bool,
    history: Option<&History<Tree>>,
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
    history: Option<&History<Tree>>,
) -> Result<()> {
    print!("{indent}  parents    : ");
    let mut parents = graph_entry.parents.iter();

    while let Some(public_key) = parents.next() {
        let mut xor_string = if history.is_none() {
            public_key.to_hex()
        } else {
            GraphEntryAddress::new(*public_key).to_hex()
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
    history: Option<&History<Tree>>,
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
            let child = GraphEntryAddress::new(next_entry_pk);
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

fn print_files(indent: &str, directory: &Tree, files_args: &FilesArgs) -> Result<()> {
    let directory_stats = directory_stats(directory)?;

    let _ = print_counts(indent, directory, directory_stats.0);
    let _ = print_total_size(indent, directory_stats.1);

    if files_args.print_paths || files_args.print_all_details {
        for (path_string, path_map) in directory.directory_map.paths_to_files_map.iter() {
            for (file_name, _datamap_chunk, data_address, metadata) in path_map.iter() {
                let data_address = if data_address.is_empty() {
                    "[private]".to_string()
                } else {
                    data_address.clone()
                };
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
                        "{indent}{} c({created}) m({modified}) \"{path_string}{file_name}\" {size} bytes and JSON: \"{extra}\"", data_address
                    );
                } else {
                    println!("{indent}{} \"{path_string}{file_name}\"", data_address);
                }
            }
        }
    }

    Ok(())
}

fn directory_stats(directory: &Tree) -> Result<(usize, u64)> {
    let mut files_count: usize = 0;
    let mut total_bytes: u64 = 0;

    for (_, directory_map) in directory.directory_map.paths_to_files_map.iter() {
        files_count = files_count + directory_map.len();

        for directory_entry in directory_map {
            total_bytes = total_bytes + directory_entry.3.size
        }
    }

    Ok((files_count, total_bytes))
}

fn print_counts(indent: &str, directory: &Tree, count_files: usize) -> Result<()> {
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
pub async fn handle_inspect_files(
    client: DwebClient,
    archive_address: ArchiveAddress,
    files_args: FilesArgs,
) -> Result<()> {
    println!("fetching directory at {}", archive_address.to_hex());
    match Tree::from_archive_address(&client, archive_address).await {
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
