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
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use bytes::Bytes;
use color_eyre::eyre::{eyre, Result};

use autonomi::client::{data_types::chunk::DataMapChunk, files::RenameError};
use autonomi::data::DataAddress;
use autonomi::files::{Metadata, PrivateArchive, PublicArchive};

use crate::storage::DwebType;

/// A single type to handle Autonomi PublicArchive and PrivateArchive
///
/// Note: A DirectoryTree uses and adds to the features of a DualArchive and
/// is better suited for typical use cases. Both provide simplified handling
/// of the two Atonomi archive types.
///
/// The only difference between Autonomi PublicArchive and PrivateArchive is that
/// a PublicArchive contains datamap addresses for each file entry, whereas a
/// PrivateArchive stores the datamap Chunk. This Chunk contains both the datamap
/// and its address. In the case of a PrivateArchive it cannot be assumed that the
/// datamap has been stored on the network.
///
/// DualArchive is intended to simplify handling of both representations in a
/// single struct, and to facilitate use of PrivateArchive to handle public
/// files, since it is more efficient to hold the datamaps along with file
/// metadata.
///
/// Internally DualArchive contains both types and provides equivalent methods
/// for both. A dweb_type can be set to indicate which internal object is being
/// used, and this type will be set automatically by deserialisation methods.
///
#[derive(Clone)]
pub struct DualArchive {
    pub dweb_type: DwebType,
    pub public_archive: PublicArchive,
    pub private_archive: PrivateArchive,
}

impl DualArchive {
    /// Create a new emtpy local archive
    pub fn new() -> Self {
        Self {
            dweb_type: DwebType::Unknown,
            public_archive: PublicArchive::new(),
            private_archive: PrivateArchive::new(),
        }
    }

    /// Lookup a file and return available metadata - works for DwebType::PublicArchive and DwebType::PrivateArchive
    ///
    /// The return tuple contains hex encoded strings, one of which will be empty depending on the type of the
    /// underlying archive.
    ///
    /// If the file entry is found, its datamap_chunk will be returned for a PublicArchive or its data_address if
    /// in a PublicArchive.
    //
    /// Returned tuple: (datamap_chuString, data_address: String, &Metadata)
    pub fn lookup_file(&self, path: &PathBuf) -> Option<(String, String, &Metadata)> {
        match self.dweb_type {
            DwebType::PrivateArchive => {
                if let Some((datamap_chunk, metadata)) = self.private_archive.map().get(path) {
                    Some((datamap_chunk.to_hex(), "".to_string(), metadata))
                } else {
                    None
                }
            }
            DwebType::PublicArchive => {
                if let Some((data_address, metadata)) = self.public_archive.map().get(path) {
                    Some(("".to_string(), data_address.to_hex(), metadata))
                } else {
                    None
                }
            }
            _ => {
                println!(
                    "DEBUG DualArchive accessed with type {:?} - this is probably a bug",
                    self.dweb_type
                );
                None
            }
        }
    }

    /// Rename a file in an archive.
    /// Note that this does not upload the archive to the network
    pub fn rename_file(&mut self, old_path: &Path, new_path: &Path) -> Result<(), RenameError> {
        match self.public_archive.rename_file(old_path, new_path) {
            Ok(()) => (),
            Err(e) => return Err(e),
        }
        self.private_archive.rename_file(old_path, new_path)
    }

    /// Add file as a public archive.
    /// Assumes data_addr is the address of a datamap stored on the network
    pub fn add_file_as_public(&mut self, path: PathBuf, data_addr: DataAddress, meta: Metadata) {
        self.public_archive.add_file(path, data_addr, meta);
    }

    /// Add file as a private archive
    pub fn add_file_as_private(&mut self, path: PathBuf, data_map: DataMapChunk, meta: Metadata) {
        self.private_archive.add_file(path, data_map, meta);
    }

    /// List all files in the active archive
    pub fn files(&self) -> Vec<(PathBuf, Metadata)> {
        match self.dweb_type {
            DwebType::PrivateArchive => self.private_archive.files(),
            DwebType::PublicArchive => self.public_archive.files(),
            _ => Vec::<(PathBuf, Metadata)>::new(),
        }
    }

    /// List all data addresses of the files in the public archive
    pub fn addresses(&self) -> Vec<DataAddress> {
        self.public_archive.addresses()
    }

    /// List all data [`DataMapChunk`]s of the files in the private archive
    pub fn data_maps(&self) -> Vec<DataMapChunk> {
        self.private_archive.data_maps()
    }

    /// Iterate over the public archive items
    /// Returns an iterator over ([`PathBuf`], [`DataAddress`], [`Metadata`])
    pub fn iter_as_public(&self) -> impl Iterator<Item = (&PathBuf, &DataAddress, &Metadata)> {
        self.public_archive.iter()
    }

    /// Get the underlying map for the public archive
    pub fn map_as_public(&self) -> &BTreeMap<PathBuf, (DataAddress, Metadata)> {
        &self.public_archive.map()
    }

    /// Get the underlying map
    pub fn map_as_private(&self) -> &BTreeMap<PathBuf, (DataMapChunk, Metadata)> {
        self.private_archive.map()
    }

    /// Deserialise bytes for either public or private and return as a DualArchive
    ///
    /// Attempts to deserialise as PrivateArchive first and if that fails tries again as PublicArchive
    ///
    /// Priority is given to PrivateArchive because it seems preferable even for public data to
    /// use a PrivateArchive to store the addresses and datamaps of files stored on the network.
    /// This avoids the need to fetch the datamap of each file before getting the content of the file.
    pub fn from_bytes(data: Bytes) -> Result<DualArchive> {
        // println!("DEBUG PrivateArchive detection... from_bytes_as_private()");
        // match DualArchive::from_bytes_as_private(data.clone()) {
        //     Ok(archive) => {
        //         println!(
        //             "PrivateArchive shows {} files",
        //             archive.private_archive.files().len()
        //         );
        //         for (path_buf, (datamap, _metadata)) in archive.private_archive.map().iter() {
        //             println!(
        //                 "{}: address: {}",
        //                 path_buf.as_path().to_str().unwrap(),
        //                 datamap.address()
        //             );
        //             println!("hex: {}", datamap.to_hex());
        //         }
        //     }
        //     Err(_) => println!("FAIL"),
        // };

        // println!("DEBUG PublicArchive detection... from_bytes_as_public()");
        // match DualArchive::from_bytes_as_public(data.clone()) {
        //     Ok(archive) => {
        //         println!(
        //             "PublicArchive shows {} files",
        //             archive.public_archive.files().len()
        //         );
        //         for (path_buf, (data_address, _metadata)) in archive.public_archive.map() {
        //             println!(
        //                 "{}: address: {}",
        //                 path_buf.as_path().to_str().unwrap(),
        //                 data_address.to_hex()
        //             );
        //         }
        //     }
        //     Err(_) => println!("FAIL"),
        // };

        println!("DEBUG DualArchive attempting to deserialise as PrivateArchive...");
        match DualArchive::from_bytes_as_private(data.clone()) {
            Ok(private_dual_archive) => Ok(private_dual_archive),
            Err(_e) => {
                println!("DEBUG DualArchive attempting to deserialise as PublicArchive...");
                match DualArchive::from_bytes_as_public(data) {
                    Ok(public_dual_archive) => Ok(public_dual_archive),
                    Err(e) => Err(e),
                }
            }
        }

        // For testing from_bytes_as_public() fails for PrivateArchive
        // println!("DEBUG TEST DualArchive attempting to deserialise as PublicArchive...");
        // match DualArchive::from_bytes_as_public(data.clone()) {
        //     Ok(archive) => Ok(archive),
        //     Err(_e) => {
        //         println!("DEBUG TEST DualArchive attempting to deserialise as PrivateArchive...");
        //         match DualArchive::from_bytes_as_private(data) {
        //             Ok(archive) => Ok(archive),
        //             Err(e) => Err(e),
        //         }
        //     }
        // }
    }

    /// Deserialize as a PrivateArchive
    pub fn from_bytes_as_private(data: Bytes) -> Result<DualArchive> {
        match PrivateArchive::from_bytes(data) {
            Ok(archive) => {
                // We can reject a PublicArchive that deserialises as a PrivateArchive by checking the datamap length
                // See also: https://github.com/maidsafe/autonomi/issues/2892
                if let Some((_path_buf, (datamap_chunk, _metadata))) = archive.map().iter().next() {
                    if datamap_chunk.to_hex().len() <= crate::helpers::DATA_ADDRESS_LEN {
                        return Err(eyre!(
                            "Found DataAddress instead of DatamapChunk: {}",
                            datamap_chunk.to_hex()
                        ));
                    }
                }
                {
                    Ok(DualArchive {
                        dweb_type: DwebType::PrivateArchive,
                        public_archive: PublicArchive::new(),
                        private_archive: archive,
                    })
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    /// Deserialize as a PublicArchive
    pub fn from_bytes_as_public(data: Bytes) -> Result<DualArchive> {
        match Self::from_bytes_as_private(data.clone()) {
            Ok(_archive) => return Err(eyre!("Found PrivateArchive not PublicArchive")),
            Err(_) => {}
        }

        match PublicArchive::from_bytes(data) {
            Ok(archive) => Ok(DualArchive {
                dweb_type: DwebType::PublicArchive,
                public_archive: archive,
                private_archive: PrivateArchive::new(),
            }),
            Err(e) => return Err(e.into()),
        }
    }

    /// Serialize as a PublicArchive
    pub fn to_bytes_as_public(&self) -> Result<Bytes, rmp_serde::encode::Error> {
        match self.dweb_type {
            DwebType::PublicArchive => PublicArchive::to_bytes(&self.public_archive),

            _ => {
                return Err(rmp_serde::encode::Error::InvalidDataModel(
                    "DualArchive::to_bytes_as_public() can only serialise DwebType::PublicArchive",
                ))
            }
        }
    }

    /// Serialize as a PrivateArchive
    pub fn to_bytes_as_private(&self) -> Result<Bytes, rmp_serde::encode::Error> {
        match self.dweb_type {
            DwebType::PrivateArchive => PrivateArchive::to_bytes(&self.private_archive),

            _ => return Err(rmp_serde::encode::Error::InvalidDataModel(
                "DualArchive::to_bytes_as_private() can only serialise DwebType::PrivateArchive",
            )),
        }
    }

    /// Merge as a PublicArchive
    ///
    /// If there are duplicate entries for the same filename, the files from `other` will overwrite those of `self`.
    pub fn merge_as_public(&mut self, other: &PublicArchive) {
        self.public_archive.merge(other)
    }

    /// Merge as a PrivateArchive
    ///
    /// If there are duplicate entries for the same filename, the files from `other` will overwrite those of `self`.
    pub fn merge_as_private(&mut self, other: &PrivateArchive) {
        self.private_archive.merge(other)
    }
}
