pub mod convert;

use ant_registers::Entry;
use autonomi::client::registers::Register;
use xor_name::{xor_name, XorName};

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
pub fn node_entries_as_vec(register: &Register) -> Vec<Entry> {
    const AWV_REG_TYPE_PUBLIC: &str =
        "5ebbbc4f061702c875b6cacb76e537eb482713c458b9d83c2f1e86ea9e0d0d0f";

    let mut entries_vec: Vec<Entry> = Vec::new();

    let type_entry = match convert::str_to_xor_name(AWV_REG_TYPE_PUBLIC) {
        Ok(entry) => entry,
        Err(e) => panic!("Failed to decode AWV_REG_TYPE_PUBLIC"),
    };
    entries_vec.push(type_entry.to_vec());
    if register.values().len() > 0 {
        entries_vec.push(register.values()[0].to_vec());
    }
    entries_vec
}
