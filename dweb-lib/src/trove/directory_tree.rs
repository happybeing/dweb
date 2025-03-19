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
use std::collections::HashMap;
use std::path::PathBuf;

use bytes::Bytes;
use color_eyre::eyre::{eyre, Result};
use http::status::StatusCode;
use mime_guess;

use autonomi::client::data::DataAddress;
use autonomi::client::files::archive_public::PublicArchive;
use autonomi::client::files::Metadata as FileMetadata;
use autonomi::files::archive_public::ArchiveAddress;

use crate::client::DwebClient;
use crate::helpers::convert::str_to_data_address;
use crate::trove::{History, Trove};

// The Trove type for a DirectoryTree
const FILE_TREE_TYPE: &str = "ee383f084cffaab845617b1c43ffaee8b5c17e8fbbb3ad3d379c96b5b844f24e";

// Default favicon.ico file, fixed by content addressing
//
// Safe dotted blue cube
const ADDRESS_DEFAULT_FAVICON: &str =
    "35cae9297780dcc0fe2d328dd1dc8060ec352eb54cc8192faaf3aedd803c119d";
// Safe quill inkpot
// const ADDRESS_DEFAULT_FAVICON: &str =
//     "164ea083d71e6e756e81244840b9bb46bd6284ce3316af91acf018a62a1c2af7";
// Safe dotted black cube. Low resolution
// const ADDRESS_DEFAULT_FAVICON: &str =
//     "9fa52fc5027ed23cbb080b7f049cf1ec742606ed8fd2a8cf72219e17098114ac";

// Early Safepress icon, blue cube inside a cube. Nice resolution
// const ADDRESS_DEFAULT_FAVICON: &str = "";

/// Separator used in DirectoryTree::directory_map
pub const PATH_SEPARATOR: char = '/';

/// Manage settings as a JSON string
//
// This struct is used for two multiple groups of settings, under separate 'keys':
//
//      dweb            - settings that should only be written by dweb
//      app/<APPNAME>   - settings for third party apps. Advise that <APPNAME> should
//                        attempt to be unique (e.g. using a domain owned by the
//                        author such as com.yourdomain.yourapp)
//
// The dweb section will contain settings for defining website redirects, overrides
// for default index files etc.
//
// The app sections will contain settings forthird-party applications which are not
// needed by dweb, but may be used to change the behaviour of a client app when it
// accesses the DirectoryTree, or provide information about the client used to create or
// the DirectoryTree.
//
#[derive(Clone)]
pub struct JsonSettings {
    json_string: String,
    // TODO implement non-serialised holder for JSON query object
}

impl JsonSettings {
    pub fn new() -> JsonSettings {
        JsonSettings {
            json_string: String::from(""),
        }
    }
    // TODO implement parsing to/from JSON query object
    // TODO implement setting/getting values using a hierarchy of keys

    /// Reads a JSON website configuration and returns a JSON query object
    /// TODO replace return type with a JSON query object holding settings
    pub fn from_file(_dweb_settings: &PathBuf) -> Result<JsonSettings> {
        // TODO load_json_file()
        Ok(JsonSettings::new())
    }

    /// Reads a JSON website configuration and returns a JSON query object
    /// TODO replace return type with a JSON query object holding settings
    pub fn from_string(json_string: String) -> Result<JsonSettings> {
        // TODO parse the JSON
        Ok(JsonSettings { json_string })
    }
}

pub const DWEB_SETTINGS_PATH: &str = "/.dweb/dweb-settings.json";

/// A set of default settings for use with a website when dweb_settings is none
#[derive(Clone)]
pub struct DwebSettings {
    // Content of the DWEB_SETTINGS_FILE
    pub json_config: JsonSettings,

    // Settings read from json_config
    pub index_filenames: Vec<String>, // Acceptable default index filenames (e.g. 'index.html')
}

impl DwebSettings {
    // TODO implement parsing to/from JSON query object
    // TODO implement setting/getting values using a hierarchy of keys

    pub fn from_bytes(bytes: &Bytes) -> Result<DwebSettings> {
        match String::from_utf8(bytes.to_vec()) {
            Ok(string) => return Self::from_string(string),
            Err(e) => panic!("DwebSettings::from_bytes() - failed {e}"),
        };
    }

    pub fn from_string(string: String) -> Result<DwebSettings> {
        // TODO
        let json_config = JsonSettings::from_string(string)?;

        Ok(DwebSettings {
            json_config,
            index_filenames: Vec::from([String::from("index.html"), String::from("index.htm")]),
        })
    }

    pub fn default() -> DwebSettings {
        DwebSettings {
            index_filenames: Vec::from([String::from("index.html"), String::from("index.htm")]),
            json_config: JsonSettings::new(),
        }
    }

    /// Reads a JSON website configuration and returns a JSON query object
    /// TODO replace return type with a JSON query object holding settings
    pub fn load_json_file(_dweb_settings: &PathBuf) -> Result<DwebSettings> {
        // TODO load_json_file()
        Ok(DwebSettings::default())
    }
}

/// DirectoryTree is a directory tree of files stored on Autonomi. It supports
/// optional metadata for a website which is stored in the Autonomi Archive
/// as a special file.
///
/// See also, History<DirectoryTree> which provides a persistent history of all versions
/// of the tree or website which have been stored.
#[derive(Clone)]
pub struct DirectoryTree {
    // We use a different map structure than Archive here but use an Autonomi PublicArchive
    // for serialisation in order to be compatible with other apps using that to store files.
    //
    /// Map using directory as key:
    pub directory_map: DirectoryTreePathMap,

    /// Keep a copy of the archive for serialisation.
    pub archive: PublicArchive,

    /// Optional settings for dweb or third party apps. These are stored as a JSON formatted
    /// file in the Archive, updated whenever the DirectoryTree is stored on or retrieved
    /// from the network.
    //
    // TODO document usage of dweb_settings JSON for metadata created by and accessible
    // TODO to unknown applications such as a website builder or the dweb CLI. Mandate that:
    // TODO   Only a single application unique key per application be stored at the top level
    // TODO   and that applications only create values under their own key.
    // TODO   In the default provide "awe" as a top level key with no sub-values
    pub dweb_settings: DwebSettings,
}

impl Trove<DirectoryTree> for DirectoryTree {
    fn trove_type() -> DataAddress {
        DataAddress::from_hex(FILE_TREE_TYPE).unwrap() // An error here is a bug that should be fixed
    }

    fn to_bytes(directory_tree: &DirectoryTree) -> Result<Bytes> {
        match directory_tree.archive.to_bytes() {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(eyre!("Failed to serialise DirectoryTree::archive - {e}")),
        }
    }

    async fn from_bytes(client: &DwebClient, bytes: Bytes) -> Result<DirectoryTree> {
        match PublicArchive::from_bytes(bytes) {
            Ok(archive) => Ok(DirectoryTree::from_archive(client, archive).await),
            Err(e) => Err(eyre!("Failed to serialise DirectoryTree::archive - {e}")),
        }
    }
}

// TODO consider how to use DirectoryTree for a virtual file store:
// TODO - currently it is given a file tree to upload in one operation, based on a local path
// TODO A virtual file store would:
// TODO - have a std::fs style interface which return fs style error codes
// TODO - track what is uploaded and not
// TODO - have methods to upload / get files, subtrees / the whole tree
/// Work in progress and subject to breaking changes
/// TODO consider how to handle use as a virtual file store (see comments above this in the code)
impl DirectoryTree {
    // pub fn new(website_settings: Option<DwebSettings>) -> DirectoryTree {
    //     DirectoryTree {
    //         archive: &PublicArchive::new(),
    //         directory_map: DirectoryTreePathMap::new(),
    //         dweb_settings: website_settings,
    //     }
    // }

    /// Get an archive from the network and use it to create a new DirectoryTree
    // TODO was directory_tree_download()
    pub async fn from_archive_address(
        client: &DwebClient,
        archive_address: ArchiveAddress,
    ) -> Result<DirectoryTree> {
        println!(
            "DEBUG directory_tree_download() at {}",
            archive_address.to_hex()
        );
        match client.client.archive_get_public(&archive_address).await {
            Ok(archive) => {
                println!(
                    "DEBUG Retrieved Public Archive of {} files",
                    archive.files().len()
                );
                let mut directory_tree = Self::from_archive_raw(archive);
                directory_tree.update_dweb_settings(client).await;
                Ok(directory_tree)
            }

            Err(e) => {
                println!("FAILED: {e}");
                Err(e.into())
            }
        }
    }

    // Initialise with data from the archive without accessing the network for DwebSettings
    pub async fn from_archive(client: &DwebClient, archive: PublicArchive) -> DirectoryTree {
        let mut directory_tree = Self::from_archive_raw(archive);
        directory_tree.update_dweb_settings(client).await;
        directory_tree
    }

    // Initialise with data from the archive without accessing the network for DwebSettings
    fn from_archive_raw(archive: PublicArchive) -> DirectoryTree {
        DirectoryTree {
            directory_map: DirectoryTreePathMap::from_public_archive(&archive),
            archive,
            dweb_settings: DwebSettings::default(),
        }
    }

    // Update DwebSettings if present in the archive
    // Return true if settings were updated
    async fn update_dweb_settings(&mut self, client: &DwebClient) -> bool {
        // Initialise dweb settings
        let dweb_settings_path = PathBuf::from(DWEB_SETTINGS_PATH);
        if let Some((settings_address, _metadata)) = self.archive.map().get(&dweb_settings_path) {
            if let Ok(bytes) = client.client.data_get_public(settings_address).await {
                if let Ok(parsed_settings) = DwebSettings::from_bytes(&bytes) {
                    self.dweb_settings = parsed_settings;
                    return true;
                };
            }
        }
        false
    }

    /// Looks up the web resource in a version of a History
    /// First gets a DirectoryTree version, using cached data if held by the history
    /// If version is None attempts obtain the default (most recent version)
    /// Returns a tuple with the address of the resource and optional content type if it can be determined
    pub async fn history_lookup_web_resource(
        history: &mut History<DirectoryTree>,
        resource_path: &String,
        version: Option<u32>,
    ) -> Result<(DataAddress, Option<String>), StatusCode> {
        if !history.fetch_version_trove(version).await.is_none() {
            if history.cached_version.is_some()
                && history.cached_version.as_ref().unwrap().trove.is_some()
            {
                let cached_version = history.cached_version.as_ref().unwrap();
                let directory = cached_version.trove.as_ref().unwrap();
                return directory.lookup_web_resource(resource_path);
            } else {
                println!("Failed to fetch directory.");
            }
        }
        Err(StatusCode::NOT_FOUND)
    }

    /// Look up a canonicalised web resource path (which must begin with '/').
    /// If the path ends with '/' or no file matches a directory is assumed.
    /// For directories it will look for a default index file based on any dweb settings.
    /// If found returns a tuple with the resource's xor address if found and content type if known
    /// On error, returns a suitable status code??? TODO
    pub fn lookup_web_resource(
        &self,
        resource_path: &String,
    ) -> Result<(DataAddress, Option<String>), StatusCode> {
        let last_separator_result = resource_path.rfind(PATH_SEPARATOR);
        if last_separator_result.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let original_resource_path = resource_path.clone();
        let mut resource_path = resource_path.clone();
        let last_separator = last_separator_result.unwrap();
        println!("Splitting path '{}'", resource_path);
        let second_part = resource_path.split_off(last_separator + 1);
        println!("...into '{}' and '{}'", resource_path, second_part);

        println!("Looking for resource at '{resource_path}'");
        let mut path_and_address = None;
        if let Some(resources) = self.directory_map.paths_to_files_map.get(&resource_path) {
            if second_part.len() > 0 {
                println!("DEBUG DirectoryTree looking up '{}'", second_part);
                match Self::lookup_name_in_vec(&second_part, resources) {
                    Some(data_address) => path_and_address = Some((second_part, data_address)),
                    None => {}
                }
            }
        };

        if path_and_address.is_none() {
            // Assume the second part is a directory name, so remake the path for that
            let new_resource_path = if original_resource_path.ends_with(PATH_SEPARATOR) {
                original_resource_path.clone()
            } else {
                original_resource_path.clone() + PATH_SEPARATOR.to_string().as_str()
            };

            println!("Retrying for index file in new_resource_path '{new_resource_path}'");
            let index_filenames = &self.dweb_settings.index_filenames;
            if let Some(new_resources) = self
                .directory_map
                .paths_to_files_map
                .get(&new_resource_path)
            {
                println!("DEBUG looking for a default INDEX file, one of {index_filenames:?}",);
                // Look for a default index file
                for index_file in index_filenames {
                    // TODO might it be necessary to return the name of the resource?
                    match Self::lookup_name_in_vec(&index_file, &new_resources) {
                        Some(xorname) => path_and_address = Some((index_file.clone(), xorname)),
                        None => {}
                    };
                }
            };
        };

        match path_and_address {
            Some((path, address)) => {
                let content_type = match mime_guess::from_path(path).first_raw() {
                    Some(str) => Some(str.to_string()),
                    None => None,
                };
                Ok((address, content_type))
            }
            None => {
                println!("FAILED to find resource for path: '{original_resource_path}' in:");
                println!("{:?}", self.directory_map.paths_to_files_map);
                if original_resource_path == "/favicon.ico" {
                    if let Ok(address) = str_to_data_address(ADDRESS_DEFAULT_FAVICON) {
                        return Ok((address, None));
                    }
                }
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    fn lookup_name_in_vec(
        name: &String,
        resources_vec: &Vec<(String, DataAddress, FileMetadata)>,
    ) -> Option<DataAddress> {
        println!("DEBUG lookup_name_in_vec({name})");
        for (resource_name, xor_name, _metadata) in resources_vec {
            if resource_name.eq(name) {
                return Some(xor_name.clone());
            }
        }
        None
    }
}

/// Map each directory path to a vector of the metadata for each file in the directory
/// The metadata tuple for a file is:
///   (filename: String, archive_address: XorName, metadata: FileMetadata)
#[derive(Clone)]
pub struct DirectoryTreePathMap {
    pub paths_to_files_map: HashMap<String, Vec<(String, DataAddress, FileMetadata)>>,
}

// TODO replace OS path separator with '/' when storing web paths
// TODO canonicalise path strings when adding them
impl DirectoryTreePathMap {
    pub fn new() -> DirectoryTreePathMap {
        DirectoryTreePathMap {
            paths_to_files_map: HashMap::<String, Vec<(String, DataAddress, FileMetadata)>>::new(),
        }
    }

    pub fn from_public_archive(archive: &PublicArchive) -> DirectoryTreePathMap {
        let mut path_map = DirectoryTreePathMap::new();
        let mut iter = archive.map().iter();
        while let Some((path_buf, (xor_name, metadata))) = iter.next() {
            // Remove the containing directory to produce a path from the website root, and which starts with '/'
            let mut path_string = String::from(path_buf.to_string_lossy());
            let offset = path_string.find("/").unwrap_or(path_string.len());
            path_string.replace_range(..offset, "");
            match path_map.add_content_to_map(&path_string, *xor_name, metadata.clone()) {
                Ok(_) => (),
                Err(e) => {
                    println!("add_content_to_metadata() failed to add path {path_string} - {e}");
                }
            }
        }
        path_map
    }

    /// Add a website resource to the metadata map
    /// resource_website_path MUST begin with a path separator denoting the website root
    /// This method handles translation of path separators
    pub fn add_content_to_map(
        &mut self,
        resource_website_path: &String,
        xor_name: DataAddress,
        metadata: FileMetadata,
    ) -> Result<()> {
        // println!("DEBUG add_content_to_map() path '{resource_website_path}'");
        let mut web_path = Self::webify_string(&resource_website_path);
        if let Some(last_separator_position) = web_path.rfind(PATH_SEPARATOR) {
            let resource_file_name = web_path.split_off(last_separator_position + 1);
            // println!(
            //     "DEBUG Splitting at {last_separator_position} into path: '{web_path}' file: '{resource_file_name}'"
            // );
            let entry_tuple = (resource_file_name.clone(), xor_name, metadata);
            self.paths_to_files_map
                .entry(web_path)
                .and_modify(|vector| vector.push(entry_tuple.clone()))
                .or_insert(vec![entry_tuple]);
        } else {
            return Err(eyre!(
                "Path separator not found in resource website path: {resource_website_path}"
            ));
        }

        Ok(())
    }

    // Replace OS path separators with '/'
    // fn webify_path(path: &Path) -> String {
    //     match path.to_str() {
    //         Some(path_string) => {
    //             return Self::webify_string(&path_string.to_string());
    //         }
    //         None => {}
    //     }

    //     String::from("")
    // }

    // Replace OS path separators with '/'
    fn webify_string(path_string: &String) -> String {
        let path_string = path_string.clone();
        return path_string.replace(std::path::MAIN_SEPARATOR_STR, "/");
    }
}

pub fn osstr_to_string(file_name: &std::ffi::OsStr) -> Option<String> {
    if let Some(str) = file_name.to_str() {
        return Some(String::from(str));
    }
    None
}

// Helper which gets a directory version and looks up a web resource.
// Returns a tuple of the the resource address and content type string if known
// pub async fn lookup_resource_for_website_version(
//     client: &DwebClient,
//     resource_path: &String,
//     history_address: HistoryAddress,
//     version: Option<u32>,
// ) -> Result<(DataAddress, Option<String>), StatusCode> {
//     println!("DEBUG lookup_resource_for_website_version() version {version:?}");
//     println!("DEBUG history_address: {}", history_address.to_hex());
//     println!("DEBUG resource_path    : {resource_path}");

//     match History::<DirectoryTree>::from_history_address(client.clone(), history_address).await {
//         Ok(mut history) => {
//             return DirectoryTree::history_lookup_web_resource(
//                 &mut history,
//                 resource_path,
//                 version,
//             )
//             .await;
//         }
//         Err(e) => {
//             println!("Failed to load versions register: {e:?}");
//             return Err(StatusCode::NOT_FOUND);
//         }
//     };
// }
