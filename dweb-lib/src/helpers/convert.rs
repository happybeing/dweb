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
use xor_name::XorName;

use ant_protocol::storage::PointerAddress;
use autonomi::client::data::DataAddress;
use autonomi::client::files::archive_public::ArchiveAddress;
use autonomi::GraphEntryAddress;

use crate::cache::directory_with_name::HISTORY_NAMES;
use crate::trove::HistoryAddress;

// The following functions copied from sn_cli with minor changes (eg to message text)

/// Parse a hex HistoryAddress
pub fn str_to_history_address(str: &str) -> Result<HistoryAddress> {
    match HistoryAddress::from_hex(str) {
        Ok(history_address) => Ok(history_address),
        Err(e) => Err(eyre!("Invalid History address string '{str}':\n{e:?}")),
    }
}

/// Parse a hex HistoryAddress
pub fn str_to_graph_entry_address(str: &str) -> Result<GraphEntryAddress> {
    match GraphEntryAddress::from_hex(str) {
        Ok(graphentry_address) => Ok(graphentry_address),
        Err(e) => Err(eyre!("Invalid graph entry address string '{str}':\n{e:?}")),
    }
}

/// Parse a hex PointerAddress
pub fn str_to_pointer_address(str: &str) -> Result<PointerAddress> {
    match PointerAddress::from_hex(str) {
        Ok(pointer_address) => Ok(pointer_address),
        Err(e) => Err(eyre!("Invalid pointer address string '{str}':\n{e:?}")),
    }
}

pub fn str_to_xor_name(str: &str) -> Result<XorName> {
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match hex::decode(str) {
        Ok(bytes) => match bytes.try_into() {
            Ok(xor_name_bytes) => Ok(XorName(xor_name_bytes)),
            Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
        },
        Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
    }
}

pub fn str_to_archive_address(str: &str) -> Result<DataAddress> {
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match ArchiveAddress::from_hex(str) {
        Ok(archive_address) => Ok(archive_address),
        Err(e) => Err(eyre!("ArchiveAddress not valid due to {e:?}")),
    }
}

pub fn str_to_data_address(str: &str) -> Result<DataAddress> {
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match DataAddress::from_hex(str) {
        Ok(data_address) => Ok(data_address),
        Err(e) => Err(eyre!("DataAddress not valid due to {e:?}")),
    }
}

/// Parse a string which is a recognised HISTORY-ADDRESS or ARCHIVE-ADDRESS
/// See also
pub fn address_tuple_from_address(
    address: &str,
) -> (Option<HistoryAddress>, Option<ArchiveAddress>) {
    if let Ok(address) = str_to_history_address(address) {
        return (Some(address), None);
    }

    if let Ok(address) = ArchiveAddress::from_hex(address) {
        return (None, Some(address));
    }

    return (None, None);
}

/// Parse a string which is a recognised DWEB-NAME, HISTORY-ADDRESS or ARCHIVE-ADDRESS
/// For now the only recognised DWEB-NAME is 'awesome'
pub fn address_tuple_from_address_or_name(
    address_or_name: &str,
) -> (Option<HistoryAddress>, Option<ArchiveAddress>) {
    if let Ok(address) = str_to_history_address(address_or_name) {
        return (Some(address), None);
    }

    if let Ok(address) = ArchiveAddress::from_hex(address_or_name) {
        return (None, Some(address));
    }

    if let Ok(lock) = &mut HISTORY_NAMES.lock() {
        if let Some(history_address) = lock.get(address_or_name).copied() {
            return (Some(history_address), None);
        }
    }

    return (None, None);
}

////// awe protocol versions of the above for use by dweb CLI

pub const AWE_PROTOCOL_HISTORY: &str = "awv://";
#[allow(dead_code)]
pub const AWE_PROTOCOL_DIRECTORY: &str = "awm://";
#[allow(dead_code)]
pub const AWE_PROTOCOL_FILE: &str = "awf://";

// Assignable port range (https://en.wikipedia.org/wiki/Registered_port)
pub const MIN_SERVER_PORT: u16 = 1024;
pub const MAX_SERVER_PORT: u16 = 49451;

/// Parse a port number for a server to listen on
pub fn parse_port_number(str: &str) -> Result<u16> {
    let port = str.parse::<u16>()?;

    if port >= MIN_SERVER_PORT && port <= MAX_SERVER_PORT {
        Ok(port)
    } else {
        Err(eyre!(
            "Invalid port number. Valid numbers are {MIN_SERVER_PORT}-{MAX_SERVER_PORT}"
        ))
    }
}

/// Parse a hostname for a server to listen on
pub fn parse_host(hostname: &str) -> Result<String> {
    let host = hostname.parse::<String>()?;

    match url::Url::parse(&format!("https://{host}")) {
        Ok(_url) => Ok(String::from(hostname)),
        Err(e) => Err(eyre!(e)),
    }
}

/// Parse a URL
pub fn parse_url(url: &str) -> Result<String> {
    match url::Url::parse(url) {
        Ok(_url) => Ok(String::from(url)),
        Err(e) => Err(eyre!(e)),
    }
}

/// Parse a hex HistoryAddress with optional URL scheme
pub fn awe_str_to_history_address(str: &str) -> Result<HistoryAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_HISTORY) {
        &str[AWE_PROTOCOL_HISTORY.len()..]
    } else {
        &str
    };

    match str_to_history_address(str) {
        Ok(history_address) => Ok(history_address),
        Err(e) => Err(eyre!(
            "Invalid History (pointer) address string '{str}':\n{e:?}"
        )),
    }
}

/// Parse a hex PointerAddress with optional URL scheme
pub fn awe_str_to_pointer_address(str: &str) -> Result<PointerAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_HISTORY) {
        &str[AWE_PROTOCOL_HISTORY.len()..]
    } else {
        &str
    };

    match str_to_pointer_address(str) {
        Ok(pointer_address) => Ok(pointer_address),
        Err(e) => Err(eyre!("Invalid pointer address string '{str}':\n{e:?}")),
    }
}

pub fn awe_str_to_data_address(str: &str) -> Result<DataAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_DIRECTORY) {
        &str[AWE_PROTOCOL_DIRECTORY.len()..]
    } else if str.starts_with(AWE_PROTOCOL_FILE) {
        &str[AWE_PROTOCOL_FILE.len()..]
    } else {
        &str
    };
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match DataAddress::from_hex(str) {
        Ok(data_address) => Ok(data_address),
        Err(e) => Err(eyre!("DataAddress not valid due to {e:?}")),
    }
}

pub fn awe_str_to_xor_name(str: &str) -> Result<XorName> {
    let str = if str.starts_with(AWE_PROTOCOL_DIRECTORY) {
        &str[AWE_PROTOCOL_DIRECTORY.len()..]
    } else if str.starts_with(AWE_PROTOCOL_FILE) {
        &str[AWE_PROTOCOL_FILE.len()..]
    } else {
        &str
    };
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match hex::decode(str) {
        Ok(bytes) => match bytes.try_into() {
            Ok(xor_name_bytes) => Ok(XorName(xor_name_bytes)),
            Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
        },
        Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
    }
}
