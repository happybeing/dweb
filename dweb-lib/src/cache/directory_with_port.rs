/*
 Copyright (c) 2025 Mark Hughes

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

//! Cache of active per-port directory/website listeners.
//!
//! The server 'with ports' (default mode) uses a port per directory/website, and
//! adds a new listener each time a request for data at the address is
//! received. Redirection of the request to the new port causes the current
//! and subsequent requests to be served by the correct listener.

use std::fmt::{self, Display, Formatter};
use std::sync::{LazyLock, Mutex};

use color_eyre::eyre::{eyre, Result};
use schnellru::{ByLength, LruMap};

use autonomi::client::files::archive_public::ArchiveAddress;

use crate::cache::directory_with_name::HISTORY_NAMES;
use crate::client::DwebClient;
use crate::files::directory::Tree;
use crate::helpers::convert::*;
use crate::trove::HistoryAddress;

// TODO: tune cache size values
const WITH_PORT_CAPACITY: u32 = u16::MAX as u32; // When exceeded, port servers will be forgotten and new versions inaccessible

/// A cache of DirectoryVersionWithPort
///
/// Key:     ARCHIVE_ADDRESS
///
/// Entry:   DirectoryVersionWithPort
///

pub fn key_for_directory_versions_with_port(archive_address: ArchiveAddress) -> String {
    format!("{}", archive_address.to_hex()).to_ascii_lowercase()
}

// pub fn directory_versions_with_port_key(address: &str, version: Option<u32>) -> String {
//     let version_str = if version.is_some() {
//         &format!("{}", version.unwrap())
//     } else {
//         ""
//     };
//     format!("{address}-v{version_str}")
// }

// TODO use Mutex here because LazyLock.get_mut() is a Nightly Rust feature (01/2025)
pub static DIRECTORY_VERSIONS_WITH_PORT: LazyLock<Mutex<LruMap<String, DirectoryVersionWithPort>>> =
    LazyLock::new(|| {
        Mutex::<LruMap<String, DirectoryVersionWithPort>>::new(LruMap::<
            String,
            DirectoryVersionWithPort,
        >::new(ByLength::new(
            WITH_PORT_CAPACITY,
        )))
    });
#[derive(Clone)]
pub struct DirectoryVersionWithPort {
    /// The port on which a listener has been started - used for redirection of a URL by another listener
    pub port: u16,
    /// Address of a History<trove::Tree> on Autonomi
    pub history_address: Option<HistoryAddress>,
    /// A version of 0 implies use most recent version (highest available)
    pub version: Option<u32>,
    /// Directory / website metadata
    pub archive_address: ArchiveAddress,
    /// Directory / website metadata
    pub directory_tree: Tree,

    #[cfg(feature = "fixed-dweb-hosts")]
    is_fixed_webname: bool,
}

impl Display for DirectoryVersionWithPort {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "DirectoryVersionWithPort\n             port: {}\n  history_address: {}\n          version: {}\n  archive_address: {}",
            self.port,
            if self.history_address.is_some() { self.history_address.unwrap().to_hex() } else { "None".to_string() },
            if self.version.is_some() { self.version.unwrap() } else { 0 },
            self.archive_address.to_string(),
        )
    }
}

impl DirectoryVersionWithPort {
    pub fn new(
        port: u16,
        history_address: Option<HistoryAddress>,
        version: Option<u32>,
        archive_address: ArchiveAddress,
        directory_tree: Tree,
    ) -> DirectoryVersionWithPort {
        DirectoryVersionWithPort {
            port,
            history_address,
            version,
            archive_address,
            directory_tree,

            #[cfg(feature = "fixed-dweb-hosts")]
            is_fixed_webname: false,
        }
    }
}

/// Look-up the DirectoryVersionWithPort for a given address/version combination in the cache
/// and return it. If not found, create a DirectoryVersionWithPort with a free port for a given
/// address/version combination.
pub async fn lookup_or_create_directory_version_with_port(
    client: &DwebClient,
    address_or_name: &String,
    version: Option<u32>,
) -> Result<(DirectoryVersionWithPort, bool)> {
    let (history_address, archive_address) = tuple_from_address_or_name(address_or_name);

    let mut history_address = history_address;

    // If the address appears to be a name, try using that to get the history address
    if history_address.is_none() && archive_address.is_none() {
        if let Ok(lock) = &mut HISTORY_NAMES.lock() {
            let cached_address = lock.get(address_or_name);
            if cached_address.is_none() {
                return Err(eyre!(
                    "Unrecognised DWEB-NAME or invalid address: '{address_or_name}'"
                ));
            } else {
                history_address = Some(*cached_address.unwrap());
            }
        };
    };

    // Get the archive address and Tree
    let archive_address = if archive_address.is_none() {
        let min_entry = version.unwrap_or(1);
        match crate::trove::History::<Tree>::from_history_address(
            client.clone(),
            history_address.unwrap(),
            false,
            min_entry,
        )
        .await
        {
            Ok(mut history) => {
                let ignore_pointer = false;
                let version = version.unwrap_or(history.num_versions().unwrap_or(0));
                let archive_address = match history
                    .get_version_entry_value(version, ignore_pointer)
                    .await
                {
                    Ok(archive_address) => archive_address,
                    Err(e) => {
                        let msg =
                            format!("Unable to get archive address for version {version} - {e}");
                        println!("DEBUG {msg}");
                        return Err(eyre!(msg));
                    }
                };
                archive_address
            }
            Err(e) => {
                let msg = format!("Unable to create directory version because from_history_address() failed - {e}");
                println!("DEBUG {msg}");
                return Err(eyre!(msg));
            }
        }
    } else {
        archive_address.unwrap()
    };

    // Try the cache
    let key = key_for_directory_versions_with_port(archive_address);
    if let Ok(lock) = &mut DIRECTORY_VERSIONS_WITH_PORT.lock() {
        if let Some(directory_version) = lock.get(&key) {
            return Ok((directory_version.clone(), true));
        };
    };

    // Not in the cache, so create and add to cache
    let directory_tree = match Tree::from_archive_address(client, archive_address).await {
        Ok(directory_tree) => directory_tree,
        Err(e) => {
            let msg = format!("Failed to fetch archive from network - {e}");
            println!("DEBUG {msg}");
            return Err(eyre!(msg));
        }
    };

    // Create a new one on a free port
    if let Some(port) = port_check::free_local_port() {
        let directory_version = DirectoryVersionWithPort::new(
            port,
            history_address,
            version,
            archive_address,
            directory_tree,
        );

        // Add it to the cache
        if let Ok(lock) = &mut DIRECTORY_VERSIONS_WITH_PORT.lock() {
            let key = key_for_directory_versions_with_port(archive_address);
            if lock.insert(key, directory_version.clone()) {
                return Ok((directory_version, false));
            } else {
                let msg = format!("Failed to add new DirectoryVersionWithPort to the cache");
                println!("DEBUG {msg}");
                return Err(eyre!(msg));
            }
        }
        Ok((directory_version, false))
    } else {
        return Err(eyre!(
            "Unable to spawn a dweb server - no free ports available"
        ));
    }
}
