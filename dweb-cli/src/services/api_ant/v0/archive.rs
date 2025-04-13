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
    post,
    web::{self, Data},
    HttpRequest, HttpResponse,
};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use autonomi::chunk::DataMapChunk;
use autonomi::client::data::DataAddress;
use autonomi::client::files::Metadata as FileMetadata;
use autonomi::AttoTokens;
use autonomi::{files::PrivateArchive, files::PublicArchive};

use dweb::client::DwebClient;
use dweb::files::archive::DualArchive;
use dweb::files::directory::Tree;
use dweb::helpers::{convert::*, retry::retry_until_ok, web::*};
use dweb::storage::DwebType;
use dweb::trove::History;

use crate::services::api_dweb::v0::PutResult;
use crate::services::helpers::*;

// TODO archive_public_post() for POST
// TODO remove /directory-load and update Fileman example to use it
// TODO replicate archive_public.rs for PrivateArchive

/// Get a directory tree (from PublicArchive or PrivateArchive)
///
/// Returns a DwebArchive schema containing metadata for files and directories
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation (DwebArchive schema) of an Autonomi PublicArchive or PrivateArchive.", body = [DwebArchive])
        ),
    tags = ["Autonomi"],
    params(
        ("datamap_or_address", description = "the hex encoded datamap chunk or data address of an Autonomi archive"),
    )
)]
#[get("/archive/{datamap_or_address}")]
pub async fn get(
    request: HttpRequest,
    datamap_or_address: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let (datamap_chunk, _history_address, archive_address) =
        tuple_from_datamap_address_or_name(&datamap_or_address);

    let tree = match Tree::from_datamap_or_address(&client, datamap_chunk, archive_address).await {
        Ok(archive) => archive,
        Err(e) => {
            let message = format!("/archive GET archive_get() failed - {e}");
            println!("DEBUG {message}");
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive GET error".to_string(),
                &message,
            );
        }
    };

    let dweb_archive = DwebArchive::from_tree(&tree);
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

/// Get a versioned directory tree from a dweb History of PublicArchive or PrivateArchive
///
/// Returns JSON containing metadata for files and directories stored in a PublicArchive or PrivateArchive
///
/// Path parameters refer to the required version and dweb History:
///
///     [v{VERSION-NUMBER}/]{ADDRESS-OR-NAME}
///
/// VERSION-NUMBER      Optional version when ADDRESS-OR-NAME refers to a <code>History<Tree></code>
///
/// ADDRESS-OR-NAME     A hexadecimal address or a short name referring to a History or an Autonomi archive
///
/// url: <code>http://127.0.0.1:8080/archive-version/[v<VERSION-NUMBER>/]<ADDRESS-OR-NAME></code>
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation (DwebArchive schema) of an Autonomi PublicArchive or PrivateArchive.", body = [DwebArchive])
        ),
    tags = ["Autonomi"],
    // params(
    //     ("params" = String, Path, description = "Optional version (integer > 0) of an archive History"),
    //     ("ADDRESS-OR-NAME", description = "A hexadecimal address or a short name referring to an archive History"),
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

    let (history_address, archive_address) = tuple_from_address_or_name(&address_or_name);
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
        archive_address
    } else {
        let history_address = history_address.unwrap();
        let mut history =
            match History::<Tree>::from_history_address(client.clone(), history_address, false, 0)
                .await
            {
                Ok(history) => history,
                Err(e) => {
                    let message =
                        format!("/archive-version GET failed to get directory History - {e}");
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
            Ok(archive_address) => Some(archive_address),
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

    let tree = match Tree::from_datamap_or_address(&client, None, archive_address).await {
        Ok(archive) => archive,
        Err(e) => {
            let message = format!("/archive-version GET archive_get() failed - {e}");
            println!("DEBUG {message}");
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive-version GET error".to_string(),
                &message,
            );
        }
    };

    let mut dweb_archive = DwebArchive::from_tree(&tree);
    dweb_archive.history_metadata = history_metadata;
    let json = match serde_json::to_string(&dweb_archive) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                "/archive-version GET error".to_string(),
                &format!("archive version GET failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG DwebArchive as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Get the file metadata in a directory tree
///
/// Retrieves a PublicArchive from Autonomi and returns metadata for all files it contains.
///
/// Path parameters:
///
///     [v{version}/]{address_or_name}
///
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of a Tree formatted for an SVAR file manager component.
            <p>Note: this may be changed to return a JSON representation of a Tree.", body = str)
        ),
    tags = ["Dweb"],
    params(
        ("params", description = "[v{version}/]{address_or_name}<br/><br/>Optional version (integer > 0), an address_or_name which refers to a History<Tree>"),
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

    let (history_address, archive_address) = tuple_from_address_or_name(&address_or_name);
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
        let mut history =
            match History::<Tree>::from_history_address(client.clone(), history_address, false, 0)
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
        "DEBUG Tree::from_archive_address() with address: {}",
        archive_address.to_hex()
    );
    let directory_tree = match Tree::from_archive_address(&client, archive_address).await {
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

#[derive(Deserialize, ToSchema)]
struct QueryParams {
    tries: Option<u32>,
}

/// Upload a PrivateArchive using POST request body
///
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each file upload, 0 means unlimited. This overrides the API control setting in the server.")),
    request_body(content = DwebArchive, content_type = "application/json"),
    responses(
        (status = 200, description = "A PutResult featuring either status 200 with cost and data address on the network, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;INTERNAL_SERVER_ERROR: Error reading file or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;BAD_GATEWAY: Autonomi network error", body = PutResult,
            example = json!("{\"file_name\": \"\", \"status\": \"200\", \"cost_in_attos\": \"12\", \"data_address\": \"a9cd8dd0c9f2b9dc71ad548d1f37fcba6597d5eb1be0b8c63793802cc6c7de27\", \"data_map\": \"\", \"message\": \"\" }")),
    ),
    tags = ["Autonomi"],
)]
#[post("/archive-private")]
pub async fn post_private(
    request: HttpRequest,
    dweb_archive: web::Json<DwebArchive>,
    query_params: web::Query<QueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let tries = query_params.tries.unwrap_or(client.api_control.tries);

    let private_archive = match dweb_archive.into_inner().to_private_archive() {
        Ok(archive) => archive,
        Err(_e) => {
            let message =
                format!("/archive-private POST failed to deserialise body as DwebArchive");
            println!("DEBUG {message}");
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive POST error".to_string(),
                &message,
            );
        }
    };

    let put_result = put_archive_private(&client, &private_archive, tries).await;

    let json = match serde_json::to_string(&put_result) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                "/archive-private POST error".to_string(),
                &format!("archive::post_private() failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG put_result as JSON: {json:?}");
    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// Upload a PublicArchive using POST request body
///
#[utoipa::path(
    post,
    params(
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each file upload, 0 means unlimited. This overrides the API control setting in the server.")),
    request_body(content = DwebArchive, content_type = "application/json"),
    responses(
        (status = 200, description = "A PutResult featuring either status 200 with cost and data address on the network, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;INTERNAL_SERVER_ERROR: Error reading file or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;BAD_GATEWAY: Autonomi network error", body = PutResult,
            example = json!("{\"file_name\": \"\", \"status\": \"200\", \"cost_in_attos\": \"12\", \"data_address\": \"a9cd8dd0c9f2b9dc71ad548d1f37fcba6597d5eb1be0b8c63793802cc6c7de27\", \"data_map\": \"\", \"message\": \"\" }")),
    ),
    tags = ["Autonomi"],
)]
#[post("/archive-public")]
pub async fn post_public(
    request: HttpRequest,
    dweb_archive: web::Json<DwebArchive>,
    query_params: web::Query<QueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let tries = query_params.tries.unwrap_or(client.api_control.tries);

    let public_archive = match dweb_archive.into_inner().to_public_archive() {
        Ok(archive) => archive,
        Err(_e) => {
            let message = format!("/archive-public POST failed to deserialise body as DwebArchive");
            println!("DEBUG {message}");
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/archive POST error".to_string(),
                &message,
            );
        }
    };

    let put_result = put_archive_public(&client, &public_archive, tries).await;

    let json = match serde_json::to_string(&put_result) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                "/archive-public POST error".to_string(),
                &format!("archive::post_public() failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG put_result as JSON: {json:?}");
    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

async fn put_archive_private(
    client: &DwebClient,
    archive: &PrivateArchive,
    tries: u32,
) -> PutResult {
    let payment_option = client.payment_option().clone();
    let result = retry_until_ok(
        tries,
        &"archive_put_private()",
        (archive, payment_option),
        async move |(archive, payment_option)| match client
            .client
            .archive_put(archive, payment_option.clone())
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(eyre!(e)),
        },
    )
    .await;

    match result {
        Ok(result) => {
            println!("DEBUG put_archive_public() stored PublicArchive on the network at address");
            let mut put_result = PutResult::new(
                DwebType::PrivateArchive,
                StatusCode::OK,
                "success".to_string(),
                result.0,
            );

            put_result.data_map = result.1.to_hex();
            put_result
        }
        Err(e) => {
            let status_message =
                format!("put_archive_private() failed store PrivateArchive on the network - {e}");
            println!("DEBUG {status_message}");
            return PutResult::new(
                DwebType::PrivateArchive,
                StatusCode::BAD_GATEWAY,
                status_message,
                AttoTokens::zero(),
            );
        }
    }
}

async fn put_archive_public(client: &DwebClient, archive: &PublicArchive, tries: u32) -> PutResult {
    let payment_option = client.payment_option().clone();
    let result = retry_until_ok(
        tries,
        &"archive_put_public()",
        (archive, payment_option),
        async move |(archive, payment_option)| match client
            .client
            .archive_put_public(archive, payment_option.clone())
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(eyre!(e)),
        },
    )
    .await;

    match result {
        Ok(result) => {
            println!("DEBUG put_archive_public() stored PublicArchive on the network at address");
            let mut put_result = PutResult::new(
                DwebType::PublicArchive,
                StatusCode::OK,
                "success".to_string(),
                result.0,
            );

            put_result.data_address = result.1.to_hex();
            put_result
        }
        Err(e) => {
            let status_message =
                format!("put_archive_public() failed store PublicArchive on the network - {e}");
            println!("DEBUG {status_message}");
            return PutResult::new(
                DwebType::PublicArchive,
                StatusCode::BAD_GATEWAY,
                status_message,
                AttoTokens::zero(),
            );
        }
    }
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
    /// Information about a History will only be present only when retrieved from a History using /archive-public-version this
    pub history_metadata: Option<DwebHistoryReference>,
}

impl DwebArchive {
    pub fn new() -> DwebArchive {
        DwebArchive {
            history_metadata: None,
            entries: Vec::<DwebArchiveEntry>::new(),
        }
    }

    /// Create a DwebArchive from a dweb::files::directory::Tree
    pub fn from_tree(tree: &Tree) -> DwebArchive {
        Self::from_dual_archive(&tree.archive)
    }

    /// Create a DwebArchive from a DualArchive (which may wrap either PublicArchive or PrivateArchive)
    pub fn from_dual_archive(archive: &DualArchive) -> DwebArchive {
        match archive.dweb_type {
            DwebType::PrivateArchive => Self::from_private_archive(&archive.private_archive),
            DwebType::PublicArchive => Self::from_public_archive(&archive.public_archive),
            _ => {
                println!("DEBUG DwebArchive::from_dual_archive() unable to deserialise unknown DwebType - this is probably a bug",);
                DwebArchive::new()
            }
        }
    }

    /// Create a DwebArchive from a PublicArchive
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
            let mut web_path = dweb::files::directory::TreePathMap::webify_string(&path_string);

            if let Some(last_separator_position) =
                web_path.rfind(dweb::files::archive::ARCHIVE_PATH_SEPARATOR)
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
                        Some(*xor_name),
                        None,
                        metadata,
                    ));
                    files_added.insert(file_full_path);
                }
            } else {
                println!(
                    "DEBUG DwebArchive::from_public_archive(): path separator not found in resource website path: {web_path} - this is probably a bug"
                );
            }
        }
        dweb_archive
    }

    /// Create a DwebArchive from a PrivateArchive
    pub fn from_private_archive(archive: &PrivateArchive) -> DwebArchive {
        let mut dweb_archive = DwebArchive::new();
        let mut directories_added = HashSet::<String>::new();
        let mut files_added = HashSet::<String>::new();

        let mut iter = archive.map().iter();
        while let Some((path_buf, (datamap_chunk, metadata))) = iter.next() {
            // Remove the containing directory to produce a path from the website root, and which starts with '/'
            let mut path_string = String::from(path_buf.to_string_lossy());
            let offset = path_string.find("/").unwrap_or(path_string.len());
            path_string.replace_range(..offset, "");
            let mut web_path = dweb::files::directory::TreePathMap::webify_string(&path_string);

            if let Some(last_separator_position) =
                web_path.rfind(dweb::files::archive::ARCHIVE_PATH_SEPARATOR)
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

                let data_address = match DataAddress::from_hex(&datamap_chunk.address()) {
                    Ok(data_address) => Some(data_address),
                    Err(e) => {
                        println!("DEBUG DwebArchive::from_private_archive() failed to decode datamap_chunk - {e}");
                        None
                    }
                };
                if !files_added.contains(&file_full_path) {
                    dweb_archive.entries.push(DwebArchiveEntry::new_file(
                        file_full_path.clone(),
                        data_address,
                        Some(datamap_chunk.clone()),
                        metadata,
                    ));
                    files_added.insert(file_full_path);
                }
            } else {
                println!(
                    "DEBUG DwebArchive::from_private_archive(): path separator not found in resource website path: {web_path} - this is probably a bug"
                );
            }
        }
        dweb_archive
    }

    /// Return as a new PrivateArchive. Assumes all files added are public files (ie have valid data addresses)
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

    /// Return as a new PrivateArchive. Assumes all files added are private files (ie have valid datamaps)
    pub fn to_private_archive(&self) -> Result<PrivateArchive> {
        let mut archive = PrivateArchive::new();

        for entry in &self.entries {
            match entry.dweb_type {
                DwebArchiveEntryType::File => {
                    let file_path = PathBuf::from(&entry.full_path);
                    let datamap_chunk = match DataMapChunk::from_hex(&entry.datamap) {
                        Ok(datamap_chunk) => datamap_chunk,
                        Err(e) => {
                            let message =
                                format!("entry has invalid datamap: {}, {e}", entry.datamap);
                            println!("DEBUG DEBUG DwebEntry::to_private_archive() - {message}");
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

                    archive.add_file(file_path, datamap_chunk, metadata)
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
    /// Hexadecimal address of the datamap for a file (required for a DwebType::PublicArchive)
    pub data_address: String,
    /// Hexadecimal representation of the datamap for a file (required for a DwebType::PrivateArchive)
    pub datamap: String,
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
            datamap: "".to_string(),
            created: "".to_string(),
            modified: "".to_string(),
            size: 0,
            extra: "".to_string(),
        }
    }

    pub fn new_file(
        full_path: String,
        data_address: Option<DataAddress>,
        datamap_chunk: Option<DataMapChunk>,
        metadata: &FileMetadata,
    ) -> DwebArchiveEntry {
        let data_address = if let Some(data_address) = data_address {
            data_address.to_hex()
        } else {
            "".to_string()
        };
        let datamap = if let Some(datamap_chunk) = datamap_chunk {
            datamap_chunk.to_hex()
        } else {
            "".to_string()
        };
        DwebArchiveEntry {
            dweb_type: DwebArchiveEntryType::File,
            full_path,
            data_address,
            datamap,
            created: metadata_date_to_json_datestring(metadata.created),
            modified: metadata_date_to_json_datestring(metadata.modified),
            size: metadata.size,
            extra: metadata.extra.clone().unwrap_or("".to_string()).clone(),
        }
    }

    pub fn new_file_from_hex(
        full_path: String,
        data_address: String,
        datamap_chunk: String,
        metadata: &FileMetadata,
    ) -> DwebArchiveEntry {
        DwebArchiveEntry {
            dweb_type: DwebArchiveEntryType::File,
            full_path,
            data_address,
            datamap: datamap_chunk,
            created: metadata_date_to_json_datestring(metadata.created),
            modified: metadata_date_to_json_datestring(metadata.modified),
            size: metadata.size,
            extra: metadata.extra.clone().unwrap_or("".to_string()).clone(),
        }
    }
}
