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
use std::path::PathBuf;

use autonomi::chunk::DataMapChunk;
use blsttc::SecretKey;
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use walkdir::WalkDir;

use autonomi::client::files::archive_public::PublicArchive;
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::{Metadata as FileMetadata, PrivateArchive};
use autonomi::AttoTokens;

use crate::client::DwebClient;
use crate::files::archive::ARCHIVE_PATH_SEPARATOR;
use crate::files::directory::{osstr_to_string, Tree, DWEB_DIRECTORY_HISTORY_DATAMAPCHUNK};
use crate::files::directory::{
    DWEB_DIRECTORY_HISTORY_CONTENT, DWEB_HISTORY_DIRECTORY, DWEB_SETTINGS_PATH,
};
use crate::helpers::retry::retry_until_ok;
use crate::history::{History, HistoryAddress};

/// Network data types for dweb APIs
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
pub enum DwebType {
    Chunk,
    PublicFile,
    PrivateFile,
    PublicArchive,
    PrivateArchive,
    History,
    Register,
    Pointer,
    PublicScratchpad,
    PrivateScratchpad,
    Vault,
    Unknown,
}

/// Publish a history entry, creating the history if no name is provided
///
/// files_root is the path to a the directory tree to upload
/// name is required for update but not publishing the first version
/// dweb_settings is an optional configuration if publishing a website (TODO)
///
/// Returns the amount paid (cost), history name for updates, and the history address
pub async fn publish_or_update_files(
    client: &DwebClient,
    files_root: &PathBuf,
    app_secret_key: SecretKey,
    name: Option<String>,
    dweb_settings: Option<PathBuf>,
    is_publish: bool,
) -> Result<(AttoTokens, String, HistoryAddress, u64)> {
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

    // check the history does not exist
    let (history_cost, mut files_history) = if is_publish {
        println!("Creating History on network...");
        match History::<Tree>::create_online(client.clone(), name.clone(), app_secret_key.clone())
            .await
        {
            Ok((cost, history)) => (cost, history),
            Err(e) => {
                let message = format!("Failed to publish new content - {e}");
                println!("{message}");
                return Err(eyre!(message));
            }
        }
    } else {
        println!("Getting History from network...");
        match History::<Tree>::from_name(
            client.clone(),
            app_secret_key.clone(),
            name.clone(),
            false,
            0,
        )
        .await
        {
            Ok(history) => (Into::into(0), history),
            Err(e) => {
                let message = format!("Failed to publish update to content - {e}");
                println!("{message}");
                return Err(eyre!(message));
            }
        }
    };

    // When the directory belongs to a history add a file whose name is History and version
    let history_filename = if let Ok(num_versions) = files_history.num_versions() {
        format!(
            "{}:{}",
            files_history.history_address().to_hex(),
            num_versions + 1
        )
    } else {
        files_history.history_address().to_hex()
    };

    let history_file_path = DWEB_HISTORY_DIRECTORY.to_string()
        + &ARCHIVE_PATH_SEPARATOR.to_string()
        + &history_filename;
    let history_file_path = PathBuf::from(history_file_path);
    let autonomi_metadata = FileMetadata {
        created: 0,
        modified: 0,
        size: 1,
        extra: None,
    };

    println!("Uploading files to network...");

    let (files_cost, archive_bytes, archive_type) = if client.api_control.use_public_archive {
        let (cost, mut archive) = publish_files_public(&client, &files_root, dweb_settings)
            .await
            .inspect_err(|e| println!("{}", e))?;

        let data_address = DataAddress::from_hex(DWEB_DIRECTORY_HISTORY_CONTENT).unwrap();
        archive.add_file(history_file_path, data_address, autonomi_metadata);

        let bytes = archive
            .to_bytes()
            .map_err(|e| eyre!(format!("Failed to serialize archive: {e:?}")))?;
        (cost, bytes, "PublicArchive")
    } else {
        let (cost, mut archive) = publish_files_private(&client, &files_root, dweb_settings)
            .await
            .inspect_err(|e| println!("{}", e))?;

        let datamap_chunk = DataMapChunk::from_hex(DWEB_DIRECTORY_HISTORY_DATAMAPCHUNK).unwrap();
        archive.add_file(history_file_path, datamap_chunk, autonomi_metadata);

        let bytes = archive
            .to_bytes()
            .map_err(|e| eyre!(format!("Failed to serialize archive: {e:?}")))?;
        (cost, bytes, "PrivateArchive")
    };

    println!("DEBUG storing {archive_type}...");
    let (archive_cost, archive_address) = match retry_until_ok(
        client.api_control.api_tries,
        &"Storing archive as bytes with data_put_public()",
        (client, archive_bytes),
        async move |(client, archive_bytes)| match client
            .client
            .data_put_public(archive_bytes, client.payment_option())
            .await
        {
            Ok((cost, archive_address)) => {
                // println!(
                //     "ARCHIVE ADDRESS:\n{}\nCost: {cost} ANT",
                //     archive_address.to_hex()
                // );
                Ok((cost, archive_address))
            }
            Err(e) => Err(eyre!("Failed to store the archive of uploaded files: {e}")),
        },
    )
    .await
    {
        Ok((chunk_cost, archive_address)) => (chunk_cost, archive_address),
        Err(e) => {
            let message = format!("max tries reached: {e:?}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };
    println!("{archive_type}: {archive_address}");

    let mut total_cost = files_cost.checked_add(history_cost).or(Some(files_cost));
    total_cost = total_cost.unwrap().checked_add(archive_cost).or(total_cost);

    println!("Updating History...");
    match files_history
        .publish_new_version(app_secret_key, &archive_address)
        .await
    {
        Ok((update_cost, version)) => {
            total_cost = total_cost.unwrap().checked_add(update_cost).or(total_cost);
            Ok((
                total_cost.unwrap(),
                name,
                files_history.history_address(),
                version,
            ))
        }
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
    _cost: AttoTokens,
    files_root: &PathBuf,
    is_website: bool,
    is_new: bool,
    is_awe: bool,
) {
    // Use same generic term "CONTENT" for website and directory (TODO remove is_website parameter)
    let type_str = if is_website { "CONTENT" } else { "CONTENT" };
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
        "\n{type_str} {action_str} (version {version}).\nAll versions available at HISTORY-ADDRESS:\n{}\nDWEBNAME:\n{name}",
        &history_address.to_hex()
    );
    if is_awe {
        println!("\nNOTE:\n- To update thiscontent, use:\n\n    awe publish-update --name \"{name}\" --files-root {files_root:?}\n");
        println!("- To browse the content use:\n\n    awe awv://{files_history}\n");
        println!("- For help use 'awe --help'\n");
    } else {
        println!("\nNOTE:\n- To update this content use:\n\n    dweb publish-update --name \"{name}\" --files-root {files_root:?}\n");
        println!("- To browse the content (after starting the server with 'dweb serve'):\n\n    dweb open {files_history}\n\n");
        println!("- For help use 'dweb --help'\n");
    }
}

/// Upload a directory tree to Autonomi and store the PrivateArchive
/// Returns the network address of the PrivateArchive (which can be used to initialise a Tree).
pub async fn publish_directory(
    client: &DwebClient,
    files_root: &PathBuf,
    use_public_archive: bool,
    dweb_settings: Option<PathBuf>,
) -> Result<(AttoTokens, ArchiveAddress)> {
    println!("DEBUG publish_directory() files_root '{files_root:?}'");

    let (files_cost, archive_bytes, archive_type) = if use_public_archive {
        let (cost, archive) = publish_files_public(&client, &files_root, dweb_settings)
            .await
            .inspect_err(|e| println!("{}", e))?;

        let bytes = archive
            .to_bytes()
            .map_err(|e| eyre!(format!("Failed to serialize archive: {e:?}")))?;
        (cost, bytes, "PublicArchive")
    } else {
        let (cost, archive) = publish_files_private(&client, &files_root, dweb_settings)
            .await
            .inspect_err(|e| println!("{}", e))?;

        let bytes = archive
            .to_bytes()
            .map_err(|e| eyre!(format!("Failed to serialize archive: {e:?}")))?;
        (cost, bytes, "PrivateArchive")
    };

    println!("DEBUG storing {archive_type}...");
    let (archive_cost, archive_address) = match retry_until_ok(
        client.api_control.api_tries,
        &"Storing archive as bytes with data_put_public()",
        (client, archive_bytes),
        async move |(client, archive_bytes)| match client
            .client
            .data_put_public(archive_bytes, client.payment_option())
            .await
        {
            Ok((cost, archive_address)) => {
                // println!(
                //     "ARCHIVE ADDRESS:\n{}\nCost: {cost} ANT",
                //     archive_address.to_hex()
                // );
                Ok((cost, archive_address))
            }
            Err(e) => Err(eyre!("Failed to store the archive of uploaded files: {e}")),
        },
    )
    .await
    {
        Ok((cost, archive_address)) => (cost, archive_address),
        Err(e) => {
            let message = format!("max tries reached: {e:?}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };
    println!("{archive_type}: {archive_address}");

    let total_cost = files_cost.checked_add(archive_cost).unwrap_or(files_cost);
    Ok((total_cost, archive_address))
}

/// Upload the tree of files with the option to include a dweb settings file.
/// Each file's datamap chunk is stored in the PrivateArchive but not on the
/// network. Does not store the PrivateArchive.
///
/// Returns the autonomi PrivateArchive if all files have been uploaded.
pub async fn publish_files_private(
    client: &DwebClient,
    files_root: &PathBuf,
    dweb_settings: Option<PathBuf>,
) -> Result<(AttoTokens, PrivateArchive)> {
    if !files_root.is_dir() {
        return Err(eyre!("Path to files must be a directory: {files_root:?}"));
    }

    if !files_root.exists() {
        return Err(eyre!("Path to files not found: {files_root:?}"));
    }

    if !files_root.read_dir().iter().len() == 0 {
        return Err(eyre!("Path to files is empty: {files_root:?}"));
    }

    let (files_cost, mut archive) = match directory_upload_private(client, files_root).await {
        Ok(result) => result,
        Err(e) => return Err(eyre!("Error max tries reached - {e}")),
    };

    let settings_cost = if let Some(dweb_path) = dweb_settings {
        let dweb_settings_file = dweb_path.to_string_lossy();
        let dweb_settings_path = PathBuf::from(DWEB_SETTINGS_PATH);
        println!("Uploading {dweb_settings_file}");

        match retry_until_ok(
            client.api_control.api_tries,
            &"file_content_upload_public()",
            (client, dweb_path.clone(), client.payment_option()),
            async move |(client, dweb_path, payment_option)| match client
                .client
                .file_content_upload(dweb_path, payment_option)
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    println!("Failed to upload dweb settings - {e}");
                    return Err(e.into());
                }
            },
        )
        .await
        {
            Ok((cost, datamap_chunk)) => {
                let autonomi_metadata =
                    crate::helpers::file::metadata_for_file(&dweb_settings_file);
                archive.add_file(dweb_settings_path, datamap_chunk, autonomi_metadata);
                cost
            }
            Err(e) => {
                println!("Error max tries reached - {e}");
                0.into()
            }
        }
    } else {
        0.into()
    };

    // let cost = settings_cost.checked_add(cost).unwrap_or(cost);
    // println!(
    //     "publish completed files: {:?}. Cost {cost} ANT",
    //     archive.map().len()
    // );

    println!("CONTENT UPLOADED:");
    for (path, datamap_chunk, _metadata) in archive.iter() {
        println!("{} {path:?}", datamap_chunk.to_hex());
    }
    // println!("Cost: {cost} ANT");

    let total_cost = files_cost.checked_add(settings_cost).unwrap_or(files_cost);
    Ok((total_cost, archive))
}

/// Upload the tree of files with the option to include a dweb settings file
///
/// Return the autonomi PublicArchive if all files have been uploaded. Does not store the PublicArchive.
pub async fn publish_files_public(
    client: &DwebClient,
    files_root: &PathBuf,
    dweb_settings: Option<PathBuf>,
) -> Result<(AttoTokens, PublicArchive)> {
    if !files_root.is_dir() {
        return Err(eyre!("Path to files must be a directory: {files_root:?}"));
    }

    if !files_root.exists() {
        return Err(eyre!("Path to files not found: {files_root:?}"));
    }

    if !files_root.read_dir().iter().len() == 0 {
        return Err(eyre!("Path to files is empty: {files_root:?}"));
    }

    let (files_cost, mut archive) = match directory_upload_public(client, files_root).await {
        Ok(result) => result,
        Err(e) => return Err(eyre!("Error max tries reached - {e}")),
    };

    let settings_cost = if let Some(dweb_path) = dweb_settings {
        let dweb_settings_file = dweb_path.to_string_lossy();
        let dweb_settings_path = PathBuf::from(DWEB_SETTINGS_PATH);
        println!("Uploading {dweb_settings_file}");

        match retry_until_ok(
            client.api_control.api_tries,
            &"file_content_upload_public()",
            (client, dweb_path.clone(), client.payment_option()),
            async move |(client, dweb_path, payment_option)| match client
                .client
                .file_content_upload_public(dweb_path, payment_option)
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    println!("Failed to upload dweb settings - {e}");
                    return Err(e.into());
                }
            },
        )
        .await
        {
            Ok((cost, upload_address)) => {
                let autonomi_metadata =
                    crate::helpers::file::metadata_for_file(&dweb_settings_file);
                archive.add_file(dweb_settings_path, upload_address, autonomi_metadata);
                cost
            }
            Err(e) => {
                println!("Error max tries reached - {e}");
                0.into()
            }
        }
    } else {
        0.into()
    };

    // let cost = settings_cost.checked_add(cost).unwrap_or(cost);
    // println!(
    //     "publish completed files: {:?}. Cost {cost} ANT",
    //     archive.map().len()
    // );

    println!("CONTENT UPLOADED:");
    for (path, datamap_chunk, _metadata) in archive.iter() {
        println!("{} {path:?}", datamap_chunk.to_hex());
    }
    // println!("Cost: {cost} ANT");

    let total_cost = files_cost.checked_add(settings_cost).unwrap_or(files_cost);
    Ok((total_cost, archive))
}

/// Upload a directory either using the Autonomi directory upload API or
/// do each file separately if file_by_file is true. Each file's datamap is
/// not stored in the PrivateArchive but not on the network.
pub async fn directory_upload_private(
    client: &DwebClient,
    files_root: &PathBuf,
) -> Result<(AttoTokens, PrivateArchive)> {
    let file_by_file = client.api_control.upload_file_by_file;

    let method = if file_by_file { " (file by file)" } else { "" };

    println!("Uploading files from directory{method}: {files_root:?}");
    if !file_by_file {
        return retry_until_ok(
            client.api_control.api_tries,
            &"dir_content_upload_public()",
            (client, files_root.clone(), client.payment_option()),
            async move |(client, files_root, payment_option)| match client
                .client
                .dir_content_upload(files_root.clone(), payment_option)
                .await
            {
                Ok((cost, archive)) => Ok((cost, archive)),
                Err(e) => return Err(eyre!("Failed to upload directory tree: {e}")),
            },
        )
        .await;
    };

    let mut archive = PrivateArchive::new();
    let mut total_cost = AttoTokens::zero();
    for entry in walkdir::WalkDir::new(files_root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                let msg = format!("Error walking directory {files_root:?} - {e}");
                println!("{msg}");
                return Err(eyre!(msg));
            }
        };

        if !entry.file_type().is_file() {
            continue;
        };
        let file = entry;

        println!("file: {:?}", file.file_name());
        let file_path = file.into_path();
        let file_path_str;
        match file_path.clone().into_os_string().into_string() {
            Ok(path_str) => file_path_str = path_str.clone(),
            Err(os_str) => {
                let msg = format!("Error converting file os_str to str - {os_str:?}");
                println!("{msg}");
                return Err(eyre!(msg));
            }
        };

        let cost = match retry_until_ok(
            client.api_control.api_tries,
            &"file_content_upload_public()",
            (client, file_path.clone(), client.payment_option()),
            async move |(client, file_path, payment_option)| match client
                .client
                .file_content_upload(file_path, payment_option)
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    println!("Failed to upload file - {e}");
                    return Err(e.into());
                }
            },
        )
        .await
        {
            Ok((cost, datamap_chunk)) => {
                let relative_path = match file_path.strip_prefix(&files_root) {
                    Ok(p) => p.to_path_buf(),
                    Err(_) => PathBuf::from(file_path_str.clone()),
                };
                let relative_path: String = relative_path.to_string_lossy().into_owned();

                let autonomi_metadata = crate::helpers::file::metadata_for_file(&file_path_str);
                archive.add_file(relative_path.into(), datamap_chunk, autonomi_metadata);
                cost
            }
            Err(e) => {
                println!("Error max tries reached - {e}");
                0.into()
            }
        };
        total_cost = total_cost.checked_add(cost).unwrap_or(total_cost);
    }

    Ok((total_cost, archive))
}

/// Upload a directory either using the Autonomi directory upload API or
/// do each file separately if file_by_file is true.
pub async fn directory_upload_public(
    client: &DwebClient,
    files_root: &PathBuf,
) -> Result<(AttoTokens, PublicArchive)> {
    let file_by_file = client.api_control.upload_file_by_file;

    let method = if file_by_file { " (file by file)" } else { "" };

    println!("Uploading files from directory{method}: {files_root:?}");
    if !file_by_file {
        return retry_until_ok(
            client.api_control.api_tries,
            &"dir_content_upload_public()",
            (client, files_root.clone(), client.payment_option()),
            async move |(client, files_root, payment_option)| match client
                .client
                .dir_content_upload_public(files_root.clone(), payment_option)
                .await
            {
                Ok((cost, archive)) => Ok((cost, archive)),
                Err(e) => return Err(eyre!("Failed to upload directory tree: {e}")),
            },
        )
        .await;
    };

    let mut archive = PublicArchive::new();
    let mut total_cost = AttoTokens::zero();
    for entry in walkdir::WalkDir::new(files_root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                let msg = format!("Error walking directory {files_root:?} - {e}");
                println!("{msg}");
                return Err(eyre!(msg));
            }
        };

        if !entry.file_type().is_file() {
            continue;
        };
        let file = entry;

        println!("file: {:?}", file.file_name());
        let file_path = file.into_path();
        let file_path_str;
        match file_path.clone().into_os_string().into_string() {
            Ok(path_str) => file_path_str = path_str.clone(),
            Err(os_str) => {
                let msg = format!("Error converting file os_str to str - {os_str:?}");
                println!("{msg}");
                return Err(eyre!(msg));
            }
        };

        let cost = match retry_until_ok(
            client.api_control.api_tries,
            &"file_content_upload_public()",
            (client, file_path.clone(), client.payment_option()),
            async move |(client, file_path, payment_option)| match client
                .client
                .file_content_upload_public(file_path, payment_option)
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    println!("Failed to upload file - {e}");
                    return Err(e.into());
                }
            },
        )
        .await
        {
            Ok((cost, upload_address)) => {
                let relative_path = match file_path.strip_prefix(&files_root) {
                    Ok(p) => p.to_path_buf(),
                    Err(_) => PathBuf::from(file_path_str.clone()),
                };
                let relative_path: String = relative_path.to_string_lossy().into_owned();

                let autonomi_metadata = crate::helpers::file::metadata_for_file(&file_path_str);
                archive.add_file(relative_path.into(), upload_address, autonomi_metadata);
                cost
            }
            Err(e) => {
                println!("Error max tries reached - {e}");
                0.into()
            }
        };
        total_cost = total_cost.checked_add(cost).unwrap_or(total_cost);
    }

    Ok((total_cost, archive))
}

/// Check that the path is a directory tree containing at least one file
fn check_path_for_upload(files_root: &PathBuf) -> Result<()> {
    if !does_path_contain_files(&files_root) {
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
fn does_path_contain_files(directory_path: &PathBuf) -> bool {
    let entries_iterator = WalkDir::new(directory_path).into_iter().flatten();
    for entry in entries_iterator {
        if entry.file_type().is_file() {
            return true;
        }
    }
    false
}
