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
use color_eyre::eyre::{eyre, Result};
use std::path::PathBuf;
use walkdir::WalkDir;
use xor_name::XorName;

use ant_protocol::storage::PointerAddress as HistoryAddress;
use autonomi::client::files::archive_public::PublicArchive;

use crate::client::AutonomiClient;
use crate::trove::directory_tree::{osstr_to_string, DirectoryTree, JsonSettings, WebsiteSettings};
use crate::trove::History;

/// If the tree contains a website, 'server style' configuration can be provided
/// files_root is the path to a the directory tree to upload
/// history_address optional address if an update
/// website_config is an optional configuration if publishing a website (TODO)
/// Returns the address of the history for updates (newly created if not supplied)
pub async fn publish_or_update_files(
    client: &AutonomiClient,
    files_root: &PathBuf,
    name: Option<String>,
    website_config: Option<PathBuf>,
    is_publish: bool,
) -> Result<(String, HistoryAddress, u64)> {
    println!("DEBUG publish_or_update_files()...");
    check_path_for_upload(&files_root)?;

    #[cfg(not(feature = "skip-network-compatibility-check"))]
    if is_publish && !is_new_network && !is_compatible_network(&client).await {
        let message = format!(
            "ERROR: This version of awe cannot publish to this Autonomi network\
        \nERROR: Please update awe and try again. See {MAIN_REPOSITORY}"
        )
        .clone();
        println!("{message}");
        return Err(eyre!(message));
    }

    println!("Uploading files to network...");
    let files_address = publish_files(&client, &files_root, &website_config)
        .await
        .inspect_err(|e| println!("{}", e))?;

    let name = if name.is_none() {
        if let Some(osstr) = files_root.file_name() {
            osstr_to_string(osstr)
        } else {
            None
        }
    } else {
        name
    };
    let name = if let Some(name) = name {
        name
    } else {
        return Err(eyre!(
            "DEBUG failed to obtain directory name from files_root: {files_root:?}"
        ));
    };

    let result = if is_publish {
        println!("Creating History on network...");
        History::<DirectoryTree>::create_online(client.clone(), name.clone(), None).await
    } else {
        println!("Getting History from network...");
        History::<DirectoryTree>::from_name(client.clone(), name.clone(), None).await
    };

    let mut files_history = match result {
        Ok(history) => history,
        Err(e) => {
            println!("DEBUG failed - {e}");
            return Err(e);
        }
    };

    println!("Updating History...");
    match files_history.publish_new_version(&files_address).await {
        Ok(version) => Ok((name, files_history.history_address(), version)),
        Err(e) => {
            let message = format!("Failed to update History: {e:?}");
            println!("{message}");
            return Err(eyre!(message));
        }
    }
}

pub fn report_content_published_or_updated(
    history_address: &HistoryAddress,
    name: &String,
    version: u64,
    files_root: &PathBuf,
    is_website: bool,
    is_new: bool,
) {
    let type_str = if is_website { "WEBSITE" } else { "FILES" };
    let action_str = if is_new { "PUBLISHED" } else { "UPDATED" };

    let files_history = history_address.to_hex();
    let root_default = format!("<{type_str}-ROOT>");

    let files_root = files_root.to_str();
    let files_root = if files_root.is_some() {
        files_root.unwrap()
    } else {
        root_default.as_str()
    };

    println!(
        "\n{type_str} {action_str} (version {version}). All versions available at XOR-URL:\nawv://{}\nDWEBNAME:\n{name}",
        &history_address.to_hex()
    );
    println!("\nNOTE:\n- To update thiscontent, use 'awe publish-update' as follows:\n\n   awe publish-update --name \"{name}\" --files-root {files_root:?}\n");
    println!(
        "- To browse the content use 'awe awv://<HISTORY-ADDRESS>' as follows:\n\n   awe awv://{files_history}\n"
    );
    println!("- For help use 'awe --help'\n");
}

/// Upload a directory of content and related metadata to Autonomi
/// Returns address of the uploaded metadata, needed to access the content
pub async fn publish_files(
    client: &AutonomiClient,
    files_root: &PathBuf,
    website_config: &Option<PathBuf>,
) -> Result<XorName> {
    let website_config = if let Some(website_config) = website_config {
        match JsonSettings::load_json_file(&website_config) {
            Ok(config) => Some(config),
            Err(e) => {
                return Err(eyre!(
                    "Failed to load website config from {website_config:?}. {}",
                    e.root_cause()
                ));
            }
        }
    } else {
        None
    };

    let mut website_settings = WebsiteSettings::new();
    if let Some(website_config) = website_config {
        website_settings.website_config = website_config;
    };

    match publish_content(client, files_root).await {
        Ok(archive) => {
            match publish_metadata(client, files_root, &archive, website_settings).await {
                Ok(files_metadata) => Ok(files_metadata),
                Err(e) => Err(eyre!(
                    "Failed to store metadata for files: {}",
                    e.root_cause()
                )),
            }
        }
        Err(e) => Err(eyre!("Failed to store content. {}", e.root_cause())),
    }
}

/// Upload the tree of files at files_root
/// Return the autonomi PublicArchive if all files are uploaded
pub async fn publish_content(
    client: &AutonomiClient,
    files_root: &PathBuf,
) -> Result<PublicArchive> {
    if !files_root.is_dir() {
        return Err(eyre!("Path to files must be a directory: {files_root:?}"));
    }

    if !files_root.exists() {
        return Err(eyre!("Path to files not found: {files_root:?}"));
    }

    if !files_root.read_dir().iter().len() == 0 {
        return Err(eyre!("Path to files is empty: {files_root:?}"));
    }

    println!("Uploading files from: {files_root:?}");
    let archive = match client
        .client
        .dir_upload_public(files_root.clone(), &client.wallet)
        .await
    {
        Ok(archive) => archive,
        Err(e) => return Err(eyre!("Failed to upload directory tree: {e}")),
    };

    println!("web publish completed files: {:?}", archive.map().len());

    println!("CONTENT UPLOADED:");
    for (path, datamap_chunk, _metadata) in archive.iter() {
        println!("{:64x} {path:?}", datamap_chunk);
    }

    Ok(archive)
}

// TODO review handling of errors that might occur here
// TODO consider extracting FilesApi::get_local_payment_and_upload_chunk() to StorageApi module
/// Create metadata for a files using the files_uploaded and website_settings
/// and store it on Autonomi
/// Assumes paths are canonical
/// Returns the xor address of the metadata stored
pub async fn publish_metadata(
    client: &AutonomiClient,
    files_root: &PathBuf,
    files_uploaded: &PublicArchive,
    website_settings: WebsiteSettings,
) -> Result<XorName> {
    let mut metadata = DirectoryTree::new(Some(website_settings));

    if let Some(mut files_root_string) = osstr_to_string(files_root.as_os_str()) {
        // Ensure the full_root_string ends with OS path separator
        if !files_root_string.ends_with(std::path::MAIN_SEPARATOR) {
            files_root_string += &String::from(std::path::MAIN_SEPARATOR);
        }
        println!("DEBUG publish_metadata() files_root '{files_root_string}'");

        for (relative_path, datamap_address, _file_metadata) in files_uploaded.iter() {
            // Archive paths include the parent directory of the upload so remove it
            let mut components = relative_path.components();
            components.next();
            let relative_path = components.as_path();

            if let Some(resource_relative_path) = osstr_to_string(relative_path.as_os_str()) {
                let resource_full_path = files_root_string.clone() + &resource_relative_path;
                let resource_based_path = String::from("/") + &resource_relative_path;
                println!("Adding '{resource_full_path}' as '{resource_based_path}'");
                match std::fs::metadata(resource_full_path) {
                    Ok(file_metadata) => metadata.add_content_to_metadata(
                        &resource_based_path,
                        datamap_address.clone(),
                        Some(&file_metadata),
                    )?,
                    Err(e) => {
                        println!("Failed to obtain metadata for file due to: {e:}");
                        metadata.add_content_to_metadata(
                            &resource_based_path,
                            datamap_address.clone(),
                            None,
                        )?
                    }
                };
            }
        }

        let xor_name = metadata
            .put_files_metadata_to_network(client.clone())
            .await?;
        println!("FILES METADATA UPLOADED:\nawm://{xor_name:64x}");

        return Ok(xor_name);
    }

    return Err(eyre!("Invalid root path: '{files_root:?}'"));
}

/// Check that the path is a directory tree containing at least one file
fn check_path_for_upload(files_root: &PathBuf) -> Result<()> {
    if !does_path_contains_files(&files_root) {
        if files_root.is_dir() {
            return Err(eyre!(
                "The directory specified for upload is empty. \
        Please verify the provided path."
            ));
        } else {
            return Err(eyre!(
                "The provided file path is invalid. Please verify the path."
            ));
        }
    }
    Ok(())
}

/// Return a count of all files in a directory tree
fn count_files_in_path_recursively(directory_path: &PathBuf) -> u32 {
    let entries_iterator = WalkDir::new(directory_path).into_iter().flatten();
    let mut count = 0;

    entries_iterator.for_each(|entry| {
        if entry.file_type().is_file() {
            count += 1;
        }
    });
    count
}

/// Check the directory tree containing at least one file
fn does_path_contains_files(directory_path: &PathBuf) -> bool {
    let entries_iterator = WalkDir::new(directory_path).into_iter().flatten();
    for entry in entries_iterator {
        if entry.file_type().is_file() {
            return true;
        }
    }
    false
}
