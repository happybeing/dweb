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
use xor_name::XorName as DirectoryAddress;

use crate::cache::directory_version::{DirectoryVersion, DIRECTORY_VERSIONS, HISTORY_NAMES};
use crate::client::AutonomiClient;
use crate::trove::History;
use crate::trove::{directory_tree::DirectoryTree, HistoryAddress};
use crate::web::name::decode_dweb_host;
use crate::web::name::DwebHost;

/// Fetch the requested resource from Autonomi or from cached data if available.
///  Assumes a dweb URL
/// TODO update to use response_with_body() instead of reason()
pub async fn fetch(client: &AutonomiClient, url: Url) -> HttpResponse {
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
    let response = match fetch_website_version(client, &dweb_host).await {
        // TODO cache function that wraps fetching the History/DirectoryTree
        Ok((_version, cache_version_entry)) => {
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
/// If the return is Ok(version, DirectoryVersion), the DirectoryVersion will have Some(DirectoryTree).
/// The version returned is the version retrieved, which is useful if dweb_host.version is None.
//
// Notes:
//   1) ensures that cache locks are released ASAP, and not held during network access.
//   2) may return an error, but still update the cache with an incomplete DirectoryVersion
//      if it obtains the DirectoryVersion.directory_address but not the directory_tree. A subsequent call
//      using the same DwebHost can then skip getting the directory_address and will just retry getting
//      the directory_tree.
// TODO refactor fetch_website_version() to reduce complexity
pub async fn fetch_website_version(
    client: &AutonomiClient,
    dweb_host: &DwebHost,
) -> Result<(u32, DirectoryVersion)> {
    println!(
        "DEBUG pub async fn fetch_website_version([ {}, {}, {:?} ])...",
        dweb_host.dweb_host_string, dweb_host.dweb_name, dweb_host.version
    );
    // If the cache has all the info we return, or if it has an entry but no DirectoryTree we can use the addresses
    let (history_address, directory_address) = if let Ok(lock) = &mut DIRECTORY_VERSIONS.lock() {
        if let Some(cached_directory_version) = lock.get(&dweb_host.dweb_host_string) {
            if cached_directory_version.directory_tree.is_some() {
                // Version 0 is ok here because if we have the tree we will already have cached the version
                return Ok((0, cached_directory_version.clone()));
            } else {
                (
                    Some(cached_directory_version.history_address),
                    Some(cached_directory_version.directory_address),
                )
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let history_address = if history_address.is_none() {
        // We need the history to get either the DirectoryAddress and/or the DirectoryTree
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
    if directory_address.is_some() {
        let directory_address = directory_address.unwrap();
        let directory_tree =
            match History::<DirectoryTree>::raw_trove_download(client, directory_address).await {
                Ok(directory_tree) => directory_tree,
                Err(e) => return Err(eyre!("failed to download directory from network: {e}")),
            };
        return update_cached_directory_version(
            &dweb_host,
            history_address,
            directory_address,
            Some(directory_tree),
        );
    } else {
        let mut history =
            match History::<DirectoryTree>::from_history_address(client.clone(), history_address)
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
                if version > history_versions {
                    return Err(eyre!(
                        "Invalid version {version}, highest version is {history_versions}"
                    ));
                } else {
                    return Err(eyre!("History is empty - no website to display"));
                }
            }
        }

        let (directory_address, directory_tree, version) =
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
        let default_result = update_cached_directory_version(
            &dweb_host,
            history_address,
            directory_address,
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
                // Development build feature for non-versioned DirectoryTree references
                is_fixed_dweb_host: false,
            };

            return update_cached_directory_version(
                &versioned_dweb_host,
                history_address,
                directory_address,
                Some(directory_tree),
            );
        }

        return default_result;
    };
}

pub fn update_cached_directory_version(
    dweb_host: &DwebHost,
    history_address: HistoryAddress,
    directory_address: DirectoryAddress,
    directory_tree: Option<DirectoryTree>,
) -> Result<(u32, DirectoryVersion)> {
    // TODO may need both version_retrieved and version_requested in DirectoryVersion
    let new_directory_version = DirectoryVersion::new(
        &dweb_host,
        history_address,
        directory_address,
        directory_tree,
    );

    match &mut DIRECTORY_VERSIONS.lock() {
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
                "Failed to store DirectoryVersion in cache for DWEB-NAME '{}': {e}",
                dweb_host.dweb_name
            ));
        }
    }

    Ok((dweb_host.version.unwrap_or(0), new_directory_version))
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

pub fn response_redirect(req: &HttpRequest, host: &String, path: Option<&String>) -> HttpResponse {
    let scheme = &String::from(req.full_url().scheme());
    let port = if let Some(port) = req.full_url().port() {
        &format!(":{port}")
    } else {
        ""
    };

    #[cfg(feature = "development")]
    println!("DEBUG req.full_url(): {}", req.full_url());
    println!("DEBUG scheme   : {scheme}");
    println!("DEBUG port     : {port}");

    let mut redirect_url = String::from(scheme) + "://" + host;
    if let Some(path) = path {
        redirect_url = redirect_url + &path;
    }

    redirect_url = redirect_url + port;

    #[cfg(feature = "development")]
    println!("DEBUG response_redirect() redirecting to {redirect_url}");
    HttpResponseBuilder::new(StatusCode::SEE_OTHER)
        .insert_header((header::LOCATION, redirect_url))
        .finish()
}
