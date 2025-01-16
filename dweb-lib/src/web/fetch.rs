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

use actix_web::{
    body,
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    get, guard,
    http::{header, header::HeaderValue, StatusCode},
    post, web, App, Error, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use color_eyre::eyre::{eyre, Result};
use url::Url;
use xor_name::XorName as DirectoryAddress;

use ant_registers::RegisterAddress as HistoryAddress;

use crate::cache::directory_version::{DirectoryVersion, DIRECTORY_VERSIONS, HISTORY_NAMES};
use crate::client::AutonomiClient;
use crate::trove::directory_tree::DirectoryTree;
use crate::trove::History;
use crate::web::name::decode_dweb_host;
use crate::web::name::DwebHost;

/// Fetch the requested resource from Autonomi or from cached data if available.
///  Assumes a dweb URL
pub async fn fetch(client: &AutonomiClient, url: Url) -> HttpResponse {
    let host = match url.host_str() {
        Some(host) => host,
        None => {
            return HttpResponseBuilder::new(StatusCode::BAD_REQUEST)
                .reason("bad host in URL")
                .finish()
        }
    };

    let web_name = match decode_dweb_host(host) {
        Ok(web_name) => web_name,
        Err(_e) => {
            return HttpResponseBuilder::new(StatusCode::NOT_FOUND)
                .reason("failed to decode web name")
                .finish()
        }
    };

    let mut reason: &'static str = "";
    let response = match fetch_website_version(client, web_name).await {
        // TODO cache function that wraps fetching the History/DirectoryTree
        Ok(cache_version_entry) => {
            match cache_version_entry
                .directory_tree
                .unwrap() // Guaranteed to be Some() by fetch_website_version()
                .lookup_web_resource(&url.path().to_string())
            {
                Ok((file_address, content_type)) => {
                    let content_type = if content_type.is_some() {
                        content_type.unwrap().clone()
                    } else {
                        String::from("text/plain")
                    };

                    match client.data_get_public(file_address).await {
                        Ok(bytes) => Some(
                            HttpResponseBuilder::new(StatusCode::OK)
                                .insert_header((header::CONTENT_TYPE, content_type.as_str()))
                                .body(bytes),
                        ),
                        Err(_e) => {
                            reason = "Failed to get file from network";
                            None
                        }
                    }
                }
                Err(_e) => {
                    reason = "Failed at lookup_or_fetch_file()";
                    None
                }
            }
        }
        Err(_e) => {
            reason = "Failed to get website version";
            None
        }
    };

    if response.is_some() {
        response.unwrap()
    } else {
        HttpResponseBuilder::new(StatusCode::NOT_FOUND)
            .reason(reason)
            .finish()
    }
}

/// Retrieve a given DirectoryVersion from the cache, or if not access the network and
/// create a new DirectoryVersion based on the DwebHost.
/// If the return is Ok(DirectoryVersion), it is guaranteed to have Some(DirectoryTree)
//
// Notes:
//   1) ensures that cache locks are released ASAP, and not held during network access.
//   2) may return an error, but still update the cache with an incomplete DirectoryVersion
//      if it obtains the DirectoryVersion.directory_address but not the directory_tree. A subsequent call
//      using the same DwebHost can then skip getting the directory_address and will just retry getting
//      the directory_tree.
pub async fn fetch_website_version(
    client: &AutonomiClient,
    web_name: DwebHost,
) -> Result<DirectoryVersion> {
    // If the cache has all the info we return, or if it has an entry but no DirectoryTree we can use the addresses
    let (history_address, directory_address) = if let Ok(lock) = &mut DIRECTORY_VERSIONS.lock() {
        let cached_directory_version = lock.get(&web_name.dweb_host_string);
        if cached_directory_version.is_some()
            && cached_directory_version
                .as_ref()
                .unwrap()
                .directory_tree
                .is_some()
        {
            let directory_version = cached_directory_version.unwrap().clone();
            return Ok(directory_version);
        } else {
            let directory_version = cached_directory_version.unwrap().clone();
            (
                Some(directory_version.history_address),
                Some(directory_version.directory_address),
            )
        }
    } else {
        (None, None)
    };

    let history_address = if history_address.is_none() {
        // We need the history to get either the DirectoryAddress and/or the DirectoryTree
        if let Ok(lock) = &mut HISTORY_NAMES.lock() {
            lock.get(&web_name.dweb_name).copied().unwrap()
        } else {
            return Err(eyre!(format!("Unknown DWEB-NAME '{}'", web_name.dweb_name)));
        }
    } else {
        history_address.unwrap()
    };

    // At this point we have at least a history address
    if directory_address.is_some() {
        let directory_address = directory_address.unwrap();
        let directory_tree =
            match History::<DirectoryTree>::raw_trove_download(client, directory_address).await {
                Ok(directory_tree) => directory_tree,
                Err(e) => return Err(eyre!("Failed to download directory from network: {e}")),
            };
        return update_cached_directory_version(
            &web_name,
            history_address,
            directory_address,
            Some(directory_tree),
        );
    } else {
        let mut history =
            match History::<DirectoryTree>::new(client.clone(), Some(history_address)).await {
                Ok(history) => history,
                Err(e) => {
                    return Err(eyre!(
                        "Failed to get History for DWEB-NAME '{}': {e}",
                        web_name.dweb_name,
                    ))
                }
            };

        let (directory_address, directory_tree) =
            match history.fetch_version_metadata(web_name.version).await {
                Some(directory_tree) => match history.get_cached_version() {
                    Some(cached_version) => (cached_version.metadata_address(), directory_tree),
                    None => return Err(eyre!("History failed to get_cached_version()")),
                },
                None => return Err(eyre!("History failed to fetch_version_metadata()")),
            };

        return update_cached_directory_version(
            &web_name,
            history_address,
            directory_address,
            Some(directory_tree),
        );
    };
}

pub fn update_cached_directory_version(
    web_name: &DwebHost,
    history_address: HistoryAddress,
    directory_address: DirectoryAddress,
    directory_tree: Option<DirectoryTree>,
) -> Result<DirectoryVersion> {
    // TODO may need both version_retrieved and version_requested in DirectoryVersion
    let new_directory_version = DirectoryVersion::new(
        &web_name,
        history_address,
        directory_address,
        directory_tree,
    );

    match &mut DIRECTORY_VERSIONS.lock() {
        Ok(lock) => {
            lock.insert(
                web_name.dweb_host_string.clone(),
                new_directory_version.clone(),
            );
        }
        Err(e) => {
            return Err(eyre!(
                "Failed to store DirectoryVersion in cache for DWEB-NAME '{}': {e}",
                web_name.dweb_name
            ));
        }
    }

    Ok(new_directory_version)
}
