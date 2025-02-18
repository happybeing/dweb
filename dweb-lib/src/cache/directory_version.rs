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

//! Caching of dweb URLs and DWEB-NAMEs is used to both reduce network access for repeated
//! requests and to provide a local DNS based on DWEB-NAME, where each DWEB-NAME corresponds
//! do a DirectoryTree history (History<DirectoryTree>).

use std::sync::{LazyLock, Mutex};

use schnellru::{ByLength, LruMap};

use xor_name::XorName as ArchiveAddress;

use crate::trove::{directory_tree::DirectoryTree, HistoryAddress};
use crate::web::name::DwebHost;

// TODO: tune these values
const DWEB_NAMES_CAPACITY: u32 = 1000; // When exceeded, DWEB-NAMES will be forgotten and new versions inaccessible
const VERSIONS_CAPACITY: u32 = 1000; // When exceeded, particular versions will be dropped but will remain accessible so long as the DWEB-NAME is cached

// Note:
// I considered using DWEB-NAME.www-dweb.au as the key to avoid clashes of the same
// DWEB-NAME were used for an app (ie DWEB-NAME.app-dweb.au address) but since the DWEB-NAME
// contains a 16-bit disambibuator based on the HISTORY-ADDRESS, the chances of a clash
// are negligible.

/// DIRECTORY_VERSIONS is a cache of DirectoryVersion, the metadata needed to
/// access a specific version of a DirectoryTree corrsponding to a DwebHost string.
///
/// Key:     DwebHost.dweb_host_string, ie [vVERSION.]DWEB-NAME.www-dweb.au
///
/// Entry:   DirectoryVersion
///
// TODO use Mutex here because LazyLock.get_mut() is a Nightly Rust feature (01/2025)
pub static DIRECTORY_VERSIONS: LazyLock<Mutex<LruMap<String, DirectoryVersion>>> =
    LazyLock::new(|| {
        Mutex::<LruMap<String, DirectoryVersion>>::new(LruMap::<String, DirectoryVersion>::new(
            ByLength::new(VERSIONS_CAPACITY),
        ))
    });

/// HISTORY_NAMES is a cache which acts like local DNS, providing a lookup of DWEB-NAME
/// to HistoryAddress.
///
/// Key:     DWEB-NAME
///
/// Entry:   HistoryAddress
///
/// This cache is populated by a successful API call to create a DWEB-NAME, so long as a
/// a History can initialise using a supplied HISTORY-ADDRESS.
///
// TODO use Mutex here because LazyLock.get_mut() is a Nightly Rust feature (01/2025)
pub static HISTORY_NAMES: LazyLock<Mutex<LruMap<String, HistoryAddress>>> = LazyLock::new(|| {
    Mutex::<LruMap<String, HistoryAddress>>::new(LruMap::<String, HistoryAddress>::new(
        ByLength::new(DWEB_NAMES_CAPACITY),
    ))
});

#[derive(Clone)]
pub struct DirectoryVersion {
    /// The 'v[VERSION].DWEB-NAME.www-dweb.au' part of a dweb URL (see dweb::web::name)
    dweb_host_string: String,
    /// Address of a History<trove::DirectoryTree> on Autonomi (saves lookup based on DWEB-NAME.www-dweb.au)
    pub history_address: HistoryAddress,
    /// A version of 0 implies use most recent version (highest available)
    version: Option<u32>,
    /// Directory / website metadata
    pub archive_address: ArchiveAddress,
    /// Directory / website metadata
    pub directory_tree: Option<DirectoryTree>,

    #[cfg(feature = "fixed-dweb-hosts")]
    is_fixed_webname: bool,
}

impl DirectoryVersion {
    pub fn new(
        web_name: &DwebHost,
        history_address: HistoryAddress,
        archive_address: ArchiveAddress,
        directory_tree: Option<DirectoryTree>,
    ) -> DirectoryVersion {
        DirectoryVersion {
            dweb_host_string: web_name.dweb_host_string.clone(),
            history_address,
            version: web_name.version,
            archive_address,
            directory_tree,

            #[cfg(feature = "fixed-dweb-hosts")]
            is_fixed_webname: false,
        }
    }
}
