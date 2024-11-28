pub mod convert;

use autonomi::client::registers::Register;
use sn_registers::Entry;
use xor_name::XorName;

// Make a vector of node Entry with vector[0] being the first node in the history.
// We take the first 'root' node and the first child of the root, the first child
// of that child and so on.
// So if there were multiple children (i.e. conflicting versions) only one is included
pub fn node_entries_as_vec(register: &Register) -> Vec<Entry> {
    let merkle_reg = register.inner_merkle_reg();
    let content = merkle_reg.read();
    let mut entries_vec: Vec<Entry> = Vec::new();
    let mut node = content.nodes().nth(0);
    while node.is_some() {
        let node_ref = node.unwrap();
        entries_vec.push(node_ref.value.clone());
        node = if let Some(first_child_hash) = node_ref.children.clone().into_iter().nth(0) {
            merkle_reg.node(first_child_hash)
        } else {
            None
        };
    }
    entries_vec.reverse();
    entries_vec
}
