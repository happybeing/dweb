// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use ant_protocol::storage::PointerAddress as HistoryAddress;
use color_eyre::eyre::{eyre, Result};
use xor_name::XorName;

// The following functions copied from sn_cli with minor changes (eg to message text)

/// Parse a hex HistoryAddress  with optional URL scheme
pub fn str_to_pointer_address(str: &str) -> Result<HistoryAddress> {
    match str_to_xor_name(str) {
        Ok(xor_name) => Ok(HistoryAddress::new(xor_name)),
        Err(e) => Err(eyre!(
            "Invalid History (pointer) address string '{str}':\n{e:?}"
        )),
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

////// awe protocol versions of the above for use by dweb CLI

pub const AWE_PROTOCOL_HISTORY: &str = "awv://";
#[allow(dead_code)]
pub const AWE_PROTOCOL_DIRECTORY: &str = "awm://";
#[allow(dead_code)]
pub const AWE_PROTOCOL_FILE: &str = "awf://";

// Default ports for HTTP / HTTPS
pub const DEFAULT_HTTP_PORT_STR: &str = "8080";
pub const DEFAULT_HTTPS_PORT_STR: &str = "8443";
pub const LOCALHOST: &str = "127.0.0.1";

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

    match str_to_pointer_address(str) {
        Ok(history_address) => Ok(history_address),
        Err(e) => Err(eyre!(
            "Invalid History (pointer) address string '{str}':\n{e:?}"
        )),
    }
}

/// Parse a hex PointerAddress with optional URL scheme
pub fn awe_str_to_pointer_address(str: &str) -> Result<HistoryAddress> {
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
