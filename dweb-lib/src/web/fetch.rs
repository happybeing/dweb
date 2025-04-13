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
    http::{header, StatusCode},
    HttpRequest, HttpResponse, HttpResponseBuilder,
};
use color_eyre::eyre::{eyre, Result};
use mime;
use url::Url;

use autonomi::client::files::archive_public::ArchiveAddress;

use crate::client::DwebClient;
use crate::files::directory::{get_content_using_hex, Tree};
use crate::trove::{History, HistoryAddress};
use crate::web::name::decode_dweb_host;
use crate::web::name::DwebHost;
use crate::{
    cache::directory_with_name::{
        DirectoryVersionWithName, DIRECTORY_VERSIONS_WITH_NAME, HISTORY_NAMES,
    },
    cache::directory_with_port::{
        key_for_directory_versions_with_port, DirectoryVersionWithPort,
        DIRECTORY_VERSIONS_WITH_PORT,
    },
    helpers::convert::address_tuple_from_address,
};

/// Fetch the requested resource from Autonomi or from cached data if available.
/// Assumes a dweb URL.
///
/// If as_website is false the URL is handled as an exact file path.
///
/// When as_website is true website specific handling such as redirecting
/// a directory path to an index.html etc is enabled.
///
/// TODO update to use response_with_body() instead of reason()
pub async fn fetch(client: &DwebClient, url: Url, as_website: bool) -> HttpResponse {
    println!("DEBUG fetch({url:?})...");
    let host = match url.host_str() {
        Some(host) => host,
        None => {
            return HttpResponseBuilder::new(StatusCode::BAD_REQUEST)
                .reason("bad host in URL")
                .finish()
        }
    };

    let dweb_host = match decode_dweb_host(host) {
        Ok(dweb_host) => dweb_host,
        Err(_e) => {
            return HttpResponseBuilder::new(StatusCode::NOT_FOUND)
                .reason("failed to decode web name")
                .finish()
        }
    };

    let mut reason: &'static str = "";
    let response = match directory_version_get(client, &dweb_host).await {
        // TODO cache function that wraps fetching the History/Tree
        Ok((_version, cache_version_entry)) => {
            match cache_version_entry
                .directory_tree
                .unwrap() // Guaranteed to be Some() by directory_version_get()
                .lookup_file(&url.path().to_string(), as_website)
            {
                Ok((datamap_chunk, data_address, content_type)) => {
                    let content_type = if content_type.is_some() {
                        content_type.unwrap().clone()
                    } else {
                        String::from("text/plain")
                    };

                    match get_content_using_hex(client, datamap_chunk, data_address).await {
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

/// Retrieve a given DirectoryVersionWithName from the cache, or if not access the network and
/// create a new DirectoryVersionWithName based on the DwebHost.
/// If the return is Ok(version, DirectoryVersionWithName), the DirectoryVersionWithName will have Some(Tree).
/// The version returned is the version retrieved, which is useful if dweb_host.version is None.
//
// Notes:
//   1) ensures that cache locks are released ASAP, and not held during network access.
//   2) may return an error, but still update the cache with an incomplete DirectoryVersionWithName
//      if it obtains the DirectoryVersionWithName.archive_address but not the directory_tree. A subsequent call
//      using the same DwebHost can then skip getting the archive_address and will just retry getting
//      the directory_tree.
// TODO refactor directory_version_get() to reduce complexity
pub async fn directory_version_get(
    client: &DwebClient,
    dweb_host: &DwebHost,
) -> Result<(u32, DirectoryVersionWithName)> {
    println!(
        "DEBUG directory_version_get([ {}, {}, {:?} ])...",
        dweb_host.dweb_host_string, dweb_host.dweb_name, dweb_host.version
    );

    // If the cache has all the info we return, or if it has an entry but no Tree we can use the addresses
    let (history_address, archive_address) =
        if let Ok(lock) = &mut DIRECTORY_VERSIONS_WITH_NAME.lock() {
            if let Some(cached_directory_version) = lock.get(&dweb_host.dweb_host_string) {
                if cached_directory_version.directory_tree.is_some() {
                    // Version 0 is ok here because if we have the tree we will already have cached the version
                    return Ok((0, cached_directory_version.clone()));
                } else {
                    (
                        Some(cached_directory_version.history_address),
                        Some(cached_directory_version.archive_address),
                    )
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

    let history_address = if history_address.is_none() {
        // We need the history to get either the ArchiveAddress and/or the Tree
        if let Ok(lock) = &mut HISTORY_NAMES.lock() {
            if let Some(history_address) = lock.get(&dweb_host.dweb_name).copied() {
                history_address
            } else {
                return Err(eyre!(format!(
                    "unknown DWEB-NAME '{}'",
                    dweb_host.dweb_name
                )));
            }
        } else {
            return Err(eyre!(format!("failed to access DWEB-NAME cache",)));
        }
    } else {
        history_address.unwrap()
    };

    // At this point we have at least a history address
    if archive_address.is_some() {
        let archive_address = archive_address.unwrap();
        let directory_tree =
            match History::<Tree>::raw_trove_download(client, archive_address).await {
                Ok(directory_tree) => directory_tree,
                Err(e) => return Err(eyre!("failed to download directory from network: {e}")),
            };
        return update_cached_directory_version_with_name(
            &dweb_host,
            history_address,
            archive_address,
            Some(directory_tree),
        );
    } else {
        // TODO using dweb_host.version.is_none() for ignore pointer would ensures all versions
        // TODO are available even if the pointer is out of date, but this takes more than 20s.
        // TODO If apps cache the pointer counter, provide a way they can pass that for minimum_entry_index
        // TODO so that from_history_address() never has to wait while walking the graph, and
        // TODO can know the pointer is up-to-date from the minimum_entry_index

        // TODO this avoids issue where pointer is not up-to-date but makes the first load take ~20s
        let (ignore_pointer, minimum_entry_index) = if dweb_host.version.is_some() {
            (false, dweb_host.version.unwrap() + 1)
        } else {
            (true, 0)
        };

        // TODO this will load fast but may be missing later updates if the pointer
        // TODO isn't up-to-date on the network
        let (ignore_pointer, minimum_entry_index) = (false, 0);

        let mut history = match History::<Tree>::from_history_address(
            client.clone(),
            history_address,
            ignore_pointer,
            minimum_entry_index,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                return Err(eyre!(
                    "failed to get History for DWEB-NAME '{}': {e}",
                    dweb_host.dweb_name,
                ));
            }
        };

        if let Some(version) = dweb_host.version {
            if let Ok(history_versions) = history.num_versions() {
                if history_versions == 0 {
                    return Err(eyre!("History is empty - no website to display"));
                } else if version > history_versions {
                    return Err(eyre!(
                        "Invalid version {version}, highest version is {history_versions}"
                    ));
                } else if version < 1 {
                    return Err(eyre!("Invalid version {version}, lowest version is 1"));
                };
            }
        }

        let (archive_address, directory_tree, version) =
            match history.fetch_version_trove(dweb_host.version).await {
                Some(directory_tree) => match history.get_cached_version() {
                    Some(cached_version) => (
                        cached_version.trove_address(),
                        directory_tree,
                        cached_version.version,
                    ),
                    None => return Err(eyre!("History failed to get_cached_version()")),
                },
                None => return Err(eyre!("History failed to fetch_version_metadata()")),
            };

        // When retrieving the most recent version, ensure that the corresponding versioned DwebHost is cached
        let default_result = update_cached_directory_version_with_name(
            &dweb_host,
            history_address,
            archive_address,
            Some(directory_tree.clone()),
        );

        // When retrieving the most recent version, ensure that the corresponding versioned DwebHost is cached
        if dweb_host.version.is_none() {
            let versioned_host = format!("v{version}.{}", dweb_host.dweb_host_string);
            let versioned_dweb_host = DwebHost {
                dweb_host_string: versioned_host,
                dweb_name: dweb_host.dweb_name.clone(),
                version: Some(version),

                #[cfg(feature = "fixed-dweb-hosts")]
                // Development build feature for non-versioned Tree references
                is_fixed_dweb_host: false,
            };

            return update_cached_directory_version_with_name(
                &versioned_dweb_host,
                history_address,
                archive_address,
                Some(directory_tree),
            );
        }

        return default_result;
    };
}

/// Get a Tree from the network using the address and if a history, the optional version
pub async fn get_directory_tree_for_address_string(
    client: &DwebClient,
    // The hex representation of either a HistoryAddress or an ArchiveAddress
    address: &String,
    // Optional version when the address is a HistoryAddress
    version: Option<u32>,
) -> Result<(Option<HistoryAddress>, ArchiveAddress, Option<u32>, Tree)> {
    println!("DEBUG get_directory_tree_for_address_string({address}, {version:?})...");

    let (history_address, archive_address) = address_tuple_from_address(address);
    if history_address.is_none() && archive_address.is_none() {
        let msg = format!("Not a history or archive address: {address}");
        return Err(eyre!(msg));
    };

    if archive_address.is_some() {
        return Ok((
            None,
            archive_address.unwrap(),
            version,
            Tree::from_archive_address(client, archive_address.unwrap()).await?,
        ));
    }

    let ignore_pointer = true; // Fast but may not get most recent version when version is None
    let minimum_entry_index = version.unwrap_or(0);
    match History::<Tree>::from_history_address(
        client.clone(),
        history_address.unwrap(),
        ignore_pointer,
        minimum_entry_index,
    )
    .await
    {
        Ok(mut history) => {
            let (archive_address, directory_tree, version) =
                match history.fetch_version_trove(version).await {
                    Some(directory_tree) => match history.get_cached_version() {
                        Some(cached_version) => (
                            cached_version.trove_address(),
                            directory_tree,
                            cached_version.version,
                        ),
                        None => return Err(eyre!("History failed to get_cached_version()")),
                    },
                    None => return Err(eyre!("History failed to fetch_version_metadata()")),
                };
            Ok((
                history_address,
                archive_address,
                Some(version),
                directory_tree,
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn update_cached_directory_version_with_name(
    dweb_host: &DwebHost,
    history_address: HistoryAddress,
    archive_address: ArchiveAddress,
    directory_tree: Option<Tree>,
) -> Result<(u32, DirectoryVersionWithName)> {
    // TODO may need both version_retrieved and version_requested in DirectoryVersionWithName
    let new_directory_version =
        DirectoryVersionWithName::new(&dweb_host, history_address, archive_address, directory_tree);

    match &mut DIRECTORY_VERSIONS_WITH_NAME.lock() {
        Ok(lock) => {
            #[cfg(feature = "development")]
            println!(
                "DEBUG directory version (v {:?}) added to cache for host: {}",
                dweb_host.version, dweb_host.dweb_host_string
            );

            lock.insert(
                dweb_host.dweb_host_string.clone(),
                new_directory_version.clone(),
            );
        }
        Err(e) => {
            return Err(eyre!(
                "Failed to store DirectoryVersionWithName in cache for DWEB-NAME '{}': {e}",
                dweb_host.dweb_name
            ));
        }
    }

    Ok((dweb_host.version.unwrap_or(0), new_directory_version))
}

pub fn update_cached_directory_version_with_port(
    port: u16,
    history_address: Option<HistoryAddress>,
    archive_address: ArchiveAddress,
    version: Option<u32>,
    directory_tree: Tree,
) -> Result<(u32, DirectoryVersionWithPort)> {
    // TODO may need both version_retrieved and version_requested in DirectoryVersionWithName
    let new_directory_version = DirectoryVersionWithPort::new(
        port,
        history_address,
        version,
        archive_address,
        directory_tree,
    );

    match &mut DIRECTORY_VERSIONS_WITH_PORT.lock() {
        Ok(lock) => {
            #[cfg(feature = "development")]
            println!(
                "DEBUG directory version with port (v {version:?}) added to cache for port: {port}",
            );

            let key = key_for_directory_versions_with_port(archive_address);
            lock.insert(key, new_directory_version.clone());
        }
        Err(e) => {
            return Err(eyre!(
                "Failed to store DirectoryVersionWithPort in cache for PORT '{port}': {e}",
            ));
        }
    }

    Ok((version.unwrap_or(0), new_directory_version))
}

#[cfg(not(feature = "development"))]
const NO_REASON: &str = "";

pub fn response_with_body(status: StatusCode, reason: Option<String>) -> HttpResponse {
    if let Some(reason) = reason {
        let html = format!(
            "<html>
            <head><title>{status}</title></head>
            <body>
            <p>{status}</p>
            <p>{reason}</p>
            </body>
            </html>",
        );
        #[cfg(feature = "development")]
        let reason = Box::leak(reason.into_boxed_str()); // This memory is leaked, hence development only

        #[cfg(not(feature = "development"))]
        let reason = NO_REASON;

        return HttpResponseBuilder::new(status)
            .append_header(header::ContentType(mime::TEXT_HTML))
            .reason(reason)
            .body(html);
    } else {
        HttpResponseBuilder::new(status).finish()
    }
}

pub fn response_redirect(
    req: &HttpRequest,
    host: &str,
    port: Option<u16>,
    path: Option<String>,
) -> HttpResponse {
    let scheme = &String::from(req.full_url().scheme());
    let port_str = if let Some(port) = port {
        &format!(":{port}")
    } else {
        if let Some(port) = req.full_url().port() {
            &format!(":{port}")
        } else {
            ""
        }
    };

    #[cfg(feature = "development")]
    println!("DEBUG req.full_url(): {}", req.full_url());
    println!("DEBUG scheme   : {scheme}");
    println!("DEBUG port     : {port_str}");

    let mut redirect_url = String::from(scheme) + "://" + host;
    redirect_url = redirect_url + port_str;
    if let Some(path) = path {
        redirect_url = redirect_url + &path;
    }

    #[cfg(feature = "development")]
    println!("DEBUG response_redirect() redirecting to {redirect_url}");
    HttpResponseBuilder::new(StatusCode::SEE_OTHER)
        .insert_header((header::LOCATION, redirect_url))
        .finish()
}
