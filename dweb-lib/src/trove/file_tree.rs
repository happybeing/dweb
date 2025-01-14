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

use ant_registers::RegisterAddress;
use bytes::{BufMut, BytesMut};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result};
use http::status::StatusCode;
use mime_guess;
use serde::{Deserialize, Serialize};
use xor_name::XorName as FileAddress;

use autonomi::client::payment::PaymentOption;
use autonomi::client::Client;
use autonomi::Wallet;
use self_encryption::MAX_CHUNK_SIZE;

use crate::client::AutonomiClient;
use crate::data::autonomi_get_file_public;
use crate::trove::{Trove, TroveHistory};

// The Trove type for a FileTree
const FILE_TREE_TYPE: &str = "ee383f084cffaab845617b1c43ffaee8b5c17e8fbbb3ad3d379c96b5b844f24e";

/// Separator used in FileTree.path_map
pub const PATH_SEPARATOR: char = '/';

/// Manage settings as a JSON string in order to ensure serialisation and deserialisation
/// of FileTree succeeds even as different settings are added or removed.
//
// This struct is used for two separate groups of settings. The first configure the
// website by defining redirects, overrides for default index files etc. The second is
// for awe and third-party application settings which are not needed in order to
// display the website itself, but may be used to change the behaviour of a client
// app when it accesses the website, or provide information about the client used
// to create or publish the site.
#[derive(Serialize, Deserialize, Clone)]
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
    pub fn load_json_file(_website_config: &PathBuf) -> Result<JsonSettings> {
        // TODO load_json_file()
        Ok(JsonSettings::new())
    }
}

/// WIP: FileTree is a work in progress and subject to breaking changes
/// Container for a directory tree of files stored on Autonomi. Includes metadata needed
/// to store and view a website using the dweb CLI.
/// Used by FileTreeHistory to provide a persistent history of all versions of the tree or website.
#[derive(Serialize, Deserialize, Clone)]
pub struct FileTree {
    /// System time of the device used, when publishing this FileTree to Autonomi
    pub date_published: DateTime<Utc>,
    /// Map of paths to directory and file metadata
    pub path_map: FileTreePathMap,
    // TODO document usage of third_party_settings JSON for metadata created by and accessible
    // TODO to unknown applications such as a website builder or the dweb CLI. Mandate that:
    // TODO   Only a single application unique key per application be stored at the top level
    // TODO   and that applications only create values under their own key.
    // TODO   In the default provide "awe" as a top level key with no sub-values
    /// Optionally used by an app to store arbitrary app specific settings or metadata.
    pub third_party_settings: JsonSettings,

    // Optional settings when a FileTree is used to store a website
    website_settings: Option<WebsiteSettings>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WebsiteSettings {
    // TODO use website_config to implement web server like configuration such as redirects, short links and subdomains
    pub website_config: JsonSettings,
    pub index_filenames: Vec<String>, // Acceptable default index filenames (e.g. 'index.html')
}

impl WebsiteSettings {
    pub fn new() -> WebsiteSettings {
        WebsiteSettings {
            index_filenames: Vec::from([String::from("index.html"), String::from("index.htm")]),
            website_config: JsonSettings::new(),
        }
    }
}

impl Trove for FileTree {
    fn trove_type() -> FileAddress {
        FileAddress::from_content(FILE_TREE_TYPE.as_bytes())
    }
}

// TODO consider how to handle use as a virtual file store:
// TODO - currently it is given a file tree to upload in one operation, based on a local path
// TODO A virtual file store would:
// TODO - have a std::fs style interface which return fs style error codes
// TODO - track what is uploaded and not
// TODO - have methods to upload / get files, subtrees / the whole tree
/// Work in progress and subject to breaking changes
/// TODO consider how to handle use as a virtual file store (see comments above this in the code)
impl FileTree {
    pub fn new(website_settings: Option<WebsiteSettings>) -> FileTree {
        FileTree {
            date_published: Utc::now(),
            third_party_settings: JsonSettings::new(),
            path_map: FileTreePathMap::new(),
            website_settings,
        }
    }

    pub async fn file_tree_download(
        client: &AutonomiClient,
        data_address: FileAddress,
    ) -> Result<FileTree> {
        println!("DEBUG file_tree_download() at {data_address:64x}");
        match autonomi_get_file_public(client, &data_address).await {
            Ok(content) => {
                println!("Retrieved {} bytes", content.len());
                let metadata: FileTree = rmp_serde::from_slice(&content)?;
                Ok(metadata)
            }

            Err(e) => {
                println!("FAILED: {e}");
                Err(e.into())
            }
        }
    }

    /// Looks up the web resource in a version of a TroveHistory
    /// First gets a FileTree version, using cached data if held by the history
    /// If version is None attempts obtain the default (most recent version)
    /// Returns a tuple with the address of the resource and optional content type if it can be determined
    pub async fn history_lookup_web_resource(
        history: &mut TroveHistory<FileTree>,
        resource_path: &String,
        version: Option<u64>,
    ) -> Result<(FileAddress, Option<String>), StatusCode> {
        if !history.fetch_version_metadata(version).await.is_none() {
            if history.cached_version.is_some()
                && history.cached_version.as_ref().unwrap().metadata.is_some()
            {
                let cached_version = history.cached_version.as_ref().unwrap();
                let metadata = cached_version.metadata.as_ref().unwrap();
                return metadata.lookup_web_resource(resource_path);
            } else {
                println!("Failed to fetch metadata.");
            }
        }
        Err(StatusCode::NOT_FOUND)
    }

    /// Look up a canonicalised web resource path (which must begin with '/').
    /// If the path ends with '/' or no file matches a directory is assumed.
    /// For directories it will look for a default index file based on the site metadata settings.
    /// If found returns a tuple with the resource's xor address if found and content type if known
    /// On error, returns a suitable status code??? TODO
    pub fn lookup_web_resource(
        &self,
        resource_path: &String,
    ) -> Result<(FileAddress, Option<String>), StatusCode> {
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
        if let Some(resources) = self.path_map.paths_to_files_map.get(&resource_path) {
            if second_part.len() > 0 {
                println!("DEBUG FileTree looking up '{}'", second_part);
                match Self::lookup_name_in_vec(&second_part, &resources) {
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
            let index_filenames = if let Some(website_settings) = &self.website_settings {
                &website_settings.index_filenames
            } else {
                &Vec::from([String::from("index.html"), String::from("index.htm")])
            };

            if let Some(new_resources) = self.path_map.paths_to_files_map.get(&new_resource_path) {
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
                println!("{:?}", self.path_map.paths_to_files_map);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    fn lookup_name_in_vec(
        name: &String,
        resources_vec: &Vec<(String, FileAddress, std::time::SystemTime, u64, String)>,
    ) -> Option<FileAddress> {
        for (resource_name, xor_name, _modified, _size, _json_metadata) in resources_vec {
            if resource_name.eq(name) {
                return Some(xor_name.clone());
            }
        }
        None
    }

    /// Add a resource to the metadata map
    /// The resource_website_path MUST:
    /// - start with a forward slash denoting the website root
    /// - use forward slash to separate directories
    pub fn add_content_to_metadata(
        &mut self,
        resource_website_path: &String,
        xor_name: FileAddress,
        file_metadata: Option<&std::fs::Metadata>,
    ) -> Result<()> {
        self.path_map
            .add_content_to_metadata(resource_website_path, xor_name, file_metadata)
    }

    /// Upload content metadata and return the address (ie of the data map)
    pub async fn put_files_metadata_to_network(
        &self,
        client: AutonomiClient,
    ) -> Result<FileAddress> {
        let serialised_metadata = rmp_serde::to_vec(self)?;
        if serialised_metadata.len() > MAX_CHUNK_SIZE {
            return Err(eyre!(
                "Failed to store website metadata - too large for one Chunk",
            ));
        }

        let mut bytes = BytesMut::with_capacity(MAX_CHUNK_SIZE);
        bytes.put(serialised_metadata.as_slice());

        match client
            .client
            .data_put_public(bytes.freeze(), PaymentOption::from(client.wallet))
            .await
        {
            Ok(data_map) => Ok(data_map),
            Err(e) => {
                let message = format!("Failed to upload website metadata - {e}");
                println!("{}", &message);
                return Err(eyre!(message.clone()));
            }
        }
    }
}

/// A map of paths to files used to access xor addresses of content
#[derive(Serialize, Deserialize, Clone)]
pub struct FileTreePathMap {
    // Maps of paths of directories and files to metadata.
    // File metadata tuple is (filename, data_address, date modified, size, extended metadata)
    // where extended metadata can be a JSON encoded String at some point
    // TODO consider if using a BTree or other collection would make serialization deterministic
    pub paths_to_files_map:
        HashMap<String, Vec<(String, FileAddress, std::time::SystemTime, u64, String)>>,
}

// TODO replace OS path separator with '/' when storing web paths
// TODO canonicalise path strings when adding them
impl FileTreePathMap {
    pub fn new() -> FileTreePathMap {
        FileTreePathMap {
            paths_to_files_map: HashMap::<
                String,
                Vec<(String, FileAddress, std::time::SystemTime, u64, String)>,
            >::new(),
        }
    }

    /// Add a website resource to the metadata map
    /// resource_website_path MUST begin with a path separator denoting the website root
    /// This method handles translation of path separators
    pub fn add_content_to_metadata(
        &mut self,
        resource_website_path: &String,
        xor_name: FileAddress,
        file_metadata: Option<&std::fs::Metadata>,
    ) -> Result<()> {
        // println!("DEBUG add_content_to_metadata() path '{resource_website_path}'");
        let mut web_path = Self::webify_string(&resource_website_path);
        if let Some(last_separator_position) = web_path.rfind(PATH_SEPARATOR) {
            let resource_file_name = web_path.split_off(last_separator_position + 1);
            // println!(
            //     "DEBUG Splitting at {last_separator_position} into path: '{web_path}' file: '{resource_file_name}'"
            // );
            let metadata_tuple = if let Some(metadata) = file_metadata {
                (
                    // Use metadata for time and size
                    resource_file_name.clone(),
                    xor_name,
                    metadata.modified().unwrap(),
                    metadata.len(),
                    String::from(""), // Provision for future JSON encoded extended metadata (e.g. with MIME type)
                )
            } else {
                (
                    // Use system time and set unknown size (0)
                    resource_file_name.clone(),
                    xor_name,
                    std::time::SystemTime::now(),
                    0,
                    String::from(""), // Provision for future JSON encoded extended metadata (e.g. with MIME type)
                )
            };

            self.paths_to_files_map
                .entry(web_path)
                .and_modify(|vector| vector.push(metadata_tuple.clone()))
                .or_insert(vec![metadata_tuple]);
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

/// Helper which gets a website version and looks up a web resource.
/// Returns a tuple of the the resource address and content type string if known
pub async fn lookup_resource_for_website_version(
    client: &AutonomiClient,
    resource_path: &String,
    history_address: RegisterAddress,
    version: Option<u64>,
) -> Result<(FileAddress, Option<String>), StatusCode> {
    println!("DEBUG lookup_resource_for_website_version() version {version:?}");
    println!("DEBUG history_address: {history_address}");
    println!("DEBUG resource_path    : {resource_path}");

    match TroveHistory::<FileTree>::from_register_address(client.clone(), history_address, None)
        .await
    {
        Ok(mut history) => {
            return FileTree::history_lookup_web_resource(&mut history, resource_path, version)
                .await;
        }
        Err(e) => {
            println!("Failed to load versions register: {e:?}");
            return Err(StatusCode::NOT_FOUND);
        }
    };
}
