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

use autonomi::{graph::GraphError, GraphEntryAddress};
use color_eyre::eyre::{eyre, Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use xor_name::XorName;

use ant_protocol::storage::{GraphEntry, Pointer, PointerAddress as HistoryAddress, PointerTarget};
use autonomi::client::key_derivation::{DerivationIndex, MainSecretKey};
use autonomi::client::vault::VaultSecretKey as SecretKey;
use autonomi::AttoTokens;

use crate::autonomi::access::keys::get_vault_secret_key;
use crate::client::AutonomiClient;
use crate::data::autonomi_get_file_public;

const LARGEST_VERSION: u32 = u32::MAX;

/// The Trove trait enables any serializable struct to be saved in Autonomi
/// decentralised storage using a History<T> for your struct. The History then
/// gives access to every version of the struct that has ever been stored on Autonomi.
///
/// For example, using the built-in dweb::trove::DirectoryTree struct you can store and access
/// every published version of a website or tree of files.
///
/// Notes:
/// - the dweb-cli supports viewing of versioned websites and directories using
/// a standard web browser, including viewing every version published on Autonomi (similar
/// to the Internet Archive).
/// -  History manages a sequence of versions of a struct implementing Trove,
/// amounting to a versioned history for any struct impl Trove.
pub trait Trove {
    fn trove_type() -> XorName;
}

/// A history of versions of a type implementing the Trove trait. This
/// can be used to create and access versions of a file, a collection of
/// files such as a directory, or all the files and settings that make up a website,
/// and so on.
pub struct History<T: Trove + Serialize + DeserializeOwned + Clone> {
    client: AutonomiClient,

    // For operations when no version is specified. Typically, None implies most recent
    default_version: Option<u32>,
    // Cached data for the selected version
    cached_version: Option<TroveVersion<T>>,

    /// The history_secret_key is only required to create or if you later want to update it (not for access)
    history_secret_key: Option<SecretKey>,
    pointer: Pointer,

    // Pretend we hold a Trove so we can restrict some values to type T in the implementation
    phantom: std::marker::PhantomData<T>,
}

impl<T: Trove + Serialize + DeserializeOwned + Clone> History<T> {
    /// Create a History for read-write access and store it on the network
    /// Uses the name and a SecretKey to create a HistoryAddress
    /// If not provided, uses the default SecretKey
    /// To update a History, the name and SecretKey must be consistent
    // TODO:
    // [ ] Result<(cost, value>) returns for any functions pay to write data
    //
    // [ ] inspect-history
    // [ ] inspect-graph --root-address|--history-address|--pointer-address
    //
    // [x] History::root_entry_address(&self) -> GraphEntryAddress { self.pointer.network_address() }
    // [x] History::create_online()
    // [x] History::add_xor_name() -> History::update_online(new_value: T)
    // [x] create the root GraphEntry based on root_entry_address() with value Self::trove_type()
    // [x] create the pointer with that address as value
    // [x] History::get_version_entry()
    // [ ] History::get_version_entry() start at the nearest GraphEntry and iterate to the target, and return the value aas XorName
    // [x] change all version: u64 to u32:
    // [ ] review everywhere using LARGEST_VERSION
    // [ ] update notes about version 2^64-1 to 2^32-1
    // [ ]  and bash aliases related to that
    pub async fn create_online(
        client: AutonomiClient,
        name: String,
        app_secret_key: SecretKey,
    ) -> Result<(AttoTokens, Self)> {
        println!("DEBUG History::create_online({name})");

        // put the first entry in the graph
        let history_secret_key = app_secret_key.derive_child(name.as_bytes());
        let root_entry = create_graph_entry(&history_secret_key, None, Self::trove_type()).await?;
        println!("DEBUG call graph_entry_put()");
        let (graph_cost, root_entry_address) = client
            .client
            .graph_entry_put(root_entry, client.payment_option())
            .await?;

        let pointer = Self::create_pointer_for_update(0, &root_entry_address, &history_secret_key);
        match client
            .client
            .pointer_put(pointer.clone(), client.wallet.clone().into())
            .await
        {
            Ok((pointer_cost, pointer_address)) => {
                println!(
                    "DEBUG History::new() created new pointer at {:64x}",
                    pointer_address.xorname()
                );
                let history = History {
                    client: client.clone(),
                    default_version: None,
                    cached_version: None,
                    pointer,
                    history_secret_key: Some(history_secret_key),
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

    /// Load a History from the network that has read and write access
    /// Uses the name and the owner's SecretKey to create a history address and secret
    /// If the owner's SecretKey is not provided, uses the default SecretKey
    /// To update the History later, the name and SecretKey must be consistent
    pub async fn from_name(
        client: AutonomiClient,
        app_secret_key: SecretKey,
        name: String,
    ) -> Result<(AttoTokens, Self)> {
        println!("DEBUG History::from_name({name})");

        let history_secret_key = app_secret_key.derive_child(name.as_bytes());
        let history_address = HistoryAddress::from_owner(history_secret_key.public_key());
        match client.client.pointer_get(history_address).await {
            Ok(pointer) => {
                println!(
                    "DEBUG History::from_name() obtained pointer from {:64x}",
                    pointer.network_address().xorname()
                );

                let history = History {
                    client: client.clone(),
                    default_version: None,
                    cached_version: None,
                    pointer,
                    history_secret_key: Some(history_secret_key),
                    phantom: PhantomData,
                };

                Ok((Into::into(0), history))
            }
            Err(e) => {
                let message = format!("History::from_name() pointer not found on network: {e}");
                println!("DEBUG {message}");
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

    /// Load a read-only History from the network
    /// The owner_secret is not required for read access, only if doing publish/update subsequently
    pub async fn from_history_address(
        client: AutonomiClient,
        history_address: HistoryAddress,
    ) -> Result<History<T>> {
        println!(
            "DEBUG History::from_history_address({})",
            history_address.to_hex()
        );

        // Check it exists to avoid accidental creation (and payment)
        let result = client.client.pointer_get(history_address).await;
        let mut history = if result.is_ok() {
            History::<T> {
                client,
                default_version: None,
                cached_version: None,
                pointer: result.unwrap(),
                history_secret_key: None,
                phantom: PhantomData,
            }
        } else {
            println!("DEBUG from_history_address() error:");
            return Err(eyre!("History pointer not found on network"));
        };
        history.update_default_version();
        Ok(history)
    }

    /// The address of the root entry (GraphEntry) which stores Trove type, not the first value
    ///
    /// The value of this entry will be the Trove<T>::trove_type() which provides a way to check that the
    /// stored History is of the type expected, but means that the first value of that type will be in
    /// the second to earliest entry.
    fn root_entry_address(&self) -> GraphEntryAddress {
        GraphEntryAddress::new(*self.pointer.address().xorname())
    }

    /// The address of the head in the current pointer
    /// Does not update pointer from network
    fn head_entry_address(&self) -> GraphEntryAddress {
        GraphEntryAddress::new(self.pointer.target().xorname())
    }

    fn history_secret_key(&self) -> Result<SecretKey, Error> {
        match self.history_secret_key.clone() {
            Some(history_secret_key) => Ok(history_secret_key),
            None => Err(eyre!(
                "ERROR: History can't be updated without ::owner_secret"
            )),
        }
    }

    fn update_default_version(&mut self) -> Option<u32> {
        self.default_version = match self.num_versions() {
            Ok(version) => Some(version),
            Err(_) => None,
        };
        println!(
            "update_default_version() set to {}",
            self.default_version.unwrap()
        );
        self.default_version
    }

    pub fn history_address(&self) -> HistoryAddress {
        self.pointer.network_address()
    }

    fn trove_type() -> XorName {
        T::trove_type()
    }

    /// Return the number of entries in the history
    /// This is one more than the number of versions
    /// because the first entry is reserved for use
    /// as a type (which may point to metadata about
    /// the Trove type). Example types include file
    /// system and website.
    pub fn num_entries(&self) -> u32 {
        self.pointer.counter()
    }

    /// Return the number of available versions
    /// or an error if no versions are available.
    /// The first version is 1 last version is num_versions()
    pub fn num_versions(&self) -> Result<u32> {
        let num_entries = self.pointer.counter() + 1;

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
        println!("DEBUG directory_tree_download() at {data_address:64x}");
        match autonomi_get_file_public(client, &data_address).await {
            Ok(content) => {
                println!("Retrieved {} bytes", content.len());
                let trove: T = match rmp_serde::from_slice(&content) {
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
    pub async fn get_version_entry(&mut self, version: u32) -> Result<XorName> {
        println!("DEBUG History::get_version_entry(version: {version})");
        self.update_pointer().await?;
        let num_entries = self.pointer.counter() + 1;

        // The first entry is the Trove<T>::trove_type(), and not used so max version is num_entries - 1
        let max_version = if num_entries > 0 { num_entries - 1 } else { 0 };

        if version > max_version {
            let message = format!(
                "History::get_version_entry({version}) out of range for max_version: {max_version}"
            );
            println!("{message}");
            return Err(eyre!(message));
        }

        self.get_entry(version - 1).await
    }

    pub async fn update_pointer(&mut self) -> Result<()> {
        self.pointer = self
            .client
            .client
            .pointer_get(self.pointer.network_address())
            .await?;
        Ok(())
    }

    /// Get the value by absolute index, which is offset by one from that returned by get_version_entry()
    pub async fn get_entry(&mut self, index: u32) -> Result<XorName> {
        println!("DEBUG History::get_entry(index: {index})");
        self.update_pointer().await?;
        let num_entries = self.pointer.counter() + 1;

        if index >= num_entries {
            return Err(eyre!(
                "Index out of range, index: {index}, number of entries {num_entries}"
            ));
        };

        // TODO various ways this might be made more efficient (e.g. start at the closest end to index)

        let mut iter_entry = match self.get_head_entry().await {
            Ok(head) => {
                if head.is_some() {
                    head.unwrap()
                } else {
                    return Err(eyre!("Empty history has no head entry()"));
                }
            }
            Err(e) => return Err(e),
        };

        let iter_index = num_entries - 1;
        while index < iter_index {
            iter_entry = if let Some(entry) = self.get_parent_entry_of(&iter_entry).await? {
                entry
            } else {
                return Err(eyre!(
                    "Ran out of entries - probably a bug in History::get_entry()"
                ));
            }
        }

        let trove_address = XorName::from_content(&iter_entry.content);
        Ok(trove_address)
    }

    // Get a GraphEntry from the network
    async fn get_graph_entry_from_network(
        &self,
        graph_entry_address: &GraphEntryAddress,
    ) -> Result<GraphEntry> {
        match self
            .client
            .client
            .graph_entry_get(*graph_entry_address)
            .await
        {
            Ok(entry) => Ok(entry),
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
                    entry
                } else {
                    return Err(eyre!(
                        "No valid descendants found for forked entry at {graph_entry_address:?}"
                    ));
                };
                Ok(entry_by_smallest_derivation)
            }
            Err(err) => return Err(err.into()),
        }
    }

    // Does not need to update pointer
    async fn get_root_entry(&self) -> Result<Option<GraphEntry>> {
        Ok(Some(
            self.get_graph_entry_from_network(&self.root_entry_address())
                .await?,
        ))
    }

    // Get the most recent GraphEntry
    // Does not update pointer from network
    async fn get_head_entry(&self) -> Result<Option<GraphEntry>> {
        Ok(Some(
            self.get_graph_entry_from_network(&self.head_entry_address())
                .await?,
        ))
    }

    // Get the parent of a GraphEntry
    // Does not update pointer from network
    pub async fn get_parent_entry_of(
        &self,
        graph_entry: &GraphEntry,
    ) -> Result<Option<GraphEntry>> {
        let parent = GraphEntryAddress::from_owner(graph_entry.parents[0]);
        Ok(Some(self.get_graph_entry_from_network(&parent).await?))
    }

    // Get the child (first descendent) of a GraphEntry
    // Does not update pointer from network
    pub async fn get_child_entry(&self, graph_entry: &GraphEntry) -> Result<Option<GraphEntry>> {
        if graph_entry.descendants.len() > 0 {
            let (child_address, _) = graph_entry.descendants[0];
            Ok(Some(
                self.get_graph_entry_from_network(&GraphEntryAddress::from_owner(child_address))
                    .await?,
            ))
        } else {
            Ok(None)
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
    async fn history_get_graph_entry_and_next_derivation_index(
        &self,
        graph_entry_address: &GraphEntryAddress,
    ) -> Result<(GraphEntry, DerivationIndex)> {
        let entry = self
            .get_graph_entry_from_network(graph_entry_address)
            .await?;
        let new_derivation = get_derivation_from_graph_entry(&entry)?;
        Ok((entry, new_derivation))
    }

    /// Add a trove to the History and return the index of the most recent entry (1 = first trove entry, 0 = root entry)
    pub async fn update_online(&mut self, trove_address: XorName) -> Result<(AttoTokens, u32)> {
        let history_address = self.pointer.network_address().to_hex();
        println!("Updating History at {history_address}");

        match self
            .client
            .client
            .pointer_get(self.pointer.network_address())
            .await
        {
            Ok(old_pointer) => {
                if !old_pointer.verify_signature() {
                    let message =
                        format!("Error - pointer retrieved from network has INVALID SIGNATURE");
                    println!("{message}");
                    return Err(eyre!(message));
                }

                let history_secret_key = if self.history_secret_key.is_some() {
                    self.history_secret_key.clone().unwrap()
                } else {
                    return Err(eyre!("Cannot update Pointer - owner secret key is None"));
                };

                let head_address = match old_pointer.target() {
                    PointerTarget::GraphEntryAddress(address) => address,
                    other => return Err(eyre!("Invalid head address {:?}", other.clone())),
                };

                let (graph_cost, next_address) = match self
                    .create_next_graph_entry_online(
                        history_secret_key.clone(),
                        *head_address,
                        &trove_address,
                    )
                    .await
                {
                    Ok((cost, address)) => (cost, address),
                    Err(e) => return Err(eyre!("failed to create next GraphEnry: {e}")),
                };

                println!("Pointer retrieved with counter {}", old_pointer.counter());
                let new_pointer = Self::create_pointer_for_update(
                    old_pointer.counter() + 1,
                    &next_address,
                    &history_secret_key,
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
                        self.pointer = new_pointer.clone();
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

    async fn create_next_graph_entry_online(
        &self,
        history_secret_key: SecretKey,
        head_address: GraphEntryAddress,
        content: &XorName,
    ) -> Result<(AttoTokens, GraphEntryAddress)> {
        // get the next derivation index from the current most recent entry
        let (parent_entry, _new_derivation) = self
            .history_get_graph_entry_and_next_derivation_index(&head_address)
            .await?;

        let new_entry =
            create_graph_entry(&history_secret_key, Some(&parent_entry), *content).await?;
        Ok(self
            .client
            .client
            .graph_entry_put(new_entry, self.client.payment_option())
            .await?)
    }

    /// Publishes a new version pointing to the trove provided
    /// which becomes the newly selected version
    /// Returns the selected version as a number
    pub async fn publish_new_version(
        &mut self,
        trove_address: &XorName,
    ) -> Result<(AttoTokens, u32)> {
        let (update_cost, _) = self.update_online(*trove_address).await?;
        println!("trove_address added to history: {trove_address:64x}");
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
        self.get_version_entry(version).await
    }
}

// create a new entry with the new value
async fn create_graph_entry(
    history_secret_key: &SecretKey,
    parent_entry: Option<&GraphEntry>,
    new_value: XorName,
) -> Result<GraphEntry> {
    let app_secret_key = MainSecretKey::new(history_secret_key.clone());
    let parents = if let Some(parent_entry) = parent_entry {
        vec![parent_entry.owner]
    } else {
        vec![]
    };

    let content: [u8; 32] = new_value.to_vec().as_slice().try_into()?;
    let next_derivation = DerivationIndex::random(&mut rand::thread_rng());
    let next_public_key = app_secret_key.public_key().derive_key(&next_derivation);
    let descendants = vec![(next_public_key.into(), next_derivation.into_bytes())];

    Ok(GraphEntry::new(
        &app_secret_key.into(),
        parents,
        content,
        descendants,
    ))
}

fn get_derivation_from_graph_entry(entry: &GraphEntry) -> Result<DerivationIndex> {
    let graph_entry_addr = GraphEntryAddress::from_owner(entry.owner);
    let d = match entry.descendants.as_slice() {
        [d] => d.1,
        // TODO maybe just use first descendent rather than error?
        _ => {
            return Err(eyre!(
            "History graph_entry_addr: {:?} is corrupted, expected one descendant but got {}: {:?}",
            graph_entry_addr,
            entry.descendants.len(),
            entry.descendants
        ));
        }
    };
    Ok(DerivationIndex::from_bytes(d))
}

/// The state of a Trove struct at a given version  with optional cache of its data
#[derive(Clone)]
pub struct TroveVersion<ST: Trove + Serialize + DeserializeOwned + Clone> {
    // Version of Some(trove) with address trove_address
    pub version: u32,

    trove_address: XorName,
    trove: Option<ST>,
}

impl<ST: Trove + Serialize + DeserializeOwned + Clone> TroveVersion<ST> {
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
