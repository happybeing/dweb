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

//! Caching of dweb URLs and SHORTNAMEs is used to both reduce network access for repeated
//! requests and to provide a local DNS based on SHORTNAME, where each SHORTNAME corresponds
//! do a DirectoryTree history (History<DirectoryTree>).

use std::sync::{LazyLock, Mutex};

use schnellru::{ByLength, LruMap};

use ant_registers::RegisterAddress as HistoryAddress;
use xor_name::XorName as DirectoryAddress;

use crate::trove::directory_tree::DirectoryTree;
use crate::web::name::WebName;

// TODO: tune these values
const SHORTNAMES_CAPACITY: u32 = 1000; // When exceeded, SHORTNAMES will be forgotten and new versions inaccessible
const VERSIONS_CAPACITY: u32 = 1000; // When exceeded, particular versions will be dropped but will remain accessible so long as the SHORTNAME is cached

// Note:
// I considered using SHORTNAME.www-dweb.au as the key to avoid clashes of the same
// SHORTNAME were used for an app (ie SHORTNAME.app-dweb.au address) but since the SHORTNAME
// contains a 16-bit disambibuator based on the HISTORY-ADDRESS, the chances of a clash
// are negligible.

/// DIRECTORY_VERSIONS is a cache of DirectoryVersion, the metadata needed to
/// access a specific version of a DirectoryTree corrsponding to a WebName string.
///
/// Key:     WebName.web_name_string, ie v[VERSION].SHORTNAME.www-dweb.au
///
/// Entry:   DirectoryVersion
///
/// TODO: consider persisting the caches (do any feature serde?)
// TODO use Mutex here because LazyLock.get_mut() is a Nightly Rust feature (01/2025)
pub static DIRECTORY_VERSIONS: LazyLock<Mutex<LruMap<String, DirectoryVersion>>> =
    LazyLock::new(|| {
        Mutex::<LruMap<String, DirectoryVersion>>::new(LruMap::<String, DirectoryVersion>::new(
            ByLength::new(VERSIONS_CAPACITY),
        ))
    });

/// HISTORY_NAMES is a cache which acts like local DNS, providing a lookup of SHORTNAME
/// to HistoryAddress.
///
/// Key:     SHORTNAME
///
/// Entry:   HistoryAddress
///
/// This cache is populated by a successful API call to create a SHORTNAME, so long as a
/// a History can initialise using a supplied HISTORY-ADDRESS.
///
// TODO use Mutex here because LazyLock.get_mut() is a Nightly Rust feature (01/2025)
pub static HISTORY_NAMES: LazyLock<Mutex<LruMap<String, HistoryAddress>>> = LazyLock::new(|| {
    Mutex::<LruMap<String, HistoryAddress>>::new(LruMap::<String, HistoryAddress>::new(
        ByLength::new(SHORTNAMES_CAPACITY),
    ))
});

#[derive(Clone)]
pub struct DirectoryVersion {
    /// The 'v[VERSION].SHORTNAME.www-dweb.au' part of a dweb URL (see dweb::web::name)
    web_name_string: String,
    /// Address of a History<trove::DirectoryTree> on Autonomi (saves lookup based on SHORTNAME.www-dweb.au)
    pub history_address: HistoryAddress,
    /// A version of 0 implies use most recent version (highest available)
    version: Option<u64>,
    /// Directory / website metadata
    pub directory_address: DirectoryAddress,
    /// Directory / website metadata
    pub directory_tree: Option<DirectoryTree>,

    #[feature("fixed-webnames")]
    is_fixed_webname: bool,
}

impl DirectoryVersion {
    pub fn new(
        web_name: &WebName,
        history_address: HistoryAddress,
        directory_address: DirectoryAddress,
        directory_tree: Option<DirectoryTree>,
    ) -> DirectoryVersion {
        DirectoryVersion {
            web_name_string: web_name.web_name_string.clone(),
            history_address,
            version: web_name.version,
            directory_address,
            directory_tree,

            #[feature("fixed-webnames")]
            is_fixed_webname: false,
        }
    }
}
