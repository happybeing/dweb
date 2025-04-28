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

use std::marker::PhantomData;

use autonomi::client::payment::PaymentOption;
use autonomi::files::archive_public::ArchiveAddress;
use blsttc::PublicKey;
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

use autonomi::client::data::DataAddress;
use autonomi::client::data_types::graph::{GraphContent, GraphError};
use autonomi::client::key_derivation::{DerivationIndex, MainPubkey, MainSecretKey};
use autonomi::SecretKey;
use autonomi::{
    pointer::PointerTarget, AttoTokens, Bytes, GraphEntry, GraphEntryAddress, Pointer,
    PointerAddress,
};

use crate::client::DwebClient;
use crate::data::autonomi_get_file_public;
use crate::helpers::graph_entry::{
    create_graph_entry, get_derivation_from_graph_entry, graph_entry_get,
};
use crate::helpers::retry::retry_until_ok;
use crate::token::{show_spend_return_value, Spends};

use crate::types::{derive_named_object_secret, HISTORY_POINTER_DERIVATION_INDEX};

const LARGEST_VERSION: u32 = u32::MAX;

/// The value of a history: a 32 bytes array (same as [`GraphContent`])
pub type HistoryValue = GraphContent;

/// The size of a history value: 32 bytes
pub const HISTORY_VALUE_SIZE: usize = size_of::<HistoryValue>();

/// Create a new [`HistoryValue`] from bytes, make sure the bytes are not longer than [`HISTORY_VALUE_SIZE`]
pub fn history_value_from_bytes(bytes: &[u8]) -> Result<HistoryValue> {
    if bytes.len() > HISTORY_VALUE_SIZE {
        return Err(eyre!(
            "history_value_from_bytes() invalid length of bytes: {}",
            bytes.len()
        ));
    }
    let mut content: HistoryValue = [0; HISTORY_VALUE_SIZE];
    content[..bytes.len()].copy_from_slice(bytes);
    Ok(content)
}

/// A History is addressed at a [`HistoryAddress`] which is derived from the owner's
/// [`PublicKey`] and a name. This means a single owner key can manage multiple
/// histories.
///
/// Any data stored in the register is stored as is, without encryption or modifications.
/// Since the data is publicly accessible by anyone knowing the [`HistoryAddress`],
/// it is up to the owner to encrypt the data uploaded to the register, if wanted.
/// Only the owner can update the register with its [`SecretKey`].
/// The [`SecretKey`] is the only piece of information an owner should keep to access to the register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        GraphEntryAddress::new(self.owner)
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
/// Example, using the built-in dweb::trove::Tree struct you can store and access
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
    fn trove_type() -> DataAddress;
    fn to_bytes(trove: &T) -> Result<Bytes>;
    async fn from_bytes(client: &DwebClient, bytes: Bytes) -> Result<T>;
}

/// A history of versions of a type implementing the Trove trait. This
/// can be used to create and access versions of a file, a collection of
/// files such as a directory, or all the files and settings that make up a website,
/// and so on.
/// TODO provide a way to initialise a history from an Autonomi Register pointer and graph
//  TODO this will require storing the HISTORY_DERIVATION_INDEX in the History, and
//  TODO initialising it according to whether or not it is initialised from a Register .
//  TODO Also changes to History::history_main_secret_key() for compatibility with Registers
pub struct History<T: Trove<T> + Clone> {
    client: DwebClient,

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
    pub cached_version: Option<TroveVersion<T>>,

    // Pretend we hold a Trove so we can restrict some values to type T in the implementation
    phantom: std::marker::PhantomData<T>,
}

impl<T: Trove<T> + Clone> History<T> {
    /// Create a new History for read-write access and store it on the network
    /// To update the history use the same owner_secret_key
    /// name cannot be an empty string
    pub async fn create_online(
        client: DwebClient,
        name: String,
        owner_secret_key: SecretKey,
    ) -> Result<(AttoTokens, Self)> {
        println!("DEBUG History::create_online({name})");
        if name.is_empty() {
            return Err(eyre!(
                "History::create_online() failed - cannot use an empty name"
            ));
        }
        let spends = Spends::new(&client, Some(&"History create online cost: ")).await?;

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
                    return show_spend_return_value::<Result<(AttoTokens, Self)>>(
                        &spends,
                        Err(eyre!("Invalid cost")),
                    )
                    .await;
                };
                Ok((total_cost, history))
            }
            Err(e) => {
                let message = format!("History::new() failed to create pointer: {e}");
                println!("DEBUG {message}");
                return show_spend_return_value::<Result<(AttoTokens, Self)>>(
                    &spends,
                    Err(eyre!("Invalid cost")),
                )
                .await;
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
        client: DwebClient,
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
        let ignore_pointer = client.api_control.ignore_pointers.unwrap_or(ignore_pointer);

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

        let pointer_target = match pointer.target() {
            PointerTarget::GraphEntryAddress(pointer_target) => *pointer_target,
            other => {
                return Err(eyre!(
                "History::from_name() pointer target is not a GraphEntry - this is probably a bug. Target: {other:?}"
            ))
            }
        };

        let mut history = History {
            client: client.clone(),
            name,
            history_address,
            num_entries: 0,
            head_graphentry: None,
            pointer_counter: pointer.counter(),
            pointer_target: Some(pointer_target),
            default_version: None,
            cached_version: None,
            phantom: PhantomData,
        };
        // Necessary because the pointer may not be up-to-data
        if ignore_pointer || (!ignore_pointer && minimum_entry_index > pointer.counter()) {
            // Ignore the pointer because that was specified,
            // or the pointer counter() is behind minimum_entry_index
            history
                .update_from_graph_internal(&pointer_target, pointer.counter())
                .await?;
        } else {
            // Use the pointer even though it may not be up-to-date
            match history
                .get_graph_entry_from_network(&pointer_target, false)
                .await
            {
                Ok(pointer_head) => {
                    history.num_entries = pointer.counter() + 1;
                    history.head_graphentry = Some(pointer_head);
                    history.pointer_counter = pointer.counter() + 1;
                    history.pointer_target = Some(pointer_target);
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
        client: DwebClient,
        history_address: HistoryAddress,
        ignore_pointer: bool,
        minimum_entry_index: u32,
    ) -> Result<History<T>> {
        println!(
            "DEBUG History::from_history_address({})",
            history_address.to_hex()
        );
        let ignore_pointer = client.api_control.ignore_pointers.unwrap_or(ignore_pointer);

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

        let pointer_target = match pointer.target() {
            PointerTarget::GraphEntryAddress(pointer_target) => *pointer_target,
            other => {
                return Err(eyre!(
                "History::from_history_address() pointer target is not a GraphEntry - this is probably a bug. Target: {other:?}"
            ))
            }
        };

        let mut history = History::<T> {
            client,
            name: String::from(""),
            history_address,
            num_entries: 0,
            head_graphentry: None,
            pointer_counter: pointer.counter(),
            pointer_target: Some(pointer_target),
            default_version: None,
            cached_version: None,
            phantom: PhantomData,
        };
        // Necessary because the pointer may not be up-to-data
        if ignore_pointer || (!ignore_pointer && minimum_entry_index > pointer.counter()) {
            // Ignore the pointer because that was specified,
            // or the pointer counter() is behind minimum_entry_index
            history
                .update_from_graph_internal(&pointer_target, pointer.counter())
                .await?;
        } else {
            // Use the pointer even though it may not be up-to-date
            match history
                .get_graph_entry_from_network(&pointer_target, false)
                .await
            {
                Ok(pointer_head) => {
                    history.num_entries = pointer.counter() + 1;
                    history.head_graphentry = Some(pointer_head);
                    history.pointer_counter = pointer.counter() + 1;
                    history.pointer_target = Some(pointer_target);
                }
                Err(e) => return Err(eyre!("Failed to get pointer target entry - {e}")),
            };
        }

        println!(
            "DEBUG from_history_address() returning History with num_entries: {}",
            history.num_entries
        );
        history.update_default_version();
        Ok(history)
    }

    /// Safely get the actual head even if the pointer_target is not the heaad.
    ///
    /// If the pointer_target is out of date this function scans the graph starting at pointer_target
    /// until it reaches the end and can correctly set the head GraphEntry and num_entries.
    ///
    /// This will only happen if the target is out of date, so a maximum of once after the history
    /// is created. When it does, it will take few seconds because it has to wait for the request
    /// for a graph entry to not be found on the network.
    ///
    /// Returns the head GraphEntry
    pub async fn update_from_graph(&mut self) -> Result<GraphEntry> {
        println!("DEBUG History::update_from_graph()");
        if self.head_graphentry.is_some() {
            return Ok(self.head_graphentry.clone().unwrap());
        }

        // Check the pointer exists to avoid accidental creation (and payment)
        let pointer_address =
            Self::pointer_address_from_history_address(self.history_address.clone())?;
        let pointer = match Self::get_and_verify_pointer(&self.client, &pointer_address).await {
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

        let pointer_target = match pointer.target() {
            PointerTarget::GraphEntryAddress(pointer_target) => *pointer_target,
            other => {
                return Err(eyre!(
                "History::from_history_address() pointer target is not a GraphEntry - this is probably a bug. Target: {other:?}"
            ))
            }
        };

        self.update_from_graph_internal(&pointer_target, pointer.counter())
            .await
    }

    // See update_from_graph() for description
    async fn update_from_graph_internal(
        &mut self,
        pointer_target: &GraphEntryAddress,
        pointer_counter: u32,
    ) -> Result<GraphEntry> {
        println!("DEBUG History::update_from_graph_internal()");
        if self.head_graphentry.is_some() {
            return Ok(self.head_graphentry.clone().unwrap());
        }

        // Get the Pointer target entry and move forwards - because the pointer may not be up to date
        let pointer_target_entry = match self
            .get_graph_entry_from_network(pointer_target, false)
            .await
        {
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
            iter_entry = if let Some(entry) = self.get_child_entry_of(&iter_entry, true).await {
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
        client: &DwebClient,
        pointer_address: &PointerAddress,
    ) -> Result<Pointer> {
        retry_until_ok(
            client.api_control.tries,
            &"pointer_get()",
            (client, pointer_address),
            async move |(client, pointer_address)| match client
                .client
                .pointer_get(pointer_address)
                .await
            {
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
            },
        )
        .await
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
        // If I need to wipe the History<Tree> address space clean, tweak
        // and re-upload the awv site type use the new value for FILE_TREE_TYPE
        let derivation_index: [u8; 32] = Self::trove_type().xorname().to_vec().try_into().unwrap();
        MainSecretKey::new(owner_secret_key.clone())
            .derive_key(&DerivationIndex::from_bytes(derivation_index))
            .into()
    }

    /// Get the main secret key for the pointer belonging to a history
    fn history_pointer_secret_key(history_secret_key: SecretKey) -> SecretKey {
        derive_named_object_secret(
            history_secret_key,
            &HISTORY_POINTER_DERIVATION_INDEX,
            &None,
            None,
            None,
        )
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
        let derivation_index: [u8; 32] = HISTORY_POINTER_DERIVATION_INDEX
            .as_bytes()
            .try_into()
            .unwrap();
        let pointer_pk =
            history_main_public_key.derive_key(&DerivationIndex::from_bytes(derivation_index));
        Ok(PointerAddress::new(pointer_pk.into()))
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

    pub fn trove_type() -> ArchiveAddress {
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

    /// Download a `Tree` from the network
    async fn trove_download(&self, data_address: ArchiveAddress) -> Result<T> {
        return History::<T>::raw_trove_download(&self.client, data_address).await;
    }

    /// Type-safe download directly from the network.
    /// Useful if you already have the address and don't want to initialise a History
    pub async fn raw_trove_download(
        client: &DwebClient,
        data_address: ArchiveAddress,
    ) -> Result<T> {
        println!(
            "DEBUG directory_tree_download() at {}",
            data_address.to_hex()
        );

        retry_until_ok(
            client.api_control.tries,
            &"autonomi_get_file_public()",
            (client, data_address),
            async move |(client, data_address)| match autonomi_get_file_public(
                client,
                &data_address,
            )
            .await
            {
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
            },
        )
        .await
    }

    /// Get the entry value for the given version.
    /// The root entry (index 0) is not a valid version so the earliest version
    /// is version 1, and passing a value of 0 will retrieve the most recent
    /// version.
    ///
    /// Note: when retrieving the most recent entry (version passed as 0) it will
    /// either assume the Pointer is up-to-date, or if ignore_pointer is true it
    /// will traverse the graph from the Pointer entry to the end. Doing the latter
    /// is much slower because it takes time to determine that the next entry does not
    /// exist (minutes as of March 2025). The ignore pointer option is provided
    /// because pointers can take an unknown time to be updated.
    pub async fn get_version_entry_value(
        &mut self,
        version: u32,
        ignore_pointer: bool,
    ) -> Result<ArchiveAddress> {
        println!("DEBUG History::get_version_entry_value(version: {version})");
        if ignore_pointer {
            self.update_from_graph().await?;
        }

        let num_entries = self.num_entries();
        let version = if version == 0 {
            num_entries - 1
        } else {
            version
        };

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
    pub async fn get_entry_value(&mut self, index: u32) -> Result<ArchiveAddress> {
        println!("DEBUG History::get_entry_value(index: {index})");
        match self.get_graph_entry(index).await {
            Ok(entry) => {
                if let Ok(entry) = ArchiveAddress::from_hex(&hex::encode(entry.content)) {
                    Ok(entry)
                } else {
                    Err(eyre!("History::get_entry_value() - invalid ArchiveAddress in GraphEntry - probably a bug"))
                }
            }
            Err(e) => Err(eyre!("History::get_entry_value() - {e}")),
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
                iter_entry = if let Some(entry) = self.get_child_entry_of(&iter_entry, false).await
                {
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
        check_exists: bool,
    ) -> Result<GraphEntry> {
        // println!(
        //     "DEBUG get_graph_entry_from_network() at {}",
        //     graph_entry_address.to_hex()
        // );

        Ok(graph_entry_get(&self.client.client, graph_entry_address, check_exists).await?)
    }

    // Does not need to update pointer
    pub async fn get_root_entry(&self) -> Result<Option<GraphEntry>> {
        Ok(Some(
            self.get_graph_entry_from_network(
                &Self::root_graph_entry_address(GraphEntryAddress::new(
                    self.history_address.owner(),
                )),
                false,
            )
            .await?,
        ))
    }

    /// Get the most recent GraphEntry
    pub async fn get_head_entry(&self) -> Result<Option<GraphEntry>> {
        Ok(Some(
            self.get_graph_entry_from_network(&self.head_entry_address()?, false)
                .await?,
        ))
    }

    /// Get the parent of a GraphEntry
    pub async fn get_parent_entry_of(
        &self,
        graph_entry: &GraphEntry,
    ) -> Result<Option<GraphEntry>> {
        let parent = GraphEntryAddress::new(graph_entry.parents[0]);
        Ok(Some(
            self.get_graph_entry_from_network(&parent, false).await?,
        ))
    }

    /// Get the child of a GraphEntry
    /// Assumes each entry has only one descendent
    pub async fn get_child_entry_of(
        &self,
        graph_entry: &GraphEntry,
        check_exists: bool,
    ) -> Option<GraphEntry> {
        // // TODO I don't understand why this isn't sufficient:
        // let child = GraphEntryAddress::from_owner(graph_entry.descendants[0].0);

        // TODO this is how Autonomi History does it:
        let next_derivation = DerivationIndex::from_bytes(graph_entry.descendants[0].1);
        let next_entry_pk: PublicKey = MainPubkey::from(self.history_address().owner)
            .derive_key(&next_derivation)
            .into();
        let child = GraphEntryAddress::new(next_entry_pk);

        match self
            .get_graph_entry_from_network(&child, check_exists)
            .await
        {
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
        let entry = match self
            .get_graph_entry_from_network(graph_entry_address, false)
            .await
        {
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
        trove_address: ArchiveAddress,
    ) -> Result<(AttoTokens, u32)> {
        println!("DEBUG History::update_online()");
        let history_secret_key =
            Self::history_main_secret_key(owner_secret_key).derive_child(self.name.as_bytes());

        let history_address = HistoryAddress::new(history_secret_key.public_key());
        println!("Updating History at {}", history_address.to_hex());

        let spends = Spends::new(&self.client, Some(&"History update online cost: ")).await?;
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
                    Err(e) => {
                        return show_spend_return_value::<Result<(AttoTokens, u32)>>(
                            &spends,
                            Err(eyre!("failed to create next GraphEnry: {e}")),
                        )
                        .await;
                    }
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
                let client = self.client.client.clone();
                let new_pointer_clone = new_pointer.clone();
                let payment_option: PaymentOption = self.client.wallet.clone().into();
                match retry_until_ok(
                    self.client.api_control.tries,
                    &"pointer_put()",
                    (client, new_pointer_clone.clone(), payment_option),
                    async move |(client, new_pointer_clone, payment_option)| match client
                        .pointer_put(new_pointer_clone, payment_option)
                        .await
                    {
                        Ok(result) => Ok(result),
                        Err(e) => {
                            return Err(eyre!("Failed to add a trove to history: {e:?}"));
                        }
                    },
                )
                .await
                {
                    Ok((pointer_cost, _pointer_address)) => {
                        self.pointer_counter = new_pointer_clone.counter();
                        self.pointer_target = match new_pointer_clone.target() {
                            PointerTarget::GraphEntryAddress(pointer_target) => {
                                Some(*pointer_target)
                            }
                            other => {
                                return show_spend_return_value::<Result<(AttoTokens, u32)>>(&spends, Err(eyre!(
                                    "History::update_online() pointer target is not a GraphEntry - this is probably a bug. Target: {other:?}"
                                )),
                            )
                            .await;
                            }
                        };
                        let total_cost = pointer_cost.checked_add(graph_cost);
                        if total_cost.is_none() {
                            return Err(eyre!("Invalid cost"));
                        }
                        return show_spend_return_value::<Result<(AttoTokens, u32)>>(
                            &spends,
                            Ok((total_cost.unwrap(), new_pointer_clone.counter())),
                        )
                        .await;
                    }
                    Err(e) => return Err(eyre!("Retries exceeded: {e:?}")),
                }
            }
            Err(e) => return Err(eyre!("DEBUG failed to get history prior to update!\n{e}")),
        }
    }

    /// Create the next graph entry.
    /// Begins at the provided head_address but handles the case where this is
    /// not the head by moving along the graph until it finds the real head.
    async fn create_next_graph_entry_online(
        &self,
        history_secret_key: SecretKey,
        head_address: GraphEntryAddress,
        content: &ArchiveAddress,
    ) -> Result<(AttoTokens, GraphEntryAddress)> {
        println!(
            "DEBUG create_next_graph_entry_online() with content {}",
            content.to_hex()
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

            // println!("DEBUG new_entry: {new_entry:?}");
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
        trove_address: &ArchiveAddress,
    ) -> Result<(AttoTokens, u32)> {
        let (update_cost, _) = self.update_online(owner_secret_key, *trove_address).await?;
        println!("trove_address added to history: {}", trove_address.to_hex());
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

    pub async fn get_trove_address_from_history(&mut self, version: u32) -> Result<ArchiveAddress> {
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
        let ignore_pointer = false;
        self.get_version_entry_value(version, ignore_pointer).await
    }
}

/// The state of a Trove struct at a given version  with optional cache of its data
#[derive(Clone)]
pub struct TroveVersion<ST: Trove<ST> + Clone> {
    // Version of Some(trove) with address trove_address
    pub version: u32,

    pub trove_address: ArchiveAddress,
    pub trove: Option<ST>,
}

impl<ST: Trove<ST> + Clone> TroveVersion<ST> {
    pub fn new(version: u32, trove_address: ArchiveAddress, trove: Option<ST>) -> TroveVersion<ST> {
        TroveVersion {
            version,
            trove_address: trove_address,
            trove,
        }
    }

    pub fn trove_address(&self) -> ArchiveAddress {
        self.trove_address
    }

    pub fn trove(&self) -> Option<ST> {
        match &self.trove {
            Some(trove) => Some(trove.clone()),
            None => None,
        }
    }
}
