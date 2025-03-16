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

pub mod convert;
pub mod file;
pub mod graph_entry;
pub mod web;

use blsttc::SecretKey;

use color_eyre::{eyre::eyre, Result};

/// Get the maini secret key. This is currently derived from a key set in the environment
/// TODO ?provide a primary method with environment setting as a backup
pub fn get_app_secret_key() -> Result<SecretKey> {
    match crate::autonomi::access::keys::get_vault_secret_key() {
        Ok(secret_key) => Ok(secret_key),
        Err(e) => {
            let message = format!("No secret key is available - {e}");
            println!("DEBUG {message}");
            return Err(eyre!(message));
        }
    }
}

// Make a vector of node Entry with vector[0] being the first node in the history.
// We take the first 'root' node and the first child of the root, the first child
// of that child and so on.
// So if there were multiple children (i.e. conflicting versions) only one is included
// pub fn node_entries_as_vec(register: &Register) -> Vec<Entry> {
//     let merkle_reg = register.inner_merkle_reg();
//     let content = merkle_reg.read();
//     let mut entries_vec: Vec<Entry> = Vec::new();
//     let mut node = content.nodes().nth(0);
//     while node.is_some() {
//         let node_ref = node.unwrap();
//         entries_vec.push(node_ref.value.clone());
//         node = if let Some(first_child_hash) = node_ref.children.clone().into_iter().nth(0) {
//             merkle_reg.node(first_child_hash)
//         } else {
//             None
//         };
//     }
//     entries_vec.reverse();
//     entries_vec
// }

// TODO replace with Transactions based history
// Interim using defanged Register until implemented for Transactions
// Always returns the most recent entry at Vec[1] and a dummy entry, the type at Vec[0]
// pub fn node_entries_as_vec(register: &Register) -> Vec<Entry> {
//     const AWV_REG_TYPE_PUBLIC: &str =
//         "5ebbbc4f061702c875b6cacb76e537eb482713c458b9d83c2f1e86ea9e0d0d0f";

//     let mut entries_vec: Vec<Entry> = Vec::new();

//     let type_entry = match convert::str_to_xor_name(AWV_REG_TYPE_PUBLIC) {
//         Ok(entry) => entry,
//         Err(e) => panic!("Failed to decode AWV_REG_TYPE_PUBLIC"),
//     };
//     entries_vec.push(type_entry.to_vec());
//     if register.values().len() > 0 {
//         entries_vec.push(register.values()[0].to_vec());
//     }
//     entries_vec
// }
