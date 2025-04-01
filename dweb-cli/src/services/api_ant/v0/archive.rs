/*
 Copyright (c) 2025- Mark Hughes

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

use std::collections::HashSet;
use std::path::PathBuf;

use actix_web::{
    get,
    http::{header::ContentType, StatusCode},
    web,
    web::Data,
    HttpRequest, HttpResponse,
};
use autonomi::files::PublicArchive;
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::client::data::DataAddress;
use autonomi::client::files::Metadata as FileMetadata;

use dweb::trove::History;
use dweb::{helpers::convert::*, helpers::web::*, trove::directory_tree::DirectoryTree};

use crate::services::helpers::*;

// TODO archive_put() for /put
// TODO remove /directory-load

/// Get a directory tree (PublicArchive)
///
/// Returns a DwebArchive schema containing metadata for files and directories stored in a PublicArchive
///
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of an Autonomi PublicArchive as a DwebArchive schema.", body = [DwebArchive])
        ),
    tags = ["Autonomi"],
    params(
        ("archive_address", description = "the network address of an Autonomi PublicArchive as a hexadecimal string"),
    )
)]
#[get("/archive/{archive_address}")]
pub async fn get(
    request: HttpRequest,
    archive_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let archive_address = match DataAddress::from_hex(&archive_address.into_inner()) {
        Ok(archive_address) => archive_address,
        Err(_e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive GET".to_string(),
                "/archive GET invalid archive_address parameter",
            )
        }
    };

    let public_archive = match client.client.archive_get_public(&archive_address).await {
        Ok(archive) => archive,
        Err(e) => {
            let message = format!("/archive GET archive_get_public() failed - {e}");
            println!("DEBUG {message}");
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive GET error".to_string(),
                &message,
            );
        }
    };

    let dweb_archive = DwebArchive::from_public_archive(&public_archive);
    let json = match serde_json::to_string(&dweb_archive) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                "/archive GET error".to_string(),
                &format!("archive GET failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG DwebArchive as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Get a versioned directory tree from a dweb History of Autonomi PublicArchive
///
/// Returns a DwebArchive schema containing metadata for files and directories stored in a PublicArchive
///
/// Path parameters refer to the required version and dweb History:
///
///     [v{VERSION-NUMBER}/]{ADDRESS-OR-NAME}
///
/// VERSION-NUMBER      Optional version when ADDRESS-OR-NAME refers to a <code>History<DirectoryTree></code>
///
/// ADDRESS-OR-NAME     A hexadecimal address or a short name referring to a History or PublicArchive
///
/// url: <code>http://127.0.0.1:8080/archive-version/[v<VERSION-NUMBER>/]<ADDRESS-OR-NAME></code>
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of an Autonomi PublicArchive as a DwebArchive schema.", body = [DwebArchive])
        ),
    tags = ["Autonomi"],
    // params(
    //     ("params" = String, Path, description = "Optional version (integer > 0) of a PublicArchive History"),
    //     ("ADDRESS-OR-NAME", description = "A hexadecimal address or a short name referring to a PublicArchive History"),
    // )
)]
#[get("/archive-version/{params:.*}")]
pub async fn get_version(
    request: HttpRequest,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG archive::get_version() {}", request.path());

    let params = params.into_inner();
    let decoded_params = match parse_versioned_path_params(&params) {
        Ok(params) => params,
        Err(_e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive-version GET".to_string(),
                "/archive-version GET invalid parameters",
            )
        }
    };

    let (version, _as_name, address_or_name, _remote_path) = decoded_params;
    let version = version.clone();
    let mut history_metadata = None;

    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            "/archive-version GET error".to_string(),
            &format!("/archive-version GET parameter error - unrecognised DWEB-NAME or invalid address: '{address_or_name}'"),
        );
    }

    let client = client.into_inner().as_ref().clone();
    let archive_address = if archive_address.is_some() {
        archive_address.unwrap()
    } else {
        let history_address = history_address.unwrap();
        let mut history = match History::<DirectoryTree>::from_history_address(
            client.clone(),
            history_address,
            false,
            0,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                let message = format!("/archive-version GET failed to get directory History - {e}");
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    "/archive-version GET error".to_string(),
                    &message,
                );
            }
        };

        let ignore_pointer = false;
        let version = version.unwrap_or(0);
        history_metadata = Some(DwebHistoryReference {
            version,
            history_address: history_address.to_hex(),
            history_size: history.num_entries() - 1,
        });

        match history
            .get_version_entry_value(version, ignore_pointer)
            .await
        {
            Ok(archive_address) => archive_address,
            Err(e) => {
                let message = format!("/archive-version GET invalid parameters - {e}");
                return make_error_response_page(
                    None,
                    &mut HttpResponse::BadRequest(),
                    "/archive-version GET error".to_string(),
                    &message,
                );
            }
        }
    };

    let public_archive = match client.client.archive_get_public(&archive_address).await {
        Ok(archive) => archive,
        Err(e) => {
            let message = format!("/archive-version GET archive_get_public() failed - {e}");
            println!("DEBUG {message}");
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive-version GET error".to_string(),
                &message,
            );
        }
    };

    let mut dweb_archive = DwebArchive::from_public_archive(&public_archive);
    dweb_archive.history_metadata = history_metadata;
    let json = match serde_json::to_string(&dweb_archive) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                "/archive-version GET error".to_string(),
                &format!("archive GET failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG DwebArchive as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Metadata about the History from which a DwebArchive was obtained
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DwebHistoryReference {
    /// The address in hexadecimal of the History from which the PublicArchive was retrieved
    history_address: String,
    /// The version entry of the retrieved PublicArchive. A version of 0 indicates the most recent version was obtained
    version: u32,
    /// The total number of versions when the History was accessed
    history_size: u32,
}

/// A representation of the Autonomi PublicArchive for web clients
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DwebArchive {
    /// File and directory entries represented in a PublicArchive. For PUT, directory entries are ignored so not required
    pub entries: Vec<DwebArchiveEntry>,
    /// Information about a History will only be present only when retrieved from a History using /archive-versions this
    pub history_metadata: Option<DwebHistoryReference>,
}

impl DwebArchive {
    pub fn new() -> DwebArchive {
        DwebArchive {
            history_metadata: None,
            entries: Vec::<DwebArchiveEntry>::new(),
        }
    }

    pub fn from_public_archive(archive: &PublicArchive) -> DwebArchive {
        let mut dweb_archive = DwebArchive::new();
        let mut directories_added = HashSet::<String>::new();
        let mut files_added = HashSet::<String>::new();

        let mut iter = archive.map().iter();
        while let Some((path_buf, (xor_name, metadata))) = iter.next() {
            // Remove the containing directory to produce a path from the website root, and which starts with '/'
            let mut path_string = String::from(path_buf.to_string_lossy());
            let offset = path_string.find("/").unwrap_or(path_string.len());
            path_string.replace_range(..offset, "");
            let mut web_path =
                dweb::trove::directory_tree::DirectoryTreePathMap::webify_string(&path_string);

            if let Some(last_separator_position) =
                web_path.rfind(dweb::trove::directory_tree::ARCHIVE_PATH_SEPARATOR)
            {
                let file_full_path = web_path.clone();
                let _file_name = web_path.split_off(last_separator_position + 1);
                // println!(
                //     "DEBUG Splitting at {last_separator_position} into path: '{web_path}' file: '{_file_name}'"
                // );

                if !directories_added.contains(&web_path) {
                    dweb_archive
                        .entries
                        .push(DwebArchiveEntry::new_directory(web_path.clone()));
                    directories_added.insert(web_path.clone());
                }

                if !files_added.contains(&file_full_path) {
                    dweb_archive.entries.push(DwebArchiveEntry::new_file(
                        file_full_path.clone(),
                        xor_name,
                        metadata,
                    ));
                    files_added.insert(file_full_path);
                }
            } else {
                println!(
                    "DEBUG Path separator not found in resource website path: {web_path} - this is probably a bug"
                );
            }
        }
        dweb_archive
    }

    pub fn to_public_archive(&self) -> Result<PublicArchive> {
        let mut archive = PublicArchive::new();

        for entry in &self.entries {
            match entry.dweb_type {
                DwebArchiveEntryType::File => {
                    let file_path = PathBuf::from(&entry.full_path);
                    let data_address = match DataAddress::from_hex(&entry.data_address) {
                        Ok(data_address) => data_address,
                        Err(e) => {
                            let message = format!(
                                "entry has invalid data address: {}, {e}",
                                entry.data_address
                            );
                            println!("DEBUG DEBUG DwebEntry::to_public_archive() - {message}");
                            return Err(eyre!(message));
                        }
                    };
                    let created = json_date_to_metadata_date(&entry.created).unwrap_or(0);
                    let modified = json_date_to_metadata_date(&entry.modified).unwrap_or(0);
                    let extra = if entry.extra.is_empty() {
                        None
                    } else {
                        Some(entry.extra.clone())
                    };

                    let metadata = FileMetadata {
                        created,
                        modified,
                        size: entry.size,
                        extra,
                    };

                    archive.add_file(file_path, data_address, metadata)
                }
                _ => {}
            }
        }

        Ok(archive)
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum DwebArchiveEntryType {
    File,
    Directory,
}

/// Metadata for each directory and file present in a DwebArchive
///
/// Notes:
///
/// - for directories, only essential metadata is stored because directories are not
/// first class objects in an Autonomi PublicArchive
///
/// - for files, only the full_path and data_address are required to assist with
/// anonimisation for improved privacy. However size should also be present as this
/// can always be obtained from the file itself.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DwebArchiveEntry {
    /// File or directory (required)
    pub dweb_type: DwebArchiveEntryType,
    /// The path of the directory or file from the root of the Archive (required). Must start with '/'
    pub full_path: String,
    /// Hexadecimal address of the datamap for a file (required)
    pub data_address: String,
    /// File creation date (optional) TODO define format
    pub created: String,
    /// File modification date (optional) TODO define format
    pub modified: String,
    /// File size in bytes (recommended)
    pub size: u64,
    /// Additional JSON format metadata for a file or directory
    pub extra: String,
}

impl DwebArchiveEntry {
    pub fn new_directory(full_path: String) -> DwebArchiveEntry {
        DwebArchiveEntry {
            dweb_type: DwebArchiveEntryType::Directory,
            full_path,
            data_address: "".to_string(),
            created: "".to_string(),
            modified: "".to_string(),
            size: 0,
            extra: "".to_string(),
        }
    }

    pub fn new_file(
        full_path: String,
        data_address: &DataAddress,
        metadata: &FileMetadata,
    ) -> DwebArchiveEntry {
        DwebArchiveEntry {
            dweb_type: DwebArchiveEntryType::File,
            full_path,
            data_address: data_address.to_hex(),
            created: metadata_date_to_json_datestring(metadata.created),
            modified: metadata_date_to_json_datestring(metadata.modified),
            size: metadata.size,
            extra: metadata.extra.clone().unwrap_or("".to_string()).clone(),
        }
    }
}

/// TODO archive_get() for /get
/// TODO archive_put() for /put
/// TODO decide how to handle the current /directory-load functionality (params and returned format<- leave for client to handle conversion to SVAR format)

/// Get the file metadata in a directory tree
///
/// Retrieves a PublicArchive from Autonomi and returns metadata for all files it contains.
///
/// Path parameters:
///
///     [v{version}/]{address_or_name}
///
// TODO Consider change this to be /ant-0/archive POST/PUT/GET and use a new struct DwebArchive which can be easily
// TODO converted in Rust to/from Archive (and in client to the format needed there). So will use Vec of struct
// TODO rather than map and will impl utoipa ToSchema
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of a DirectoryTree formatted for an SVAR file manager component.
            <p>Note: this may be changed to return a JSON representation of a DirectoryTree.", body = str)
        ),
    tags = ["Dweb"],
    params(
        ("params", description = "[v{version}/]{address_or_name}<br/><br/>Optional version (integer > 0), an address_or_name which refers to a History<DirectoryTree>"),
    )
)]
#[get("/directory-load/{params:.*}")]
pub async fn api_directory_load(
    request: HttpRequest,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let params = params.into_inner();
    let decoded_params = match parse_versioned_path_params(&params) {
        Ok(params) => params,
        Err(_e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/directory-load error".to_string(),
                "/directory-load invalid parameters",
            )
        }
    };

    let (version, _as_name, address_or_name, _remote_path) = decoded_params;
    let version = version.clone();

    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            "/directory-load error".to_string(),
            &format!("Unrecognised DWEB-NAME or invalid address: '{address_or_name}'"),
        );
    }

    let client = client.into_inner().as_ref().clone();
    let archive_address = if archive_address.is_some() {
        archive_address.unwrap()
    } else {
        let history_address = history_address.unwrap();
        let mut history = match History::<DirectoryTree>::from_history_address(
            client.clone(),
            history_address,
            false,
            0,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                let message = format!("/directory-load failed to get directory History - {e}");
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    "/directory-load error".to_string(),
                    &message,
                );
            }
        };

        let ignore_pointer = false;
        let version = version.unwrap_or(0);
        match history
            .get_version_entry_value(version, ignore_pointer)
            .await
        {
            Ok(archive_address) => archive_address,
            Err(e) => {
                let message = format!("/directory-load invalid parameters - {e}");
                return make_error_response_page(
                    None,
                    &mut HttpResponse::BadRequest(),
                    "/directory-load error".to_string(),
                    &message,
                );
            }
        }
    };

    println!(
        "DEBUG DirectoryTree::from_archive_address() with address: {}",
        archive_address.to_hex()
    );
    let directory_tree = match DirectoryTree::from_archive_address(&client, archive_address).await {
        Ok(directory_tree) => directory_tree,
        Err(e) => {
            let message = format!("/directory-load failed to get directory Archive - {e}");
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/directory-load error".to_string(),
                &message,
            );
        }
    };

    // println!(
    //     "DEBUG JSON:\n{}",
    //     json_for_svar_file_manager(&directory_tree.directory_map)
    // );

    // let remote_path = if !remote_path.is_empty() {
    //     Some(format!("/{remote_path}"))
    // } else {
    //     None
    // };

    HttpResponse::Ok().body(json_for_svar_file_manager(&directory_tree.directory_map))
}
