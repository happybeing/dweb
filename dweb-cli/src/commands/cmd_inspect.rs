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
use xor_name::XorName;

use ant_protocol::storage::{Pointer, PointerAddress};

use dweb::autonomi::access::network::NetworkPeers;
use dweb::trove::directory_tree::DirectoryTree;

use crate::cli_options::{EntriesRange, FilesArgs};

/// Implement 'inspect-history' subcommand
pub async fn handle_inspect_pointer(
    peers: NetworkPeers,
    pointer_address: PointerAddress,
    print_summary: bool,
    print_type: bool,
    print_size: bool,
    print_audit: bool,
    print_merkle_reg: bool,
    entries_range: Option<EntriesRange>,
    include_files: bool,
    files_args: FilesArgs,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    let pointer = match client.client.pointer_get(pointer_address).await {
        Ok(pointer) => pointer,
        Err(e) => {
            let message = format!("Failed to get pointer from network - {e}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };

    let count = pointer.counter();
    if print_summary {
        do_print_summary(&pointer, &pointer_address)?;
    } else {
        if print_type {
            // if size > 0 {
            //     do_print_type(Some(&entries_vec[0]))?;
            // } else {
            //     do_print_type(None)?;
            // }
        }

        if print_size {
            // do_print_size(size)?;
        }
    }

    // if let Some(entries_range) = entries_range {
    //     do_print_entries(
    //         &client,
    //         &entries_range,
    //         entries_vec,
    //         include_files,
    //         &files_args,
    //     )
    //     .await?;
    // };

    // if print_audit {
    //     let _ = do_print_audit(&register);
    // }

    // if print_merkle_reg {
    //     let _ = do_print_merkle_reg(&register);
    // }

    Ok(())
}

fn do_print_summary(pointer: &Pointer, pointer_address: &PointerAddress) -> Result<()> {
    println!("pointer     : {}", pointer_address.to_hex());
    // println!("owner       : {:?}", pointer.owner());
    // println!("permissions : {:?}", pointer.permissions());
    println!("count       : {:?}", pointer.counter());

    // if entries_vec.len() > 0 {
    //     do_print_type(Some(&entries_vec[0]))?;
    // } else {
    //     do_print_type(None)?;
    // }
    // do_print_size(size)?;
    // do_print_audit_summary(&register)?;
    Ok(())
}

fn do_print_type(history_type: Option<XorName>) -> Result<()> {
    if history_type.is_some() {
        println!("history type: {:64x}", history_type.unwrap());
    } else {
        println!("history type: not set");
    }
    Ok(())
}

fn do_print_size(size: usize) -> Result<()> {
    println!("size        : {size}");
    Ok(())
}

// TODO refactor all Register refs to use Transactions when available
// fn do_print_merkle_reg(register: &Register) -> Result<()> {
//     println!("{:?}", register.inner_merkle_reg());
//     Ok(())
// }

// fn do_print_audit_summary(register: &Register) -> Result<()> {
//     let merkle_reg = register.inner_merkle_reg();
//     let content = merkle_reg.read();

//     if content.nodes().nth(0).is_some() {
//         println!("audit       :");
//         // Print current/root value(s)
//         // The 'roots' are one or more current values
//         // We always write and merge so this should always be a single value
//         let num_root_values = content.values().into_iter().count();

//         if num_root_values == 1 {
//             println!("   current state is merged, 1 value:");
//         } else {
//             println!("   current state is NOT merged, {num_root_values} values:");
//         }
//         for value in content.values().into_iter() {
//             let xor_name = xorname_from_entry(value);
//             println!("   {xor_name:64x}");
//         }
//     } else {
//         println!("audit       : empty (no values)");
//     }

//     Ok(())
// }

// fn do_print_audit(register: &Register) -> Result<()> {
//     let merkle_reg = register.inner_merkle_reg();
//     let content = merkle_reg.read();

//     let node = content.nodes().nth(0);
//     if let Some(_node) = node {
//         print_audit_for_nodes(merkle_reg);
//     } else {
//         println!("history     : empty (no values)");
//     }

//     Ok(())
// }

// fn print_audit_for_nodes(merkle_reg: &crdts::merkle_reg::MerkleReg<Entry>) {
//     // Show the Register structure
//     let content = merkle_reg.read();

//     // Index nodes to make it easier to see where a
//     // node appears multiple times in the output.
//     // Note: it isn't related to the order of insertion
//     // which is hard to determine.
//     let mut index: usize = 0;
//     let mut node_ordering: HashMap<Hash, usize> = HashMap::new();
//     for (_hash, node) in content.hashes_and_nodes() {
//         index_node_and_descendants(node, &mut index, &mut node_ordering, merkle_reg);
//     }

//     println!("======================");
//     println!("Root (Latest) Node(s):");
//     for node in content.nodes() {
//         let _ = print_node(0, node, &node_ordering);
//     }

//     println!("======================");
//     println!("Register Structure:");
//     println!("(In general, earlier nodes are more indented)");
//     let mut indents = 0;
//     for (_hash, node) in content.hashes_and_nodes() {
//         print_node_and_descendants(&mut indents, node, &node_ordering, merkle_reg);
//     }

//     println!("======================");
// }

// fn index_node_and_descendants(
//     node: &Node<Entry>,
//     index: &mut usize,
//     node_ordering: &mut HashMap<Hash, usize>,
//     merkle_reg: &MerkleReg<Entry>,
// ) {
//     let node_hash = node.hash();
//     if node_ordering.get(&node_hash).is_none() {
//         node_ordering.insert(node_hash, *index);
//         *index += 1;
//     }

//     for child_hash in node.children.iter() {
//         if let Some(child_node) = merkle_reg.node(*child_hash) {
//             index_node_and_descendants(child_node, index, node_ordering, merkle_reg);
//         } else {
//             println!("ERROR looking up hash of child");
//         }
//     }
// }

// fn print_node_and_descendants(
//     indents: &mut usize,
//     node: &Node<Entry>,
//     node_ordering: &HashMap<Hash, usize>,
//     merkle_reg: &MerkleReg<Entry>,
// ) {
//     let _ = print_node(*indents, node, node_ordering);

//     *indents += 1;
//     for child_hash in node.children.iter() {
//         if let Some(child_node) = merkle_reg.node(*child_hash) {
//             // let xor_name = xorname_from_entry(child_node.value());
//             print_node_and_descendants(indents, child_node, node_ordering, merkle_reg);
//         }
//     }
//     *indents -= 1;
// }

// fn print_node(
//     indents: usize,
//     node: &Node<Entry>,
//     node_ordering: &HashMap<Hash, usize>,
// ) -> Result<()> {
//     let order = match node_ordering.get(&node.hash()) {
//         Some(order) => format!("{order}"),
//         None => String::new(),
//     };

//     let indentation = "  ".repeat(indents);
//     let node_entry = xorname_from_entry(&node.value);
//     println!(
//         "{indentation}[{order:>2}] Node({:?}..) Entry({node_entry:64x})",
//         order
//     );
//     Ok(())
// }

// async fn do_print_entries(
//     client: &AutonomiClient,
//     entries_range: &EntriesRange,
//     entries_vec: Vec<Entry>,
//     include_files: bool,
//     files_args: &FilesArgs,
// ) -> Result<()> {
//     let size = entries_vec.len();
//     if size == 0 {
//         return Ok(());
//     }

//     let first = if entries_range.start.is_some() {
//         entries_range.start.unwrap()
//     } else {
//         0
//     };

//     let last = if entries_range.end.is_some() {
//         entries_range.end.unwrap()
//     } else {
//         size - 1
//     };

//     if last > size - 1 {
//         return Err(eyre!(
//             "range exceeds maximum register entry which is {}",
//             size - 1
//         ));
//     }

//     // As entries_vec[] is in reverse order we adjust the start and end and count backwards
//     println!("entries {first} to {last}:");
//     for index in first..=last {
//         let xor_name = xorname_from_entry(&entries_vec[index]);
//         if include_files {
//             println!("entry {index} - fetching metadata at {xor_name:64x}");
//             match DirectoryTree::directory_tree_download(client, xor_name).await {
//                 Ok(metadata) => {
//                     let _ = do_print_files(&metadata, &files_args);
//                 }
//                 Err(e) => {
//                     println!("Failed to get website metadata from network");
//                     return Err(eyre!(e));
//                 }
//             };
//         } else {
//             println!("{xor_name:64x}");
//         }
//     }

//     Ok(())
// }

fn do_print_files(metadata: &DirectoryTree, files_args: &FilesArgs) -> Result<()> {
    let metadata_stats = if files_args.print_metadata_summary
        || files_args.print_counts
        || files_args.print_total_bytes
    {
        metadata_stats(metadata)?
    } else {
        (0 as usize, 0 as u64)
    };

    if files_args.print_metadata_summary {
        let _ = do_print_metadata_summary(metadata, metadata_stats);
    } else {
        if files_args.print_counts {
            let _ = do_print_counts(metadata, metadata_stats.0);
        }

        if files_args.print_total_bytes {
            let _ = do_print_total_bytes(metadata_stats.1);
        }
    }

    if files_args.print_paths || files_args.print_all_details {
        for (path_string, path_map) in metadata.path_map.paths_to_files_map.iter() {
            for (file_name, xor_name, modified, size, json_metadata) in path_map.iter() {
                if files_args.print_all_details {
                    let date_time = DateTime::<Utc>::from(*modified);
                    let modified_str = date_time.format("%Y-%m-%d %H:%M:%S").to_string();
                    println!(
                        "{xor_name:64x} {modified_str} \"{path_string}{file_name}\" {size} bytes and JSON: \"{json_metadata}\"",
                    );
                } else {
                    println!("{xor_name:64x} \"{path_string}{file_name}\"");
                }
            }
        }
    }

    Ok(())
}

fn metadata_stats(metadata: &DirectoryTree) -> Result<(usize, u64)> {
    let mut files_count: usize = 0;
    let mut total_bytes: u64 = 0;

    for (_, path_map) in metadata.path_map.paths_to_files_map.iter() {
        files_count = files_count + path_map.len();

        for file_metadata in path_map {
            total_bytes = total_bytes + file_metadata.3
        }
    }

    Ok((files_count, total_bytes))
}

fn do_print_metadata_summary(metadata: &DirectoryTree, metadata_stats: (usize, u64)) -> Result<()> {
    println!("published  : {}", metadata.date_published);
    let _ = do_print_counts(metadata, metadata_stats.0);
    let _ = do_print_total_bytes(metadata_stats.1);
    Ok(())
}

fn do_print_counts(metadata: &DirectoryTree, count_files: usize) -> Result<()> {
    println!(
        "directories: {}",
        metadata.path_map.paths_to_files_map.len()
    );
    println!("files      : {count_files}");
    Ok(())
}

fn do_print_total_bytes(total_bytes: u64) -> Result<()> {
    println!("total bytes: {total_bytes}");
    Ok(())
}

/// Implement 'inspect-files' subcommand
///
/// Accepts a metadata address
///
/// TODO extend treatment to handle register with branches etc (post stabilisation of the Autonomi API)
pub async fn handle_inspect_files(
    peers: NetworkPeers,
    metadata_address: XorName,
    files_args: FilesArgs,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    println!("fetching metadata at {metadata_address:64x}");
    match DirectoryTree::directory_tree_download(&client, metadata_address).await {
        Ok(metadata) => {
            let _ = do_print_files(&metadata, &files_args);
        }
        Err(e) => {
            println!("Failed to get website metadata from network");
            return Err(eyre!(e).into());
        }
    };
    Ok(())
}
