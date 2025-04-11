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

use autonomi::chunk::DataMapChunk;
use autonomi::client::data::DataAddress;
use autonomi::client::files::archive_private::PrivateArchive;
use autonomi::client::files::archive_public::PublicArchive;
use autonomi::client::files::Metadata as FileMetadata;
use autonomi::files::archive_public::ArchiveAddress;

use crate::client::DwebClient;
use crate::files::archive::DualArchive;
use crate::storage::DwebType;
use crate::trove::{History, Trove};

// The Trove type for a Tree
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

/// Separator used in PublicArchive/PrivateArchive and Tree::directory_map
pub const ARCHIVE_PATH_SEPARATOR: char = '/';

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
// accesses the Tree, or provide information about the client used to create or
// the Tree.
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

/// A JSON file used for dweb settings for website and other apps
pub const DWEB_SETTINGS_PATH: &str = "/.dweb/dweb-settings.json";

/// Archives which are part of a History can indicate the address by including
/// a file whose name is the history address at this locaion using DWEB_HISTORY_CONTENT_ADDRESS
/// as the address to avoid needing to load any content
///
/// The directory which holds a single file whose name is the address of a History of this directory
pub const DWEB_HISTORY_DIRECTORY: &str = "/.dweb/history-address";
/// The address of existing content that can be re-used to avoid uploading any content for the History address file
pub const DWEB_DIRECTORY_HISTORY_CONTENT: &str =
    "0a2768c3ebbb3651cfb4219222ddef9feafd485e07ed9cf1b27b8f97afa4595d";
pub const DWEB_DIRECTORY_HISTORY_DATAMAPCHUNK: &str =
    "81a54669727374939400dc00204f71ccc3ccdb7bcc95ccbaccd80a6eccd6125a2dccab2acc9657ccb5ccd9ccf518ccc4ccc455ccb311cc96ccd3ccf6cca139dc0020ccdbccf2cce1cccdccd03bccfccce4ccbe563220cca74a7fcca6cca13c2f21cc8e0248ccbaccc967cc87ccabcce65ecc99ccdcce000123179401dc0020cc8064cc8112cc9fcca6ccbcccf5151e4a4ccc87ccfc6d455650cc980dcc9a11cc8c7ccc83cced01cc90ccdbccc8cca341dc0020cc891d5677cc8267ccd7ccf6ccaf7acccb75ccde01ccb3cc81cc88cc82cc8c354f135cccd9cc836b48cc85ccf601ccebcce6ce000123179402dc00206accd622cccf50ccf5ccb4ccbccca738027bccd1ccceccc95f222173020921ccb000cc82ccd43a1ecc80cce2cce3ccfedc0020386cccc344360619157d6541ccd2ccbf7dcc866ecca812ccc424cc841c43cc81ccf1cc8563ccab353bccc3ccf4ce00012318";

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

/// Tree is a directory tree of files stored on Autonomi. It supports
/// optional metadata for a website which is stored in the Autonomi Archive
/// as a special file.
///
/// See also, History<Tree> which provides a persistent history of all versions
/// of the tree or website which have been stored.
#[derive(Clone)]
pub struct Tree {
    // We use a different map structure than Archive here and can consume either Autonomi
    // PublicArchive or PrivateArchive in the form of a dweb::archive::DualArchive
    //
    // Serialisation is designed to be compatible with other apps using either PublicArchive
    // or PrivateArchive to store files. We prefer PrivateArchive even for public data
    // as explained in the docs for dweb::archive::DualArchive.
    //
    /// Map using directory as key:
    pub directory_map: TreePathMap,

    /// Holds the Autonomi archive type for serialisation
    pub archive: DualArchive,

    /// Optional settings for dweb or third party apps. These are stored as a JSON formatted
    /// file in the Archive, updated whenever the Tree is stored on or retrieved
    /// from the network.
    //
    // TODO document usage of dweb_settings JSON for metadata created by and accessible
    // TODO to unknown applications such as a website builder or the dweb CLI. Mandate that:
    // TODO   Only a single application unique key per application be stored at the top level
    // TODO   and that applications only create values under their own key.
    // TODO   In the default provide "awe" as a top level key with no sub-values
    pub dweb_settings: DwebSettings,
}

impl Trove<Tree> for Tree {
    fn trove_type() -> DataAddress {
        DataAddress::from_hex(FILE_TREE_TYPE).unwrap() // An error here is a bug that should be fixed
    }

    /// Serialise as PrivateArchive
    ///
    /// A PrivateArchive is used in preference to PublicArchive even for public data.
    /// See dweb::archive::DualArchive for an explanation.
    fn to_bytes(directory_tree: &Tree) -> Result<Bytes> {
        match directory_tree.archive.to_bytes_as_private() {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(eyre!("Failed to serialise Tree::archive - {e}")),
        }
    }

    async fn from_bytes(client: &DwebClient, bytes: Bytes) -> Result<Tree> {
        match DualArchive::from_bytes(bytes) {
            Ok(archive) => Ok(Tree::from_dual_archive(client, archive).await),
            Err(e) => Err(eyre!("Failed to serialise Tree::archive - {e}")),
        }
    }
}

// TODO consider how to use Tree for a virtual file store:
// TODO - currently it is given a file tree to upload in one operation, based on a local path
// TODO A virtual file store would:
// TODO - have a std::fs style interface which return fs style error codes
// TODO - track what is uploaded and not
// TODO - have methods to upload / get files, subtrees / the whole tree
/// Work in progress and subject to breaking changes
/// TODO consider how to handle use as a virtual file store (see comments above this in the code)
impl Tree {
    // pub fn new(website_settings: Option<DwebSettings>) -> Tree {
    //     Tree {
    //         archive: &PublicArchive::new(),
    //         directory_map: TreePathMap::new(),
    //         dweb_settings: website_settings,
    //     }
    // }

    /// Get an archive from the network and use it to create a new Tree
    // TODO was directory_tree_download()
    pub async fn from_archive_address(
        client: &DwebClient,
        archive_address: ArchiveAddress,
    ) -> Result<Tree> {
        println!(
            "DEBUG directory_tree_download() at {}",
            archive_address.to_hex()
        );
        match client.client.data_get_public(&archive_address).await {
            Ok(data) => match DualArchive::from_bytes(data) {
                Ok(dual_archive) => {
                    println!(
                        "DEBUG Retrieved {:?} of {} files",
                        dual_archive.dweb_type,
                        dual_archive.files().len()
                    );
                    let mut directory_tree = Self::from_dual_archive(client, dual_archive).await;
                    directory_tree.update_dweb_settings(client).await;
                    Ok(directory_tree)
                }
                Err(e) => {
                    let message = format!("DEBUG failed to deseralise archive - {e}");
                    println!("DEBUG {message}");
                    return Err(eyre!(message));
                }
            },
            Err(e) => {
                println!("FAILED to get archive data {e}");
                Err(e.into())
            }
        }
    }

    /// Initialise from an PublicArchive
    ///
    /// If the archive contains a DwebSettings file this will be read from the network
    pub async fn from_public_archive(
        client: &DwebClient,
        public_archive: PublicArchive,
    ) -> Tree {
        Self::from_dual_archive(
            client,
            DualArchive {
                public_archive,
                private_archive: PrivateArchive::new(),
                dweb_type: DwebType::PublicArchive,
            },
        )
        .await
    }

    /// Initialise from an PrivateArchive
    ///
    /// If the archive contains a DwebSettings file this will be read from the network
    pub async fn from_private_archive(
        client: &DwebClient,
        private_archive: PrivateArchive,
    ) -> Tree {
        Self::from_dual_archive(
            client,
            DualArchive {
                public_archive: PublicArchive::new(),
                private_archive,
                dweb_type: DwebType::PrivateArchive,
            },
        )
        .await
    }

    /// Initialise from a DualArchive
    ///
    /// If the archive contains a DwebSettings file this will be read from the network
    ///
    /// Check the value of DualArchive.dweb_type to determine the type or archive. On failure it will be DwebType::Uknown
    pub async fn from_dual_archive(client: &DwebClient, archive: DualArchive) -> Tree {
        let dweb_type = archive.dweb_type;
        let mut directory_tree = match dweb_type {
            DwebType::PrivateArchive => Tree {
                directory_map: TreePathMap::from_private_archive(&archive.private_archive),
                archive,
                dweb_settings: DwebSettings::default(),
            },
            DwebType::PublicArchive => Tree {
                directory_map: TreePathMap::from_public_archive(&archive.public_archive),
                archive,
                dweb_settings: DwebSettings::default(),
            },
            _ => {
                let message =
                    format!("Tree cannot initialise using unknown DualArchive.dweb_type");
                println!("DEBUG {message}");
                Tree {
                    directory_map: TreePathMap::new(),
                    archive,
                    dweb_settings: DwebSettings::default(),
                }
            }
        };
        directory_tree.update_dweb_settings(client).await;
        println!("DEBUG Tree initialised using {dweb_type:?}",);
        directory_tree
    }

    // Update DwebSettings if present in the archive
    // Return true if settings were updated
    // TODOxxx update to handle public/private archive getting using address or datamap
    async fn update_dweb_settings(&mut self, client: &DwebClient) -> bool {
        // Initialise dweb settings
        let dweb_settings_path = PathBuf::from(DWEB_SETTINGS_PATH);
        if let Some((datamap_chunk, data_address, _metadata)) =
            self.archive.lookup_file(&dweb_settings_path)
        {
            if let Ok(bytes) = get_content(client, datamap_chunk, data_address).await {
                if let Ok(parsed_settings) = DwebSettings::from_bytes(&bytes) {
                    self.dweb_settings = parsed_settings;
                    return true;
                };
            }
        }
        false
    }

    /// Looks up a file or website resource in a version of a History
    ///
    /// First gets a Tree version, using cached data if held by the history
    /// If version is None attempts obtain the default (most recent version)
    ///
    /// as_website controls special handling for a website. See `lookup_file()`
    ///
    /// If found, returns a tuple: (datamap_chunk: String, data_address: String, content_type: Option<String>)
    ///
    /// Only one of datamap_chunk and data_address will be a hex encoded string, the other empty.
    ///
    // TODOxxx update to handle public/private archive getting using address or datamap
    pub async fn history_lookup_file(
        history: &mut History<Tree>,
        resource_path: &String,
        as_website: bool,
        version: Option<u32>,
    ) -> Result<(String, String, Option<String>), StatusCode> {
        if !history.fetch_version_trove(version).await.is_none() {
            if history.cached_version.is_some()
                && history.cached_version.as_ref().unwrap().trove.is_some()
            {
                let cached_version = history.cached_version.as_ref().unwrap();
                let directory = cached_version.trove.as_ref().unwrap();
                return directory.lookup_file(resource_path, as_website);
            } else {
                println!("Failed to fetch directory.");
            }
        }
        Err(StatusCode::NOT_FOUND)
    }

    /// Look up a canonicalised file path (which must begin with '/') with special handling for websites.
    ///
    /// If the path ends with '/' or no file matches a directory is assumed and handling
    /// depends on the value of as_website.
    ///
    /// if as_website is false the path must be an exact match for a file.
    ///
    /// If as_website is true, an exact path match is not always required. For directories it will
    /// look for a default index file based on any dweb settings. It will also return a
    /// default for '/faviocon.ico' if not matched.
    ///
    /// If found, returns a tuple: (datamap_chunk: String, data_address: String, content_type: Option<String>)
    ///
    /// Only one of datamap_chunk and data_address will be a hex encoded string, the other empty.
    ///
    /// On error, returns a suitable status code??? TODO
    pub fn lookup_file(
        &self,
        resource_path: &String,
        as_website: bool,
    ) -> Result<(String, String, Option<String>), StatusCode> {
        let last_separator_result = resource_path.rfind(ARCHIVE_PATH_SEPARATOR);
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
                println!("DEBUG Tree looking up '{}'", second_part);
                match Self::lookup_name_in_vec(&second_part, resources) {
                    Some((datamap_chunk, data_address)) => {
                        path_and_address = Some((second_part, datamap_chunk, data_address))
                    }
                    None => {}
                }
            }
        };

        // For a wesbite directory, look for an index file
        if as_website && path_and_address.is_none() {
            // Assume the second part is a directory name, so remake the path for that
            let new_resource_path = if original_resource_path.ends_with(ARCHIVE_PATH_SEPARATOR) {
                original_resource_path.clone()
            } else {
                original_resource_path.clone() + ARCHIVE_PATH_SEPARATOR.to_string().as_str()
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
                        Some((datamap_chunk, data_address)) => {
                            path_and_address =
                                Some((index_file.clone(), datamap_chunk, data_address))
                        }
                        None => {}
                    };
                }
            };
        };

        match path_and_address {
            Some((path, datamap_chunk, data_address)) => {
                let content_type = match mime_guess::from_path(path).first_raw() {
                    Some(str) => Some(str.to_string()),
                    None => None,
                };
                Ok((datamap_chunk, data_address, content_type))
            }
            None => {
                println!("FAILED to find resource for path: '{original_resource_path}' in:");
                println!("{:?}", self.directory_map.paths_to_files_map);
                if as_website && original_resource_path == "/favicon.ico" {
                    return Ok((
                        "".to_string(),
                        ADDRESS_DEFAULT_FAVICON.to_string(),
                        None::<String>,
                    ));
                }
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    /// Lookup a filename and return a xxx
    // TODOxxx update to handle public/private archive getting using address or datamap
    fn lookup_name_in_vec(
        name: &String,
        resources_vec: &Vec<(String, String, String, FileMetadata)>,
    ) -> Option<(String, String)> {
        println!("DEBUG lookup_name_in_vec({name})");
        for (resource_name, datamap_chunk, data_address, _metadata) in resources_vec {
            if resource_name.eq(name) {
                return Some((datamap_chunk.clone(), data_address.clone()));
            }
        }
        None
    }
}

/// Map each directory path to a vector of the metadata for each file in the directory
/// The metadata tuple for a file is:
///   (filename: String, datamap_chunk: String, data_adddress: String, metadata: FileMetadata)
#[derive(Clone)]
pub struct TreePathMap {
    pub paths_to_files_map: HashMap<String, Vec<(String, String, String, FileMetadata)>>,
}

// TODO replace OS path separator with '/' when storing web paths
// TODO canonicalise path strings when adding them
impl TreePathMap {
    pub fn new() -> TreePathMap {
        TreePathMap {
            paths_to_files_map: HashMap::<String, Vec<(String, String, String, FileMetadata)>>::new(
            ),
        }
    }

    pub fn from_public_archive(archive: &PublicArchive) -> TreePathMap {
        let mut path_map = TreePathMap::new();
        let mut iter = archive.map().iter();
        while let Some((path_buf, (data_address, metadata))) = iter.next() {
            // Remove the containing directory to produce a path from the website root, and which starts with '/'
            let mut path_string = String::from(path_buf.to_string_lossy());
            let offset = path_string.find("/").unwrap_or(path_string.len());
            path_string.replace_range(..offset, "");
            match path_map.add_content_to_map(
                &path_string,
                "".to_string(),
                data_address.to_hex(),
                metadata.clone(),
            ) {
                Ok(_) => (),
                Err(e) => {
                    println!("add_content_to_metadata() failed to add path {path_string} - {e}");
                }
            }
        }
        path_map
    }

    pub fn from_private_archive(archive: &PrivateArchive) -> TreePathMap {
        let mut path_map = TreePathMap::new();
        let mut iter = archive.map().iter();
        while let Some((path_buf, (datamap, metadata))) = iter.next() {
            // Remove the containing directory to produce a path from the website root, and which starts with '/'
            let mut path_string = String::from(path_buf.to_string_lossy());
            let offset = path_string.find("/").unwrap_or(path_string.len());
            path_string.replace_range(..offset, "");
            match path_map.add_content_to_map(
                &path_string,
                datamap.to_hex(),
                "".to_string(),
                metadata.clone(),
            ) {
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
        datamap_chunk: String,
        data_address: String,
        metadata: FileMetadata,
    ) -> Result<()> {
        // println!("DEBUG add_content_to_map() path '{resource_website_path}'");
        let mut web_path = Self::webify_string(&resource_website_path);
        if let Some(last_separator_position) = web_path.rfind(ARCHIVE_PATH_SEPARATOR) {
            let resource_file_name = web_path.split_off(last_separator_position + 1);
            // println!(
            //     "DEBUG Splitting at {last_separator_position} into path: '{web_path}' file: '{resource_file_name}'"
            // );
            let entry_tuple = (
                resource_file_name.clone(),
                datamap_chunk,
                data_address,
                metadata,
            );
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
    pub fn webify_string(path_string: &String) -> String {
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

/// Get content from the network using a hex encoded datamap if provided, otherwise a hex encoded address
pub async fn get_content(
    client: &DwebClient,
    datamap_chunk: String,
    data_address: String,
) -> Result<Bytes> {
    let datamap_chunk = if !datamap_chunk.is_empty() {
        match DataMapChunk::from_hex(&datamap_chunk) {
            Ok(datamap_chunk) => Some(datamap_chunk),
            Err(_) => None,
        }
    } else {
        None
    };

    let data_address = if !data_address.is_empty() {
        match DataAddress::from_hex(&data_address) {
            Ok(data_address) => Some(data_address),
            Err(_) => None,
        }
    } else {
        None
    };

    let autonomi_result = match datamap_chunk.clone() {
        Some(datamap_chunk) => {
            println!(
                "DEBUG get_content() calling data_get() with datamap_chunk: {}",
                datamap_chunk.to_hex()
            );
            client.client.data_get(&datamap_chunk).await
        }
        None => match data_address {
            Some(data_address) => {
                println!(
                    "DEBUG get_content() calling data_get_public() with data_address: {}",
                    data_address.to_hex()
                );
                client.client.data_get_public(&data_address).await
            }
            None => {
                return Err(eyre!(
                "DEBUG get_content() failed to decode data_address: '{:?}' and datamap_chunk: '{:?}'",
                data_address,
                datamap_chunk,
            ))
            }
        },
    };

    match autonomi_result {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            let message = format!("get_content() failed to access data from network using data_address: '{:?}' and datamap_chunk: '{:?}' - {e}",
            data_address,
            datamap_chunk,
                );
            println!("DEBUG {message}");
            Err(eyre!(message))
        }
    }
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

//     match History::<Tree>::from_history_address(client.clone(), history_address).await {
//         Ok(mut history) => {
//             return Tree::history_lookup_file(
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
