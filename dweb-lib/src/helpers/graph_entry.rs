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
use color_eyre::{eyre::eyre, Result};
use xor_name::XorName;

use autonomi::client::key_derivation::{DerivationIndex, MainSecretKey};
use autonomi::client::vault::VaultSecretKey as SecretKey;
use autonomi::client::Client;
use autonomi::{graph::GraphError, GraphEntry, GraphEntryAddress};

/// Print a summary for a GraphEntry. If main_owner.is_some() the output
/// will use this to show the addresses of parent and descendents instead
/// of their PublicKeys.
pub fn debug_print_graph_entry(
    intro: &str,
    graph_entry: &GraphEntry,
    main_owner: Option<MainSecretKey>,
) {
    let showing_addresses = if main_owner.is_some() {
        "\n       (showing GraphEntry addresses of parents/descendents)"
    } else {
        ""
    };

    println!(
        "debug_print_graph_entry() {intro} graph entry with address {}{showing_addresses}",
        graph_entry.address().to_hex()
    );
    let parents = if graph_entry.parents.len() > 0 {
        if main_owner.is_none() {
            &graph_entry.parents[0].to_hex() // PublicKey
        } else {
            let parent_public_key = graph_entry.parents[0];
            let address = GraphEntryAddress::from_owner(parent_public_key);
            &address.to_hex()
        }
    } else {
        ""
    };

    let descendents = if graph_entry.descendants.len() > 0 {
        if main_owner.is_none() {
            &graph_entry.descendants[0].0.to_hex() // PublicKey
        } else {
            let derivation_index = get_derivation_from_graph_entry(&graph_entry);
            let descendent_public_key = main_owner
                .unwrap()
                .derive_key(&derivation_index.unwrap())
                .public_key();

            // let descendent_public_key = graph_entry.descendants[0].0;
            let address = GraphEntryAddress::from_owner(descendent_public_key.into());
            &address.to_hex()
        }
    } else {
        ""
    };

    println!(
        "\n       owner      : {}\n       parents    : [{}]\n       content    : {}\n       descendents: [{}])",
        graph_entry.owner.to_hex(),
        parents,
        hex::encode(&graph_entry.content),
        descendents
    );
}

/// Get a GraphEntry from the network
pub async fn graph_entry_get(
    client: &Client,
    graph_entry_address: &GraphEntryAddress,
) -> Result<GraphEntry> {
    // println!("DEBUG graph_entry_get() {}", graph_entry_address.to_hex());

    match client.graph_entry_get(graph_entry_address).await {
        Ok(entry) => {
            // debug_print_graph_entry("returning", &entry, None);
            Ok(entry)
        }
        Err(GraphError::Fork(entries)) => {
            println!("Forked history, {entries:?} found. Using the smallest derivation index for the next entry");
            let (entry_by_smallest_derivation, _) = if let Some(entry) = entries
                .into_iter()
                .filter_map(|e| {
                    get_derivation_from_graph_entry(&e)
                        .ok()
                        .map(|derivation| (e, derivation))
                })
                .min_by(|a, b| a.1.cmp(&b.1))
            {
                // debug_print_graph_entry("returning", &entry.0, None);
                entry
            } else {
                let msg = format!(
                    "No valid descendants found for forked entry at {graph_entry_address:?}"
                );
                println!("{msg}");
                return Err(eyre!(msg));
            };
            // debug_print_graph_entry(
            //     "returning smallest by derivation ",
            //     &entry_by_smallest_derivation,
            //     None,
            // );
            Ok(entry_by_smallest_derivation)
        }
        Err(e) => {
            let msg = format!("failed to get graph entry - {e}");
            // println!("DEBUG graph_entry_get() {msg}");
            return Err(eyre!(msg));
        }
    }
}

/// Create a new entry with the new value
pub async fn create_graph_entry(
    history_secret_key: &SecretKey,
    parent_entry: Option<&GraphEntry>,
    new_derivation: &DerivationIndex,
    new_value: XorName,
) -> Result<GraphEntry> {
    println!("DEBUG create_graph_entry()");

    let history_secret_key = MainSecretKey::new(history_secret_key.clone());
    let parents = if let Some(parent_entry) = parent_entry {
        vec![parent_entry.owner]
    } else {
        vec![]
    };

    let content: [u8; 32] = new_value.to_vec().as_slice().try_into()?;
    let entry_secret_key: SecretKey = if parent_entry.is_none() {
        history_secret_key.clone().into()
    } else {
        history_secret_key
            .clone()
            .derive_key(&new_derivation)
            .into()
    };
    let next_public_key = history_secret_key.public_key().derive_key(new_derivation);
    let next_derivation = DerivationIndex::random(&mut rand::thread_rng());
    let descendants: Vec<(PublicKey, [u8; 32])> =
        vec![(next_public_key.into(), next_derivation.into_bytes())];

    println!(
        "DEBUG entry_secret_key: {}",
        hex::encode(&history_secret_key.to_bytes())
    );
    println!(
        "DEBUG next_public_key : {}",
        hex::encode(&next_public_key.to_bytes())
    );
    let parents_str = if parents.len() > 0 {
        &parents[0].to_hex()
    } else {
        ""
    };
    println!("DEBUG creating GraphEntry::new(\n       owner      : {}\n       parents    : [{}]\n       content    : {:x}\n       descendents: [{}])",
        entry_secret_key.public_key().to_hex(), parents_str, new_value, descendants[0].0.to_hex() );

    let next_entry = GraphEntry::new(&entry_secret_key, parents, content, descendants);
    debug_print_graph_entry(
        "returning created next_entry",
        &next_entry,
        Some(history_secret_key),
    );

    Ok(next_entry)
}

/// Get a graph entry and the next derivation index (from its first descendent)
/// In normal circumstances, there is only one entry with one descendant, yielding ONE entry and ONE derivation index
/// In the case of a fork or a corrupt History, the smallest derivation index among all the entries descendants is chosen
/// We chose here to deal with the errors instead of erroring out to allow users to solve Fork and Corrupt issues by
/// updating the register
pub async fn get_graph_entry_and_next_derivation_index(
    client: &Client,
    graph_entry_addr: &GraphEntryAddress,
) -> Result<(GraphEntry, DerivationIndex)> {
    let entry = match client.graph_entry_get(graph_entry_addr).await {
        Ok(e) => e,
        Err(GraphError::Fork(entries)) => {
            println!("DEBUG Forked register, multiple entries found: {entries:?}, choosing the one with the smallest derivation index for the next entry");
            let (entry_by_smallest_derivation, _) = entries
                .into_iter()
                .filter_map(|e| {
                    get_derivation_from_graph_entry(&e)
                        .ok()
                        .map(|derivation| (e, derivation))
                })
                .min_by(|a, b| a.1.cmp(&b.1))
                .ok_or(eyre!(
                    "no valid descendants found for FORKED entry at {graph_entry_addr:?}"
                ))?;
            entry_by_smallest_derivation
        }
        Err(err) => return Err(err.into()),
    };
    let new_derivation = get_derivation_from_graph_entry(&entry)?;
    Ok((entry, new_derivation))
}

/// Get the derivation index of the first descendent
pub fn get_derivation_from_graph_entry(entry: &GraphEntry) -> Result<DerivationIndex> {
    let graph_entry_addr = GraphEntryAddress::from_owner(entry.owner);
    let d = match entry.descendants.as_slice() {
        [d] => d.1,
        // TODO maybe just use first descendent rather than error?
        _ => {
            let msg =    format!("History graph_entry_addr: {:?} is corrupted, expected one descendant but got {}: {:?}",
            graph_entry_addr,
            entry.descendants.len(),
            entry.descendants);
            println!("DEBUG get_derivation_from_graph_entry() failed: {msg}");
            return Err(eyre!(msg));
        }
    };
    Ok(DerivationIndex::from_bytes(d))
}
