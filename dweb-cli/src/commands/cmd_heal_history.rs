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
use blsttc::{PublicKey, SecretKey};
use color_eyre::{eyre::eyre, Result};

use autonomi::client::key_derivation::{DerivationIndex, MainPubkey};
use autonomi::{GraphEntry, GraphEntryAddress};

use dweb::client::DwebClient;
use dweb::files::directory::Tree;
use dweb::helpers::convert::tuple_from_address_or_name;
use dweb::history::History;
use dweb::history::{get_and_verify_pointer, pointer_address_from_history_address};

use crate::cli_options::{EntriesRange, FilesArgs};

/// Implement 'heal-history' subcommand
///
/// This command walks the History and if when it reaches the end, the pointer
/// is not in sync, it attempts to heal history by updating the pointer counter
/// and target.
pub async fn handle_heal_history(
    client: DwebClient,
    app_secret_key: SecretKey,
    name: String,
    graph_keys: bool,
    shorten_hex_strings: bool,
) -> Result<()> {
    println!("Getting History from network...");
    let mut history = match History::<Tree>::from_name(
        client.clone(),
        app_secret_key.clone(),
        name.clone(),
        true,
        0,
    )
    .await
    {
        Ok(history) => history,
        Err(e) => {
            let message = format!("Failed to heal history for '{name}' - {e}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };

    print_history(&client.clone(), &history, true, shorten_hex_strings);
    let entries_range = EntriesRange {
        start: Some(0),
        end: None,
    };
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

    // Get the actual pointer for counter value needed later
    let pointer_address = pointer_address_from_history_address(history.history_address().clone())?;
    let pointer = match get_and_verify_pointer(&client, &pointer_address).await {
        Ok(pointer) => pointer,
        Err(e) => {
            let msg = format!(
                "failed to get pointer network address {} - {e}",
                pointer_address.to_hex()
            );
            println!("DEBUG History::handle_heal_history() {msg}");
            return Err(e.into());
        }
    };

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
            &client.clone(),
            "    ",
            &entry_iter,
            graph_keys,
            true,
            shorten_hex_strings,
            Some(&history),
        )
        .await?;

        index = index + 1;
        if index <= last {
            entry_iter = match history.get_child_entry_of(&entry_iter, true).await {
                Some(entry) => entry,
                None => return Err(eyre!("failed to get child entry for history")),
            }
        }
    }

    // Check that the history (pointer target) points to the head entry (most recent entry)
    let point_target_mismatch = if let Ok(Some(head_entry)) = history.get_head_entry().await {
        entry_iter.address() != head_entry.address()
    } else {
        true // Something up so attempt to heal
    };

    if history.pointer_counter() != last || point_target_mismatch {
        println!(
            "POINTER MISMATCH, attempting to heal by putting a new pointer with correct counter and target:"
        );
        println!("GraphEntry: {}", entry_iter.address().to_hex());

        if pointer.counter() == last - 1 {
            // Use pointer_update() (which is free)
            match history
                .heal_pointer_using_update(app_secret_key.clone(), entry_iter.address())
                .await
            {
                Ok(new_counter) => {
                    println!("Pointer heaing complete");
                    println!("\nPrinting History to confirm this has been effective...\n");
                    let _ = super::cmd_inspect::handle_inspect_history(
                        client.clone(),
                        &name,
                        true,
                        Some(EntriesRange {
                            start: Some(0),
                            end: None,
                        }),
                        false,
                        true,
                        true,
                        FilesArgs::default(),
                    )
                    .await;
                }
                Err(e) => {
                    println!("Failed to heal pointer - {e}");
                    return Ok(());
                }
            }
        } else {
            // Use pointer_put() if the counter can't just be bumped by 1 (this has a cost)
            match history
                .heal_pointer_using_put(app_secret_key.clone(), last, entry_iter.address())
                .await
            {
                Ok(new_counter) => {
                    println!("Pointer heaing complete");
                    println!("\nPrinting History to confirm this has been effective...\n");
                    let _ = super::cmd_inspect::handle_inspect_history(
                        client.clone(),
                        &history.history_address().to_hex(),
                        true,
                        Some(EntriesRange {
                            start: Some(0),
                            end: None,
                        }),
                        false,
                        true,
                        true,
                        FilesArgs::default(),
                    )
                    .await;
                }
                Err(e) => {
                    println!("Failed to heal pointer - {e}");
                    return Ok(());
                }
            }
        }
    } else {
        println!("\nPointer was not in error - no changes were applied.");
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

    let mut type_string = format!("{}", hex::encode(History::<Tree>::trove_type().to_hex()));

    let mut pointer_string = if let Ok(pointer_address) =
        pointer_address_from_history_address(history.history_address())
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
            pointer_address_from_history_address(history.history_address())
        {
            format!("{}", pointer_address.to_hex())
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
                .to_hex()
        );

        head_string = if let Ok(head) = history.head_entry_address() {
            format!("{}", head.to_hex())
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

/// Print full or partial details for a GraphEntry
/// If History is Some, shows parent and descendent as network addresses rather than public keys
async fn print_graphentry(
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
