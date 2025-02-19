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

pub mod directory_tree;

use std::marker::PhantomData;

use blsttc::PublicKey;
use color_eyre::eyre::{eyre, Result};
// use serde::{de::DeserializeOwned, Deserialize, Serialize};
use xor_name::XorName;

use ant_protocol::storage::{GraphEntry, Pointer, PointerAddress, PointerTarget};
use autonomi::client::data_types::graph::{GraphContent, GraphError};
use autonomi::client::key_derivation::{DerivationIndex, MainPubkey, MainSecretKey};
use autonomi::client::vault::VaultSecretKey as SecretKey;
use autonomi::AttoTokens;
use autonomi::Bytes;
use autonomi::GraphEntryAddress;

use crate::client::AutonomiClient;
use crate::data::autonomi_get_file_public;
use crate::helpers::convert::str_to_xor_name;
use crate::helpers::graph_entry::{
    self, create_graph_entry, get_derivation_from_graph_entry, graph_entry_get,
};

const LARGEST_VERSION: u32 = u32::MAX;

/// Derivation index to avoid address clashes between types with the same owner
/// Note: the string must be exactly 32 bytes long
const POINTER_DERIVATION_INDEX: &str = "dweb Pointer derivatation index ";

/// A History is addressed at a [`HistoryAddress`] which is derived from the owner's
/// [`PublicKey`] and a name. This means a single owner key can manage multiple
/// histories.
///
/// Any data stored in the register is stored as is, without encryption or modifications.
/// Since the data is publicly accessible by anyone knowing the [`HistoryAddress`],
/// it is up to the owner to encrypt the data uploaded to the register, if wanted.
/// Only the owner can update the register with its [`SecretKey`].
/// The [`SecretKey`] is the only piece of information an owner should keep to access to the register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HistoryAddress {
    pub owner: PublicKey,
}

impl HistoryAddress {
    /// Create a new register address
    pub fn new(owner: PublicKey) -> Self {
        Self { owner }
    }

    /// Get the owner of the register
    pub fn owner(&self) -> PublicKey {
        self.owner
    }

    /// To underlying graph representation
    pub fn to_underlying_graph_root(&self) -> GraphEntryAddress {
        GraphEntryAddress::from_owner(self.owner)
    }

    /// Convert a register address to a hex string
    pub fn to_hex(&self) -> String {
        self.owner.to_hex()
    }

    /// Convert a hex string to a register address
    pub fn from_hex(hex: &str) -> Result<Self, blsttc::Error> {
        let owner = PublicKey::from_hex(hex)?;
        Ok(Self { owner })
    }
}

impl std::fmt::Display for HistoryAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// The value of a register: a 32 bytes array (same as [`GraphContent`])
pub type HistoryValue = GraphContent;

/// The size of a register value: 32 bytes
pub const REGISTER_VALUE_SIZE: usize = size_of::<HistoryValue>();

/// gives access to every version of the struct that has ever been stored
/// on Autonomi.
///
/// Each History<T> is created using an owner secret key and a name (String). Different T
/// can have overlapping names without problems, because the Trove::trove_type() is also used
/// when a new history is created.
///
/// For read-only access, only the address of the history is needed, which gives access to both
/// the first entry in the history and the most recent.
///
/// For write access, both the owning secret key and the name must be provided.
///
/// Example, using the built-in dweb::trove::DirectoryTree struct you can store and access
/// every published version of a tree of files, which might represent a website.
///
/// Notes:
/// - the dweb-cli supports viewing of versioned websites and directories using
/// a standard web browser, including viewing every version published on Autonomi (similar
/// to the Internet Archive).
/// -  History manages a sequence of versions of a struct implementing Trove,
/// amounting to a versioned history for any struct impl Trove.
#[allow(async_fn_in_trait)]
pub trait Trove<T> {
    fn trove_type() -> XorName;
    fn to_bytes(trove: &T) -> Result<Bytes>;
    async fn from_bytes(client: &AutonomiClient, bytes: Bytes) -> Result<T>;
}

/// A history of versions of a type implementing the Trove trait. This
/// can be used to create and access versions of a file, a collection of
/// files such as a directory, or all the files and settings that make up a website,
/// and so on.
/// TODO provide a way to initialise a history from an Autonomi Register pointer and graph
//  TODO this will require storing the POINTER_DERIVATION_INDEX in the History, and
//  TODO initialising it according to whether or not it is initialised from a Register .
//  TODO Also changes to History::history_main_secret_key() for compatibility with Registers
pub struct History<T: Trove<T> + Clone> {
    client: AutonomiClient,

    history_address: HistoryAddress,
    name: String,

    // We can't trust a pointer from the network to be up-to-date, so these are updated from the graph
    // Once set, head_graphentry will always be the real head and num_entries always correct
    num_entries: u32,
    head_graphentry: Option<GraphEntry>,

    // Track the pointer version for comparisson (e.g. using 'inspect-history')
    pointer_counter: u32,
    pointer_target: Option<GraphEntryAddress>,

    // For operations when no version is specified. Typically, None implies most recent
    default_version: Option<u32>,
    // Cached data for the selected version
    cached_version: Option<TroveVersion<T>>,

    // Pretend we hold a Trove so we can restrict some values to type T in the implementation
    phantom: std::marker::PhantomData<T>,
}

impl<T: Trove<T> + Clone> History<T> {
    /// Create a new History for read-write access and store it on the network
    /// To update the history use the same owner_secret_key
    /// name cannot be an empty string
    pub async fn create_online(
        client: AutonomiClient,
        name: String,
        owner_secret_key: SecretKey,
    ) -> Result<(AttoTokens, Self)> {
        println!("DEBUG History::create_online({name})");
        if name.is_empty() {
            return Err(eyre!(
                "History::create_online() failed - cannot use an empty name"
            ));
        }

        let history_secret_key =
            Self::history_main_secret_key(owner_secret_key).derive_child(name.as_bytes());
        let history_address = HistoryAddress::new(history_secret_key.public_key());

        // Put the first entry in the graph
        let root_entry = create_graph_entry(
            &history_secret_key,
            None,
            &DerivationIndex::random(&mut rand::thread_rng()),
            Self::trove_type(),
        )
        .await?;
        println!(
            "DEBUG graph_entry_put() at {}",
            root_entry.address().to_hex()
        );
        let (graph_cost, root_entry_address) = match client
            .client
            .graph_entry_put(root_entry.clone(), client.payment_option())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                let msg = format!("failed to put graph entry - {e}");
                println!("DEBUG graph_entry_put() {msg}");
                return Err(eyre!(msg));
            }
        };

        let pointer_secret_key = Self::history_pointer_secret_key(history_secret_key);
        let pointer = Self::create_pointer_for_update(0, &root_entry_address, &pointer_secret_key);
        println!("DEBUG created pointer at {}", pointer.address().to_hex());
        match client
            .client
            .pointer_put(pointer, client.wallet.clone().into())
            .await
        {
            Ok((pointer_cost, pointer_address)) => {
                println!(
                    "DEBUG History::new() created new pointer at {:x}",
                    pointer_address.xorname()
                );
                let history = History {
                    client: client.clone(),
                    name,
                    history_address,
                    num_entries: 1,
                    head_graphentry: Some(root_entry), // The first and only entry so far
                    pointer_counter: 0,
                    pointer_target: None,
                    default_version: None,
                    cached_version: None,
                    phantom: PhantomData,
                };

                let total_cost = if let Some(total_cost) = pointer_cost.checked_add(graph_cost) {
                    total_cost
                } else {
                    return Err(eyre!("Invalid cost"));
                };
                Ok((total_cost, history))
            }
            Err(e) => {
                let message = format!("History::new() failed to create pointer: {e}");
                println!("DEBUG {message}");
                return Err(eyre!(message));
            }
        }
    }

    /// Load a History from the network that can be used for read or write access
    /// To update the History use the same owner_secret_key
    ///
    /// Note the following behaviour which allows you to control whether to trust the
    /// History pointer is up-to-date and pointing at the head of the graph. You can
    /// choose to scan the graph to ensure the History is up-to-date even if the pointer
    /// is not, but when scanning the graph this function will take several seconds
    /// in order to detect the end of the graph.
    ///
    ///   If  ignore_pointer, updates the head from the graph (which will take several seconds)
    ///
    ///   if !ignore_pointer, and minimum_entry_index is non-zero will ignore the pointer only if
    ///   its counter is less than minimum_entry_index.
    ///
    ///   if !ignore_pointer, and minimum_entry_index is 0, uses the pointer (even though it may be
    ///   out of date). This should be fast.
    pub async fn from_name(
        client: AutonomiClient,
        owner_secret_key: SecretKey,
        name: String,
        ignore_pointer: bool,
        minimum_entry_index: u32,
    ) -> Result<(AttoTokens, Self)> {
        println!("DEBUG History::from_name({name})");
        if name.is_empty() {
            return Err(eyre!(
                "History::from_name() failed - cannot use an empty name"
            ));
        }

        let history_secret_key =
            Self::history_main_secret_key(owner_secret_key).derive_child(name.as_bytes());
        let history_address = HistoryAddress::new(history_secret_key.public_key());

        // Check it exists to avoid accidental creation (and payment)
        let pointer_address = Self::pointer_address_from_history_address(history_address.clone())?;
        let pointer = match Self::get_and_verify_pointer(&client, &pointer_address).await {
            Ok(pointer) => pointer,
            Err(e) => {
                let msg = format!(
                    "failed to get pointer network address {} - {e}",
                    pointer_address.to_hex()
                );
                println!("DEBUG History::from_name() {msg}");
                return Err(e.into());
            }
        };

        println!(
            "DEBUG History::from_name() obtained pointer from {:x}",
            pointer.address().xorname()
        );

        let mut history = History {
            client: client.clone(),
            name,
            history_address,
            num_entries: 0,
            head_graphentry: None,
            pointer_counter: pointer.counter(),
            pointer_target: Some(GraphEntryAddress(pointer.target().xorname())),
            default_version: None,
            cached_version: None,
            phantom: PhantomData,
        };
        // Necessary because the pointer may not be up-to-data
        if ignore_pointer || (!ignore_pointer && minimum_entry_index > pointer.counter()) {
            // Ignore the pointer because that was specified,
            // or the pointer counter() is behind minimum_entry_index
            history
                .update_from_graph(
                    &GraphEntryAddress(pointer.target().xorname()),
                    pointer.counter(),
                )
                .await?;
        } else {
            // Use the pointer even though it may not be up-to-date
            match history
                .get_graph_entry_from_network(&GraphEntryAddress(pointer.target().xorname()))
                .await
            {
                Ok(pointer_head) => {
                    history.num_entries = pointer.counter() + 1;
                    history.head_graphentry = Some(pointer_head);
                    history.pointer_counter = pointer.counter() + 1;
                    history.pointer_target = Some(GraphEntryAddress(pointer.target().xorname()));
                }
                Err(e) => return Err(eyre!("Failed to get pointer target entry - {e}")),
            };
        }

        Ok((Into::into(0), history))
    }

    /// Load a read-only History from the network
    ///
    /// Note the following behaviour which allows you to control whether to trust the
    /// History pointer is up-to-date and pointing at the head of the graph. You can
    /// choose to scan the graph to ensure the History is up-to-date even if the pointer
    /// is not, but when scanning the graph this function will take several seconds
    /// in order to detect the end of the graph.
    ///
    ///   If  ignore_pointer, updates the head from the graph (which will take several seconds)
    ///
    ///   if !ignore_pointer, and minimum_entry_index is non-zero will ignore the pointer only if
    ///   its counter is less than minimum_entry_index.
    ///
    ///   if !ignore_pointer, and minimum_entry_index is 0, uses the pointer (even though it may be
    ///   out of date). This should be fast.
    pub async fn from_history_address(
        client: AutonomiClient,
        history_address: HistoryAddress,
        ignore_pointer: bool,
        minimum_entry_index: u32,
    ) -> Result<History<T>> {
        // println!(
        //     "DEBUG History::from_history_address({})",
        //     history_address.to_hex()
        // );

        // Check it exists to avoid accidental creation (and payment)
        let pointer_address = Self::pointer_address_from_history_address(history_address.clone())?;
        let pointer = match Self::get_and_verify_pointer(&client, &pointer_address).await {
            Ok(pointer) => pointer,
            Err(e) => {
                let msg = format!(
                    "failed to get pointer network address {} - {e}",
                    pointer_address.to_hex()
                );
                println!("DEBUG History::from_history_address() {msg}");
                return Err(e.into());
            }
        };

        let mut history = History::<T> {
            client,
            name: String::from(""),
            history_address,
            num_entries: 0,
            head_graphentry: None,
            pointer_counter: pointer.counter(),
            pointer_target: Some(GraphEntryAddress(pointer.target().xorname())),
            default_version: None,
            cached_version: None,
            phantom: PhantomData,
        };
        // Necessary because the pointer may not be up-to-data
        if ignore_pointer || (!ignore_pointer && minimum_entry_index > pointer.counter()) {
            // Ignore the pointer because that was specified,
            // or the pointer counter() is behind minimum_entry_index
            history
                .update_from_graph(
                    &GraphEntryAddress(pointer.target().xorname()),
                    pointer.counter(),
                )
                .await?;
        } else {
            // Use the pointer even though it may not be up-to-date
            match history
                .get_graph_entry_from_network(&GraphEntryAddress(pointer.target().xorname()))
                .await
            {
                Ok(pointer_head) => {
                    history.num_entries = pointer.counter() + 1;
                    history.head_graphentry = Some(pointer_head);
                    history.pointer_counter = pointer.counter() + 1;
                    history.pointer_target = Some(GraphEntryAddress(pointer.target().xorname()));
                }
                Err(e) => return Err(eyre!("Failed to get pointer target entry - {e}")),
            };
        }

        history.update_default_version();
        Ok(history)
    }

    /// Safely get the actual head even if the pointer_target is not the heaad.
    ///
    /// If the pointer_target is out of date this function scans the graph starting at pointer_target
    /// until it reaches the end and can correctly set the head GraphEntry and num_entries.
    ///
    /// This will take few seconds because it has to wait for the request for a graph entry to
    /// not be found on the network.
    ///
    /// Returns the head GraphEntry
    pub async fn update_from_graph(
        &mut self,
        pointer_target: &GraphEntryAddress,
        pointer_counter: u32,
    ) -> Result<GraphEntry> {
        println!("DEBUG History::update_from_graph()");

        if self.head_graphentry.is_some() {
            return Ok(self.head_graphentry.clone().unwrap());
        }

        // Get the Pointer target entry and move forwards - because the pointer may not be up to date
        let pointer_target_entry = match self.get_graph_entry_from_network(pointer_target).await {
            Ok(head) => head,
            Err(e) => return Err(eyre!("Failed to get pointer target entry - {e}")),
        };

        let mut iter_entry = pointer_target_entry;
        let mut iter_index = pointer_counter;

        let mut final_index = iter_index;
        let mut final_entry;
        loop {
            println!("DEBUG stepping forwards: iter_index {iter_index}");
            final_entry = iter_entry.clone();
            iter_entry = if let Some(entry) = self.get_child_entry_of(&iter_entry).await {
                iter_index = iter_index + 1;
                final_index = final_index + 1;
                entry
            } else {
                break;
            }
        }

        self.head_graphentry = Some(final_entry.clone());
        self.num_entries = final_index + 1;

        Ok(final_entry)
    }

    async fn get_and_verify_pointer(
        client: &AutonomiClient,
        pointer_address: &PointerAddress,
    ) -> Result<Pointer> {
        match client.client.pointer_get(pointer_address).await {
            Ok(pointer) => {
                if !pointer.verify_signature() {
                    let message =
                        format!("Error - pointer retrieved from network has INVALID SIGNATURE");
                    println!("{message}");
                    return Err(eyre!(message));
                }

                let head_address = match pointer.target() {
                    PointerTarget::GraphEntryAddress(address) => address,
                    other => return Err(eyre!("Invalid head address {:?}", other.clone())),
                };
                println!(
                    "DEBUG pointer counter: {}, head address: {}",
                    pointer.counter(),
                    head_address.to_hex()
                );
                Ok(pointer)
            }
            Err(e) => {
                let message = format!("failed to get pointer from the network - {e}");
                println!("{message}");
                return Err(eyre!(message));
            }
        }
    }

    fn create_pointer_for_update(
        counter: u32,
        graph_entry_address: &GraphEntryAddress,
        history_secret_key: &SecretKey,
    ) -> Pointer {
        println!("DEBUG create_pointer_for_update()");
        let pointer_target = PointerTarget::GraphEntryAddress(*graph_entry_address);
        Pointer::new(history_secret_key, counter, pointer_target)
    }

    /// Get the main secret key for all histories belonging to an owner
    fn history_main_secret_key(owner_secret_key: SecretKey) -> SecretKey {
        // For release use the trove type:
        let derivation_index: [u8; 32] = Self::trove_type().to_vec().try_into().unwrap();
        // TODO DEBUG For testing until the scripts are uploading to the Arbitrum One network reliably
        // TODO use this, and change it to wipe the slate clean
        let derivation_index: [u8; 32] = [0; 32]; // Modify each time I need to start afresh
        MainSecretKey::new(owner_secret_key.clone())
            .derive_key(&DerivationIndex::from_bytes(derivation_index))
            .into()
    }

    /// Get the main secret key for the pointer belonging to a history
    fn history_pointer_secret_key(history_secret_key: SecretKey) -> SecretKey {
        let derivation_index: [u8; 32] = POINTER_DERIVATION_INDEX.as_bytes().try_into().unwrap();
        MainSecretKey::new(history_secret_key.clone())
            .derive_key(&DerivationIndex::from_bytes(derivation_index))
            .into()
    }

    /// The root graph entry of the History (not the entry for the first value).
    /// This is not the entry for the first value, because the root graph entry is used to store the Trove::trove_type()
    /// To get the graph entry for the first value in the history, get the root entry and then get its child.
    /// This function is provided for clarity in documentation.
    pub fn root_graph_entry_address(history_address: GraphEntryAddress) -> GraphEntryAddress {
        history_address
    }

    pub fn pointer_address_from_history_address(
        history_address: HistoryAddress,
    ) -> Result<PointerAddress> {
        let history_main_public_key: MainPubkey = MainPubkey::new(history_address.owner());
        let derivation_index: [u8; 32] = POINTER_DERIVATION_INDEX.as_bytes().try_into().unwrap();
        let pointer_pk =
            history_main_public_key.derive_key(&DerivationIndex::from_bytes(derivation_index));
        Ok(PointerAddress::from_owner(pointer_pk.into()))
    }

    /// The address of the head in the current pointer
    /// Does not update pointer from network
    pub fn head_entry_address(&self) -> Result<GraphEntryAddress> {
        match self.head_graphentry.clone() {
            Some(head_entry) => Ok(head_entry.address()),
            None => Err(eyre!("History has uninitialised head_graphentry_entry")),
        }
    }

    pub fn pointer_counter(&self) -> u32 {
        self.pointer_counter
    }

    fn update_default_version(&mut self) -> Option<u32> {
        self.default_version = match self.num_versions() {
            Ok(version) => Some(version),
            Err(_) => None,
        };
        // println!(
        //     "DEBUG update_default_version() set to {}",
        //     self.default_version.unwrap()
        // );
        self.default_version
    }

    pub fn trove_type() -> XorName {
        T::trove_type()
    }

    pub fn history_address(&self) -> HistoryAddress {
        self.history_address.clone()
    }

    /// Return the number of entries in the history
    /// This is one more than the number of versions
    /// because the first entry is reserved for use
    /// as a type (which may point to metadata about
    /// the Trove type). Example types include file
    /// system and website.
    pub fn num_entries(&self) -> u32 {
        self.num_entries
    }

    /// Return the number of available versions
    /// or an error if no versions are available.
    /// The first version is 1 last version is num_versions()
    pub fn num_versions(&self) -> Result<u32> {
        let num_entries = self.num_entries;

        if num_entries == 0 {
            let message = "pointer is empty (0 entries)";
            Err(eyre!(message))
        } else {
            Ok(num_entries - 1)
        }
    }

    /// Download a `DirectoryTree` from the network
    async fn trove_download(&self, data_address: XorName) -> Result<T> {
        return History::<T>::raw_trove_download(&self.client, data_address).await;
    }

    /// Type-safe download directly from the network.
    /// Useful if you already have the address and don't want to initialise a History
    pub async fn raw_trove_download(client: &AutonomiClient, data_address: XorName) -> Result<T> {
        println!("DEBUG directory_tree_download() at {data_address:x}");
        match autonomi_get_file_public(client, &data_address).await {
            Ok(content) => {
                println!("Retrieved {} bytes", content.len());
                let trove: T = match T::from_bytes(client, content).await {
                    Ok(trove) => trove,
                    Err(e) => {
                        println!("FAILED: {e}");
                        return Err(eyre!(e));
                    }
                };
                Ok(trove)
            }

            Err(e) => {
                println!("FAILED: {e}");
                Err(eyre!(e))
            }
        }
    }

    /// Get the entry value for the given version.
    /// The first entry in the history is version 0, but that is reserved so the
    /// first version in a history is 1 and the last is the number of entries - 1
    pub async fn get_version_entry_value(&mut self, version: u32) -> Result<XorName> {
        println!("DEBUG History::get_version_entry_value(version: {version})");
        // self.update_pointer().await?;
        let num_entries = self.num_entries();

        // The first entry is the Trove<T>::trove_type(), and not used so max version is num_entries - 1
        let max_version = if num_entries > 0 { num_entries - 1 } else { 0 };

        if version > max_version {
            let message = format!(
                "History::get_version_entry_value({version}) out of range for max_version: {max_version}"
            );
            println!("{message}");
            return Err(eyre!(message));
        }

        self.get_entry_value(version).await
    }

    /// Get the value by absolute entry index.
    /// Note that the root entry (index 0) is not a valid version. Version 1 is at index 1.
    pub async fn get_entry_value(&mut self, index: u32) -> Result<XorName> {
        println!("DEBUG History::get_entry_value(index: {index})");
        match self.get_graph_entry(index).await {
            Ok(entry) => str_to_xor_name(&hex::encode(entry.content)),
            Err(e) => return Err(e),
        }
    }

    /// Get the graph entry by absolute entry index.
    /// Note that the root entry (index 0) is not a valid version. Version 1 is at index 1.
    pub async fn get_graph_entry(&mut self, index: u32) -> Result<GraphEntry> {
        // println!("DEBUG History::get_graph_entry(index: {index})");
        // self.update_pointer().await?;
        let num_entries = self.num_entries();

        if index >= num_entries {
            return Err(eyre!(
                "Index out of range, index: {index}, number of entries {num_entries}"
            ));
        };

        Ok(if index > num_entries / 2 {
            // Start at the head and move backwards
            let mut iter_entry = match self.get_head_entry().await {
                Ok(head) => {
                    if head.is_some() {
                        head.unwrap()
                    } else {
                        return Err(eyre!("Empty history - no head entry"));
                    }
                }
                Err(e) => return Err(e),
            };

            let mut iter_index = num_entries - 1;
            while index < iter_index {
                println!("DEBUG stepping backwards: index {index} < {iter_index} iter_index");
                iter_index = iter_index - 1;
                iter_entry = if let Some(entry) = self.get_parent_entry_of(&iter_entry).await? {
                    entry
                } else {
                    return Err(eyre!(
                        "Ran out of entries - probably a bug in History::get_entry_value()"
                    ));
                }
            }
            iter_entry
        } else {
            // Start at the root and count forwards
            let mut iter_entry = match self.get_root_entry().await {
                Ok(root) => {
                    if root.is_some() {
                        root.unwrap()
                    } else {
                        return Err(eyre!(
                            "Failed to get root entry in History::get_entry_value()"
                        ));
                    }
                }
                Err(e) => return Err(e),
            };

            let mut iter_index = 0;
            while index > iter_index {
                println!("DEBUG stepping forwards: index {index} > {iter_index} iter_index");
                iter_index = iter_index + 1;
                iter_entry = if let Some(entry) = self.get_child_entry_of(&iter_entry).await {
                    entry
                } else {
                    return Err(eyre!(
                        "Ran out of entries - may be a bug in History::get_entry_value()"
                    ));
                }
            }
            iter_entry
        })
    }

    // Get a GraphEntry from the network
    async fn get_graph_entry_from_network(
        &self,
        graph_entry_address: &GraphEntryAddress,
    ) -> Result<GraphEntry> {
        // println!(
        //     "DEBUG get_graph_entry_from_network() at {}",
        //     graph_entry_address.to_hex()
        // );
        Ok(graph_entry_get(&self.client.client, graph_entry_address).await?)
    }

    // Does not need to update pointer
    pub async fn get_root_entry(&self) -> Result<Option<GraphEntry>> {
        Ok(Some(
            self.get_graph_entry_from_network(&Self::root_graph_entry_address(
                GraphEntryAddress::from_owner(self.history_address.owner()),
            ))
            .await?,
        ))
    }

    /// Get the most recent GraphEntry
    pub async fn get_head_entry(&self) -> Result<Option<GraphEntry>> {
        Ok(Some(
            self.get_graph_entry_from_network(&self.head_entry_address()?)
                .await?,
        ))
    }

    /// Get the parent of a GraphEntry
    pub async fn get_parent_entry_of(
        &self,
        graph_entry: &GraphEntry,
    ) -> Result<Option<GraphEntry>> {
        let parent = GraphEntryAddress::from_owner(graph_entry.parents[0]);
        Ok(Some(self.get_graph_entry_from_network(&parent).await?))
    }

    /// Get the child of a GraphEntry
    /// Assumes each entry has only one descendent
    pub async fn get_child_entry_of(&self, graph_entry: &GraphEntry) -> Option<GraphEntry> {
        // // TODO I don't understand why this isn't sufficient:
        // let child = GraphEntryAddress::from_owner(graph_entry.descendants[0].0);

        // TODO this is how Autonomi History does it:
        let next_derivation = DerivationIndex::from_bytes(graph_entry.descendants[0].1);
        let next_entry_pk: PublicKey = MainPubkey::from(self.history_address().owner)
            .derive_key(&next_derivation)
            .into();
        let child = GraphEntryAddress::from_owner(next_entry_pk);

        match self.get_graph_entry_from_network(&child).await {
            Ok(graph_entry) => Some(graph_entry),
            Err(_) => None,
        }
    }

    // Returns the version of the cached entry if present
    pub fn get_cached_version_number(&self) -> Option<u32> {
        if let Some(trove_version) = &self.cached_version {
            if trove_version.trove.is_some() {
                return Some(trove_version.version);
            }
        }
        None
    }

    /// Get a graph entry and the next derivation index
    ///
    /// A history entry should only have one descendent. If this is not the case we use the first descendent.
    /// Dealing with the errors instead of failing allows users to solve forks and corruption by updating the history.
    ///
    /// TODO maybe this should call dweb::helpers::graph_entry::get_graph_entry_and_next_derivation_index()
    async fn history_get_graph_entry_and_next_derivation_index(
        &self,
        graph_entry_address: &GraphEntryAddress,
    ) -> Result<(GraphEntry, DerivationIndex)> {
        println!("DEBUG history_get_graph_entry_and_next_derivation_index()");
        let entry = match self.get_graph_entry_from_network(graph_entry_address).await {
            Ok(entry) => entry,
            Err(e) => {
                let msg = format!("Failed to get graph entry from network - {e}");
                println!("DEBUG get_graph_entry_from_network() {msg}");
                return Err(eyre!("msg"));
            }
        };
        let new_derivation = get_derivation_from_graph_entry(&entry)?;
        println!(
            "DEBUG returning ({}, {})",
            entry.address().to_hex(),
            hex::encode(new_derivation.as_bytes())
        );
        Ok((entry, new_derivation))
    }

    /// Add a trove to the History and return the index of the most recent entry (1 = first trove entry, 0 = root entry)
    async fn update_online(
        &mut self,
        owner_secret_key: SecretKey,
        trove_address: XorName,
    ) -> Result<(AttoTokens, u32)> {
        println!("DEBUG History::update_online()");
        let history_secret_key =
            Self::history_main_secret_key(owner_secret_key).derive_child(self.name.as_bytes());

        let history_address = HistoryAddress::new(history_secret_key.public_key());
        println!("Updating History at {}", history_address.to_hex());

        let pointer_address = Self::pointer_address_from_history_address(history_address.clone())?;
        match Self::get_and_verify_pointer(&self.client, &pointer_address).await {
            Ok(old_pointer) => {
                self.pointer_counter = old_pointer.counter();
                let head_address = self.head_graphentry.clone().unwrap().address();

                // Note: if head_address isn't the head, create_next_graph_entry_online() will retry until it reaches it
                let (graph_cost, next_address) = match self
                    .create_next_graph_entry_online(
                        history_secret_key.clone(),
                        head_address,
                        &trove_address,
                    )
                    .await
                {
                    Ok(result) => result,
                    Err(e) => return Err(eyre!("failed to create next GraphEnry: {e}")),
                };

                println!("Pointer retrieved with counter {}", old_pointer.counter());
                let pointer_secret_key = Self::history_pointer_secret_key(history_secret_key);
                let new_pointer = Self::create_pointer_for_update(
                    self.num_entries(),
                    &next_address,
                    &pointer_secret_key,
                );
                println!(
                    "DEBUG created pointer at {}",
                    new_pointer.address().to_hex()
                );

                println!(
                    "Calling pointer_put() with new GraphEntry at: {}",
                    next_address.to_hex()
                );
                match self
                    .client
                    .client
                    .pointer_put(new_pointer.clone(), self.client.wallet.clone().into())
                    .await
                {
                    Ok((pointer_cost, _pointer_address)) => {
                        self.pointer_counter = new_pointer.counter();
                        self.pointer_target =
                            Some(GraphEntryAddress(new_pointer.target().xorname()));

                        let total_cost = pointer_cost.checked_add(graph_cost);
                        if total_cost.is_none() {
                            return Err(eyre!("Invalid cost"));
                        }
                        return Ok((total_cost.unwrap(), new_pointer.counter()));
                    }
                    Err(e) => {
                        return Err(eyre!("Failed to add a trove to history: {e:?}"));
                    }
                }
            }
            Err(e) => return Err(eyre!("DEBUG failed to get history prior to update!\n{e}")),
        };
    }

    /// Create the next graph entry.
    /// Begins at the provided head_address but handles the case where this is
    /// not the head by moving along the graph until it finds the real head.
    async fn create_next_graph_entry_online(
        &self,
        history_secret_key: SecretKey,
        head_address: GraphEntryAddress,
        content: &XorName,
    ) -> Result<(AttoTokens, GraphEntryAddress)> {
        println!(
            "DEBUG create_next_graph_entry_online() with content {:x}",
            content
        );

        println!("DEBUG head_address: {}", head_address.to_hex());
        let mut head_address = head_address;
        loop {
            // Get the next derivation index from the current most recent entry
            let (parent_entry, new_derivation) = self
                .history_get_graph_entry_and_next_derivation_index(&head_address)
                .await?;

            let new_entry = create_graph_entry(
                &history_secret_key,
                Some(&parent_entry),
                &new_derivation,
                *content,
            )
            .await?;

            println!("DEBUG new_entry: {new_entry:?}");
            println!("DEBUG new_entry address: {}", new_entry.address().to_hex());
            match self
                .client
                .client
                .graph_entry_put(new_entry, self.client.payment_option())
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) => match e {
                    GraphError::AlreadyExists(existing_address) => {
                        println!(
                            "DEBUG new_entry already exists, trying again with that as 'head'"
                        );
                        head_address = existing_address
                    }
                    _ => {
                        let msg = format!("Failed graph_entry_put() - {e}");
                        println!("DEBUG {msg}");
                        return Err(eyre!("{msg}"));
                    }
                },
            }
        } // loop
    }

    /// Publishes a new version pointing to the trove provided
    /// which becomes the newly selected version
    /// Returns the selected version as a number
    pub async fn publish_new_version(
        &mut self,
        owner_secret_key: SecretKey,
        trove_address: &XorName,
    ) -> Result<(AttoTokens, u32)> {
        let (update_cost, _) = self.update_online(owner_secret_key, *trove_address).await?;
        println!("trove_address added to history: {trove_address:x}");
        let version = self.num_versions()?;
        self.default_version = Some(version);
        self.cached_version = Some(TroveVersion::<T>::new(version, trove_address.clone(), None));
        Ok((update_cost, version))
    }

    /// Makes the given version current by retrieving and storing the Trove.
    /// If version is None, selects the most recent version.
    /// The first version is 1, and the last version is History::num_versions()
    /// If version 0 or None is specified, the default/last version will be retrieved.
    /// After success, the trove can be accessed using current trove.
    /// If it fails, the selected version will be unchanged and any cached data retained.
    // Version 0 is hidden (and set to Trove::trove_type()) but can be accessed by
    // specifying a version of LARGEST_VERSION
    pub async fn fetch_version_trove(&mut self, version: Option<u32>) -> Option<T> {
        println!(
            "DEBUG fetch_version_trove(version: {version:?}) self.cached_version.is_some(): {}",
            self.cached_version.is_some()
        );
        let mut version = if version.is_some() {
            version.unwrap()
        } else {
            0
        };

        if version == 0 {
            if self.default_version.is_some() {
                version = self.default_version.unwrap()
            } else {
                println!("No default_version available to select");
                return None;
            }
        };

        // Allow access to the zeroth version
        let version = if version == LARGEST_VERSION {
            0
        } else {
            version
        };

        // Return if already cached
        if let Some(trove_version) = &self.cached_version {
            if trove_version.version == version && trove_version.trove.is_some() {
                return trove_version.trove.clone();
            }
        }

        let data_address = match self.get_trove_address_from_history(version).await {
            Ok(data_address) => data_address,
            Err(e) => {
                println!("select_version() failed to get version {version} from history:\n{e}");
                return None;
            }
        };

        let trove: Option<T> = match self.trove_download(data_address).await {
            Ok(trove) => Some(trove),
            Err(e) => {
                println!("select_version() failed to get trove from network:\n{e}");
                None
            }
        };

        self.cached_version = Some(TroveVersion::new(version, data_address, trove.clone()));
        trove
    }

    /// Get a copy of the cached TroveVersion<T>
    pub fn get_cached_version(&self) -> Option<TroveVersion<T>> {
        if let Some(cached_version) = &self.cached_version {
            Some(cached_version.clone())
        } else {
            None
        }
    }

    pub async fn get_trove_address_from_history(&mut self, version: u32) -> Result<XorName> {
        println!("DEBUG get_trove_address_from_history(version: {version})");
        // Use cached trove_version if available
        if let Some(trove_version) = &self.cached_version {
            if trove_version.version == version {
                println!(
                    "DEBUG get_trove_address_from_history() returning cached trove address: {}",
                    &trove_version.trove_address
                );
                return Ok(trove_version.trove_address.clone());
            }
        };
        self.get_version_entry_value(version).await
    }
}

/// The state of a Trove struct at a given version  with optional cache of its data
#[derive(Clone)]
pub struct TroveVersion<ST: Trove<ST> + Clone> {
    // Version of Some(trove) with address trove_address
    pub version: u32,

    trove_address: XorName,
    trove: Option<ST>,
}

impl<ST: Trove<ST> + Clone> TroveVersion<ST> {
    pub fn new(version: u32, trove_address: XorName, trove: Option<ST>) -> TroveVersion<ST> {
        TroveVersion {
            version,
            trove_address: trove_address,
            trove,
        }
    }

    pub fn trove_address(&self) -> XorName {
        self.trove_address
    }

    pub fn trove(&self) -> Option<ST> {
        match &self.trove {
            Some(trove) => Some(trove.clone()),
            None => None,
        }
    }
}
